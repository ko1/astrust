// Kernel#format / String#%: CRuby sprintf.c semantics (argument numbering checks, BSD
// vfprintf float layout on top of a dtoa that mimics CRuby's quick path).
#![forbid(unsafe_code)]
#[allow(unused_imports)]
use crate::heap::no_legacy::*;   // the old accessors cannot be named here
#[allow(unused_imports)]
use crate::heap::no_legacy_str::*;
use crate::builtins::kernel::*;
use crate::builtins::numeric_methods::{num_to_long, to_f, to_integer};
use crate::object::*;
use crate::string::*;
use crate::symbol::intern;
use crate::value::*;
use crate::vm::*;

// ---- dtoa -----------------------------------------------------------------------------------

const TENS: [f64; 23] = [1e0, 1e1, 1e2, 1e3, 1e4, 1e5, 1e6, 1e7, 1e8, 1e9, 1e10, 1e11, 1e12, 1e13, 1e14, 1e15, 1e16, 1e17, 1e18, 1e19, 1e20, 1e21, 1e22];
const BIGTENS: [f64; 5] = [1e16, 1e32, 1e64, 1e128, 1e256];

/// Exact decimal digits by Rust's correctly rounded formatter (round-half-even on ties).
/// mode 2: `nd` significant digits; mode 3: `nd` digits after the point.  Trailing zeros trimmed.
fn exact_digits(d: f64, mode: u8, nd: i32) -> (Vec<u8>, i32) {
    let s = if mode == 2 { format!("{:.*e}", (nd.max(1) - 1) as usize, d) } else { format!("{:.*e}", 40, d) };
    let (m, e) = s.split_once('e').unwrap();
    let mut exp: i32 = e.parse().unwrap();
    let mut digits: Vec<u8> = m.bytes().filter(|b| b.is_ascii_digit()).collect();
    if mode == 3 {
        // re-round to nd decimals exactly
        let f = format!("{:.*}", nd.max(0) as usize, d);
        let (ip, fp) = f.split_once('.').map(|(a, b)| (a.to_string(), b.to_string())).unwrap_or((f.clone(), String::new()));
        let all: Vec<u8> = format!("{}{}", ip, fp).into_bytes();
        let lead = all.iter().position(|&x| x != b'0');
        match lead {
            None => return (vec![], -(nd)),
            Some(p) => { digits = all[p..].to_vec(); exp = ip.len() as i32 - p as i32 - 1; }
        }
    }
    while digits.len() > 1 && *digits.last().unwrap() == b'0' { digits.pop(); }
    if digits == [b'0'] { return (vec![], exp + 1); }
    (digits, exp + 1)
}

/// CRuby's ruby_dtoa (mode 2 or 3) for a positive finite d: (digits, decpt).
fn dtoa(d0: f64, mode: u8, ndigits: i32) -> (Vec<u8>, i32) {
    if d0 == 0.0 { return (vec![b'0'], 1); }
    let kx: i32 = format!("{:e}", d0).split_once('e').unwrap().1.parse().unwrap();
    // Gay's estimate of floor(log10(d)): may be one too large (k_check)
    let bits = d0.to_bits();
    let bexp = ((bits >> 52) & 0x7ff) as i32;
    let (k, k_check) = if bexp == 0 { (kx, false) } else {
        let d2 = f64::from_bits((bits & ((1u64 << 52) - 1)) | (1023u64 << 52));
        let ds = (d2 - 1.5) * 0.289529654602168 + 0.1760912590558 + (bexp - 1023) as f64 * 0.301029995663981;
        let mut k = ds as i32;
        if ds < 0.0 && ds != k as f64 { k -= 1; }
        if (0..=22).contains(&k) { if d0 < TENS[k as usize] { k -= 1; } (k, false) } else { (k, true) }
    };
    let (ilim, ilim1) = if mode == 2 { let n = ndigits.max(1); (n, n) } else { (k + 1 + ndigits, k + ndigits) };
    if ilim >= 1 && ilim <= 14 {
        match quick(d0, k, ilim, ilim1, k_check) { Some(Ok(r)) => return r, Some(Err((s, k2))) => return half_even(d0, s, k2), None => {} }
    }
    let ilim = if mode == 2 { ilim } else { kx + 1 + ndigits };
    if ilim <= 0 && mode == 3 {
        // at most a single rounding digit
        if ilim < 0 { return (vec![], -ndigits); }
        let half = 0.5 * 10f64.powi(kx + 1);
        return if d0 > half { (vec![b'1'], kx + 2) } else { (vec![], -ndigits) };
    }
    exact_digits(d0, mode, ndigits)
}

/// Quick path ended within its error of one half with an even last digit: CRuby falls back
/// to exact arithmetic but with `half` set (never increments an even digit, keeps zeros).
fn half_even(d0: f64, s: Vec<u8>, k: i32) -> (Vec<u8>, i32) {
    // exact decimal expansion: compare the dropped tail with one half
    let full = format!("{:.800e}", d0);
    let (m, e) = full.split_once('e').unwrap();
    let ex: i32 = e.parse().unwrap();
    let digs: Vec<u8> = m.bytes().filter(|b| b.is_ascii_digit()).collect();
    // position (in digs) right after the last kept digit
    let cut = (s.len() as i32 + (ex - k)) as usize;
    let tail = if cut < digs.len() { &digs[cut..] } else { &[][..] };
    let up = !tail.is_empty() && (tail[0] > b'5' || (tail[0] == b'5' && tail[1..].iter().any(|&x| x != b'0')));
    if d0.fract() == 0.0 && k <= 14 {
        return if up { bump(s, k) } else { (s, k + 1) };
    }
    if up { return (s, k + 1); }
    let mut s = s;
    while s.len() > 1 && *s.last().unwrap() == b'0' { s.pop(); }
    (s, k + 1)
}

type Quick = Option<Result<(Vec<u8>, i32), (Vec<u8>, i32)>>;
fn quick(d0: f64, k0: i32, ilim0: i32, ilim1: i32, k_check: bool) -> Quick {
    let mut d = d0;
    let mut k = k0;
    let mut ilim = ilim0;
    let mut ieps = 2;
    if k > 0 {
        let mut ds = TENS[(k & 0xf) as usize];
        let mut j = k >> 4;
        if j & 0x10 != 0 { j &= 0xf; d /= BIGTENS[4]; ieps += 1; }
        let mut i = 0;
        while j != 0 { if j & 1 != 0 { ieps += 1; ds *= BIGTENS[i]; } j >>= 1; i += 1; }
        d /= ds;
    } else if k < 0 {
        let j1 = -k;
        d *= TENS[(j1 & 0xf) as usize];
        let mut j = j1 >> 4; let mut i = 0;
        while j != 0 { if j & 1 != 0 { ieps += 1; d *= BIGTENS[i]; } j >>= 1; i += 1; }
    }
    if k_check && d < 1.0 && ilim > 0 {
        if ilim1 <= 0 { return None; }
        ilim = ilim1; k -= 1; d *= 10.0; ieps += 1;
    }
    let mut eps = ieps as f64 * d + 7.0;
    eps *= 2f64.powi(-52);
    eps *= TENS[(ilim - 1) as usize];
    let mut s: Vec<u8> = vec![];
    let mut i = 1;
    loop {
        let l = d as i64;
        d -= l as f64;
        if d == 0.0 { ilim = i; }
        s.push(b'0' + l as u8);
        if i == ilim {
            if d > 0.5 + eps { return Some(Ok(bump(s, k))); }
            if d < 0.5 - eps { while s.len() > 1 && *s.last().unwrap() == b'0' { s.pop(); } return Some(Ok((s, k + 1))); }
            // within the error of one half: round half to even, keeping the digits as they are
            if (s.last().unwrap() - b'0') & 1 == 1 { return Some(Ok(bump(s, k))); }
            return Some(Err((s, k)));
        }
        i += 1;
        d *= 10.0;
    }
}

fn bump(mut s: Vec<u8>, k: i32) -> (Vec<u8>, i32) {
    loop {
        match s.pop() {
            Some(b'9') => continue,
            Some(x) => { s.push(x + 1); return (s, k + 1); }
            None => return (vec![b'1'], k + 2),
        }
    }
}

/// "%-.10g" of a finite value (CRuby's float-out-of-range messages).
pub fn fmt_g10(f: f64) -> String { format!("{}{}", if f < 0.0 { "-" } else { "" }, fmt_float(f.abs(), b'g', Some(10), false)) }

/// BSD vfprintf layout of a non-negative finite value for f/e/E/g/G.
fn fmt_float(f: f64, conv: u8, prec: Option<usize>, alt: bool) -> String {
    let p = prec.unwrap_or(6) as i32;
    match conv {
        b'f' => {
            let (dg, ex) = dtoa(f, 3, p);
            layout_f(&dg, ex, p, alt)
        }
        b'e' | b'E' => {
            let (dg, ex) = dtoa(f, 2, p + 1);
            let s = layout_e(&dg, if dg.is_empty() { 1 } else { ex }, p, alt);
            if conv == b'E' { s.to_uppercase() } else { s }
        }
        _ => {
            let p = if p == 0 { 1 } else { p };
            let (dg, ex) = dtoa(f, 2, p);
            let ndig = dg.len().max(1) as i32;
            let ex = if f == 0.0 { 1 } else { ex };
            let s = if ex <= -4 || ex > p {
                layout_e(&dg, ex, if alt { p - 1 } else { ndig - 1 }, alt)
            } else {
                let decs = if alt { p - ex } else { ndig - ex };
                layout_f(&dg, ex, decs.max(0), alt)
            };
            if conv == b'G' { s.to_uppercase() } else { s }
        }
    }
}
fn digit_at(dg: &[u8], i: i32) -> char { if i >= 0 && (i as usize) < dg.len() { dg[i as usize] as char } else { '0' } }
fn layout_f(dg: &[u8], ex: i32, decs: i32, alt: bool) -> String {
    let mut s = String::new();
    if ex <= 0 { s.push('0'); } else { for i in 0..ex { s.push(digit_at(dg, i)); } }
    if decs > 0 || alt { s.push('.'); }
    for i in 0..decs { s.push(digit_at(dg, ex + i)); }
    s
}
fn layout_e(dg: &[u8], ex: i32, decs: i32, alt: bool) -> String {
    let mut s = String::new();
    s.push(digit_at(dg, 0));
    if decs > 0 || alt { s.push('.'); }
    for i in 0..decs { s.push(digit_at(dg, 1 + i)); }
    let e = if dg.is_empty() { 0 } else { ex - 1 };
    s.push_str(&format!("e{}{:02}", if e < 0 { '-' } else { '+' }, e.abs()));
    s
}
/// %a: C99 hexadecimal float of |f|.
fn fmt_hex_float(f: f64, prec: Option<usize>, upper: bool, alt: bool) -> String {
    let bits = f.to_bits();
    let mut exp = ((bits >> 52) & 0x7ff) as i32;
    let mut mant = bits & ((1u64 << 52) - 1);
    let s = if exp == 0 && mant == 0 {
        let fr = match prec { Some(p) if p > 0 => format!(".{}", "0".repeat(p)), _ if alt => ".".to_string(), _ => String::new() };
        format!("0x0{}p+0", fr)
    } else {
        if exp == 0 { // subnormal: normalize
            while mant & (1u64 << 52) == 0 { mant <<= 1; exp -= 1; }
            mant &= (1u64 << 52) - 1; exp += 1;
        }
        let e = exp - 1023;
        let mut lead = 1u64;
        let mut hex = format!("{:013x}", mant);
        if let Some(p) = prec {
            if p < 13 {
                let shift = (13 - p) * 4;
                let full = (lead << 52) | mant;
                let rem = full & ((1u64 << shift) - 1);
                let mut q = full >> shift;
                let half = 1u64 << (shift - 1);
                if rem > half || (rem == half && q & 1 == 1) { q += 1; }
                lead = q >> (p * 4);
                hex = if p > 0 { format!("{:0w$x}", q & ((1u64 << (p * 4)) - 1), w = p) } else { String::new() };
            } else { hex.push_str(&"0".repeat(p - 13)); }
        } else { hex = hex.trim_end_matches('0').to_string(); }
        let fr = if !hex.is_empty() { format!(".{}", hex) } else if alt { ".".to_string() } else { String::new() };
        format!("0x{}{}p{}{}", lead, fr, if e < 0 { '-' } else { '+' }, e.abs())
    };
    if upper { s.to_uppercase() } else { s }
}

// ---- format ---------------------------------------------------------------------------------

struct Args<'a> { args: &'a [Value], next: usize, posarg: i64, hash: Option<Value> }
impl<'a> Args<'a> {
    fn next_arg(&mut self, c: &mut Ctx) -> RV {
        if self.posarg == -1 { return arg_err(c, format!("unnumbered({}) mixed with numbered", self.next + 1)); }
        if self.posarg == -2 { return arg_err(c, format!("unnumbered({}) mixed with named", self.next + 1)); }
        self.next += 1;
        self.posarg = self.next as i64;
        match self.args.get(self.next - 1) { Some(&v) => Ok(v), None => arg_err(c, "too few arguments") }
    }
    fn pos_arg(&mut self, c: &mut Ctx, n: i64) -> RV {
        if self.posarg > 0 { return arg_err(c, format!("numbered({}) after unnumbered({})", n, self.posarg)); }
        if self.posarg == -2 { return arg_err(c, format!("numbered({}) after named", n)); }
        if n < 1 { return arg_err(c, format!("invalid index - {}$", n)); }
        self.posarg = -1;
        match self.args.get(n as usize - 1) { Some(&v) => Ok(v), None => arg_err(c, "too few arguments") }
    }
    fn hash(&mut self, c: &mut Ctx, name: &[u8], enc: u8) -> RV {
        if self.posarg > 0 { return Err(raise_bytes(c, c.vm().c_argument_error.get(), &[b"named", name, format!(" after unnumbered({})", self.posarg).as_bytes()].concat(), enc)); }
        if self.posarg == -1 { return Err(raise_bytes(c, c.vm().c_argument_error.get(), &[b"named", name, b" after numbered"].concat(), enc)); }
        self.posarg = -2;
        if let Some(h) = self.hash { return Ok(h); }
        if self.args.len() != 1 { return arg_err(c, "one hash required"); }
        let a = self.args[0];
        let h = if is_hash(a) { a } else if a.is_heap() && respond_to(a, intern(b"to_hash"), true) { funcall(c, a, intern(b"to_hash"), &[], std::ptr::null())? } else { NIL };
        if !is_hash(h) { return arg_err(c, "one hash required"); }
        self.hash = Some(h);
        Ok(h)
    }
}

const FMINUS: u32 = 1; const FPLUS: u32 = 2; const FSPACE: u32 = 4; const FZERO: u32 = 8; const FSHARP: u32 = 16;
const FWIDTH: u32 = 32; const FPREC: u32 = 64;

fn int_arg(c: &mut Ctx, v: Value) -> RV {
    if is_float(v) {
        let f = float_val(v);
        if f.is_nan() || f.is_infinite() { return err(c, c.vm().c_float_domain_error.get(), crate::numeric::float_to_s(f)); }
        return Ok(crate::legacy::f64_to_int(f.trunc()));
    }
    if is_string(v) { return parse_int_for_format(c, v); }
    if v == NIL { return type_err(c, "can't convert nil into Integer"); }
    if is_integer(v) { return Ok(v); }
    if v.is_heap() && respond_to(v, crate::symbol::s::to_int, true) { return to_integer(c, v); }
    if v.is_heap() && respond_to(v, intern(b"to_i"), true) {
        let r = funcall(c, v, intern(b"to_i"), &[], std::ptr::null())?;
        if is_integer(r) { return Ok(r); }
    }
    type_err(c, format!("can't convert {} into Integer", class_name_for_err(v)))
}
fn float_arg(c: &mut Ctx, v: Value) -> Result<f64, Esc> {
    if is_string(v) { let f = funcall(c, c.vm().main_obj.get(), intern(b"Float"), &[v], std::ptr::null())?; return to_f(c, f); }
    to_f(c, v)
}

pub fn format(c: &mut Ctx, fmtv: Value, args: &[Value]) -> RV {
    use crate::builtins::encoding as en;
    let fmts = expect_str(c, fmtv)?;
    let (fenc, fmt) = { let r = c.rstring(fmts); (r.enc, r.b.to_vec()) };
    if !en::ascii_compat(fenc) { return err(c, c.vm().c_compat_error.get(), format!("ASCII incompatible encoding: {}", en::enc_name(fenc))); }
    let mut out: Vec<u8> = Vec::with_capacity(fmt.len() + 16);
    let mut renc = fenc;
    let mut st = Args { args, next: 0, posarg: 0, hash: None };
    let mut i = 0;
    while i < fmt.len() {
        let ch = fmt[i];
        if ch != b'%' { out.push(ch); i += 1; continue; }
        let start = i;
        i += 1;
        let mut flags = 0u32;
        let mut width: i64 = 0; let mut prec: i64 = -1;
        let mut val: Option<Value> = None;
        let mut braced = false;
        let mut name_for_err: Option<Vec<u8>> = None;
        let conv: u8;
        loop {
            if i >= fmt.len() {
                // sprintf.c reads the terminating NUL: only a lone trailing '%' is "incomplete"
                if i == start + 1 { return arg_err(c, "incomplete format specifier; use %% (double %) instead"); }
                return arg_err(c, "malformed format string");
            }
            let x = fmt[i];
            match x {
                b' ' | b'#' | b'+' | b'-' | b'0' => {
                    if flags & FWIDTH != 0 { return arg_err(c, "flag after width"); }
                    if flags & FPREC != 0 { return arg_err(c, "flag after precision"); }
                    flags |= match x { b' ' => FSPACE, b'#' => FSHARP, b'+' => FPLUS, b'-' => FMINUS, _ => FZERO };
                    i += 1;
                }
                b'1'..=b'9' => {
                    let s0 = i; while i < fmt.len() && fmt[i].is_ascii_digit() { i += 1; }
                    let n: i64 = std::str::from_utf8(&fmt[s0..i]).unwrap().parse().unwrap_or(i64::MAX);
                    if i < fmt.len() && fmt[i] == b'$' {
                        if val.is_some() { return arg_err(c, format!("value given twice - {}$", n)); }
                        val = Some(st.pos_arg(c, n)?); i += 1;
                    } else {
                        if flags & FWIDTH != 0 { return arg_err(c, "width given twice"); }
                        if n > i32::MAX as i64 { return arg_err(c, "width too big"); }
                        width = n; flags |= FWIDTH;
                    }
                }
                b'<' | b'{' => {
                    let close = if x == b'<' { b'>' } else { b'}' };
                    let s0 = i;
                    let e = match fmt[s0 + 1..].iter().position(|&z| z == close) { Some(p) => s0 + 1 + p, None => return arg_err(c, "malformed name - unmatched parenthesis") };
                    let name = &fmt[s0 + 1..e];
                    let label = &fmt[s0..=e];
                    let h = st.hash(c, label, fenc)?;
                    if val.is_some() {
                        let prev = name_for_err.clone().unwrap_or_default();
                        return Err(raise_bytes(c, c.vm().c_argument_error.get(), &[b"named", label, b" after <", &prev, b">"].concat(), fenc));
                    }
                    let key = Value::sym(intern(name));
                    let v = match crate::hash::hash_get_opt(c, h, key)? {
                        Some(v) => v,
                        None => {
                            // rb_hash_default_value; a nil default is "not found"
                            let d = if c.hash(h).default_proc != NIL || c.hash(h).default != NIL { crate::hash::hash_aref(c, h, key)? } else { NIL };
                            if d == NIL {
                                let m = c.str_new_enc(&[b"key", label, b" not found"].concat(), fenc);
                                let e2 = funcall(c, c.vm().c_key_error.get(), intern(b"new"), &[m], std::ptr::null())?;
                                ivar_set_raw(e2, crate::symbol::s::key, key); ivar_set_raw(e2, crate::symbol::s::receiver, h);
                                return Err(raise_exc(c, e2));
                            }
                            d
                        }
                    };
                    val = Some(v); name_for_err = Some(name.to_vec());
                    i = e + 1;
                    if close == b'}' { braced = true; conv = b's'; break; }
                }
                b'*' => {
                    if flags & FWIDTH != 0 { return arg_err(c, "width given twice"); }
                    i += 1;
                    let wv = aster(c, &fmt, &mut i, &mut st)?;
                    let w = num_to_long(c, wv)?;
                    if w < 0 { flags |= FMINUS; width = -w; } else { width = w; }
                    flags |= FWIDTH;
                }
                b'.' => {
                    if flags & FPREC != 0 { return arg_err(c, "precision given twice"); }
                    flags |= FPREC;
                    i += 1;
                    if i < fmt.len() && fmt[i] == b'*' {
                        i += 1;
                        let pv = aster(c, &fmt, &mut i, &mut st)?;
                        let p = num_to_long(c, pv)?;
                        if p < 0 { flags &= !FPREC; } else { prec = p; }
                    } else {
                        let s0 = i; while i < fmt.len() && fmt[i].is_ascii_digit() { i += 1; }
                        prec = std::str::from_utf8(&fmt[s0..i]).unwrap().parse().unwrap_or(0);
                    }
                }
                b'%' => {
                    if flags != 0 || i != start + 1 { return arg_err(c, "invalid format character - %"); }
                    out.push(b'%'); i += 1; conv = 0; break;
                }
                0 => return arg_err(c, "malformed format string"),
                _ => { conv = x; i += 1; break; }
            }
        }
        if conv == 0 { continue; }
        if !braced && !matches!(conv, b'd' | b'i' | b'u' | b'f' | b'e' | b'E' | b'g' | b'G' | b'a' | b'A' | b'x' | b'X' | b'o' | b'b' | b'B' | b's' | b'p' | b'c') {
            if conv < 0x20 || conv == 0x7f { return arg_err(c, "malformed format string"); }
            let ch = crate::builtins::encoding::mbclen(fenc, &fmt, i - 1);
            return arg_err(c, format!("malformed format string - %{}", String::from_utf8_lossy(&fmt[i - 1..i - 1 + ch])));
        }
        let v = match val { Some(v) => v, None => st.next_arg(c)? };
        let prec_o: Option<usize> = if flags & FPREC != 0 { Some(prec as usize) } else { None };
        let w = if flags & FWIDTH != 0 { width as usize } else { 0 };
        match conv {
            b's' | b'p' | b'c' => {
                let sv = match conv {
                    b'p' => inspect_value(c, v)?,
                    b's' => crate::rt::srb_tostr(c, v)?,
                    _ => {
                        if is_string(v) || (v.is_heap() && respond_to(v, crate::symbol::s::to_str, true) && !is_integer(v)) {
                            let s2 = expect_str(c, v)?;
                            let first = c.chars_of(s2).into_iter().next().unwrap_or_default();
                            c.str_new_enc(&first, c.str_enc(s2))
                        } else {
                            let n = num_to_long(c, v)?;
                            if n < 0 { return err(c, c.vm().c_range_error.get(), format!("{} out of char range", n)); }
                            match en::code_to_bytes(renc, n as u32) {
                                Some(bs) => c.str_new_enc(&bs, renc),
                                None => return err(c, c.vm().c_range_error.get(), if en::is_unicode(renc) && n > 0x10ffff || en::is_single_byte(renc) { format!("{} out of char range", n) } else { format!("invalid character") }),
                            }
                        }
                    }
                };
                // result encoding negotiation
                let (senc, mut bb) = { let r = c.rstring(sv); (r.enc, r.b.to_vec()) };
                if senc != renc && !is_ascii(&bb) {
                    if !en::ascii_compat(senc) { return err(c, c.vm().c_compat_error.get(), format!("incompatible character encodings: {} and {}", en::enc_name_msg(renc), en::enc_name_msg(senc))); }
                    if is_ascii(&out) && is_ascii(&fmt[i..]) && is_ascii(&fmt[..start]) || out.is_empty() && is_ascii(&fmt) { renc = senc; }
                    else { return err(c, c.vm().c_compat_error.get(), format!("incompatible character encodings: {} and {}", en::enc_name_msg(renc), en::enc_name_msg(senc))); }
                }
                if let Some(p) = prec_o { if conv != b'c' { let off = char_offsets_of(&bb, senc); if p + 1 < off.len() { bb.truncate(off[p]); } } }
                let n = enc_strlen(&bb, senc);
                pad(&mut out, b"", &bb, n, w, flags, false);
            }
            b'd' | b'i' | b'u' => {
                let iv = int_arg(c, v)?;
                let s = if iv.is_fixnum() { iv.as_fix().to_string() } else { c.to_big(iv).to_string_radix(10) };
                let (neg, mut digits) = if let Some(r) = s.strip_prefix('-') { (true, r.to_string()) } else { (false, s) };
                let sign = if neg { "-" } else if flags & FPLUS != 0 { "+" } else if flags & FSPACE != 0 { " " } else { "" };
                if prec_o == Some(0) && digits == "0" { digits.clear(); }
                if let Some(p) = prec_o { if digits.len() < p { digits = format!("{}{}", "0".repeat(p - digits.len()), digits); } }
                let n = sign.len() + digits.len();
                pad(&mut out, sign.as_bytes(), digits.as_bytes(), n, w, flags, prec_o.is_none());
            }
            b'f' if is_integer(v) || crate::builtins::rational::is_rational(c, v) => fmt_exact_f(c, &mut out, v, prec_o, w, flags),
            b'f' | b'e' | b'E' | b'g' | b'G' | b'a' | b'A' => {
                let f = float_arg(c, v)?;
                let neg = f.is_sign_negative() && !f.is_nan();
                let sign = if neg { "-" } else if flags & FPLUS != 0 { "+" } else if flags & FSPACE != 0 { " " } else { "" };
                let (body, num) = if f.is_nan() { ("NaN".to_string(), false) } else if f.is_infinite() { ("Inf".to_string(), false) }
                    else if conv == b'a' || conv == b'A' { (fmt_hex_float(f.abs(), prec_o, conv == b'A', flags & FSHARP != 0), true) }
                    else { (fmt_float(f.abs(), conv, prec_o, flags & FSHARP != 0), true) };
                let n = sign.len() + body.len();
                if (conv == b'a' || conv == b'A') && num && flags & FZERO != 0 && flags & FMINUS == 0 && n < w {
                    // zeros go after the 0x prefix
                    out.extend_from_slice(sign.as_bytes()); out.extend_from_slice(&body.as_bytes()[..2]);
                    out.extend(std::iter::repeat(b'0').take(w - n)); out.extend_from_slice(&body.as_bytes()[2..]);
                } else { pad(&mut out, sign.as_bytes(), body.as_bytes(), n, w, flags, num); }
            }
            _ => {
                // x X o b B
                let iv = int_arg(c, v)?;
                let radix: u32 = match conv { b'x' | b'X' => 16, b'o' => 8, _ => 2 };
                // non-negative fixnums (the common case) skip the bignum conversion
                let small = if iv.is_fixnum() && iv.as_fix() >= 0 { Some(iv.as_fix() as u64) } else { None };
                let big = if small.is_none() { c.to_big(iv) } else { crate::bignum::BigInt::from_i64(0) };
                let (bneg, bzero) = if small.is_some() { (false, small == Some(0)) } else { (big.neg, big.is_zero()) };
                let dots = bneg && flags & (FPLUS | FSPACE) == 0;
                let mut digits = if let Some(n) = small {
                    match radix { 16 => format!("{:x}", n), 8 => format!("{:o}", n), _ => format!("{:b}", n) }
                } else if dots {
                    let per = match radix { 16 => 4, 8 => 3, _ => 1 };
                    let nb = (big.bit_length() / per + 2) as u64;
                    let m = crate::bignum::BigInt::from_i64(radix as i64).pow(nb);
                    let tw = m.add(&big).to_string_radix(radix);
                    let fch = std::char::from_digit(radix - 1, radix).unwrap();
                    let t = tw.trim_start_matches(fch).to_string();
                    format!("{}{}", fch, t)
                } else { big.abs().to_string_radix(radix) };
                if prec_o == Some(0) && bzero { digits.clear(); }
                let fill = if dots { std::char::from_digit(radix - 1, radix).unwrap() } else { '0' };
                if let Some(p) = prec_o { if digits.len() < p { digits = format!("{}{}", fill.to_string().repeat(p - digits.len()), digits); } }
                else if dots && flags & FZERO != 0 && flags & FMINUS == 0 && w > digits.len() + 2 { digits = format!("{}{}", fill.to_string().repeat(w - digits.len() - 2), digits); }
                if conv == b'X' { digits = digits.to_uppercase(); }
                let mut sign = String::from(if bneg && !dots { "-" } else if flags & FPLUS != 0 { "+" } else if flags & FSPACE != 0 { " " } else { "" });
                if flags & FSHARP != 0 {
                    // '#o' makes the first digit a zero (none for "..7"); x/b prefixes only a non-zero value
                    let pre = match conv { b'o' => if dots || digits.starts_with('0') { "" } else { "0" }, _ if bzero => "", b'x' => "0x", b'X' => "0X", b'b' => "0b", _ => "0B" };
                    sign.push_str(pre);
                }
                if dots { sign.push_str(".."); }
                let n = sign.len() + digits.len();
                pad(&mut out, sign.as_bytes(), digits.as_bytes(), n, w, flags, prec_o.is_none() && !dots);
            }
        }
    }
    if st.posarg >= 0 && st.next < args.len() && !(args.len() == 1 && is_hash(args[0])) {
        let dbg = gvar_get(c, intern(b"$DEBUG"))?;
        if dbg.truthy() { return arg_err(c, "too many arguments for format string"); }
        if c.vm().verbose.get() == TRUE {
            let loc = backtrace_lines(c, 0).first().map(|l| l.split(":in ").next().unwrap_or("").to_string()).unwrap_or_default();
            let m = c.str_new(format!("{}: warning: too many arguments for format string\n", loc).as_bytes());
            let w = c.vm().m_warning.get();
            funcall(c, w, intern(b"warn"), &[m], std::ptr::null())?;
        }
    }
    Ok(c.str_from_vec_enc(out, renc))
}

/// %f of an Integer or Rational: exact decimal digits (sprintf.c's 'f' case, '#' has no effect).
fn fmt_exact_f(c: &mut Ctx, out: &mut Vec<u8>, v: Value, prec_o: Option<usize>, width: usize, flags: u32) {
    use crate::builtins::rational as r;
    let prec = prec_o.unwrap_or(6);
    let (num, den) = if is_integer(v) { (v, Value::fix(1)) } else { r::rat_parts(c, v) };
    let mut sign: i32 = if flags & FPLUS != 0 { 1 } else { 0 };
    let mut num = num;
    if r::isneg(c, num) { sign = -1; num = r::ineg(c, num); }
    let mut zero = 0;
    if den != Value::fix(1) {
        let p = r::ipow(c, Value::fix(10), prec as u64);
        num = r::imul(c, num, p);
        let h = r::idiv(c, den, Value::fix(2));
        num = r::iadd(c, num, h);
        num = r::idiv(c, num, den);
    } else { zero = prec; }
    let t = r::istr(c, num).into_bytes();
    let mut len = t.len() + zero;
    if prec >= len { len = prec + 1; }
    if sign != 0 || flags & FSPACE != 0 { len += 1; }
    if prec > 0 { len += 1; }
    let fill = width.saturating_sub(len);
    if fill > 0 && flags & (FMINUS | FZERO) == 0 { out.extend(std::iter::repeat(b' ').take(fill)); }
    if sign != 0 || flags & FSPACE != 0 { out.push(if sign > 0 { b'+' } else if sign < 0 { b'-' } else { b' ' }); }
    if fill > 0 && flags & (FMINUS | FZERO) == FZERO { out.extend(std::iter::repeat(b'0').take(fill)); }
    let len = t.len() + zero;
    if len > prec { out.extend_from_slice(&t[..len - prec]); } else { out.push(b'0'); }
    if prec > 0 { out.push(b'.'); }
    if zero > 0 { out.extend(std::iter::repeat(b'0').take(zero)); }
    else if prec > len { out.extend(std::iter::repeat(b'0').take(prec - len)); out.extend_from_slice(&t); }
    else if prec > 0 { out.extend_from_slice(&t[len - prec..]); }
    if fill > 0 && flags & FMINUS != 0 { out.extend(std::iter::repeat(b' ').take(fill)); }
}

/// Raise `klass` with a message made of raw bytes in encoding `enc`.
fn raise_bytes(c: &mut Ctx, klass: Value, msg: &[u8], enc: u8) -> Esc {
    let m = c.str_new_enc(msg, enc);
    match funcall(c, klass, intern(b"new"), &[m], std::ptr::null()) { Ok(e) => raise_exc(c, e), Err(e) => e }
}

/// `*` or `*N$` argument.
fn aster(c: &mut Ctx, fmt: &[u8], i: &mut usize, st: &mut Args) -> RV {
    let s0 = *i; let mut j = *i;
    while j < fmt.len() && fmt[j].is_ascii_digit() { j += 1; }
    if j > s0 && j < fmt.len() && fmt[j] == b'$' {
        let n: i64 = std::str::from_utf8(&fmt[s0..j]).unwrap().parse().unwrap_or(0);
        *i = j + 1;
        return st.pos_arg(c, n);
    }
    st.next_arg(c)
}

/// Write sign+body padded to width `w` (n = display width of sign+body).
fn pad(out: &mut Vec<u8>, sign: &[u8], body: &[u8], n: usize, w: usize, flags: u32, zero_ok: bool) {
    if n >= w { out.extend_from_slice(sign); out.extend_from_slice(body); return; }
    let fill = w - n;
    if flags & FMINUS != 0 { out.extend_from_slice(sign); out.extend_from_slice(body); out.extend(std::iter::repeat(b' ').take(fill)); }
    else if flags & FZERO != 0 && zero_ok { out.extend_from_slice(sign); out.extend(std::iter::repeat(b'0').take(fill)); out.extend_from_slice(body); }
    else { out.extend(std::iter::repeat(b' ').take(fill)); out.extend_from_slice(sign); out.extend_from_slice(body); }
}

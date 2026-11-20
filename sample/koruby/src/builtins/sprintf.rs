// Kernel#format / String#%.
use crate::builtins::kernel::*;
use crate::builtins::numeric_methods::{num_to_long, to_f, to_integer};
use crate::object::*;
use crate::string::*;
use crate::symbol::intern;
use crate::value::*;
use crate::vm::*;

fn fmt_float(f: f64, conv: u8, prec: Option<usize>, alt: bool) -> String {
    if f.is_nan() { return "NaN".into(); }
    if f.is_infinite() { return if f > 0.0 { "Inf".into() } else { "-Inf".into() }; }
    let p = prec.unwrap_or(6);
    match conv {
        b'f' => format!("{:.*}", p, f),
        b'e' | b'E' => {
            let s = format!("{:.*e}", p, f);
            let (m, e) = s.split_once('e').unwrap();
            let ei: i32 = e.parse().unwrap();
            let r = format!("{}e{}{:02}", m, if ei < 0 { '-' } else { '+' }, ei.abs());
            if conv == b'E' { r.to_uppercase() } else { r }
        }
        b'g' | b'G' => {
            let p = if p == 0 { 1 } else { p };
            if f == 0.0 { return if alt { format!("{:.*}", p - 1, 0.0) } else { "0".into() }; }
            let exp = f.abs().log10().floor() as i32;
            let s = if exp < -4 || exp >= p as i32 {
                let s = format!("{:.*e}", p - 1, f);
                let (m, e) = s.split_once('e').unwrap();
                let m = if alt { m.to_string() } else if m.contains('.') { m.trim_end_matches('0').trim_end_matches('.').to_string() } else { m.to_string() };
                let ei: i32 = e.parse().unwrap();
                format!("{}e{}{:02}", m, if ei < 0 { '-' } else { '+' }, ei.abs())
            } else {
                let dec = (p as i32 - 1 - exp).max(0) as usize;
                let s = format!("{:.*}", dec, f);
                if alt { s } else if s.contains('.') { s.trim_end_matches('0').trim_end_matches('.').to_string() } else { s }
            };
            if conv == b'G' { s.to_uppercase() } else { s }
        }
        b'a' | b'A' => format!("{}", f),
        _ => format!("{}", f),
    }
}

pub fn format(c: &mut Ctx, fmtv: Value, args: &[Value]) -> RV {
    let fmts = expect_str(c, fmtv)?;
    let fmt = rstr(fmts).b.clone();
    let mut out: Vec<u8> = vec![];
    let mut ai = 0usize;
    let mut i = 0;
    let hash_arg = args.last().copied().filter(|v| is_hash(*v));
    let next_arg = |c: &mut Ctx, ai: &mut usize| -> RV {
        if *ai >= args.len() { return arg_err(c, "too few arguments"); }
        let v = args[*ai]; *ai += 1; Ok(v)
    };
    while i < fmt.len() {
        let ch = fmt[i];
        if ch != b'%' { out.push(ch); i += 1; continue; }
        i += 1;
        if i >= fmt.len() { return arg_err(c, "incomplete format specifier; use %% (double %) instead"); }
        if fmt[i] == b'%' { out.push(b'%'); i += 1; continue; }
        let (mut minus, mut plus, mut space, mut zero, mut alt) = (false, false, false, false, false);
        let mut width: Option<usize> = None; let mut prec: Option<usize> = None;
        let mut named: Option<Value> = None;
        let mut pos_arg: Option<usize> = None;
        loop {
            if i >= fmt.len() { break; }
            match fmt[i] {
                b'-' => { minus = true; i += 1; }
                b'+' => { plus = true; i += 1; }
                b' ' => { space = true; i += 1; }
                b'0' => { zero = true; i += 1; }
                b'#' => { alt = true; i += 1; }
                b'<' | b'{' => {
                    let close = if fmt[i] == b'<' { b'>' } else { b'}' };
                    let st = i + 1;
                    let e = fmt[st..].iter().position(|&x| x == close).map(|p| st + p).unwrap_or(fmt.len());
                    let key = Value::sym(intern(&fmt[st..e]));
                    let h = match hash_arg { Some(h) => h, None => return arg_err(c, "one hash required") };
                    let v = match crate::hash::hash_get_opt(c, h, key)? { Some(v) => v, None => {
                        if rhash(h).default_proc != NIL || rhash(h).default != NIL { crate::hash::hash_aref(c, h, key)? } else {
                            let e2 = make_exception(vm().c_key_error, &format!("key<{}> not found", String::from_utf8_lossy(&fmt[st..e])));
                            ivar_set_raw(e2, crate::symbol::s::key, key); ivar_set_raw(e2, crate::symbol::s::receiver, h);
                            return Err(raise_exc(c, e2));
                        }
                    } };
                    if close == b'}' {
                        let sv = crate::rt::rt_tostr(c, v)?; out.extend_from_slice(str_bytes(sv)); i = e + 1; named = Some(UNDEF); break;
                    }
                    named = Some(v); i = e + 1;
                }
                b'*' => {
                    i += 1;
                    let w = next_arg(c, &mut ai)?; let w = num_to_long(c, w)?;
                    if w < 0 { minus = true; width = Some((-w) as usize); } else { width = Some(w as usize); }
                }
                x if x.is_ascii_digit() => {
                    let st = i; while i < fmt.len() && fmt[i].is_ascii_digit() { i += 1; }
                    let n: usize = std::str::from_utf8(&fmt[st..i]).unwrap().parse().unwrap_or(0);
                    if i < fmt.len() && fmt[i] == b'$' { pos_arg = Some(n); i += 1; } else { width = Some(n); }
                }
                b'.' => {
                    i += 1;
                    if i < fmt.len() && fmt[i] == b'*' { i += 1; let p = next_arg(c, &mut ai)?; prec = Some(num_to_long(c, p)?.max(0) as usize); }
                    else { let st = i; while i < fmt.len() && fmt[i].is_ascii_digit() { i += 1; } prec = Some(std::str::from_utf8(&fmt[st..i]).unwrap().parse().unwrap_or(0)); }
                }
                _ => break,
            }
        }
        if named == Some(UNDEF) { continue; }
        if i >= fmt.len() {
            if named.is_some() { let v = named.unwrap(); let sv = crate::rt::rt_tostr(c, v)?; out.extend_from_slice(str_bytes(sv)); continue; }
            return arg_err(c, "incomplete format specifier; use %% (double %) instead");
        }
        let conv = fmt[i]; i += 1;
        let mut get = |c: &mut Ctx| -> RV {
            if let Some(v) = named { return Ok(v); }
            if let Some(p) = pos_arg { return args.get(p - 1).copied().ok_or(()).or_else(|_| arg_err(c, "too few arguments")); }
            next_arg(c, &mut ai)
        };
        let body: Vec<u8>;
        let mut sign = String::new();
        let mut numeric = false;
        match conv {
            b'd' | b'i' | b'u' => {
                let v = get(c)?;
                let iv = if is_float(v) { crate::builtins::numeric_methods::f64_to_int(float_val(v).floor()) } else if is_string(v) { crate::builtins::kernel::parse_int_for_format(c, v)? } else { to_integer(c, v)? };
                let s = crate::bignum::to_big(iv).to_string_radix(10);
                let (neg, digits) = if let Some(r) = s.strip_prefix('-') { (true, r.to_string()) } else { (false, s) };
                sign = if neg { "-".into() } else if plus { "+".into() } else if space { " ".into() } else { String::new() };
                let digits = match prec { Some(p) if digits.len() < p => format!("{}{}", "0".repeat(p - digits.len()), digits), _ => digits };
                body = digits.into_bytes(); numeric = true;
            }
            b'f' | b'e' | b'E' | b'g' | b'G' | b'a' | b'A' => {
                let v = get(c)?;
                let f = to_f(c, v)?;
                let s = fmt_float(f.abs(), conv, prec, alt);
                let neg = f.is_sign_negative() && !(f == 0.0 && !f.is_sign_negative()) && !f.is_nan();
                let neg = neg && !(f == 0.0 && s.chars().all(|c| c == '0' || c == '.')) || (f.is_sign_negative() && f != 0.0) || (f == 0.0 && f.is_sign_negative());
                sign = if neg { "-".into() } else if plus { "+".into() } else if space { " ".into() } else { String::new() };
                if f.is_infinite() || f.is_nan() { zero = false; }
                body = s.into_bytes(); numeric = true;
            }
            b'x' | b'X' | b'o' | b'b' | b'B' => {
                let v = get(c)?;
                let iv = if is_float(v) { crate::builtins::numeric_methods::f64_to_int(float_val(v).floor()) } else { to_integer(c, v)? };
                let radix = match conv { b'x' | b'X' => 16, b'o' => 8, _ => 2 };
                let big = crate::bignum::to_big(iv);
                let mut digits = if big.neg && !(plus || space) {
                    // two's complement with ".." prefix
                    let nb = (big.bit_length() / (match radix { 16 => 4, 8 => 3, _ => 1 }) + 2) as u32;
                    let m = crate::bignum::BigInt::from_i64(radix as i64).pow(nb as u64);
                    let tw = m.add(&big).to_string_radix(radix);
                    let f = std::char::from_digit(radix - 1, radix).unwrap();
                    let t = tw.trim_start_matches(f).to_string();
                    format!("..{}{}", f, t)
                } else { big.abs().to_string_radix(radix) };
                if conv == b'X' { digits = digits.to_uppercase(); }
                sign = if big.neg && (plus || space) { "-".into() } else if plus { "+".into() } else if space { " ".into() } else { String::new() };
                if let Some(p) = prec { if digits.len() < p { digits = format!("{}{}", "0".repeat(p - digits.len()), digits); } }
                if alt && !big.is_zero() { let pre = match conv { b'x' => "0x", b'X' => "0X", b'o' => "0", b'b' => "0b", _ => "0B" }; sign.push_str(pre); }
                body = digits.into_bytes(); numeric = true;
            }
            b's' | b'p' => {
                let v = get(c)?;
                let sv = if conv == b'p' { inspect_value(c, v)? } else { crate::rt::rt_tostr(c, v)? };
                let mut bb = str_bytes(sv).to_vec();
                if let Some(p) = prec { let chars = chars_of(sv); bb = chars.into_iter().take(p).flatten().collect(); }
                body = bb;
            }
            b'c' => {
                let v = get(c)?;
                body = if is_string(v) { chars_of(v).into_iter().next().unwrap_or_default() } else { let n = num_to_long(c, v)?; let mut b2 = vec![]; encode_utf8(n as u32, &mut b2); b2 };
            }
            _ => {
                return arg_err(c, format!("malformed format string - %{}", conv as char));
            }
        }
        let blen = if numeric { body.len() } else { utf8_len(&body) };
        let total = sign.len() + blen;
        let w = width.unwrap_or(0);
        if total >= w { out.extend_from_slice(sign.as_bytes()); out.extend_from_slice(&body); }
        else if minus { out.extend_from_slice(sign.as_bytes()); out.extend_from_slice(&body); out.extend(std::iter::repeat(b' ').take(w - total)); }
        else if zero && numeric && !(prec.is_some() && matches!(conv, b'd' | b'i' | b'x' | b'X' | b'o' | b'b')) { out.extend_from_slice(sign.as_bytes()); out.extend(std::iter::repeat(b'0').take(w - total)); out.extend_from_slice(&body); }
        else { out.extend(std::iter::repeat(b' ').take(w - total)); out.extend_from_slice(sign.as_bytes()); out.extend_from_slice(&body); }
    }
    if ai < args.len() && hash_arg.is_none() && vm().debug_flag { return arg_err(c, "too many arguments for format string"); }
    Ok(str_from_vec_enc(out, rstr(fmts).enc))
}

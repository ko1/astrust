// Marshal (format 4.8): dump / load.
// On the context-borrowing heap accessors (heap.rs, docs/safe_heap.md).
#![forbid(unsafe_code)]
#[allow(unused_imports)]
use crate::heap::no_legacy::*;   // the old accessors cannot be named here
#[allow(unused_imports)]
use crate::heap::no_legacy_hn::*;
#[allow(unused_imports)]
use crate::heap::no_legacy_str::*;
use crate::heap::{obj_klass, obj_set_flags, obj_set_klass};
use crate::array::*;
use crate::bignum::BigInt;
use crate::builtins::numeric_methods::num_to_long;
use crate::fxhash::HashMap;
use crate::hash::*;
use crate::string::*;
use crate::symbol::{intern, s, sym_bytes};
use crate::value::*;
use crate::vm::*;

struct Dumper { out: Vec<u8>, syms: HashMap<Sym, usize>, objs: HashMap<u64, usize>, keep: Value }

fn w_long(out: &mut Vec<u8>, x: i64) {
    if x == 0 { out.push(0); }
    else if 0 < x && x < 123 { out.push((x + 5) as u8); }
    else if -124 < x && x < 0 { out.push(((x - 5) & 0xff) as u8); }
    else {
        let mut buf = vec![];
        let mut v = x;
        for _ in 0..4 {
            buf.push((v & 0xff) as u8);
            v >>= 8;
            if v == 0 || v == -1 { break; }
        }
        out.push(if x < 0 { (-(buf.len() as i64)) as u8 } else { buf.len() as u8 });
        out.extend(buf);
    }
}
fn w_bytes(out: &mut Vec<u8>, b: &[u8]) { w_long(out, b.len() as i64); out.extend_from_slice(b); }
/// CRuby's shortest round-trip float text (flo_to_s without the trailing ".0").
fn float_repr(f: f64) -> String {
    if f.is_nan() { return "nan".into(); }
    if f.is_infinite() { return if f < 0.0 { "-inf".into() } else { "inf".into() }; }
    if f == 0.0 { return if f.is_sign_negative() { "-0".into() } else { "0".into() }; }
    let e = format!("{:e}", f.abs());
    let (mant, exp) = e.split_once('e').unwrap();
    let digits: String = mant.chars().filter(|c| *c != '.').collect();
    let decpt = exp.parse::<i32>().unwrap() + 1;
    let n = digits.len() as i32;
    let body = if decpt < -3 || decpt > n {
        format!("{}{}{}e{}", &digits[..1], if n > 1 { "." } else { "" }, &digits[1..], decpt - 1)
    } else if decpt > 0 {
        if n <= decpt { format!("{}{}", digits, "0".repeat((decpt - n) as usize)) } else { format!("{}.{}", &digits[..decpt as usize], &digits[decpt as usize..]) }
    } else { format!("0.{}{}", "0".repeat((-decpt) as usize), digits) };
    if f < 0.0 { format!("-{}", body) } else { body }
}

impl Dumper {
    fn sym(&mut self, sy: Sym) {
        if let Some(&i) = self.syms.get(&sy) { self.out.push(b';'); w_long(&mut self.out, i as i64); return; }
        let n = self.syms.len();
        self.syms.insert(sy, n);
        let b = sym_bytes(sy);
        let ascii = is_ascii(b);
        if !ascii { self.out.push(b'I'); }
        self.out.push(b':');
        w_bytes(&mut self.out, b);
        if !ascii { w_long(&mut self.out, 1); self.sym(intern(b"E")); self.out.push(b'T'); }
    }
    fn class_name(&mut self, c: &mut Ctx, k: Value) -> Result<String, Esc> {
        if c.class(k).name.is_none() { let a = any_to_s_anon(c, k); return type_err(c, format!("can't dump anonymous {} {}", if c.class(k).is_module { "module" } else { "class" }, a)); }
        Ok(class_path(k))
    }
    fn uniq_class(&mut self, c: &mut Ctx, k: Value) -> Result<(), Esc> { let n = self.class_name(c, k)?; self.sym(intern(n.as_bytes())); Ok(()) }
    /// 'e' markers for modules a singleton class includes; a singleton with state can't be dumped.
    fn extended(&mut self, c: &mut Ctx, v: Value, check: bool) -> Result<(), Esc> {
        let k = obj_klass(v);
        if k == NIL || !c.class(k).is_singleton { return Ok(()); }
        if check && (!c.class(k).mtbl.is_empty() || !c.class(k).consts.is_empty() || ivar_list(k).iter().any(|(n, _)| sym_bytes(*n).starts_with(b"@"))) { return type_err(c, "singleton can't be dumped"); }
        let mods: Vec<Value> = c.class(k).includes.iter().rev().copied().collect();
        for m in mods { self.out.push(b'e'); self.uniq_class(c, m)?; }
        Ok(())
    }
    fn class_of_obj(&mut self, c: &mut Ctx, v: Value, tag: u8, check: bool) -> Result<(), Esc> {
        self.extended(c, v, check)?;
        self.out.push(tag);
        self.uniq_class(c, real_class_of(v))
    }
    /// 'C' marker when v is an instance of a subclass of the built-in class `base`.
    fn uclass(&mut self, c: &mut Ctx, v: Value, base: Value) -> Result<(), Esc> {
        self.extended(c, v, true)?;
        let k = real_class_of(v);
        if k != base { self.out.push(b'C'); self.uniq_class(c, k)?; }
        Ok(())
    }
    fn w_enc(&mut self, enc: u8) {
        match enc {
            ENC_BINARY => {}
            ENC_UTF8 => { self.sym(intern(b"E")); self.out.push(b'T'); }
            ENC_ASCII => { self.sym(intern(b"E")); self.out.push(b'F'); }
            e => { self.sym(intern(b"encoding")); self.out.push(b'"'); w_bytes(&mut self.out, crate::builtins::encoding::enc_name(e).as_bytes()); }
        }
    }
    /// The ivars Marshal writes; their values are kept alive in `keep` (a hook may drop them from `v`).
    fn user_ivars(&mut self, c: &mut Ctx, v: Value) -> Vec<(Sym, Value)> {
        if !v.is_heap() || is_class_or_module(v) { return vec![]; }
        let exc = is_exception(v);
        let mut l: Vec<(Sym, Value)> = ivar_list(v).into_iter().filter(|(n, _)| { let b = sym_bytes(*n); b.starts_with(b"@") || (exc && b == b"mesg") }).collect();
        if exc {
            if !l.iter().any(|(n, _)| sym_bytes(*n) == b"mesg") { l.insert(0, (intern(b"mesg"), NIL)); }
            let bt = ivar_get(v, intern(b"bt"));
            let at = l.iter().position(|(n, _)| sym_bytes(*n) == b"mesg").unwrap() + 1;
            l.insert(at, (intern(b"bt"), bt));
        }
        for &(_, x) in &l { if x.is_heap() { c.ary_push(self.keep, x); } }
        l
    }
    fn w_ivars(&mut self, c: &mut Ctx, enc: Option<u8>, ivs: &[(Sym, Value)], limit: i64) -> Result<(), Esc> {
        let ne = match enc { Some(e) if e != ENC_BINARY => 1, _ => 0 };
        w_long(&mut self.out, (ivs.len() + ne) as i64);
        if let Some(e) = enc { self.w_enc(e); }
        for &(n, val) in ivs { self.sym(n); self.obj(c, val, limit)?; }
        Ok(())
    }
    fn obj(&mut self, c: &mut Ctx, v: Value, limit: i64) -> Result<(), Esc> {
        if limit == 0 { return arg_err(c, "exceed depth limit"); }
        if v == NIL { self.out.push(b'0'); return Ok(()); }
        if v == TRUE { self.out.push(b'T'); return Ok(()); }
        if v == FALSE { self.out.push(b'F'); return Ok(()); }
        if v.is_fixnum() {
            let n = v.as_fix();
            if (-(1 << 30)..(1 << 30)).contains(&n) { self.out.push(b'i'); w_long(&mut self.out, n); return Ok(()); }
            let id = self.objs.len();
            self.objs.insert(u64::MAX - id as u64, id);
            bignum(&mut self.out, &BigInt::from_i64(n));
            return Ok(());
        }
        if v.is_sym() { self.sym(v.as_sym()); return Ok(()); }
        if let Some(&i) = self.objs.get(&v.raw()) { self.out.push(b'@'); w_long(&mut self.out, i as i64); return Ok(()); }
        let limit = limit - 1;
        let id = self.objs.len();
        self.objs.insert(v.raw(), id);
        if is_float(v) { self.out.push(b'f'); w_bytes(&mut self.out, float_repr(float_val(v)).as_bytes()); return Ok(()); }
        if crate::builtins::inherits(real_class_of(v), c.vm().c_time.get()) && crate::builtins::time::time_to_ns(c, v).is_some() { return self.time(c, v, limit); }
        // user hooks first
        if respond_to(v, intern(b"marshal_dump"), true) && !is_class_or_module(v) {
            let d = funcall(c, v, intern(b"marshal_dump"), &[], std::ptr::null())?;
            c.ary_push(self.keep, d);
            self.class_of_obj(c, v, b'U', false)?;
            return self.obj(c, d, limit);
        }
        if respond_to(v, intern(b"_dump"), true) && !is_class_or_module(v) {
            let d = funcall(c, v, intern(b"_dump"), &[Value::fix(limit)], std::ptr::null())?;
            if !is_string(d) { return type_err(c, "_dump() must return string"); }
            c.ary_push(self.keep, d);
            let ivs = self.user_ivars(c, d);
            let hasiv = !ivs.is_empty() || c.str_enc(d) != ENC_BINARY;
            if hasiv { self.out.push(b'I'); }
            self.class_of_obj(c, v, b'u', false)?;
            w_bytes(&mut self.out, c.str_bytes(d));
            if hasiv { let e = c.str_enc(d); self.w_ivars(c, Some(e), &ivs, limit)?; }
            return Ok(());
        }
        let ivs = self.user_ivars(c, v);
        if is_string(v) {
            let enc = c.str_enc(v);
            let hasiv = !ivs.is_empty() || enc != ENC_BINARY;
            if hasiv { self.out.push(b'I'); }
            let k = c.vm().c_string.get();
            self.uclass(c, v, k)?;
            self.out.push(b'"');
            w_bytes(&mut self.out, c.str_bytes(v));
            if hasiv { self.w_ivars(c, Some(enc), &ivs, limit)?; }
            return Ok(());
        }
        if let Some(r) = crate::regexp::rregexp(c, v) {
            let (src, opts) = (r.src.clone(), r.opts);
            let enc = if is_ascii(&src) && !r.enc_fixed { ENC_ASCII } else { ENC_UTF8 };
            self.out.push(b'I');
            let k = c.vm().c_regexp.get();
            self.uclass(c, v, k)?;
            self.out.push(b'/');
            w_bytes(&mut self.out, &src);
            self.out.push((opts & 0xff) as u8);
            return self.w_ivars(c, Some(enc), &ivs, limit);
        }
        // the kind and the fields needed, by value: the arms below call out
        enum K { Big, Class(bool, bool), Ary, Hash(Value, Value, bool), Range(Value, Value, bool), Obj, Data, Other }
        let kind = match c.body_of(v) {
            Body::Bignum(_) => K::Big, Body::Class(cd) => K::Class(cd.is_singleton, cd.is_module), Body::Array(_) => K::Ary,
            Body::Hash(h) => K::Hash(h.default_proc, h.default, h.identity), Body::Range(b, e, x) => K::Range(*b, *e, *x),
            Body::Object(_) => K::Obj, Body::Data(_) => K::Data, _ => K::Other,
        };
        let has_iv_wrapper = !ivs.is_empty() && !matches!(kind, K::Obj);
        if has_iv_wrapper { self.out.push(b'I'); }
        match kind {
            K::Big => bignum(&mut self.out, c.big(v)),
            K::Class(singleton, m) => {
                if singleton { return type_err(c, "singleton class can't be dumped"); }
                let n = self.class_name(c, v)?;
                self.out.push(if m { b'm' } else { b'c' });
                w_bytes(&mut self.out, n.as_bytes());
            }
            K::Ary => {
                let k = c.vm().c_array.get();
                self.uclass(c, v, k)?;
                self.out.push(b'[');
                w_long(&mut self.out, c.ary_len(v) as i64);
                let mut i = 0;
                while let Some(&x) = c.ary(v).get(i) { self.obj(c, x, limit)?; i += 1; }
            }
            K::Hash(default_proc, def, identity) => {
                if default_proc != NIL { return type_err(c, "can't dump hash with default proc"); }
                let kh = c.vm().c_hash.get();
                if identity && real_class_of(v) == kh { self.out.push(b'C'); self.uniq_class(c, kh)?; } else { self.uclass(c, v, kh)?; }
                self.out.push(if def == NIL { b'{' } else { b'}' });
                w_long(&mut self.out, c.hash_len(v) as i64);
                each_pair(c, v, |c, k, x| { self.obj(c, k, limit)?; self.obj(c, x, limit) })?;
                if def != NIL { self.obj(c, def, limit)?; }
            }
            K::Range(b, e, excl) => {
                self.class_of_obj(c, v, b'o', true)?;
                w_long(&mut self.out, 3);
                self.sym(intern(b"excl")); self.out.push(if excl { b'T' } else { b'F' });
                self.sym(intern(b"begin")); self.obj(c, b, limit)?;
                self.sym(intern(b"end")); self.obj(c, e, limit)?;
            }
            K::Obj => {
                let k = real_class_of(v);
                let (ks, kd) = (c.vm().c_struct.get(), c.vm().c_data.get());
                if crate::builtins::inherits(k, ks) || (kd != NIL && crate::builtins::inherits(k, kd)) {
                    let mem = funcall(c, k, intern(b"members"), &[], std::ptr::null())?;
                    if !ivs.is_empty() { self.out.push(b'I'); }
                    self.class_of_obj(c, v, b'S', true)?;
                    w_long(&mut self.out, c.ary_len(mem) as i64);
                    let mut i = 0;
                    while let Some(&m) = c.ary(mem).get(i) { let m = m.as_sym(); self.sym(m); let x = struct_get(v, m); self.obj(c, x, limit)?; i += 1; }
                    if !ivs.is_empty() { self.w_ivars(c, None, &ivs, limit)?; }
                } else {
                    self.class_of_obj(c, v, b'o', true)?;
                    self.w_ivars(c, None, &ivs, limit)?;
                }
                return Ok(());
            }
            K::Data => {
                if crate::builtins::rational::is_rational(c, v) {
                    let (n, d) = crate::builtins::rational::rat_parts(c, v);
                    self.class_of_obj(c, v, b'U', false)?;
                    let k = self.objs.len(); self.objs.insert(u64::MAX - k as u64, k);
                    self.out.push(b'['); w_long(&mut self.out, 2); self.obj(c, n, limit)?; self.obj(c, d, limit)?;
                } else if crate::builtins::complex::is_complex(c, v) {
                    let (r, i) = crate::builtins::complex::cpx_parts(c, v);
                    self.class_of_obj(c, v, b'U', false)?;
                    let k = self.objs.len(); self.objs.insert(u64::MAX - k as u64, k);
                    self.out.push(b'['); w_long(&mut self.out, 2); self.obj(c, r, limit)?; self.obj(c, i, limit)?;
                } else {
                    let k = real_class_of(v);
                    if crate::builtins::io::is_io(c, v) { return type_err(c, format!("can't dump {}", class_path(k))); }
                    return type_err(c, format!("no _dump_data is defined for class {}", class_path(k)));
                }
            }
            K::Other => { let k = real_class_of(v); return type_err(c, format!("no _dump_data is defined for class {}", class_path(k))); }
        }
        if has_iv_wrapper { self.w_ivars(c, None, &ivs, limit)?; }
        Ok(())
    }
    /// Time#_dump layout: two packed little-endian words plus ivars for the sub-microsecond part / zone.
    fn time(&mut self, c: &mut Ctx, v: Value, limit: i64) -> Result<(), Esc> {
        let call = |c: &mut Ctx, o: Value, m: &str| funcall(c, o, intern(m.as_bytes()), &[], std::ptr::null());
        let utc = call(c, v, "utc?")?.truthy();
        let g = call(c, v, "getutc")?;
        let mut f = [0i64; 6];
        for (i, m) in ["year", "mon", "mday", "hour", "min", "sec"].iter().enumerate() { let x = call(c, g, m)?; f[i] = num_to_long(c, x)?; }
        let nsec = { let x = call(c, v, "nsec")?; num_to_long(c, x)? };
        let (usec, nano) = (nsec / 1000, nsec % 1000);
        if !(1900..=1900 + 0xffff).contains(&f[0]) { return arg_err(c, format!("year too {} to marshal: {} UTC", if f[0] < 1900 { "small" } else { "big" }, f[0])); }
        let p: u32 = 1 << 31 | (utc as u32) << 30 | ((f[0] - 1900) as u32) << 14 | ((f[1] - 1) as u32) << 10 | (f[2] as u32) << 5 | f[3] as u32;
        let sw: u32 = (f[4] as u32) << 26 | (f[5] as u32) << 20 | usec as u32;
        let zone = if utc { c.str_new_enc(b"UTC", ENC_ASCII) } else { call(c, v, "zone")? };
        c.ary_push(self.keep, zone);
        let off = if utc { NIL } else { call(c, v, "utc_offset")? };
        self.out.push(b'I');
        self.class_of_obj(c, v, b'u', false)?;
        w_long(&mut self.out, 8);
        self.out.extend_from_slice(&p.to_le_bytes()); self.out.extend_from_slice(&sw.to_le_bytes());
        let mut ivs = self.user_ivars(c, v);
        if nano != 0 {
            ivs.push((intern(b"nano_num"), Value::fix(nano))); ivs.push((intern(b"nano_den"), Value::fix(1)));
            let b2 = ((nano % 10) << 4) as u8;
            let mut sm = vec![(((nano / 100) << 4) | (nano / 10 % 10)) as u8];
            if b2 != 0 { sm.push(b2); }
            let smv = c.str_from_vec_enc(sm, ENC_BINARY);
            c.ary_push(self.keep, smv);
            ivs.push((intern(b"submicro"), smv));
        }
        if !utc { ivs.push((intern(b"offset"), off)); }
        let zv = if zone != NIL && is_string(zone) { let z = c.str_copy_enc(zone, ENC_ASCII); c.ary_push(self.keep, z); z } else { NIL };
        ivs.push((intern(b"zone"), zv));
        self.w_ivars(c, None, &ivs, limit)
    }
}
fn bignum(out: &mut Vec<u8>, b: &BigInt) {
    out.push(b'l');
    out.push(if b.neg { b'-' } else { b'+' });
    let mut bytes: Vec<u8> = b.mag.iter().flat_map(|w| w.to_le_bytes()).collect();
    while bytes.len() > 2 && bytes[bytes.len() - 1] == 0 && bytes[bytes.len() - 2] == 0 { bytes.truncate(bytes.len() - 2); }
    if bytes.len() % 2 == 1 { bytes.push(0); }
    w_long(out, (bytes.len() / 2) as i64);
    out.extend(bytes);
}
/// Each live pair of Hash h, slot by slot (the callback may run Ruby code that changes h); new
/// keys are refused meanwhile, as CRuby's rb_hash_foreach does.
pub fn each_pair(c: &mut Ctx, h: Value, mut f: impl FnMut(&mut Ctx, Value, Value) -> Result<(), Esc>) -> Result<(), Esc> {
    c.hash_iter_enter(h);
    let mut i = 0;
    let mut r = Ok(());
    while let Some((k, x)) = c.hash_entry_at(h, i) {
        i += 1;
        if k == UNDEF { continue; }
        r = f(c, k, x);
        if r.is_err() { break; }
    }
    c.hash_iter_leave(h);
    r
}
fn any_to_s_anon(c: &Ctx, k: Value) -> String { format!("#<{}:0x{:016x}>", if c.class(k).is_module { "Module" } else { "Class" }, k.raw()) }
fn struct_get(v: Value, m: Sym) -> Value { ivar_get(v, m) }

fn m_dump(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    check_argc(c, a, 1, 3)?;
    let mut io = NIL; let mut limit = -1;
    for &x in &a[1..] {
        if x.is_fixnum() { limit = x.as_fix(); } else if x == NIL { } else if respond_to(x, intern(b"write"), true) { io = x; } else { return type_err(c, "instance of IO needed"); }
    }
    let keep = c.ary_new();
    let mut d = Dumper { out: vec![4, 8], syms: HashMap::default(), objs: HashMap::default(), keep };
    d.obj(c, a[0], limit)?;
    let out = c.str_from_vec_enc(std::mem::take(&mut d.out), ENC_BINARY);
    if io != NIL {
        if respond_to(io, intern(b"binmode"), true) { funcall(c, io, intern(b"binmode"), &[], std::ptr::null())?; }
        funcall(c, io, intern(b"write"), &[out], std::ptr::null())?;
        return Ok(io);
    }
    Ok(out)
}

// ---- load -------------------------------------------------------------------------------------

// `data`: the source String's bytes, copied once (hooks run while reading may change the String)
struct Loader { src: Value, data: Vec<u8>, io: bool, pos: usize, syms: Vec<Sym>, objs: Value, proc_: Value, freeze: bool, symbolize: bool }
const TOO_SHORT: &str = "marshal data too short";

impl Loader {
    fn byte(&mut self, c: &mut Ctx) -> Result<u8, Esc> {
        if self.io {
            let v = funcall(c, self.src, intern(b"getbyte"), &[], std::ptr::null())?;
            if v == NIL { return err(c, c.vm().c_eof_error.get(), "end of file reached"); }
            return Ok(num_to_long(c, v)? as u8);
        }
        match self.data.get(self.pos) { Some(&b) => { self.pos += 1; Ok(b) } None => arg_err(c, TOO_SHORT) }
    }
    fn bytes(&mut self, c: &mut Ctx, n: usize) -> Result<Vec<u8>, Esc> {
        if n == 0 { return Ok(vec![]); }
        if self.io {
            let v = funcall(c, self.src, intern(b"read"), &[Value::fix(n as i64)], std::ptr::null())?;
            if v == NIL || !is_string(v) || c.str_bytes(v).len() != n { return arg_err(c, TOO_SHORT); }
            return Ok(c.str_bytes(v).to_vec());
        }
        if self.pos + n > self.data.len() { return arg_err(c, TOO_SHORT); }
        self.pos += n;
        Ok(self.data[self.pos - n..self.pos].to_vec())
    }
    fn long(&mut self, c: &mut Ctx) -> Result<i64, Esc> {
        let b = self.byte(c)? as i8 as i64;
        if b == 0 { return Ok(0); }
        if b > 0 {
            if 4 < b && b < 128 { return Ok(b - 5); }
            let mut x: i64 = 0;
            for i in 0..b { x |= (self.byte(c)? as i64) << (8 * i); }
            Ok(x)
        } else {
            if -129 < b && b < -4 { return Ok(b + 5); }
            let n = -b;
            let mut x: i64 = -1;
            for i in 0..n { x &= !(0xff << (8 * i)); x |= (self.byte(c)? as i64) << (8 * i); }
            Ok(x)
        }
    }
    fn len(&mut self, c: &mut Ctx) -> Result<usize, Esc> { let n = self.long(c)?; if n < 0 { return arg_err(c, "negative length"); } Ok(n as usize) }
    fn rbytes(&mut self, c: &mut Ctx) -> Result<Vec<u8>, Esc> { let n = self.len(c)?; self.bytes(c, n) }
    fn entry(&mut self, c: &mut Ctx, v: Value) -> usize { c.ary_push(self.objs, v); c.ary_len(self.objs) - 1 }
    fn symbol(&mut self, c: &mut Ctx) -> Result<Sym, Esc> {
        let t = self.byte(c)?;
        self.symbol_of(c, t, false)
    }
    fn symbol_of(&mut self, c: &mut Ctx, t: u8, ivar: bool) -> Result<Sym, Esc> {
        match t {
            b';' => { let i = self.long(c)? as usize; match self.syms.get(i) { Some(&sy) => Ok(sy), None => arg_err(c, "bad symbol") } }
            b':' => {
                let b = self.rbytes(c)?;
                let idx = self.syms.len();
                self.syms.push(intern(&b));
                if ivar { let n = self.len(c)?; for _ in 0..n { self.symbol(c)?; self.object(c)?; } }
                Ok(self.syms[idx])
            }
            b'I' => { let t2 = self.byte(c)?; self.symbol_of(c, t2, true) }
            _ => arg_err(c, format!("dump format error for symbol(0x{:x})", t)),
        }
    }
    fn path2class(&mut self, c: &mut Ctx, name: &[u8]) -> RV {
        let mut k = c.vm().c_object.get();
        for part in String::from_utf8_lossy(name).split("::") {
            match const_get_inherit(c, k, intern(part.as_bytes())) {
                Ok(Some(v)) => k = v,
                _ => { c.esc = EscKind::None; return arg_err(c, format!("undefined class/module {}", String::from_utf8_lossy(name))); }
            }
            if !is_class_or_module(k) { return type_err(c, format!("{} does not refer to class/module", String::from_utf8_lossy(name))); }
        }
        Ok(k)
    }
    fn class(&mut self, c: &mut Ctx) -> RV {
        let n = self.symbol(c)?;
        let k = self.path2class(c, sym_bytes(n))?;
        if c.class(k).is_module { return type_err(c, format!("{} does not refer to class", String::from_utf8_lossy(sym_bytes(n)))); }
        Ok(k)
    }
    fn leave(&mut self, c: &mut Ctx, v: Value) -> RV {
        if self.freeze && v.is_heap() && !is_class_or_module(v) { obj_set_flags(v, FL_FROZEN); }
        if self.proc_ != NIL { return funcall(c, self.proc_, intern(b"call"), &[v], std::ptr::null()); }
        Ok(v)
    }
    /// Read `n` instance variables onto v (":E" / ":encoding" set a String's encoding).
    fn ivars(&mut self, c: &mut Ctx, v: Value) -> Result<(), Esc> {
        let n = self.len(c)?;
        for _ in 0..n {
            let name = self.symbol(c)?;
            let val = self.object(c)?;
            let nb = sym_bytes(name);
            if nb == b"E" && (is_string(v) || crate::regexp::is_regexp(c, v)) { if is_string(v) { c.rstring_mut(v).enc = if val == TRUE { ENC_UTF8 } else { ENC_ASCII }; } continue; }
            if nb == b"encoding" && is_string(v) { if is_string(val) { if let Some(e) = crate::builtins::encoding::find_enc(c.str_bytes(val)) { c.rstring_mut(v).enc = e; } } continue; }
            if v.is_heap() { ivar_set_raw(v, name, val); }
        }
        Ok(())
    }
    fn object(&mut self, c: &mut Ctx) -> RV {
        let t = self.byte(c)?;
        self.object_of(c, t, false, &mut vec![])
    }
    fn object_of(&mut self, c: &mut Ctx, t: u8, ivar: bool, ext: &mut Vec<Value>) -> RV {
        stack_check(c, 16)?;
        let v = match t {
            b'@' => { let i = self.long(c)? as usize; let Some(&v) = c.ary(self.objs).get(i) else { return arg_err(c, "dump format error (unlinked)") }; if self.proc_ != NIL { return funcall(c, self.proc_, intern(b"call"), &[v], std::ptr::null()); } return Ok(v); }
            b'I' => { let t2 = self.byte(c)?; if t2 == b':' { let sy = self.symbol_of(c, t2, true)?; return self.leave(c, Value::sym(sy)); } return self.object_of(c, t2, true, ext); }
            b'e' => { let n = self.symbol(c)?; let m = self.path2class(c, sym_bytes(n))?; ext.push(m); let t2 = self.byte(c)?; return self.object_of(c, t2, ivar, ext); }
            b'C' => {
                let k = self.class(c)?;
                let t2 = self.byte(c)?;
                // the wrapped object is built first, then re-classed; leave/ivars are handled by the inner call
                let inner = self.object_of_raw(c, t2, ivar, ext)?;
                // marshal.c: the user class must be a subclass of what the data built
                if !inner.is_heap() || !crate::builtins::boot::inherits(k, real_class_of(inner)) { return arg_err(c, "dump format error (user class)"); }
                obj_set_klass(inner, k);
                return self.leave(c, inner);
            }
            _ => self.object_of_raw(c, t, ivar, ext)?,
        };
        self.leave(c, v)
    }
    fn object_of_raw(&mut self, c: &mut Ctx, t: u8, ivar: bool, ext: &mut Vec<Value>) -> RV {
        let mut ivar_done = !ivar;
        let v = match t {
            b'0' => NIL, b'T' => TRUE, b'F' => FALSE,
            b'i' => Value::fix(self.long(c)?),
            b':' | b';' => Value::sym(self.symbol_of(c, t, false)?),
            b'f' => {
                let b = self.rbytes(c)?;
                let f = match &b[..] { b"nan" => f64::NAN, b"inf" => f64::INFINITY, b"-inf" => f64::NEG_INFINITY, _ => { let n = b.iter().position(|&x| x == 0).unwrap_or(b.len()); String::from_utf8_lossy(&b[..n]).parse::<f64>().unwrap_or(0.0) } };
                let v = c.float_new(f);
                self.entry(c, v); v
            }
            b'l' => {
                let sign = self.byte(c)?;
                let n = self.len(c)?;
                let b = self.bytes(c, n * 2)?;
                let mut mag: Vec<u32> = b.chunks(4).map(|ch| { let mut w = [0u8; 4]; w[..ch.len()].copy_from_slice(ch); u32::from_le_bytes(w) }).collect();
                while mag.len() > 1 && *mag.last().unwrap() == 0 { mag.pop(); }
                let v = c.big_to_value(BigInt { neg: sign == b'-', mag });
                self.entry(c, v); v
            }
            b'"' => { let b = self.rbytes(c)?; let v = c.str_from_vec_enc(b, ENC_BINARY); self.entry(c, v); v }
            b'/' => {
                let src = self.rbytes(c)?;
                let opts = self.byte(c)? as u32;
                let idx = self.entry(c, NIL);
                let tmp = c.str_from_vec_enc(src, ENC_BINARY);
                if ivar { self.ivars(c, tmp)?; ivar_done = true; }
                let b = c.str_bytes(tmp).to_vec();
                let v = crate::regexp::regexp_new(c, &b, opts)?;
                c.ary_mut(self.objs)[idx] = v; v
            }
            b'[' => {
                let n = self.len(c)?;
                let v = c.ary_new();
                self.entry(c, v);
                for _ in 0..n { let x = self.object(c)?; c.ary_push(v, x); }
                v
            }
            b'{' | b'}' => {
                let n = self.len(c)?;
                let v = c.hash_new();
                self.entry(c, v);
                for _ in 0..n {
                    let k = self.object(c)?;
                    let x = self.object(c)?;
                    let k = if self.symbolize && is_string(k) { c.str_to_sym(k) } else { k };
                    hash_aset(c, v, k, x)?;
                }
                if t == b'}' { let d = self.object(c)?; c.hash_mut(v).default = d; }
                v
            }
            b'c' | b'm' | b'M' => {
                let n = self.rbytes(c)?;
                let k = self.path2class(c, &n)?;
                if t == b'c' && c.class(k).is_module { return type_err(c, format!("{} does not refer to class", String::from_utf8_lossy(&n))); }
                if t == b'm' && !c.class(k).is_module { return type_err(c, format!("{} does not refer to module", String::from_utf8_lossy(&n))); }
                self.entry(c, k); k
            }
            b'S' => {
                let k = self.class(c)?;
                let n = self.len(c)?;
                let mem = funcall(c, k, intern(b"members"), &[], std::ptr::null())?;
                if !is_array(mem) || c.ary_len(mem) != n { return type_err(c, format!("struct {} not compatible (struct size differs)", class_path(k))); }
                let v = new_object(k);
                self.entry(c, v);
                for i in 0..n {
                    let slot = self.symbol(c)?;
                    let want = c.ary_get(mem, i);
                    if want != Value::sym(slot) { return type_err(c, format!("struct {} not compatible (:{} for :{})", class_path(k), String::from_utf8_lossy(sym_bytes(slot)), String::from_utf8_lossy(sym_bytes(want.as_sym())))); }
                    let x = self.object(c)?;
                    ivar_set_raw(v, slot, x);
                }
                v
            }
            b'u' => {
                let k = self.class(c)?;
                let b = self.rbytes(c)?;
                let data = c.str_from_vec_enc(b, ENC_BINARY);
                if ivar { self.ivars(c, data)?; ivar_done = true; }
                if crate::builtins::inherits(k, c.vm().c_time.get()) && c.str_bytes(data).len() == 8 {
                    let v = load_time(c, k, data)?;
                    self.entry(c, v);
                    return Ok(v);
                }
                if !respond_to(k, intern(b"_load"), true) { return type_err(c, format!("class {} needs to have method '_load'", class_path(k))); }
                let v = funcall(c, k, intern(b"_load"), &[data], std::ptr::null())?;
                self.entry(c, v); v
            }
            b'U' => {
                let k = self.class(c)?;
                let idx = self.entry(c, NIL);
                let (kr, kc) = (c.vm().c_rational.get(), c.vm().c_complex.get());
                if k == kr || k == kc {
                    let d = self.object(c)?;
                    if !is_array(d) || c.ary_len(d) != 2 { return arg_err(c, "marshaled rational must have an array whose length is 2 but 0"); }
                    let (x, y) = (c.ary_get(d, 0), c.ary_get(d, 1));
                    let v = if k == kr { crate::builtins::rational::rat_new(c, x, y) } else { crate::builtins::complex::cpx_raw(c, x, y) };
                    c.ary_mut(self.objs)[idx] = v;
                    v
                } else {
                    let v = crate::builtins::object::alloc_instance(c, k)?;
                    for &m in ext.iter().rev() { funcall(c, v, intern(b"extend"), &[m], std::ptr::null())?; }
                    ext.clear();
                    if !respond_to(v, intern(b"marshal_load"), true) { return type_err(c, format!("instance of {} needs to have method 'marshal_load'", class_path(k))); }
                    c.ary_mut(self.objs)[idx] = v;
                    let d = self.object(c)?;
                    funcall(c, v, intern(b"marshal_load"), &[d], std::ptr::null())?;
                    v
                }
            }
            b'o' => {
                let k = self.class(c)?;
                let idx = self.entry(c, NIL);
                let v = crate::builtins::object::alloc_instance(c, k)?;
                c.ary_mut(self.objs)[idx] = v;
                if is_range(v) {
                    let n = self.len(c)?;
                    let (mut b, mut e, mut excl) = (NIL, NIL, false);
                    for _ in 0..n {
                        let name = self.symbol(c)?; let x = self.object(c)?;
                        match sym_bytes(name) { b"begin" => b = x, b"end" => e = x, b"excl" => excl = x.truthy(), _ => { ivar_set_raw(v, name, x); } }
                    }
                    c.range_set(v, b, e, excl);
                    obj_set_flags(v, FL_FROZEN);
                } else { self.ivars(c, v)?; }
                v
            }
            b'd' => return type_err(c, "dump format error (data objects are not supported)"),
            _ => return arg_err(c, format!("dump format error(0x{:x})", t)),
        };
        if !ivar_done { self.ivars(c, v)?; }
        if !ext.is_empty() && v.is_heap() { for &m in ext.iter().rev() { funcall(c, v, intern(b"extend"), &[m], std::ptr::null())?; } ext.clear(); }
        if self.freeze && is_string(v) { obj_set_flags(v, FL_FROZEN); }
        Ok(v)
    }
}

fn load_time(c: &mut Ctx, k: Value, data: Value) -> RV {
    let b: [u8; 8] = c.str_bytes(data).try_into().expect("8 bytes");
    let p = u32::from_le_bytes([b[0], b[1], b[2], b[3]]);
    let sw = u32::from_le_bytes([b[4], b[5], b[6], b[7]]);
    let utc = p & (1 << 30) != 0;
    let (year, mon, mday, hour) = (((p >> 14) & 0xffff) as i64 + 1900, ((p >> 10) & 0xf) as i64 + 1, ((p >> 5) & 0x1f) as i64, (p & 0x1f) as i64);
    let (min, sec, usec) = ((sw >> 26) as i64, ((sw >> 20) & 0x3f) as i64, (sw & 0xfffff) as i64);
    let nn = ivar_get(data, intern(b"nano_num"));
    let nano = if nn.is_fixnum() { let d = ivar_get(data, intern(b"nano_den")); nn.as_fix() / if d.is_fixnum() && d.as_fix() != 0 { d.as_fix() } else { 1 } } else { 0 };
    let args = [year, mon, mday, hour, min, sec].map(Value::fix);
    let base = funcall(c, k, intern(b"utc"), &args, std::ptr::null())?;
    let secs = funcall(c, base, intern(b"to_i"), &[], std::ptr::null())?;
    let t = funcall(c, k, intern(b"at"), &[secs, Value::fix(usec * 1000 + nano), sym_value("nsec")], std::ptr::null())?;
    if utc { return funcall(c, t, intern(b"utc"), &[], std::ptr::null()); }
    let off = ivar_get(data, intern(b"offset"));
    if off.is_fixnum() {
        let local = funcall(c, t, intern(b"utc_offset"), &[], std::ptr::null())?;
        if local != off || ivar_get(data, intern(b"zone")) == NIL { return funcall(c, t, intern(b"localtime"), &[off], std::ptr::null()); }
    }
    Ok(t)
}

fn m_load(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    let (pos, opts) = match a.last() { Some(&l) if is_hash(l) && a.len() > 1 => (&a[..a.len() - 1], Some(l)), _ => (a, None) };
    check_argc(c, pos, 1, 2)?;
    let objs = c.ary_new();
    let mut l = Loader { src: pos[0], data: vec![], io: false, pos: 0, syms: vec![], objs, proc_: pos.get(1).copied().unwrap_or(NIL), freeze: false, symbolize: false };
    if let Some(h) = opts {
        if let Some(v) = hash_get_opt(c, h, sym_value("freeze"))? { l.freeze = v.truthy(); }
        if let Some(v) = hash_get_opt(c, h, sym_value("symbolize_names"))? { l.symbolize = v.truthy(); }
        if let Some(v) = hash_get_opt(c, h, sym_value("proc"))? { l.proc_ = v; }
    }
    if !is_string(l.src) {
        if respond_to(l.src, intern(b"to_str"), true) { l.src = expect_str(c, l.src)?; }
        else if respond_to(l.src, intern(b"getbyte"), true) && respond_to(l.src, intern(b"read"), true) { l.io = true; if respond_to(l.src, intern(b"binmode"), true) { funcall(c, l.src, intern(b"binmode"), &[], std::ptr::null())?; } }
        else { return type_err(c, "instance of IO needed"); }
    }
    if !l.io { l.data = c.str_bytes(l.src).to_vec(); }
    let major = l.byte(c)?;
    let minor = l.byte(c)?;
    if major != 4 || minor > 8 {
        return type_err(c, format!("incompatible marshal file format (can't be read)\n\tformat version 4.8 required; {}.{} given", major, minor));
    }
    l.object(c)
}

/// Defines Marshal and returns it (boot).
pub fn init() -> Value {
    let m = define_module("Marshal");
    define_module_function(m, "dump", m_dump, -1);
    define_module_function(m, "load", m_load, -1);
    define_module_function(m, "restore", m_load, -1);
    const_set(m, intern(b"MAJOR_VERSION"), Value::fix(4));
    const_set(m, intern(b"MINOR_VERSION"), Value::fix(8));
    let _ = s::mesg;
    m
}

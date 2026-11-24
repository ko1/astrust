// Marshal (format 4.8): dump / load, after CRuby's marshal.c (w_object / r_object_for).
// On the context-borrowing heap accessors (heap.rs, docs/safe_heap.md).
#![forbid(unsafe_code)]
#[allow(unused_imports)]
use crate::heap::no_legacy::*;   // the old accessors cannot be named here
#[allow(unused_imports)]
use crate::heap::no_legacy_hn::*;
#[allow(unused_imports)]
use crate::heap::no_legacy_str::*;
use crate::heap::{obj_flags, obj_klass, obj_set_flags, obj_set_klass};
use crate::array::*;
use crate::bignum::BigInt;
use crate::builtins::numeric_methods::num_to_long;
use crate::fxhash::HashMap;
use crate::hash::*;
use crate::string::*;
use crate::symbol::{intern, s, sym_bytes};
use crate::value::*;
use crate::vm::*;

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

/// encoding_name(): nil, US-ASCII / UTF-8 as false / true, or the name String.
#[derive(Clone, Copy, PartialEq)]
enum EncName { None, Short(bool), Name(Value) }

fn data_class(c: &Ctx) -> Value { const_get_at(c, c.vm().c_object.get(), intern(b"Data")).unwrap_or(NIL) }
/// T_STRUCT in CRuby: Struct and Data instances (saruby keeps their members as hidden ivars).
fn is_struct_class(c: &Ctx, k: Value) -> bool {
    let d = data_class(c);
    crate::builtins::inherits(c, k, c.vm().c_struct.get()) || (d != NIL && crate::builtins::inherits(c, k, d))
}
/// A Dir is T_DATA in CRuby (an Object with ivars here).
fn is_dir(c: &Ctx, v: Value) -> bool { crate::builtins::inherits(c, real_class_of(c, v), c.vm().c_dir.get()) }
/// The instance variables Marshal sees on v (the ones Ruby code can name; Exception's internal ones).
fn dump_ivars(c: &mut Ctx, v: Value) -> Vec<(Sym, Value)> {
    if !v.is_heap() || is_class_or_module(v) { return vec![]; }
    let user = |l: Vec<(Sym, Value)>| l.into_iter().filter(|(n, _)| sym_bytes(*n).starts_with(b"@")).collect::<Vec<_>>();
    if !matches!(c.body_of(v), Body::Object(_)) {
        // generic ivars: also the ones C code sets (Time#_dump's nano_num, zone, ...), not saruby's "__" ones
        return ivar_list(c, v).into_iter().filter(|(n, _)| !sym_bytes(*n).starts_with(b"__")).collect();
    }
    if !is_exception(c, v) { let l = ivar_list(c, v); return user(l); }
    // exc_init sets mesg and bt; raising adds cause and bt_locations (the same Array as bt once dumped)
    let bt = exc_bt(c, v);
    let raised = bt != NIL && ivar_defined(c, v, intern(b"__bt_nids"));
    let mut l = vec![(s::mesg, ivar_get(c, v, s::mesg)), (s::bt, bt)];
    let all = ivar_list(c, v);
    for n in [&b"cause"[..], b"name", b"args", b"private_call?"] {
        if let Some(&(k, x)) = all.iter().find(|(k, _)| sym_bytes(*k) == n) { l.push((k, x)); }
    }
    if raised { l.push((intern(b"bt_locations"), bt)); }
    l.extend(user(all));
    l
}
fn path_of(c: &mut Ctx, k: Value) -> (Vec<u8>, u8) {
    match c.class(k).name {
        Some(n) => (c.str_bytes(n).to_vec(), c.str_enc(n)),
        None => (class_path(c, k).into_bytes(), ENC_ASCII),
    }
}
fn must_not_be_anonymous(c: &mut Ctx, kind: &str, p: &[u8]) -> Result<(), Esc> {
    if p.first() == Some(&b'#') { return type_err(c, format!("can't dump anonymous {} {}", kind, String::from_utf8_lossy(p))); }
    Ok(())
}
/// rb_path_to_class: each segment in the namespace's own table (autoload included), no lexical fallback.
pub fn path_to_class(c: &mut Ctx, path: &[u8]) -> RV {
    let ps = |b: &[u8]| String::from_utf8_lossy(b).into_owned();
    if path.is_empty() || path[0] == b'#' { return arg_err(c, format!("can't retrieve anonymous class {}", ps(path))); }
    let mut k = c.vm().c_object.get();
    let mut p = 0;
    while p < path.len() {
        let beg = p;
        while p < path.len() && path[p] != b':' { p += 1; }
        let id = &path[beg..p];
        let undefined = |c: &mut Ctx, end: usize| -> RV { arg_err(c, format!("undefined class/module {}", ps(&path[..end]))) };
        let end = p;
        if p < path.len() {
            if path.len() - p < 2 || path[p + 1] != b':' { return undefined(c, end); }
            p += 2;
        }
        if id.is_empty() { return undefined(c, end); }
        let name = intern(id);
        let v = match const_get_at(c, k, name) {
            Some(v) => Some(v),
            None if c.class(k).autoload.contains_key(&name) => do_autoload(c, k, name)?,
            None => None,
        };
        let Some(v) = v else { return undefined(c, end) };
        if !is_class_or_module(v) { return type_err(c, format!("{} does not refer to class/module", ps(path))); }
        k = v;
    }
    Ok(k)
}
fn class2path(c: &mut Ctx, k: Value) -> Result<(Vec<u8>, u8), Esc> {
    let (p, e) = path_of(c, k);
    let kind = if c.class(k).is_module { "module" } else { "class" };
    must_not_be_anonymous(c, kind, &p)?;
    if path_to_class(c, &p)? != k { return type_err(c, format!("{} can't be referred to", String::from_utf8_lossy(&p))); }
    Ok((p, e))
}
/// Modules between a singleton class and its object's class, in method-lookup order.
fn extended_modules(c: &Ctx, sing: Value, real: Value) -> Vec<Value> {
    ancestors(c, sing).iter().copied().take_while(|&m| m != real).filter(|&m| m != sing).collect()
}
fn extend_object(c: &mut Ctx, v: Value, m: Value) -> Result<(), Esc> {
    if is_frozen(v) && v.is_heap() { return Err(raise_frozen(c, v)); }
    let k = singleton_class(c, v);
    if k == NIL { return type_err(c, "can't define singleton"); }
    include_module_c(c, k, m);
    c.class_mut(m).extended_by.push(v);
    Ok(())
}

struct Dumper { out: Vec<u8>, syms: HashMap<Sym, usize>, objs: HashMap<u64, usize>, n: usize, keep: Value, encs: HashMap<u8, Value>, userdefs: Vec<u64> }

impl Dumper {
    fn remember(&mut self, v: Value) { self.objs.insert(v.raw(), self.n); self.n += 1; }
    fn enc_name(&mut self, c: &mut Ctx, e: u8) -> EncName {
        match e {
            ENC_BINARY => EncName::None,
            ENC_ASCII => EncName::Short(false),
            ENC_UTF8 => EncName::Short(true),
            e => {
                if let Some(&v) = self.encs.get(&e) { return EncName::Name(v); }
                let v = c.str_from_vec_enc(crate::builtins::encoding::enc_name(e).as_bytes().to_vec(), ENC_BINARY);
                c.ary_push(self.keep, v);
                self.encs.insert(e, v);
                EncName::Name(v)
            }
        }
    }
    fn encoding_name(&mut self, c: &mut Ctx, v: Value) -> EncName {
        if is_string(v) { let e = c.str_enc(v); return self.enc_name(c, e); }
        if let Some(r) = crate::regexp::rregexp(c, v) { let e = r.enc; return self.enc_name(c, e); }
        EncName::None
    }
    fn w_encoding(&mut self, c: &mut Ctx, en: EncName, limit: i64) -> Result<usize, Esc> {
        let limit = if limit >= 0 { limit + 1 } else { limit };
        match en {
            EncName::None => Ok(0),
            EncName::Short(b) => { self.sym(c, intern(b"E"))?; self.obj(c, Value::bool(b), limit)?; Ok(1) }
            EncName::Name(v) => { self.sym(c, intern(b"encoding"))?; self.obj(c, v, limit)?; Ok(1) }
        }
    }
    /// w_encivar + the bytes + w_encname, for a class path.
    fn w_named(&mut self, c: &mut Ctx, tag: u8, b: &[u8], enc: u8) -> Result<(), Esc> {
        let en = if is_ascii(b) { EncName::None } else { self.enc_name(c, enc) };
        if en != EncName::None { self.out.push(b'I'); }
        self.out.push(tag);
        w_bytes(&mut self.out, b);
        if en != EncName::None { w_long(&mut self.out, 1); self.w_encoding(c, en, 1)?; }
        Ok(())
    }
    fn sym(&mut self, c: &mut Ctx, sy: Sym) -> Result<(), Esc> {
        if let Some(&i) = self.syms.get(&sy) { self.out.push(b';'); w_long(&mut self.out, i as i64); return Ok(()); }
        let b = sym_bytes(sy).to_vec();
        let enc = crate::builtins::symbol_methods::sym_encoding(sy);
        let en = if is_ascii(&b) { EncName::None } else { self.enc_name(c, enc) };
        if en != EncName::None { self.out.push(b'I'); }
        self.out.push(b':');
        w_bytes(&mut self.out, &b);
        let n = self.syms.len();
        self.syms.insert(sy, n);
        if en != EncName::None { w_long(&mut self.out, 1); self.w_encoding(c, en, 1)?; }
        Ok(())
    }
    fn unique(&mut self, c: &mut Ctx, p: &[u8], enc: u8) -> Result<(), Esc> {
        must_not_be_anonymous(c, "class", p)?;
        self.sym(c, crate::string::sym_of(p, if is_ascii(p) { ENC_ASCII } else { enc }))
    }
    fn extended(&mut self, c: &mut Ctx, klass: Value, real: Value, check: bool) -> Result<(), Esc> {
        if !check || !c.class(klass).is_singleton { return Ok(()); }
        if !c.class(klass).mtbl.is_empty() || ivar_list(c, klass).iter().any(|(n, _)| sym_bytes(*n).starts_with(b"@")) { return type_err(c, "singleton can't be dumped"); }
        for m in extended_modules(c, klass, real) {
            self.out.push(b'e');
            let (p, e) = path_of(c, m);
            self.unique(c, &p, e)?;
        }
        Ok(())
    }
    fn w_class(&mut self, c: &mut Ctx, tag: u8, v: Value, check: bool) -> Result<(), Esc> {
        let real = real_class_of(c, v);
        self.extended(c, obj_klass(v), real, check)?;
        self.out.push(tag);
        let (p, e) = class2path(c, real)?;
        self.unique(c, &p, e)
    }
    fn w_uclass(&mut self, c: &mut Ctx, v: Value, base: Value) -> Result<(), Esc> {
        let real = real_class_of(c, v);
        self.extended(c, obj_klass(v), real, true)?;
        if real != base { self.out.push(b'C'); let (p, e) = class2path(c, real)?; self.unique(c, &p, e)?; }
        Ok(())
    }
    /// has_ivars: (count incl. the encoding, the ivars).
    fn has_ivars(&mut self, c: &mut Ctx, v: Value, en: EncName, generic: bool) -> (usize, Vec<(Sym, Value)>) {
        let mut ivs = if generic { dump_ivars(c, v) } else { vec![] };
        for &(_, x) in &ivs { if x.is_heap() { c.ary_push(self.keep, x); } }
        let mut n = (en != EncName::None) as usize + ivs.len();
        if generic && is_hash(v) && obj_flags(v) & crate::object::FL_KWHASH != 0 { n += 1; ivs.insert(0, (s::ruby2_keywords_flag, UNDEF)); }
        (n, ivs)
    }
    fn w_ivar(&mut self, c: &mut Ctx, num: usize, en: EncName, ivs: &[(Sym, Value)], limit: i64) -> Result<(), Esc> {
        w_long(&mut self.out, num as i64);
        self.w_encoding(c, en, limit)?;
        for &(n, x) in ivs {
            if n == s::ruby2_keywords_flag {
                self.sym(c, intern(b"K"))?;
                let l = if limit >= 0 { limit + 1 } else { limit };
                self.obj(c, TRUE, l)?;
                continue;
            }
            self.sym(c, n)?; self.obj(c, x, limit)?;
        }
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
            bignum(&mut self.out, &BigInt::from_i64(n));
            self.n += 1;
            return Ok(());
        }
        if v.is_sym() { return self.sym(c, v.as_sym()); }
        if let Some(&i) = self.objs.get(&v.raw()) { self.out.push(b'@'); w_long(&mut self.out, i as i64); return Ok(()); }
        let limit = if limit > 0 { limit - 1 } else { limit };
        if v.is_flonum() { self.remember(v); self.out.push(b'f'); w_bytes(&mut self.out, float_repr(float_val(v)).as_bytes()); return Ok(()); }
        if respond_to(c, v, intern(b"marshal_dump"), true) {
            self.remember(v);
            let d = funcall(c, v, intern(b"marshal_dump"), &[], std::ptr::null())?;
            c.ary_push(self.keep, d);
            self.w_class(c, b'U', v, false)?;
            return self.obj(c, d, limit);
        }
        if respond_to(c, v, intern(b"_dump"), true) {
            if self.userdefs.contains(&v.raw()) { return err(c, c.vm().c_runtime_error.get(), "can't dump recursive object using _dump()"); }
            let d = funcall(c, v, intern(b"_dump"), &[Value::fix(limit)], std::ptr::null())?;
            if !is_string(d) { return type_err(c, "_dump() must return string"); }
            c.ary_push(self.keep, d);
            let en = self.encoding_name(c, v);
            let mut iv = self.has_ivars(c, v, en, !matches!(c.body_of(v), Body::Object(_) | Body::Class(_)));
            let mut en = en;
            let en2 = self.encoding_name(c, d);
            let iv2 = self.has_ivars(c, d, en2, true);
            if iv2.0 != 0 { iv = iv2; en = en2; }
            if iv.0 != 0 { self.out.push(b'I'); }
            self.w_class(c, b'u', v, false)?;
            w_bytes(&mut self.out, c.str_bytes(d));
            if iv.0 != 0 {
                self.userdefs.push(v.raw());
                let r = self.w_ivar(c, iv.0, en, &iv.1, limit);
                self.userdefs.retain(|&x| x != v.raw());
                r?;
            }
            self.remember(v);
            return Ok(());
        }
        self.remember(v);
        let en = self.encoding_name(c, v);
        // the kind and the fields needed, by value: the arms below call out
        enum K { Big, Class(bool, bool), Str, Re, Ary, Hash(Value, Value, bool), Range(Value, Value, bool), Struct, Obj, Float, Data }
        let kind = match c.body_of(v) {
            Body::Bignum(_) => K::Big, Body::Class(cd) => K::Class(cd.is_singleton, cd.is_module), Body::Array(_) => K::Ary,
            Body::String(_) => K::Str, Body::Float(_) => K::Float,
            Body::Hash(h) => K::Hash(h.default_proc, h.default, h.identity), Body::Range(b, e, x) => K::Range(*b, *e, *x),
            Body::Object(_) => K::Obj, _ => if crate::regexp::is_regexp(c, v) { K::Re } else { K::Data },
        };
        let kind = if matches!(kind, K::Obj) && is_struct_class(c, real_class_of(c, v)) { K::Struct } else if matches!(kind, K::Obj) && is_dir(c, v) { K::Data } else { kind };
        let (hasiv, ivs) = match kind { K::Obj | K::Class(..) | K::Range(..) => (0, vec![]), _ => self.has_ivars(c, v, en, true) };
        if hasiv != 0 { self.out.push(b'I'); }
        match kind {
            K::Class(singleton, m) => {
                if singleton { return type_err(c, "singleton class can't be dumped"); }
                let (p, e) = class2path(c, v)?;
                self.w_named(c, if m { b'm' } else { b'c' }, &p, e)?;
            }
            K::Float => { self.out.push(b'f'); w_bytes(&mut self.out, float_repr(float_val(v)).as_bytes()); }
            K::Big => bignum(&mut self.out, c.big(v)),
            K::Str => {
                let k = c.vm().c_string.get();
                self.w_uclass(c, v, k)?;
                self.out.push(b'"');
                w_bytes(&mut self.out, c.str_bytes(v));
            }
            K::Re => {
                let k = c.vm().c_regexp.get();
                self.w_uclass(c, v, k)?;
                let (src, opts) = { let r = crate::regexp::rregexp(c, v).expect("a Regexp"); (r.src.clone(), r.opts) };
                self.out.push(b'/');
                w_bytes(&mut self.out, &src);
                self.out.push((opts & 0xff) as u8);
            }
            K::Ary => {
                let k = c.vm().c_array.get();
                self.w_uclass(c, v, k)?;
                self.out.push(b'[');
                let len = c.ary_len(v);
                w_long(&mut self.out, len as i64);
                let mut i = 0;
                while let Some(&x) = c.ary(v).get(i) {
                    self.obj(c, x, limit)?;
                    if c.ary_len(v) != len { return err(c, c.vm().c_runtime_error.get(), "array modified during dump"); }
                    i += 1;
                }
            }
            K::Hash(default_proc, def, identity) => {
                let kh = c.vm().c_hash.get();
                self.w_uclass(c, v, kh)?;
                if identity { self.out.push(b'C'); self.sym(c, intern(b"Hash"))?; }
                if default_proc != NIL { return type_err(c, "can't dump hash with default proc"); }
                self.out.push(if def == NIL { b'{' } else { b'}' });
                w_long(&mut self.out, c.hash_len(v) as i64);
                each_pair(c, v, |c, k, x| { self.obj(c, k, limit)?; self.obj(c, x, limit) })?;
                if def != NIL { self.obj(c, def, limit)?; }
            }
            K::Range(b, e, excl) => {
                // range_dumper: an Object with the ivars, then excl / begin / end
                self.w_class(c, b'o', v, true)?;
                let ivs = dump_ivars(c, v);
                for &(_, x) in &ivs { if x.is_heap() { c.ary_push(self.keep, x); } }
                w_long(&mut self.out, ivs.len() as i64 + 3);
                for &(n, x) in &ivs { self.sym(c, n)?; self.obj(c, x, limit)?; }
                self.sym(c, intern(b"excl"))?; self.obj(c, Value::bool(excl), limit)?;
                self.sym(c, intern(b"begin"))?; self.obj(c, b, limit)?;
                self.sym(c, intern(b"end"))?; self.obj(c, e, limit)?;
            }
            K::Struct => {
                self.w_class(c, b'S', v, true)?;
                let k = real_class_of(c, v);
                let mem = funcall(c, k, intern(b"members"), &[], std::ptr::null())?;
                c.ary_push(self.keep, mem);
                w_long(&mut self.out, c.ary_len(mem) as i64);
                let mut i = 0;
                while let Some(&m) = c.ary(mem).get(i) {
                    let m = m.as_sym();
                    self.sym(c, m)?;
                    let x = ivar_get(c, v, m);
                    self.obj(c, x, limit)?;
                    i += 1;
                }
            }
            K::Obj => {
                self.w_class(c, b'o', v, true)?;
                let ivs = dump_ivars(c, v);
                for &(_, x) in &ivs { if x.is_heap() { c.ary_push(self.keep, x); } }
                w_long(&mut self.out, ivs.len() as i64);
                for &(n, x) in &ivs { self.sym(c, n)?; self.obj(c, x, limit)?; }
            }
            K::Data => {
                let k = real_class_of(c, v);
                if crate::builtins::rational::is_rational(c, v) || crate::builtins::complex::is_complex(c, v) {
                    let (x, y) = if crate::builtins::rational::is_rational(c, v) { crate::builtins::rational::rat_parts(c, v) } else { crate::builtins::complex::cpx_parts(c, v) };
                    self.w_class(c, b'U', v, false)?;
                    self.n += 1;
                    self.out.push(b'['); w_long(&mut self.out, 2); self.obj(c, x, limit)?; self.obj(c, y, limit)?;
                } else if crate::builtins::io::is_io(c, v) || crate::builtins::inherits(c, k, c.vm().c_match_data.get()) {
                    return type_err(c, format!("can't dump {}", class_path(c, k)));
                } else if respond_to(c, v, intern(b"_dump_data"), true) {
                    let d = funcall(c, v, intern(b"_dump_data"), &[], std::ptr::null())?;
                    c.ary_push(self.keep, d);
                    self.w_class(c, b'd', v, true)?;
                    self.obj(c, d, limit)?;
                } else {
                    return type_err(c, format!("no _dump_data is defined for class {}", class_path(c, k)));
                }
            }
        }
        if hasiv != 0 { self.w_ivar(c, hasiv, en, &ivs, limit)?; }
        Ok(())
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

fn io_needed<T>(c: &mut Ctx) -> Result<T, Esc> { type_err(c, "instance of IO needed") }

fn m_dump(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    check_argc(c, a, 1, 3)?;
    let mut port = NIL; let mut limit = -1;
    if a.len() == 3 {
        if a[2] != NIL { limit = num_to_long(c, a[2])?; }
        if a[1] == NIL { return io_needed(c); }
        port = a[1];
    } else if a.len() == 2 {
        if a[1].is_fixnum() { limit = a[1].as_fix(); } else if a[1] == NIL { return io_needed(c); } else { port = a[1]; }
    }
    if port != NIL {
        if !respond_to(c, port, intern(b"write"), false) { return io_needed(c); }
        if respond_to(c, port, intern(b"binmode"), true) { funcall(c, port, intern(b"binmode"), &[], std::ptr::null())?; }
    }
    let keep = c.ary_new();
    let mut d = Dumper { out: vec![4, 8], syms: HashMap::default(), objs: HashMap::default(), n: 0, keep, encs: HashMap::default(), userdefs: vec![] };
    d.obj(c, a[0], limit)?;
    let out = c.str_from_vec_enc(std::mem::take(&mut d.out), ENC_BINARY);
    if port != NIL {
        funcall(c, port, intern(b"write"), &[out], std::ptr::null())?;
        return Ok(port);
    }
    Ok(out)
}

// ---- load -------------------------------------------------------------------------------------

// `data`: the source String's bytes, copied once (hooks run while reading may change the String)
struct Loader { src: Value, data: Vec<u8>, io: bool, pos: usize, syms: Vec<Sym>, objs: Value, partial: Option<Vec<u64>>, proc_: Value, freeze: bool }
const TOO_SHORT: &str = "marshal data too short";

fn ps(b: &[u8]) -> String { String::from_utf8_lossy(b).into_owned() }

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
            if v == NIL { return arg_err(c, TOO_SHORT); }
            let v = expect_str(c, v)?;
            if c.str_bytes(v).len() != n { return arg_err(c, TOO_SHORT); }
            return Ok(c.str_bytes(v).to_vec());
        }
        if self.data.len() - self.pos < n { return arg_err(c, TOO_SHORT); }
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
    fn rbytes(&mut self, c: &mut Ctx) -> Result<Vec<u8>, Esc> { let n = self.long(c)?; if n < 0 { return arg_err(c, "negative length"); } self.bytes(c, n as usize) }
    fn r_string(&mut self, c: &mut Ctx) -> RV { let b = self.rbytes(c)?; Ok(c.str_from_vec_enc(b, ENC_BINARY)) }
    fn entry0(&mut self, c: &mut Ctx, v: Value, idx: usize) -> Value {
        if idx == c.ary_len(self.objs) { c.ary_push(self.objs, v); } else { c.ary_mut(self.objs)[idx] = v; }
        if let Some(p) = self.partial.as_mut() { p.push(v.raw()); }
        v
    }
    fn entry(&mut self, c: &mut Ctx, v: Value) -> Value { let i = c.ary_len(self.objs); self.entry0(c, v, i) }
    fn prepare(&mut self, c: &mut Ctx) -> usize { c.ary_push(self.objs, UNDEF); c.ary_len(self.objs) - 1 }
    fn post_proc(&mut self, c: &mut Ctx, v: Value) -> RV {
        if self.proc_ != NIL { return funcall(c, self.proc_, intern(b"call"), &[v], std::ptr::null()); }
        Ok(v)
    }
    fn leave(&mut self, c: &mut Ctx, v: Value, partial: bool) -> RV {
        if partial { return Ok(v); }
        if let Some(p) = self.partial.as_mut() { if let Some(i) = p.iter().rposition(|&x| x == v.raw()) { p.swap_remove(i); } }
        let mut v = v;
        if self.freeze && v.is_heap() && !is_class_or_module(v) {
            if is_string(v) { v = interned_str(c, v); } else { obj_set_flags(v, FL_FROZEN); }
        }
        self.post_proc(c, v)
    }
    fn symlink(&mut self, c: &mut Ctx) -> Result<Sym, Esc> {
        let i = self.long(c)?;
        match usize::try_from(i).ok().and_then(|i| self.syms.get(i)) { Some(&sy) => Ok(sy), None => arg_err(c, "bad symbol") }
    }
    fn symreal(&mut self, c: &mut Ctx, ivar: bool) -> Result<Sym, Esc> {
        let b = self.rbytes(c)?;
        let idx = self.syms.len();
        let mut enc = if is_ascii(&b) { ENC_ASCII } else { ENC_BINARY };
        self.syms.push(crate::string::sym_of(&b, enc));
        if ivar {
            let n = self.long(c)?;
            let mut e: Option<u8> = None;
            for _ in 0..n.max(0) { let k = self.symbol(c)?; let v = self.object(c)?; e = sym2enc(c, k, v)?; }
            if let Some(e2) = e {
                if e2 != ENC_BINARY {
                    enc = e2;
                    if !crate::builtins::encoding::is_valid(enc, &b) {
                        let s = c.str_new_enc(&b, enc);
                        let i = crate::builtins::kernel::inspect_str(c, s)?;
                        return arg_err(c, format!("invalid byte sequence in {}: {}", crate::builtins::encoding::enc_name(enc), i));
                    }
                }
            }
        }
        self.syms[idx] = crate::string::sym_of(&b, enc);
        Ok(self.syms[idx])
    }
    fn symbol(&mut self, c: &mut Ctx) -> Result<Sym, Esc> {
        let mut ivar = false;
        loop {
            match self.byte(c)? {
                b'I' => ivar = true,
                b':' => return self.symreal(c, ivar),
                b';' => { if ivar { return arg_err(c, "dump format error (symlink with encoding)"); } return self.symlink(c); }
                t => return arg_err(c, format!("dump format error for symbol(0x{:x})", t)),
            }
        }
    }
    fn unique(&mut self, c: &mut Ctx) -> Result<Vec<u8>, Esc> { let sy = self.symbol(c)?; Ok(sym_bytes(sy).to_vec()) }
    fn path2class(&mut self, c: &mut Ctx, p: &[u8]) -> RV {
        let v = path_to_class(c, p)?;
        if c.class(v).is_module { return arg_err(c, format!("{} does not refer to class", ps(p))); }
        Ok(v)
    }
    fn path2module(&mut self, c: &mut Ctx, p: &[u8]) -> RV {
        let v = path_to_class(c, p)?;
        if !c.class(v).is_module { return arg_err(c, format!("{} does not refer to module", ps(p))); }
        Ok(v)
    }
    /// r_ivar_encoding: an E / encoding pair sets v's encoding.
    fn ivar_encoding(&mut self, c: &mut Ctx, v: Value, k: Sym, val: Value) -> Result<bool, Esc> {
        let Some(e) = sym2enc(c, k, val)? else { return Ok(false) };
        if is_string(v) { if c.str_enc(v) != e { c.rstring_mut(v).enc = e; } }
        else if !crate::regexp::is_regexp(c, v) { let sv = crate::builtins::kernel::to_s_value(c, v)?; let sv = ps(c.str_bytes(sv)); return arg_err(c, format!("{} is not enc_capable", sv)); }
        Ok(true)
    }
    /// r_encname: the first pair if it is the encoding; the count left.
    fn encname(&mut self, c: &mut Ctx, v: Value) -> Result<i64, Esc> {
        let mut n = self.long(c)?;
        if n > 0 { let k = self.symbol(c)?; let x = self.object(c)?; if self.ivar_encoding(c, v, k, x)? { n -= 1; } }
        Ok(n)
    }
    fn r_ivar(&mut self, c: &mut Ctx, v: Value, has_enc: &mut bool) -> Result<(), Esc> {
        let n = self.long(c)?;
        if n <= 0 { return Ok(()); }
        if is_class_or_module(v) {
            let kind = if c.class(v).is_module { "module" } else { "class" };
            return type_err(c, format!("can't override instance variable of {} '{}'", kind, class_path(c, v)));
        }
        let exc = is_exception(c, v);
        for _ in 0..n {
            let k = self.symbol(c)?;
            let x = self.object(c)?;
            if self.ivar_encoding(c, v, k, x)? { *has_enc = true; }
            else if sym_bytes(k) == b"K" {
                if !is_hash(v) { let sv = crate::builtins::kernel::inspect_str(c, v)?; return arg_err(c, format!("ruby2_keywords flag is given but {} is not a Hash", sv)); }
                obj_set_flags(v, crate::object::FL_KWHASH);
            }
            else if exc && sym_bytes(k) == b"bt_locations" {}
            else if v.is_heap() { ivar_set_raw(c, v, k, x); }
            else { return Err(raise_frozen(c, v)); }
        }
        Ok(())
    }
    fn object(&mut self, c: &mut Ctx) -> RV { self.object0(c, false, None, NIL) }
    fn object0(&mut self, c: &mut Ctx, partial: bool, ivp: Option<&mut bool>, extmod: Value) -> RV {
        let t = self.byte(c)?;
        self.object_for(c, partial, ivp, NIL, extmod, t)
    }
    fn object_for(&mut self, c: &mut Ctx, partial: bool, mut ivp: Option<&mut bool>, klass: Value, extmod: Value, t: u8) -> RV {
        stack_check(c, 16)?;
        let v = match t {
            b'@' => {
                let id = self.long(c)?;
                let Some(&v) = usize::try_from(id).ok().and_then(|i| c.ary(self.objs).get(i)) else { return arg_err(c, "dump format error (unlinked)") };
                if v == UNDEF { return arg_err(c, "dump format error (bad link)"); }
                let mut v = v;
                if self.partial.as_ref().is_some_and(|p| !p.contains(&v.raw())) {
                    if self.freeze && is_string(v) { v = interned_str(c, v); }
                    v = self.post_proc(c, v)?;
                }
                v
            }
            b'I' => {
                let mut ivar = true;
                let v = self.object0(c, true, Some(&mut ivar), extmod)?;
                if ivar { self.r_ivar(c, v, &mut false)?; }
                self.leave(c, v, partial)?
            }
            b'e' => {
                let path = self.unique(c)?;
                let m = path_to_class(c, &path)?;
                let extmod = if extmod == NIL { c.ary_new() } else { extmod };
                let v;
                if !c.class(m).is_module {
                    v = self.object0(c, true, None, NIL)?;
                    let k = obj_klass(v);
                    if !v.is_heap() || k != m || c.class(k).is_singleton {
                        let kn = class_path(c, class_of(v));
                        return arg_err(c, format!("prepended class {} differs from class {}", ps(&path), kn));
                    }
                    let sk = singleton_class(c, v);
                    while let Some(m2) = c.ary_mut(extmod).pop() { crate::vm::prepend_module(c, sk, m2); }
                } else {
                    c.ary_push(extmod, m);
                    v = self.object0(c, true, None, extmod)?;
                    while let Some(m2) = c.ary_mut(extmod).pop() { extend_object(c, v, m2)?; }
                }
                self.leave(c, v, partial)?
            }
            b'C' => {
                let path = self.unique(c)?;
                let k = self.path2class(c, &path)?;
                if c.class(k).is_singleton { return type_err(c, "singleton can't be loaded"); }
                let t2 = self.byte(c)?;
                let v = if k == c.vm().c_hash.get() && (t2 == b'{' || t2 == b'}') {
                    return self.hash(c, partial, t2, true);
                } else { self.object_for(c, partial, None, k, extmod, t2)? };
                let fmt = |c: &mut Ctx| -> RV { arg_err(c, "dump format error (user class)") };
                if !v.is_heap() || is_class_or_module(v) || (matches!(c.body_of(v), Body::Object(_)) && !is_struct_class(c, real_class_of(c, v))) { return fmt(c); }
                let vk = real_class_of(c, v);
                if !crate::builtins::inherits(c, k, vk) {
                    let tmp = crate::builtins::object::alloc_instance(c, k)?;
                    if std::mem::discriminant(c.body_of(tmp)) != std::mem::discriminant(c.body_of(v)) { return fmt(c); }
                }
                if is_struct_class(c, vk) {
                    let m1 = funcall(c, k, intern(b"members"), &[], std::ptr::null())?;
                    let m0 = funcall(c, vk, intern(b"members"), &[], std::ptr::null())?;
                    if c.ary_len(m1) != c.ary_len(m0) { return type_err(c, format!("struct {} not compatible (struct size differs)", class_path(c, k))); }
                }
                obj_set_klass(v, k);
                v
            }
            b'0' => self.leave(c, NIL, false)?,
            b'T' => self.leave(c, TRUE, false)?,
            b'F' => self.leave(c, FALSE, false)?,
            b'i' => { let n = self.long(c)?; self.leave(c, Value::fix(n), false)? }
            b'f' => {
                let b = self.rbytes(c)?;
                let n = b.iter().position(|&x| x == 0).unwrap_or(b.len());
                let f = match &b[..n] { b"nan" => f64::NAN, b"inf" => f64::INFINITY, b"-inf" => f64::NEG_INFINITY, s => strtod(s) };
                let v = c.float_new(f);
                let v = self.entry(c, v);
                self.leave(c, v, false)?
            }
            b'l' => {
                let sign = self.byte(c)?;
                if sign != b'+' && sign != b'-' { return arg_err(c, "invalid Bignum sign"); }
                let n = self.len(c)?;
                let b = self.bytes(c, n * 2)?;
                let mut mag: Vec<u32> = b.chunks(4).map(|ch| { let mut w = [0u8; 4]; w[..ch.len()].copy_from_slice(ch); u32::from_le_bytes(w) }).collect();
                while mag.len() > 1 && *mag.last().unwrap() == 0 { mag.pop(); }
                let v = c.big_to_value(BigInt { neg: sign == b'-', mag });
                let v = self.entry(c, v);
                self.leave(c, v, false)?
            }
            b'"' => { let s = self.r_string(c)?; let v = self.entry(c, s); self.leave(c, v, partial)? }
            b'/' => {
                let str = self.r_string(c)?;
                let opts = self.byte(c)? as u32;
                let idx = self.prepare(c);
                let mut has_enc = false;
                if let Some(iv) = ivp.as_deref_mut() { self.r_ivar(c, str, &mut has_enc)?; *iv = false; }
                let b = if has_enc { c.str_bytes(str).to_vec() } else { strip_18_escapes(c.str_bytes(str)) };
                let e = c.str_enc(str);
                let re = crate::regexp::regexp_new_enc(c, &b, e, opts)?;
                if klass != NIL { obj_set_klass(re, klass); }
                for (n, x) in ivar_list(c, str) { if !ivar_defined(c, re, n) { ivar_set_raw(c, re, n, x); } }
                let v = self.entry0(c, re, idx);
                self.leave(c, v, partial)?
            }
            b'[' => {
                let n = self.len(c)?;
                let v = c.ary_new();
                self.entry(c, v);
                for _ in 0..n { let x = self.object(c)?; c.ary_push(v, x); }
                self.leave(c, v, partial)?
            }
            b'{' | b'}' => return self.hash(c, partial, t, false),
            b'S' => {
                let idx = self.prepare(c);
                let path = self.unique(c)?;
                let k = self.path2class(c, &path)?;
                let n = self.len(c)?;
                if !is_struct_class(c, k) { return type_err(c, format!("class {} not a struct", class_path(c, k))); }
                let v = crate::builtins::object::alloc_instance(c, k)?;
                let mem = funcall(c, k, intern(b"members"), &[], std::ptr::null())?;
                if !is_array(mem) || c.ary_len(mem) != n { return type_err(c, format!("struct {} not compatible (struct size differs)", class_path(c, k))); }
                self.entry0(c, v, idx);
                for i in 0..n {
                    let want = c.ary_get(mem, i);
                    let slot = self.symbol(c)?;
                    if !want.is_sym() || sym_bytes(want.as_sym()) != sym_bytes(slot) {
                        let wn = if want.is_sym() { ps(sym_bytes(want.as_sym())) } else { String::new() };
                        return type_err(c, format!("struct {} not compatible (:{} for :{})", class_path(c, k), ps(sym_bytes(slot)), wn));
                    }
                    let x = self.object(c)?;
                    ivar_set_raw(c, v, want.as_sym(), x);
                }
                let d = data_class(c);
                if d != NIL && crate::builtins::inherits(c, k, d) { obj_set_flags(v, FL_FROZEN); }
                self.leave(c, v, partial)?
            }
            b'u' => {
                let name = self.unique(c)?;
                let k = self.path2class(c, &name)?;
                if !respond_to(c, k, intern(b"_load"), true) { return type_err(c, format!("class {} needs to have method '_load'", ps(&name))); }
                let data = self.r_string(c)?;
                if let Some(iv) = ivp.as_deref_mut() { self.r_ivar(c, data, &mut false)?; *iv = false; }
                let v = funcall(c, k, intern(b"_load"), &[data], std::ptr::null())?;
                let mut v = self.entry(c, v);
                if !partial {
                    if self.freeze && v.is_heap() && !is_class_or_module(v) { obj_set_flags(v, FL_FROZEN); }
                    v = self.post_proc(c, v)?;
                }
                v
            }
            b'U' => {
                let name = self.unique(c)?;
                let k = self.path2class(c, &name)?;
                let (kr, kc) = (c.vm().c_rational.get(), c.vm().c_complex.get());
                if k == kr || k == kc {
                    let idx = self.prepare(c);
                    let d = self.object(c)?;
                    if !is_array(d) || c.ary_len(d) != 2 {
                        let n = if is_array(d) { c.ary_len(d) } else { 0 };
                        return arg_err(c, format!("marshaled {} must have an array whose length is 2 but {}", if k == kr { "rational" } else { "complex" }, n));
                    }
                    let (x, y) = (c.ary_get(d, 0), c.ary_get(d, 1));
                    let v = if k == kr { crate::builtins::rational::rat_new(c, x, y) } else { crate::builtins::complex::cpx_raw(c, x, y) };
                    let v = self.entry0(c, v, idx);
                    self.post_proc(c, v)?
                } else {
                    let v = crate::builtins::object::alloc_instance(c, k)?;
                    if extmod != NIL { let n = c.ary_len(extmod); for i in (0..n).rev() { let m = c.ary_get(extmod, i); extend_object(c, v, m)?; } }
                    if !respond_to(c, v, intern(b"marshal_load"), true) { return type_err(c, format!("instance of {} needs to have method 'marshal_load'", ps(&name))); }
                    let v = self.entry(c, v);
                    let d = self.object(c)?;
                    funcall(c, v, intern(b"marshal_load"), &[d], std::ptr::null())?;
                    for (n, x) in ivar_list(c, d) { if !ivar_defined(c, v, n) { ivar_set_raw(c, v, n, x); } }
                    if self.freeze && v.is_heap() { obj_set_flags(v, FL_FROZEN); }
                    let v = self.post_proc(c, v)?;
                    if extmod != NIL { c.ary_mut(extmod).clear(); }
                    v
                }
            }
            b'o' => {
                let idx = self.prepare(c);
                let path = self.unique(c)?;
                let k = self.path2class(c, &path)?;
                if crate::builtins::inherits(c, k, c.vm().c_range.get()) {
                    // range_loader: the ivars of an Object, then the Range
                    let v = crate::builtins::object::alloc_instance(c, k)?;
                    self.entry0(c, v, idx);
                    let n = self.long(c)?;
                    let (mut b, mut e, mut excl) = (NIL, NIL, false);
                    for _ in 0..n.max(0) {
                        let name = self.symbol(c)?; let x = self.object(c)?;
                        match sym_bytes(name) { b"begin" => b = x, b"end" => e = x, b"excl" => excl = x.truthy(), _ => ivar_set_raw(c, v, name, x) }
                    }
                    c.range_set(v, b, e, excl);
                    obj_set_flags(v, FL_FROZEN);
                    self.leave(c, v, partial)?
                } else {
                    let v = crate::builtins::object::alloc_instance(c, k)?;
                    if !matches!(c.body_of(v), Body::Object(_)) || is_struct_class(c, k) || is_dir(c, v) { return arg_err(c, "dump format error"); }
                    self.entry0(c, v, idx);
                    self.r_ivar(c, v, &mut false)?;
                    self.leave(c, v, partial)?
                }
            }
            b'd' => {
                let name = self.unique(c)?;
                let k = self.path2class(c, &name)?;
                let v = crate::builtins::object::alloc_instance(c, k)?;
                if !matches!(c.body_of(v), Body::Data(_)) && !is_dir(c, v) { return arg_err(c, "dump format error"); }
                let v = self.entry(c, v);
                if !respond_to(c, v, intern(b"_load_data"), true) { return type_err(c, format!("class {} needs to have instance method '_load_data'", ps(&name))); }
                let r = self.object0(c, partial, None, extmod)?;
                funcall(c, v, intern(b"_load_data"), &[r], std::ptr::null())?;
                self.leave(c, v, partial)?
            }
            b'M' => {
                let b = self.rbytes(c)?;
                let v = path_to_class(c, &b)?;
                if ivp.as_deref().is_some_and(|x| *x) { return type_err(c, format!("can't override instance variable of class/module '{}'", ps(&b))); }
                let v = self.entry(c, v);
                self.leave(c, v, partial)?
            }
            b'c' | b'm' => {
                let str = self.r_string(c)?;
                if let Some(iv) = ivp.as_deref_mut() { if *iv { *iv = self.encname(c, str)? > 0; } }
                let b = c.str_bytes(str).to_vec();
                let v = if t == b'c' { self.path2class(c, &b)? } else { self.path2module(c, &b)? };
                if ivp.as_deref().is_some_and(|x| *x) { return type_err(c, format!("can't override instance variable of {} '{}'", if t == b'c' { "class" } else { "module" }, ps(&b))); }
                let v = self.entry(c, v);
                self.leave(c, v, partial)?
            }
            b':' => {
                let iv = match ivp.as_deref_mut() { Some(x) => { let r = *x; *x = false; r } None => false };
                let sy = self.symreal(c, iv)?;
                self.leave(c, Value::sym(sy), partial)?
            }
            b';' => Value::sym(self.symlink(c)?),
            _ => return arg_err(c, format!("dump format error(0x{:x})", t)),
        };
        if v == UNDEF { return arg_err(c, "dump format error (bad link)"); }
        Ok(v)
    }
    fn hash(&mut self, c: &mut Ctx, partial: bool, t: u8, ident: bool) -> RV {
        let n = self.len(c)?;
        let v = c.hash_new();
        if ident { c.hash_mut(v).identity = true; }
        self.entry(c, v);
        for _ in 0..n {
            let k = self.object(c)?;
            let x = self.object(c)?;
            hash_aset(c, v, k, x)?;
        }
        if t == b'}' { let d = self.object(c)?; c.hash_mut(v).default = d; }
        self.leave(c, v, partial)
    }
}

/// sym2encidx: the encoding an E / encoding ivar names (None: not an encoding ivar).
fn sym2enc(c: &mut Ctx, k: Sym, v: Value) -> Result<Option<u8>, Esc> {
    match sym_bytes(k) {
        b"encoding" => { let s = expect_str(c, v)?; Ok(crate::builtins::encoding::find_enc(c.str_bytes(s))) }
        b"E" if v == FALSE => Ok(Some(ENC_ASCII)),
        b"E" if v == TRUE => Ok(Some(ENC_UTF8)),
        _ => Ok(None),
    }
}
/// strtod on the text before the NUL: the longest leading float.
fn strtod(s: &[u8]) -> f64 {
    let t = String::from_utf8_lossy(s);
    let t = t.trim_start();
    let mut end = t.len();
    while end > 0 { if let Ok(f) = t[..end].parse::<f64>() { return f; } end -= 1; while end > 0 && !t.is_char_boundary(end) { end -= 1; } }
    0.0
}
/// A Regexp source dumped by 1.8 without an encoding: drop escapes 1.8 did not define.
fn strip_18_escapes(b: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(b.len());
    let mut bs = 0usize;
    for &ch in b {
        match ch {
            b'\\' => bs += 1,
            b'g' | b'h' | b'i' | b'j' | b'k' | b'l' | b'm' | b'o' | b'p' | b'q' | b'u' | b'y' | b'E' | b'F' | b'H' | b'I' | b'J' | b'K'
            | b'L' | b'N' | b'O' | b'P' | b'Q' | b'R' | b'S' | b'T' | b'U' | b'V' | b'X' | b'Y' => { if bs & 1 == 1 { out.pop(); } bs = 0; }
            _ => bs = 0,
        }
        out.push(ch);
    }
    out
}
/// rb_str_to_interned_str: a bare String becomes the shared frozen one.
fn interned_str(c: &mut Ctx, v: Value) -> Value {
    let has_iv = !ivar_list(c, v).is_empty();
    if obj_klass(v) != c.vm().c_string.get() || has_iv { obj_set_flags(v, FL_FROZEN); return v; }
    let key = c.str_bytes(v).to_vec();
    let e = c.str_enc(v);
    if let Some(&f) = c.vm().frozen_lits.borrow().get(&key) { if c.str_enc(f) == e { return f; } }
    let f = if is_frozen(v) { v } else { let f = c.str_new_enc(&key, e); obj_set_flags(f, FL_FROZEN); f };
    c.vm().frozen_lits.borrow_mut().insert(key, f);
    f
}

fn m_load(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    let (pos, opts) = kw_split(c, a);
    check_argc(c, pos, 1, 2)?;
    let objs = c.ary_new();
    let mut l = Loader { src: pos[0], data: vec![], io: false, pos: 0, syms: vec![], objs, partial: None, proc_: pos.get(1).copied().unwrap_or(NIL), freeze: false };
    if let Some(h) = opts {
        let mut bad = vec![];
        each_pair(c, h, |_c, k, _| { if k != sym_value("freeze") { bad.push(k); } Ok(()) })?;
        if !bad.is_empty() {
            let mut names = vec![];
            for k in bad { names.push(crate::builtins::kernel::inspect_str(c, k)?); }
            return arg_err(c, format!("unknown keyword{}: {}", if names.len() > 1 { "s" } else { "" }, names.join(", ")));
        }
        if let Some(v) = hash_get_opt(c, h, sym_value("freeze"))? { l.freeze = v.truthy(); }
    }
    if !is_string(l.src) {
        if respond_to(c, l.src, intern(b"to_str"), true) { l.src = expect_str(c, l.src)?; }
        else if respond_to(c, l.src, intern(b"getbyte"), false) && respond_to(c, l.src, intern(b"read"), false) { l.io = true; if respond_to(c, l.src, intern(b"binmode"), true) { funcall(c, l.src, intern(b"binmode"), &[], std::ptr::null())?; } }
        else { return io_needed(c); }
    }
    if l.proc_ != NIL || l.freeze { l.partial = Some(vec![]); }
    if !l.io { l.data = c.str_bytes(l.src).to_vec(); }
    let major = l.byte(c)?;
    let minor = l.byte(c)?;
    if major != 4 || minor > 8 {
        return type_err(c, format!("incompatible marshal file format (can't be read)\n\tformat version 4.8 required; {}.{} given", major, minor));
    }
    l.object(c)
}

// ---- Time#_dump / Time._load (time.c time_mdump / time_mload) ---------------------------------

fn call0(c: &mut Ctx, o: Value, m: &str) -> RV { funcall(c, o, intern(m.as_bytes()), &[], std::ptr::null()) }
fn call1(c: &mut Ctx, o: Value, m: &str, x: Value) -> RV { funcall(c, o, intern(m.as_bytes()), &[x], std::ptr::null()) }

fn time_dump(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    check_argc(c, a, 0, 1)?;
    let utc = call0(c, slf, "utc?")?.truthy();
    let g = call0(c, slf, "getutc")?;
    let mut f = [0i64; 6];
    let yv = call0(c, g, "year")?;
    for (i, m) in ["year", "mon", "mday", "hour", "min", "sec"].iter().enumerate().skip(1) { let x = call0(c, g, m)?; f[i] = num_to_long(c, x)?; }
    // nano = subsec * 10**9: whole ns (usec + nsec) and the rest
    let sub = call0(c, slf, "subsec")?;
    let nano = call1(c, sub, "*", Value::fix(1_000_000_000))?;
    let dm = call1(c, nano, "divmod", Value::fix(1))?;
    let (whole, subnano) = (c.ary_get(dm, 0), c.ary_get(dm, 1));
    let whole = num_to_long(c, whole)?;
    let (usec, nsec) = (whole / 1000, whole % 1000);
    let nano = call1(c, subnano, "+", Value::fix(nsec))?;
    let max_year = 1900 + 0xffff;
    let (year, ext) = if yv.is_fixnum() && (1900..=max_year).contains(&yv.as_fix()) { (yv.as_fix(), NIL) }
        else if call1(c, yv, ">", Value::fix(max_year))?.truthy() { (max_year, call1(c, yv, "-", Value::fix(max_year))?) }
        else { (1900, call1(c, Value::fix(1900), "-", yv)?) };
    f[0] = year;
    let p: u32 = 1 << 31 | (utc as u32) << 30 | ((f[0] - 1900) as u32) << 14 | ((f[1] - 1) as u32) << 10 | (f[2] as u32) << 5 | f[3] as u32;
    let sw: u32 = (f[4] as u32) << 26 | (f[5] as u32) << 20 | usec as u32;
    let mut buf = p.to_le_bytes().to_vec();
    buf.extend_from_slice(&sw.to_le_bytes());
    if ext != NIL {
        let big = c.to_big(ext);
        let mut yb: Vec<u8> = big.mag.iter().flat_map(|w| w.to_le_bytes()).collect();
        while yb.len() > 1 && *yb.last().unwrap() == 0 { yb.pop(); }
        if yb.len() > 4 { let ys = crate::builtins::kernel::inspect_str(c, yv)?; return arg_err(c, format!("year too {} to marshal: {} UTC", if year == 1900 { "small" } else { "big" }, ys)); }
        w_long(&mut buf, yb.len() as i64);
        buf.extend(yb);
    }
    let str = c.str_from_vec_enc(buf, ENC_BINARY);
    for (n, x) in ivar_list(c, slf) { if sym_bytes(n).starts_with(b"@") { ivar_set_raw(c, str, n, x); } }
    let (num, den) = if crate::builtins::rational::is_rational(c, nano) { crate::builtins::rational::rat_parts(c, nano) } else { (nano, Value::fix(1)) };
    if num != Value::fix(0) {
        ivar_set_raw(c, str, intern(b"nano_num"), num);
        ivar_set_raw(c, str, intern(b"nano_den"), den);
    }
    if nsec != 0 {
        let mut sm = vec![(((nsec / 100) << 4) | (nsec / 10 % 10)) as u8];
        let b1 = ((nsec % 10) << 4) as u8;
        if b1 != 0 { sm.push(b1); }
        let smv = c.str_from_vec_enc(sm, ENC_BINARY);
        ivar_set_raw(c, str, intern(b"submicro"), smv);
    }
    if !utc {
        let off = call0(c, slf, "utc_offset")?;
        let dm = call1(c, off, "divmod", Value::fix(1))?;
        let off = if c.ary_get(dm, 1) == Value::fix(0) { c.ary_get(dm, 0) } else { off };
        ivar_set_raw(c, str, intern(b"offset"), off);
    }
    // a zone object by its #name (NoMethodError without one)
    let mut zone = call0(c, slf, "zone")?;
    if zone != NIL && !is_string(zone) { zone = call0(c, zone, "name")?; }
    else if is_string(zone) && is_ascii(c.str_bytes(zone)) { zone = c.str_copy_enc(zone, ENC_ASCII); }
    ivar_set_raw(c, str, intern(b"zone"), zone);
    Ok(str)
}

fn time_load(c: &mut Ctx, k: Value, a: &[Value], _b: Blk) -> RV {
    let str = expect_str(c, a[0])?;
    let take = |c: &mut Ctx, n: &[u8]| ivar_remove(c, str, intern(n)).unwrap_or(NIL);
    let nano_num = take(c, b"nano_num"); let nano_den = take(c, b"nano_den"); let submicro = take(c, b"submicro");
    let mut offset = take(c, b"offset"); let zone = take(c, b"zone"); let year_iv = take(c, b"year");
    if offset != NIL && !(is_integer(offset) || crate::builtins::rational::is_rational(c, offset)) { offset = NIL; }
    let b = c.str_bytes(str).to_vec();
    if b.len() < 8 { return type_err(c, "marshaled time format differ"); }
    let p = u32::from_le_bytes([b[0], b[1], b[2], b[3]]);
    let sw = u32::from_le_bytes([b[4], b[5], b[6], b[7]]);
    let t = if p & (1 << 31) == 0 {
        offset = NIL;
        funcall(c, k, intern(b"at"), &[Value::fix(p as i64), Value::fix(sw as i64)], std::ptr::null())?
    } else {
        let p = p & !(1 << 31);
        let gmt = (p >> 30) & 1 != 0;
        let mut year = if year_iv != NIL { year_iv } else { Value::fix(((p >> 14) & 0xffff) as i64 + 1900) };
        if b.len() > 8 {
            let mut l = Loader { src: NIL, data: b[8..].to_vec(), io: false, pos: 0, syms: vec![], objs: NIL, partial: None, proc_: NIL, freeze: false };
            let ys = l.long(c)?;
            if ys < 0 || ys as usize > l.data.len() - l.pos { return type_err(c, "marshaled time format differ"); }
            let mut ext: i64 = 0;
            for (i, &x) in l.data[l.pos..l.pos + ys as usize].iter().enumerate().take(8) { ext |= (x as i64) << (8 * i); }
            year = if year == Value::fix(1900) { call1(c, year, "-", Value::fix(ext))? } else { call1(c, year, "+", Value::fix(ext))? };
        }
        let mut mon = ((p >> 10) & 0xf) as i64;
        if mon >= 12 { mon -= 12; year = call1(c, year, "+", Value::fix(1))?; }
        let (mday, hour) = (((p >> 5) & 0x1f) as i64, (p & 0x1f) as i64);
        let (min, sec, usec) = (((sw >> 26) & 0x3f) as i64, ((sw >> 20) & 0x3f) as i64, (sw & 0xfffff) as i64);
        let mut ns = Value::fix(usec * 1000);
        if nano_num != NIL {
            let q = call1(c, nano_num, "quo", nano_den)?;
            ns = call1(c, ns, "+", q)?;
        } else if is_string(submicro) {
            let sm = c.str_bytes(submicro).to_vec();
            let mut n = 0i64;
            let d = |x: u8| (x < 10).then_some(x as i64);
            if let Some(&b0) = sm.first() { if let (Some(h), Some(l)) = (d(b0 >> 4), d(b0 & 0xf)) { n = h * 100 + l * 10; if let Some(&b1) = sm.get(1) { if let Some(x) = d(b1 >> 4) { n += x; } } } }
            ns = Value::fix(usec * 1000 + n);
        }
        let base = funcall(c, k, intern(b"utc"), &[year, Value::fix(mon + 1), Value::fix(mday), Value::fix(hour), Value::fix(min), Value::fix(sec)], std::ptr::null())?;
        let secs = call0(c, base, "to_i")?;
        let t = funcall(c, k, intern(b"at"), &[secs, ns, sym_value("nsec")], std::ptr::null())?;
        if gmt { call0(c, t, "utc")?; offset = NIL; }
        t
    };
    if offset != NIL { call1(c, t, "localtime", offset)?; }
    if zone != NIL { crate::builtins::time::mload_zone(c, t, zone)?; }
    for (n, x) in ivar_list(c, str) { ivar_set_raw(c, t, n, x); }
    Ok(t)
}

/// Defines Marshal and returns it (boot).
pub fn init(c: &mut Ctx) -> Value {
    let m = define_module(c, "Marshal");
    define_module_function(c, m, "dump", m_dump, -1);
    define_module_function(c, m, "load", m_load, -1);
    define_module_function(c, m, "restore", m_load, -1);
    const_set(c, m, intern(b"MAJOR_VERSION"), Value::fix(4));
    const_set(c, m, intern(b"MINOR_VERSION"), Value::fix(8));
    m
}
/// Time's marshal hooks (boot, after Time is defined).
pub fn init_time(c: &mut Ctx, t: Value) {
    define_private_cfunc(c, t, "_dump", time_dump, -1);
    let st = singleton_class(c, t);
    define_private_cfunc(c, st, "_load", time_load, 1);
}

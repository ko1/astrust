// ObjectSpace: the allocation tracer, the objspace extension (objspace.so of CRuby 4.0.2:
// memsize_of, reachable_objects_from, dump, ...), ObjectSpace::WeakMap / WeakKeyMap and _id2ref.
//
// Allocation tracer: gc::set_alloc_hook makes every allocation call `allocated` (through
// GcLang::allocated, on the collector's slow path, which the fast path already tests for stress
// mode: nothing is added to an allocation while tracing is off).  A traced object is flagged
// FL_TRACED; the collector's finalize drops its entry (`forget`).
//
// Weak tables: a hidden Hash (no class) flagged FL_WEAK_KEYS (| FL_WEAK_VALS for WeakMap); the
// collector does not mark the weak parts and prunes the entries whose referents died (gc.rs).
#![forbid(unsafe_code)]
#[allow(unused_imports)]
use crate::heap::no_legacy::*;   // the old accessors cannot be named here
#[allow(unused_imports)]
use crate::heap::no_legacy_hn::*;
#[allow(unused_imports)]
use crate::heap::no_legacy_str::*;
use crate::array::*;
use crate::builtins::kernel::*;
use crate::hash::*;
use crate::heap::Global;
use crate::symbol::{intern, sym_bytes, sym_str};
use crate::value::*;
use crate::vm::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const PRELUDE_FILE: &str = "<internal:prelude>";

// ---- allocation tracer --------------------------------------------------------------------------

struct Info { file: &'static str, line: u32, path: Option<Rc<str>>, mid: Option<Sym>, generation: usize }

thread_local! {
    static TABLE: RefCell<crate::fxhash::HashMap<usize, Info>> = RefCell::new(Default::default());
    static PATHS: RefCell<std::collections::HashSet<Rc<str>>> = RefCell::new(Default::default());
    static RUNNING: Cell<u32> = const { Cell::new(0) };
}

/// The collector's hook: `v` was just allocated (GC.count = `generation`).  Allocates nothing.
pub fn allocated(c: &mut Ctx, v: Value, generation: usize) {
    if obj_klass(v) == NIL { return; }   // hidden (envs, weak tables)
    // CRuby 4.0 has no frame for Class#new (nor for a C `new` that is rb_class_new_instance, as Struct's)
    let mut f = c.cur_frame();
    while let Some(fr) = f { if fr.kind == FK_CFUNC && c.me_ref(fr.me).is_some_and(|m| m.name == crate::symbol::s::new || opt_insn(c, m)) { f = c.prev_frame(fr); } else { break; } }
    let Some(fr) = f else { return };
    // a method implemented in C (a cfunc, or saruby's prelude) reports its Ruby caller's place
    let mut g = Some(fr);
    while let Some(x) = g { if x.kind != FK_CFUNC && frame_file(x) != PRELUDE_FILE && frame_file(x) != "-" { break; } g = c.prev_frame(x); }
    let (file, line) = g.map_or((frame_file(fr), fr.line()), |x| (frame_file(x), x.line()));
    let (mid, owner) = c.me_ref(fr.me).map_or((None, NIL), |m| (Some(m.name), m.owner));
    let path = if owner.is_heap() && is_class_or_module(owner) && !c.class(owner).is_singleton {
        c.class(owner).name.filter(|&n| is_string(n)).map(|n| {
            let s = String::from_utf8_lossy(c.str_bytes(n));
            PATHS.with_borrow_mut(|p| match p.get(s.as_ref()) { Some(r) => r.clone(), None => { let r: Rc<str> = s.as_ref().into(); p.insert(r.clone()); r } })
        })
    } else { None };
    obj_set_flags(v, FL_TRACED);
    TABLE.with_borrow_mut(|t| t.insert(v.raw() as usize, Info { file, line, path, mid, generation }));
}

/// A builtin that CRuby runs as a specialized instruction (opt_plus, opt_aref, ...): no frame of its own.
fn opt_insn(c: &Ctx, m: &MethodEntry) -> bool {
    let vmr = c.vm();
    let (k, n) = (m.owner, sym_bytes(m.name));
    // the receivers each opt_ instruction of CRuby 4.0 handles itself
    if k == vmr.c_integer.get() || k == vmr.c_float.get() {
        matches!(n, b"+" | b"-" | b"*" | b"/" | b"%" | b"==" | b"!=" | b"<" | b"<=" | b">" | b">=" | b"&" | b"|" | b"succ")
    } else if k == vmr.c_string.get() {
        matches!(n, b"+" | b"==" | b"!=" | b"<<" | b"length" | b"size" | b"empty?" | b"succ" | b"=~")
    } else if k == vmr.c_array.get() {
        matches!(n, b"+" | b"<<" | b"[]" | b"[]=" | b"length" | b"size" | b"empty?")
    } else if k == vmr.c_hash.get() {
        matches!(n, b"[]" | b"[]=" | b"length" | b"size" | b"empty?")
    } else { false }
}

/// A traced object died (gc.rs finalize).
pub fn forget(addr: usize) { TABLE.with_borrow_mut(|t| t.remove(&addr)); }

fn with_info<R>(v: Value, f: impl FnOnce(&Info) -> R) -> Option<R> {
    if !v.is_heap() || obj_flags(v) & FL_TRACED == 0 { return None; }
    TABLE.with_borrow(|t| t.get(&(v.raw() as usize)).map(f))
}

fn tr_start(_c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV {
    if RUNNING.replace(RUNNING.get() + 1) == 0 {
        crate::gc::set_alloc_hook(true);
        // code compiled from now on keeps each statement's line in its frame (as for TracePoint)
        TRACE_COMPILE.store(true, std::sync::atomic::Ordering::Relaxed);
    }
    Ok(NIL)
}
fn tr_stop(_c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV {
    match RUNNING.get() { 0 => {} 1 => { RUNNING.set(0); crate::gc::set_alloc_hook(false); } n => RUNNING.set(n - 1) }
    Ok(NIL)
}
fn tr_clear(_c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV {
    // flags stay: forget and lookups tolerate a missing entry
    TABLE.with_borrow_mut(|t| t.clear());
    Ok(NIL)
}
fn tr_block(c: &mut Ctx, s: Value, _a: &[Value], b: *const Block) -> RV {
    tr_start(c, s, &[], b)?;
    let r = if b.is_null() { err(c, c.vm().c_local_jump_error.get(), "no block given (yield)") } else { yield1(c, b, NIL) };
    tr_stop(c, s, &[], b)?;
    r
}
fn alloc_file(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    match with_info(a[0], |i| i.file) { Some(f) => Ok(c.str_new(f.as_bytes())), None => Ok(NIL) }
}
fn alloc_line(_c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV { Ok(with_info(a[0], |i| Value::fix(i.line as i64)).unwrap_or(NIL)) }
fn alloc_path(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    match with_info(a[0], |i| i.path.clone()).flatten() { Some(p) => Ok(c.str_new(p.as_bytes())), None => Ok(NIL) }
}
fn alloc_mid(_c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV { Ok(with_info(a[0], |i| i.mid.map(Value::sym)).flatten().unwrap_or(NIL)) }
fn alloc_gen(_c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV { Ok(with_info(a[0], |i| Value::fix(i.generation as i64)).unwrap_or(NIL)) }

// ---- object kinds and sizes ---------------------------------------------------------------------

/// CRuby's T_xxx name of `v` (dump's "type", count_objects' keys without "T_").
pub fn type_name(c: &Ctx, v: Value) -> &'static str {
    if !v.is_heap() {
        return if v.is_fixnum() { "FIXNUM" } else if v.is_flonum() { "FLOAT" } else if v.is_sym() { "SYMBOL" }
            else if v == NIL { "NIL" } else if v == TRUE { "TRUE" } else { "FALSE" };
    }
    let vmr = c.vm();
    match c.body_of(v) {
        Body::Free(_) => "NONE",
        Body::Object(_) => if obj_klass(v) != NIL && is_kind_of(c, v, vmr.c_struct.get()) { "STRUCT" } else { "OBJECT" },
        Body::Class(cd) => if cd.is_module { "MODULE" } else { "CLASS" },
        Body::String(_) => "STRING",
        Body::Array(_) => "ARRAY",
        Body::Hash(_) => "HASH",
        Body::Float(_) => "FLOAT",
        Body::Bignum(_) => "BIGNUM",
        Body::Range(..) => "STRUCT",
        Body::Proc(_) | Body::Method(_) => "DATA",
        Body::Env(..) => "IMEMO",
        Body::Data(_) => {
            if c.data::<crate::regexp::RRegexp>(v).is_some() { "REGEXP" }
            else if c.data::<crate::regexp::RMatch>(v).is_some() { "MATCH" }
            else if c.data::<crate::builtins::rational::RRational>(v).is_some() { "RATIONAL" }
            else if c.data::<crate::builtins::complex::RComplex>(v).is_some() { "COMPLEX" }
            else if c.data::<crate::builtins::io::RIo>(v).is_some() { "FILE" }
            else { "DATA" }
        }
    }
}

/// Bytes `v` occupies: its slot and the payload it owns (ObjectSpace.memsize_of).
pub fn memsize(c: &Ctx, v: Value) -> usize {
    if !v.is_heap() { return 0; }
    let slot = std::mem::size_of::<RObj>();
    slot + match c.body_of(v) {
        Body::String(s) => if s.b.is_heap() { s.b.capacity() } else { 0 },
        Body::Array(a) => if a.is_heap() { a.capacity() * 8 } else { 0 },
        Body::Hash(h) => std::mem::size_of::<RHash>() + h.entries.capacity() * std::mem::size_of::<HEntry>() + h.index.capacity() * 4,
        Body::Object(iv) => if iv.is_inline() { 0 } else { iv.len() * 8 },
        Body::Class(cd) => std::mem::size_of::<ClassData>() + (cd.mtbl.len() + cd.consts.len() + cd.ivar_index.len()) * 16,
        Body::Bignum(b) => std::mem::size_of_val(&**b),
        Body::Proc(_) => std::mem::size_of::<ProcData>(),
        Body::Method(_) => std::mem::size_of::<MethodObj>(),
        Body::Data(d) => std::mem::size_of_val(&**d),
        Body::Env(e, _) => if e.is_heap() { std::mem::size_of_val(e) } else { 0 },
        Body::Float(_) | Body::Range(..) | Body::Free(_) => 0,
    }
}

/// Every allocated object, hidden ones included (they are not handed to Ruby code).
fn all_objects() -> Vec<Value> {
    let mut vs = vec![];
    crate::gc::each_object(|v| vs.push(v));
    vs
}
/// What ObjectSpace.each_object shows.
fn visible(c: &Ctx, v: Value) -> bool {
    obj_klass(v) != NIL && !matches!(c.body_of(v), Body::Env(..) | Body::Free(_)) && !(is_class_or_module(v) && c.class(v).is_singleton)
}

fn memsize_of(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV { Ok(Value::fix(memsize(c, a[0]) as i64)) }
fn memsize_of_all(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    check_argc(c, a, 0, 1)?;
    let k = a.first().copied();
    let total: usize = all_objects().into_iter().filter(|&v| visible(c, v) && k.is_none_or(|k| is_kind_of(c, v, k))).map(|v| memsize(c, v)).sum();
    Ok(Value::fix(total as i64))
}
/// A Hash of counts by key (count_objects_size, count_tdata_objects, ...), into `h` when given.
fn count_hash(c: &mut Ctx, a: &[Value], counts: Vec<(Value, usize)>) -> RV {
    let h = match a.first() { Some(&h) if is_hash(h) => { c.hash_clear(h); h } Some(_) => return type_err(c, "non-hash given"), None => c.hash_new() };
    for (k, n) in counts { hash_set(c, h, k, Value::fix(n as i64))?; }
    Ok(h)
}
fn tally<K: PartialEq + Copy>(out: &mut Vec<(K, usize)>, k: K, n: usize) {
    match out.iter_mut().find(|e| e.0 == k) { Some(e) => e.1 += n, None => out.push((k, n)) }
}
/// ObjectSpace.count_objects: slots, free slots, and objects by type.
pub fn count_objects(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    let st = crate::gc::stats();
    let mut by: Vec<(&'static str, usize)> = vec![];
    for v in all_objects() { tally(&mut by, type_name(c, v), 1); }
    let mut counts = vec![(sym_value("TOTAL"), st.slots), (sym_value("FREE"), st.free)];
    for t in ["OBJECT", "CLASS", "MODULE", "FLOAT", "STRING", "REGEXP", "ARRAY", "HASH", "STRUCT", "BIGNUM", "FILE", "DATA", "MATCH", "COMPLEX", "RATIONAL", "SYMBOL", "IMEMO"] {
        if let Some(&(_, n)) = by.iter().find(|e| e.0 == t) { counts.push((sym_value(&format!("T_{}", t)), n)); }
    }
    count_hash(c, a, counts)
}
fn count_objects_size(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    let mut by: Vec<(&'static str, usize)> = vec![];
    let mut total = 0;
    for v in all_objects() { let n = memsize(c, v); total += n; tally(&mut by, type_name(c, v), n); }
    let mut counts: Vec<(Value, usize)> = by.into_iter().map(|(t, n)| (sym_value(&format!("T_{}", t)), n)).collect();
    counts.push((sym_value("TOTAL"), total));
    count_hash(c, a, counts)
}
fn count_tdata_objects(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    let mut by: Vec<(Value, usize)> = vec![];
    for v in all_objects() {
        if matches!(c.body_of(v), Body::Data(_) | Body::Proc(_) | Body::Method(_)) && type_name(c, v) == "DATA" {
            let k = obj_klass(v);
            let k = if k == NIL { sym_value("hidden") } else { real_class_of(c, v) };
            tally(&mut by, k, 1);
        }
    }
    count_hash(c, a, by)
}
fn count_imemo_objects(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    let n = all_objects().into_iter().filter(|&v| matches!(c.body_of(v), Body::Env(..))).count();
    count_hash(c, a, if n > 0 { vec![(sym_value("imemo_env"), n)] } else { vec![] })
}
fn count_symbols(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    // saruby's symbols are never collected: all immortal
    let n = crate::symbol::all_count();
    count_hash(c, a, vec![(sym_value("mortal_dynamic_symbol"), 0), (sym_value("immortal_dynamic_symbol"), 0), (sym_value("immortal_static_symbol"), n), (sym_value("immortal_symbol"), n)])
}

// ---- references -------------------------------------------------------------------------------

static WRAPPER: Global = Global::new();   // ObjectSpace::InternalObjectWrapper

/// An object Ruby code must not hold (no class): wrapped, as CRuby wraps internal objects.
fn wrap_internal(c: &mut Ctx, v: Value) -> Value {
    if obj_klass(v) != NIL { return v; }
    let k = WRAPPER.get(c);
    let w = new_object(c, k);
    ivar_set_raw(c, w, intern(b"__obj"), v);
    w
}
fn iow_obj(c: &Ctx, s: Value) -> Value { ivar_get(c, s, intern(b"__obj")) }
fn iow_type(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { let o = iow_obj(c, s); Ok(sym_value(&format!("T_{}", type_name(c, o)))) }
fn iow_id(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { let o = iow_obj(c, s); Ok(Value::fix(object_id(o) as i64)) }
fn iow_inspect(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV {
    let o = iow_obj(c, s);
    Ok(c.str_new(format!("#<InternalObject:0x{:016x} T_{}>", o.raw(), type_name(c, o)).as_bytes()))
}

fn reachable_objects_from(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    let o = a[0];
    if !o.is_heap() { return Ok(NIL); }
    // weak tables are not shown (CRuby's WeakMap keeps its table outside the heap)
    let vs: Vec<Value> = crate::gc::reachable_from(c, o).into_iter().filter(|&v| obj_flags(v) & (FL_WEAK_KEYS | FL_WEAK_VALS) == 0).collect();
    let n = vs.len();
    let r = c.ary_from_vec(vs);
    for i in 0..n { let v = c.ary_get(r, i); let w = wrap_internal(c, v); c.ary_mut(r).vec()[i] = w; }
    Ok(r)
}
fn reachable_objects_from_root(c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV {
    let vs = crate::gc::exact_roots(c);
    let r = c.ary_from_vec(vs);
    let h = c.hash_new();
    let k = c.str_new(b"vm");
    hash_set(c, h, k, r)?;
    Ok(h)
}
fn internal_class_of(_c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    let k = class_of(a[0]);
    Ok(if k == NIL { NIL } else { k })
}
fn internal_super_of(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    if !is_class_or_module(a[0]) { return arg_err(c, "class or module is expected"); }
    Ok(c.class(a[0]).superclass)
}

// ---- dump ---------------------------------------------------------------------------------------

/// "%#g", as dump.c prints a Float.
fn fmt_g(x: f64) -> String {
    if !x.is_finite() { return if x.is_nan() { "NaN".into() } else if x > 0.0 { "Inf".into() } else { "-Inf".into() }; }
    if x == 0.0 { return if x.is_sign_negative() { "-0.00000".into() } else { "0.00000".into() }; }
    let e = format!("{:.5e}", x);
    let (m, ex) = e.split_once('e').unwrap_or((&e, "0"));
    let ex: i32 = ex.parse().unwrap_or(0);
    if !(-4..6).contains(&ex) { format!("{}e{}{:02}", m, if ex < 0 { '-' } else { '+' }, ex.abs()) } else { format!("{:.*}", (5 - ex) as usize, x) }
}

fn json_str(out: &mut String, b: &[u8]) {
    out.push('"');
    for &ch in b {
        match ch {
            b'"' => out.push_str("\\\""), b'\\' => out.push_str("\\\\"), b'\n' => out.push_str("\\n"), b'\r' => out.push_str("\\r"),
            b'\t' => out.push_str("\\t"), 0x0c => out.push_str("\\f"), 0x08 => out.push_str("\\b"),
            0..=0x1f | 0x7f.. => out.push_str(&format!("\\u{:04x}", ch)),
            _ => out.push(ch as char),
        }
    }
    out.push('"');
}

fn addr(v: Value) -> String { format!("\"0x{:x}\"", v.raw()) }

/// One object as dump.c prints it (one line, no newline).
fn dump_obj(c: &mut Ctx, v: Value) -> String {
    if !v.is_heap() {
        if v.is_fixnum() { return v.as_fix().to_string(); }
        if v.is_flonum() { return fmt_g(v.flonum_val()); }
        if v.is_sym() { let mut s = String::from("{\"type\":\"SYMBOL\", \"value\":"); json_str(&mut s, sym_bytes(v.as_sym())); s.push('}'); return s; }
        return (if v == NIL { "null" } else if v == TRUE { "true" } else { "false" }).into();
    }
    let t = type_name(c, v);
    let mut s = format!("{{\"address\":{}, \"type\":\"{}\", \"shape_id\":0, \"slot_size\":{}", addr(v), t, std::mem::size_of::<RObj>());
    let k = obj_klass(v);
    if k != NIL { s.push_str(&format!(", \"class\":{}", addr(k))); }
    if obj_flags(v) & FL_FROZEN != 0 { s.push_str(", \"frozen\":true"); }
    match c.body_of(v) {
        Body::String(r) => {
            let (heap, enc, b) = (r.b.is_heap(), r.enc, c.str_bytes(v));
            if !heap { s.push_str(", \"embedded\":true"); }
            if obj_flags(v) & FL_CHILLED != 0 { s.push_str(", \"chilled\":true"); }
            s.push_str(&format!(", \"bytesize\":{}", b.len()));
            let ascii = b.is_ascii();
            if ascii { s.push_str(", \"value\":"); json_str(&mut s, b); }
            if enc != ENC_BINARY { s.push_str(&format!(", \"encoding\":\"{}\"", crate::builtins::encoding::enc_name(enc))); }
            let cr = if ascii { "7bit" } else if crate::builtins::encoding::is_valid(enc, b) { "valid" } else { "broken" };
            s.push_str(&format!(", \"coderange\":\"{}\"", cr));
        }
        Body::Array(a) => { s.push_str(&format!(", \"length\":{}", a.len())); if !a.is_heap() { s.push_str(", \"embedded\":true"); } }
        Body::Hash(h) => s.push_str(&format!(", \"size\":{}", h.live)),
        Body::Object(iv) => { if iv.is_inline() { s.push_str(", \"embedded\":true"); } s.push_str(&format!(", \"ivars\":{}", (0..iv.len()).filter(|&i| iv.get(i).is_some_and(|x| x != UNDEF)).count())); }
        Body::Float(f) => s.push_str(&format!(", \"value\":\"{}\"", fmt_g(*f))),
        Body::Class(cd) => {
            if cd.superclass != NIL { s.push_str(&format!(", \"superclass\":{}", addr(cd.superclass))); }
            if let Some(n) = cd.name.filter(|&n| is_string(n)) { s.push_str(", \"name\":"); json_str(&mut s, c.str_bytes(n)); }
        }
        Body::Proc(_) => s.push_str(", \"struct\":\"proc\""),
        Body::Method(_) => s.push_str(", \"struct\":\"method\""),
        Body::Data(_) if t == "DATA" => { let n = class_path(c, real_class_of(c, v)).to_lowercase(); s.push_str(&format!(", \"struct\":\"{}\"", n.rsplit("::").next().unwrap_or(""))); }
        _ => {}
    }
    let refs: Vec<String> = crate::gc::reachable_from(c, v).into_iter().filter(|&r| r != k).map(addr).collect();
    if !refs.is_empty() { s.push_str(&format!(", \"references\":[{}]", refs.join(", "))); }
    let _ = with_info(v, |i| {
        s.push_str(", \"file\":"); json_str(&mut s, i.file.as_bytes());
        s.push_str(&format!(", \"line\":{}", i.line));
        if let Some(m) = i.mid { s.push_str(", \"method\":"); json_str(&mut s, sym_bytes(m)); }
        s.push_str(&format!(", \"generation\":{}", i.generation));
    });
    s.push_str(&format!(", \"memsize\":{}, \"flags\":{{\"wb_protected\":true}}}}", memsize(c, v)));
    s
}

/// Append to a String or write to an IO (dump.c's output).
fn emit(c: &mut Ctx, out: Value, b: &[u8]) -> Result<(), Esc> {
    if is_string(out) { c.str_modify(out)?.b.extend_from_slice(b); Ok(()) } else { let st = c.str_new(b); funcall(c, out, intern(b"write"), &[st], std::ptr::null()).map(|_| ()) }
}
// ObjectSpace._dump(obj, out)
fn os_dump(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    let mut line = dump_obj(c, a[0]);
    // dump.c ends the line of a heap object only
    if a[0].is_heap() { line.push('\n'); }
    emit(c, a[1], line.as_bytes())?;
    Ok(a[1])
}
// ObjectSpace._dump_all(out, full, since, shapes)
fn os_dump_all(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    let out = a[0];
    let since = if a[2] == NIL { None } else { Some(a[2].as_fix().max(0) as usize) };
    let roots: Vec<String> = crate::gc::exact_roots(c).into_iter().map(addr).collect();
    let r = format!("{{\"type\":\"ROOT\", \"root\":\"vm\", \"references\":[{}]}}\n", roots.join(", "));
    emit(c, out, r.as_bytes())?;
    // dumped first, written after: writing allocates, and the list must not see its own strings
    let mut text = String::new();
    for v in all_objects() {
        if matches!(c.body_of(v), Body::Free(_)) || obj_flags(v) & (FL_WEAK_KEYS | FL_WEAK_VALS) != 0 { continue; }
        if let Some(g) = since { if with_info(v, |i| i.generation).is_none_or(|x| x < g) { continue; } }
        text.push_str(&dump_obj(c, v));
        text.push('\n');
    }
    emit(c, out, text.as_bytes())?;
    Ok(out)
}
// ObjectSpace._dump_shapes(out, since): saruby has no shapes
fn os_dump_shapes(_c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV { Ok(a[0]) }

/// `require 'objspace.so'`.
pub fn init_ext(c: &mut Ctx) {
    let os = c.vm().m_object_space.get();
    let fs: [(&str, CFunc, i32); 22] = [
        ("trace_object_allocations_start", tr_start, 0), ("trace_object_allocations_stop", tr_stop, 0),
        ("trace_object_allocations_clear", tr_clear, 0), ("trace_object_allocations", tr_block, 0),
        ("trace_object_allocations_debug_start", tr_start, 0),
        ("allocation_sourcefile", alloc_file, 1), ("allocation_sourceline", alloc_line, 1), ("allocation_class_path", alloc_path, 1),
        ("allocation_method_id", alloc_mid, 1), ("allocation_generation", alloc_gen, 1),
        ("memsize_of", memsize_of, 1), ("memsize_of_all", memsize_of_all, -1), ("count_objects_size", count_objects_size, -1),
        ("count_nodes", |c, _s, a, _b| count_hash(c, a, vec![]), -1), ("count_tdata_objects", count_tdata_objects, -1),
        ("count_imemo_objects", count_imemo_objects, -1), ("count_symbols", count_symbols, -1),
        ("reachable_objects_from", reachable_objects_from, 1), ("reachable_objects_from_root", reachable_objects_from_root, 0),
        ("internal_class_of", internal_class_of, 1), ("internal_super_of", internal_super_of, 1), ("_dump", os_dump, 2),
    ];
    for (n, f, ar) in fs { define_module_function(c, os, n, f, ar); }
    define_singleton_cfunc(c, os, "_dump_all", os_dump_all, 4);
    define_singleton_cfunc(c, os, "_dump_shapes", os_dump_shapes, 2);
    let w = define_class_under(c, os, "InternalObjectWrapper", c.vm().c_object.get());
    WRAPPER.set(c, w);
    define_cfunc(c, w, "type", iow_type, 0);
    define_cfunc(c, w, "internal_object_id", iow_id, 0);
    define_cfunc(c, w, "inspect", iow_inspect, 0);
}

// ---- WeakMap / WeakKeyMap -----------------------------------------------------------------------

/// The table of a WeakMap / WeakKeyMap, made on first use (allocate without initialize).
fn table(c: &mut Ctx, s: Value) -> Value {
    let h = ivar_get(c, s, intern(b"__h"));
    if h != NIL { return h; }
    let map = obj_klass(s) != NIL && is_kind_of(c, s, c.vm().c_object_space_weak_map.get());
    let h = c.alloc(NIL, Body::Hash(Box::new(RHash::with_capacity(0))));
    // WeakMap: by identity, keys and values weak; WeakKeyMap: by eql?, keys weak
    c.hash_mut(h).identity = map;
    obj_set_flags(h, if map { FL_WEAK_KEYS | FL_WEAK_VALS } else { FL_WEAK_KEYS });
    ivar_set_raw(c, s, intern(b"__h"), h);
    h
}
fn wm_init(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { table(c, s); Ok(NIL) }
fn wm_aref(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV { let h = table(c, s); Ok(hash_get_opt(c, h, a[0])?.unwrap_or(NIL)) }
fn wm_aset(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV { let h = table(c, s); hash_set(c, h, a[0], a[1])?; Ok(a[1]) }
fn wm_key(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV { let h = table(c, s); Ok(Value::bool(hash_lookup(c, h, a[0])?.is_some())) }
fn wm_size(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { let h = table(c, s); Ok(Value::fix(c.hash_len(h) as i64)) }
fn wm_delete(c: &mut Ctx, s: Value, a: &[Value], b: *const Block) -> RV {
    let h = table(c, s);
    match hash_delete(c, h, a[0])? { Some(v) => Ok(v), None => if b.is_null() { Ok(NIL) } else { yield1(c, b, a[0]) } }
}
/// The pairs of the table, slot by slot (the block may change it; a collection may prune it).
fn wm_walk(c: &mut Ctx, s: Value, mut f: impl FnMut(&mut Ctx, Value, Value) -> Result<(), Esc>) -> Result<(), Esc> {
    let h = table(c, s);
    let mut i = 0;
    while let Some((k, v)) = c.hash_entry_at(h, i) { if k != UNDEF { f(c, k, v)?; } i += 1; }
    Ok(())
}
fn wm_list(c: &mut Ctx, s: Value, vals: bool) -> RV {
    let r = c.ary_new();
    wm_walk(c, s, |c, k, v| { c.ary_push(r, if vals { v } else { k }); Ok(()) })?;
    Ok(r)
}
fn wm_keys(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { wm_list(c, s, false) }
fn wm_values(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { wm_list(c, s, true) }
fn wm_each(c: &mut Ctx, s: Value, _a: &[Value], b: *const Block) -> RV {
    if b.is_null() {
        // CRuby yields only when there is an entry
        let h = table(c, s);
        if c.hash_len(h) == 0 { return Ok(s); }
        return err(c, c.vm().c_local_jump_error.get(), "no block given (yield)");
    }
    let name = c.cur_method_name().unwrap_or(crate::symbol::s::each);
    wm_walk(c, s, |c, k, v| {
        match sym_bytes(name) { b"each_key" => { yield1(c, b, k)?; } b"each_value" => { yield1(c, b, v)?; } _ => { yield_block(c, b, &[k, v])?; } }
        Ok(())
    })?;
    Ok(s)
}
fn wm_inspect(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV {
    let mut parts = vec![];
    // as weakmap.c: immediates by #inspect, objects by rb_any_to_s
    let show = |c: &mut Ctx, v: Value| if v.is_imm() { inspect_str(c, v) } else { Ok(any_to_s_str(c, v)) };
    wm_walk(c, s, |c, k, v| { parts.push(format!("{} => {}", show(c, k)?, show(c, v)?)); Ok(()) })?;
    Ok(c.str_new(format!("#<{}:0x{:016x}{}{}>", class_path(c, real_class_of(c, s)), s.raw(), if parts.is_empty() { "" } else { ": " }, parts.join(", ")).as_bytes()))
}
fn wkm_aset(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    if a[0].is_imm() || is_float(a[0]) || is_bignum(a[0]) { return arg_err(c, "WeakKeyMap keys must be garbage collectable"); }
    let h = table(c, s);
    hash_set(c, h, a[0], a[1])?;
    Ok(a[1])
}
fn wkm_getkey(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    let h = table(c, s);
    Ok(match hash_lookup(c, h, a[0])? { Some(i) => c.hash_entry_at(h, i).map(|e| e.0).unwrap_or(NIL), None => NIL })
}
fn wkm_clear(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { let h = table(c, s); c.hash_clear(h); Ok(s) }
fn wkm_inspect(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV {
    let h = table(c, s);
    let n = c.hash_len(h);
    Ok(c.str_new(format!("#<{}:0x{:016x} size={}>", class_path(c, real_class_of(c, s)), s.raw(), n).as_bytes()))
}

// ---- core ObjectSpace leftovers -----------------------------------------------------------------

/// The Symbol or flonum whose object_id (its own word) is `n`.
fn imm_of_id(n: u64) -> Option<Value> {
    if n & 3 == 2 {
        let t = if n == 0x8000000000000002 { 0 } else { 2u64.wrapping_sub(n >> 63) | (n & !3) };
        return Value::flonum(f64::from_bits(t.rotate_right(3))).filter(|v| v.raw() == n);
    }
    let s = Value::sym(Sym((n >> 8) as u32));
    if s.raw() == n && ((n >> 8) as usize) < crate::symbol::all_count() { Some(s) } else { None }
}

// ObjectSpace._id2ref (deprecated in 4.0)
fn os_id2ref(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    warn_deprecated(c, "ObjectSpace._id2ref is deprecated")?;
    let id = a[0];
    if !is_integer(id) { return type_err(c, format!("wrong argument type {} (expected Integer)", class_name_for_err(c, id))); }
    let shown = || -> String { if id.is_fixnum() { format!("0x{:016x}", id.as_fix()) } else { "a large id".into() } };
    if id.is_fixnum() {
        let n = id.as_fix() as u64;
        // immediates: object_id is n*2+1 for an Integer, the word itself otherwise
        if n & 1 == 1 { return Ok(Value::fix((n as i64 - 1) / 2)); }
        for v in [NIL, TRUE, FALSE] { if object_id(v) == n { return Ok(v); } }
        if let Some(v) = imm_of_id(n) { return Ok(v); }
        let found = c.vm().object_ids.borrow().iter().find(|e| *e.1 == n).map(|e| *e.0);
        if let Some(raw) = found {
            if let Some(v) = all_objects().into_iter().find(|v| v.raw() == raw) { if visible(c, v) { return Ok(v); } }
        }
    }
    let m = format!("{} is not id value", shown());
    let m = if id.is_fixnum() { m } else { format!("{} is not id value", inspect_str(c, id)?) };
    err(c, c.vm().c_range_error.get(), m)
}

/// Boot: ObjectSpace::WeakMap, WeakKeyMap, _id2ref.
pub fn init_core(c: &mut Ctx) {
    let vmr = c.vm();
    let os = vmr.m_object_space.get();
    define_module_function(c, os, "_id2ref", os_id2ref, 1);
    let wm = define_class_under(c, os, "WeakMap", vmr.c_object.get());
    vmr.c_object_space_weak_map.set(wm);
    include_module(c, wm, vmr.m_enumerable.get());
    define_private_cfunc(c, wm, "initialize", wm_init, 0);
    define_cfunc(c, wm, "[]", wm_aref, 1);
    define_cfunc(c, wm, "[]=", wm_aset, 2);
    for n in ["key?", "include?", "member?"] { define_cfunc(c, wm, n, wm_key, 1); }
    for n in ["size", "length"] { define_cfunc(c, wm, n, wm_size, 0); }
    define_cfunc(c, wm, "delete", wm_delete, 1);
    define_cfunc(c, wm, "keys", wm_keys, 0);
    define_cfunc(c, wm, "values", wm_values, 0);
    for n in ["each", "each_pair", "each_key", "each_value"] { define_cfunc(c, wm, n, wm_each, 0); }
    define_cfunc(c, wm, "inspect", wm_inspect, 0);
    let _ = crate::builtins::object::alias_method(c, wm, intern(b"member?"), intern(b"include?"));
    let wk = define_class_under(c, os, "WeakKeyMap", vmr.c_object.get());
    define_private_cfunc(c, wk, "initialize", wm_init, 0);
    define_cfunc(c, wk, "[]", wm_aref, 1);
    define_cfunc(c, wk, "[]=", wkm_aset, 2);
    define_cfunc(c, wk, "getkey", wkm_getkey, 1);
    define_cfunc(c, wk, "key?", wm_key, 1);
    define_cfunc(c, wk, "delete", wm_delete, 1);
    define_cfunc(c, wk, "clear", wkm_clear, 0);
    define_cfunc(c, wk, "inspect", wkm_inspect, 0);
    let _ = sym_str;
}

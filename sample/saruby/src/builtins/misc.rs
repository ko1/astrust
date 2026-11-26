// Assorted built-ins: pack/unpack, GC, ObjectSpace, Warning, RbConfig, constants.
// On the context-borrowing heap accessors (heap.rs, docs/safe_heap.md).
#![forbid(unsafe_code)]
#[allow(unused_imports)]
use crate::heap::no_legacy::*;   // the old accessors cannot be named here
#[allow(unused_imports)]
use crate::heap::no_legacy_hn::*;
#[allow(unused_imports)]
use crate::heap::no_legacy_str::*;
use crate::heap::{obj_klass, obj_set_flags, Global};
use crate::array::*;
use crate::builtins::kernel::*;
use crate::builtins::numeric_methods::num_to_long;
use crate::hash::*;
use crate::symbol::intern;
use crate::value::*;
use crate::vm::*;
use std::sync::atomic::AtomicBool;

// ---- Encoding (the methods and objects are encoding.rs's) --------------------------------

pub struct REnc { pub id: u8 }
impl RData for REnc {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}
pub fn encoding_value(c: &mut Ctx, id: u8) -> RV { Ok(crate::builtins::encoding::encoding_value(c, id)) }

// ---- base64 (pack itself is pack.rs) ----------------------------------------------------------------------

pub fn base64(b: &[u8]) -> String {
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for ch in b.chunks(3) {
        let n = (ch[0] as u32) << 16 | (*ch.get(1).unwrap_or(&0) as u32) << 8 | *ch.get(2).unwrap_or(&0) as u32;
        out.push(T[(n >> 18) as usize & 63] as char); out.push(T[(n >> 12) as usize & 63] as char);
        out.push(if ch.len() > 1 { T[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if ch.len() > 2 { T[n as usize & 63] as char } else { '=' });
    }
    out
}
pub fn unbase64(s2: &[u8]) -> Vec<u8> {
    let mut out = vec![]; let mut buf = 0u32; let mut bits = 0;
    for &x in s2 {
        let v = match x { b'A'..=b'Z' => x - b'A', b'a'..=b'z' => x - b'a' + 26, b'0'..=b'9' => x - b'0' + 52, b'+' | b'-' => 62, b'/' | b'_' => 63, _ => continue } as u32;
        buf = buf << 6 | v; bits += 6;
        if bits >= 8 { bits -= 8; out.push((buf >> bits) as u8); }
    }
    out
}
// ---- GC / ObjectSpace -----------------------------------------------------------------------

fn gc_start(c: &mut Ctx, _s: Value, _a: &[Value], _b: Blk) -> RV { explicit_gc(c, true); Ok(NIL) }
// GC.__start(full_mark), behind the prelude's GC.start (keywords as in CRuby's gc.rb, no Hash)
fn gc_start_full(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV { explicit_gc(c, a[0].truthy()); Ok(NIL) }
static LAST_EXPLICIT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(usize::MAX);   // GC.count after the last GC.start
static LAST_FULL: AtomicBool = AtomicBool::new(false);
fn explicit_gc(c: &mut Ctx, full: bool) {
    if full { crate::gc::collect(c) } else { crate::gc::collect_minor(c) }
    LAST_EXPLICIT.store(crate::gc::collections(), Relaxed);
    LAST_FULL.store(full, Relaxed);
    run_finalizer_queue(c, FinAt::Internal(44));
}
fn gc_count(_c: &mut Ctx, _s: Value, _a: &[Value], _b: Blk) -> RV { Ok(Value::fix(crate::gc::collections() as i64)) }
fn gc_disable(_c: &mut Ctx, _s: Value, _a: &[Value], _b: Blk) -> RV { Ok(Value::bool(crate::gc::set_disabled(true))) }
fn gc_enable(_c: &mut Ctx, _s: Value, _a: &[Value], _b: Blk) -> RV { Ok(Value::bool(crate::gc::set_disabled(false))) }
/// CRuby's heap pages (64 KiB) in slots of this heap's size: the page counts GC.stat reports.
fn page_slots() -> usize { 65536 / std::mem::size_of::<RObj>() }
/// GC.stat's keys in CRuby 4.0's order. saruby has one heap without generations, compaction or
/// malloc accounting: those keys are 0 (limits: CRuby's defaults); marking_time is the whole time.
fn gc_stat_pairs() -> Vec<(&'static str, i64)> {
    let (n, m, h) = (crate::gc::collections() as i64, crate::gc::minor_collections() as i64, crate::gc::stats());
    let ms = crate::gc::total_time().as_millis() as i64;
    let pages = h.slots.div_ceil(page_slots()) as i64;
    let live = (h.slots - h.free) as i64;
    vec![("count", n), ("time", ms), ("marking_time", ms), ("sweeping_time", 0), ("heap_allocated_pages", pages), ("heap_empty_pages", 0),
         ("heap_allocatable_slots", 0), ("heap_available_slots", h.slots as i64), ("heap_live_slots", live), ("heap_free_slots", h.free as i64),
         ("heap_final_slots", 0), ("heap_marked_slots", live), ("heap_eden_pages", pages), ("total_allocated_pages", pages), ("total_freed_pages", 0),
         ("total_allocated_objects", h.allocated as i64), ("total_freed_objects", h.freed as i64), ("malloc_increase_bytes", 0),
         ("malloc_increase_bytes_limit", 16 << 20), ("minor_gc_count", m), ("major_gc_count", n - m), ("compact_count", 0), ("read_barrier_faults", 0),
         ("total_moved_objects", 0), ("remembered_wb_unprotected_objects", 0), ("remembered_wb_unprotected_objects_limit", 0), ("old_objects", 0),
         ("old_objects_limit", 0), ("oldmalloc_increase_bytes", 0), ("oldmalloc_increase_bytes_limit", 16 << 20)]
}
/// GC.stat / GC.latest_gc_info / GC.stat_heap's argument: nil (a new Hash), a Hash (filled), or a Symbol (that value).
fn stat_reply(c: &mut Ctx, arg: Option<Value>, pairs: Vec<(&'static str, Value)>) -> RV {
    match arg {
        Some(k) if k.is_sym() => match pairs.iter().find(|p| sym_value(p.0) == k) {
            Some(p) => Ok(p.1),
            None => arg_err(c, format!("unknown key: {}", crate::symbol::sym_str(k.as_sym()))),
        },
        Some(h) if is_hash(h) => { for (k, v) in pairs { hash_set(c, h, sym_value(k), v)?; } Ok(h) }
        Some(x) if x != NIL => type_err(c, "non-hash or symbol given"),
        _ => { let h = c.hash_new(); for (k, v) in pairs { hash_set(c, h, sym_value(k), v)?; } Ok(h) }
    }
}
fn gc_stat(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    check_argc(c, a, 0, 1)?;
    let pairs = gc_stat_pairs().into_iter().map(|(k, v)| (k, Value::fix(v))).collect();
    stat_reply(c, a.first().copied(), pairs)
}
/// GC.latest_gc_info: what saruby knows of the last collection (GC.start or an allocation).
fn gc_latest_info(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    check_argc(c, a, 0, 1)?;
    let n = crate::gc::collections();
    let explicit = n != 0 && LAST_EXPLICIT.load(Relaxed) == n;
    let major = if explicit && LAST_FULL.load(Relaxed) { sym_value("force") } else { NIL };
    let pairs = vec![("major_by", major), ("need_major_by", NIL), ("gc_by", sym_value(if explicit { "method" } else { "newobj" })),
        ("have_finalizer", FALSE), ("immediate_sweep", Value::bool(explicit)), ("state", sym_value("none")),
        ("weak_references_count", Value::fix(0)), ("retained_weak_references_count", Value::fix(0))];
    stat_reply(c, a.first().copied(), pairs)
}
/// GC.stat_heap(heap = nil, hash_or_key = nil): one heap (0).
fn gc_stat_heap(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    check_argc(c, a, 0, 2)?;
    let h = crate::gc::stats();
    let pages = h.slots.div_ceil(page_slots()) as i64;
    let v = |x: i64| Value::fix(x);
    let pairs = vec![("slot_size", v(std::mem::size_of::<RObj>() as i64)), ("heap_live_slots", v((h.slots - h.free) as i64)), ("heap_free_slots", v(h.free as i64)),
        ("heap_final_slots", v(0)), ("heap_eden_pages", v(pages)), ("heap_eden_slots", v(h.slots as i64)), ("total_allocated_pages", v(pages)),
        ("force_major_gc_count", v(0)), ("force_incremental_marking_finish_count", v(0)), ("total_allocated_objects", v(h.allocated as i64)), ("total_freed_objects", v(h.freed as i64))];
    match a.first().copied() {
        None | Some(NIL) => {
            let all = if let Some(&x) = a.get(1) { if !is_hash(x) { return type_err(c, "non-hash given"); } x } else { c.hash_new() };
            let one = c.hash_new();
            stat_reply(c, Some(one), pairs)?;
            hash_set(c, all, Value::fix(0), one)?;
            Ok(all)
        }
        Some(i) if i == Value::fix(0) => stat_reply(c, a.get(1).copied(), pairs),
        Some(i) if i.is_fixnum() => arg_err(c, "size pool index out of range"),
        Some(i) => { let k = class_name_for_err(c, i); type_err(c, format!("no implicit conversion of {} into Integer", k)) }
    }
}
// GC.config / auto_compact / measure_total_time / stress: settings saruby keeps (stress is real)
static ALLOW_FULL_MARK: AtomicBool = AtomicBool::new(true);
static AUTO_COMPACT: AtomicBool = AtomicBool::new(false);
static MEASURE_TIME: AtomicBool = AtomicBool::new(true);
static STRESS_V: Global = Global::new();
fn gc_config(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    check_argc(c, a, 0, 1)?;
    if let Some(&h) = a.first() { if h != NIL {
        if !is_hash(h) { return arg_err(c, "ArgumentError"); }
        if hash_get_opt(c, h, sym_value("implementation"))?.is_some() { return arg_err(c, "Attempting to set read-only key \"Implementation\""); }
        if let Some(v) = hash_get_opt(c, h, sym_value("rgengc_allow_full_mark"))? { ALLOW_FULL_MARK.store(v.truthy(), Relaxed); }
    } }
    let h = c.hash_new();
    hash_set(c, h, sym_value("rgengc_allow_full_mark"), Value::bool(ALLOW_FULL_MARK.load(Relaxed)))?;
    let d = c.str_new(b"default");
    hash_set(c, h, sym_value("implementation"), d)?;
    Ok(h)
}
fn gc_set_stress(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    crate::gc::set_stress(if a[0].truthy() { 1 } else { 0 });
    STRESS_V.set(c, if a[0] == NIL { UNDEF } else { a[0] });   // UNDEF: set to nil (NIL: never set)
    Ok(a[0])
}
/// GC.compact's answer: nothing moves in saruby's heap.
fn compact_info(c: &mut Ctx) -> RV {
    let h = c.hash_new();
    for k in ["considered", "moved", "moved_up", "moved_down"] { let e = c.hash_new(); hash_set(c, h, sym_value(k), e)?; }
    Ok(h)
}
// ---- GC::Profiler (gc.c): records kept by the runtime while enabled ----
fn prof_raw_data(c: &mut Ctx, _s: Value, _a: &[Value], _b: Blk) -> RV {
    let Some(rs) = crate::gc::profile_records() else { return Ok(NIL) };
    let slot = std::mem::size_of::<RObj>() as i64;
    let out = c.ary_new();
    for r in rs {
        let fl = c.hash_new();
        for (k, v) in [("major_by", sym_value("force")), ("gc_by", sym_value("newobj")), ("have_finalizer", FALSE), ("immediate_sweep", TRUE),
                       ("weak_references_count", Value::fix(0)), ("retained_weak_references_count", Value::fix(0))] { hash_set(c, fl, sym_value(k), v)?; }
        let h = c.hash_new();
        hash_set(c, h, sym_value("GC_FLAGS"), fl)?;
        let t = c.float_new(r.time.as_secs_f64());
        hash_set(c, h, sym_value("GC_TIME"), t)?;
        let at = c.float_new(crate::gc::since_start(r.at));
        hash_set(c, h, sym_value("GC_INVOKE_TIME"), at)?;
        for (k, v) in [("HEAP_USE_SIZE", (r.stats.slots - r.stats.free) as i64 * slot), ("HEAP_TOTAL_SIZE", r.stats.slots as i64 * slot), ("HEAP_TOTAL_OBJECTS", r.stats.slots as i64), ("MOVED_OBJECTS", 0)] {
            hash_set(c, h, sym_value(k), Value::fix(v))?;
        }
        hash_set(c, h, sym_value("GC_IS_MARKED"), TRUE)?;
        c.ary_push(out, h);
    }
    Ok(out)
}
/// gc_profile_dump_on
/// gc_profile_dump_on's pieces (each one rb_io_write to the report's output)
fn prof_result_pieces() -> Vec<String> {
    let Some(rs) = crate::gc::profile_records() else { return vec![] };
    if rs.is_empty() { return vec![]; }
    let slot = std::mem::size_of::<RObj>();
    let mut o = vec![format!("GC {} invokes.\n", crate::gc::collections()),
        "Index    Invoke Time(sec)       Use Size(byte)     Total Size(byte)         Total Object                    GC Time(ms)\n".to_string()];
    for (i, r) in rs.iter().enumerate() {
        o.push(format!("{:5} {:19.3} {:20} {:20} {:20} {:30.20}\n", i + 1, crate::gc::since_start(r.at), (r.stats.slots - r.stats.free) * slot, r.stats.slots * slot, r.stats.slots, r.time.as_secs_f64() * 1000.0));
    }
    o
}
fn prof_result_bytes() -> Vec<u8> { prof_result_pieces().concat().into_bytes() }
fn prof_report(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    check_argc(c, a, 0, 1)?;
    for p in prof_result_pieces() {
        match a.first() { Some(&out) => { let s = c.str_new(p.as_bytes()); funcall(c, out, intern(b"write"), &[s], std::ptr::null())?; } None => { stdout_write(c, p.as_bytes())?; } }
    }
    Ok(NIL)
}
fn prof_total_time(c: &mut Ctx, _s: Value, _a: &[Value], _b: Blk) -> RV {
    let t: f64 = crate::gc::profile_records().map_or(0.0, |rs| rs.iter().map(|r| r.time.as_secs_f64()).sum());
    Ok(c.float_new(t))
}
fn init_gc_extras(c: &mut Ctx, gc: Value) {
    define_singleton_cfunc(c, gc, "latest_gc_info", gc_latest_info, -1);
    define_singleton_cfunc(c, gc, "stat_heap", gc_stat_heap, -1);
    define_singleton_cfunc(c, gc, "config", gc_config, -1);
    define_singleton_cfunc(c, gc, "auto_compact", |_c, _s, _a, _b| Ok(Value::bool(AUTO_COMPACT.load(Relaxed))), 0);
    define_singleton_cfunc(c, gc, "auto_compact=", |_c, _s, a, _b| { AUTO_COMPACT.store(a[0].truthy(), Relaxed); Ok(a[0]) }, 1);
    define_singleton_cfunc(c, gc, "measure_total_time", |_c, _s, _a, _b| Ok(Value::bool(MEASURE_TIME.load(Relaxed))), 0);
    define_singleton_cfunc(c, gc, "measure_total_time=", |_c, _s, a, _b| { MEASURE_TIME.store(a[0].truthy(), Relaxed); Ok(a[0]) }, 1);
    define_singleton_cfunc(c, gc, "stress", |c, _s, _a, _b| { let v = STRESS_V.get(c); Ok(if v == NIL { FALSE } else if v == UNDEF { NIL } else { v }) }, 0);
    define_singleton_cfunc(c, gc, "stress=", gc_set_stress, 1);
    #[cfg(feature = "gc_check")]
    define_singleton_cfunc(c, gc, "__check_rooting", crate::gc::check_rooting, -1);
    define_singleton_cfunc(c, gc, "compact", |c, _s, _a, _b| { explicit_gc(c, true); compact_info(c) }, 0);
    define_singleton_cfunc(c, gc, "latest_compact_info", |c, _s, _a, _b| compact_info(c), 0);
    define_singleton_cfunc(c, gc, "verify_compaction_references", |c, _s, _a, _b| { explicit_gc(c, true); compact_info(c) }, -1);
    // gc.c's constants, of saruby's one heap
    let ic = c.hash_new();
    let slot = std::mem::size_of::<RObj>() as i64;
    for (k, v) in [("DEBUG", FALSE), ("BASE_SLOT_SIZE", Value::fix(slot)), ("RBASIC_SIZE", Value::fix(16)), ("RVALUE_OVERHEAD", Value::fix(0)),
                   ("HEAP_PAGE_OBJ_LIMIT", Value::fix(page_slots() as i64)), ("HEAP_PAGE_BITMAP_SIZE", Value::fix(page_slots().div_ceil(64) as i64 * 8)),
                   ("HEAP_PAGE_SIZE", Value::fix(65536)), ("HEAP_COUNT", Value::fix(1)), ("RVARGC_MAX_ALLOCATE_SIZE", Value::fix(slot)), ("RVALUE_OLD_AGE", Value::fix(3))] {
        let _ = hash_set(c, ic, sym_value(k), v);
    }
    obj_set_flags(ic, crate::object::FL_FROZEN);
    const_set(c, gc, intern(b"INTERNAL_CONSTANTS"), ic);
    let opts = c.ary_new();
    obj_set_flags(opts, crate::object::FL_FROZEN);
    const_set(c, gc, intern(b"OPTS"), opts);
    // GC#garbage_collect (include GC)
    define_cfunc(c, gc, "garbage_collect", os_garbage_collect, -1);
    let pr = define_module_under(c, gc, "Profiler");
    let ds = |c: &mut Ctx, n: &str, f: CFunc, a: i32| define_singleton_cfunc(c, pr, n, f, a);
    ds(c, "enabled?", |_c, _s, _a, _b| Ok(Value::bool(crate::gc::profile_records().is_some())), 0);
    ds(c, "enable", |_c, _s, _a, _b| { crate::gc::set_profile(true); Ok(NIL) }, 0);
    ds(c, "disable", |_c, _s, _a, _b| { crate::gc::set_profile(false); Ok(NIL) }, 0);
    ds(c, "clear", |_c, _s, _a, _b| { crate::gc::profile_clear(); Ok(NIL) }, 0);
    ds(c, "raw_data", prof_raw_data, 0);
    ds(c, "result", |c, _s, _a, _b| { let b = prof_result_bytes(); Ok(c.str_new(&b)) }, 0);
    ds(c, "report", prof_report, -1);
    ds(c, "total_time", prof_total_time, 0);
}
fn os_each_object(c: &mut Ctx, s2: Value, a: &[Value], blk: Blk) -> RV {
    if blk.is_null() { return crate::builtins::enumerator::enum_for(c, s2, intern(b"each_object"), a, NIL); }
    let k = a.first().copied();
    // nothing allocates between the walk and the Array that roots the result
    let mut vs = vec![];
    crate::gc::each_object(|v| vs.push(v));
    vs.retain(|&v| {
        if matches!(c.body_of(v), Body::Env(..)) || obj_klass(v) == NIL || v == c.vm().m_internal.get() { return false; }
        if is_class_or_module(v) && crate::builtins::objspace::singleton_internal(c, v) { return false; }
        match k { Some(k) => is_class_or_module(obj_klass(v)) && is_kind_of(c, v, k), None => true }
    });
    let n = vs.len();
    let list = c.ary_from_vec(vs);
    for i in 0..n { let x = c.ary_get(list, i); yield1(c, blk, x)?; }
    Ok(Value::fix(n as i64))
}
// ObjectSpace.garbage_collect(full_mark: true, immediate_mark: true, immediate_sweep: true), as gc.rb
fn os_garbage_collect(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    let mut full = true;
    if let Some(h) = kw_arg(c, a) {
        let mut unknown = vec![];
        crate::hash::hash_each(c, h, |c, k, v| {
            if k == sym_value("full_mark") { full = v.truthy(); }
            else if k != sym_value("immediate_mark") && k != sym_value("immediate_sweep") { unknown.push(inspect_str(c, k)?); }
            Ok(())
        })?;
        if !unknown.is_empty() { return arg_err(c, format!("unknown keyword{}: {}", if unknown.len() > 1 { "s" } else { "" }, unknown.join(", "))); }
    }
    if full { crate::gc::collect(c) } else { crate::gc::collect_minor(c) }
    run_finalizer_queue(c, FinAt::Internal(595));
    Ok(NIL)
}

// Warning[category] flags (bit = index in WARN_CAT_NAMES); experimental is on by default, -w turns on deprecated.
static WARN_CATS: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0b0010);
const WARN_CAT_NAMES: [&str; 4] = ["deprecated", "experimental", "performance", "strict_unused_block"];
use std::sync::atomic::Ordering::Relaxed;
pub fn warning_enable_deprecated() { WARN_CATS.fetch_or(1, Relaxed); }
/// rb_warning_category_from_name: a Symbol naming a category.
pub fn warning_category(c: &mut Ctx, v: Value) -> Result<usize, Esc> {
    if !v.is_sym() { return type_err(c, format!("wrong argument type {} (expected Symbol)", class_name_for_err(c, v))); }
    let n = crate::symbol::sym_str(v.as_sym());
    match WARN_CAT_NAMES.iter().position(|&x| x == n) { Some(i) => Ok(i), None => arg_err(c, format!("unknown category: {}", n)) }
}
pub fn warning_category_enabled(i: usize) -> bool { WARN_CATS.load(Relaxed) & (1 << i) != 0 }
fn warning_warn(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    check_argc(c, a, 1, 2)?;
    if !is_string(a[0]) { return type_err(c, format!("wrong argument type {} (expected String)", class_name_for_err(c, a[0]))); }
    // rb_warning_s_warn: nothing for a category that is off
    if let Some(h) = kw_arg(c, a) {
        let cat = crate::hash::hash_get_opt(c, h, sym_value("category"))?.unwrap_or(NIL);
        if cat != NIL { let i = warning_category(c, cat)?; if !warning_category_enabled(i) { return Ok(NIL); } }
    }
    let b = c.str_bytes(a[0]).to_vec(); stderr_write(c, &b)?; Ok(NIL)
}
// Finalizers (gc/default.c): FIN_TABLE maps an object to [object_id, callable...] by identity with weak
// keys; the collector moves the array of an object that died to FIN_QUEUE (gc.rs sweep_weak) and sets the
// interrupt bit, so the queue runs at the next check point (thread::poll), or at once after GC.start.
static FIN_TABLE: Global = Global::new();
static FIN_QUEUE: Global = Global::new();
static FIN_ANY: AtomicBool = AtomicBool::new(false);       // some finalizer was ever defined
static FIN_PENDING: AtomicBool = AtomicBool::new(false);   // FIN_QUEUE is not empty
static FIN_RUNNING: AtomicBool = AtomicBool::new(false);   // CRuby's `finalizing`: the queue is not re-entered
fn fin_table(c: &mut Ctx) -> Value {
    let t = FIN_TABLE.get(c);
    if t != NIL { return t; }
    let h = c.alloc(NIL, Body::Hash(Box::new(RHash::with_capacity(0))));
    c.hash_mut(h).identity = true;
    obj_set_flags(h, FL_WEAK_KEYS);
    FIN_TABLE.set(c, h);
    let q = c.ary_new();
    FIN_QUEUE.set(c, q);
    FIN_ANY.store(true, Relaxed);
    h
}
/// The finalizer table (gc.rs tells it from the other weak tables), nil before the first define.
pub fn fin_table_value(c: &Ctx) -> Value { FIN_TABLE.get(c) }
/// The collector found these tables' objects dead: queue them (no allocation, no Ruby code).
pub fn fin_enqueue(c: &mut Ctx, dead: Vec<Value>) {
    let q = FIN_QUEUE.get(c);
    c.ary_mut(q).extend(dead);
    FIN_PENDING.store(true, Relaxed);
    crate::vm::set_ints();
}
fn os_define_finalizer(c: &mut Ctx, _s: Value, a: &[Value], blk: Blk) -> RV {
    check_argc(c, a, 1, 2)?;
    let o = a[0];
    if !o.is_heap() || is_float(o) || is_bignum(o) { return arg_err(c, format!("cannot define finalizer for {}", class_path(c, real_class_of(c, o)))); }
    if is_frozen(o) { return Err(raise_frozen(c, o)); }
    let p = if a.len() > 1 { a[1] } else if !blk.is_null() { block_to_proc(c, blk)? } else { return arg_err(c, "tried to create Proc object without a block"); };
    if !respond_to(c, p, intern(b"call"), true) { let d = inspect_str(c, p)?; return arg_err(c, format!("wrong type argument {} (should be callable)", d)); }
    // the callable keeps the object alive: CRuby warns (it then runs only at exit)
    let recv = if is_proc(p) { c.proc_data(p).blk.self_.get() } else if is_method_obj(p) { c.method_obj(p).recv } else { NIL };
    if recv == o { gvar_warn(c, "finalizer references object to be finalized", false)?; }
    let t = fin_table(c);
    let p = match hash_get_opt(c, t, o)? {
        Some(fs) => {
            // an equal callable is defined once
            let mut found = None;
            let mut i = 1;
            while i < c.ary_len(fs) {
                let q = c.ary_get(fs, i);
                if funcall(c, q, intern(b"=="), &[p], std::ptr::null())?.truthy() { found = Some(q); break; }
                i += 1;
            }
            match found { Some(q) => q, None => { c.ary_push(fs, p); p } }
        }
        None => { let id = Value::fix(object_id(o) as i64); let fs = c.ary_new_from(&[id, p]); hash_set(c, t, o, fs)?; p }
    };
    let r = c.ary_new_from(&[Value::fix(0), p]);
    obj_set_flags(r, FL_FROZEN);
    Ok(r)
}
fn os_undefine_finalizer(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    if a[0].is_heap() && is_frozen(a[0]) { return Err(raise_frozen(c, a[0])); }
    let t = FIN_TABLE.get(c);
    if t != NIL { hash_delete(c, t, a[0])?; }
    Ok(a[0])
}
/// dup / clone: the copy gets the finalizers too, called with its own id (rb_gc_copy_finalizer).
pub fn copy_finalizer(c: &mut Ctx, from: Value, to: Value) -> Result<(), Esc> {
    if !FIN_ANY.load(Relaxed) { return Ok(()); }
    let t = FIN_TABLE.get(c);
    let Some(fs) = hash_get_opt(c, t, from)? else { return Ok(()) };
    let v = c.ary(fs).to_vec();
    let n = c.ary_from_vec(v);
    c.ary_mut(n)[0] = Value::fix(object_id(to) as i64);
    hash_set(c, t, to, n)
}
/// Where a finalizer's exception is reported: the current frame, CRuby's gc.rb line, or the script (exit).
#[derive(Clone, Copy)]
pub enum FinAt { Here, Internal(u32), Exit }
/// rb_gc_run_obj_finalizer: each callable of `fs` with the id; an exception is reported and the next one runs.
fn run_one(c: &mut Ctx, fs: Value, at: FinAt) {
    let id = c.ary_get(fs, 0);
    let saved = c.errinfo;
    let mut i = 1;
    while i < c.ary_len(fs) {
        let p = c.ary_get(fs, i);
        i += 1;
        if funcall(c, p, intern(b"call"), &[id], std::ptr::null()).is_ok() { continue; }
        let e = c.esc_val;
        c.esc = EscKind::None;
        if c.vm().verbose.get() != NIL {
            let pi = inspect_str(c, p).unwrap_or_default();
            c.esc = EscKind::None;
            let loc = match at {
                FinAt::Here => c.cur_frame().map(|f| format!("{}:{}", frame_file(f), f.line())).unwrap_or_default(),
                FinAt::Internal(l) => format!("<internal:gc>:{}", l),
                FinAt::Exit => { let z = c.vm().dollar_zero.get(); if is_string(z) { String::from_utf8_lossy(c.str_bytes(z)).into_owned() } else { String::new() } }
            };
            let msg = format!("{}: warning: Exception in finalizer {}\n", loc, pi);
            if warning_write(c, msg.as_bytes()).is_err() { c.esc = EscKind::None; }
            crate::report_exception(c, e);
        }
        c.esc = EscKind::None;
    }
    c.errinfo = saved;
}
/// Run the finalizers of objects collected so far (gc_finalize_deferred); a no-op while they run.
pub fn run_finalizer_queue(c: &mut Ctx, at: FinAt) {
    if !FIN_PENDING.load(Relaxed) || FIN_RUNNING.swap(true, Relaxed) { return; }
    drain_queue(c, at);
    FIN_RUNNING.store(false, Relaxed);
}
fn drain_queue(c: &mut Ctx, at: FinAt) {
    loop {
        let q = FIN_QUEUE.get(c);
        if c.ary_len(q) == 0 { break; }
        let fs = c.ary_mut(q).remove(0);
        run_one(c, fs, at);
    }
    FIN_PENDING.store(false, Relaxed);
}
/// At exit: the finalizers of every object still registered (in the order defined, including ones
/// defined meanwhile), then those of objects collected but not finalized yet.
pub fn run_finalizers(c: &mut Ctx) {
    if !FIN_ANY.load(Relaxed) { return; }
    FIN_RUNNING.store(true, Relaxed);
    let t = FIN_TABLE.get(c);
    loop {
        let first = (0..).map_while(|i| c.hash_entry_at(t, i)).find(|e| e.0 != UNDEF);
        let Some((o, fs)) = first else { break };
        let _ = hash_delete(c, t, o);
        run_one(c, fs, FinAt::Exit);
    }
    drain_queue(c, FinAt::Exit);
    FIN_RUNNING.store(false, Relaxed);
}
fn rbconfig_ruby(c: &mut Ctx, _s: Value, _a: &[Value], _b: Blk) -> RV {
    let exe = std::env::current_exe().map(|p| p.to_string_lossy().into_owned()).unwrap_or_else(|_| "saruby".into());
    Ok(c.str_new(exe.as_bytes()))
}
/// false when there is no such file or it rejects this version (then the built-in table is used).
fn rbconfig_from_ref(c: &mut Ctx, archdir: &str, bindir: &str, name: &str) -> bool {
    let path = format!("{}/rbconfig.rb", archdir);
    let Ok(src) = std::fs::read(&path) else { return false };
    let file: &'static str = Box::leak(path.into_boxed_str());
    if crate::builtins::kernel::run_source(c, &src, file, 1).is_err() { return false; }
    let Some(rbc) = const_get_at(c, c.vm().c_object.get(), intern(b"RbConfig")) else { return false };
    // saruby's binary; the library paths stay the reference ruby's (its libruby.so)
    let Some(h) = const_get_at(c, rbc, intern(b"CONFIG")).filter(|&h| is_hash(h)) else { return false };
    for (k, v) in [("ruby_install_name", name), ("RUBY_INSTALL_NAME", name), ("bindir", bindir)] {
        let (ks, vs) = (c.str_new_frozen(k.as_bytes()), c.str_new(v.as_bytes()));
        if hash_set(c, h, ks, vs).is_err() { return false; }
    }
    // bindir in MAKEFILE_CONFIG too, so that expanding it gives CONFIG's
    let Some(m) = const_get_at(c, rbc, intern(b"MAKEFILE_CONFIG")).filter(|&m| is_hash(m)) else { return false };
    let (ks, vs) = (c.str_new_frozen(b"bindir"), c.str_new(bindir.as_bytes()));
    hash_set(c, m, ks, vs).is_ok()
}
pub fn rbconfig_init(c: &mut Ctx) {
    if const_get_at(c, c.vm().c_object.get(), intern(b"RbConfig")).is_some() { return; }
    let rbc = define_module(c, "RbConfig");
    let cfg = c.hash_new_capa(32);
    let exe = std::env::current_exe().unwrap_or_else(|_| "saruby".into());
    let bindir = exe.parent().map(|p| p.to_string_lossy().into_owned()).unwrap_or_else(|| ".".into());
    let name = exe.file_name().map(|p| p.to_string_lossy().into_owned()).unwrap_or_else(|| "saruby".into());
    let (lib, _gem) = crate::builtins::prelude::stdlib_dirs().unwrap_or_default();
    let arch = if cfg!(target_arch = "aarch64") { "aarch64" } else { "x86_64" };
    let archname = format!("{}-linux", arch);
    let archdir = format!("{}/{}", lib, archname);
    // the reference ruby's own rbconfig.rb (every key, expand, fire_update!), saruby's binary in place of ruby's
    if !lib.is_empty() && rbconfig_from_ref(c, &archdir, &bindir, &name) { return; }
    let libprefix = std::path::Path::new(&lib).parent().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default();
    // the Unicode version of the case / property tables (Rust's char)
    let (u0, u1, u2) = char::UNICODE_VERSION;
    let (uv, uev) = (format!("{}.{}.{}", u0, u1, u2), format!("{}.{}", u0, u1));
    let mut ver = crate::RUBY_VERSION.split('.');
    let (major, minor, teeny) = (ver.next().unwrap_or("0"), ver.next().unwrap_or("0"), ver.next().unwrap_or("0"));
    let (abi, api) = (format!("{}.{}.0", major, minor), format!("{}.{}", major, minor));
    let entries: &[(&str, &str)] = &[("ruby_install_name", &name), ("RUBY_INSTALL_NAME", &name), ("RUBY_BASE_NAME", "ruby"), ("bindir", &bindir), ("EXEEXT", ""),
        ("host_os", "linux"), ("target_os", "linux"), ("host_cpu", arch), ("target_cpu", arch), ("arch", &archname), ("host", &archname),
        ("MAJOR", major), ("MINOR", minor), ("TEENY", teeny), ("PATCHLEVEL", "0"), ("ruby_version", &abi), ("RUBY_PROGRAM_VERSION", crate::RUBY_VERSION),
        ("RUBY_API_VERSION", &api), ("rubylibdir", &lib), ("rubylibprefix", &libprefix), ("archdir", &archdir), ("rubyarchdir", &archdir), ("DLEXT", "so"), ("SOEXT", "so"),
        ("LIBEXT", "a"), ("OBJEXT", "o"), ("PATH_SEPARATOR", ":"), ("SHELL", "/bin/sh"), ("CC", "gcc"), ("CPP", "gcc -E"), ("ENABLE_SHARED", "yes"),
        ("build_os", "linux"), ("build_cpu", arch), ("build", &archname), ("LIBRUBY_SO", "libsaruby.so"), ("LIBRUBY", "libsaruby.so"), ("libdirname", "libdir"), ("LIBPATHENV", "LD_LIBRARY_PATH"), ("AR", "ar"), ("STRIP", "strip"), ("UNICODE_VERSION", &uv), ("UNICODE_EMOJI_VERSION", &uev), ("prefix", &bindir), ("exec_prefix", &bindir),
        ("libdir", &bindir), ("sitedir", ""), ("sitelibdir", ""), ("sitearchdir", ""), ("vendordir", ""), ("vendorlibdir", ""), ("vendorarchdir", ""),
        ("datadir", ""), ("mandir", ""), ("sysconfdir", "/etc"), ("localstatedir", "/var"), ("configure_args", ""), ("CFLAGS", ""), ("LDFLAGS", "")];
    for (k, v) in entries { let ks = c.str_new_frozen(k.as_bytes()); let vs = c.str_new(v.as_bytes()); let _ = hash_set(c, cfg, ks, vs); }
    const_set(c, rbc, intern(b"CONFIG"), cfg);
    const_set(c, rbc, intern(b"MAKEFILE_CONFIG"), cfg);
    let top = c.str_new(bindir.as_bytes());
    const_set(c, rbc, intern(b"TOPDIR"), top);
    define_singleton_cfunc(c, rbc, "ruby", rbconfig_ruby, 0);
}

pub fn dup_data(c: &mut Ctx, v: Value) -> Result<Option<Value>, Esc> {
    Ok(if c.data::<REnc>(v).is_some() { Some(v) } else { None })
}
pub fn alloc_data(_c: &mut Ctx, _klass: Value) -> Result<Option<Value>, Esc> { Ok(None) }

/// A frozen constant String.
fn boot_str(c: &mut Ctx, b: &[u8]) -> Value {
    let k = c.vm().c_string.get();
    let s = c.alloc(k, Body::String(RString::new(crate::sbuf::SBuf::from_slice(b), ENC_UTF8)));
    obj_set_flags(s, FL_FROZEN);
    s
}

pub fn init(c: &mut Ctx) {
    let vmr = c.vm();
    let ob = vmr.c_object.get();
    // GC / ObjectSpace
    let gc = define_module(c, "GC");
    vmr.m_gc.set(gc);
    define_singleton_cfunc(c, gc, "start", gc_start, -1);
    define_singleton_cfunc(c, vmr.m_internal.get(), "__gc_start", gc_start_full, 1);
    define_singleton_cfunc(c, gc, "count", gc_count, 0);
    define_singleton_cfunc(c, gc, "disable", gc_disable, 0);
    define_singleton_cfunc(c, gc, "enable", gc_enable, 0);
    define_singleton_cfunc(c, gc, "stat", gc_stat, -1);
    init_gc_extras(c, gc);
    let os = define_module(c, "ObjectSpace");
    vmr.m_object_space.set(os);
    define_module_function(c, os, "each_object", os_each_object, -1);
    define_module_function(c, os, "count_objects", crate::builtins::objspace::count_objects, -1);
    define_module_function(c, os, "garbage_collect", os_garbage_collect, -1);
    define_module_function(c, os, "define_finalizer", os_define_finalizer, -1);
    define_module_function(c, os, "undefine_finalizer", os_undefine_finalizer, 1);
    crate::builtins::objspace::init_core(c);
    let wn = define_module(c, "Warning");
    vmr.m_warning.set(wn);
    // rb_define_method + rb_extend_object: a Warning#warn defined later is what Warning.warn calls
    define_cfunc(c, wn, "warn", warning_warn, -1);
    let wsc = singleton_class(c, wn);
    include_module(c, wsc, wn);
    define_singleton_cfunc(c, wn, "[]", |c, _s, a, _b| { let i = warning_category(c, a[0])?; Ok(Value::bool(WARN_CATS.load(Relaxed) & (1 << i) != 0)) }, 1);
    define_singleton_cfunc(c, wn, "[]=", |c, _s, a, _b| {
        let i = warning_category(c, a[0])?;
        if a[1].truthy() { WARN_CATS.fetch_or(1 << i, Relaxed); } else { WARN_CATS.fetch_and(!(1 << i), Relaxed); }
        Ok(a[1])
    }, 2);
    define_singleton_cfunc(c, wn, "categories", |c, _s, _a, _b| { let v: Vec<Value> = WARN_CAT_NAMES.iter().map(|n| sym_value(n)).collect(); Ok(c.ary_new_from(&v)) }, 0);
    // constants
    let set = |c: &mut Ctx, n: &str, v: Value| const_set(c, ob, intern(n.as_bytes()), v);
    // Ruby::VERSION and co. are the same objects (4.0)
    let rb = define_module(c, "Ruby");
    let desc = crate::version();
    for (n, v) in [("VERSION", crate::RUBY_VERSION), ("RELEASE_DATE", crate::RUBY_RELEASE_DATE), ("PLATFORM", crate::RUBY_PLATFORM), ("ENGINE", "saruby"),
                   ("ENGINE_VERSION", crate::ENGINE_VERSION), ("COPYRIGHT", "saruby - Copyright (C) 2026 Koichi Sasada"),
                   ("REVISION", crate::RUBY_REVISION), ("DESCRIPTION", &desc)] {
        let s = boot_str(c, v.as_bytes());
        set(c, &format!("RUBY_{}", n), s);
        const_set(c, rb, intern(n.as_bytes()), s);
    }
    set(c, "RUBY_PATCHLEVEL", Value::fix(0));
    const_set(c, rb, intern(b"PATCHLEVEL"), Value::fix(0));
    set(c, "ARGV", vmr.argv.get());
    crate::builtins::lazy_const(c, ob, "TOPLEVEL_BINDING", "saruby/toplevel_binding");
    let _ = (intern, num_to_long);
    vmr.c_location.set(crate::builtins::exception::init_location(c));
}

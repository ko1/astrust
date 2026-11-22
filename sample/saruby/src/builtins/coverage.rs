// coverage.so (ext/coverage/coverage.c): lines, oneshot_lines, methods and eval coverage.
// Files compiled while coverage is set up get node_cov counters (compile.rs); nothing else pays.
#![forbid(unsafe_code)]
use crate::array::*;
#[allow(unused_imports)]
use crate::heap::no_legacy::*;   // the old accessors cannot be named here
use crate::symbol::intern;
use crate::value::*;
use crate::vm::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};

const T_LINES: u32 = 1;
const T_BRANCHES: u32 = 2;
const T_METHODS: u32 = 4;
const T_ONESHOT: u32 = 8;
const T_EVAL: u32 = 16;

static SETUP: AtomicBool = AtomicBool::new(false);
static RUNNING: AtomicBool = AtomicBool::new(false);

struct FileCov { nlines: usize, oneshot: Vec<u32> }
#[derive(Clone, Copy)]
enum Slot { Line(usize, u32), Method(usize, [u32; 4]), Branch }
#[derive(Default)]
struct Cov {
    mode: u32,          // current_mode (0: Coverage.start without arguments)
    state: u8,          // 0 idle, 1 suspended, 2 running
    files: Vec<FileCov>,  // never shrinks: compiled code keeps its file ids
    order: Vec<String>, // paths in first-registration order
    by_name: HashMap<String, usize>,
    slots: Vec<Slot>,
    counts: Vec<i64>,
    iseq_slot: HashMap<usize, u32>,
    defs: Vec<(u32, Value, Sym)>,
    bases: Vec<(usize, &'static str, [u32; 4], Vec<(&'static str, [u32; 4], u32)>)>,   // branch bases in declaration order
}
thread_local! { static COV: RefCell<Cov> = RefCell::new(Cov::default()); }

/// node_cov: one more execution of counter `id`.
pub fn hit(id: u32) {
    if !RUNNING.load(Relaxed) { return; }
    COV.with(|cv| {
        let mut g = cv.borrow_mut();
        let i = id as usize;
        if i >= g.counts.len() { return; }
        g.counts[i] += 1;
        if g.mode & T_ONESHOT != 0 && g.counts[i] == 1 {
            if let Slot::Line(f, l) = g.slots[i] { g.files[f].oneshot.push(l); }
        }
    })
}

/// Compiling `file` (from `line`): Some(file id) when its lines are to be counted.
pub fn compile_wants(file: &str, eval: bool, src: &[u8], line: i32) -> Option<usize> {
    if !SETUP.load(Relaxed) || file.starts_with("<internal:") { return None; }
    COV.with(|cv| {
        let mut g = cv.borrow_mut();
        if eval {
            if g.mode & T_EVAL == 0 { return None; }
            if let Some(&f) = g.by_name.get(file) { return Some(f); }
        }
        // the parser's line count: a last line without a newline is not counted
        let nlines = src.iter().filter(|&&b| b == b'\n').count() + if eval { (line.max(1) - 1) as usize } else { 0 };
        let id = g.files.len();
        g.files.push(FileCov { nlines, oneshot: vec![] });
        if g.by_name.insert(file.to_string(), id).is_none() { g.order.push(file.to_string()); }
        Some(id)
    })
}
fn new_slot(g: &mut Cov, s: Slot) -> u32 { g.slots.push(s); g.counts.push(0); (g.slots.len() - 1) as u32 }
/// The counter of one line event (oneshot_lines reports each event's first run).
pub fn line_slot(file: usize, line: u32) -> u32 {
    COV.with(|cv| new_slot(&mut cv.borrow_mut(), Slot::Line(file, line)))
}
/// A method's call counter, when methods are measured.
pub fn method_slot(file: usize, pos: [u32; 4]) -> Option<u32> {
    COV.with(|cv| {
        let mut g = cv.borrow_mut();
        if g.mode & T_METHODS == 0 { return None; }
        Some(new_slot(&mut g, Slot::Method(file, pos)))
    })
}
/// decl_branch_base: Some(base) when branches are measured.
pub fn branch_base(file: usize, ty: &'static str, pos: [u32; 4]) -> Option<usize> {
    COV.with(|cv| {
        let mut g = cv.borrow_mut();
        if g.mode & T_BRANCHES == 0 { return None; }
        g.bases.push((file, ty, pos, vec![]));
        Some(g.bases.len() - 1)
    })
}
/// add_trace_branch_coverage: the counter of one target of `base`.
pub fn branch_target(base: usize, label: &'static str, pos: [u32; 4]) -> u32 {
    COV.with(|cv| {
        let mut g = cv.borrow_mut();
        let s = new_slot(&mut g, Slot::Branch);
        g.bases[base].3.push((label, pos, s));
        s
    })
}
pub fn bind_iseq(is: usize, slot: u32) { COV.with(|cv| { cv.borrow_mut().iseq_slot.insert(is, slot); }) }
/// A def ran: the method entry (klass, name) counts with its body's counter.
pub fn on_def(c: &mut Ctx, is: usize, klass: Value, name: Sym) {
    if !SETUP.load(Relaxed) { return; }
    let found = COV.with(|cv| {
        let mut g = cv.borrow_mut();
        let Some(&s) = g.iseq_slot.get(&is) else { return false };
        g.defs.retain(|d| !(d.1 == klass && d.2 == name));
        g.defs.push((s, klass, name));
        true
    });
    if found { root(c, klass); }
}
/// Keep the classes named by method results alive (hidden array on Coverage).
fn root(c: &mut Ctx, v: Value) {
    let m = const_get_at(c.vm().c_object.get(), intern(b"Coverage")).unwrap_or(NIL);
    if m == NIL { return; }
    let mut a = ivar_get(c, m, intern(b"__classes"));
    if a == NIL { a = c.ary_new(); ivar_set_raw(c, m, intern(b"__classes"), a); }
    c.ary_push(a, v);
}

/// rb_convert_type(v, T_HASH, "Hash", "to_hash")
fn to_hash(c: &mut Ctx, v: Value) -> RV {
    match crate::hash::check_hash(c, v)? {
        Some(h) => Ok(h),
        None => type_err(c, format!("no implicit conversion of {} into Hash", crate::builtins::kernel::class_name_for_err(v))),
    }
}
fn rt_err<T>(c: &mut Ctx, m: &str) -> Result<T, Esc> { err(c, c.vm().c_runtime_error.get(), m) }
fn state(_: &mut Ctx) -> u8 { COV.with(|cv| cv.borrow().state) }
fn set_state(s: u8) {
    COV.with(|cv| cv.borrow_mut().state = s);
    SETUP.store(s != 0, Relaxed);
    RUNNING.store(s == 2, Relaxed);
}
fn cv_supported(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    if !a[0].is_sym() { return type_err(c, format!("wrong argument type {} (expected Symbol)", crate::builtins::kernel::class_name_for_err(a[0]))); }
    Ok(Value::bool(matches!(crate::symbol::sym_bytes(a[0].as_sym()), b"lines" | b"oneshot_lines" | b"branches" | b"methods" | b"eval")))
}
fn cv_setup(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    if state(c) != 0 { return rt_err(c, "coverage measurement is already setup"); }
    check_argc(c, a, 0, 1)?;
    let mode = if a.is_empty() { 0 } else if a[0] == Value::sym(intern(b"all")) { T_LINES | T_BRANCHES | T_METHODS | T_EVAL } else {
        let h = to_hash(c, a[0])?;
        let on = |c: &mut Ctx, k: &str| -> Result<bool, Esc> { Ok(crate::hash::hash_get_opt(c, h, sym_value(k))?.is_some_and(|v| v.truthy())) };
        let mut m = 0;
        if on(c, "lines")? { m |= T_LINES; }
        if on(c, "branches")? { m |= T_BRANCHES; }
        if on(c, "methods")? { m |= T_METHODS; }
        if on(c, "oneshot_lines")? {
            if m & T_LINES != 0 { return rt_err(c, "cannot enable lines and oneshot_lines simultaneously"); }
            m |= T_LINES | T_ONESHOT;
        }
        if on(c, "eval")? { m |= T_EVAL; }
        m
    };
    COV.with(|cv| {
        let mut g = cv.borrow_mut();
        g.mode = mode;
        g.order.clear(); g.by_name.clear(); g.defs.clear(); g.iseq_slot.clear(); g.bases.retain(|_| false);
    });
    set_state(1);
    Ok(NIL)
}
fn cv_resume(c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV {
    match state(c) { 0 => return rt_err(c, "coverage measurement is not set up yet"), 2 => return rt_err(c, "coverage measurement is already running"), _ => {} }
    set_state(2);
    Ok(NIL)
}
fn cv_start(c: &mut Ctx, s: Value, a: &[Value], b: *const Block) -> RV { cv_setup(c, s, a, b)?; cv_resume(c, s, &[], b) }
fn cv_suspend(c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV {
    if state(c) != 2 { return rt_err(c, "coverage measurement is not running"); }
    set_state(1);
    Ok(NIL)
}
/// rb_coverage_peek_result
fn peek(c: &mut Ctx) -> RV {
    if state(c) == 0 { return rt_err(c, "coverage measurement is not enabled"); }
    type Branches = Vec<(&'static str, [u32; 4], Vec<(&'static str, [u32; 4], i64)>)>;
    struct Row { path: String, lines: Vec<Option<i64>>, oneshot: Vec<u32>, methods: Vec<(Value, Sym, [u32; 4], i64)>, branches: Branches }
    let (mode, rows) = COV.with(|cv| {
        let g = cv.borrow();
        let mut rows = vec![];
        for p in &g.order {
            let Some(&f) = g.by_name.get(p) else { continue };
            let mut lines = vec![None; g.files[f].nlines];
            let mut methods: Vec<(Value, Sym, [u32; 4], i64)> = vec![];
            for (i, s) in g.slots.iter().enumerate() {
                if let Slot::Line(ff, l) = *s { if ff == f && (l as usize) >= 1 && (l as usize) <= lines.len() { let e = &mut lines[l as usize - 1]; *e = Some(e.unwrap_or(0) + g.counts[i]); } }
            }
            for &(s, k, n) in &g.defs {
                if let Slot::Method(ff, pos) = g.slots[s as usize] { if ff == f { methods.push((k, n, pos, g.counts[s as usize])); } }
            }
            let branches: Branches = g.bases.iter().filter(|b| b.0 == f)
                .map(|b| (b.1, b.2, b.3.iter().map(|t| (t.0, t.1, g.counts[t.2 as usize])).collect())).collect();
            rows.push(Row { path: p.clone(), lines, oneshot: g.files[f].oneshot.clone(), methods, branches });
        }
        (g.mode, rows)
    });
    let out = c.hash_new();
    for r in rows {
        let lines = if mode & T_ONESHOT != 0 {
            let v: Vec<Value> = r.oneshot.iter().map(|&l| Value::fix(l as i64)).collect(); c.ary_from_vec(v)
        } else {
            let v: Vec<Value> = r.lines.iter().map(|x| x.map_or(NIL, Value::fix)).collect(); c.ary_from_vec(v)
        };
        obj_set_flags(lines, FL_FROZEN);
        let val = if mode == 0 { lines } else {
            let h = c.hash_new();
            if mode & T_LINES != 0 { let k = if mode & T_ONESHOT != 0 { "oneshot_lines" } else { "lines" }; crate::hash::hash_aset(c, h, sym_value(k), lines)?; }
            if mode & T_BRANCHES != 0 {
                // branch_coverage: ids numbered over bases and their targets in order
                let b = c.hash_new();
                let mut id = 0;
                let key = |c: &mut Ctx, ty: &str, id: i64, p: [u32; 4]| c.ary_from_vec(vec![sym_value(ty), Value::fix(id), Value::fix(p[0] as i64), Value::fix(p[1] as i64), Value::fix(p[2] as i64), Value::fix(p[3] as i64)]);
                for (ty, p, ts) in r.branches {
                    let k = key(c, ty, id, p); id += 1;
                    let ch = c.hash_new();
                    for (l, tp, cnt) in ts { let tk = key(c, l, id, tp); id += 1; crate::hash::hash_aset(c, ch, tk, Value::fix(cnt))?; }
                    crate::hash::hash_aset(c, b, k, ch)?;
                }
                crate::hash::hash_aset(c, h, sym_value("branches"), b)?;
            }
            if mode & T_METHODS != 0 {
                let m = c.hash_new();
                for (k, n, p, cnt) in r.methods {
                    let key = c.ary_from_vec(vec![k, Value::sym(n), Value::fix(p[0] as i64), Value::fix(p[1] as i64), Value::fix(p[2] as i64), Value::fix(p[3] as i64)]);
                    crate::hash::hash_aset(c, m, key, Value::fix(cnt))?;
                }
                crate::hash::hash_aset(c, h, sym_value("methods"), m)?;
            }
            h
        };
        let ks = c.str_new(r.path.as_bytes());
        crate::hash::hash_aset(c, out, ks, val)?;
    }
    obj_set_flags(out, FL_FROZEN);
    Ok(out)
}
fn cv_peek_result(c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV { peek(c) }
fn cv_result(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    if state(c) == 0 { return rt_err(c, "coverage measurement is not enabled"); }
    check_argc(c, a, 0, 1)?;
    let (mut stop, mut clear) = (true, true);
    if let Some(&o) = a.first() {
        let h = to_hash(c, o)?;
        stop = crate::hash::hash_get_opt(c, h, sym_value("stop"))?.is_some_and(|v| v.truthy());
        clear = crate::hash::hash_get_opt(c, h, sym_value("clear"))?.is_some_and(|v| v.truthy());
    }
    let r = peek(c)?;
    if stop && !clear {
        let loc = backtrace_lines(c, 0).first().map(|l| l.split(":in ").next().unwrap_or("").to_string()).unwrap_or_default();
        crate::builtins::kernel::warning_write(c, format!("{}: warning: stop implies clear\n", loc).as_bytes())?;
        clear = true;
    }
    if clear { COV.with(|cv| { let mut g = cv.borrow_mut(); for x in g.counts.iter_mut() { *x = 0; } for f in g.files.iter_mut() { f.oneshot.clear(); } }); }
    if stop {
        COV.with(|cv| { let mut g = cv.borrow_mut(); g.order.clear(); g.by_name.clear(); g.defs.clear(); g.iseq_slot.clear(); g.bases.retain(|_| false); });
        set_state(0);
    }
    Ok(r)
}
fn cv_state(c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV {
    Ok(sym_value(match state(c) { 0 => "idle", 1 => "suspended", _ => "running" }))
}
fn cv_running(c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::bool(state(c) == 2)) }

/// Init_coverage
pub fn init(c: &mut Ctx) {
    let ob = c.vm().c_object.get();
    let m = const_get_at(ob, intern(b"Coverage")).unwrap_or_else(|| define_module("Coverage"));
    define_singleton_cfunc(m, "supported?", cv_supported, 1);
    let f = |n: &str, fun: CFunc, ar: i32| define_module_function(m, n, fun, ar);
    f("setup", cv_setup, -1); f("start", cv_start, -1); f("resume", cv_resume, 0); f("suspend", cv_suspend, 0);
    f("result", cv_result, -1); f("peek_result", cv_peek_result, 0); f("state", cv_state, 0); f("running?", cv_running, 0);
}

// Ractor (CRuby 4.0's ractor.rb / ractor.c / ractor_sync.c) on green threads: every Ractor's main
// thread is a green thread of thread.rs, so nothing runs in parallel, but what CRuby makes observable
// is kept: messages are deep-copied or moved, ports belong to their Ractor, and non-main Ractors may
// not reach non-shareable constants, globals, class ivars or class variables.  Those checks sit on
// slow paths and run only while another Ractor is alive (`LIVE`); docs/ractor.md.
#![forbid(unsafe_code)]
#[allow(unused_imports)]
use crate::heap::no_legacy::*;   // the old accessors cannot be named here
#[allow(unused_imports)]
use crate::heap::no_legacy_hn::*;
use crate::array::*;
use crate::builtins::kernel::*;
use crate::builtins::thread;
use crate::fxhash::{HashMap, HashSet};
use crate::hash::*;
use crate::heap::{is_frozen, obj_clear_flags, obj_flags, obj_klass, obj_set_flags, obj_set_klass, Global};
use crate::object::{FL_CHILLED, FL_EXIVAR, FL_FROZEN, FL_KWHASH, FL_SHAREABLE, FL_TMPLOCK};
use crate::string::*;
use crate::symbol::{intern, s, sym_bytes, sym_str};
use crate::value::*;
use crate::vm::*;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering::Relaxed};

/// One port's queue (struct ractor_queue); messages are already copied / moved.
struct PortQ { id: u64, q: VecDeque<Value>, closed: bool }

/// struct rb_ractor_struct, the parts green threads need.
pub struct RRactor {
    id: u64, name: Value, loc: Value,
    thread: Value,          // its main thread (nil: the main Ractor)
    start: Value,           // [proc, args] until it starts
    ports: Vec<PortQ>,
    next_port: u64,
    default_port: Value,
    monitors: Vec<Value>,   // ports told :exited / :aborted when it ends
    done: bool, aborted: bool,
    value: Value,           // the block's value or exception
    successor: Value,       // the Ractor that took the value
    locals: Value,          // Ractor-local storage (Hash)
}
impl RData for RRactor {
    fn visit(&mut self, m: &mut crate::gc::Visitor) {
        for v in [&mut self.name, &mut self.loc, &mut self.thread, &mut self.start, &mut self.default_port, &mut self.value, &mut self.successor, &mut self.locals] { m.visit(v); }
        for p in &mut self.ports { for v in p.q.iter_mut() { m.visit(v); } }
        for v in &mut self.monitors { m.visit(v); }
    }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}
/// Ractor::Port: (owner, id), frozen and shareable.
pub struct RPort { owner: Value, id: u64 }
impl RData for RPort {
    fn visit(&mut self, m: &mut crate::gc::Visitor) { m.visit(&mut self.owner); }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

static C_RACTOR: Global = Global::new();
static C_PORT: Global = Global::new();
static C_MOVED: Global = Global::new();
static E_ERROR: Global = Global::new();
static E_REMOTE: Global = Global::new();
static E_CLOSED: Global = Global::new();
static E_ISOLATION: Global = Global::new();
static E_MOVED: Global = Global::new();
static MAIN_R: Global = Global::new();
/// Non-main Ractors alive: the isolation checks run only while it is not 0.
static LIVE: AtomicUsize = AtomicUsize::new(0);
static NEXT_ID: AtomicU64 = AtomicU64::new(2);
static STARTED: AtomicBool = AtomicBool::new(false);   // ruby_single_main_ractor is gone

fn rr(c: &Ctx, v: Value) -> &RRactor { c.data::<RRactor>(v).expect("a Ractor") }
fn rr_mut(c: &mut Ctx, v: Value) -> &mut RRactor { c.data_mut::<RRactor>(v).expect("a Ractor") }
fn is_ractor(c: &Ctx, v: Value) -> bool { v.is_heap() && c.data::<RRactor>(v).is_some() }
fn is_port(c: &Ctx, v: Value) -> bool { v.is_heap() && c.data::<RPort>(v).is_some() }
fn port(c: &Ctx, v: Value) -> (Value, u64) { let p = c.data::<RPort>(v).expect("a Ractor::Port"); (p.owner, p.id) }

/// Another Ractor is alive (the gate of every isolation check).
#[inline(always)]
pub fn multi() -> bool { LIVE.load(Relaxed) != 0 }
/// The running code is in a non-main Ractor.
#[inline(always)]
pub fn isolated(c: &Ctx) -> bool { multi() && thread::current_ractor(c) != NIL }
pub fn current(c: &Ctx) -> Value { let r = thread::current_ractor(c); if r == NIL { MAIN_R.get(c) } else { r } }
/// Thread.main in a non-main Ractor.
pub fn main_thread(c: &Ctx, r: Value) -> Value { rr(c, r).thread }
/// What a Ractor wait waits on: every send, close and Ractor end notifies it (waiters re-check).
fn evt(c: &Ctx) -> u64 { MAIN_R.get(c).raw() | 2 }

// ---- errors ---------------------------------------------------------------------------------

fn error<T>(c: &mut Ctx, g: &Global, msg: impl AsRef<str>) -> Result<T, Esc> { let k = g.get(c); err(c, k, msg) }
fn closed_err<T>(c: &mut Ctx) -> Result<T, Esc> { error(c, &E_CLOSED, "The port was already closed") }
pub fn isolation_error_class(c: &Ctx) -> Value { E_ISOLATION.get(c) }
fn isolation<T>(c: &mut Ctx, msg: impl AsRef<str>) -> Result<T, Esc> { error(c, &E_ISOLATION, msg) }

// ---- shareable ------------------------------------------------------------------------------

/// Built-in data CRuby lets be shareable once frozen (RUBY_TYPED_FROZEN_SHAREABLE, T_FILE, T_MATCH, ...).
fn frozen_shareable_data(c: &Ctx, v: Value) -> bool {
    let m = c.vm();
    [m.c_time.get(), m.c_regexp.get(), m.c_match_data.get(), m.c_rational.get(), m.c_complex.get(), m.c_encoding.get(), m.c_io.get()]
        .into_iter().any(|k| k != NIL && is_kind_of(c, v, k))
}
/// Kinds that are frozen whether or not the flag says so (immutable in CRuby).
fn always_frozen(c: &Ctx, v: Value) -> bool {
    match c.body_of(v) {
        Body::Float(_) | Body::Bignum(_) | Body::Range(..) => true,
        Body::Data(_) => { let m = c.vm(); [m.c_rational.get(), m.c_complex.get(), m.c_encoding.get()].into_iter().any(|k| k != NIL && is_kind_of(c, v, k)) }
        _ => false,
    }
}
fn frozen(c: &Ctx, v: Value) -> bool { is_frozen(v) || always_frozen(c, v) }
/// Internal objects (hidden, backtraces) are passed as they are, as CRuby's imemos.
fn internal(c: &Ctx, v: Value) -> bool { obj_klass(v) == NIL || c.data::<crate::vm::RawBt>(v).is_some() }
/// allow_frozen_shareable_p
fn frozen_shareable_kind(c: &Ctx, v: Value) -> bool {
    match c.body_of(v) {
        Body::Proc(_) | Body::Method(_) | Body::Env(..) => false,
        Body::Data(_) => frozen_shareable_data(c, v),
        _ => !data_in_cruby(c, v),
    }
}
/// Plain objects here that are T_DATA without RUBY_TYPED_FROZEN_SHAREABLE in CRuby.
fn data_in_cruby(c: &Ctx, v: Value) -> bool {
    let m = c.vm();
    let (th, os) = (m.c_thread.get(), m.m_object_space.get());
    let lazy = |owner: Value, n: &[u8]| if owner == NIL { NIL } else { const_get_at(c, owner, intern(n)).unwrap_or(NIL) };
    [m.c_dir.get(), m.c_object_space_weak_map.get(), lazy(th, b"ConditionVariable"), lazy(os, b"WeakKeyMap")]
        .into_iter().any(|k| k != NIL && is_class_or_module(k) && is_kind_of(c, v, k))
}
/// The references a traversal follows (CRuby: ivars, then the body's).
fn kids(c: &mut Ctx, v: Value, out: &mut Vec<Value>) {
    if obj_flags(v) & FL_EXIVAR != 0 || crate::object::is_object(v) { out.extend(ivar_list(c, v).into_iter().map(|p| p.1)); }
    match c.body_of(v) {
        Body::Array(_) => out.extend_from_slice(c.ary(v)),
        Body::Hash(_) => {
            let h = c.hash(v);
            out.extend(h.entries.iter().filter(|e| e.key != UNDEF).flat_map(|e| [e.key, e.val]));
            out.push(h.default); out.push(h.default_proc);
        }
        Body::Range(b, e, _) => { out.push(*b); out.push(*e); }
        Body::Data(_) => { let k = obj_klass(v); out.extend(crate::gc::reachable_from(c, v).into_iter().filter(|&x| x != k)); }
        _ => {}
    }
}
/// rb_ractor_shareable_p: a positive answer is remembered in FL_SHAREABLE (only for objects that
/// stay so: frozen ones whose references are shareable).
pub fn shareable(c: &mut Ctx, v: Value) -> bool {
    if !v.is_heap() || obj_flags(v) & FL_SHAREABLE != 0 { return true; }
    if !crate::object::is_class_or_module(v) && !frozen(c, v) && !internal(c, v) { return false; }
    let mut seen: HashSet<u64> = HashSet::default();
    let mut order = vec![];
    let mut work = vec![v];
    let mut ks = vec![];
    while let Some(x) = work.pop() {
        if !x.is_heap() || obj_flags(x) & FL_SHAREABLE != 0 { continue; }
        if crate::object::is_class_or_module(x) { obj_set_flags(x, FL_SHAREABLE); continue; }
        if !seen.insert(x.raw()) || internal(c, x) { continue; }
        if !frozen(c, x) || !frozen_shareable_kind(c, x) { return false; }
        order.push(x);
        ks.clear();
        kids(c, x, &mut ks);
        work.extend_from_slice(&ks);
    }
    for x in order { obj_set_flags(x, FL_SHAREABLE); }
    true
}

/// rb_ractor_make_shareable: freeze everything reachable (calling #freeze), procs isolated.
pub fn make_shareable(c: &mut Ctx, v: Value) -> Result<(), Esc> {
    let mut seen: HashSet<u64> = HashSet::default();
    let mut order = vec![];
    let mut work = vec![v];
    let mut ks = vec![];
    while let Some(x) = work.pop() {
        if !x.is_heap() || obj_flags(x) & FL_SHAREABLE != 0 { continue; }
        if !seen.insert(x.raw()) || internal(c, x) { continue; }
        if shareable(c, x) { continue; }
        if !frozen_shareable_kind(c, x) {
            match c.body_of(x) {
                Body::Proc(_) => { proc_make_shareable(c, x)?; order.push(x); continue; }
                Body::Method(m) => {
                    let recv = m.recv;
                    if shareable(c, recv) { obj_set_flags(x, FL_FROZEN); order.push(x); continue; }
                    let s = inspect_str(c, x)?;
                    return error(c, &E_ERROR, format!("can not make shareable object for {} because it refers unshareable objects", s));
                }
                _ => { let s = inspect_str(c, x)?; return error(c, &E_ERROR, format!("can not make shareable object for {}", s)); }
            }
        }
        if !frozen(c, x) {
            funcall(c, x, s::freeze, &[], std::ptr::null())?;
            if !is_frozen(x) { return error(c, &E_ERROR, "#freeze does not freeze object correctly"); }
        }
        order.push(x);
        ks.clear();
        kids(c, x, &mut ks);
        work.extend(ks.iter().rev());   // depth first, in order
    }
    for x in order { obj_set_flags(x, FL_SHAREABLE); }
    Ok(())
}

/// rb_proc_ractor_make_shareable: self and the outer variables it reads must be shareable.
fn proc_make_shareable(c: &mut Ctx, p: Value) -> Result<(), Esc> {
    let b = c.proc_data(p).blk;
    // Method#to_proc, Hash#to_proc, ... (CRuby: block_type is not iseq / symbol)
    if matches!(b.kind, BlockKind::Method(_) | BlockKind::Cfunc(..) | BlockKind::Proc(_)) { return err(c, c.vm().c_runtime_error.get(), "not supported yet"); }
    if !shareable(c, b.self_) {
        let s = inspect_str(c, p)?;
        return isolation(c, format!("Proc's self is not shareable: {}", s));
    }
    if let BlockKind::Iseq(is) = b.kind {
        let ov: Vec<Sym> = c.iseq_ref(is).map(|i| i.outer_vars.clone()).unwrap_or_default();
        if ov.contains(&s::yield_) { return arg_err(c, "can not make a Proc shareable because it uses 'yield'."); }
        if !ov.is_empty() {
            let bind = crate::builtins::binding::binding_of_proc(c, p)?;
            for n in ov {
                let v = funcall(c, bind, intern(b"local_variable_get"), &[Value::sym(n)], std::ptr::null())?;
                if !shareable(c, v) {
                    let s = inspect_str(c, v)?;
                    return isolation(c, format!("cannot make a shareable Proc because it can refer unshareable object {} from variable '{}'", s, sym_str(n)));
                }
            }
        }
    }
    obj_set_flags(p, FL_FROZEN);
    Ok(())
}

// ---- copy and move --------------------------------------------------------------------------

/// A reference slot of an object.
#[derive(Clone, Copy)]
enum Slot { Iv(Sym), El(usize), HKey(usize), HVal(usize), HDef, HProc, RBeg, REnd }
fn slots(c: &Ctx, v: Value) -> Vec<(Slot, Value)> {
    let mut out: Vec<(Slot, Value)> = vec![];
    if obj_flags(v) & FL_EXIVAR != 0 || crate::object::is_object(v) { out.extend(ivar_list(c, v).into_iter().map(|(n, x)| (Slot::Iv(n), x))); }
    match c.body_of(v) {
        Body::Array(_) => out.extend(c.ary(v).iter().enumerate().map(|(i, &x)| (Slot::El(i), x))),
        Body::Hash(_) => {
            let h = c.hash(v);
            for (i, e) in h.entries.iter().enumerate() { if e.key != UNDEF { out.push((Slot::HKey(i), e.key)); out.push((Slot::HVal(i), e.val)); } }
            out.push((Slot::HDef, h.default)); out.push((Slot::HProc, h.default_proc));
        }
        Body::Range(b, e, _) => { out.push((Slot::RBeg, *b)); out.push((Slot::REnd, *e)); }
        _ => {}
    }
    out
}
fn set_slot(c: &mut Ctx, v: Value, s: Slot, x: Value) {
    match s {
        Slot::Iv(n) => ivar_set_raw(c, v, n, x),
        Slot::El(i) => c.ary_mut(v)[i] = x,
        Slot::HKey(i) => c.hash_mut(v).entries[i].key = x,
        Slot::HVal(i) => c.hash_mut(v).entries[i].val = x,
        Slot::HDef => c.hash_mut(v).default = x,
        Slot::HProc => c.hash_mut(v).default_proc = x,
        Slot::RBeg | Slot::REnd => if let Body::Range(b, e, _) = c.body_mut_of(v) { if matches!(s, Slot::RBeg) { *b = x } else { *e = x } },
    }
}

/// Copy or move of an object graph (obj_traverse_replace_i): identity and cycles are kept.
struct Replace { mv: bool, rec: HashMap<u64, Value>, keep: Value, stack: Vec<(Value, Value, Vec<(Slot, Value)>, usize)> }
impl Replace {
    fn enter(&mut self, c: &mut Ctx, v: Value) -> RV {
        // Set is T_DATA in CRuby: copied only when its elements are shareable, never moved
        let set = const_get_at(c, c.vm().c_object.get(), intern(b"Set")).unwrap_or(NIL);
        if is_class_or_module(set) && is_kind_of(c, v, set) && (self.mv || !set_elems_shareable(c, v)) {
            let k = class_path(c, real_class_of(c, v));
            return error(c, &E_ERROR, format!("can not {} {} object.", if self.mv { "move" } else { "copy" }, k));
        }
        let n = if self.mv {
            if !matches!(c.body_of(v), Body::Object(_) | Body::String(_) | Body::Array(_) | Body::Hash(_) | Body::Range(..) | Body::Float(_) | Body::Bignum(_)) {
                let k = class_path(c, real_class_of(c, v));
                return error(c, &E_ERROR, format!("can not move {} object.", k));
            }
            c.alloc(obj_klass(v), Body::Object(crate::ivbuf::IvBuf::filled(0)))
        } else {
            match c.body_of(v) {
                Body::Proc(_) | Body::Method(_) => { let k = class_path(c, real_class_of(c, v)); return type_err(c, format!("allocator undefined for {}", k)); }
                _ => {}
            }
            crate::builtins::object::obj_dup(c, v, true, None)?
        };
        self.rec.insert(v.raw(), n);
        c.ary_push(self.keep, n);
        // a copy's references are fixed in the clone, a move's in the original (its body moves at leave)
        let at = if self.mv { v } else { n };
        if !self.mv && matches!(c.body_of(n), Body::Data(_)) && !refers_only_shareables(c, n) {
            let k = class_path(c, real_class_of(c, v));
            return error(c, &E_ERROR, format!("can not copy {} object.", k));
        }
        let sl = slots(c, at);
        self.stack.push((v, n, sl, 0));
        Ok(n)
    }
    fn run(&mut self, c: &mut Ctx, v: Value) -> RV {
        if !v.is_heap() || shareable(c, v) { return Ok(v); }
        let root = self.enter(c, v)?;
        while let Some(top) = self.stack.last_mut() {
            let (orig, n, i) = (top.0, top.1, top.3);
            if i == top.2.len() { self.stack.pop(); if self.mv { leave_move(c, orig, n); } continue; }
            let (slot, x) = top.2[i];
            top.3 += 1;
            let r = if !x.is_heap() || shareable(c, x) { x } else if let Some(&r) = self.rec.get(&x.raw()) { r } else { self.enter(c, x)? };
            if r != x { set_slot(c, if self.mv { orig } else { n }, slot, r); }
        }
        Ok(root)
    }
}
fn set_elems_shareable(c: &mut Ctx, v: Value) -> bool {
    let h = ivar_get(c, v, intern(b"__h"));
    if !crate::object::is_hash(h) { return true; }
    c.hash_pairs(h).into_iter().all(|(k, _)| shareable(c, k))
}
fn refers_only_shareables(c: &mut Ctx, v: Value) -> bool {
    let k = obj_klass(v);
    crate::gc::reachable_from(c, v).into_iter().filter(|&x| x != k).all(|x| shareable(c, x))
}
/// move_leave: the body goes to the new object; the old one becomes a frozen Ractor::MovedObject.
fn leave_move(c: &mut Ctx, v: Value, n: Value) {
    let gen = if obj_flags(v) & FL_EXIVAR != 0 { ivar_list(c, v) } else { vec![] };
    for (k, _) in &gen { ivar_remove(c, v, *k); }
    obj_clear_flags(v, FL_EXIVAR);
    let body = std::mem::replace(c.body_mut_of(v), Body::Object(crate::ivbuf::IvBuf::filled(0)));
    *c.body_mut_of(n) = body;
    obj_set_flags(n, obj_flags(v) & (FL_FROZEN | FL_CHILLED | FL_KWHASH | FL_TMPLOCK));
    for (k, x) in gen { ivar_set_raw(c, n, k, x); }
    obj_clear_flags(v, FL_FROZEN | FL_CHILLED | FL_KWHASH | FL_TMPLOCK);
    obj_set_flags(v, FL_FROZEN);
    let m = C_MOVED.get(c);
    obj_set_klass(v, m);
}
fn deep(c: &mut Ctx, v: Value, mv: bool) -> RV {
    let keep = c.ary_new();   // roots what is made: the record table is not traced
    let mut r = Replace { mv, rec: HashMap::default(), keep, stack: vec![] };
    r.run(c, v)
}
/// What a send delivers: the object itself when shareable, else a deep copy or the moved graph.
pub fn payload(c: &mut Ctx, v: Value, mv: bool) -> RV { deep(c, v, mv) }

// ---- ports ----------------------------------------------------------------------------------

fn new_port(c: &mut Ctx, owner: Value) -> Value {
    let r = rr_mut(c, owner);
    let id = r.next_port;
    r.next_port += 1;
    r.ports.push(PortQ { id, q: VecDeque::new(), closed: false });
    let k = C_PORT.get(c);
    let p = c.alloc_with(k, move || Body::Data(Box::new(RPort { owner, id })));
    obj_set_flags(p, FL_FROZEN | FL_SHAREABLE);
    p
}
fn queue(c: &Ctx, owner: Value, id: u64) -> Option<&PortQ> { rr(c, owner).ports.iter().find(|p| p.id == id) }
fn queue_mut(c: &mut Ctx, owner: Value, id: u64) -> Option<&mut PortQ> { rr_mut(c, owner).ports.iter_mut().find(|p| p.id == id) }
fn port_closed(c: &Ctx, owner: Value, id: u64) -> bool { rr(c, owner).done || queue(c, owner, id).is_none_or(|q| q.closed) }
/// A message waits on the port, or it is closed (receive raises).
fn port_ready(c: &Ctx, owner: Value, id: u64) -> bool { queue(c, owner, id).is_none_or(|q| q.closed || !q.q.is_empty()) }
fn deliver(c: &mut Ctx, owner: Value, id: u64, v: Value) -> Result<(), Esc> {
    if rr(c, owner).done { return closed_err(c); }
    match queue_mut(c, owner, id) { Some(q) if !q.closed => q.q.push_back(v), _ => return closed_err(c) }
    let k = evt(c);
    crate::builtins::blop::notify(c, k);
    Ok(())
}
fn port_send(c: &mut Ctx, pt: Value, v: Value, mv: bool) -> Result<(), Esc> {
    let x = payload(c, v, mv)?;
    let (o, id) = port(c, pt);
    deliver(c, o, id, x)
}
fn check_owner(c: &mut Ctx, owner: Value) -> Result<(), Esc> {
    if owner != current(c) { return error(c, &E_ERROR, "only allowed from the creator Ractor of this port"); }
    Ok(())
}
/// Take a message (None: closed and empty).
fn take(c: &mut Ctx, owner: Value, id: u64) -> Option<Value> { queue_mut(c, owner, id).and_then(|q| q.q.pop_front()) }
fn port_receive(c: &mut Ctx, pt: Value) -> RV {
    let (o, id) = port(c, pt);
    check_owner(c, o)?;
    let k = evt(c);
    thread::block_on(c, |c| Ok(port_ready(c, o, id)), Some(k), None, false)?;
    match take(c, o, id) { Some(v) => Ok(v), None => closed_err(c) }
}
fn port_close(c: &mut Ctx, pt: Value) -> RV {
    let (o, id) = port(c, pt);
    if o != current(c) { return error(c, &E_ERROR, "closing port by other ractors is not allowed"); }
    if let Some(q) = queue_mut(c, o, id) { q.closed = true; }
    let k = evt(c);
    crate::builtins::blop::notify(c, k);
    Ok(pt)
}

fn p_s_new(c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV { let r = current(c); Ok(new_port(c, r)) }
fn p_send(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    let (pos, mv) = move_kw(c, a)?;
    check_argc(c, pos, 1, 1)?;
    port_send(c, s, pos[0], mv)?;
    Ok(s)
}
fn p_receive(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { port_receive(c, s) }
fn p_close(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { port_close(c, s) }
fn p_closed(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { let (o, id) = port(c, s); Ok(Value::bool(port_closed(c, o, id))) }
fn p_inspect(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV {
    let (o, id) = port(c, s);
    let t = format!("#<Ractor::Port to:#{} id:{}>", rr(c, o).id, id);
    Ok(c.str_new(t.as_bytes()))
}
/// (positional args, move:) of a send.
fn move_kw<'a>(c: &mut Ctx, a: &'a [Value]) -> Result<(&'a [Value], bool), Esc> {
    let Some(h) = kw_arg(a) else { return Ok((a, false)) };
    let mut mv = false;
    for (k, v) in c.hash_pairs(h) {
        if k == sym_value("move") { mv = v.truthy(); } else { let s = inspect_str(c, k)?; return arg_err(c, format!("unknown keyword: {}", s)); }
    }
    Ok((&a[..a.len() - 1], mv))
}

// ---- Ractor ---------------------------------------------------------------------------------

fn alloc_ractor(c: &mut Ctx, klass: Value, id: u64, name: Value, loc: Value) -> Value {
    let l = c.hash_new();
    let r = c.alloc_with(klass, move || Body::Data(Box::new(RRactor {
        id, name, loc, thread: NIL, start: NIL, ports: vec![], next_port: 0, default_port: NIL, monitors: vec![],
        done: false, aborted: false, value: NIL, successor: NIL, locals: l,
    })));
    obj_set_flags(r, FL_SHAREABLE);
    let dp = new_port(c, r);
    rr_mut(c, r).default_port = dp;
    r
}

/// Ractor.new(*args, name: nil) { |*args| }: the block runs isolated, self = the new Ractor.
fn r_s_new(c: &mut Ctx, slf: Value, a: &[Value], b: *const Block) -> RV {
    let (args, kw) = match kw_arg(a) { Some(h) => (&a[..a.len() - 1], Some(h)), None => (a, None) };
    let mut name = NIL;
    if let Some(h) = kw {
        for (k, v) in c.hash_pairs(h) {
            if k == sym_value("name") { name = v; } else { let s = inspect_str(c, k)?; return arg_err(c, format!("unknown keyword: {}", s)); }
        }
    }
    if b.is_null() { return arg_err(c, "must be called with a block"); }
    if !STARTED.swap(true, Relaxed) {
        category_warn(c, "Ractor API is experimental and may change in future versions of Ruby.", 1, "experimental")?;
    }
    let loc = match c.caller_frame() { Some(f) => { let s = format!("{}:{}", frame_file(f), f.line()); c.str_new(s.as_bytes()) } None => NIL };
    if name != NIL {
        let s = expect_str(c, name)?;
        if c.str_bytes(s).contains(&0) { return arg_err(c, "string contains null byte"); }
        let n = c.str_copy(s);
        obj_set_flags(n, FL_FROZEN | FL_SHAREABLE);
        name = n;
    }
    if loc != NIL { obj_set_flags(loc, FL_FROZEN | FL_SHAREABLE); }
    let id = NEXT_ID.fetch_add(1, Relaxed);
    let p = block_to_proc(c, b)?;
    let p = isolate(c, p)?;
    let r = alloc_ractor(c, slf, id, name, loc);
    c.proc_data_mut(p).blk.self_ = r;
    // the arguments travel as messages do (rb_ractor_send_parameters)
    let mut av = Vec::with_capacity(args.len());
    let keep = c.ary_new_from(&[r]);
    for &x in args { let y = payload(c, x, false)?; c.ary_push(keep, y); av.push(y); }
    let av = c.ary_from_vec(av);
    let st = c.ary_new_from(&[p, av]);
    rr_mut(c, r).start = st;
    let blk = Block::cfunc(c, ractor_main, r);
    let tp = block_to_proc(c, &blk)?;
    if LIVE.fetch_add(1, Relaxed) == 0 {
        // constant caches may hold values a non-main Ractor must not see (rt::srb_const_get)
        let m = c.vm();
        m.const_serial.set(m.const_serial.get() + 1);
    }
    let t = thread::spawn_ractor_thread(c, tp, r)?;
    rr_mut(c, r).thread = t;
    // it starts at once, as CRuby's runs in parallel: its status is not "created" after this
    thread::pass(c)?;
    Ok(r)
}

/// rb_proc_isolate_bang: the block may not use outer variables or `yield`; a copy runs.
fn isolate(c: &mut Ctx, p: Value) -> RV {
    let pd = c.proc_data(p);
    let (blk, is_lambda) = (pd.blk, pd.is_lambda);
    if let BlockKind::Iseq(is) = blk.kind {
        let ov: Vec<Sym> = c.iseq_ref(is).map(|i| i.outer_vars.clone()).unwrap_or_default();
        let y = ov.contains(&s::yield_);
        let names: Vec<String> = ov.iter().filter(|&&n| n != s::yield_).map(|&n| sym_str(n)).collect();
        if !names.is_empty() {
            return arg_err(c, format!("can not isolate a Proc because it accesses outer variables ({}){}", names.join(", "), if y { " and uses 'yield'." } else { "." }));
        }
        if y { return arg_err(c, "can not isolate a Proc because it uses 'yield'."); }
    }
    let k = c.vm().c_proc.get();
    let n = c.alloc_with(k, move || Body::Proc(Box::new(ProcData { blk, is_lambda, from_method: NIL })));
    c.proc_data_mut(n).blk.proc_val = n;
    obj_set_flags(n, FL_SHAREABLE);
    Ok(n)
}

/// The body of a Ractor's main thread.
fn ractor_main(c: &mut Ctx, r: Value, _a: &[Value], _b: *const Block) -> RV {
    let st = std::mem::replace(&mut rr_mut(c, r).start, NIL);
    let p = c.ary_get(st, 0);
    let args = c.ary_get(st, 1);
    let argv = c.ary(args).to_vec();
    // a class ivar read node_ivar deferred may be the last thing the block did
    let res = call_proc(c, p, &argv, std::ptr::null()).and_then(|v| check_deferred(c).map(|_| v));
    match res {
        Ok(v) => finish(c, r, v, false),
        Err(_) => {
            let (ek, ev) = (c.esc, c.esc_val);
            finish(c, r, if ek == EscKind::Raise { ev } else { NIL }, true);
            c.esc = ek; c.esc_val = ev;
        }
    }
    res
}
/// ractor_notify_exit: ports close, monitors hear :exited / :aborted.
fn finish(c: &mut Ctx, r: Value, v: Value, aborted: bool) {
    let x = rr_mut(c, r);
    if x.done { return; }
    x.done = true; x.aborted = aborted; x.value = v;
    for p in &mut x.ports { p.closed = true; p.q.clear(); }
    let mons = std::mem::take(&mut x.monitors);
    LIVE.fetch_sub(1, Relaxed);
    let tok = sym_value(if aborted { "aborted" } else { "exited" });
    for m in mons { let (o, id) = port(c, m); let _ = deliver(c, o, id, tok); }
    let k = evt(c);
    crate::builtins::blop::notify(c, k);
}

fn r_s_current(c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(current(c)) }
fn r_s_main(c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(MAIN_R.get(c)) }
fn r_s_main_p(c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::bool(thread::current_ractor(c) == NIL)) }
fn r_s_count(_c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::fix(1 + LIVE.load(Relaxed) as i64)) }
fn r_s_receive(c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV { let p = rr(c, current(c)).default_port; port_receive(c, p) }
fn r_receive(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { let p = rr(c, s).default_port; port_receive(c, p) }
fn r_send(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    let (pos, mv) = move_kw(c, a)?;
    check_argc(c, pos, 1, 1)?;
    let p = rr(c, s).default_port;
    port_send(c, p, pos[0], mv)?;
    Ok(s)
}
fn r_close(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { let p = rr(c, s).default_port; port_close(c, p) }
fn r_default_port(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(rr(c, s).default_port) }
fn r_name(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(rr(c, s).name) }
fn status(c: &Ctx, r: Value) -> &'static str {
    let x = rr(c, r);
    if x.done { return "terminated"; }
    if x.thread == NIL || r == current(c) { return "running"; }
    match thread::thread_state(c, x.thread) { (false, _, _) => "created", (_, _, true) => "terminated", (_, true, _) => "blocking", _ => "running" }
}
fn r_inspect(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV {
    let x = rr(c, s);
    let name = if x.name != NIL { format!(" {}", String::from_utf8_lossy(c.str_bytes(x.name))) } else { String::new() };
    let loc = if x.loc != NIL { format!(" {}", String::from_utf8_lossy(c.str_bytes(x.loc))) } else { String::new() };
    let t = format!("#<Ractor:#{}{}{} {}>", x.id, name, loc, status(c, s));
    Ok(c.str_new(t.as_bytes()))
}
/// Wait for the end; the value (or RemoteError) for the successor only.
fn wait_end(c: &mut Ctx, r: Value) -> Result<(), Esc> {
    let k = evt(c);
    thread::block_on(c, |c| Ok(rr(c, r).done), Some(k), None, false)?;
    Ok(())
}
fn take_value(c: &mut Ctx, r: Value) -> RV {
    let me = current(c);
    let x = rr_mut(c, r);
    if x.successor == NIL { x.successor = me; }
    if x.successor != me { return error(c, &E_ERROR, "Only the successor ractor can take a value"); }
    let (v, aborted) = (x.value, x.aborted);
    if aborted {
        let e = make_exception(c, E_REMOTE.get(c), "thrown by remote Ractor.");
        ivar_set_raw(c, e, intern(b"@ractor"), r);
        ivar_set_raw(c, e, s::cause, v);
        return Err(raise_exc(c, e));
    }
    Ok(v)
}
/// CRuby's join / select make a Port in the caller to monitor with: its id is used up.
fn burn_port_id(c: &mut Ctx) { let me = current(c); rr_mut(c, me).next_port += 1; }
fn r_join(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV {
    burn_port_id(c);
    wait_end(c, s)?;
    if rr(c, s).aborted { take_value(c, s)?; }
    Ok(s)
}
fn r_value(c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { burn_port_id(c); wait_end(c, s)?; take_value(c, s) }
fn r_monitor(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    if !is_port(c, a[0]) { return type_err(c, "wrong argument type (expected Ractor::Port)"); }
    let x = rr(c, s);
    if !x.done { rr_mut(c, s).monitors.push(a[0]); return Ok(TRUE); }
    let tok = sym_value(if x.aborted { "aborted" } else { "exited" });
    port_send(c, a[0], tok, false)?;
    Ok(FALSE)
}
fn r_unmonitor(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    if !is_port(c, a[0]) { return type_err(c, "wrong argument type (expected Ractor::Port)"); }
    rr_mut(c, s).monitors.retain(|&m| m != a[0]);
    Ok(s)
}
/// Ractor.select(*ports_or_ractors): [port, message] or [ractor, its value], the first ready in order.
fn r_s_select(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    if a.is_empty() { return arg_err(c, "specify at least one Ractor::Port or Ractor"); }
    for &x in a { if !is_ractor(c, x) && !is_port(c, x) { return arg_err(c, "should be Ractor::Port or Ractor"); } }
    let me = current(c);
    for &x in a { if is_ractor(c, x) { burn_port_id(c); } }
    for &x in a { if is_port(c, x) { let (o, _) = port(c, x); if o != me { return error(c, &E_ERROR, "only allowed from the creator Ractor of this port"); } } }
    let ready = |c: &Ctx| a.iter().position(|&x| if is_ractor(c, x) { rr(c, x).done } else { let (o, id) = port(c, x); port_ready(c, o, id) });
    let k = evt(c);
    thread::block_on(c, |c| Ok(ready(c).is_some()), Some(k), None, false)?;
    let x = a[ready(c).expect("a ready port")];
    let v = if is_ractor(c, x) { take_value(c, x)? } else { let (o, id) = port(c, x); match take(c, o, id) { Some(v) => v, None => return closed_err(c) } };
    Ok(c.ary_new_from(&[x, v]))
}
fn local_key(c: &mut Ctx, k: Value) -> RV { thread::key_sym(c, k) }
fn r_aref(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    if s != current(c) { return err(c, c.vm().c_runtime_error.get(), "Cannot get ractor local storage for non-current ractor"); }
    let k = local_key(c, a[0])?;
    let h = rr(c, s).locals;
    Ok(hash_get_opt(c, h, k)?.unwrap_or(NIL))
}
fn r_aset(c: &mut Ctx, s: Value, a: &[Value], _b: *const Block) -> RV {
    if s != current(c) { return err(c, c.vm().c_runtime_error.get(), "Cannot set ractor local storage for non-current ractor"); }
    let k = local_key(c, a[0])?;
    let h = rr(c, s).locals;
    hash_aset(c, h, k, a[1])?;
    Ok(a[1])
}
fn r_s_aref(c: &mut Ctx, _s: Value, a: &[Value], b: *const Block) -> RV { let r = current(c); r_aref(c, r, a, b) }
fn r_s_aset(c: &mut Ctx, _s: Value, a: &[Value], b: *const Block) -> RV { let r = current(c); r_aset(c, r, a, b) }
fn r_s_store_if_absent(c: &mut Ctx, _s: Value, a: &[Value], b: *const Block) -> RV {
    let r = current(c);
    let k = local_key(c, a[0])?;
    let h = rr(c, r).locals;
    if let Some(v) = hash_get_opt(c, h, k)? { return Ok(v); }
    if b.is_null() { return err(c, c.vm().c_local_jump_error.get(), "no block given"); }
    let v = yield1(c, b, NIL)?;
    hash_aset(c, h, k, v)?;
    Ok(v)
}
fn r_s_shareable_p(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV { Ok(Value::bool(shareable(c, a[0]))) }
fn r_s_make_shareable(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    let (pos, copy) = match kw_arg(a) {
        Some(h) => {
            let mut cp = false;
            for (k, v) in c.hash_pairs(h) { if k == sym_value("copy") { cp = v.truthy(); } else { let s = inspect_str(c, k)?; return arg_err(c, format!("unknown keyword: {}", s)); } }
            (&a[..a.len() - 1], cp)
        }
        None => (a, false),
    };
    check_argc(c, pos, 1, 1)?;
    let v = if copy { payload(c, pos[0], false)? } else { pos[0] };
    make_shareable(c, v)?;
    Ok(v)
}
/// Ractor.shareable_proc / shareable_lambda(self: nil) { }
fn shareable_proc(c: &mut Ctx, a: &[Value], b: *const Block, lambda: bool) -> RV {
    if b.is_null() { return arg_err(c, "tried to create Proc object without a block"); }
    let mut slf = NIL;
    if let Some(h) = kw_arg(a) { if let Some(v) = hash_get_opt(c, h, sym_value("self"))? { slf = v; } }
    if !shareable(c, slf) { let s = inspect_str(c, slf)?; return isolation(c, format!("self should be shareable: {}", s)); }
    let p = block_to_proc(c, b)?;
    let blk = c.proc_data(p).blk;
    let k = c.vm().c_proc.get();
    let n = c.alloc_with(k, move || Body::Proc(Box::new(ProcData { blk, is_lambda: lambda, from_method: NIL })));
    let pd = c.proc_data_mut(n);
    pd.blk.proc_val = n; pd.blk.self_ = slf;
    proc_make_shareable(c, n)?;
    obj_set_flags(n, FL_SHAREABLE);
    Ok(n)
}

// ---- isolation checks (multi() is true; called from rt.rs / vm.rs / object.rs slow paths) ----

/// Globals a non-main Ractor may use (rb_gvar_ractor_local in CRuby 4.0).
fn gvar_local(name: &[u8]) -> bool {
    matches!(name, b"$~" | b"$&" | b"$`" | b"$'" | b"$+" | b"$@" | b"$!" | b"$/" | b"$-0" | b"$_" | b"$stdin" | b"$stdout" | b"$>" | b"$stderr"
        | b"$-i" | b"$$" | b"$?" | b"$-p" | b"$-l" | b"$-a" | b"$VERBOSE" | b"$-v" | b"$-w" | b"$-W" | b"$DEBUG" | b"$-d")
        || (name.len() > 1 && name[1] != b'0' && name[1..].iter().all(|b| b.is_ascii_digit()))   // $1..
}
#[cold]
#[inline(never)]
pub fn gvar_check(c: &mut Ctx, name: Sym) -> Result<(), Esc> {
    if !isolated(c) || gvar_local(sym_bytes(name)) { return Ok(()); }
    isolation(c, format!("can not access global variable {} from non-main Ractor", sym_str(name)))
}
#[cold]
#[inline(never)]
pub fn global_variables_check(c: &mut Ctx) -> Result<(), Esc> {
    if !isolated(c) { return Ok(()); }
    isolation(c, "can not access global variables from non-main Ractors")
}
/// The class that holds constant `name` as seen from `k` (rb_const_search's found_in).
fn const_owner(c: &mut Ctx, k: Value, name: Sym) -> Value {
    let anc = crate::vm::ancestors(c, k).to_vec();
    let o = c.vm().c_object.get();
    anc.into_iter().chain([o]).find(|&a| c.class(a).consts.contains_key(&name)).unwrap_or(k)
}
/// A constant's value read in a non-main Ractor must be shareable; `lexical`: found in an
/// enclosing scope (vm_get_ev_const's message), else by ancestry from `k`.
#[cold]
#[inline(never)]
pub fn const_check(c: &mut Ctx, k: Value, lexical: bool, name: Sym, v: Value) -> Result<(), Esc> {
    if !isolated(c) || shareable(c, v) { return Ok(()); }
    let (o, r) = if lexical { (k, "ractor") } else { (const_owner(c, k, name), "Ractor") };
    let p = class_path(c, o);
    isolation(c, format!("can not access non-shareable objects in constant {}::{} by non-main {}.", p, sym_str(name), r))
}
/// The class a lexical constant lookup from `cref` found `name` in, and whether it was lexical.
pub fn const_found_in(c: &mut Ctx, cref: *const Cref, name: Sym) -> (Value, bool) {
    let mut cr = c.cref_ref(cref);
    while let Some(x) = cr {
        if c.cref_ref(x.prev).is_none() { break; }   // the top-level scope is not searched lexically
        if c.class(x.klass).consts.contains_key(&name) { return (x.klass, true); }
        cr = c.cref_ref(x.prev);
    }
    let k = c.cref_ref(cref).map_or(c.vm().c_object.get(), |x| x.klass);
    (k, false)
}
#[cold]
#[inline(never)]
pub fn cdecl_check(c: &mut Ctx, v: Value) -> Result<(), Esc> {
    if !isolated(c) || shareable(c, v) { return Ok(()); }
    isolation(c, "can not set constants with non-shareable objects by non-main Ractors")
}
#[cold]
#[inline(never)]
pub fn cvar_check(c: &mut Ctx, k: Value, name: Sym) -> Result<(), Esc> {
    if !isolated(c) { return Ok(()); }
    let p = class_path(c, k);
    isolation(c, format!("can not access class variables from non-main Ractors ({} from {})", sym_str(name), p))
}
/// An instance variable of a class or module read in a non-main Ractor.
#[cold]
#[inline(never)]
pub fn class_ivar_get_check(c: &mut Ctx, k: Value, name: Sym, v: Value) -> Result<(), Esc> {
    if !crate::object::is_class_or_module(k) || !isolated(c) || sym_bytes(name).first() != Some(&b'@') || shareable(c, v) { return Ok(()); }
    let p = class_path(c, k);
    isolation(c, format!("can not get unshareable values from instance variables of classes/modules from non-main Ractors ({} from {})", sym_str(name), p))
}
/// attr_reader / attr_writer outside the plain-object fast path (vm::invoke_untraced).
#[inline(never)]
pub fn attr_get(c: &mut Ctx, recv: Value, id: Sym) -> RV {
    let v = ivar_get(c, recv, id);
    if multi() { class_ivar_get_check(c, recv, id, v)?; }
    Ok(v)
}
#[inline(never)]
pub fn attr_set(c: &mut Ctx, recv: Value, id: Sym, v: Value) -> RV {
    if multi() { class_ivar_set_check(c, recv, id)?; }
    ivar_set(c, recv, id, v)
}
/// node_ivar's miss path read a class's ivar (it cannot raise): a non-main Ractor gets the check at
/// its next check point (the interrupt bit makes the next call site poll; thread::poll calls
/// `check_deferred`), before the value can reach a method call.
static DEFERRED_K: Global = Global::new();
static DEFERRED_R: Global = Global::new();   // the Ractor that read it
static DEFERRED_N: AtomicU64 = AtomicU64::new(0);   // name + 1, 0: nothing pending
#[cold]
#[inline(never)]
pub fn class_ivar_read(c: &Ctx, k: Value, name: Sym, v: Value) {
    if !crate::object::is_class_or_module(k) || !isolated(c) || !v.is_heap() || obj_flags(v) & FL_SHAREABLE != 0 || crate::object::is_class_or_module(v) { return; }
    if sym_bytes(name).first() != Some(&b'@') { return; }
    DEFERRED_K.set_vm(c.vm(), k);
    DEFERRED_R.set_vm(c.vm(), thread::current_ractor(c));
    DEFERRED_N.store(name.0 as u64 + 1, Relaxed);
    crate::vm::set_ints();
}
#[cold]
#[inline(never)]
pub fn check_deferred(c: &mut Ctx) -> Result<(), Esc> {
    if DEFERRED_N.load(Relaxed) == 0 || DEFERRED_R.get(c) != thread::current_ractor(c) { return Ok(()); }
    let n = DEFERRED_N.swap(0, Relaxed);
    let name = Sym((n - 1) as u32);
    let k = DEFERRED_K.get(c);
    DEFERRED_K.set(c, NIL);
    let v = ivar_get(c, k, name);
    class_ivar_get_check(c, k, name, v)
}
#[cold]
#[inline(never)]
pub fn class_ivar_set_check(c: &mut Ctx, k: Value, name: Sym) -> Result<(), Esc> {
    if !crate::object::is_class_or_module(k) || !isolated(c) || sym_bytes(name).first() != Some(&b'@') { return Ok(()); }
    isolation(c, "can not set instance variables of classes/modules by non-main Ractors")
}

// ---- Ractor::MovedObject ----------------------------------------------------------------------

fn moved_missing(c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV { error(c, &E_MOVED, "can not send any methods to a moved object") }

/// The "saruby/ractor" feature: Ractor is defined on first reference.
pub fn init(c: &mut Ctx) {
    if C_RACTOR.get(c) != NIL { return; }
    let vmr = c.vm();
    let ob = vmr.c_object.get();
    let k = define_class(c, "Ractor", ob);
    class_set_alloc(c, k, AllocKind::None);
    C_RACTOR.set(c, k);
    let rt = vmr.c_runtime_error.get();
    let e = define_class_under(c, k, "Error", rt);
    E_ERROR.set(c, e);
    let sub = |c: &mut Ctx, g: &Global, n: &str, sup: Value| { let x = define_class_under(c, k, n, sup); g.set(c, x); };
    sub(c, &E_REMOTE, "RemoteError", e);
    sub(c, &E_CLOSED, "ClosedError", vmr.c_stop_iteration.get());
    sub(c, &E_ISOLATION, "IsolationError", e);
    sub(c, &E_MOVED, "MovedError", e);
    define_class_under(c, k, "UnsafeError", e);
    let re = E_REMOTE.get(c);
    define_cfunc(c, re, "ractor", |c, s, _a, _b| Ok(ivar_get(c, s, intern(b"@ractor"))), 0);
    let mo = define_class_under(c, k, "MovedObject", vmr.c_basic_object.get());
    class_set_alloc(c, mo, AllocKind::None);
    C_MOVED.set(c, mo);
    for n in ["method_missing", "__send__", "!", "==", "!=", "__id__", "equal?", "instance_eval", "instance_exec"] { define_cfunc(c, mo, n, moved_missing, -1); }
    let pk = define_class_under(c, k, "Port", ob);
    class_set_alloc(c, pk, AllocKind::None);
    C_PORT.set(c, pk);
    let main = alloc_ractor(c, k, 1, NIL, NIL);   // after Port: it has a default port
    MAIN_R.set(c, main);
    define_singleton_cfunc(c, pk, "new", p_s_new, 0);
    let dp = |c: &mut Ctx, n: &str, f: CFunc, a: i32| define_cfunc(c, pk, n, f, a);
    dp(c, "send", p_send, -1); dp(c, "<<", p_send, -1); dp(c, "receive", p_receive, 0); dp(c, "close", p_close, 0); dp(c, "closed?", p_closed, 0); dp(c, "inspect", p_inspect, 0);
    let ds = |c: &mut Ctx, n: &str, f: CFunc, a: i32| define_singleton_cfunc(c, k, n, f, a);
    ds(c, "new", r_s_new, -1); ds(c, "current", r_s_current, 0); ds(c, "main", r_s_main, 0); ds(c, "main?", r_s_main_p, 0); ds(c, "count", r_s_count, 0);
    ds(c, "receive", r_s_receive, 0); ds(c, "recv", r_s_receive, 0); ds(c, "select", r_s_select, -1);
    ds(c, "[]", r_s_aref, 1); ds(c, "[]=", r_s_aset, 2); ds(c, "store_if_absent", r_s_store_if_absent, 1);
    ds(c, "shareable?", r_s_shareable_p, 1); ds(c, "make_shareable", r_s_make_shareable, -1);
    ds(c, "shareable_proc", |c, _s, a, b| shareable_proc(c, a, b, false), -1);
    ds(c, "shareable_lambda", |c, _s, a, b| shareable_proc(c, a, b, true), -1);
    let d = |c: &mut Ctx, n: &str, f: CFunc, a: i32| define_cfunc(c, k, n, f, a);
    d(c, "send", r_send, -1); d(c, "<<", r_send, -1); d(c, "close", r_close, 0); d(c, "default_port", r_default_port, 0);
    d(c, "name", r_name, 0); d(c, "inspect", r_inspect, 0); d(c, "to_s", r_inspect, 0);
    d(c, "[]", r_aref, 1); d(c, "[]=", r_aset, 2); d(c, "join", r_join, 0); d(c, "value", r_value, 0);
    d(c, "monitor", r_monitor, 1); d(c, "unmonitor", r_unmonitor, 1);
    define_private_cfunc(c, k, "receive", r_receive, 0); define_private_cfunc(c, k, "recv", r_receive, 0);
}

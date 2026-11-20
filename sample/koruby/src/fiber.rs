// Fibers: each fiber has its own native stack (mmap) and its own Ctx (VM value
// stack + frame chain).  Switching saves callee-saved registers on the current
// stack and swaps rsp (x86-64 System V).  The GC scans the running stack
// conservatively up to its top, and every suspended stack from its saved rsp.
use crate::array::*;
use crate::astrust_gc::stack;
use crate::gc::{Koruby, RootVisitor};
use crate::value::*;
use crate::vm::*;

const STACK_SIZE: usize = 16 << 20;
const VSTACK: usize = (1 << 18) + FRAME;   // the last FRAME slots cannot start a frame

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FState { Created, Running, Suspended, Done }

pub struct FiberData {
    pub ctx: Box<Ctx>,
    pub vstack: Stack,
    pub stack: *mut u8,
    pub sp: usize,
    pub state: FState,
    pub proc_: Value,
    pub xfer: Value,
    pub err: bool,
    pub resumer: Value,
    pub peek: Option<Value>,
    pub enum_src: Value,
}
impl RData for FiberData {
    fn visit(&mut self, m: &mut crate::gc::Visitor) { m.visit(&mut self.proc_); m.visit(&mut self.xfer); m.visit(&mut self.resumer); m.visit(&mut self.enum_src); if let Some(p) = &mut self.peek { m.visit(p); } }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

static mut CUR: Value = NIL;
static mut MAIN_SP: usize = 0;
static mut LIVE: Vec<Value> = Vec::new();

fn fd(v: Value) -> &'static mut FiberData { rdata::<FiberData>(v).unwrap() }

#[cfg(target_arch = "x86_64")]
#[unsafe(naked)]
unsafe extern "C" fn switch_stack(_save: *mut usize, _to: usize) {
    std::arch::naked_asm!(
        "push rbp", "push rbx", "push r12", "push r13", "push r14", "push r15",
        "mov [rdi], rsp",
        "mov rsp, rsi",
        "pop r15", "pop r14", "pop r13", "pop r12", "pop rbx", "pop rbp",
        "ret",
    );
}

#[cfg(target_arch = "x86_64")]
#[unsafe(naked)]
unsafe extern "C" fn fiber_start() {
    std::arch::naked_asm!("and rsp, -16", "call {main}", "ud2", main = sym fiber_main);
}

extern "C" { fn mmap(a: *mut u8, l: usize, p: i32, f: i32, fd: i32, o: i64) -> *mut u8; fn munmap(a: *mut u8, l: usize) -> i32; }

/// Top (highest address) of the stack the CPU is running on.
pub fn current_stack_top() -> usize {
    unsafe { if CUR == NIL { crate::gc::stack_base() } else { fd(CUR).stack as usize + STACK_SIZE } }
}

/// Roots held by fibers: every suspended machine stack (ambiguous), each fiber's Ctx.
pub fn visit_fibers<V: RootVisitor<Koruby>>(m: &mut V) {
    unsafe {
        if CUR != NIL {
            m.visit_ambiguous(stack::words(MAIN_SP, crate::gc::stack_base()));
            m.visit(&mut *(&raw mut CUR));
        }
        for f in (*(&raw mut LIVE)).iter_mut() {
            m.visit(f);
            let d = fd(*f);
            visit_ctx(m, &mut d.ctx);
            if d.state == FState::Suspended && d.sp != 0 { m.visit_ambiguous(stack::words(d.sp, d.stack as usize + STACK_SIZE)); }
        }
    }
}

extern "C" fn fiber_main() -> ! {
    unsafe {
        let f = CUR;
        let d = fd(f);
        let c: &mut Ctx = &mut d.ctx;
        let mut top = Frame::empty(c.bottom());
        c.frame = &mut top.h;
        let args = d.xfer;
        let argv: Vec<Value> = if is_array(args) { rary(args).clone() } else { vec![args] };
        let r = call_proc(c, d.proc_, &argv, std::ptr::null());
        let d = fd(f);
        match r {
            Ok(v) => { d.xfer = v; d.err = false; }
            Err(_) => {
                d.err = true;
                d.xfer = if c.esc == EscKind::Raise { c.esc_val } else {
                    make_exception(vm().c_local_jump_error, "break from proc-closure")
                };
                c.esc = EscKind::None;
            }
        }
        d.state = FState::Done;
        LIVE.retain(|&x| x != f);
        let back = d.resumer;
        CUR = back;
        let mut dummy = 0usize;
        let to = if back == NIL { MAIN_SP } else { fd(back).state = FState::Running; fd(back).sp };
        switch_stack(&mut dummy, to);
        unreachable!();
    }
}

fn new_fiber(klass: Value, proc_: Value) -> Value {
    let stack = unsafe { mmap(std::ptr::null_mut(), STACK_SIZE, 3, 0x22 | 0x4000, -1, 0) };
    if stack as isize == -1 { panic!("koruby: cannot allocate fiber stack"); }
    let vstack = stack_new(VSTACK);
    let mut ctx = Box::new(Ctx {
        sp: NO_SP, stk: NO_STACK, frame: std::ptr::null_mut(),
        esc: EscKind::None, esc_val: NIL, esc_tag: 0, esc_tag_v: NIL, serial: 1 << 40, errinfo: NIL, last_match: NIL,
        native_limit: stack as usize + (512 << 10), in_ensure: 0, call_me: std::ptr::null(), call_blk: std::ptr::null(),
    });
    ctx.attach_stack(vstack);
    // initial frame: six zero registers, then the return address fiber_start
    let top = (stack as usize + STACK_SIZE) & !15;
    let sp = top - 8 * 8;
    unsafe {
        let p = sp as *mut usize;
        for i in 0..6 { *p.add(i) = 0; }
        *p.add(6) = fiber_start as *const () as usize;
        *p.add(7) = 0;
    }
    let v = crate::gc::alloc(klass, Body::Data(Box::new(FiberData { ctx, vstack, stack, sp, state: FState::Created, proc_, xfer: NIL, err: false, resumer: NIL, peek: None, enum_src: NIL })));
    unsafe { LIVE.push(v); }
    v
}

// ---- green threads (builtins/thread.rs): a thread is a fiber chain entered from the scheduler ----

static mut SCHED: Value = NIL;     // fiber the scheduler runs in (NIL = main stack)
static mut LAST_OUT: Value = NIL;  // innermost fiber of the thread that just suspended

pub fn new_thread_fiber(proc_: Value, args: Value) -> Value { let f = new_fiber(vm().c_fiber, proc_); fd(f).xfer = args; f }
pub fn is_done(f: Value) -> bool { fd(f).state == FState::Done }
pub fn take_result(f: Value) -> (bool, Value) { let d = fd(f); let r = (d.err, d.xfer); d.err = false; d.xfer = NIL; r }
pub fn last_switched_out() -> Value { unsafe { LAST_OUT } }
pub fn discard(f: Value) { unsafe { LIVE.retain(|&x| x != f); } fd(f).state = FState::Done; }
/// Run a thread (root fiber `root`, resuming at `inner`) until it suspends or ends.
pub fn thread_switch_in(root: Value, inner: Value) {
    unsafe {
        let prev = CUR;
        fd(root).resumer = prev;
        let d = fd(inner);
        d.state = FState::Running;
        let save: *mut usize = if prev == NIL { &raw mut MAIN_SP } else { fd(prev).state = FState::Suspended; &mut fd(prev).sp };
        let old = SCHED;
        SCHED = prev;
        CUR = inner;
        switch_stack(save, d.sp);
        SCHED = old;
        CUR = prev;
        if prev != NIL { fd(prev).state = FState::Running; }
    }
}
/// Suspend the running thread (whatever fiber it is in) back to the scheduler.
pub fn thread_switch_out() {
    unsafe {
        let cur = CUR;
        let d = fd(cur);
        d.state = FState::Suspended;
        LAST_OUT = cur;
        let back = SCHED;
        let to = if back == NIL { MAIN_SP } else { fd(back).sp };
        switch_stack(&mut d.sp, to);
    }
}

pub fn on_free(p: *mut RObj) {
    unsafe {
        if let Body::Data(d) = &(*p).body {
            if let Some(f) = d.as_any().downcast_ref::<FiberData>() { munmap(f.stack, STACK_SIZE); stack_release(f.vstack); }
        }
    }
}

/// Switch into fiber f passing v; returns what it yields/returns.
pub fn resume(c: &mut Ctx, f: Value, v: Value) -> RV {
    let d = fd(f);
    match d.state {
        FState::Done => return err(c, vm().c_fiber_error, "attempt to resume a terminated fiber"),
        FState::Running => return err(c, vm().c_fiber_error, "attempt to resume the current fiber"),
        _ => {}
    }
    if d.resumer != NIL && d.state == FState::Suspended && is_resuming(f) { return err(c, vm().c_fiber_error, "attempt to resume a resuming fiber"); }
    unsafe {
        let prev = CUR;
        d.resumer = prev;
        d.xfer = v;
        d.state = FState::Running;
        let save: *mut usize = if prev == NIL { &raw mut MAIN_SP } else { fd(prev).state = FState::Suspended; &mut fd(prev).sp };
        CUR = f;
        let to = d.sp;
        switch_stack(save, to);
        // back here after f yielded or finished
        CUR = prev;
        if prev != NIL { fd(prev).state = FState::Running; }
    }
    let d = fd(f);
    let r = d.xfer;
    d.xfer = NIL;
    if d.err { d.err = false; return Err(raise_exc(c, r)); }
    Ok(r)
}

fn is_resuming(_f: Value) -> bool { false }

/// Fiber.yield from the current fiber.
pub fn fiber_yield(c: &mut Ctx, v: Value) -> RV {
    let f = unsafe { CUR };
    if f == NIL { return err(c, vm().c_fiber_error, "can't yield from root fiber"); }
    let d = fd(f);
    d.xfer = v;
    d.state = FState::Suspended;
    unsafe {
        let back = d.resumer;
        let to = if back == NIL { MAIN_SP } else { fd(back).sp };
        switch_stack(&mut d.sp, to);
    }
    let d = fd(f);
    let r = d.xfer;
    d.xfer = NIL;
    Ok(r)
}

fn args_value(a: &[Value]) -> Value { match a.len() { 0 => NIL, 1 => a[0], _ => ary_new_from(a) } }

fn f_s_new(c: &mut Ctx, slf: Value, _a: &[Value], b: *const Block) -> RV {
    if b.is_null() { return arg_err(c, "tried to create Proc object without a block"); }
    let p = block_to_proc(c, b)?;
    Ok(new_fiber(slf, p))
}
fn f_resume(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let v = if fd(slf).state == FState::Created { ary_new_from(a) } else { args_value(a) };
    resume(c, slf, v)
}
fn f_s_yield(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV { fiber_yield(c, args_value(a)) }
fn f_alive(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::bool(fd(slf).state != FState::Done)) }
fn f_s_current(_c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(unsafe { CUR }) }

// ---- Enumerator#next / #peek --------------------------------------------------------------

fn enum_feed(c: &mut Ctx, _data: Value, args: &[Value], _b: *const Block) -> RV {
    let v = args_value(args);
    fiber_yield(c, v)?;
    Ok(NIL)
}
fn enum_body(c: &mut Ctx, e: Value, _args: &[Value], _b: *const Block) -> RV {
    let blk = Block::cfunc(c, enum_feed, NIL);
    let r = crate::builtins::enumerator::enum_each_with(c, e, &blk)?;
    let f = unsafe { CUR };
    fd(f).enum_src = r;
    Ok(UNDEF)
}

fn enum_fiber(c: &mut Ctx, e: Value) -> Value {
    let en = crate::builtins::enumerator::renum(e).unwrap();
    if en.fiber != NIL { return en.fiber; }
    let blk = Block::cfunc(c, enum_body, e);
    let p = crate::gc::alloc(vm().c_proc, Body::Proc(Box::new(ProcData { blk, is_lambda: false, from_method: NIL })));
    rproc(p).blk.proc_val = p;
    let f = new_fiber(vm().c_fiber, p);
    crate::builtins::enumerator::renum(e).unwrap().fiber = f;
    f
}

fn enum_pull(c: &mut Ctx, e: Value) -> Result<Option<Value>, Esc> {
    let f = enum_fiber(c, e);
    if let Some(v) = fd(f).peek.take() { return Ok(Some(v)); }
    if fd(f).state == FState::Done { return Ok(None); }
    let v = resume(c, f, NIL)?;
    if fd(f).state == FState::Done { return Ok(None); }
    let _ = v;
    Ok(Some(v))
}

fn stop_iteration(c: &mut Ctx, e: Value) -> Esc {
    let en = crate::builtins::enumerator::renum(e).unwrap();
    let f = en.fiber;
    let res = if f != NIL { fd(f).enum_src } else { NIL };
    let ex = make_exception(vm().c_stop_iteration, "iteration reached an end");
    ivar_set_raw(ex, crate::symbol::intern(b"result"), res);
    raise_exc(c, ex)
}

pub fn enum_next(c: &mut Ctx, e: Value) -> Result<Option<Value>, Esc> {
    match enum_pull(c, e)? { Some(v) => Ok(Some(v)), None => Err(stop_iteration(c, e)) }
}
pub fn enum_has_fiber(_e: Value) -> bool { true }
pub fn enum_peek(c: &mut Ctx, e: Value) -> RV {
    match enum_pull(c, e)? {
        Some(v) => { let f = crate::builtins::enumerator::renum(e).unwrap().fiber; fd(f).peek = Some(v); Ok(v) }
        None => Err(stop_iteration(c, e)),
    }
}

pub fn init() {
    let vm = vm();
    let k = define_class("Fiber", vm.c_object);
    rclass(k).alloc = AllocKind::None;
    vm.c_fiber = k;
    define_singleton_cfunc(k, "new", f_s_new, -1);
    define_singleton_cfunc(k, "yield", f_s_yield, -1);
    define_singleton_cfunc(k, "current", f_s_current, 0);
    define_cfunc(k, "resume", f_resume, -1);
    define_cfunc(k, "alive?", f_alive, 0);
    vm.data_free_hooks.push(on_free);
}

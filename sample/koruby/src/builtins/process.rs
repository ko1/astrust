// Process / Process::Status / clock.
use crate::object::*;
use crate::symbol::intern;
use crate::value::*;
use crate::vm::*;

static mut LAST_STATUS: i64 = -1;
pub fn set_last_status(st: i32) { unsafe { LAST_STATUS = st as i64; } }
pub fn last_status() -> Value {
    let st = unsafe { LAST_STATUS };
    if st < 0 { return NIL; }
    let k = const_get_at(vm().m_process, intern(b"Status")).unwrap();
    let o = new_object(k);
    ivar_set_raw(o, intern(b"@status"), Value::fix(st));
    o
}

fn clock_gettime(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    let id = a.first().copied().unwrap_or(Value::fix(1));
    let unit = a.get(1).copied().unwrap_or(NIL);
    let secs = if id == Value::fix(0) {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs_f64()
    } else if id == Value::fix(2) || id == Value::fix(3) {
        cpu_time()
    } else {
        mono().elapsed().as_secs_f64() + 1000.0
    };
    if unit == NIL || unit == sym_value("float_second") { return Ok(crate::numeric::float_new(secs)); }
    let u = crate::symbol::sym_str(unit.as_sym());
    let _ = c;
    Ok(match u.as_str() {
        "nanosecond" => Value::fix((secs * 1e9) as i64),
        "microsecond" => Value::fix((secs * 1e6) as i64),
        "millisecond" => Value::fix((secs * 1e3) as i64),
        "second" => Value::fix(secs as i64),
        "float_millisecond" => crate::numeric::float_new(secs * 1e3),
        "float_microsecond" => crate::numeric::float_new(secs * 1e6),
        _ => crate::numeric::float_new(secs),
    })
}
fn mono() -> &'static std::time::Instant {
    static mut T: Option<std::time::Instant> = None;
    unsafe { if T.is_none() { T = Some(std::time::Instant::now()); } T.as_ref().unwrap() }
}
extern "C" { fn clock(); }
fn cpu_time() -> f64 {
    #[repr(C)] struct Ts { s: i64, ns: i64 }
    extern "C" { fn clock_gettime(id: i32, ts: *mut Ts) -> i32; }
    let mut t = Ts { s: 0, ns: 0 };
    unsafe { clock_gettime(2, &mut t); }
    t.s as f64 + t.ns as f64 * 1e-9
}
fn pid(_c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::fix(std::process::id() as i64)) }
fn st_exitstatus(_c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(ivar_get(s, intern(b"@status"))) }
fn st_success(_c: &mut Ctx, s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::bool(ivar_get(s, intern(b"@status")) == Value::fix(0))) }
fn times(_c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV {
    let t = cpu_time();
    let k = const_get_at(vm().m_process, intern(b"Tms")).unwrap();
    let o = new_object(k);
    ivar_set_raw(o, intern(b"@utime"), crate::numeric::float_new(t));
    ivar_set_raw(o, intern(b"@stime"), crate::numeric::float_new(0.0));
    Ok(o)
}

pub fn init() {
    let vm = vm();
    let p = define_module("Process");
    vm.m_process = p;
    define_module_function(p, "clock_gettime", clock_gettime, -1);
    define_module_function(p, "pid", pid, 0);
    define_module_function(p, "times", times, 0);
    for (n, v) in [("CLOCK_REALTIME", 0), ("CLOCK_MONOTONIC", 1), ("CLOCK_PROCESS_CPUTIME_ID", 2), ("CLOCK_THREAD_CPUTIME_ID", 3)] {
        const_set(p, intern(n.as_bytes()), Value::fix(v));
    }
    let st = define_class_under(p, "Status", vm.c_object);
    define_cfunc(st, "exitstatus", st_exitstatus, 0);
    define_cfunc(st, "to_i", st_exitstatus, 0);
    define_cfunc(st, "success?", st_success, 0);
    let tms = define_class_under(p, "Tms", vm.c_object);
    define_cfunc(tms, "utime", |_c, s, _a, _b| Ok(ivar_get(s, intern(b"@utime"))), 0);
    define_cfunc(tms, "stime", |_c, s, _a, _b| Ok(ivar_get(s, intern(b"@stime"))), 0);
    let _ = mono();
    let _ = clock as usize;
}

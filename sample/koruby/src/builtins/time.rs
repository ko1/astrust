// Time (local time via libc).
use crate::array::*;
use crate::string::*;
use crate::symbol::intern;
use crate::value::*;
use crate::vm::*;

pub struct RTime { pub sec: i64, pub nsec: i64, pub utc: bool, pub off: i64 }
impl RData for RTime {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}
#[repr(C)]
#[derive(Default)]
struct Tm { sec: i32, min: i32, hour: i32, mday: i32, mon: i32, year: i32, wday: i32, yday: i32, isdst: i32, gmtoff: i64, zone: usize }
extern "C" { fn localtime_r(t: *const i64, tm: *mut Tm) -> *mut Tm; fn gmtime_r(t: *const i64, tm: *mut Tm) -> *mut Tm; fn timegm(tm: *mut Tm) -> i64; fn mktime(tm: *mut Tm) -> i64; }

fn rt(v: Value) -> &'static mut RTime { rdata::<RTime>(v).unwrap() }
fn tm_of(t: &RTime) -> Tm {
    let mut tm = Tm::default();
    unsafe { if t.utc { gmtime_r(&t.sec, &mut tm); } else { let s2 = t.sec + t.off - local_off(t.sec); localtime_r(&s2, &mut tm); tm.gmtoff = t.off; } }
    tm
}
fn local_off(sec: i64) -> i64 { let mut tm = Tm::default(); unsafe { localtime_r(&sec, &mut tm); } tm.gmtoff }
fn new_time(klass: Value, sec: i64, nsec: i64, utc: bool) -> Value {
    let off = if utc { 0 } else { local_off(sec) };
    crate::gc::alloc(klass, Body::Data(Box::new(RTime { sec, nsec, utc, off })))
}
pub fn time_at(_c: &mut Ctx, f: f64) -> RV { Ok(new_time(vm().c_time, f.floor() as i64, ((f - f.floor()) * 1e9).round() as i64, false)) }

fn t_now(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let d = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap();
    Ok(new_time(slf, d.as_secs() as i64, d.subsec_nanos() as i64, false))
}
fn t_at(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let f = crate::builtins::numeric_methods::to_f(c, a[0])?;
    let mut t = new_time(slf, f.floor() as i64, ((f - f.floor()) * 1e9).round() as i64, false);
    if a.len() > 1 && a[1].is_fixnum() { let unit = a.get(2).copied(); let mul = match unit.map(|u| crate::symbol::sym_str(u.as_sym())) { Some(ref u) if u == "millisecond" => 1_000_000, Some(ref u) if u == "nanosecond" || u == "nsec" => 1, _ => 1000 }; rt(t).nsec += a[1].as_fix() * mul; }
    if let Some(h) = crate::vm::kw_arg(a) { if let Some(z) = crate::hash::hash_get_opt(c, h, sym_value("in"))? { if is_string(z) && (str_bytes(z) == b"UTC" || str_bytes(z) == b"Z") { rt(t).utc = true; rt(t).off = 0; } } }
    let _ = &mut t;
    Ok(t)
}
fn mk(c: &mut Ctx, slf: Value, a: &[Value], utc: bool) -> RV {
    let mut tm = Tm::default();
    let g = |c: &mut Ctx, i: usize, d: i64| -> Result<i64, Esc> { match a.get(i) { Some(&v) if v != NIL => { if is_string(v) { Ok(String::from_utf8_lossy(str_bytes(v)).parse().unwrap_or(d)) } else { crate::builtins::numeric_methods::num_to_long(c, v) } } _ => Ok(d) } };
    tm.year = (g(c, 0, 1970)? - 1900) as i32; tm.mon = (g(c, 1, 1)? - 1) as i32; tm.mday = g(c, 2, 1)? as i32;
    tm.hour = g(c, 3, 0)? as i32; tm.min = g(c, 4, 0)? as i32;
    let secf = match a.get(5) { Some(&v) if v != NIL => crate::builtins::numeric_methods::to_f(c, v)?, _ => 0.0 };
    tm.sec = secf.floor() as i32; tm.isdst = -1;
    let sec = unsafe { if utc { timegm(&mut tm) } else { mktime(&mut tm) } };
    Ok(new_time(slf, sec, ((secf - secf.floor()) * 1e9).round() as i64, utc))
}
fn t_local(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV { mk(c, slf, a, false) }
fn t_utc_s(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV { mk(c, slf, a, true) }
fn t_to_f(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { let t = rt(slf); Ok(crate::numeric::float_new(t.sec as f64 + t.nsec as f64 * 1e-9)) }
fn t_to_i(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::fix(rt(slf).sec)) }
fn t_usec(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::fix(rt(slf).nsec / 1000)) }
fn t_nsec(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::fix(rt(slf).nsec)) }
fn t_minus(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let t = rt(slf);
    if let Some(o) = rdata::<RTime>(a[0]) { return Ok(crate::numeric::float_new((t.sec - o.sec) as f64 + (t.nsec - o.nsec) as f64 * 1e-9)); }
    let d = crate::builtins::numeric_methods::to_f(c, a[0])?;
    let total = t.sec as f64 + t.nsec as f64 * 1e-9 - d;
    let n = new_time(real_class_of(slf), total.floor() as i64, ((total - total.floor()) * 1e9).round() as i64, t.utc);
    Ok(n)
}
fn t_plus(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let t = rt(slf);
    let d = crate::builtins::numeric_methods::to_f(c, a[0])?;
    let total = t.sec as f64 + t.nsec as f64 * 1e-9 + d;
    Ok(new_time(real_class_of(slf), total.floor() as i64, ((total - total.floor()) * 1e9).round() as i64, t.utc))
}
fn t_cmp(_c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let Some(o) = rdata::<RTime>(a[0]) else { return Ok(NIL) };
    let t = rt(slf);
    Ok(Value::fix(((t.sec, t.nsec).cmp(&(o.sec, o.nsec))) as i64))
}
fn t_eq(c: &mut Ctx, slf: Value, a: &[Value], b: *const Block) -> RV { let r = t_cmp(c, slf, a, b)?; Ok(Value::bool(r == Value::fix(0))) }
macro_rules! tf { ($n:ident, $e:expr) => { fn $n(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { let tm = tm_of(rt(slf)); Ok(Value::fix($e(&tm) as i64)) } }; }
tf!(t_year, |t: &Tm| t.year + 1900); tf!(t_month, |t: &Tm| t.mon + 1); tf!(t_day, |t: &Tm| t.mday); tf!(t_hour, |t: &Tm| t.hour);
tf!(t_min, |t: &Tm| t.min); tf!(t_sec, |t: &Tm| t.sec); tf!(t_wday, |t: &Tm| t.wday); tf!(t_yday, |t: &Tm| t.yday + 1);
fn t_utc_offset(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::fix(rt(slf).off)) }
fn t_utc(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { let t = rt(slf); t.utc = true; t.off = 0; Ok(slf) }
fn t_getutc(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { let t = rt(slf); Ok(new_time(real_class_of(slf), t.sec, t.nsec, true)) }
fn t_localtime(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { let t = rt(slf); t.utc = false; t.off = local_off(t.sec); Ok(slf) }
fn t_utc_p(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::bool(rt(slf).utc)) }
const MONTHS: [&str; 12] = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
const DAYS: [&str; 7] = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];
pub fn strftime(t: &RTime, fmt: &[u8]) -> String {
    let tm = tm_of(t);
    let mut out = String::new();
    let mut i = 0;
    while i < fmt.len() {
        if fmt[i] != b'%' { out.push(fmt[i] as char); i += 1; continue; }
        i += 1;
        let mut flag = None; let mut width = None;
        while i < fmt.len() && matches!(fmt[i], b'-' | b'0' | b'^' | b'_') { flag = Some(fmt[i]); i += 1; }
        let st = i; while i < fmt.len() && fmt[i].is_ascii_digit() { i += 1; }
        if i > st { width = std::str::from_utf8(&fmt[st..i]).unwrap().parse::<usize>().ok(); }
        if i >= fmt.len() { break; }
        let ch = fmt[i]; i += 1;
        let num = |v: i64, w: usize| -> String { match flag { Some(b'-') => v.to_string(), Some(b'_') => format!("{:>w$}", v, w = width.unwrap_or(w)), _ => format!("{:0w$}", v, w = width.unwrap_or(w)) } };
        let s2 = match ch {
            b'Y' => num((tm.year + 1900) as i64, 1), b'C' => num(((tm.year + 1900) / 100) as i64, 2), b'y' => num(((tm.year + 1900) % 100) as i64, 2),
            b'm' => num((tm.mon + 1) as i64, 2), b'd' => num(tm.mday as i64, 2), b'e' => format!("{:>2}", tm.mday), b'j' => num((tm.yday + 1) as i64, 3),
            b'H' => num(tm.hour as i64, 2), b'k' => format!("{:>2}", tm.hour), b'I' => num((if tm.hour % 12 == 0 { 12 } else { tm.hour % 12 }) as i64, 2),
            b'l' => format!("{:>2}", if tm.hour % 12 == 0 { 12 } else { tm.hour % 12 }), b'M' => num(tm.min as i64, 2), b'S' => num(tm.sec as i64, 2),
            b'L' => format!("{:03}", t.nsec / 1_000_000), b'N' => { let w = width.unwrap_or(9); let s3 = format!("{:09}", t.nsec); s3[..w.min(9)].to_string() }
            b'p' => if tm.hour < 12 { "AM".into() } else { "PM".into() }, b'P' => if tm.hour < 12 { "am".into() } else { "pm".into() },
            b'A' => DAYS[tm.wday as usize].into(), b'a' => DAYS[tm.wday as usize][..3].into(), b'B' => MONTHS[tm.mon as usize].into(),
            b'b' | b'h' => MONTHS[tm.mon as usize][..3].into(), b'u' => (if tm.wday == 0 { 7 } else { tm.wday }).to_string(), b'w' => tm.wday.to_string(),
            b'z' => { let o = t.off; format!("{}{:02}{:02}", if o < 0 { '-' } else { '+' }, o.abs() / 3600, o.abs() % 3600 / 60) }
            b'Z' => if t.utc { "UTC".into() } else { zone_name() }, b's' => t.sec.to_string(), b'%' => "%".into(),
            b'F' => format!("{:04}-{:02}-{:02}", tm.year + 1900, tm.mon + 1, tm.mday), b'T' | b'X' => format!("{:02}:{:02}:{:02}", tm.hour, tm.min, tm.sec),
            b'D' | b'x' => format!("{:02}/{:02}/{:02}", tm.mon + 1, tm.mday, (tm.year + 1900) % 100), b'R' => format!("{:02}:{:02}", tm.hour, tm.min),
            b'c' => format!("{} {} {:>2} {:02}:{:02}:{:02} {}", &DAYS[tm.wday as usize][..3], &MONTHS[tm.mon as usize][..3], tm.mday, tm.hour, tm.min, tm.sec, tm.year + 1900),
            b'n' => "\n".into(), b't' => "\t".into(),
            x => format!("%{}", x as char),
        };
        out.push_str(&if flag == Some(b'^') { s2.to_uppercase() } else { s2 });
    }
    out
}
fn zone_name() -> String { std::env::var("TZ").ok().filter(|z| !z.is_empty() && z.len() <= 5).unwrap_or_else(|| "JST".into()) }
fn t_strftime(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV { let f = expect_str(c, a[0])?; Ok(str_new(strftime(rt(slf), str_bytes(f)).as_bytes())) }
fn t_to_s(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { let t = rt(slf); Ok(str_new(strftime(t, if t.utc { b"%Y-%m-%d %H:%M:%S UTC" } else { b"%Y-%m-%d %H:%M:%S %z" }).as_bytes())) }
fn t_inspect(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let t = rt(slf);
    let frac = if t.nsec != 0 { let s2 = format!("{:09}", t.nsec); format!(".{}", s2.trim_end_matches('0')) } else { String::new() };
    let base = strftime(t, b"%Y-%m-%d %H:%M:%S");
    Ok(str_new(format!("{}{} {}", base, frac, if t.utc { "UTC".into() } else { strftime(t, b"%z") }).as_bytes()))
}
fn t_to_a(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let tm = tm_of(rt(slf));
    Ok(ary_from_vec([tm.sec, tm.min, tm.hour, tm.mday, tm.mon + 1, tm.year + 1900, tm.wday, tm.yday + 1].iter().map(|&x| Value::fix(x as i64)).chain([FALSE, str_new(b"UTC")]).collect()))
}
fn t_hash(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { let t = rt(slf); Ok(Value::fix(((t.sec as u64 ^ t.nsec as u64) >> 1) as i64)) }

pub fn dup_data(_c: &mut Ctx, v: Value) -> Result<Option<Value>, Esc> {
    if let Some(t) = rdata::<RTime>(v) { return Ok(Some(crate::gc::alloc(obj(v).klass, Body::Data(Box::new(RTime { sec: t.sec, nsec: t.nsec, utc: t.utc, off: t.off }))))); }
    Ok(None)
}
pub fn alloc_data(_c: &mut Ctx, klass: Value) -> Result<Option<Value>, Esc> {
    if crate::builtins::inherits(klass, vm().c_time) { return Ok(Some(new_time(klass, 0, 0, false))); }
    Ok(None)
}

pub fn init() {
    let vm = vm();
    let k = define_class("Time", vm.c_object);
    rclass(k).alloc = AllocKind::None;
    include_module(k, vm.m_comparable);
    vm.c_time = k;
    define_singleton_cfunc(k, "now", t_now, -1);
    define_singleton_cfunc(k, "at", t_at, -1);
    define_singleton_cfunc(k, "local", t_local, -1);
    define_singleton_cfunc(k, "mktime", t_local, -1);
    define_singleton_cfunc(k, "new", |c, s2, a, b| if a.is_empty() { t_now(c, s2, a, b) } else { t_local(c, s2, a, b) }, -1);
    define_singleton_cfunc(k, "utc", t_utc_s, -1);
    define_singleton_cfunc(k, "gm", t_utc_s, -1);
    let d = |n: &str, f: CFunc, a: i32| define_cfunc(k, n, f, a);
    d("to_f", t_to_f, 0); d("to_i", t_to_i, 0); d("tv_sec", t_to_i, 0); d("usec", t_usec, 0); d("nsec", t_nsec, 0); d("-", t_minus, 1); d("+", t_plus, 1);
    d("<=>", t_cmp, 1); d("==", t_eq, 1); d("eql?", t_eq, 1); d("year", t_year, 0); d("month", t_month, 0); d("mon", t_month, 0); d("day", t_day, 0);
    d("mday", t_day, 0); d("hour", t_hour, 0); d("min", t_min, 0); d("sec", t_sec, 0); d("wday", t_wday, 0); d("yday", t_yday, 0);
    d("utc_offset", t_utc_offset, 0); d("gmt_offset", t_utc_offset, 0); d("utc", t_utc, 0); d("gmtime", t_utc, 0); d("getutc", t_getutc, 0);
    d("localtime", t_localtime, -1); d("utc?", t_utc_p, 0); d("strftime", t_strftime, 1); d("to_s", t_to_s, 0); d("inspect", t_inspect, 0);
    d("to_a", t_to_a, 0); d("hash", t_hash, 0);
    let _ = intern;
}

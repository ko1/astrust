// Time: exact nanoseconds since the epoch (Integer or Rational), plus a zone mode.
use crate::array::*;
use crate::bignum::*;
use crate::builtins::kernel::*;
use crate::builtins::rational::*;
use crate::numeric::*;
use crate::string::*;
use crate::symbol::{intern, s};
use crate::value::*;
use crate::vm::*;
use std::cmp::Ordering;

pub const LOCAL: u8 = 0;
pub const UTC: u8 = 1;
pub const FIXED: u8 = 2;

pub struct RTime { pub w: Value, pub mode: u8, pub off: Value }
impl RData for RTime {
    fn mark(&self, m: &mut crate::gc::Marker) { m.mark(self.w); m.mark(self.off); }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

#[repr(C)]
struct Tm { sec: i32, min: i32, hour: i32, mday: i32, mon: i32, year: i32, wday: i32, yday: i32, isdst: i32, gmtoff: i64, zone: *const std::os::raw::c_char }
impl Default for Tm { fn default() -> Tm { Tm { sec: 0, min: 0, hour: 0, mday: 0, mon: 0, year: 0, wday: 0, yday: 0, isdst: 0, gmtoff: 0, zone: std::ptr::null() } } }
extern "C" { fn localtime_r(t: *const i64, tm: *mut Tm) -> *mut Tm; fn mktime(tm: *mut Tm) -> i64; fn tzset(); }

const NS: i64 = 1_000_000_000;
fn ns() -> Value { Value::fix(NS) }
fn rt(v: Value) -> &'static mut RTime { rdata::<RTime>(v).unwrap() }
fn is_time(v: Value) -> bool { v.is_heap() && rdata::<RTime>(v).is_some() }
fn new_time(klass: Value, w: Value, mode: u8, off: Value) -> Value {
    crate::gc::alloc(klass, Body::Data(Box::new(RTime { w, mode, off })))
}
pub fn time_at(c: &mut Ctx, f: f64) -> RV { let w = ex_mul(num_exact(c, float_new(f))?, ns()); Ok(new_time(vm().c_time, w, LOCAL, NIL)) }

// ---- calendar -------------------------------------------------------------------------------

fn days_from_civil(y: i128, m: i128, d: i128) -> i128 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}
fn civil_from_days(z: i128) -> (i128, i128, i128) {
    let z = z + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}
fn leap(y: i128) -> bool { y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) }

struct Vtm { year: i128, mon: i64, mday: i64, hour: i64, min: i64, sec: i64, wday: i64, yday: i64, isdst: bool, off: Value, zone: Option<String> }

const CYCLE: i128 = 12622780800; // seconds in 400 Gregorian years
/// (gmtoff, isdst, abbreviation) of the system zone at UTC second t.
fn local_info(t: i128) -> (i64, bool, String) {
    let mut s2 = t;
    if !(-(1i128 << 40)..(1i128 << 40)).contains(&t) { s2 = 946684800 + (t - 946684800).rem_euclid(CYCLE); }
    let s64 = s2 as i64;
    let mut tm = Tm::default();
    unsafe { tzset(); localtime_r(&s64, &mut tm); }
    let z = if tm.zone.is_null() { String::new() } else { unsafe { std::ffi::CStr::from_ptr(tm.zone) }.to_string_lossy().into_owned() };
    (tm.gmtoff, tm.isdst > 0, z)
}
fn secs_floor(w: Value) -> i128 {
    let f = ex_floor(ex_div(w, ns()));
    if f.is_fixnum() { f.as_fix() as i128 } else { to_big(f).to_string_radix(10).parse::<i128>().unwrap_or(if isneg(f) { i128::MIN / 4 } else { i128::MAX / 4 }) }
}
fn int_of(v: i128) -> Value { int_from_i128(v) }
fn i128_of(v: Value) -> i128 { if v.is_fixnum() { v.as_fix() as i128 } else { to_big(v).to_string_radix(10).parse::<i128>().unwrap_or(0) } }

fn vtm_of(t: &RTime) -> Vtm {
    let secs = ex_div(t.w, ns());
    let (off, isdst, zone) = match t.mode {
        UTC => (Value::fix(0), false, Some("UTC".to_string())),
        FIXED => (t.off, false, None),
        _ => { let (o, d, z) = local_info(secs_floor(t.w)); (Value::fix(o), d, Some(z)) }
    };
    let local = ex_floor(ex_add(secs, off));
    let s = i128_of(local);
    let days = s.div_euclid(86400); let rem = s.rem_euclid(86400);
    let (y, m, d) = civil_from_days(days);
    let yday = days - days_from_civil(y, 1, 1) + 1;
    Vtm { year: y, mon: m as i64, mday: d as i64, hour: (rem / 3600) as i64, min: (rem / 60 % 60) as i64, sec: (rem % 60) as i64,
          wday: (days + 4).rem_euclid(7) as i64, yday: yday as i64, isdst, off, zone }
}
fn timegm_fields(year: i128, mon: i64, mday: i64, hour: i64, min: i64, sec: i64) -> i128 {
    days_from_civil(year, mon as i128, mday as i128) * 86400 + hour as i128 * 3600 + min as i128 * 60 + sec as i128
}
/// local wall-clock seconds (as if UTC) -> UTC seconds
fn local_to_utc(l: i128, isdst: Option<bool>) -> i128 {
    if let Some(dst) = isdst {
        // let the C library apply an explicit isdst
        let shift = if !(-(1i128 << 40)..(1i128 << 40)).contains(&l) { (l - 946684800).div_euclid(CYCLE) * CYCLE } else { 0 };
        let l2 = l - shift;
        let days = l2.div_euclid(86400); let rem = l2.rem_euclid(86400);
        let (y, m, d) = civil_from_days(days);
        let mut tm = Tm { sec: (rem % 60) as i32, min: (rem / 60 % 60) as i32, hour: (rem / 3600) as i32, mday: d as i32, mon: m as i32 - 1, year: (y - 1900) as i32, isdst: dst as i32, ..Tm::default() };
        let r = unsafe { tzset(); mktime(&mut tm) };
        if r != -1 || (tm.year == 69) { return r as i128 + shift; }
    }
    let a = local_info(l - 86400).0 as i128;
    let b = local_info(l + 86400).0 as i128;
    if a == b { return l - a; }
    let ta = l - a; let tb = l - b;
    let va = local_info(ta).0 as i128 == a; let vb = local_info(tb).0 as i128 == b;
    match (va, vb) { (true, true) => ta.max(tb), (true, false) => ta, (false, true) => tb, _ => ta }
}

// ---- conversions ----------------------------------------------------------------------------

/// CRuby's num_exact: Integer/Rational (canonical), Float exactly, else to_r / to_int.
fn num_exact(c: &mut Ctx, v: Value) -> RV {
    if is_integer(v) { return Ok(v); }
    if is_rational(v) { return Ok(ex_canon(v)); }
    if is_float(v) { let r = float_to_r(c, float_val(v))?; return Ok(ex_canon(r)); }
    if v != NIL && !is_string(v) && v.is_heap() {
        if respond_to(v, intern(b"to_r"), true) {
            if respond_to(v, s::to_int, true) {
                let r = funcall(c, v, intern(b"to_r"), &[], std::ptr::null())?;
                if is_integer(r) { return Ok(r); }
                if is_rational(r) { return Ok(ex_canon(r)); }
            }
        } else if respond_to(v, s::to_int, true) {
            let r = funcall(c, v, s::to_int, &[], std::ptr::null())?;
            if is_integer(r) { return Ok(r); }
        }
    }
    let cn = class_name_for_err(v);
    type_err(c, format!("can't convert {} into an exact number", cn))
}
fn str_to_inum(c: &mut Ctx, v: Value) -> RV { funcall(c, vm().main_obj, intern(b"Integer"), &[v, Value::fix(10)], std::ptr::null()) }
fn obj2int(c: &mut Ctx, v: Value) -> Result<i64, Esc> {
    let v = if is_string(v) { str_to_inum(c, v)? } else { v };
    let i = crate::builtins::numeric_methods::num_to_long(c, v)?;
    if i < i32::MIN as i64 || i > i32::MAX as i64 { return err(c, vm().c_range_error, format!("integer {} too {} to convert to 'int'", i, if i < 0 { "small" } else { "big" })); }
    Ok(i)
}
fn obj2ubits(c: &mut Ctx, v: Value, bits: u32) -> Result<i64, Esc> {
    let i = obj2int(c, v)? as i32 as u32;
    if i & ((1u32 << bits) - 1) != i { return arg_err(c, "argument out of range"); }
    Ok(i as i64)
}
fn obj2vint(c: &mut Ctx, v: Value) -> RV { if is_string(v) { str_to_inum(c, v) } else { crate::builtins::numeric_methods::to_integer(c, v) } }
/// sec field and exact subsecond (fraction of a second)
fn obj2subsec(c: &mut Ctx, v: Value) -> Result<(i64, Value), Esc> {
    if is_string(v) { let i = str_to_inum(c, v)?; return Ok((obj2ubits(c, i, 6)?, Value::fix(0))); }
    let x = num_exact(c, v)?;
    let f = ex_floor(x);
    let sub = ex_sub(x, f);
    Ok((obj2ubits(c, f, 6)?, sub))
}
const MONTHS: [&str; 12] = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
const DAYS: [&str; 7] = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];
fn month_arg(c: &mut Ctx, v: Value) -> Result<i64, Esc> {
    if v.is_fixnum() { return obj2ubits(c, v, 4); }
    let sv = if is_string(v) { Some(v) } else if v.is_heap() && respond_to(v, s::to_str, true) { Some(funcall(c, v, s::to_str, &[], std::ptr::null())?) } else { None };
    if let Some(sv) = sv { if is_string(sv) {
        let b = str_bytes(sv);
        if b.len() == 3 { for (i, m) in MONTHS.iter().enumerate() { if m.as_bytes()[..3].eq_ignore_ascii_case(b) { return Ok(i as i64 + 1); } } }
        return obj2ubits(c, sv, 4);
    } }
    obj2ubits(c, v, 4)
}

struct Args { year: i128, mon: i64, mday: i64, hour: i64, min: i64, sec: i64, sub: Value, isdst: Option<bool> }
fn validate(c: &mut Ctx, a: &Args) -> Result<(), Esc> {
    if a.mon < 1 || a.mon > 12 { return arg_err(c, "mon out of range"); }
    if a.mday < 1 || a.mday > 31 { return arg_err(c, "mday out of range"); }
    if a.hour < 0 || a.hour > 24 { return arg_err(c, "hour out of range"); }
    if a.min < 0 || a.min > if a.hour == 24 { 0 } else { 59 } { return arg_err(c, "min out of range"); }
    if a.sec < 0 || a.sec > if a.hour == 24 { 0 } else { 60 } { return arg_err(c, "sec out of range"); }
    if ex_cmp(a.sub, Value::fix(0)) == Ordering::Less || ex_cmp(a.sub, Value::fix(1)) != Ordering::Less { return arg_err(c, "subsecx out of range"); }
    Ok(())
}
fn normalize_mday(a: &mut Args) {
    match a.mon {
        2 => { let m2 = if leap(a.year) { 29 } else { 28 }; if a.mday > m2 { a.mday -= m2; a.mon += 1; } }
        4 | 6 | 9 | 11 => if a.mday == 31 { a.mon += 1; a.mday = 1; },
        _ => {}
    }
}
fn year_i128(c: &mut Ctx, y: Value) -> Result<i128, Esc> {
    let y = obj2vint(c, y)?;
    if y.is_fixnum() { return Ok(y.as_fix() as i128); }
    match to_big(y).to_string_radix(10).parse::<i128>() { Ok(v) if v.abs() < (1i128 << 100) => Ok(v), _ => err(c, vm().c_range_error, "year too big") }
}
/// Time.local / Time.utc argument forms
fn time_arg(c: &mut Ctx, argv: &[Value]) -> Result<Args, Esc> {
    let mut v = [NIL; 8];
    let mut isdst = None;
    if argv.len() == 10 {
        v[0] = argv[5]; v[1] = argv[4]; v[2] = argv[3]; v[3] = argv[2]; v[4] = argv[1]; v[5] = argv[0];
        isdst = Some(argv[8].truthy());
    } else {
        if argv.is_empty() || argv.len() > 8 { return arg_err(c, format!("wrong number of arguments (given {}, expected 1..8)", argv.len())); }
        for (i, x) in argv.iter().enumerate() { v[i] = *x; }
    }
    let year = year_i128(c, v[0])?;
    let mon = if v[1] == NIL { 1 } else { month_arg(c, v[1])? };
    let mday = if v[2] == NIL { 1 } else { obj2ubits(c, v[2], 5)? };
    let mut a = Args { year, mon, mday, hour: 0, min: 0, sec: 0, sub: Value::fix(0), isdst };
    normalize_mday(&mut a);
    a.hour = if v[3] == NIL { 0 } else { obj2ubits(c, v[3], 5)? };
    a.min = if v[4] == NIL { 0 } else { obj2ubits(c, v[4], 6)? };
    if v[6] != NIL && argv.len() == 7 {
        a.sec = if v[5] == NIL { 0 } else { obj2ubits(c, v[5], 6)? };
        let u = if is_string(v[6]) { str_to_inum(c, v[6])? } else { num_exact(c, v[6])? };
        a.sub = ex_div(u, Value::fix(1_000_000));
    } else if v[5] != NIL {
        let (s2, sub) = obj2subsec(c, v[5])?; a.sec = s2; a.sub = sub;
    }
    validate(c, &a)?;
    Ok(a)
}
fn args_to_w(a: &Args, off: Option<Value>) -> Value {
    let l = timegm_fields(a.year, a.mon, a.mday, a.hour, a.min, a.sec);
    let secs = match off { Some(o) => ex_sub(int_of(l), o), None => int_of(local_to_utc(l, a.isdst)) };
    ex_mul(ex_add(secs, a.sub), ns())
}
fn t_mk(c: &mut Ctx, slf: Value, a: &[Value], utc: bool) -> RV {
    let args = time_arg(c, a)?;
    if utc { Ok(new_time(slf, args_to_w(&args, Some(Value::fix(0))), UTC, NIL)) }
    else { Ok(new_time(slf, args_to_w(&args, None), LOCAL, NIL)) }
}
fn t_local(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV { t_mk(c, slf, a, false) }
fn t_utc_s(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV { t_mk(c, slf, a, true) }

enum Off { Utc, Fixed(Value) }
/// CRuby's utc_offset_arg: None when not an offset spec.
fn utc_offset_arg(c: &mut Ctx, v: Value) -> Result<Option<Off>, Esc> {
    if is_integer(v) { return Ok(Some(Off::Fixed(v))); }
    if is_string(v) || (v.is_heap() && !is_kind_of(v, vm().c_numeric) && respond_to(v, s::to_str, true)) {
        let sv = if is_string(v) { v } else { funcall(c, v, s::to_str, &[], std::ptr::null())? };
        let b = str_bytes(sv).to_vec();
        let d2 = |i: usize| -> Option<i64> { if i + 1 < b.len() && b[i].is_ascii_digit() && b[i + 1].is_ascii_digit() { Some(((b[i] - b'0') * 10 + (b[i + 1] - b'0')) as i64) } else { None } };
        let (mut min, mut sec) = (None, None);
        match b.len() {
            1 => {
                let ch = b[0];
                if ch == b'Z' { return Ok(Some(Off::Utc)); }
                let n = if (b'A'..=b'I').contains(&ch) { (ch - b'A' + 1) as i64 } else if (b'K'..=b'M').contains(&ch) { (ch - b'A') as i64 } else if (b'N'..=b'Y').contains(&ch) { b'M' as i64 - ch as i64 } else { return Ok(None) };
                return Ok(Some(Off::Fixed(Value::fix(n * 3600))));
            }
            3 => { if b.eq_ignore_ascii_case(b"UTC") { return Ok(Some(Off::Utc)); } }
            7 => { sec = Some(5); min = Some(3); }
            5 => { min = Some(3); }
            9 => { if b[6] != b':' || b[3] != b':' { return Ok(None); } sec = Some(7); min = Some(4); }
            6 => { if b[3] != b':' { return Ok(None); } min = Some(4); }
            _ => return Ok(None),
        }
        let mut n = 0;
        if let Some(si) = sec { match d2(si) { Some(x) if b[si] <= b'5' => n += x, _ => return Ok(None) } }
        if let Some(mi) = min { match d2(mi) { Some(x) if b[mi] <= b'5' => n += x * 60, _ => return Ok(None) } }
        if b[0] != b'+' && b[0] != b'-' { return Ok(None); }
        match d2(1) { Some(x) => n += x * 3600, None => return Ok(None) }
        if b[0] == b'-' { if n == 0 { return Ok(Some(Off::Utc)); } n = -n; }
        return Ok(Some(Off::Fixed(Value::fix(n))));
    }
    if v == NIL || v.is_sym() { return Ok(None); }
    Ok(Some(Off::Fixed(num_exact(c, v)?)))
}
fn invalid_utc_offset<T>(c: &mut Ctx, v: Value) -> Result<T, Esc> {
    let s2 = if is_string(v) { inspect_str(c, v)? } else { inspect_str(c, v)? };
    arg_err(c, format!("\"+HH:MM\", \"-HH:MM\", \"UTC\" or \"A\"..\"I\",\"K\"..\"Z\" expected for utc_offset: {}", s2.trim_matches('"')))
}
fn validate_off(c: &mut Ctx, o: Value) -> Result<(), Esc> {
    if ex_cmp(o, Value::fix(-86400)) != Ordering::Greater || ex_cmp(o, Value::fix(86400)) != Ordering::Less { return arg_err(c, "utc_offset out of range"); }
    Ok(())
}
/// apply an `in:` / zone argument to an existing time
fn zonelocal(c: &mut Ctx, t: Value, zone: Value) -> Result<(), Esc> {
    match utc_offset_arg(c, zone)? {
        Some(Off::Utc) => { rt(t).mode = UTC; rt(t).off = NIL; }
        Some(Off::Fixed(o)) => { validate_off(c, o)?; rt(t).mode = FIXED; rt(t).off = o; }
        None => return invalid_utc_offset(c, zone),
    }
    Ok(())
}

fn t_now(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let d = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap();
    let w = int_from_i128(d.as_secs() as i128 * NS as i128 + d.subsec_nanos() as i128);
    let t = new_time(slf, w, LOCAL, NIL);
    if let Some(h) = kw_arg(a) { if let Some(z) = crate::hash::hash_get_opt(c, h, sym_value("in"))? { if z != NIL { zonelocal(c, t, z)?; } } }
    Ok(t)
}
fn get_scale(c: &mut Ctx, unit: Value) -> Result<i64, Esc> {
    if unit.is_sym() {
        match crate::symbol::sym_str(unit.as_sym()).as_str() {
            "millisecond" => return Ok(1000), "usec" | "microsecond" => return Ok(1_000_000), "nsec" | "nanosecond" => return Ok(NS), _ => {}
        }
    }
    let ins = inspect_str(c, unit)?;
    arg_err(c, format!("unexpected unit: {}", ins))
}
fn t_at(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let kw = kw_arg(a);
    let a = if kw.is_some() { &a[..a.len() - 1] } else { a };
    check_argc(c, a, 1, 3)?;
    let t = if a.len() >= 2 {
        let scale = get_scale(c, a.get(2).copied().unwrap_or(sym_value("microsecond")))?;
        let x = num_exact(c, a[0])?; let sub = num_exact(c, a[1])?;
        let w = ex_add(ex_mul(x, ns()), ex_div(ex_mul(sub, ns()), Value::fix(scale)));
        new_time(slf, w, LOCAL, NIL)
    } else if is_time(a[0]) {
        let o = rt(a[0]); new_time(slf, o.w, o.mode, o.off)
    } else {
        let x = num_exact(c, a[0])?;
        new_time(slf, ex_mul(x, ns()), LOCAL, NIL)
    };
    if let Some(h) = kw { if let Some(z) = crate::hash::hash_get_opt(c, h, sym_value("in"))? { if z != NIL { zonelocal(c, t, z)?; } } }
    Ok(t)
}

fn init_vtm(c: &mut Ctx, slf: Value, mut a: Args, zone: Value) -> RV {
    a.isdst = None;
    let mut off = None; let mut utc = false;
    if zone != NIL {
        if zone == sym_value("dst") { a.isdst = Some(true); }
        else if zone == sym_value("std") { a.isdst = Some(false); }
        else {
            match utc_offset_arg(c, zone)? {
                Some(Off::Utc) => utc = true,
                Some(Off::Fixed(o)) => off = Some(o),
                None => return invalid_utc_offset(c, zone),
            }
        }
    }
    validate(c, &a)?;
    if let Some(o) = off { validate_off(c, o)?; }
    let t = rt(slf);
    if utc { t.w = args_to_w(&a, Some(Value::fix(0))); t.mode = UTC; t.off = NIL; return Ok(slf); }
    if let Some(o) = off { t.w = args_to_w(&a, Some(o)); t.mode = FIXED; t.off = o; return Ok(slf); }
    t.w = args_to_w(&a, None); t.mode = LOCAL; t.off = NIL;
    Ok(slf)
}
fn parse_err<T>(c: &mut Ctx, s2: Value) -> Result<T, Esc> { let ins = inspect_str(c, s2)?; arg_err(c, format!("can't parse: {}", ins)) }
fn init_parse(c: &mut Ctx, slf: Value, sv: Value, zone: Value, prec: Option<usize>) -> RV {
    let b = str_bytes(sv).to_vec();
    let sp = |x: u8| matches!(x, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c);
    if !b.is_empty() && (sp(b[0]) || sp(b[b.len() - 1])) { return parse_err(c, sv); }
    let mut i = 0;
    let neg = if i < b.len() && (b[i] == b'-' || b[i] == b'+') { i += 1; b[0] == b'-' } else { false };
    let ds = i; while i < b.len() && b[i].is_ascii_digit() { i += 1; }
    if i == ds { return parse_err(c, sv); }
    let nd = i - ds;
    if nd < 4 { let t = String::from_utf8_lossy(&b[ds..i]).into_owned(); return arg_err(c, format!("year must be 4 or more digits: {}", t)); }
    let yv: i128 = String::from_utf8_lossy(&b[ds..i]).parse().unwrap_or(0);
    let year = if neg { -yv } else { yv };
    let (mut mon, mut mday, mut hour, mut min, mut sec) = (-1i64, -1i64, -1i64, -1i64, -1i64);
    let mut sub = NIL;
    let two = |c: &mut Ctx, i: &mut usize, name: &str| -> Result<i64, Esc> {
        let p = *i + 1; let len = b.len() - p;
        if len < 2 || !b[p].is_ascii_digit() || !b[p + 1].is_ascii_digit() || (len > 2 && b[p + 2].is_ascii_digit()) {
            let mut m = format!("two digits {} is expected", name);
            if b[p - 1] == b'-' || b[p - 1] == b':' { m.push_str(&format!(" after '{}'", b[p - 1] as char)); }
            let e = (p + len.min(10)).min(b.len());
            m.push_str(&format!(": {}", String::from_utf8_lossy(&b[p - 1..e])));
            return arg_err(c, m);
        }
        *i = p + 2;
        Ok(((b[p] - b'0') * 10 + (b[p + 1] - b'0')) as i64)
    };
    let peek = |i: usize, ch: u8| i < b.len() && b[i] == ch;
    if i < b.len() {
        loop {
            if !peek(i, b'-') { break; }
            mon = two(c, &mut i, "mon")?; if mon > 15 { return arg_err(c, "mon out of range"); }
            if !peek(i, b'-') { break; }
            mday = two(c, &mut i, "mday")?; if mday > 31 { return arg_err(c, "mday out of range"); }
            if !peek(i, b' ') && !peek(i, b'T') { break; }
            let tp = i + 1;
            if !(i + 1 < b.len() && b[i + 1].is_ascii_digit()) { break; }
            let part = |i: usize| String::from_utf8_lossy(&b[tp..(i + 1).min(b.len())]).into_owned();
            hour = two(c, &mut i, "hour")?; if hour > 31 { return arg_err(c, "hour out of range"); }
            if peek(i, b'.') { return arg_err(c, format!("fraction hour is not supported: {}", part(i))); }
            if !peek(i, b':') { return arg_err(c, format!("missing min part: {}", part(i))); }
            min = two(c, &mut i, "min")?; if min > 63 { return arg_err(c, "min out of range"); }
            if peek(i, b'.') { return arg_err(c, format!("fraction min is not supported: {}", part(i))); }
            if !peek(i, b':') { return arg_err(c, format!("missing sec part: {}", part(i))); }
            sec = two(c, &mut i, "sec")?; if sec > 63 { return arg_err(c, "sec out of range"); }
            if peek(i, b'.') {
                i += 1;
                let st = i; let mut n = 0;
                while i < b.len() && b[i].is_ascii_digit() && prec.map(|p| n < p).unwrap_or(true) { i += 1; n += 1; }
                if n == 0 { let e = (i + 1).min(b.len()); return arg_err(c, format!("subsecond expected after dot: {}", String::from_utf8_lossy(&b[tp..e]))); }
                let digits = String::from_utf8_lossy(&b[st..i]).into_owned();
                let v = big_to_value(BigInt::parse(&digits, 10).unwrap());
                sub = ex_div(v, ipow(Value::fix(10), n as u64));
                while i < b.len() && b[i].is_ascii_digit() { i += 1; }
            }
            break;
        }
        while i < b.len() && sp(b[i]) { i += 1; }
        let zs = i; while i < b.len() && !sp(b[i]) { i += 1; }
        let ze = i; while i < b.len() && sp(b[i]) { i += 1; }
        if i < b.len() { return arg_err(c, format!("can't parse at: {}", String::from_utf8_lossy(&b[i..]))); }
        let mut zone = zone;
        if ze > zs { zone = str_new(&b[zs..ze]); }
        else if hour == -1 { return arg_err(c, "no time information"); }
        let a = Args { year, mon: if mon < 0 { 1 } else { mon }, mday: if mday < 0 { 1 } else { mday }, hour: hour.max(0), min: min.max(0), sec: sec.max(0), sub: if sub == NIL { Value::fix(0) } else { sub }, isdst: None };
        return init_vtm(c, slf, a, zone);
    }
    let a = Args { year, mon: 1, mday: 1, hour: 0, min: 0, sec: 0, sub: Value::fix(0), isdst: None };
    init_vtm(c, slf, a, zone)
}
fn t_initialize(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let kw = kw_arg(a);
    let a = if kw.is_some() { &a[..a.len() - 1] } else { a };
    if a.len() > 7 { return arg_err(c, format!("wrong number of arguments (given {}, expected 0..7)", a.len())); }
    let mut zone = a.get(6).copied().unwrap_or(NIL);
    let mut prec = Some(9usize);
    if let Some(h) = kw {
        if let Some(z) = crate::hash::hash_get_opt(c, h, sym_value("in"))? {
            if zone != NIL { return arg_err(c, "timezone argument given as positional and keyword arguments"); }
            zone = z;
        }
        if let Some(p) = crate::hash::hash_get_opt(c, h, sym_value("precision"))? { prec = if p == NIL { None } else { Some(crate::builtins::numeric_methods::num_to_long(c, p)?.max(0) as usize) }; }
    }
    if a.is_empty() {
        let d = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap();
        let t = rt(slf);
        t.w = int_from_i128(d.as_secs() as i128 * NS as i128 + d.subsec_nanos() as i128); t.mode = LOCAL; t.off = NIL;
        if zone != NIL { zonelocal(c, slf, zone)?; }
        return Ok(slf);
    }
    if a.len() == 1 && (is_string(a[0]) || (a[0].is_heap() && !is_integer(a[0]) && respond_to(a[0], s::to_str, true))) {
        let sv = if is_string(a[0]) { a[0] } else { funcall(c, a[0], s::to_str, &[], std::ptr::null())? };
        return init_parse(c, slf, sv, zone, prec);
    }
    let g = |i: usize| a.get(i).copied().unwrap_or(NIL);
    let year = year_i128(c, g(0))?;
    let mon = if g(1) == NIL { 1 } else { month_arg(c, g(1))? };
    let mday = if g(2) == NIL { 1 } else { obj2ubits(c, g(2), 5)? };
    let hour = if g(3) == NIL { 0 } else { obj2ubits(c, g(3), 5)? };
    let min = if g(4) == NIL { 0 } else { obj2ubits(c, g(4), 6)? };
    let (sec, sub) = if g(5) == NIL { (0, Value::fix(0)) } else { obj2subsec(c, g(5))? };
    let mut args = Args { year, mon, mday, hour, min, sec, sub, isdst: None };
    normalize_mday(&mut args);
    init_vtm(c, slf, args, zone)
}

// ---- accessors ------------------------------------------------------------------------------

macro_rules! vf { ($n:ident, $e:expr) => { fn $n(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { let v = vtm_of(rt(slf)); Ok($e(&v)) } }; }
vf!(t_year, |v: &Vtm| int_of(v.year)); vf!(t_month, |v: &Vtm| Value::fix(v.mon)); vf!(t_day, |v: &Vtm| Value::fix(v.mday));
vf!(t_hour, |v: &Vtm| Value::fix(v.hour)); vf!(t_min, |v: &Vtm| Value::fix(v.min)); vf!(t_sec, |v: &Vtm| Value::fix(v.sec));
vf!(t_wday, |v: &Vtm| Value::fix(v.wday)); vf!(t_yday, |v: &Vtm| Value::fix(v.yday)); vf!(t_isdst, |v: &Vtm| Value::bool(v.isdst));
vf!(t_utc_offset, |v: &Vtm| v.off);
macro_rules! wd { ($n:ident, $d:expr) => { fn $n(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::bool(vtm_of(rt(slf)).wday == $d)) } }; }
wd!(t_sunday, 0); wd!(t_monday, 1); wd!(t_tuesday, 2); wd!(t_wednesday, 3); wd!(t_thursday, 4); wd!(t_friday, 5); wd!(t_saturday, 6);
fn t_zone(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let v = vtm_of(rt(slf));
    Ok(match v.zone { Some(z) => str_new_enc(z.as_bytes(), crate::object::ENC_ASCII), None => NIL })
}
fn t_to_i(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(ex_floor(ex_div(rt(slf).w, ns()))) }
fn w_to_f(w: Value) -> f64 { if is_integer(w) { int_to_f(w) / 1e9 } else { ex_to_f(ex_div(w, ns())) } }
fn t_to_f(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(float_new(w_to_f(rt(slf).w))) }
fn t_to_r(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let r = ex_div(rt(slf).w, ns());
    Ok(if is_integer(r) { rat_raw(r, Value::fix(1)) } else { r })
}
fn subns(t: &RTime) -> Value { ex_mod(t.w, ns()) }
fn t_usec(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(ex_floor(ex_div(subns(rt(slf)), Value::fix(1000)))) }
fn t_nsec(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(ex_floor(subns(rt(slf)))) }
fn t_subsec(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let r = ex_div(subns(rt(slf)), ns());
    Ok(r)
}
fn t_utc_p(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::bool(rt(slf).mode == UTC)) }

// ---- zone changes ---------------------------------------------------------------------------

fn modify(c: &mut Ctx, v: Value) -> Result<(), Esc> { check_frozen(c, v) }
fn t_utc(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    if rt(slf).mode != UTC { modify(c, slf)?; }
    let t = rt(slf); t.mode = UTC; t.off = NIL; Ok(slf)
}
fn t_getutc(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { let t = rt(slf); Ok(new_time(real_class_of(slf), t.w, UTC, NIL)) }
fn t_localtime(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    check_argc(c, a, 0, 1)?;
    if let Some(&z) = a.first() { if z != NIL {
        let off = utc_offset_arg(c, z)?;
        match off {
            Some(Off::Utc) => { modify(c, slf)?; let t = rt(slf); t.mode = UTC; t.off = NIL; }
            Some(Off::Fixed(o)) => { validate_off(c, o)?; modify(c, slf)?; let t = rt(slf); t.mode = FIXED; t.off = o; }
            None => return invalid_utc_offset(c, z),
        }
        return Ok(slf);
    } }
    if rt(slf).mode != LOCAL { modify(c, slf)?; }
    let t = rt(slf); t.mode = LOCAL; t.off = NIL; Ok(slf)
}
fn t_getlocal(c: &mut Ctx, slf: Value, a: &[Value], b: *const Block) -> RV {
    let t = rt(slf);
    let n = new_time(real_class_of(slf), t.w, t.mode, t.off);
    t_localtime(c, n, a, b)
}

// ---- arithmetic / comparison ----------------------------------------------------------------

fn time_add(c: &mut Ctx, slf: Value, o: Value, sign: i64) -> RV {
    let x = num_exact(c, o)?;
    let d = ex_mul(x, ns());
    let t = rt(slf);
    let w = if sign > 0 { ex_add(t.w, d) } else { ex_sub(t.w, d) };
    Ok(new_time(real_class_of(slf), w, t.mode, t.off))
}
fn t_plus(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    if is_time(a[0]) { return type_err(c, "time + time?"); }
    time_add(c, slf, a[0], 1)
}
fn t_minus(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    if is_time(a[0]) { return Ok(float_new(w_to_f(ex_sub(rt(slf).w, rt(a[0]).w)))); }
    time_add(c, slf, a[0], -1)
}
fn t_cmp(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let o = a[0];
    if is_time(o) { return Ok(Value::fix(ex_cmp(rt(slf).w, rt(o).w) as i64)); }
    if o.is_heap() && respond_to(o, s::cmp, true) {
        let r = funcall(c, o, s::cmp, &[slf], std::ptr::null())?;
        if r == NIL { return Ok(NIL); }
        let n = if r.is_fixnum() { r.as_fix() } else { let gt = funcall(c, r, s::gt, &[Value::fix(0)], std::ptr::null())?.truthy(); let lt = funcall(c, r, s::lt, &[Value::fix(0)], std::ptr::null())?.truthy(); if gt { 1 } else if lt { -1 } else { 0 } };
        return Ok(Value::fix(-n.signum()));
    }
    Ok(NIL)
}
fn t_eql(_c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    Ok(Value::bool(is_time(a[0]) && ex_cmp(rt(slf).w, rt(a[0]).w) == Ordering::Equal && is_integer(rt(slf).w) == is_integer(rt(a[0]).w)))
}
fn t_hash(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { let h = crate::hash::key_hash(c, rt(slf).w)?; Ok(Value::fix((h >> 2) as i64)) }
fn ndigits_den(c: &mut Ctx, a: &[Value]) -> RV {
    check_argc(c, a, 0, 1)?;
    match a.first() {
        None => Ok(Value::fix(1)),
        Some(&v) if v == NIL => Ok(Value::fix(1)),
        Some(&v) => {
            let nd = crate::builtins::numeric_methods::num_to_long(c, v)?;
            if nd < 0 { return arg_err(c, "negative ndigits given"); }
            if nd == 0 { return Ok(Value::fix(1)); }
            Ok(rat_raw(Value::fix(1), ipow(Value::fix(10), nd as u64)))
        }
    }
}
fn t_round_impl(c: &mut Ctx, slf: Value, a: &[Value], mode: u8) -> RV {
    let den = ndigits_den(c, a)?;
    let v = ex_mod(ex_div(rt(slf).w, ns()), den);
    match mode {
        b'f' => time_add(c, slf, v, -1),
        b'c' => { if ex_cmp(v, Value::fix(0)) == Ordering::Equal { time_add(c, slf, v, 1) } else { time_add(c, slf, ex_sub(den, v), 1) } }
        _ => if ex_cmp(v, ex_div(den, Value::fix(2))) == Ordering::Less { time_add(c, slf, v, -1) } else { time_add(c, slf, ex_sub(den, v), 1) },
    }
}
fn t_round(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV { t_round_impl(c, slf, a, b'r') }
fn t_floor(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV { t_round_impl(c, slf, a, b'f') }
fn t_ceil(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV { t_round_impl(c, slf, a, b'c') }

// ---- formatting -----------------------------------------------------------------------------

fn off_secs_rounded(off: Value) -> i64 {
    if off.is_fixnum() { return off.as_fix(); }
    // Rational#round: half away from zero
    let half = rat_raw(Value::fix(1), Value::fix(2));
    let neg = ex_cmp(off, Value::fix(0)) == Ordering::Less;
    let a = if neg { ex_sub(Value::fix(0), off) } else { off };
    let f = ex_floor(ex_add(a, half));
    let f = if f.is_fixnum() { f.as_fix() } else { 0 };
    if neg { -f } else { f }
}
fn weeknumber(v: &Vtm, first: i64) -> i64 {
    let mut wday = v.wday;
    if first == 1 { wday = if wday == 0 { 6 } else { wday - 1 }; }
    ((v.yday - 1 + 7 - wday) / 7).max(0)
}
fn iso_week(v: &Vtm) -> (i128, i64) {
    let isow = if v.wday == 0 { 7 } else { v.wday };
    let w = (v.yday - isow + 10) / 7;
    let weeks = |y: i128| -> i64 {
        let p = |y: i128| (y + y.div_euclid(4) - y.div_euclid(100) + y.div_euclid(400)).rem_euclid(7);
        if p(y) == 4 || p(y - 1) == 3 { 53 } else { 52 }
    };
    if w < 1 { (v.year - 1, weeks(v.year - 1)) } else if w > weeks(v.year) { (v.year + 1, 1) } else { (v.year, w) }
}
fn fmt_num(out: &mut String, v: i128, def_pad: u8, def_prec: usize, left: bool, padding: u8, precision: i64) {
    let prec = if left { 1 } else if precision <= 0 { def_prec } else { precision as usize };
    let zero = padding == b'0' || (padding == 0 && def_pad == b'0');
    let s = v.abs().to_string();
    let neg = v < 0;
    let len = s.len() + neg as usize;
    if zero {
        if neg { out.push('-'); }
        for _ in len..prec { out.push('0'); }
        out.push_str(&s);
    } else {
        for _ in len..prec { out.push(' '); }
        if neg { out.push('-'); }
        out.push_str(&s);
    }
}
fn strftime_into(c: &mut Ctx, out: &mut String, t: &RTime, v: &Vtm, fmt: &[u8]) -> Result<(), Esc> {
    let gmt = t.mode == UTC;
    let mut i = 0;
    while i < fmt.len() {
        if fmt[i] != b'%' {
            let st = i; while i < fmt.len() && fmt[i] != b'%' { i += 1; }
            out.push_str(&String::from_utf8_lossy(&fmt[st..i])); continue;
        }
        let sp = i;
        let (mut precision, mut left, mut upper, mut lower, mut chcase, mut padding, mut colons) = (-1i64, false, false, false, false, 0u8, 0usize);
        let mut tp: Option<String> = None;
        loop {
            i += 1;
            if i >= fmt.len() { tp = Some(String::from_utf8_lossy(&fmt[sp..]).into_owned()); precision = -1; left = false; upper = false; lower = false; padding = 0; break; }
            let ch = fmt[i];
            macro_rules! num { ($val:expr, $dp:expr, $prec:expr) => {{ fmt_num(out, $val as i128, $dp, $prec, left, padding, precision); break; }} }
            macro_rules! sub { ($f:expr) => {{
                let mut sbuf = String::new(); strftime_into(c, &mut sbuf, t, v, $f)?;
                if upper { sbuf = sbuf.to_uppercase(); } else if lower { sbuf = sbuf.to_lowercase(); }
                if precision > sbuf.len() as i64 { for _ in 0..(precision as usize - sbuf.len()) { out.push(if padding != 0 { padding as char } else { ' ' }); } }
                out.push_str(&sbuf); break;
            }} }
            let flag_ok = precision <= 0;
            match ch {
                b'%' => { tp = Some("%".into()); break; }
                b'a' => { if chcase { upper = true; } tp = Some(DAYS[v.wday as usize][..3].into()); break; }
                b'A' => { if chcase { upper = true; } tp = Some(DAYS[v.wday as usize].into()); break; }
                b'b' | b'h' => { if chcase { upper = true; } tp = Some(MONTHS[v.mon as usize - 1][..3].into()); break; }
                b'B' => { if chcase { upper = true; } tp = Some(MONTHS[v.mon as usize - 1].into()); break; }
                b'c' => sub!(b"%a %b %e %H:%M:%S %Y"),
                b'd' => num!(v.mday, b'0', 2),
                b'H' => num!(v.hour, b'0', 2),
                b'I' => num!(if v.hour % 12 == 0 { 12 } else { v.hour % 12 }, b'0', 2),
                b'j' => num!(v.yday, b'0', 3),
                b'm' => num!(v.mon, b'0', 2),
                b'M' => num!(v.min, b'0', 2),
                b'p' | b'P' => {
                    if (ch == b'p' && chcase) || (ch == b'P' && !chcase && !upper) { upper = false; lower = true; }
                    tp = Some(if v.hour < 12 { "AM".into() } else { "PM".into() }); break;
                }
                b's' => { let sx = secs_floor(t.w); num!(sx, b'0', 1) }
                b'S' => num!(v.sec, b'0', 2),
                b'U' => num!(weeknumber(v, 0), b'0', 2),
                b'w' => num!(v.wday, b'0', 1),
                b'W' => num!(weeknumber(v, 1), b'0', 2),
                b'x' | b'D' => sub!(b"%m/%d/%y"),
                b'X' | b'T' => sub!(b"%H:%M:%S"),
                b'y' => num!(v.year.rem_euclid(100), b'0', 2),
                b'Y' => num!(v.year, b'0', if v.year >= 0 { 4 } else { 5 }),
                b'z' => {
                    let mut off = if gmt { 0 } else { off_secs_rounded(v.off) };
                    let sign = if off < 0 || (gmt && left) { off = -off; -1 } else { 1 };
                    let p = match colons {
                        0 => if precision <= 5 { 2 } else { precision - 3 },
                        1 => if precision <= 6 { 2 } else { precision - 4 },
                        2 => if precision <= 9 { 2 } else { precision - 7 },
                        _ => if off % 3600 == 0 { if precision <= 3 { 2 } else { precision - 1 } } else if off % 60 == 0 { if precision <= 6 { 2 } else { precision - 4 } } else if precision <= 9 { 2 } else { precision - 7 },
                    } as usize;
                    let h = off / 3600;
                    let sg = if sign < 0 { '-' } else { '+' };
                    if padding == b' ' {
                        let body = format!("{}{}", sg, h);
                        for _ in body.len()..p + 1 { out.push(' '); }
                        out.push_str(&body);
                    } else {
                        out.push(sg);
                        let hs = h.to_string();
                        for _ in hs.len()..p { out.push('0'); }
                        out.push_str(&hs);
                    }
                    let r = off % 3600;
                    if colons == 3 && r == 0 { break; }
                    if colons >= 1 { out.push(':'); }
                    out.push_str(&format!("{:02}", r / 60));
                    let r2 = r % 60;
                    if colons == 3 && r2 == 0 { break; }
                    if colons >= 2 { out.push_str(&format!(":{:02}", r2)); }
                    break;
                }
                b'Z' => {
                    if chcase { upper = false; lower = true; }
                    tp = Some(if gmt { "UTC".into() } else { v.zone.clone().unwrap_or_default() }); break;
                }
                b'n' => { tp = Some("\n".into()); break; }
                b't' => { tp = Some("\t".into()); break; }
                b'e' => num!(v.mday, b' ', 2),
                b'r' => sub!(b"%I:%M:%S %p"),
                b'R' => sub!(b"%H:%M"),
                b'k' => num!(v.hour, b' ', 2),
                b'l' => num!(if v.hour % 12 == 0 { 12 } else { v.hour % 12 }, b' ', 2),
                b'v' => sub!(b"%e-%^b-%4Y"),
                b'C' => num!(v.year.div_euclid(100), b'0', 2),
                b'E' => { if i + 1 < fmt.len() && b"cCxXyY".contains(&fmt[i + 1]) { continue; } tp = Some(String::from_utf8_lossy(&fmt[sp..=i]).into_owned()); precision = -1; padding = 0; left = false; upper = false; lower = false; break; }
                b'O' => { if i + 1 < fmt.len() && b"deHkIlmMSuUVwWy".contains(&fmt[i + 1]) { continue; } tp = Some(String::from_utf8_lossy(&fmt[sp..=i]).into_owned()); precision = -1; padding = 0; left = false; upper = false; lower = false; break; }
                b'V' => num!(iso_week(v).1, b'0', 2),
                b'u' => num!(if v.wday == 0 { 7 } else { v.wday }, b'0', 1),
                b'G' => { let y = iso_week(v).0; num!(y, b'0', if y >= 0 { 4 } else { 5 }) }
                b'g' => num!(iso_week(v).0.rem_euclid(100), b'0', 2),
                b'L' | b'N' => {
                    let w = if ch == b'L' { 3 } else { 9 };
                    let p = if precision <= 0 { w } else { precision as usize };
                    let sub = ex_div(subns(t), ns());
                    let n = ex_floor(ex_mul(sub, ipow(Value::fix(10), p as u64)));
                    let s2 = istr(n);
                    for _ in s2.len()..p { out.push('0'); }
                    out.push_str(&s2);
                    break;
                }
                b'F' => sub!(b"%Y-%m-%d"),
                b'-' if flag_ok => { left = true; padding = 0; precision = 0; }
                b'^' if flag_ok => { upper = true; }
                b'#' if flag_ok => { chcase = true; }
                b'_' if flag_ok => { padding = b' '; }
                b':' => {
                    let mut k = 1;
                    let mut ok = false;
                    while k <= 3 { if i + k >= fmt.len() { break; } if fmt[i + k] == b'z' { ok = true; break; } if fmt[i + k] != b':' { break; } k += 1; }
                    if !ok { tp = Some(String::from_utf8_lossy(&fmt[sp..=i]).into_owned()); precision = -1; padding = 0; left = false; upper = false; lower = false; break; }
                    colons = k; i += k - 1;
                }
                b'0'..=b'9' => {
                    if ch == b'0' { padding = b'0'; }
                    let st = i; while i < fmt.len() && fmt[i].is_ascii_digit() { i += 1; }
                    precision = String::from_utf8_lossy(&fmt[st..i]).parse().unwrap_or(0);
                    i -= 1;
                }
                _ => { tp = Some(String::from_utf8_lossy(&fmt[sp..=i]).into_owned()); precision = -1; padding = 0; left = false; upper = false; lower = false; break; }
            }
        }
        i += 1;
        if let Some(mut s2) = tp {
            if !left && precision > s2.chars().count() as i64 { for _ in 0..(precision as usize - s2.chars().count()) { out.push(if padding != 0 { padding as char } else { ' ' }); } }
            if upper { s2 = s2.to_ascii_uppercase(); } else if lower { s2 = s2.to_ascii_lowercase(); }
            out.push_str(&s2);
        }
    }
    Ok(())
}
fn strf(c: &mut Ctx, slf: Value, fmt: &[u8]) -> Result<String, Esc> {
    let t = rt(slf); let v = vtm_of(t);
    let mut out = String::new();
    strftime_into(c, &mut out, t, &v, fmt)?;
    Ok(out)
}
fn t_strftime(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let f = expect_str(c, a[0])?;
    let fb = str_bytes(f).to_vec();
    let enc = rstr(f).enc;
    let s2 = strf(c, slf, &fb)?;
    Ok(str_new_enc(s2.as_bytes(), if enc == crate::object::ENC_ASCII && !s2.is_ascii() { crate::object::ENC_UTF8 } else { enc }))
}
fn ascii(s2: String) -> Value { str_new_enc(s2.as_bytes(), crate::object::ENC_ASCII) }
fn t_to_s(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let f: &[u8] = if rt(slf).mode == UTC { b"%Y-%m-%d %H:%M:%S UTC" } else { b"%Y-%m-%d %H:%M:%S %z" };
    Ok(ascii(strf(c, slf, f)?))
}
fn t_ctime(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(ascii(strf(c, slf, b"%a %b %e %H:%M:%S %Y")?)) }
fn t_inspect(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let mut s2 = strf(c, slf, b"%Y-%m-%d %H:%M:%S")?;
    let t = rt(slf);
    let sub = subns(t);
    if sub == Value::fix(0) {}
    else if sub.is_fixnum() { let f = format!(".{:09}", sub.as_fix()); s2.push_str(f.trim_end_matches('0')); }
    else { s2.push(' '); s2.push_str(&ex_to_s(ex_div(sub, ns()))); }
    if t.mode == UTC { s2.push_str(" UTC"); }
    else {
        let v = vtm_of(t);
        let mut off = off_secs_rounded(v.off);
        let sign = if off < 0 { off = -off; '-' } else { '+' };
        s2.push_str(&format!(" {}{:02}{:02}", sign, off / 3600, off / 60 % 60));
        if off % 60 != 0 { s2.push_str(&format!("{:02}", off % 60)); }
    }
    Ok(ascii(s2))
}
fn t_to_a(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let v = vtm_of(rt(slf));
    let z = t_zone(c, slf, &[], std::ptr::null())?;
    Ok(ary_from_vec(vec![Value::fix(v.sec), Value::fix(v.min), Value::fix(v.hour), Value::fix(v.mday), Value::fix(v.mon), int_of(v.year), Value::fix(v.wday), Value::fix(v.yday), Value::bool(v.isdst), z]))
}
fn t_deconstruct_keys(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let v = vtm_of(rt(slf));
    let keys = ["year", "month", "day", "yday", "wday", "hour", "min", "sec", "subsec", "dst", "zone"];
    let h = crate::hash::hash_new();
    let want = a[0];
    if want != NIL && !is_array(want) { let cn = class_name_for_err(want); return type_err(c, format!("wrong argument type {} (expected Array or nil)", cn)); }
    for k in keys {
        let ks = sym_value(k);
        if want != NIL { let mut found = false; for i in 0..ary_len(want) { if ary_get(want, i) == ks { found = true; } } if !found { continue; } }
        let val = match k {
            "year" => int_of(v.year), "month" => Value::fix(v.mon), "day" => Value::fix(v.mday), "yday" => Value::fix(v.yday), "wday" => Value::fix(v.wday),
            "hour" => Value::fix(v.hour), "min" => Value::fix(v.min), "sec" => Value::fix(v.sec), "subsec" => t_subsec(c, slf, &[], std::ptr::null())?,
            "dst" => Value::bool(v.isdst), _ => t_zone(c, slf, &[], std::ptr::null())?,
        };
        crate::hash::hash_set(c, h, ks, val)?;
    }
    Ok(h)
}
fn t_xmlschema(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    check_argc(c, a, 0, 1)?;
    let fd = if a.is_empty() { 0 } else { crate::builtins::numeric_methods::num_to_long(c, a[0])?.max(0) as usize };
    let t = rt(slf); let v = vtm_of(t);
    let mut s2 = String::new();
    if v.year < 0 { s2.push_str(&format!("-{:04}", -v.year)); } else { s2.push_str(&format!("{:04}", v.year)); }
    s2.push_str(&format!("-{:02}-{:02}T{:02}:{:02}:{:02}", v.mon, v.mday, v.hour, v.min, v.sec));
    if fd > 0 {
        s2.push('.');
        let n = ex_floor(ex_mul(ex_div(subns(t), ns()), ipow(Value::fix(10), fd as u64)));
        let d = istr(n);
        for _ in d.len()..fd { s2.push('0'); }
        s2.push_str(&d);
    }
    if t.mode == UTC { s2.push('Z'); }
    else {
        let mut off = off_secs_rounded(v.off);
        let sign = if off < 0 { off = -off; '-' } else { '+' };
        off /= 60;
        s2.push_str(&format!("{}{:02}:{:02}", sign, off / 60, off % 60));
    }
    Ok(ascii(s2))
}
fn t_init_copy(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    if slf == a[0] { return Ok(slf); }
    modify(c, slf)?;
    let Some(o) = rdata::<RTime>(a[0]) else { return type_err(c, "initialize_copy should take same class object") };
    let (w, m, off) = (o.w, o.mode, o.off);
    let t = rt(slf); t.w = w; t.mode = m; t.off = off;
    Ok(slf)
}

pub fn dup_data(_c: &mut Ctx, v: Value) -> Result<Option<Value>, Esc> {
    if let Some(t) = rdata::<RTime>(v) { return Ok(Some(new_time(real_class_of(v), t.w, t.mode, t.off))); }
    Ok(None)
}
pub fn alloc_data(_c: &mut Ctx, klass: Value) -> Result<Option<Value>, Esc> {
    if crate::builtins::inherits(klass, vm().c_time) { return Ok(Some(new_time(klass, Value::fix(0), LOCAL, NIL))); }
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
    define_singleton_cfunc(k, "utc", t_utc_s, -1);
    define_singleton_cfunc(k, "gm", t_utc_s, -1);
    define_private_cfunc(k, "initialize", t_initialize, -1);
    define_private_cfunc(k, "initialize_copy", t_init_copy, 1);
    let d = |n: &str, f: CFunc, a: i32| define_cfunc(k, n, f, a);
    d("to_f", t_to_f, 0); d("to_i", t_to_i, 0); d("to_r", t_to_r, 0); d("tv_sec", t_to_i, 0); d("usec", t_usec, 0); d("tv_usec", t_usec, 0);
    d("nsec", t_nsec, 0); d("tv_nsec", t_nsec, 0); d("subsec", t_subsec, 0); d("-", t_minus, 1); d("+", t_plus, 1);
    d("<=>", t_cmp, 1); d("eql?", t_eql, 1); d("hash", t_hash, 0); d("year", t_year, 0); d("month", t_month, 0); d("mon", t_month, 0);
    d("day", t_day, 0); d("mday", t_day, 0); d("hour", t_hour, 0); d("min", t_min, 0); d("sec", t_sec, 0); d("wday", t_wday, 0); d("yday", t_yday, 0);
    d("isdst", t_isdst, 0); d("dst?", t_isdst, 0); d("zone", t_zone, 0);
    d("utc_offset", t_utc_offset, 0); d("gmt_offset", t_utc_offset, 0); d("gmtoff", t_utc_offset, 0); d("utc", t_utc, 0); d("gmtime", t_utc, 0);
    d("getutc", t_getutc, 0); d("getgm", t_getutc, 0); d("localtime", t_localtime, -1); d("getlocal", t_getlocal, -1);
    d("utc?", t_utc_p, 0); d("gmt?", t_utc_p, 0); d("strftime", t_strftime, 1); d("to_s", t_to_s, 0); d("inspect", t_inspect, 0);
    d("ctime", t_ctime, 0); d("asctime", t_ctime, 0); d("to_a", t_to_a, 0); d("deconstruct_keys", t_deconstruct_keys, 1);
    d("round", t_round, -1); d("floor", t_floor, -1); d("ceil", t_ceil, -1); d("xmlschema", t_xmlschema, -1); d("iso8601", t_xmlschema, -1);
    d("sunday?", t_sunday, 0); d("monday?", t_monday, 0); d("tuesday?", t_tuesday, 0); d("wednesday?", t_wednesday, 0);
    d("thursday?", t_thursday, 0); d("friday?", t_friday, 0); d("saturday?", t_saturday, 0);
}

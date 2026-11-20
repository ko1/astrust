// Assorted built-ins: Encoding, pack/unpack, GC, ObjectSpace, ENV, Thread (green, sequential), Mutex, Queue, Signal, constants.
use crate::array::*;
use crate::builtins::kernel::*;
use crate::builtins::numeric_methods::num_to_long;
use crate::hash::*;
use crate::string::*;
use crate::symbol::{intern, s};
use crate::value::*;
use crate::vm::*;

// ---- Encoding ----------------------------------------------------------------------------

pub struct REnc { pub id: u8 }
impl RData for REnc {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}
const ENC_NAMES: [&str; 3] = ["UTF-8", "ASCII-8BIT", "US-ASCII"];
static mut ENC_OBJS: [u64; 3] = [0; 3];
pub fn encoding_value(_c: &mut Ctx, id: u8) -> RV { Ok(unsafe { Value::from_raw(ENC_OBJS[id as usize]) }) }
pub fn encoding_id(c: &mut Ctx, v: Value) -> Result<u8, Esc> {
    if let Some(e) = rdata::<REnc>(v) { return Ok(e.id); }
    let nm = to_rstring(c, v)?.to_ascii_uppercase();
    Ok(match nm.as_str() {
        "UTF-8" | "UTF8" | "CP65001" => ENC_UTF8,
        "ASCII-8BIT" | "BINARY" => ENC_BINARY,
        "US-ASCII" | "ASCII" | "ANSI_X3.4-1968" | "646" => ENC_ASCII,
        _ => { if nm.starts_with("UTF-16") || nm.starts_with("UTF-32") || nm.starts_with("ISO-8859") || nm.starts_with("EUC") || nm.starts_with("SHIFT") || nm.starts_with("WINDOWS") || nm.starts_with("CP") { ENC_BINARY } else { return arg_err(c, format!("unknown encoding name - {}", nm)); } }
    })
}
fn enc_to_s(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(str_new(ENC_NAMES[rdata::<REnc>(slf).unwrap().id as usize].as_bytes())) }
fn enc_inspect(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let id = rdata::<REnc>(slf).unwrap().id;
    Ok(str_new(format!("#<Encoding:{}{}>", if id == ENC_BINARY { "BINARY (ASCII-8BIT)" } else { ENC_NAMES[id as usize] }, "").as_bytes()))
}
fn enc_s_find(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV { let id = encoding_id(c, a[0])?; encoding_value(c, id) }
fn enc_default(c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV { encoding_value(c, ENC_UTF8) }
fn enc_ascii_compat(_c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(TRUE) }
fn enc_names(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { let id = rdata::<REnc>(slf).unwrap().id; Ok(ary_new_from(&[str_new(ENC_NAMES[id as usize].as_bytes())])) }

// ---- pack / unpack ----------------------------------------------------------------------

fn parse_tmpl(t: &[u8]) -> Vec<(u8, Option<usize>, bool, bool)> {
    let mut out = vec![];
    let mut i = 0;
    while i < t.len() {
        let ch = t[i]; i += 1;
        if ch.is_ascii_whitespace() { continue; }
        let mut little = false; let mut big = false;
        while i < t.len() && (t[i] == b'_' || t[i] == b'!' || t[i] == b'<' || t[i] == b'>') { if t[i] == b'<' { little = true; } if t[i] == b'>' { big = true; } i += 1; }
        let mut cnt = None; let mut star = false;
        if i < t.len() && t[i] == b'*' { star = true; i += 1; }
        else { let st = i; while i < t.len() && t[i].is_ascii_digit() { i += 1; } if i > st { cnt = Some(std::str::from_utf8(&t[st..i]).unwrap().parse().unwrap()); } }
        let _ = little;
        out.push((ch, if star { Some(usize::MAX) } else { cnt }, big, star));
    }
    out
}
pub fn pack(c: &mut Ctx, ary: Value, a: &[Value]) -> RV {
    let t = expect_str(c, a[0])?;
    let tmpl = parse_tmpl(&rstr(t).b);
    let mut out: Vec<u8> = vec![];
    let mut idx = 0;
    let mut binary = false;
    for (ch, cnt, big, _) in tmpl {
        let next = |c: &mut Ctx, idx: &mut usize| -> RV { let v = ary_get(ary, *idx); if *idx >= ary_len(ary) { return arg_err(c, "too few arguments"); } *idx += 1; Ok(v) };
        match ch {
            b'a' | b'A' | b'Z' => {
                let v = next(c, &mut idx)?; let sv = expect_str(c, v)?; let bs = rstr(sv).b.clone();
                let n = match cnt { None => 1, Some(usize::MAX) => bs.len() + if ch == b'Z' { 1 } else { 0 }, Some(n) => n };
                let pad = if ch == b'A' { b' ' } else { 0 };
                for k in 0..n { out.push(*bs.get(k).unwrap_or(&pad)); }
            }
            b'U' => {
                let n = match cnt { None => 1, Some(usize::MAX) => ary_len(ary) - idx, Some(n) => n };
                for _ in 0..n { let v = next(c, &mut idx)?; let cp = num_to_long(c, v)?; if cp < 0 { return err(c, vm().c_range_error, "pack(U): value out of range"); } encode_utf8(cp as u32, &mut out); }
            }
            b'C' | b'c' | b'S' | b's' | b'L' | b'l' | b'Q' | b'q' | b'N' | b'n' | b'V' | b'v' | b'J' | b'j' => {
                binary = true;
                let n = match cnt { None => 1, Some(usize::MAX) => ary_len(ary) - idx, Some(n) => n };
                for _ in 0..n {
                    let v = next(c, &mut idx)?;
                    let x = if crate::object::is_bignum(v) { crate::object::rbig(v).bitop(&crate::bignum::BigInt::from_u64(u64::MAX), b'&').to_i64().unwrap_or(-1) } else { num_to_long(c, v)? } as u64;
                    let (sz, be) = match ch { b'C' | b'c' => (1, false), b'S' | b's' => (2, big), b'L' | b'l' => (4, big), b'Q' | b'q' | b'J' | b'j' => (8, big), b'N' => (4, true), b'n' => (2, true), b'V' => (4, false), _ => (2, false) };
                    let bytes = x.to_le_bytes();
                    if be { for k in (0..sz).rev() { out.push(bytes[k]); } } else { out.extend_from_slice(&bytes[..sz]); }
                }
            }
            b'E' | b'e' | b'D' | b'd' | b'F' | b'f' | b'G' | b'g' => {
                binary = true;
                let n = match cnt { None => 1, Some(usize::MAX) => ary_len(ary) - idx, Some(n) => n };
                for _ in 0..n {
                    let v = next(c, &mut idx)?; let f = crate::builtins::numeric_methods::to_f(c, v)?;
                    match ch { b'E' | b'D' | b'd' => out.extend_from_slice(&f.to_le_bytes()), b'G' => out.extend_from_slice(&f.to_be_bytes()),
                        b'e' | b'F' | b'f' => out.extend_from_slice(&(f as f32).to_le_bytes()), _ => out.extend_from_slice(&(f as f32).to_be_bytes()) }
                }
            }
            b'H' | b'h' => {
                let v = next(c, &mut idx)?; let sv = expect_str(c, v)?; let hs = rstr(sv).b.clone();
                let n = match cnt { None => 1, Some(usize::MAX) => hs.len(), Some(n) => n };
                let mut k = 0;
                while k < n { let hi = hex_v(*hs.get(k).unwrap_or(&b'0')); let lo = hex_v(*hs.get(k + 1).unwrap_or(&b'0')); out.push(if ch == b'H' { hi << 4 | if k + 1 < n { lo } else { 0 } } else { lo << 4 | hi }); k += 2; }
            }
            b'm' => {
                let v = next(c, &mut idx)?; let sv = expect_str(c, v)?;
                let enc = base64(&rstr(sv).b);
                let line = match cnt { Some(0) => usize::MAX, Some(n) if n != usize::MAX => (n / 3 * 4).max(4), _ => 60 };
                if line == usize::MAX { out.extend_from_slice(enc.as_bytes()); } else { for ch2 in enc.as_bytes().chunks(line) { out.extend_from_slice(ch2); out.push(b'\n'); } }
            }
            b'x' => { let n = cnt.unwrap_or(1); for _ in 0..n { out.push(0); } }
            b'w' => {
                let n = match cnt { None => 1, Some(usize::MAX) => ary_len(ary) - idx, Some(n) => n };
                for _ in 0..n { let v = next(c, &mut idx)?; let mut x = num_to_long(c, v)? as u64; let mut tmp = vec![(x & 0x7f) as u8]; x >>= 7; while x > 0 { tmp.push((x & 0x7f) as u8 | 0x80); x >>= 7; } tmp.reverse(); out.extend(tmp); }
            }
            _ => return arg_err(c, format!("unknown pack directive '{}'", ch as char)),
        }
    }
    Ok(str_from_vec_enc(out, if binary { ENC_BINARY } else { ENC_UTF8 }))
}
fn hex_v(x: u8) -> u8 { (x as char).to_digit(16).unwrap_or(0) as u8 }
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
pub fn unpack(c: &mut Ctx, sv: Value, a: &[Value]) -> RV {
    let t = expect_str(c, a[0])?;
    let tmpl = parse_tmpl(&rstr(t).b);
    let b = rstr(sv).b.clone();
    let mut pos = 0;
    let out = ary_new();
    for (ch, cnt, big, _) in tmpl {
        match ch {
            b'a' | b'A' | b'Z' => {
                let n = match cnt { None => 1, Some(usize::MAX) => b.len().saturating_sub(pos), Some(n) => n };
                let e = (pos + n).min(b.len());
                let mut v = b[pos..e].to_vec(); pos = e;
                if ch == b'A' { while v.last().map(|x| *x == b' ' || *x == 0).unwrap_or(false) { v.pop(); } }
                if ch == b'Z' { if let Some(z) = v.iter().position(|&x| x == 0) { v.truncate(z); } }
                ary_push(out, str_from_vec_enc(v, if ch == b'a' || ch == b'Z' || ch == b'A' { ENC_BINARY } else { ENC_UTF8 }));
            }
            b'U' => {
                let n = match cnt { None => 1, Some(usize::MAX) => usize::MAX, Some(n) => n };
                let mut k = 0;
                while k < n && pos < b.len() { let (cp, l) = decode_utf8(&b, pos); ary_push(out, Value::fix(cp as i64)); pos += l; k += 1; }
            }
            b'C' | b'c' | b'S' | b's' | b'L' | b'l' | b'Q' | b'q' | b'N' | b'n' | b'V' | b'v' | b'J' | b'j' => {
                let (sz, be, signed) = match ch { b'C' => (1, false, false), b'c' => (1, false, true), b'S' => (2, big, false), b's' => (2, big, true), b'L' => (4, big, false), b'l' => (4, big, true),
                    b'Q' | b'J' => (8, big, false), b'q' | b'j' => (8, big, true), b'N' => (4, true, false), b'n' => (2, true, false), b'V' => (4, false, false), _ => (2, false, false) };
                let n = match cnt { None => 1, Some(usize::MAX) => (b.len() - pos.min(b.len())) / sz, Some(n) => n };
                for _ in 0..n {
                    if pos + sz > b.len() { ary_push(out, NIL); continue; }
                    let mut x: u64 = 0;
                    for k in 0..sz { let byte = if be { b[pos + k] } else { b[pos + sz - 1 - k] }; x = x << 8 | byte as u64; }
                    pos += sz;
                    let v = if signed { let sh = 64 - sz * 8; ((x << sh) as i64) >> sh } else { x as i64 };
                    ary_push(out, if !signed && sz == 8 { crate::bignum::int_from_i128(x as i128) } else { Value::fix(v) });
                }
            }
            b'E' | b'D' | b'd' | b'G' | b'e' | b'F' | b'f' | b'g' => {
                let sz = if matches!(ch, b'E' | b'D' | b'd' | b'G') { 8 } else { 4 };
                let n = match cnt { None => 1, Some(usize::MAX) => (b.len() - pos.min(b.len())) / sz, Some(n) => n };
                for _ in 0..n {
                    if pos + sz > b.len() { ary_push(out, NIL); continue; }
                    let sl = &b[pos..pos + sz]; pos += sz;
                    let f = match ch { b'E' | b'D' | b'd' => f64::from_le_bytes(sl.try_into().unwrap()), b'G' => f64::from_be_bytes(sl.try_into().unwrap()),
                        b'g' => f32::from_be_bytes(sl.try_into().unwrap()) as f64, _ => f32::from_le_bytes(sl.try_into().unwrap()) as f64 };
                    ary_push(out, crate::numeric::float_new(f));
                }
            }
            b'H' | b'h' => {
                let n = match cnt { None => 1, Some(usize::MAX) => (b.len() - pos) * 2, Some(n) => n };
                let mut s2 = String::new();
                let mut k = 0;
                while k < n && pos + k / 2 < b.len() { let byte = b[pos + k / 2]; let nib = if (k % 2 == 0) == (ch == b'H') { byte >> 4 } else { byte & 15 }; s2.push(std::char::from_digit(nib as u32, 16).unwrap()); k += 1; }
                pos += (k + 1) / 2;
                ary_push(out, str_new(s2.as_bytes()));
            }
            b'm' => { ary_push(out, str_from_vec_enc(unbase64(&b[pos..]), ENC_BINARY)); pos = b.len(); }
            b'x' => { pos += cnt.unwrap_or(1); }
            b'w' => {
                let n = match cnt { None => 1, Some(usize::MAX) => usize::MAX, Some(n) => n };
                let mut k = 0;
                while k < n && pos < b.len() { let mut x: u64 = 0; loop { let byte = b[pos]; pos += 1; x = x << 7 | (byte & 0x7f) as u64; if byte & 0x80 == 0 || pos >= b.len() { break; } } ary_push(out, Value::fix(x as i64)); k += 1; }
            }
            _ => return arg_err(c, format!("unknown unpack directive '{}'", ch as char)),
        }
    }
    Ok(out)
}

// ---- GC / ObjectSpace -----------------------------------------------------------------------

fn gc_start(_c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV { crate::gc::collect(); Ok(NIL) }
fn gc_count(_c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::fix(crate::gc::heap().gc_count as i64)) }
fn gc_disable(_c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV { let h = crate::gc::heap(); let old = h.disabled; h.disabled = true; Ok(Value::bool(old)) }
fn gc_enable(_c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV { let h = crate::gc::heap(); let old = h.disabled; h.disabled = false; Ok(Value::bool(old)) }
fn gc_stat(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    let h = crate::gc::heap();
    let st = hash_new();
    for (k, v) in [("count", h.gc_count as i64), ("heap_live_slots", (h.total - h.free_count) as i64), ("heap_free_slots", h.free_count as i64),
                   ("total_allocated_objects", h.total_allocated as i64), ("total_freed_objects", h.total_freed as i64), ("heap_available_slots", h.total as i64), ("major_gc_count", h.gc_count as i64), ("minor_gc_count", 0)] {
        hash_set(c, st, sym_value(k), Value::fix(v))?;
    }
    if let Some(&k) = a.first() { if k.is_sym() { return Ok(hash_get_opt(c, st, k)?.unwrap_or(Value::fix(0))); } }
    Ok(st)
}
fn os_each_object(c: &mut Ctx, _s: Value, a: &[Value], blk: *const Block) -> RV {
    let k = a.first().copied();
    let list = ary_new();
    crate::gc::each_object(|v| {
        let o = obj(v);
        if matches!(o.body, Body::Env(..)) || o.klass == NIL { return; }
        if is_class_or_module(v) && rclass(v).is_singleton { return; }
        if let Some(k) = k { if !is_class_or_module(o.klass) || !is_kind_of(v, k) { return; } }
        ary_push(list, v);
    });
    let n = ary_len(list);
    if blk.is_null() { return funcall(c, list, s::each, &[], std::ptr::null()); }
    for i in 0..n { yield1(c, blk, ary_get(list, i))?; }
    Ok(Value::fix(n as i64))
}
fn os_count_objects(c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV {
    let h = crate::gc::heap();
    let st = hash_new();
    hash_set(c, st, sym_value("TOTAL"), Value::fix(h.total as i64))?;
    hash_set(c, st, sym_value("FREE"), Value::fix(h.free_count as i64))?;
    Ok(st)
}
fn os_garbage_collect(_c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV { crate::gc::collect(); Ok(NIL) }

// ---- ENV --------------------------------------------------------------------------------------

fn env_aref(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    let k = to_rstring(c, a[0])?;
    Ok(std::env::var_os(&k).map(|v| str_new_frozen(v.to_string_lossy().as_bytes())).unwrap_or(NIL))
}
fn env_fetch(c: &mut Ctx, s2: Value, a: &[Value], b: *const Block) -> RV {
    let v = env_aref(c, s2, a, b)?;
    if v != NIL { return Ok(v); }
    if !b.is_null() { return yield1(c, b, a[0]); }
    if a.len() > 1 { return Ok(a[1]); }
    let e = make_exception(vm().c_key_error, &format!("key not found: {}", inspect_str(c, a[0])?));
    Err(raise_exc(c, e))
}
fn env_aset(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    let k = to_rstring(c, a[0])?;
    if a[1] == NIL { std::env::remove_var(&k); } else { let v = to_rstring(c, a[1])?; std::env::set_var(&k, v); }
    Ok(a[1])
}
fn env_key_p(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV { let k = to_rstring(c, a[0])?; Ok(Value::bool(std::env::var_os(&k).is_some())) }
fn env_to_h(c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV {
    let h = hash_new();
    for (k, v) in std::env::vars_os() { hash_set(c, h, str_new(k.to_string_lossy().as_bytes()), str_new(v.to_string_lossy().as_bytes()))?; }
    Ok(h)
}
fn env_delete(c: &mut Ctx, s2: Value, a: &[Value], b: *const Block) -> RV { let v = env_aref(c, s2, a, b)?; let k = to_rstring(c, a[0])?; std::env::remove_var(&k); Ok(v) }

// ---- Thread / Mutex / Queue (sequential) ----------------------------------------------------

fn th_s_new(c: &mut Ctx, slf: Value, a: &[Value], b: *const Block) -> RV {
    let t = new_object(slf);
    ivar_set_raw(t, intern(b"__locals"), hash_new());
    let r = yield_block(c, b, a);
    match r {
        Ok(v) => { ivar_set_raw(t, intern(b"__value"), v); }
        Err(_) => { if c.esc == EscKind::Raise { ivar_set_raw(t, intern(b"__error"), c.esc_val); c.esc = EscKind::None; } else { c.esc = EscKind::None; } }
    }
    Ok(t)
}
fn th_join(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let e = ivar_get(slf, intern(b"__error"));
    if e != NIL { ivar_set_raw(slf, intern(b"__error"), NIL); return Err(raise_exc(c, e)); }
    Ok(slf)
}
fn th_value(c: &mut Ctx, slf: Value, a: &[Value], b: *const Block) -> RV { th_join(c, slf, a, b)?; Ok(ivar_get(slf, intern(b"__value"))) }
fn th_current(_c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let key = intern(b"__main_thread");
    let mut t = ivar_get(slf, key);
    if t == NIL { t = new_object(slf); ivar_set_raw(t, intern(b"__locals"), hash_new()); ivar_set_raw(slf, key, t); }
    Ok(t)
}
fn th_aref(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let h = ivar_get(slf, intern(b"__locals"));
    let k = if is_string(a[0]) { str_to_sym(a[0]) } else { a[0] };
    Ok(hash_get_opt(c, h, k)?.unwrap_or(NIL))
}
fn th_aset(c: &mut Ctx, slf: Value, a: &[Value], _b: *const Block) -> RV {
    let h = ivar_get(slf, intern(b"__locals"));
    let k = if is_string(a[0]) { str_to_sym(a[0]) } else { a[0] };
    hash_set(c, h, k, a[1])?;
    Ok(a[1])
}
fn th_alive(_c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(FALSE) }
fn mutex_sync(c: &mut Ctx, _s: Value, _a: &[Value], b: *const Block) -> RV { yield_block(c, b, &[]) }
fn noop_self(_c: &mut Ctx, s2: Value, _a: &[Value], _b: *const Block) -> RV { Ok(s2) }
fn q_init(_c: &mut Ctx, s2: Value, _a: &[Value], _b: *const Block) -> RV { ivar_set_raw(s2, intern(b"@q"), ary_new()); Ok(NIL) }
fn q_push(_c: &mut Ctx, s2: Value, a: &[Value], _b: *const Block) -> RV { ary_push(ivar_get(s2, intern(b"@q")), a[0]); Ok(s2) }
fn q_pop(c: &mut Ctx, s2: Value, _a: &[Value], _b: *const Block) -> RV {
    let q = ivar_get(s2, intern(b"@q"));
    if ary_len(q) == 0 { return err(c, vm().c_thread_error, "queue empty"); }
    Ok(rary(q).remove(0))
}
fn q_size(_c: &mut Ctx, s2: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::fix(ary_len(ivar_get(s2, intern(b"@q"))) as i64)) }
fn q_empty(_c: &mut Ctx, s2: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::bool(ary_len(ivar_get(s2, intern(b"@q"))) == 0)) }

fn signal_trap(_c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(str_new(b"DEFAULT")) }
fn warning_warn(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV { let b = to_rstring(c, a[0])?; err_write(b.as_bytes()); Ok(NIL) }
fn objspace_wm_init(_c: &mut Ctx, s2: Value, _a: &[Value], _b: *const Block) -> RV { let h = hash_new(); rhash(h).identity = true; ivar_set_raw(s2, intern(b"@h"), h); Ok(NIL) }
fn wm_aref(c: &mut Ctx, s2: Value, a: &[Value], _b: *const Block) -> RV { let h = ivar_get(s2, intern(b"@h")); Ok(hash_get_opt(c, h, a[0])?.unwrap_or(NIL)) }
fn wm_aset(c: &mut Ctx, s2: Value, a: &[Value], _b: *const Block) -> RV { let h = ivar_get(s2, intern(b"@h")); hash_set(c, h, a[0], a[1])?; Ok(a[1]) }
fn wm_key(c: &mut Ctx, s2: Value, a: &[Value], _b: *const Block) -> RV { let h = ivar_get(s2, intern(b"@h")); Ok(Value::bool(hash_lookup(c, h, a[0])?.is_some())) }

pub fn dup_data(_c: &mut Ctx, v: Value) -> Result<Option<Value>, Esc> {
    if let Some(e) = rdata::<REnc>(v) { let _ = e; return Ok(Some(v)); }
    Ok(None)
}
pub fn alloc_data(_c: &mut Ctx, _klass: Value) -> Result<Option<Value>, Esc> { Ok(None) }

pub fn init() {
    let vm = vm();
    let ob = vm.c_object;
    // Encoding
    let ek = vm.c_encoding;
    for i in 0..3u8 {
        let e = crate::gc::alloc(ek, Body::Data(Box::new(REnc { id: i })));
        unsafe { ENC_OBJS[i as usize] = e.raw(); }
        vm.pinned.push(e);
    }
    unsafe {
        const_set(ek, intern(b"UTF_8"), Value::from_raw(ENC_OBJS[0]));
        const_set(ek, intern(b"ASCII_8BIT"), Value::from_raw(ENC_OBJS[1]));
        const_set(ek, intern(b"BINARY"), Value::from_raw(ENC_OBJS[1]));
        const_set(ek, intern(b"US_ASCII"), Value::from_raw(ENC_OBJS[2]));
        const_set(ek, intern(b"ASCII"), Value::from_raw(ENC_OBJS[2]));
        for n in ["UTF_16LE", "UTF_16BE", "UTF_32LE", "UTF_32BE", "ISO_8859_1", "EUC_JP", "Shift_JIS", "Windows_31J", "UTF_16", "UTF_32", "Windows_1252"] {
            const_set(ek, intern(n.as_bytes()), Value::from_raw(ENC_OBJS[1]));
        }
    }
    define_cfunc(ek, "to_s", enc_to_s, 0);
    define_cfunc(ek, "name", enc_to_s, 0);
    define_cfunc(ek, "inspect", enc_inspect, 0);
    define_cfunc(ek, "ascii_compatible?", enc_ascii_compat, 0);
    define_cfunc(ek, "names", enc_names, 0);
    define_singleton_cfunc(ek, "find", enc_s_find, 1);
    define_singleton_cfunc(ek, "default_external", enc_default, 0);
    define_singleton_cfunc(ek, "default_internal", |_c, _s, _a, _b| Ok(NIL), 0);
    define_singleton_cfunc(ek, "default_external=", |_c, _s, a, _b| Ok(a[0]), 1);
    define_singleton_cfunc(ek, "default_internal=", |_c, _s, a, _b| Ok(a[0]), 1);
    // GC / ObjectSpace
    let gc = define_module("GC");
    vm.m_gc = gc;
    define_module_function(gc, "start", gc_start, -1);
    define_module_function(gc, "count", gc_count, 0);
    define_module_function(gc, "disable", gc_disable, 0);
    define_module_function(gc, "enable", gc_enable, 0);
    define_module_function(gc, "stat", gc_stat, -1);
    define_module_function(gc, "compact", gc_start, 0);
    define_module_function(gc, "stress", |_c, _s, _a, _b| Ok(FALSE), 0);
    define_module_function(gc, "stress=", |_c, _s, a, _b| Ok(a[0]), 1);
    let os = define_module("ObjectSpace");
    vm.m_object_space = os;
    define_module_function(os, "each_object", os_each_object, -1);
    define_module_function(os, "count_objects", os_count_objects, -1);
    define_module_function(os, "garbage_collect", os_garbage_collect, -1);
    let wm = define_class_under(os, "WeakMap", ob);
    vm.c_object_space_weak_map = wm;
    define_private_cfunc(wm, "initialize", objspace_wm_init, 0);
    define_cfunc(wm, "[]", wm_aref, 1);
    define_cfunc(wm, "[]=", wm_aset, 2);
    define_cfunc(wm, "key?", wm_key, 1);
    define_cfunc(wm, "include?", wm_key, 1);
    // ENV
    let env = new_object(ob);
    for (n, f, a) in [("[]", env_aref as CFunc, 1), ("fetch", env_fetch, -1), ("[]=", env_aset, 2), ("store", env_aset, 2), ("key?", env_key_p, 1), ("has_key?", env_key_p, 1),
                      ("include?", env_key_p, 1), ("to_h", env_to_h, 0), ("to_hash", env_to_h, 0), ("delete", env_delete, 1)] {
        define_singleton_cfunc(env, n, f, a);
    }
    define_singleton_cfunc(env, "inspect", |c, s2, a, b| { let h = env_to_h(c, s2, a, b)?; crate::builtins::hash_methods::hash_inspect(c, h) }, 0);
    const_set(ob, intern(b"ENV"), env);
    // Thread & co.
    let th = define_class("Thread", ob);
    vm.c_thread = th;
    define_singleton_cfunc(th, "new", th_s_new, -1);
    define_singleton_cfunc(th, "start", th_s_new, -1);
    define_singleton_cfunc(th, "current", th_current, 0);
    define_singleton_cfunc(th, "main", th_current, 0);
    define_singleton_cfunc(th, "pass", |_c, _s, _a, _b| Ok(NIL), 0);
    define_cfunc(th, "join", th_join, -1);
    define_cfunc(th, "value", th_value, 0);
    define_cfunc(th, "[]", th_aref, 1);
    define_cfunc(th, "[]=", th_aset, 2);
    define_cfunc(th, "thread_variable_get", th_aref, 1);
    define_cfunc(th, "thread_variable_set", th_aset, 2);
    define_cfunc(th, "alive?", th_alive, 0);
    define_cfunc(th, "name=", |_c, _s, a, _b| Ok(a[0]), 1);
    define_cfunc(th, "report_on_exception=", |_c, _s, a, _b| Ok(a[0]), 1);
    define_cfunc(th, "abort_on_exception=", |_c, _s, a, _b| Ok(a[0]), 1);
    define_singleton_cfunc(th, "abort_on_exception=", |_c, _s, a, _b| Ok(a[0]), 1);
    define_singleton_cfunc(th, "report_on_exception=", |_c, _s, a, _b| Ok(a[0]), 1);
    let mx = define_class_under(th, "Mutex", ob);
    const_set(ob, intern(b"Mutex"), mx);
    vm.c_mutex = mx;
    define_cfunc(mx, "synchronize", mutex_sync, 0);
    define_cfunc(mx, "lock", noop_self, 0);
    define_cfunc(mx, "unlock", noop_self, 0);
    define_cfunc(mx, "try_lock", |_c, _s, _a, _b| Ok(TRUE), 0);
    define_cfunc(mx, "locked?", |_c, _s, _a, _b| Ok(FALSE), 0);
    define_cfunc(mx, "owned?", |_c, _s, _a, _b| Ok(FALSE), 0);
    let mon = define_class("Monitor", ob);
    vm.c_monitor = mon;
    define_cfunc(mon, "synchronize", mutex_sync, 0);
    define_cfunc(mon, "mon_synchronize", mutex_sync, 0);
    define_cfunc(mon, "enter", noop_self, 0);
    define_cfunc(mon, "exit", noop_self, 0);
    let q = define_class_under(th, "Queue", ob);
    const_set(ob, intern(b"Queue"), q);
    vm.c_queue = q;
    define_private_cfunc(q, "initialize", q_init, -1);
    for n in ["push", "<<", "enq"] { define_cfunc(q, n, q_push, 1); }
    for n in ["pop", "shift", "deq"] { define_cfunc(q, n, q_pop, -1); }
    define_cfunc(q, "size", q_size, 0);
    define_cfunc(q, "length", q_size, 0);
    define_cfunc(q, "empty?", q_empty, 0);
    define_cfunc(q, "close", noop_self, 0);
    let cv = define_class_under(th, "ConditionVariable", ob);
    const_set(ob, intern(b"ConditionVariable"), cv);
    define_cfunc(cv, "signal", noop_self, 0);
    define_cfunc(cv, "broadcast", noop_self, 0);
    define_cfunc(cv, "wait", noop_self, -1);
    // Signal / Warning
    let sg = define_module("Signal");
    vm.m_signal = sg;
    define_module_function(sg, "trap", signal_trap, -1);
    define_module_function(vm.m_kernel, "trap", signal_trap, -1);
    let wn = define_module("Warning");
    vm.m_warning = wn;
    define_module_function(wn, "warn", warning_warn, -1);
    define_singleton_cfunc(wn, "[]", |_c, _s, _a, _b| Ok(FALSE), 1);
    define_singleton_cfunc(wn, "[]=", |_c, _s, a, _b| Ok(a[1]), 2);
    // constants
    let set = |n: &str, v: Value| const_set(ob, intern(n.as_bytes()), v);
    set("RUBY_VERSION", str_new_frozen(b"3.4.0"));
    set("RUBY_PATCHLEVEL", Value::fix(0));
    set("RUBY_RELEASE_DATE", str_new_frozen(b"2026-11-20"));
    set("RUBY_PLATFORM", str_new_frozen(b"x86_64-linux"));
    set("RUBY_ENGINE", str_new_frozen(b"koruby"));
    set("RUBY_ENGINE_VERSION", str_new_frozen(b"0.1.0"));
    set("RUBY_COPYRIGHT", str_new_frozen(b"koruby - Copyright (C) 2026 Koichi Sasada"));
    set("RUBY_DESCRIPTION", str_new_frozen(crate::version().as_bytes()));
    set("ARGV", vm.argv);
    set("TOPLEVEL_BINDING", NIL);
    let rbc = define_module("RbConfig");
    let cfg = hash_new_capa(4);
    const_set(rbc, intern(b"CONFIG"), cfg);
    let _ = (intern, num_to_long);
}

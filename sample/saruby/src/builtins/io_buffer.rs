// IO::Buffer: io_buffer.c of CRuby 4.0.2 (behaviour checked against the 4.0.2 binary).
//
// The bytes live in Rust-owned memory: a `Vec<u8>` (INTERNAL), a `sys::Mapping` that unmaps on drop
// (MAPPED), or a String's own buffer (EXTERNAL, `IO::Buffer.for` / `.string`). A slice owns nothing:
// it names its source object (the root buffer or the String, traced by the GC) and keeps the
// address range [base, base+size) it was cut at, as CRuby does. Every access resolves that range
// against the source's *current* memory (`at`): if the source was freed, transferred, or resized so
// that the range no longer lies inside it, the slice is invalid (InvalidatedError), and the bytes
// are then taken with a bounds-checked `get`, so no access can reach memory the source no longer
// owns. Addresses are only compared, never dereferenced. See docs/io_design.md "IO::Buffer".
#![forbid(unsafe_code)]
#[allow(unused_imports)]
use crate::heap::no_legacy::*;   // the old accessors cannot be named here
#[allow(unused_imports)]
use crate::heap::no_legacy_str::*;
use crate::array::*;
use crate::bignum::BigInt;
use crate::builtins::kernel::class_name_for_err;
use crate::builtins::sys;
use crate::heap::{obj_clear_flags, obj_flags, obj_set_flags, Global};
use crate::string::*;
use crate::symbol::{intern, s, sym_bytes};
use crate::value::*;
use crate::vm::*;

pub const EXTERNAL: u32 = 1;
pub const INTERNAL: u32 = 2;
pub const MAPPED: u32 = 4;
pub const SHARED: u32 = 8;
pub const LOCKED: u32 = 32;
pub const PRIVATE: u32 = 64;
pub const READONLY: u32 = 128;
pub const FILE: u32 = 256;
const FLAGS_MASK: u32 = EXTERNAL | INTERNAL | MAPPED | SHARED | LOCKED | PRIVATE | READONLY;

static CLASS: Global = Global::new();
static E_LOCKED: Global = Global::new();
static E_ALLOC: Global = Global::new();
static E_ACCESS: Global = Global::new();
static E_INVALID: Global = Global::new();
static E_MASK: Global = Global::new();
static WARNED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static PAGE_SIZE: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(4096);
static DEFAULT_SIZE: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(65536);

enum Mem {
    None,
    Heap(Vec<u8>),
    /// A mapping and how many of its bytes are backed by the file (usize::MAX: anonymous), so a
    /// resize never extends a private file mapping past the end of the file (SIGBUS on access).
    Map(sys::Mapping, usize),
}
impl Mem {
    fn bytes(&self) -> &[u8] { match self { Mem::None => &[], Mem::Heap(v) => v, Mem::Map(m, _) => m.bytes() } }
    fn bytes_mut(&mut self) -> Option<&mut [u8]> { match self { Mem::None => Some(&mut []), Mem::Heap(v) => Some(v), Mem::Map(m, _) => m.bytes_mut() } }
    fn addr(&self) -> usize { match self { Mem::None => 0, Mem::Heap(v) => if v.is_empty() { 0 } else { v.as_ptr() as usize }, Mem::Map(m, _) => m.addr() } }
}

/// The payload of an IO::Buffer (struct rb_io_buffer).
pub struct RBuf {
    mem: Mem,        // the memory this buffer owns (none for EXTERNAL buffers and slices)
    base: usize,     // address of the first byte, 0 for NULL: compared, never dereferenced
    size: usize,
    flags: u32,
    source: Value,   // NIL, the String of .for / .string, or the root buffer of a slice
}
impl RData for RBuf {
    fn visit(&mut self, m: &mut crate::gc::Visitor) { m.visit(&mut self.source); }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}
fn empty() -> RBuf { RBuf { mem: Mem::None, base: 0, size: 0, flags: 0, source: NIL } }

fn nb() -> Blk<'static> { Blk::NONE }
fn buf(c: &Ctx, v: Value) -> &RBuf { c.data::<RBuf>(v).expect("an IO::Buffer") }
fn buf_mut(c: &mut Ctx, v: Value) -> &mut RBuf { c.data_mut::<RBuf>(v).expect("an IO::Buffer") }
/// TypedData_Get_Struct: TypeError unless `v` is an IO::Buffer.
fn check(c: &mut Ctx, v: Value) -> Result<(), Esc> {
    if c.data::<RBuf>(v).is_some() { return Ok(()); }
    type_err(c, format!("wrong argument type {} (expected IO::Buffer)", class_name_for_err(c, v)))
}
fn raise<T>(c: &mut Ctx, k: &Global, msg: &str) -> Result<T, Esc> { let k = k.get(c); err(c, k, msg) }

// ---- where the bytes are --------------------------------------------------------------------

#[derive(Clone, Copy)]
enum Owner { Buf(Value), Str(Value) }
/// A buffer's bytes, resolved: invalid, NULL (no bytes), or `size` bytes at an offset of the owner's bytes.
#[derive(Clone, Copy)]
enum At { Invalid, Null, Bytes(Owner, usize) }

/// io_buffer_validate + rb_io_buffer_get_bytes, by address range against the source's current memory.
fn at(c: &Ctx, v: Value) -> At { at_depth(c, v, 0) }
fn at_depth(c: &Ctx, v: Value, depth: u32) -> At {
    let Some(b) = c.data::<RBuf>(v) else { return At::Invalid };
    let (base, size, src) = (b.base, b.size, b.source);
    if src == NIL { return if base == 0 { At::Null } else { At::Bytes(Owner::Buf(v), 0) }; }
    // the source's visible range [sb, sb+sl) and where it lies in the owner
    let (sb, sl, owner, ooff) = if is_string(src) {
        let s = c.str_bytes(src);
        (s.as_ptr() as usize, s.len(), Owner::Str(src), 0)
    } else {
        if depth > 4 { return At::Invalid; }
        let Some(r) = c.data::<RBuf>(src) else { return At::Invalid };
        let (rb, rl) = (r.base, r.size);
        match at_depth(c, src, depth + 1) { At::Bytes(o, off) => (rb, rl, o, off), _ => return At::Invalid }
    };
    if base < sb || base.checked_add(size).is_none_or(|e| e > sb + sl) { return At::Invalid; }
    At::Bytes(owner, ooff + (base - sb))
}
fn valid(c: &Ctx, v: Value) -> bool { !matches!(at(c, v), At::Invalid) }
/// The `size` bytes at `a`; None if the owner no longer has them (treated as invalid).
fn bytes_at(c: &Ctx, a: At, size: usize) -> Option<&[u8]> {
    match a {
        At::Invalid => None,
        At::Null => Some(&[]),
        At::Bytes(Owner::Buf(r), off) => c.data::<RBuf>(r)?.mem.bytes().get(off..off.checked_add(size)?),
        At::Bytes(Owner::Str(s), off) => c.str_bytes(s).get(off..off.checked_add(size)?),
    }
}
fn bytes_at_mut(c: &mut Ctx, a: At, size: usize) -> Option<&mut [u8]> {
    match a {
        At::Invalid => None,
        At::Null => Some(&mut []),
        At::Bytes(Owner::Buf(r), off) => c.data_mut::<RBuf>(r)?.mem.bytes_mut()?.get_mut(off..off.checked_add(size)?),
        At::Bytes(Owner::Str(s), off) => { let b: &mut [u8] = &mut c.rstring_mut(s).b; b.get_mut(off..off.checked_add(size)?) }
    }
}
/// io_buffer_get_bytes_for_reading: (location, size); InvalidatedError for an invalid slice.
fn for_reading(c: &mut Ctx, v: Value) -> Result<(At, usize), Esc> {
    let a = at(c, v);
    let size = buf(c, v).size;
    match a {
        At::Invalid => raise(c, &E_INVALID, "Buffer has been invalidated!"),
        At::Null => Ok((a, 0)),
        _ if bytes_at(c, a, size).is_none() => raise(c, &E_INVALID, "Buffer has been invalidated!"),
        _ => Ok((a, size)),
    }
}
/// io_buffer_validate_for_writing + get_bytes_for_writing.
fn for_writing(c: &mut Ctx, v: Value) -> Result<(At, usize), Esc> {
    let (flags, src) = { let b = buf(c, v); (b.flags, b.source) };
    if flags & READONLY != 0 || (src != NIL && is_frozen(src)) { return raise(c, &E_ACCESS, "Buffer is not writable!"); }
    let a = at(c, v);
    let size = buf(c, v).size;
    match a {
        At::Invalid => raise(c, &E_INVALID, "Buffer is invalid!"),
        At::Null => Ok((a, 0)),
        _ if bytes_at(c, a, size).is_none() => raise(c, &E_INVALID, "Buffer is invalid!"),
        _ => Ok((a, size)),
    }
}
/// Bytes checked by for_reading (no call in between).
fn rd(c: &Ctx, a: At, size: usize) -> &[u8] { bytes_at(c, a, size).unwrap_or(&[]) }
/// Bytes checked by for_writing; AccessError for memory mapped without PROT_WRITE.
fn wr(c: &mut Ctx, a: At, size: usize) -> Result<&mut [u8], Esc> {
    if bytes_at_mut(c, a, size).is_none() { return raise(c, &E_ACCESS, "Buffer is not writable!"); }
    Ok(bytes_at_mut(c, a, size).unwrap_or_default())
}
/// io_buffer_validate_range (4.0.10: an invalidated buffer first, then the range).
fn validate_range(c: &mut Ctx, v: Value, off: usize, len: usize) -> Result<(), Esc> {
    for_reading(c, v)?;
    if off.checked_add(len).is_none_or(|e| e > buf(c, v).size) { return arg_err(c, "Specified offset+length is bigger than the buffer size!"); }
    Ok(())
}

// ---- allocation -----------------------------------------------------------------------------

fn experimental(c: &mut Ctx) -> Result<(), Esc> {
    if WARNED.swap(true, std::sync::atomic::Ordering::Relaxed) { return Ok(()); }
    crate::builtins::kernel::category_warn(c, "IO::Buffer is experimental and both the Ruby and C interface may change in the future!", 1, "experimental")
}
fn new_instance(c: &mut Ctx, klass: Value) -> RV {
    experimental(c)?;
    Ok(c.alloc(klass, Body::Data(Box::new(empty()))))
}
/// A locked buffer over `bytes` (rb_io_buffer_new(base, size, LOCKED[|READONLY])): what a Fiber
/// scheduler's io_read / io_write is given.  Taken back with `take_locked`.
pub fn new_locked(c: &mut Ctx, bytes: Vec<u8>, readonly: bool) -> RV {
    if CLASS.get(c) == NIL { let io = c.vm().c_io.get(); funcall(c, io, intern(b"const_get"), &[sym_value("Buffer")], nb())?; }
    let k = CLASS.get(c);
    let size = bytes.len();
    let mem = if size == 0 { Mem::None } else { Mem::Heap(bytes) };
    let base = mem.addr();
    let flags = (if size == 0 { 0 } else { INTERNAL }) | LOCKED | if readonly { READONLY } else { 0 };
    Ok(c.alloc(k, Body::Data(Box::new(RBuf { mem, base, size, flags, source: NIL }))))
}
/// The bytes of a `new_locked` buffer; the buffer is left null (rb_io_buffer_free_locked).
pub fn take_locked(c: &mut Ctx, v: Value) -> Vec<u8> {
    let Some(b) = c.data_mut::<RBuf>(v) else { return vec![] };
    let m = std::mem::replace(&mut b.mem, Mem::None);
    b.base = 0; b.size = 0; b.flags = 0;
    match m { Mem::Heap(v) => v, _ => vec![] }
}
pub fn alloc_data(c: &mut Ctx, klass: Value) -> Result<Option<Value>, Esc> {
    let k = CLASS.get(c);
    if k != NIL && crate::builtins::inherits(c, klass, k) { return Ok(Some(new_instance(c, klass)?)); }
    Ok(None)
}
pub fn dup_data(c: &mut Ctx, v: Value) -> Result<Option<Value>, Esc> {
    if c.data::<RBuf>(v).is_none() { return Ok(None); }
    let k = real_class_of(c, v);
    Ok(Some(new_instance(c, k)?))
}
fn flags_for_size(size: usize) -> u32 { if size >= PAGE_SIZE.load(std::sync::atomic::Ordering::Relaxed) { MAPPED } else { INTERNAL } }
/// Zero-filled memory of `size` > 0 bytes as `flags` asks (calloc or an anonymous mapping).
fn allocate(c: &mut Ctx, size: usize, flags: u32) -> Result<Mem, Esc> {
    if flags & INTERNAL != 0 {
        let mut v = Vec::new();
        if v.try_reserve_exact(size).is_err() { return raise(c, &E_ALLOC, "Could not allocate buffer!"); }
        v.resize(size, 0);
        c.malloc_increase(size);
        return Ok(Mem::Heap(v));
    }
    if flags & MAPPED != 0 {
        return match sys::Mapping::new(size, true, flags & SHARED != 0, -1, 0) {
            Ok(m) => Ok(Mem::Map(m, usize::MAX)),
            Err(e) => Err(crate::builtins::exception::raise_errno_plain(c, e, "io_buffer_map_memory:mmap")),
        };
    }
    raise(c, &E_ALLOC, "Could not allocate buffer!")
}
/// io_buffer_initialize with no base: a size of 0 leaves the buffer as it is.
fn initialize_with(c: &mut Ctx, v: Value, size: usize, flags: u32) -> Result<(), Esc> {
    if size == 0 { return Ok(()); }
    let mem = allocate(c, size, flags)?;
    let b = buf_mut(c, v);
    b.base = mem.addr(); b.mem = mem; b.size = size; b.flags = flags; b.source = NIL;
    Ok(())
}
/// io_buffer_free.
fn free_mem(b: &mut RBuf) {
    if b.base != 0 { *b = empty(); }
}

// ---- argument conversion (CRuby's NUM2* family, messages included) --------------------------

fn not_integer<T>(c: &mut Ctx) -> Result<T, Esc> { type_err(c, "not an Integer") }
fn is_neg(c: &Ctx, v: Value) -> bool { if v.is_fixnum() { v.as_fix() < 0 } else { c.big(v).neg } }
/// The magnitude of Integer `v` if it fits in 128 bits.
fn mag128(c: &Ctx, v: Value) -> Option<u128> {
    if v.is_fixnum() { return Some(v.as_fix().unsigned_abs() as u128); }
    let m = &c.big(v).mag;
    if m.len() > 4 { return None; }
    Some(m.iter().rev().fold(0u128, |a, &w| (a << 32) | w as u128))
}
/// io_buffer_extract_{offset,length,size,count,width}: a non-negative Integer as size_t.
fn extract(c: &mut Ctx, v: Value, what: &str) -> Result<usize, Esc> {
    if !is_integer(v) { return not_integer(c); }
    if is_neg(c, v) { return arg_err(c, format!("{} can't be negative!", what)); }
    match mag128(c, v) {
        Some(m) if m <= u64::MAX as u128 => Ok(m as usize),
        _ => err(c, c.vm().c_range_error.get(), "bignum too big to convert into 'unsigned long long'"),
    }
}
fn extract_flags(c: &mut Ctx, v: Value) -> Result<u32, Esc> {
    if !is_integer(v) { return not_integer(c); }
    if is_neg(c, v) { return arg_err(c, "Flags can't be negative!"); }
    Ok(num2uint(c, v)? & FLAGS_MASK)
}
/// io_buffer_extract_offset_length / _length_offset: Some(i) is argument i (nil counts as absent).
fn opt(a: &[Value], i: usize) -> Option<Value> { a.get(i).copied().filter(|&v| v != NIL) }
fn default_length(c: &mut Ctx, v: Value, off: usize) -> Result<usize, Esc> {
    let size = buf(c, v).size;
    if off > size { return arg_err(c, "The given offset is bigger than the buffer size!"); }
    Ok(size - off)
}
fn offset_length(c: &mut Ctx, v: Value, a: &[Value]) -> Result<(usize, usize), Esc> {
    let off = match opt(a, 0) { Some(x) => extract(c, x, "Offset")?, None => 0 };
    let len = match opt(a, 1) { Some(x) => extract(c, x, "Length")?, None => default_length(c, v, off)? };
    Ok((off, len))
}
fn length_offset(c: &mut Ctx, v: Value, a: &[Value]) -> Result<(usize, usize), Esc> {
    let off = match opt(a, 1) { Some(x) => extract(c, x, "Offset")?, None => 0 };
    let len = match opt(a, 0) { Some(x) => extract(c, x, "Length")?, None => default_length(c, v, off)? };
    Ok((len, off))
}

fn range_err<T>(c: &mut Ctx, msg: String) -> Result<T, Esc> { err(c, c.vm().c_range_error.get(), msg) }
/// "%-.10g" of rb_out_of_range-style messages.
fn g10(f: f64) -> String {
    if f.is_nan() { "NaN".into() } else if f.is_infinite() { if f < 0.0 { "-Inf".into() } else { "Inf".into() } } else { crate::builtins::sprintf::fmt_g10(f) }
}
/// rb_to_int.
fn to_int(c: &mut Ctx, v: Value) -> RV {
    if is_integer(v) { return Ok(v); }
    if v.is_heap() && respond_to(c, v, s::to_int, true) {
        let r = funcall(c, v, s::to_int, &[], nb())?;
        if is_integer(r) { return Ok(r); }
        let k = class_name_for_err(c, v);
        return type_err(c, format!("can't convert {} to Integer ({}#to_int gives {})", k, k, class_name_for_err(c, r)));
    }
    type_err(c, format!("no implicit conversion of {} into Integer", class_name_for_err(c, v)))
}
/// rb_num2long.
fn num2long(c: &mut Ctx, mut v: Value) -> Result<i64, Esc> {
    loop {
        if v == NIL { return type_err(c, "no implicit conversion from nil to integer"); }
        if v.is_fixnum() { return Ok(v.as_fix()); }
        if is_float(v) {
            let f = float_val(v);
            if f < 9223372036854775808.0 && f >= -9223372036854775808.0 { return Ok(f as i64); }
            return range_err(c, format!("float {} out of range of integer", g10(f)));
        }
        if is_bignum(v) { return match c.big(v).to_i64() { Some(i) => Ok(i), None => range_err(c, "bignum too big to convert into 'long'".into()) }; }
        v = to_int(c, v)?;
    }
}
/// rb_num2int.
fn num2int(c: &mut Ctx, v: Value) -> Result<i32, Esc> {
    let n = num2long(c, v)?;
    if n > i32::MAX as i64 { return range_err(c, format!("integer {} too big to convert to 'int'", n)); }
    if n < i32::MIN as i64 { return range_err(c, format!("integer {} too small to convert to 'int'", n)); }
    Ok(n as i32)
}
/// rb_big2ulong / rb_big2ull: two's-complement wrap for negatives down to -2**63.
fn big2u64(c: &mut Ctx, v: Value, ty: &str) -> Result<u64, Esc> {
    let neg = c.big(v).neg;
    match mag128(c, v) {
        Some(m) if m <= u64::MAX as u128 => {
            if !neg { return Ok(m as u64); }
            if m <= 1u128 << 63 { return Ok((m as u64).wrapping_neg()); }
            range_err(c, format!("bignum out of range of {}", ty))
        }
        _ => range_err(c, format!("bignum too big to convert into '{}'", ty)),
    }
}
/// rb_num2ulong_internal: (value, negative).
fn num2ulong(c: &mut Ctx, mut v: Value) -> Result<(u64, bool), Esc> {
    loop {
        if v == NIL { return type_err(c, "no implicit conversion of nil into Integer"); }
        if v.is_fixnum() { let n = v.as_fix(); return Ok((n as u64, n < 0)); }
        if is_float(v) {
            let f = float_val(v);
            if f < 18446744073709551616.0 && f >= -9223372036854775808.0 {
                return Ok((if f >= 0.0 { f as u64 } else { f as i64 as u64 }, f <= -1.0));
            }
            return range_err(c, format!("float {} out of range of integer", g10(f)));
        }
        if is_bignum(v) { let neg = c.big(v).neg; return Ok((big2u64(c, v, "unsigned long")?, neg)); }
        v = to_int(c, v)?;
    }
}
/// rb_num2uint (check_uint).
fn num2uint(c: &mut Ctx, v: Value) -> Result<u32, Esc> {
    let (n, wrap) = num2ulong(c, v)?;
    if wrap {
        if n < i32::MIN as i64 as u64 { return range_err(c, format!("integer {} too small to convert to 'unsigned int'", n as i64)); }
    } else if n > u32::MAX as u64 { return range_err(c, format!("integer {} too big to convert to 'unsigned int'", n)); }
    Ok(n as u32)
}
/// rb_num2ll's non-numeric cases.
fn ll_other(c: &mut Ctx, v: Value) -> RV {
    if v == NIL { return type_err(c, "no implicit conversion from nil"); }
    if is_string(v) { return type_err(c, "no implicit conversion from string"); }
    if v == TRUE || v == FALSE { return type_err(c, "no implicit conversion from boolean"); }
    to_int(c, v)
}
/// rb_num2ll.
fn num2ll(c: &mut Ctx, mut v: Value) -> Result<i64, Esc> {
    loop {
        if v.is_fixnum() { return Ok(v.as_fix()); }
        if is_float(v) {
            let f = float_val(v);
            if f < 9223372036854775808.0 && f >= -9223372036854775808.0 { return Ok(f as i64); }
            return range_err(c, format!("float {} out of range of long long", g10(f)));
        }
        if is_bignum(v) { return match c.big(v).to_i64() { Some(i) => Ok(i), None => range_err(c, "bignum too big to convert into 'long long'".into()) }; }
        v = ll_other(c, v)?;
    }
}
/// rb_num2ull.
fn num2ull(c: &mut Ctx, mut v: Value) -> Result<u64, Esc> {
    loop {
        if v.is_fixnum() { return Ok(v.as_fix() as u64); }
        if is_float(v) {
            let f = float_val(v);
            if f < 18446744073709551616.0 && f >= -9223372036854775808.0 { return Ok(if f >= 0.0 { f as u64 } else { f as i64 as u64 }); }
            return range_err(c, format!("float {} out of range of unsigned long long", g10(f)));
        }
        if is_bignum(v) { return big2u64(c, v, "unsigned long long"); }
        v = to_int(c, v)?;
    }
}
/// rb_numeric_to_uint128 / rb_numeric_to_int128.
fn num2u128(c: &mut Ctx, v: Value) -> Result<u128, Esc> {
    if !is_integer(v) { return type_err(c, "not an integer"); }
    if is_neg(c, v) { return range_err(c, "negative integer cannot be converted to unsigned 128-bit integer".into()); }
    match mag128(c, v) { Some(m) => Ok(m), None => range_err(c, "bignum too big to convert into 'unsigned 128-bit integer'".into()) }
}
fn num2i128(c: &mut Ctx, v: Value) -> Result<i128, Esc> {
    if !is_integer(v) { return type_err(c, "not an integer"); }
    let neg = is_neg(c, v);
    match mag128(c, v) {
        Some(m) if !neg && m <= i128::MAX as u128 => Ok(m as i128),
        Some(m) if neg && m <= 1u128 << 127 => Ok((m as i128).wrapping_neg()),
        _ => range_err(c, "bignum too big to convert into 'signed 128-bit integer'".into()),
    }
}
use crate::builtins::numeric_methods::num2dbl;
fn u64_value(c: &mut Ctx, u: u64) -> Value { if u <= i64::MAX as u64 { c.int_from_i64(u as i64) } else { c.big_to_value(BigInt::from_u64(u)) } }
fn u128_value(c: &mut Ctx, u: u128) -> Value {
    if u <= i64::MAX as u128 { return c.int_from_i64(u as i64); }
    c.big_to_value(BigInt::from_words(false, &[u as u32, (u >> 32) as u32, (u >> 64) as u32, (u >> 96) as u32]))
}

// ---- data types -----------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq)]
enum Kind { U8, S8, U16, S16, U32, S32, U64, S64, U128, S128, F32, F64 }
#[derive(Clone, Copy)]
struct Ty { k: Kind, be: bool }
/// The type of a name (:U8 .. :F64); a match, not a table of slices: no load-time relocations.
fn ty_named(n: &[u8]) -> Option<Ty> {
    let (k, be) = match n {
        b"U8" => (Kind::U8, true), b"S8" => (Kind::S8, true),
        b"u16" => (Kind::U16, false), b"U16" => (Kind::U16, true), b"s16" => (Kind::S16, false), b"S16" => (Kind::S16, true),
        b"u32" => (Kind::U32, false), b"U32" => (Kind::U32, true), b"s32" => (Kind::S32, false), b"S32" => (Kind::S32, true),
        b"u64" => (Kind::U64, false), b"U64" => (Kind::U64, true), b"s64" => (Kind::S64, false), b"S64" => (Kind::S64, true),
        b"u128" => (Kind::U128, false), b"U128" => (Kind::U128, true), b"s128" => (Kind::S128, false), b"S128" => (Kind::S128, true),
        b"f32" => (Kind::F32, false), b"F32" => (Kind::F32, true), b"f64" => (Kind::F64, false), b"F64" => (Kind::F64, true),
        _ => return None,
    };
    Some(Ty { k, be })
}
impl Ty {
    fn size(self) -> usize {
        match self.k { Kind::U8 | Kind::S8 => 1, Kind::U16 | Kind::S16 => 2, Kind::U32 | Kind::S32 | Kind::F32 => 4, Kind::U64 | Kind::S64 | Kind::F64 => 8, Kind::U128 | Kind::S128 => 16 }
    }
}
/// RB_SYM2ID: TypeError for a non-Symbol.
fn sym_of(c: &mut Ctx, v: Value) -> Result<Sym, Esc> {
    if v.is_sym() { return Ok(v.as_sym()); }
    type_err(c, format!("wrong argument type {} (expected Symbol)", class_name_for_err(c, v)))
}
fn ty_of_sym(c: &mut Ctx, id: Sym) -> Result<Ty, Esc> {
    let n = sym_bytes(id);
    match ty_named(n) { Some(t) => Ok(t), None => arg_err(c, "Invalid type name!") }
}
fn ty_of(c: &mut Ctx, v: Value) -> Result<Ty, Esc> { let id = sym_of(c, v)?; ty_of_sym(c, id) }
fn validate_type(c: &mut Ctx, size: usize, off: usize, n: usize) -> Result<(), Esc> {
    if off.checked_add(n).is_none_or(|e| e > size) {
        return arg_err(c, format!("Type extends beyond end of buffer! (offset={} > size={})", off as isize, size as isize));
    }
    Ok(())
}
/// io_buffer_read_<type> on bytes `b` (the whole buffer).
fn decode(c: &mut Ctx, t: Ty, b: &[u8], off: usize) -> RV {
    let n = t.size();
    validate_type(c, b.len(), off, n)?;
    let mut raw = [0u8; 16];
    raw[..n].copy_from_slice(&b[off..off + n]);
    if t.be { raw[..n].reverse(); }
    let le8 = u64::from_le_bytes([raw[0], raw[1], raw[2], raw[3], raw[4], raw[5], raw[6], raw[7]]);
    Ok(match t.k {
        Kind::U8 => Value::fix(raw[0] as i64),
        Kind::S8 => Value::fix(raw[0] as i8 as i64),
        Kind::U16 => Value::fix(le8 as u16 as i64),
        Kind::S16 => Value::fix(le8 as u16 as i16 as i64),
        Kind::U32 => Value::fix(le8 as u32 as i64),
        Kind::S32 => Value::fix(le8 as u32 as i32 as i64),
        Kind::U64 => u64_value(c, le8),
        Kind::S64 => c.int_from_i64(le8 as i64),
        Kind::U128 => u128_value(c, u128::from_le_bytes(raw)),
        Kind::S128 => c.int_from_i128(i128::from_le_bytes(raw)),
        Kind::F32 => c.float_new(f32::from_bits(le8 as u32) as f64),
        Kind::F64 => c.float_new(f64::from_bits(le8)),
    })
}
/// The unwrap of io_buffer_write_<type>: `v` as the type's bytes (may run Ruby code: to_int / to_f).
fn encode(c: &mut Ctx, t: Ty, v: Value) -> Result<[u8; 16], Esc> {
    let le: u128 = match t.k {
        Kind::U8 | Kind::U16 | Kind::U32 => num2uint(c, v)? as u128,
        Kind::S8 | Kind::S16 | Kind::S32 => num2int(c, v)? as u32 as u128,
        Kind::U64 => num2ull(c, v)? as u128,
        Kind::S64 => num2ll(c, v)? as u64 as u128,
        Kind::U128 => num2u128(c, v)?,
        Kind::S128 => num2i128(c, v)? as u128,
        Kind::F32 => (num2dbl(c, v)? as f32).to_bits() as u128,
        Kind::F64 => num2dbl(c, v)?.to_bits() as u128,
    };
    let mut raw = le.to_le_bytes();
    let n = t.size();
    if t.be { raw[..n].reverse(); }
    Ok(raw)
}
/// rb_io_buffer_set_value at *off: validate, convert, validate again (the conversion can run Ruby
/// code that frees or resizes the buffer), write.
fn set_one(c: &mut Ctx, slf: Value, tv: Value, off: &mut usize, v: Value) -> Result<(), Esc> {
    let t = ty_of(c, tv)?;
    let n = t.size();
    let (_, size) = for_writing(c, slf)?;
    validate_type(c, size, *off, n)?;
    let raw = encode(c, t, v)?;
    let (a, size) = for_writing(c, slf)?;
    validate_type(c, size, *off, n)?;
    wr(c, a, size)?[*off..*off + n].copy_from_slice(&raw[..n]);
    *off += n;
    Ok(())
}

// ---- class methods --------------------------------------------------------------------------

fn s_size_of(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    if is_array(a[0]) {
        let mut total = 0usize;
        let mut i = 0;
        while let Some(&x) = c.ary(a[0]).get(i) { total += ty_of(c, x)?.size(); i += 1; }
        return Ok(Value::fix(total as i64));
    }
    Ok(Value::fix(ty_of(c, a[0])?.size() as i64))
}

/// io_buffer_for_make_instance: a buffer on the String's own bytes.
fn for_instance(c: &mut Ctx, klass: Value, s: Value, mut flags: u32) -> RV {
    let inst = new_instance(c, klass)?;
    flags |= EXTERNAL;
    if is_frozen(s) { flags |= READONLY; }
    if flags & READONLY == 0 { c.str_modify(s)?; }
    let (base, size) = { let b = c.str_bytes(s); (b.as_ptr() as usize, b.len()) };
    let b = buf_mut(c, inst);
    b.base = base; b.size = size; b.flags = flags; b.source = s;
    Ok(inst)
}
/// io_buffer_for_yield_instance with its ensure: the String is locked while the block runs.
fn for_yield(c: &mut Ctx, klass: Value, s: Value, blk: Blk) -> RV {
    let r = (|| -> RV {
        let inst = for_instance(c, klass, s, 0)?;
        if !is_frozen(s) {
            if obj_flags(s) & FL_TMPLOCK != 0 { return err(c, c.vm().c_runtime_error.get(), "temporal locking already locked string"); }
            obj_set_flags(s, FL_TMPLOCK);
        }
        let r = if blk.is_null() { err(c, c.vm().c_local_jump_error.get(), "no block given") } else { yield1(c, blk, inst) };
        Ok(match r { Ok(v) => { free_checked(c, inst)?; v } Err(e) => { free_checked(c, inst)?; return Err(e); } })
    })();
    if !is_frozen(s) {
        if obj_flags(s) & FL_TMPLOCK == 0 { return err(c, c.vm().c_runtime_error.get(), "temporal unlocking already unlocked string"); }
        obj_clear_flags(s, FL_TMPLOCK);
    }
    r
}
fn s_for(c: &mut Ctx, slf: Value, a: &[Value], blk: Blk) -> RV {
    let s = expect_str(c, a[0])?;
    if !blk.is_null() { return for_yield(c, slf, s, blk); }
    // rb_str_tmp_frozen_acquire: a frozen String as is, else a frozen copy
    let s = if is_frozen(s) { s } else { let t = c.str_copy(s); obj_set_flags(t, FL_FROZEN); t };
    for_instance(c, slf, s, READONLY)
}
fn s_string(c: &mut Ctx, slf: Value, a: &[Value], blk: Blk) -> RV {
    let n = num2long(c, a[0])?;
    if n < 0 { return arg_err(c, "negative string size (or size too big)"); }
    let mut v = Vec::new();
    if v.try_reserve_exact(n as usize).is_err() { return err(c, c.vm().c_no_memory_error.get(), "failed to allocate memory"); }
    v.resize(n as usize, 0);
    let s = c.str_from_vec_enc(v, ENC_BINARY);
    for_yield(c, slf, s, blk)?;
    Ok(s)
}

/// rb_file_size of the IO and its descriptor.
fn map_target(c: &mut Ctx, io: Value) -> Result<(i64, i32), Esc> {
    if crate::builtins::io::is_io(c, io) {
        let r = crate::builtins::io::get_io(c, io)?;
        if r.writable && !r.wbuf.is_empty() { crate::builtins::io::flush_one(r); }
        let fd = r.fd;
        return match sys::fd_metadata(fd) {
            Ok(m) => Ok((m.len() as i64, fd)),
            Err(e) => Err(crate::builtins::exception::raise_errno_plain(c, crate::builtins::exception::io_err_to_errno(&e), "fstat")),
        };
    }
    let sz = funcall(c, io, intern(b"size"), &[], nb())?;
    let sz = num2long(c, sz)?;
    // rb_io_descriptor of a non-IO: #fileno
    let fileno = intern(b"fileno");
    if !respond_to(c, io, fileno, true) { return type_err(c, format!("expected IO or #fileno, {} given", class_name_for_err(c, io))); }
    let fd = funcall(c, io, fileno, &[], nb())?;
    Ok((sz, num2int(c, fd)?))
}
fn s_map(c: &mut Ctx, _slf: Value, a: &[Value], _b: Blk) -> RV {
    check_argc(c, a, 1, 4)?;
    let (file_size, fd) = map_target(c, a[0])?;
    if file_size <= 0 { return arg_err(c, "Invalid negative or zero file size!"); }
    let mut size = match opt(a, 1) {
        Some(x) => {
            let n = extract(c, x, "Size")?;
            if n == 0 { return arg_err(c, "Size can't be zero!"); }
            if n > file_size as usize { return arg_err(c, "Size can't be larger than file size!"); }
            n
        }
        None => file_size as usize,
    };
    let mut offset = 0i64;
    if a.len() >= 3 {
        offset = num2long(c, a[2])?;
        if offset < 0 { return arg_err(c, "Offset can't be negative!"); }
        if offset >= file_size { return arg_err(c, "Offset too large!"); }
        if a[1] == NIL { size = (file_size - offset) as usize; }
        else if ((file_size - offset) as usize) < size { return arg_err(c, "Offset too large!"); }
    }
    let req = if a.len() >= 4 { extract_flags(c, a[3])? } else { 0 };
    let mut flags = MAPPED | FILE;
    if req & READONLY != 0 { flags |= READONLY; }
    if req & PRIVATE != 0 { flags |= PRIVATE; } else { flags |= EXTERNAL | SHARED; }
    let m = match sys::Mapping::new(size, req & READONLY == 0, req & PRIVATE == 0, fd, offset) {
        Ok(m) => m,
        Err(e) => return Err(crate::builtins::exception::raise_errno_plain(c, e, "io_buffer_map_file:mmap")),
    };
    let k = CLASS.get(c);
    let inst = new_instance(c, k)?;
    let b = buf_mut(c, inst);
    b.base = m.addr(); b.mem = Mem::Map(m, size); b.size = size; b.flags = flags;
    Ok(inst)
}

// ---- instance methods -----------------------------------------------------------------------

fn b_initialize(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    check_argc(c, a, 0, 2)?;
    let size = match a.first() { Some(&x) => extract(c, x, "Size")?, None => DEFAULT_SIZE.load(std::sync::atomic::Ordering::Relaxed) };
    let flags = if a.len() >= 2 { extract_flags(c, a[1])? } else { flags_for_size(size) };
    initialize_with(c, slf, size, flags)?;
    Ok(slf)
}
fn b_init_copy(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    check(c, a[0])?;
    let (sa, ssize) = for_reading(c, a[0])?;
    let src = rd(c, sa, ssize).to_vec();
    initialize_with(c, slf, ssize, flags_for_size(ssize))?;
    copy_in(c, slf, &src, &[])
}

fn hexdump_into(out: &mut String, width: usize, b: &[u8], mut off: usize, end: usize, mut first: bool) {
    use std::fmt::Write;
    while off < end {
        if first { let _ = write!(out, "0x{:08x} ", off); first = false; } else { let _ = write!(out, "\n0x{:08x} ", off); }
        let mut text = String::new();
        for i in 0..width {
            if off + i < end {
                let v = b[off + i];
                text.push(if v < 127 && (0x20..0x7f).contains(&v) { v as char } else { '.' });
                let _ = write!(out, " {:02x}", v);
            } else { out.push_str("   "); }
        }
        out.push(' ');
        out.push_str(&text);
        off = off.saturating_add(width);
    }
}
fn to_s_string(c: &Ctx, slf: Value) -> String {
    use std::fmt::Write;
    let b = buf(c, slf);
    let mut s = format!("#<{}", class_path(c, real_class_of(c, slf)));
    let _ = write!(s, " 0x{:016x}+{}", b.base, b.size as isize);
    if b.base == 0 { s.push_str(" NULL"); }
    let mut flag = |f: u32, n: &str| if b.flags & f != 0 { s.push(' '); s.push_str(n); };
    flag(EXTERNAL, "EXTERNAL"); flag(INTERNAL, "INTERNAL"); flag(MAPPED, "MAPPED"); flag(FILE, "FILE");
    flag(SHARED, "SHARED"); flag(LOCKED, "LOCKED"); flag(PRIVATE, "PRIVATE"); flag(READONLY, "READONLY");
    if b.source != NIL { s.push_str(" SLICE"); }
    if !valid(c, slf) { s.push_str(" INVALID"); }
    s.push('>');
    s
}
fn b_to_s(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    let s = to_s_string(c, slf);
    Ok(c.str_new_enc(s.as_bytes(), ENC_BINARY))
}
fn b_inspect(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    let mut s = to_s_string(c, slf);
    let a = at(c, slf);
    let size = buf(c, slf).size;
    if let Some(b) = bytes_at(c, a, size) {
        let n = b.len().min(256);
        hexdump_into(&mut s, 16, b, 0, n, false);
        if size > 256 { s.push_str(&format!("\n(and {} more bytes not printed)", size - 256)); }
    }
    Ok(c.str_new_enc(s.as_bytes(), ENC_BINARY))
}
fn b_hexdump(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    check_argc(c, a, 0, 3)?;
    let (off, len) = offset_length(c, slf, a)?;
    let width = if a.len() >= 3 {
        let w = extract(c, a[2], "Width")?;
        if w < 1 { return arg_err(c, "Width must be at least 1!"); }
        if w > 1024 { return arg_err(c, "Width must be at most 1024!"); }
        w
    } else { 16 };
    validate_range(c, slf, off, len)?;
    let size = buf(c, slf).size;
    let at = at(c, slf);
    if buf(c, slf).base == 0 { return Ok(NIL); }
    let Some(b) = bytes_at(c, at, size) else { return Ok(NIL) };
    let mut s = String::new();
    hexdump_into(&mut s, width, b, off, off + len, true);
    Ok(c.str_new_enc(s.as_bytes(), ENC_BINARY))
}
fn b_size(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { let n = buf(c, slf).size; Ok(u64_value(c, n as u64)) }
fn b_valid_p(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { Ok(Value::bool(valid(c, slf))) }
fn b_null_p(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { Ok(Value::bool(buf(c, slf).base == 0)) }
fn b_empty_p(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { Ok(Value::bool(buf(c, slf).size == 0)) }
fn flag_p(c: &Ctx, slf: Value, f: u32) -> RV { Ok(Value::bool(buf(c, slf).flags & f != 0)) }
fn b_external_p(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { flag_p(c, slf, EXTERNAL) }
fn b_internal_p(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { flag_p(c, slf, INTERNAL) }
fn b_mapped_p(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { flag_p(c, slf, MAPPED) }
fn b_shared_p(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { flag_p(c, slf, SHARED) }
fn b_locked_p(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { flag_p(c, slf, LOCKED) }
fn b_private_p(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { flag_p(c, slf, PRIVATE) }
fn b_readonly_p(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { flag_p(c, slf, READONLY) }

fn b_locked(c: &mut Ctx, slf: Value, _a: &[Value], blk: Blk) -> RV {
    if buf(c, slf).flags & LOCKED != 0 { return raise(c, &E_LOCKED, "Buffer already locked!"); }
    buf_mut(c, slf).flags |= LOCKED;
    let r = if blk.is_null() { err(c, c.vm().c_local_jump_error.get(), "no block given") } else { yield1(c, blk, slf) };
    // 4.0.10: rb_ensure, unlocked after a raise / break / throw too
    buf_mut(c, slf).flags &= !LOCKED;
    r
}
/// rb_io_buffer_free.
fn free_checked(c: &mut Ctx, v: Value) -> Result<(), Esc> {
    if buf(c, v).flags & LOCKED != 0 { return raise(c, &E_LOCKED, "Buffer is locked!"); }
    free_mem(buf_mut(c, v));
    Ok(())
}
fn b_free(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { free_checked(c, slf)?; Ok(slf) }

fn b_slice(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    check_argc(c, a, 0, 2)?;
    let (off, len) = offset_length(c, slf, a)?;
    validate_range(c, slf, off, len)?;
    let k = real_class_of(c, slf);
    let inst = new_instance(c, k)?;
    let (flags, base, src) = { let b = buf(c, slf); (b.flags & READONLY, b.base, b.source) };
    let s = buf_mut(c, inst);
    s.flags = flags; s.base = base.wrapping_add(off); s.size = len; s.source = if src != NIL { src } else { slf };
    Ok(inst)
}
fn b_transfer(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    if buf(c, slf).flags & LOCKED != 0 { return raise(c, &E_LOCKED, "Cannot transfer ownership of locked buffer!"); }
    let k = real_class_of(c, slf);
    let inst = new_instance(c, k)?;
    // io_buffer_zero: the flags stay behind
    let b = buf_mut(c, slf);
    let moved = RBuf { mem: std::mem::replace(&mut b.mem, Mem::None), base: b.base, size: b.size, flags: b.flags, source: b.source };
    b.base = 0; b.size = 0; b.source = NIL;
    *buf_mut(c, inst) = moved;
    Ok(inst)
}
fn sys_fail<T>(c: &mut Ctx, e: i64, what: &str) -> Result<T, Esc> { Err(crate::builtins::exception::raise_errno_plain(c, e, what)) }
/// rb_io_buffer_resize.
fn b_resize(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let size = extract(c, a[0], "Size")?;
    for_reading(c, slf)?;
    let (flags, base) = { let b = buf(c, slf); (b.flags, b.base) };
    if flags & LOCKED != 0 { return raise(c, &E_LOCKED, "Cannot resize locked buffer!"); }
    if base == 0 { initialize_with(c, slf, size, flags_for_size(size))?; return Ok(slf); }
    if flags & EXTERNAL != 0 { return raise(c, &E_ACCESS, "Cannot resize external buffer!"); }
    if flags & MAPPED != 0 {
        let b = buf_mut(c, slf);
        let old = b.size;
        let r: Result<Option<sys::Mapping>, i64> = match &mut b.mem {
            Mem::Map(m, file_len) if size <= *file_len => m.remap(size).map(|_| None),
            // past the file-backed bytes: a private anonymous copy instead of a mapping that would fault
            Mem::Map(m, _) => sys::Mapping::new(size, true, false, -1, 0).map(|mut n| {
                let keep = old.min(size).min(m.len());
                if let Some(d) = n.bytes_mut() { d[..keep].copy_from_slice(&m.bytes()[..keep]); }
                Some(n)
            }),
            _ => Err(22),
        };
        match r { Err(e) => return sys_fail(c, e, "rb_io_buffer_resize:mremap"), Ok(Some(n)) => b.mem = Mem::Map(n, usize::MAX), Ok(None) => {} }
        if let Some(d) = b.mem.bytes_mut() { if size > old { d[old..size].fill(0); } }
        b.base = b.mem.addr(); b.size = size;
        return Ok(slf);
    }
    if flags & INTERNAL != 0 {
        if size == 0 { free_mem(buf_mut(c, slf)); return Ok(slf); }
        let b = buf_mut(c, slf);
        let ok = match &mut b.mem {
            Mem::Heap(v) => {
                if size < v.len() { v.truncate(size); v.shrink_to_fit(); true }
                else if v.try_reserve_exact(size - v.len()).is_ok() { v.resize(size, 0); crate::gc::malloc_increase(size); true }
                else { false }
            }
            _ => false,
        };
        if !ok { return sys_fail(c, 12, "rb_io_buffer_resize:realloc"); }
        b.base = b.mem.addr(); b.size = size;
        return Ok(slf);
    }
    // io_buffer_resize_copy (a slice): new memory of its own, the bytes copied
    let (sa, ssize) = for_reading(c, slf)?;
    let keep: Vec<u8> = rd(c, sa, ssize)[..ssize.min(size)].to_vec();
    if size == 0 { *buf_mut(c, slf) = empty(); return Ok(slf); }
    let fl = flags_for_size(size);
    let mut mem = allocate(c, size, fl)?;
    if let Some(d) = mem.bytes_mut() { d[..keep.len()].copy_from_slice(&keep); }
    let b = buf_mut(c, slf);
    b.base = mem.addr(); b.mem = mem; b.size = size; b.flags = fl; b.source = NIL;
    Ok(slf)
}
fn b_cmp(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let (xa, xs) = for_reading(c, slf)?;
    check(c, a[0])?;
    let (ya, ys) = for_reading(c, a[0])?;
    if xs != ys { return Ok(Value::fix(if xs < ys { -1 } else { 1 })); }
    let (x, y) = (rd(c, xa, xs), rd(c, ya, ys));
    // memcmp's result: the difference of the first differing bytes (glibc)
    Ok(Value::fix(x.iter().zip(y).find(|(p, q)| p != q).map_or(0, |(&p, &q)| p as i64 - q as i64)))
}

fn b_get_value(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let off = extract(c, a[1], "Offset")?;
    for_reading(c, slf)?;
    let t = ty_of(c, a[0])?;
    value_at(c, slf, t, off)
}
fn b_get_values(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let mut off = extract(c, a[1], "Offset")?;
    for_reading(c, slf)?;
    if !is_array(a[0]) { return arg_err(c, "Argument buffer_types should be an array!"); }
    let out = c.ary_new();
    let mut i = 0;
    while let Some(&tv) = c.ary(a[0]).get(i) {
        let t = ty_of(c, tv)?;
        let v = value_at(c, slf, t, off)?;
        c.ary_push(out, v);
        off += t.size();
        i += 1;
    }
    Ok(out)
}
/// One value at `off`, from the buffer's current bytes.
fn value_at(c: &mut Ctx, slf: Value, t: Ty, off: usize) -> RV {
    let (at, size) = for_reading(c, slf)?;
    let n = t.size();
    validate_type(c, size, off, n)?;
    let mut tmp = [0u8; 16];
    tmp[..n].copy_from_slice(&rd(c, at, size)[off..off + n]);
    decode(c, t, &tmp[..n], 0)
}
/// io_buffer_extract_offset_count.
fn offset_count(c: &mut Ctx, t: Option<Ty>, size: usize, a: &[Value]) -> Result<(usize, usize), Esc> {
    let off = match a.first() { Some(&x) => extract(c, x, "Offset")?, None => 0 };
    let count = match a.get(1) {
        Some(&x) => extract(c, x, "Count")?,
        None => {
            if off > size { return arg_err(c, "The given offset is bigger than the buffer size!"); }
            let n = match t { Some(t) => t.size(), None => return arg_err(c, "Invalid type name!") };
            (size - off) / n
        }
    };
    Ok((off, count))
}
/// The type argument of each / values (default :U8); the lookup error waits for the first use, as in CRuby.
fn type_arg(c: &mut Ctx, a: &[Value]) -> Result<(Sym, Option<Ty>), Esc> {
    let id = match a.first() { Some(&v) => sym_of(c, v)?, None => intern(b"U8") };
    let n = sym_bytes(id);
    Ok((id, ty_named(n)))
}
fn b_each(c: &mut Ctx, slf: Value, a: &[Value], blk: Blk) -> RV {
    if blk.is_null() { return crate::builtins::enumerator::enum_for(c, slf, intern(b"each"), a, NIL); }
    let (_, size) = for_reading(c, slf)?;
    let (id, t) = type_arg(c, a)?;
    let (mut off, count) = offset_count(c, t, size, a.get(1..).unwrap_or(&[]))?;
    for _ in 0..count {
        let t = match t { Some(t) => t, None => ty_of_sym(c, id)? };
        let v = value_at(c, slf, t, off)?;
        let o = u64_value(c, off as u64);
        yield_block(c, blk, &[o, v])?;
        off += t.size();
    }
    Ok(slf)
}
fn b_values(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    let (_, size) = for_reading(c, slf)?;
    let (id, t) = type_arg(c, a)?;
    let (mut off, count) = offset_count(c, t, size, a.get(1..).unwrap_or(&[]))?;
    let out = c.ary_new();
    for _ in 0..count {
        let t = match t { Some(t) => t, None => ty_of_sym(c, id)? };
        let v = value_at(c, slf, t, off)?;
        c.ary_push(out, v);
        off += t.size();
    }
    Ok(out)
}
fn b_each_byte(c: &mut Ctx, slf: Value, a: &[Value], blk: Blk) -> RV {
    if blk.is_null() { return crate::builtins::enumerator::enum_for(c, slf, intern(b"each_byte"), a, NIL); }
    let (_, size) = for_reading(c, slf)?;
    let u8t = Ty { k: Kind::U8, be: true };
    let (off, count) = offset_count(c, Some(u8t), size, a)?;
    // CRuby reads past the end here; refuse instead
    if off.checked_add(count).is_none_or(|e| e > size) { return arg_err(c, "Specified offset+length is bigger than the buffer size!"); }
    for i in 0..count {
        let v = value_at(c, slf, u8t, off + i)?;
        yield1(c, blk, v)?;
    }
    Ok(slf)
}
/// io_buffer_try_lock + rb_ensure (4.0.10): the conversions in `f` cannot free or resize the buffer.
fn with_lock(c: &mut Ctx, slf: Value, f: impl FnOnce(&mut Ctx) -> RV) -> RV {
    if buf(c, slf).flags & LOCKED != 0 { return f(c); }
    buf_mut(c, slf).flags |= LOCKED;
    let r = f(c);
    buf_mut(c, slf).flags &= !LOCKED;
    r
}
fn b_set_value(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    for_writing(c, slf)?;
    let mut off = extract(c, a[1], "Offset")?;
    with_lock(c, slf, |c| { set_one(c, slf, a[0], &mut off, a[2])?; Ok(u64_value(c, off as u64)) })
}
fn b_set_values(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    for_writing(c, slf)?;
    if !is_array(a[0]) { return arg_err(c, "Argument buffer_types should be an array!"); }
    let mut off = extract(c, a[1], "Offset")?;
    if !is_array(a[2]) { return arg_err(c, "Argument values should be an array!"); }
    if c.ary_len(a[0]) != c.ary_len(a[2]) { return arg_err(c, "Argument buffer_types and values should have the same length!"); }
    with_lock(c, slf, |c| {
        let mut i = 0;
        while i < c.ary_len(a[0]) {
            let (tv, v) = (c.ary_get(a[0], i), c.ary_get(a[2], i));
            set_one(c, slf, tv, &mut off, v)?;
            i += 1;
        }
        Ok(u64_value(c, off as u64))
    })
}

/// io_buffer_copy_from: `src` (a copy of the source bytes) into the buffer; (offset, length, source_offset).
fn copy_in(c: &mut Ctx, slf: Value, src: &[u8], a: &[Value]) -> RV {
    let off = match a.first() { Some(&x) => extract(c, x, "Offset")?, None => 0 };
    let soff = match a.get(2) {
        Some(&x) => {
            let n = extract(c, x, "Offset")?;
            if n > src.len() { return arg_err(c, "The given source offset is bigger than the source itself!"); }
            n
        }
        None => 0,
    };
    let len = match opt(a, 1) { Some(x) => extract(c, x, "Length")?, None => src.len() - soff };
    let (at, size) = for_writing(c, slf)?;
    validate_range(c, slf, off, len)?;
    if soff.checked_add(len).is_none_or(|e| e > src.len()) { return arg_err(c, "The computed source range exceeds the size of the source buffer!"); }
    if len != 0 { wr(c, at, size)?[off..off + len].copy_from_slice(&src[soff..soff + len]); }
    Ok(u64_value(c, len as u64))
}
fn b_copy(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    check_argc(c, a, 1, 4)?;
    check(c, a[0])?;
    let (sa, ss) = for_reading(c, a[0])?;
    let src = rd(c, sa, ss).to_vec();
    copy_in(c, slf, &src, &a[1..])
}
fn b_get_string(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    check_argc(c, a, 0, 3)?;
    let (off, len) = offset_length(c, slf, a)?;
    for_reading(c, slf)?;
    let enc = if a.len() >= 3 { crate::builtins::encoding::to_enc_opt(c, a[2])?.unwrap_or(ENC_BINARY) } else { ENC_BINARY };
    validate_range(c, slf, off, len)?;
    let (at, size) = for_reading(c, slf)?;
    let b = rd(c, at, size)[off..off + len].to_vec();
    Ok(c.str_from_vec_enc(b, enc))
}
fn b_set_string(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    check_argc(c, a, 1, 4)?;
    let s = expect_str(c, a[0])?;
    let src = c.str_bytes(s).to_vec();
    copy_in(c, slf, &src, &a[1..])
}
fn b_clear(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV {
    check_argc(c, a, 0, 3)?;
    let val = match a.first() { Some(&v) => num2uint(c, v)? as u8, None => 0 };
    let (off, len) = offset_length(c, slf, a.get(1..).unwrap_or(&[]))?;
    let (at, size) = for_writing(c, slf)?;
    validate_range(c, slf, off, len)?;
    wr(c, at, size)?[off..off + len].fill(val);
    Ok(slf)
}

// ---- bitwise --------------------------------------------------------------------------------

/// A new buffer of the receiver's size from op(byte, mask byte).
fn binop(c: &mut Ctx, slf: Value, mask: Value, op: fn(u8, u8) -> u8) -> RV {
    check(c, mask)?;
    let (xa, xs) = for_reading(c, slf)?;
    let (ma, ms) = for_reading(c, mask)?;
    if ms == 0 { return raise(c, &E_MASK, "Zero-length mask given!"); }
    let (x, m) = (rd(c, xa, xs), rd(c, ma, ms));
    let out: Vec<u8> = x.iter().enumerate().map(|(i, &b)| op(b, m[i % ms])).collect();
    new_with(c, &out)
}
/// rb_io_buffer_new(NULL, size, io_flags_for_size(size)) filled with `bytes`.
fn new_with(c: &mut Ctx, bytes: &[u8]) -> RV {
    let k = CLASS.get(c);
    let inst = new_instance(c, k)?;
    initialize_with(c, inst, bytes.len(), flags_for_size(bytes.len()))?;
    let (at, size) = for_writing(c, inst)?;
    wr(c, at, size)?.copy_from_slice(bytes);
    Ok(inst)
}
fn b_and(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV { binop(c, slf, a[0], |x, m| x & m) }
fn b_or(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV { binop(c, slf, a[0], |x, m| x | m) }
fn b_xor(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV { binop(c, slf, a[0], |x, m| x ^ m) }
fn b_not(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    let (xa, xs) = for_reading(c, slf)?;
    let out: Vec<u8> = rd(c, xa, xs).iter().map(|&b| !b).collect();
    new_with(c, &out)
}
/// In place: mask size, overlap (by address, as CRuby), then the write.
fn binop_bang(c: &mut Ctx, slf: Value, mask: Value, op: fn(u8, u8) -> u8) -> RV {
    check(c, mask)?;
    let (xb, xs) = { let b = buf(c, slf); (b.base, b.size) };
    let (mb, ms) = { let b = buf(c, mask); (b.base, b.size) };
    if ms == 0 { return raise(c, &E_MASK, "Zero-length mask given!"); }
    let ((lb, ls), rb) = if xb > mb { ((mb, ms), xb) } else { ((xb, xs), mb) };
    if rb >= lb && rb < lb.wrapping_add(ls) { return raise(c, &E_MASK, "Mask overlaps source buffer!"); }
    let (xa, xs) = for_writing(c, slf)?;
    let (ma, ms) = for_reading(c, mask)?;
    let m = rd(c, ma, ms).to_vec();
    if m.is_empty() { return Ok(slf); }
    for (i, b) in wr(c, xa, xs)?.iter_mut().enumerate() { *b = op(*b, m[i % m.len()]); }
    Ok(slf)
}
fn b_and_bang(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV { binop_bang(c, slf, a[0], |x, m| x & m) }
fn b_or_bang(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV { binop_bang(c, slf, a[0], |x, m| x | m) }
fn b_xor_bang(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV { binop_bang(c, slf, a[0], |x, m| x ^ m) }
fn b_not_bang(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    let (xa, xs) = for_writing(c, slf)?;
    for b in wr(c, xa, xs)?.iter_mut() { *b = !*b; }
    Ok(slf)
}

// ---- IO -------------------------------------------------------------------------------------

/// rb_io_get_io.
fn to_io(c: &mut Ctx, v: Value) -> RV {
    if crate::builtins::io::is_io(c, v) { return Ok(v); }
    let to_io = intern(b"to_io");
    if v.is_heap() && respond_to(c, v, to_io, true) {
        let r = funcall(c, v, to_io, &[], nb())?;
        if crate::builtins::io::is_io(c, r) { return Ok(r); }
        let k = class_name_for_err(c, v);
        return type_err(c, format!("can't convert {} to IO ({}#to_io gives {})", k, k, class_name_for_err(c, r)));
    }
    type_err(c, format!("no implicit conversion of {} into IO", class_name_for_err(c, v)))
}
/// rb_io_descriptor (of the write side for writes: rb_io_get_write_io).
fn descriptor(c: &mut Ctx, io: Value, write: bool) -> Result<i32, Esc> {
    let r = crate::builtins::io::get_io(c, io)?;
    Ok(if write && r.wfd >= 0 { r.wfd } else { r.fd })
}
#[derive(Clone, Copy)]
enum Dir { Read, Write }
/// The blocking region: the buffer is locked (unless it already was) while the system calls run.
/// With other threads, a blocking fd is waited on through the blop layer first, and the buffer is
/// resolved again after the wait (another thread may have run).
fn transfer_io(c: &mut Ctx, slf: Value, fd: i32, dir: Dir, from: Option<i64>, len: usize, off: usize) -> RV {
    let was = buf(c, slf).flags & LOCKED != 0;
    if !was { buf_mut(c, slf).flags |= LOCKED; }
    let r = transfer_loop(c, slf, fd, dir, from, len, off);
    if !was { buf_mut(c, slf).flags &= !LOCKED; }
    r
}
fn transfer_loop(c: &mut Ctx, slf: Value, fd: i32, dir: Dir, mut from: Option<i64>, len: usize, off: usize) -> RV {
    let mut total = 0usize;
    let blocking = sys::fcntl(fd, sys::F_GETFL, 0) & sys::O_NONBLOCK == 0;
    loop {
        // after POLLOUT only PIPE_BUF bytes are sure not to block (io_design.md, "Write" ops)
        let mut cap = usize::MAX;
        if blocking && crate::builtins::thread::active(c) {
            let ev = match dir { Dir::Read => sys::POLLIN, Dir::Write => sys::POLLOUT };
            crate::builtins::io::wait_one(c, NIL, fd, ev, None, NIL)?;
            if matches!(dir, Dir::Write) { cap = 4096; }
        }
        let start = off + total;
        let n = match dir {
            Dir::Read => {
                let (at, size) = for_writing(c, slf)?;
                if start > size { return arg_err(c, "Specified offset+length is bigger than the buffer size!"); }
                let b = &mut wr(c, at, size)?[start..];
                match from { Some(p) => sys::pread(fd, b, p), None => sys::read(fd, b) }
            }
            Dir::Write => {
                let (at, size) = for_reading(c, slf)?;
                if start > size { return arg_err(c, "Specified offset+length is bigger than the buffer size!"); }
                let b = &rd(c, at, size)[start..];
                let b = &b[..b.len().min(cap)];
                match from { Some(p) => sys::pwrite(fd, b, p), None => sys::write(fd, b) }
            }
        };
        if n < 0 {
            let e = sys::errno();
            if e == sys::EINTR { continue; }
            return Ok(Value::fix(-e));
        }
        if n == 0 { break; }
        total += n as usize;
        if total >= len { break; }
        if let Some(p) = from.as_mut() { *p += n as i64; }
    }
    Ok(u64_value(c, total as u64))
}
fn io_op(c: &mut Ctx, slf: Value, a: &[Value], dir: Dir, positional: bool) -> RV {
    let (min, max) = if positional { (2, 4) } else { (1, 3) };
    check_argc(c, a, min, max)?;
    let from = if positional { Some(num2long(c, a[1])?) } else { None };
    let (len, off) = length_offset(c, slf, &a[min..])?;
    let io = to_io(c, a[0])?;
    validate_range(c, slf, off, len)?;
    let fd = descriptor(c, io, matches!(dir, Dir::Write))?;
    match dir { Dir::Read => { for_writing(c, slf)?; } Dir::Write => { for_reading(c, slf)?; } }
    transfer_io(c, slf, fd, dir, from, len, off)
}
fn b_read(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV { io_op(c, slf, a, Dir::Read, false) }
fn b_pread(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV { io_op(c, slf, a, Dir::Read, true) }
fn b_write(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV { io_op(c, slf, a, Dir::Write, false) }
fn b_pwrite(c: &mut Ctx, slf: Value, a: &[Value], _b: Blk) -> RV { io_op(c, slf, a, Dir::Write, true) }

// ---- Init_IO_Buffer -------------------------------------------------------------------------

/// `IO::Buffer` (lazy: the constant is an internal autoload of "saruby/io_buffer").
pub fn init(c: &mut Ctx) {
    use std::sync::atomic::Ordering::Relaxed;
    let page = match sys::sysconf(30) { n if n > 0 => n as usize, _ => 4096 };   // _SC_PAGESIZE
    PAGE_SIZE.store(page, Relaxed);
    let env = std::env::var("RUBY_IO_BUFFER_DEFAULT_SIZE").ok().map(|s| {
        let t = s.trim_start();
        let (neg, d) = match t.as_bytes().first() { Some(b'-') => (true, &t[1..]), Some(b'+') => (false, &t[1..]), _ => (false, t) };
        let n: i64 = d.bytes().take_while(|b| b.is_ascii_digit()).fold(0i64, |a, b| a.saturating_mul(10).saturating_add((b - b'0') as i64)).min(i32::MAX as i64);
        if neg { -n } else { n }
    });
    DEFAULT_SIZE.store(match env { Some(n) if n > 0 => n as usize, _ => 65536usize.max(page) }, Relaxed);

    let vmr = c.vm();
    let k = define_class_under(c, vmr.c_io.get(), "Buffer", vmr.c_object.get());
    class_set_alloc(c, k, AllocKind::None);
    CLASS.set(c, k);
    let rt = vmr.c_runtime_error.get();
    let sub = |c: &mut Ctx, g: &Global, n: &str| { let e = define_class_under(c, k, n, rt); g.set(c, e); };
    sub(c, &E_LOCKED, "LockedError"); sub(c, &E_ALLOC, "AllocationError"); sub(c, &E_ACCESS, "AccessError"); sub(c, &E_INVALID, "InvalidatedError");
    let e = define_class_under(c, k, "MaskError", vmr.c_argument_error.get());
    E_MASK.set(c, e);

    let ds = |c: &mut Ctx, n: &str, f: CFunc, a: i32| define_singleton_cfunc(c, k, n, f, a);
    ds(c, "for", s_for, 1); ds(c, "string", s_string, 1); ds(c, "map", s_map, -1); ds(c, "size_of", s_size_of, 1);
    let cs = |c: &mut Ctx, n: &str, v: i64| const_set(c, k, intern(n.as_bytes()), Value::fix(v));
    cs(c, "PAGE_SIZE", page as i64); cs(c, "DEFAULT_SIZE", DEFAULT_SIZE.load(Relaxed) as i64);
    cs(c, "EXTERNAL", EXTERNAL as i64); cs(c, "INTERNAL", INTERNAL as i64); cs(c, "MAPPED", MAPPED as i64); cs(c, "SHARED", SHARED as i64);
    cs(c, "LOCKED", LOCKED as i64); cs(c, "PRIVATE", PRIVATE as i64); cs(c, "READONLY", READONLY as i64);
    cs(c, "LITTLE_ENDIAN", 4); cs(c, "BIG_ENDIAN", 8); cs(c, "HOST_ENDIAN", if cfg!(target_endian = "big") { 8 } else { 4 }); cs(c, "NETWORK_ENDIAN", 8);
    define_private_cfunc(c, k, "initialize", b_initialize, -1);
    define_private_cfunc(c, k, "initialize_copy", b_init_copy, 1);
    let d = |c: &mut Ctx, n: &str, f: CFunc, a: i32| define_cfunc(c, k, n, f, a);
    d(c, "inspect", b_inspect, 0); d(c, "hexdump", b_hexdump, -1); d(c, "to_s", b_to_s, 0); d(c, "size", b_size, 0);
    d(c, "valid?", b_valid_p, 0); d(c, "transfer", b_transfer, 0);
    d(c, "null?", b_null_p, 0); d(c, "empty?", b_empty_p, 0); d(c, "external?", b_external_p, 0); d(c, "internal?", b_internal_p, 0);
    d(c, "mapped?", b_mapped_p, 0); d(c, "shared?", b_shared_p, 0); d(c, "locked?", b_locked_p, 0); d(c, "private?", b_private_p, 0);
    d(c, "readonly?", b_readonly_p, 0); d(c, "locked", b_locked, 0);
    d(c, "slice", b_slice, -1); d(c, "<=>", b_cmp, 1); d(c, "resize", b_resize, 1); d(c, "clear", b_clear, -1); d(c, "free", b_free, 0);
    let cmp = vmr.m_comparable.get();
    include_module_c(c, k, cmp);
    d(c, "get_value", b_get_value, 2); d(c, "get_values", b_get_values, 2); d(c, "each", b_each, -1); d(c, "values", b_values, -1);
    d(c, "each_byte", b_each_byte, -1); d(c, "set_value", b_set_value, 3); d(c, "set_values", b_set_values, 3);
    d(c, "copy", b_copy, -1); d(c, "get_string", b_get_string, -1); d(c, "set_string", b_set_string, -1);
    d(c, "&", b_and, 1); d(c, "|", b_or, 1); d(c, "^", b_xor, 1); d(c, "~", b_not, 0);
    d(c, "and!", b_and_bang, 1); d(c, "or!", b_or_bang, 1); d(c, "xor!", b_xor_bang, 1); d(c, "not!", b_not_bang, 0);
    d(c, "read", b_read, -1); d(c, "pread", b_pread, -1); d(c, "write", b_write, -1); d(c, "pwrite", b_pwrite, -1);
}

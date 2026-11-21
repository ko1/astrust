// Encodings: the table (CRuby's Encoding.list order), per-encoding character scanning,
// code <-> Unicode mapping for transcoding, and the Encoding class.
#![forbid(unsafe_code)]
use crate::array::*;
use crate::builtins::enc_tables as tb;
use crate::builtins::misc::REnc;
#[allow(unused_imports)]
use crate::heap::no_legacy::*;   // the old accessors cannot be named here
#[allow(unused_imports)]
use crate::heap::no_legacy_str::*;
use crate::string::*;
use crate::symbol::intern;
use std::sync::atomic::{AtomicI32, AtomicU8, Ordering::Relaxed};
use std::sync::OnceLock;
use crate::value::*;
use crate::vm::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EK {
    Utf8, Bin, Ascii,
    Single,          // ASCII-compatible single-byte (table looked up by name)
    U16LE, U16BE, U32LE, U32BE,
    EucJp(bool),     // true = CP51932 table
    Sjis(bool),      // true = Windows-31J table
    Dbcs,            // generic EUC-style double byte (A1-FE A1-FE)
    Big5, Gbk, Gb18030, EucTw,
    EmacsMule,
    Dummy,           // byte-wise, not ASCII compatible
}

pub struct EncDef { pub name: &'static str, pub aliases: &'static [&'static str], pub kind: EK }

macro_rules! e { ($n:expr, [$($a:expr),*], $k:expr) => { EncDef { name: $n, aliases: &[$($a),*], kind: $k } }; }
use EK::*;
/// Index = internal encoding id.  0/1 are swapped relative to CRuby so ENC_UTF8 == 0.
pub static ENCS: [EncDef; 103] = [
    e!("UTF-8", ["CP65001", "locale", "external", "filesystem"], Utf8),
    e!("ASCII-8BIT", ["BINARY"], Bin),
    e!("US-ASCII", ["ASCII", "ANSI_X3.4-1968", "646"], Ascii),
    e!("UTF-16BE", ["UCS-2BE"], U16BE),
    e!("UTF-16LE", [], U16LE),
    e!("UTF-32BE", ["UCS-4BE"], U32BE),
    e!("UTF-32LE", ["UCS-4LE"], U32LE),
    e!("UTF-16", [], Dummy),
    e!("UTF-32", [], Dummy),
    e!("UTF8-MAC", ["UTF-8-MAC", "UTF-8-HFS"], Utf8),
    e!("EUC-JP", ["eucJP"], EucJp(false)),
    e!("Windows-31J", ["CP932", "csWindows31J", "SJIS", "PCK"], Sjis(true)),
    e!("Big5", [], Big5),
    e!("Big5-HKSCS", ["Big5-HKSCS:2008"], Big5),
    e!("Big5-UAO", [], Big5),
    e!("CESU-8", [], Utf8),
    e!("CP949", [], Gbk),
    e!("Emacs-Mule", [], EmacsMule),
    e!("EUC-KR", ["eucKR"], Dbcs),
    e!("EUC-TW", ["eucTW"], EucTw),
    e!("GB18030", [], Gb18030),
    e!("GBK", ["CP936"], Gbk),
    e!("ISO-8859-1", ["ISO8859-1"], Single),
    e!("ISO-8859-2", ["ISO8859-2"], Single),
    e!("ISO-8859-3", ["ISO8859-3"], Single),
    e!("ISO-8859-4", ["ISO8859-4"], Single),
    e!("ISO-8859-5", ["ISO8859-5"], Single),
    e!("ISO-8859-6", ["ISO8859-6"], Single),
    e!("ISO-8859-7", ["ISO8859-7"], Single),
    e!("ISO-8859-8", ["ISO8859-8"], Single),
    e!("ISO-8859-9", ["ISO8859-9"], Single),
    e!("ISO-8859-10", ["ISO8859-10"], Single),
    e!("ISO-8859-11", ["ISO8859-11"], Single),
    e!("ISO-8859-13", ["ISO8859-13"], Single),
    e!("ISO-8859-14", ["ISO8859-14"], Single),
    e!("ISO-8859-15", ["ISO8859-15"], Single),
    e!("ISO-8859-16", ["ISO8859-16"], Single),
    e!("KOI8-R", ["CP878"], Single),
    e!("KOI8-U", [], Single),
    e!("Shift_JIS", [], Sjis(false)),
    e!("Windows-1250", ["CP1250"], Single),
    e!("Windows-1251", ["CP1251"], Single),
    e!("Windows-1252", ["CP1252"], Single),
    e!("Windows-1253", ["CP1253"], Single),
    e!("Windows-1254", ["CP1254"], Single),
    e!("Windows-1257", ["CP1257"], Single),
    e!("IBM437", ["CP437"], Single),
    e!("IBM720", ["CP720"], Single),
    e!("IBM737", ["CP737"], Single),
    e!("IBM775", ["CP775"], Single),
    e!("CP850", ["IBM850"], Single),
    e!("IBM852", [], Single),
    e!("CP852", [], Single),
    e!("IBM855", [], Single),
    e!("CP855", [], Single),
    e!("IBM857", ["CP857"], Single),
    e!("IBM860", ["CP860"], Single),
    e!("IBM861", ["CP861"], Single),
    e!("IBM862", ["CP862"], Single),
    e!("IBM863", ["CP863"], Single),
    e!("IBM864", ["CP864"], Single),
    e!("IBM865", ["CP865"], Single),
    e!("IBM866", ["CP866"], Single),
    e!("IBM869", ["CP869"], Single),
    e!("Windows-1258", ["CP1258"], Single),
    e!("GB1988", [], Single),
    e!("macCentEuro", [], Single),
    e!("macCroatian", [], Single),
    e!("macCyrillic", [], Single),
    e!("macGreek", [], Single),
    e!("macIceland", [], Single),
    e!("macRoman", [], Single),
    e!("macRomania", [], Single),
    e!("macThai", [], Single),
    e!("macTurkish", [], Single),
    e!("macUkraine", [], Single),
    e!("CP950", [], Big5),
    e!("CP951", [], Big5),
    e!("IBM037", ["ebcdic-cp-us"], Dummy),
    e!("stateless-ISO-2022-JP", [], EucJp(false)),
    e!("eucJP-ms", ["euc-jp-ms"], EucJp(false)),
    e!("CP51932", [], EucJp(true)),
    e!("EUC-JIS-2004", ["EUC-JISX0213"], EucJp(false)),
    e!("GB2312", ["EUC-CN", "eucCN"], Dbcs),
    e!("GB12345", [], Dbcs),
    e!("ISO-2022-JP", ["ISO2022-JP"], Dummy),
    e!("ISO-2022-JP-2", ["ISO2022-JP2"], Dummy),
    e!("CP50220", [], Dummy),
    e!("CP50221", [], Dummy),
    e!("Windows-1256", ["CP1256"], Single),
    e!("Windows-1255", ["CP1255"], Single),
    e!("TIS-620", [], Single),
    e!("Windows-874", ["CP874"], Single),
    e!("MacJapanese", ["MacJapan"], Sjis(false)),
    e!("UTF-7", ["CP65000"], Dummy),
    e!("UTF8-DoCoMo", [], Utf8),
    e!("SJIS-DoCoMo", [], Sjis(true)),
    e!("UTF8-KDDI", [], Utf8),
    e!("SJIS-KDDI", [], Sjis(true)),
    e!("ISO-2022-JP-KDDI", [], Dummy),
    e!("stateless-ISO-2022-JP-KDDI", [], EucJp(false)),
    e!("UTF8-SoftBank", [], Utf8),
    e!("SJIS-SoftBank", [], Sjis(true)),
];
pub const NENC: usize = 103;
pub const ENC_UTF16LE: u8 = 4;
pub const ENC_UTF16BE: u8 = 3;
pub const ENC_UTF32LE: u8 = 6;
pub const ENC_UTF32BE: u8 = 5;
pub const ENC_UTF16: u8 = 7;
pub const ENC_UTF32: u8 = 8;
pub const ENC_EUCJP: u8 = 10;
pub const ENC_ISO_8859_1: u8 = 22;

// index into tb::SINGLE per encoding (255: none), built on first use
static SINGLE_IDX: OnceLock<[u8; NENC]> = OnceLock::new();
static DEFAULT_EXTERNAL: AtomicU8 = AtomicU8::new(ENC_UTF8);
static DEFAULT_INTERNAL: AtomicI32 = AtomicI32::new(-1);

#[inline] pub fn kind(enc: u8) -> EK { ENCS.get(enc as usize).map(|d| d.kind).unwrap_or(Bin) }
pub fn enc_name(enc: u8) -> &'static str { ENCS.get(enc as usize).map(|d| d.name).unwrap_or("ASCII-8BIT") }
/// Name as it appears in error messages ("BINARY (ASCII-8BIT)" for binary).
pub fn enc_name_msg(enc: u8) -> String { if enc == ENC_BINARY { "BINARY (ASCII-8BIT)".into() } else { enc_name(enc).into() } }
pub fn is_dummy(enc: u8) -> bool { kind(enc) == Dummy || enc == ENC_UTF16 || enc == ENC_UTF32 }
pub fn ascii_compat(enc: u8) -> bool { !matches!(kind(enc), U16LE | U16BE | U32LE | U32BE | Dummy) }
pub fn is_unicode(enc: u8) -> bool { matches!(kind(enc), Utf8 | U16LE | U16BE | U32LE | U32BE) || enc == ENC_UTF16 || enc == ENC_UTF32 }
/// Every byte is one character.
#[inline] pub fn is_single_byte(enc: u8) -> bool { matches!(kind(enc), Bin | Ascii | Single | Dummy) }
pub fn min_len(enc: u8) -> usize { match kind(enc) { U16LE | U16BE => 2, U32LE | U32BE => 4, _ => 1 } }
pub fn max_len(enc: u8) -> usize { match kind(enc) { Utf8 | U32LE | U32BE | U16LE | U16BE | Gb18030 | EucTw | EmacsMule => 4, EucJp(_) => 3, Sjis(_) | Dbcs | Big5 | Gbk => 2, _ => 1 } }

fn single_table(enc: u8) -> Option<&'static [u16; 128]> {
    let idx = SINGLE_IDX.get_or_init(|| {
        let mut t = [255u8; NENC];
        for (i, d) in ENCS.iter().enumerate() {
            if d.kind == Single { if let Some(p) = tb::SINGLE_NAMES.iter().position(|n| *n == d.name) { t[i] = p as u8; } }
        }
        t
    });
    let i = idx[enc as usize];
    if i == 255 { None } else { Some(&tb::SINGLE[i as usize]) }
}

/// Length of a valid character at b[i..], or None if invalid/truncated.
pub fn precise_len(enc: u8, b: &[u8], i: usize) -> Option<usize> {
    let r = &b[i..];
    let x = *r.first()?;
    let n = r.len();
    let tr = |k: usize, lo: u8, hi: u8| k < n && r[k] >= lo && r[k] <= hi;
    match kind(enc) {
        Bin | Single | Dummy => Some(1),
        Ascii => if x < 0x80 { Some(1) } else { None },
        Utf8 => utf8_precise(r),
        U16LE | U16BE => {
            if n < 2 { return None; }
            let w = if kind(enc) == U16LE { u16::from_le_bytes([r[0], r[1]]) } else { u16::from_be_bytes([r[0], r[1]]) };
            if (0xd800..0xdc00).contains(&w) {
                if n < 4 { return None; }
                let w2 = if kind(enc) == U16LE { u16::from_le_bytes([r[2], r[3]]) } else { u16::from_be_bytes([r[2], r[3]]) };
                if (0xdc00..0xe000).contains(&w2) { Some(4) } else { None }
            } else if (0xdc00..0xe000).contains(&w) { None } else { Some(2) }
        }
        U32LE | U32BE => {
            if n < 4 { return None; }
            let w = if kind(enc) == U32LE { u32::from_le_bytes([r[0], r[1], r[2], r[3]]) } else { u32::from_be_bytes([r[0], r[1], r[2], r[3]]) };
            if w > 0x10ffff || (0xd800..0xe000).contains(&w) { None } else { Some(4) }
        }
        EucJp(_) => {
            if x < 0x80 { Some(1) }
            else if x == 0x8e { if tr(1, 0xa1, 0xfe) { Some(2) } else { None } }
            else if x == 0x8f { if tr(1, 0xa1, 0xfe) && tr(2, 0xa1, 0xfe) { Some(3) } else { None } }
            else if (0xa1..=0xfe).contains(&x) { if tr(1, 0xa1, 0xfe) { Some(2) } else { None } }
            else { None }
        }
        Sjis(_) => {
            if x < 0x80 || (0xa1..=0xdf).contains(&x) { Some(1) }
            else if (0x81..=0x9f).contains(&x) || (0xe0..=0xfc).contains(&x) { if tr(1, 0x40, 0x7e) || tr(1, 0x80, 0xfc) { Some(2) } else { None } }
            else { None }
        }
        Dbcs => { if x < 0x80 { Some(1) } else if (0xa1..=0xfe).contains(&x) && tr(1, 0xa1, 0xfe) { Some(2) } else { None } }
        Big5 => { if x < 0x80 { Some(1) } else if (0x81..=0xfe).contains(&x) && (tr(1, 0x40, 0x7e) || tr(1, 0xa1, 0xfe)) { Some(2) } else { None } }
        Gbk => { if x < 0x80 { Some(1) } else if (0x81..=0xfe).contains(&x) && (tr(1, 0x40, 0x7e) || tr(1, 0x80, 0xfe)) { Some(2) } else { None } }
        Gb18030 => {
            if x < 0x80 { Some(1) }
            else if (0x81..=0xfe).contains(&x) {
                if tr(1, 0x40, 0x7e) || tr(1, 0x80, 0xfe) { Some(2) }
                else if tr(1, 0x30, 0x39) && tr(2, 0x81, 0xfe) && tr(3, 0x30, 0x39) { Some(4) }
                else { None }
            } else { None }
        }
        EmacsMule => {
            let n = match x { 0..=0x7f => return Some(1), 0x81..=0x8f => 2, 0x90..=0x99 => 3, 0x9a..=0x9b => 3, 0x9c..=0x9d => 4, _ => return None };
            if (1..n).all(|k| tr(k, 0xa0, 0xff)) { Some(n) } else { None }
        }
        EucTw => {
            if x < 0x80 { Some(1) }
            else if x == 0x8e { if tr(1, 0xa1, 0xb0) && tr(2, 0xa1, 0xfe) && tr(3, 0xa1, 0xfe) { Some(4) } else { None } }
            else if (0xa1..=0xfe).contains(&x) && tr(1, 0xa1, 0xfe) { Some(2) } else { None }
        }
    }
}

pub fn utf8_precise(r: &[u8]) -> Option<usize> {
    let x = *r.first()?;
    let l = if x < 0x80 { return Some(1) } else if (0xc2..=0xdf).contains(&x) { 2 } else if (0xe0..=0xef).contains(&x) { 3 } else if (0xf0..=0xf4).contains(&x) { 4 } else { return None };
    if r.len() < l { return None; }
    let (lo, hi) = match x { 0xe0 => (0xa0, 0xbf), 0xed => (0x80, 0x9f), 0xf0 => (0x90, 0xbf), 0xf4 => (0x80, 0x8f), _ => (0x80, 0xbf) };
    if r[1] < lo || r[1] > hi { return None; }
    for k in 2..l { if r[k] & 0xc0 != 0x80 { return None; } }
    Some(l)
}

/// Character length at b[i] the way CRuby's rb_enc_mbclen does (invalid: min length, clipped).
#[inline]
pub fn mbclen(enc: u8, b: &[u8], i: usize) -> usize {
    if enc == ENC_UTF8 && b[i] < 0x80 { return 1; }
    match precise_len(enc, b, i) { Some(l) => l, None => min_len(enc).min(b.len() - i).max(1) }
}

/// Is `p` a valid but incomplete prefix of a character (Onigmo's NEEDMORE)?
pub fn needmore(enc: u8, p: &[u8]) -> bool {
    let n = p.len();
    if n == 0 { return true; }
    let r = |k: usize, lo: u8, hi: u8| p[k] >= lo && p[k] <= hi;
    match kind(enc) {
        Utf8 => {
            let x = p[0];
            let l = if (0xc2..=0xdf).contains(&x) { 2 } else if (0xe0..=0xef).contains(&x) { 3 } else if (0xf0..=0xf4).contains(&x) { 4 } else { return false };
            if n >= l { return false; }
            let (lo, hi) = match x { 0xe0 => (0xa0, 0xbf), 0xed => (0x80, 0x9f), 0xf0 => (0x90, 0xbf), 0xf4 => (0x80, 0x8f), _ => (0x80, 0xbf) };
            (1..n).all(|k| if k == 1 { r(1, lo, hi) } else { r(k, 0x80, 0xbf) })
        }
        U16LE | U16BE => {
            if n == 1 { return true; }
            let w = if kind(enc) == U16LE { u16::from_le_bytes([p[0], p[1]]) } else { u16::from_be_bytes([p[0], p[1]]) };
            (0xd800..0xdc00).contains(&w) && (n == 2 || (n == 3 && (kind(enc) == U16LE || (0xdc..=0xdf).contains(&p[2]))))
        }
        U32LE | U32BE => n < 4,
        EucJp(_) => match p[0] { 0x8e | 0xa1..=0xfe => n == 1, 0x8f => n == 1 || (n == 2 && r(1, 0xa1, 0xfe)), _ => false },
        Sjis(_) => n == 1 && ((0x81..=0x9f).contains(&p[0]) || (0xe0..=0xfc).contains(&p[0])),
        Dbcs => n == 1 && (0xa1..=0xfe).contains(&p[0]),
        Big5 | Gbk => n == 1 && (0x81..=0xfe).contains(&p[0]),
        Gb18030 => r(0, 0x81, 0xfe) && match n { 1 => true, 2 => r(1, 0x30, 0x39), 3 => r(1, 0x30, 0x39) && r(2, 0x81, 0xfe), _ => false },
        EucTw => match p[0] { 0x8e => match n { 1 => true, 2 => r(1, 0xa1, 0xb0), 3 => r(1, 0xa1, 0xb0) && r(2, 0xa1, 0xfe), _ => false }, 0xa1..=0xfe => n == 1, _ => false },
        EmacsMule => {
            let l = match p[0] { 0x81..=0x8f => 2, 0x90..=0x9b => 3, 0x9c..=0x9d => 4, _ => return false };
            n < l && (1..n).all(|k| r(k, 0xa0, 0xff))
        }
        _ => false,
    }
}

/// Bytes covered by one invalid unit at b[i] (CRuby's str_scrub / transcoder rules).
pub fn invalid_len(enc: u8, b: &[u8], i: usize) -> usize {
    let rest = &b[i..];
    if needmore(enc, rest) { return rest.len(); }
    if min_len(enc) > 1 { return min_len(enc).min(rest.len()); }
    let mut clen = max_len(enc).min(rest.len());
    if clen <= 2 { return 1; }
    clen -= 1;
    while clen > 1 { if needmore(enc, &rest[..clen]) { break; } clen -= 1; }
    clen
}

pub fn is_valid(enc: u8, b: &[u8]) -> bool {
    match kind(enc) {
        Bin | Single | Dummy => true,
        Ascii => is_ascii(b),
        Utf8 => std::str::from_utf8(b).is_ok(),
        _ => { let mut i = 0; while i < b.len() { match precise_len(enc, b, i) { Some(l) => i += l, None => return false } } true }
    }
}

/// Codepoint of the (valid) character `ch` in the encoding's code space (Unicode for Unicode
/// encodings, the big-endian byte value for legacy multibyte ones).
pub fn char_code(enc: u8, ch: &[u8]) -> u32 {
    match kind(enc) {
        Utf8 => decode_utf8(ch, 0).0,
        U16LE | U16BE => {
            let le = kind(enc) == U16LE;
            let w = |k: usize| if le { u16::from_le_bytes([ch[k], ch[k + 1]]) } else { u16::from_be_bytes([ch[k], ch[k + 1]]) } as u32;
            if ch.len() >= 4 { 0x10000 + ((w(0) - 0xd800) << 10) + (w(2) - 0xdc00) } else if ch.len() >= 2 { w(0) } else { ch[0] as u32 }
        }
        U32LE | U32BE => if ch.len() >= 4 { if kind(enc) == U32LE { u32::from_le_bytes([ch[0], ch[1], ch[2], ch[3]]) } else { u32::from_be_bytes([ch[0], ch[1], ch[2], ch[3]]) } } else { ch[0] as u32 },
        _ => ch.iter().fold(0u32, |a, &x| (a << 8) | x as u32),
    }
}

/// Encode a code (as returned by char_code) into the encoding.  None if not representable.
pub fn code_to_bytes(enc: u8, cp: u32) -> Option<Vec<u8>> {
    let mut out = vec![];
    match kind(enc) {
        Utf8 => { if cp > 0x10ffff { return None; } encode_utf8(cp, &mut out); }
        Bin | Single | Dummy => { if cp > 0xff { return None; } out.push(cp as u8); }
        Ascii => { if cp > 0xff { return None; } out.push(cp as u8); }
        U16LE | U16BE => {
            if cp > 0x10ffff || (0xd800..0xe000).contains(&cp) { return None; }
            let ws: Vec<u16> = if cp >= 0x10000 { let c2 = cp - 0x10000; vec![0xd800 + (c2 >> 10) as u16, 0xdc00 + (c2 & 0x3ff) as u16] } else { vec![cp as u16] };
            for w in ws { out.extend_from_slice(&if kind(enc) == U16LE { w.to_le_bytes() } else { w.to_be_bytes() }); }
        }
        U32LE | U32BE => { if cp > 0x10ffff || (0xd800..0xe000).contains(&cp) { return None; } out.extend_from_slice(&if kind(enc) == U32LE { cp.to_le_bytes() } else { cp.to_be_bytes() }); }
        _ => {
            if cp < 0x100 { out.push(cp as u8); } else if cp < 0x10000 { out.push((cp >> 8) as u8); out.push(cp as u8); }
            else if cp < 0x1000000 { out.push((cp >> 16) as u8); out.push((cp >> 8) as u8); out.push(cp as u8); }
            else { out.extend_from_slice(&cp.to_be_bytes()); }
            if precise_len(enc, &out, 0) != Some(out.len()) { return None; }
        }
    }
    Some(out)
}

// ---- transcoding through Unicode ----------------------------------------------------------

fn jis_tables(k: EK) -> (&'static [u16], Option<&'static [u16]>) {
    match k { EucJp(true) => (&tb::CP51932[..], None), EucJp(false) => (&tb::EUCJP[..], Some(&tb::EUCJP_0212[..])), Sjis(true) => (&tb::CP932[..], None), _ => (&tb::SJIS[..], None) }
}
fn sjis_index(h: u8, l: u8) -> Option<usize> {
    let hi = if (0x81..=0x9f).contains(&h) { h - 0x81 } else if (0xe0..=0xfc).contains(&h) { h - 0xe0 + 31 } else { return None };
    if !(0x40..=0xfc).contains(&l) { return None; }
    Some(hi as usize * 189 + (l - 0x40) as usize)
}

/// Unicode scalar for a valid character of `enc`, None if it has no mapping.
pub fn to_unicode(enc: u8, ch: &[u8]) -> Option<u32> {
    let k = kind(enc);
    match k {
        Utf8 | U16LE | U16BE | U32LE | U32BE => Some(char_code(enc, ch)),
        Bin | Dummy => if ch[0] < 0x80 { Some(ch[0] as u32) } else { None },
        Ascii => if ch[0] < 0x80 { Some(ch[0] as u32) } else { None },
        Single => {
            if ch[0] < 0x80 { return Some(ch[0] as u32); }
            let t = single_table(enc)?;
            let u = t[(ch[0] - 0x80) as usize];
            if u == 0 { None } else { Some(u as u32) }
        }
        EucJp(_) => {
            if ch.len() == 1 { return if ch[0] < 0x80 { Some(ch[0] as u32) } else { None }; }
            if ch[0] == 0x8e { return if (0xa1..=0xdf).contains(&ch[1]) { Some(0xff61 + (ch[1] - 0xa1) as u32) } else { None }; }
            let (t, t3) = jis_tables(k);
            let (t, h, l) = if ch[0] == 0x8f { (t3?, ch[1], ch[2]) } else { (t, ch[0], ch[1]) };
            let u = t[(h - 0xa1) as usize * 94 + (l - 0xa1) as usize];
            if u == 0 { None } else { Some(u as u32) }
        }
        Sjis(cp) => {
            if ch.len() == 1 {
                if ch[0] < 0x80 { return Some(if !cp && ch[0] == 0x5c && false { 0xa5 } else { ch[0] as u32 }); }
                return if (0xa1..=0xdf).contains(&ch[0]) { Some(0xff61 + (ch[0] - 0xa1) as u32) } else { None };
            }
            let (t, _) = jis_tables(k);
            let u = t[sjis_index(ch[0], ch[1])?];
            if u == 0 { None } else { Some(u as u32) }
        }
        _ => if ch.len() == 1 && ch[0] < 0x80 { Some(ch[0] as u32) } else { None },
    }
}

type RevMap = std::collections::HashMap<u32, Vec<u8>>;
static REV: [OnceLock<RevMap>; NENC] = [const { OnceLock::new() }; NENC];
fn rev_map(enc: u8) -> &'static RevMap { REV[enc as usize].get_or_init(|| build_rev_map(enc)) }
fn build_rev_map(enc: u8) -> RevMap {
    let mut m = RevMap::new();
    let k = kind(enc);
    match k {
        Single => if let Some(t) = single_table(enc) { for (i, &u) in t.iter().enumerate() { if u != 0 { m.entry(u as u32).or_insert(vec![0x80 + i as u8]); } } },
        EucJp(_) => {
            let (t, t3) = jis_tables(k);
            for (i, &u) in t.iter().enumerate() { if u != 0 { m.entry(u as u32).or_insert(vec![0xa1 + (i / 94) as u8, 0xa1 + (i % 94) as u8]); } }
            if let Some(t3) = t3 { for (i, &u) in t3.iter().enumerate() { if u != 0 { m.entry(u as u32).or_insert(vec![0x8f, 0xa1 + (i / 94) as u8, 0xa1 + (i % 94) as u8]); } } }
            for x in 0xa1..=0xdfu8 { m.entry(0xff61 + (x - 0xa1) as u32).or_insert(vec![0x8e, x]); }
        }
        Sjis(_) => {
            let (t, _) = jis_tables(k);
            for (i, &u) in t.iter().enumerate() {
                if u == 0 { continue; }
                let hi = (i / 189) as u8; let h = if hi < 31 { 0x81 + hi } else { 0xe0 + hi - 31 };
                m.entry(u as u32).or_insert(vec![h, 0x40 + (i % 189) as u8]);
            }
            for x in 0xa1..=0xdfu8 { m.entry(0xff61 + (x - 0xa1) as u32).or_insert(vec![x]); }
        }
        _ => {}
    }
    m
}

/// Bytes for Unicode scalar `u` in `enc`, None if undefined there.
pub fn from_unicode(enc: u8, u: u32) -> Option<Vec<u8>> {
    match kind(enc) {
        Utf8 | U16LE | U16BE | U32LE | U32BE => code_to_bytes(enc, u),
        _ if u < 0x80 && ascii_compat(enc) => Some(vec![u as u8]),
        Bin | Ascii | Dummy => None,
        _ => rev_map(enc).get(&u).cloned(),
    }
}

/// Can this encoding pair be converted (CRuby has a converter)?
pub fn can_convert(from: u8, to: u8) -> bool {
    let ok = |e: u8| !matches!(kind(e), Dummy | Big5 | Gbk | Gb18030 | EucTw | Dbcs) || e == ENC_UTF16 || e == ENC_UTF32;
    if from == to { return true; }
    if from == ENC_BINARY || to == ENC_BINARY { return false; }
    ok(from) && ok(to)
}

// ---- transcoding ----------------------------------------------------------------------------

#[derive(Default)]
pub struct TcOpts {
    pub invalid_replace: bool,
    pub undef_replace: bool,
    pub replace: Option<Value>,
    pub xml: u8,      // 1 = :text, 2 = :attr
    pub newline: u8,  // 1 = universal, 2 = crlf, 3 = cr, 4 = lf
    pub fallback: Option<Value>,
}

fn show_bytes(b: &[u8]) -> String {
    let mut s = String::from("\"");
    for &x in b { if (0x20..0x7f).contains(&x) && x != b'"' && x != b'\\' { s.push(x as char); } else { s.push_str(&format!("\\x{:02X}", x)); } }
    s.push('"');
    s
}

pub fn parse_tc_opts(c: &mut Ctx, h: Value) -> Result<TcOpts, Esc> {
    let mut o = TcOpts::default();
    let get = |c: &mut Ctx, k: &str| crate::hash::hash_get_opt(c, h, sym_value(k));
    let replace_sym = sym_value("replace");
    if let Some(v) = get(c, "invalid")? { if v == replace_sym { o.invalid_replace = true; } else if v != NIL { return arg_err(c, "unknown value for invalid character option"); } }
    if let Some(v) = get(c, "undef")? { if v == replace_sym { o.undef_replace = true; } else if v != NIL { return arg_err(c, "unknown value for undefined character option"); } }
    if let Some(v) = get(c, "replace")? { if v != NIL { o.replace = Some(expect_str(c, v)?); } }
    if let Some(v) = get(c, "xml")? {
        if v == sym_value("text") { o.xml = 1; } else if v == sym_value("attr") { o.xml = 2; }
        else if v != NIL { let s = crate::builtins::kernel::inspect_str(c, v)?; return arg_err(c, format!("unexpected value for xml option: {}", s.trim_start_matches(':'))); }
    }
    if let Some(v) = get(c, "universal_newline")? { if v.truthy() { o.newline = 1; } }
    if let Some(v) = get(c, "crlf_newline")? { if v.truthy() { o.newline = 2; } }
    if let Some(v) = get(c, "cr_newline")? { if v.truthy() { o.newline = 3; } }
    if let Some(v) = get(c, "lf_newline")? { if v.truthy() { o.newline = 4; } }
    if let Some(v) = get(c, "newline")? {
        o.newline = if v == sym_value("universal") { 1 } else if v == sym_value("crlf") { 2 } else if v == sym_value("cr") { 3 } else if v == sym_value("lf") { 4 } else if v == NIL { 0 }
            else { let s = crate::builtins::kernel::inspect_str(c, v)?; return arg_err(c, format!("unexpected value for newline option: {}", s.trim_start_matches(':'))); };
    }
    if let Some(v) = get(c, "fallback")? { if v != NIL { o.fallback = Some(v); } }
    Ok(o)
}

/// Output encoding actually written for `to` (UTF-16/UTF-32 dummies write big-endian with BOM).
fn real_out(to: u8) -> u8 { if to == ENC_UTF16 { ENC_UTF16BE } else if to == ENC_UTF32 { ENC_UTF32BE } else { to } }

pub fn converter_not_found(c: &mut Ctx, from: &str, to: &str) -> Esc {
    raise_msg(c, c.vm().c_converter_not_found_error, &format!("code converter not found ({} to {})", from, to))
}

/// Convert `src` (in `from`) to `to`.  `same` = from == to (only scrub/xml/newline apply).
pub fn transcode(c: &mut Ctx, src: &[u8], from: u8, to: u8, o: &TcOpts) -> Result<Vec<u8>, Esc> {
    let same = from == to;
    if !same && !conv_ok(from, to) { return Err(converter_not_found(c, enc_name(from), enc_name(to))); }
    let ro = real_out(to);
    let (from_r, src) = if from == ENC_UTF16 || from == ENC_UTF32 {
        let big = from == ENC_UTF16;
        if big && src.starts_with(&[0xfe, 0xff]) { (ENC_UTF16BE, &src[2..]) } else if big && src.starts_with(&[0xff, 0xfe]) { (ENC_UTF16LE, &src[2..]) }
        else if !big && src.starts_with(&[0, 0, 0xfe, 0xff]) { (ENC_UTF32BE, &src[4..]) } else if !big && src.starts_with(&[0xff, 0xfe, 0, 0]) { (ENC_UTF32LE, &src[4..]) }
        else { (if big { ENC_UTF16BE } else { ENC_UTF32BE }, src) }
    } else { (from, src) };
    let mut out = Vec::with_capacity(src.len() + 4);
    if !same && (to == ENC_UTF16 || to == ENC_UTF32) { out.extend(code_to_bytes(ro, 0xfeff).unwrap()); }
    if o.xml == 2 { out.extend(from_unicode(ro, '"' as u32).unwrap_or_default()); }
    let rep_bytes = |c: &mut Ctx| -> Result<Vec<u8>, Esc> {
        if let Some(r) = o.replace {
            let rr = c.rstring(r);
            if rr.enc == ro || (is_ascii(&rr.b) && ascii_compat(ro) && ascii_compat(rr.enc)) { return Ok(rr.b.to_vec()); }
            let (b2, e2) = (rr.b.to_vec(), rr.enc);
            return transcode(c, &b2, e2, ro, &TcOpts::default());
        }
        Ok(if is_unicode(to) { from_unicode(ro, 0xfffd).unwrap() } else { from_unicode(ro, '?' as u32).unwrap_or(vec![b'?']) })
    };
    let mut i = 0;
    while i < src.len() {
        let Some(l) = precise_len(from_r, src, i) else {
            let n = invalid_len(from_r, src, i);
            if o.invalid_replace { let r = rep_bytes(c)?; out.extend(r); i += n; continue; }
            if same { out.extend_from_slice(&src[i..i + n]); i += n; continue; }
            let eb = &src[i..i + n];
            let msg = if i + n >= src.len() && max_len(from_r) > n && kind(from_r) != Ascii && eb[0] >= 0x80 || (min_len(from_r) > 1 && i + n >= src.len() && n < min_len(from_r)) {
                format!("incomplete {} on {}", show_bytes(eb), enc_name(from_r))
            } else if n > 1 || (kind(from_r) == Utf8 && (0xc2..=0xf4).contains(&eb[0]) && i + n < src.len()) {
                format!("{} followed by {} on {}", show_bytes(eb), show_bytes(&src[i + n..i + n + 1]), enc_name(from_r))
            } else { format!("{} on {}", show_bytes(eb), enc_name(from_r)) };
            return Err(raise_msg(c, c.vm().c_invalid_byte_seq_error, &msg));
        };
        let ch = &src[i..i + l];
        i += l;
        if same && o.xml == 0 && o.newline == 0 { out.extend_from_slice(ch); continue; }
        let u = if same && !is_unicode(from_r) && !(ascii_compat(from_r) && ch.len() == 1 && ch[0] < 0x80) { None } else { to_unicode(from_r, ch) };
        let Some(mut u) = u else {
            if same { out.extend_from_slice(ch); continue; }
            if o.undef_replace { let r = rep_bytes(c)?; out.extend(r); continue; }
            if let Some(fb) = o.fallback { let r = fallback(c, fb, ch, from_r, ro)?; out.extend(r); continue; }
            let msg = if to == ENC_UTF8 || from_r == ENC_BINARY && false { format!("{} from {} to {}", show_bytes(ch), enc_name(from), enc_name(to)) }
                      else { format!("{} to UTF-8 in conversion from {} to UTF-8 to {}", show_bytes(ch), enc_name(from), enc_name(to)) };
            return Err(raise_msg(c, c.vm().c_undef_conv_error, &msg));
        };
        // newline conversion
        if o.newline == 1 || o.newline == 4 {
            if u == 0x0d {
                if i < src.len() { if let Some(l2) = precise_len(from_r, src, i) { if to_unicode(from_r, &src[i..i + l2]) == Some(0x0a) { i += l2; } } }
                u = 0x0a;
            }
        } else if u == 0x0a && o.newline == 2 { out.extend(from_unicode(ro, 0x0d).unwrap_or_default()); }
        else if u == 0x0a && o.newline == 3 { u = 0x0d; }
        if o.xml != 0 {
            let esc: Option<&str> = match u { 0x26 => Some("&amp;"), 0x3c => Some("&lt;"), 0x3e => Some("&gt;"), 0x22 if o.xml == 2 => Some("&quot;"), _ => None };
            if let Some(e) = esc { for b in e.bytes() { out.extend(from_unicode(ro, b as u32).unwrap_or_default()); } continue; }
        }
        let enc_out = if same && !is_unicode(from_r) { Some(ch.to_vec()) } else { from_unicode(ro, u) };
        match enc_out {
            Some(bs) => out.extend(bs),
            None => {
                if o.xml != 0 { for b in format!("&#x{:X};", u).bytes() { out.extend(from_unicode(ro, b as u32).unwrap_or_default()); } continue; }
                if o.undef_replace { let r = rep_bytes(c)?; out.extend(r); continue; }
                if let Some(fb) = o.fallback { let r = fallback(c, fb, ch, from_r, ro)?; out.extend(r); continue; }
                let msg = if from_r == ENC_UTF8 || kind(from_r) == Utf8 { format!("U+{:04X} from UTF-8 to {}", u, enc_name(to)) }
                          else { format!("U+{:04X} to {} in conversion from {} to UTF-8 to {}", u, enc_name(to), enc_name(from), enc_name(to)) };
                return Err(raise_msg(c, c.vm().c_undef_conv_error, &msg));
            }
        }
    }
    if o.xml == 2 { out.extend(from_unicode(ro, '"' as u32).unwrap_or_default()); }
    Ok(out)
}

fn fallback(c: &mut Ctx, fb: Value, ch: &[u8], from: u8, to: u8) -> Result<Vec<u8>, Esc> {
    let key = c.str_new_enc(ch, from);
    let r = if is_hash(fb) { crate::hash::hash_aref(c, fb, key)? }
            else if is_proc(fb) || is_method_obj(fb) { funcall(c, fb, intern(b"call"), &[key], std::ptr::null())? }
            else { funcall(c, fb, intern(b"[]"), &[key], std::ptr::null())? };
    if r == NIL {
        let msg = format!("{} from {} to {}", if from == ENC_UTF8 { format!("U+{:04X}", decode_utf8(ch, 0).0) } else { show_bytes(ch) }, enc_name(from), enc_name(to));
        return Err(raise_msg(c, c.vm().c_undef_conv_error, &msg));
    }
    let s = expect_str(c, r)?;
    let (b2, e2) = (c.str_bytes(s).to_vec(), c.str_enc(s));
    if e2 == to { return Ok(b2); }
    match transcode(c, &b2, e2, to, &TcOpts::default()) {
        Ok(v) => Ok(v),
        Err(_) => arg_err(c, "too big fallback string"),
    }
}

fn conv_ok(from: u8, to: u8) -> bool {
    let bad = |e: u8| matches!(enc_name(e), "UTF-7" | "ISO-2022-JP" | "ISO-2022-JP-2" | "CP50220" | "CP50221" | "IBM037" | "ISO-2022-JP-KDDI" | "Emacs-Mule");
    !(bad(from) || bad(to)) && !(from == ENC_BINARY && !ascii_compat(to))
}

// ---- lookup ---------------------------------------------------------------------------------

pub fn find_enc(name: &[u8]) -> Option<u8> {
    let n = std::str::from_utf8(name).ok()?;
    if n.eq_ignore_ascii_case("internal") { return None; }
    if n.eq_ignore_ascii_case("external") || n.eq_ignore_ascii_case("locale") || n.eq_ignore_ascii_case("filesystem") { return Some(default_external()); }
    for (i, d) in ENCS.iter().enumerate() {
        if d.name.eq_ignore_ascii_case(n) || d.aliases.iter().any(|a| a.eq_ignore_ascii_case(n)) { return Some(i as u8); }
    }
    None
}
pub fn default_external() -> u8 { DEFAULT_EXTERNAL.load(Relaxed) }
pub fn default_internal() -> Option<u8> { let d = DEFAULT_INTERNAL.load(Relaxed); if d < 0 { None } else { Some(d as u8) } }

pub fn encoding_value(c: &Ctx, id: u8) -> Value { c.vm().enc_objs[id as usize] }

/// Encoding object, name string or object with to_str -> encoding id.
pub fn to_enc(c: &mut Ctx, v: Value) -> Result<u8, Esc> {
    if let Some(e) = c.data::<REnc>(v) { return Ok(e.id); }
    let s = if is_string(v) { v } else { expect_str(c, v)? };
    match find_enc(c.str_bytes(s)) {
        Some(e) => Ok(e),
        None => { let nm = String::from_utf8_lossy(c.str_bytes(s)).into_owned(); arg_err(c, format!("unknown encoding name - {}", nm)) }
    }
}
/// Like to_enc but returns None for unknown names (encode's "converter not found" path).
pub fn to_enc_opt(c: &mut Ctx, v: Value) -> Result<Result<u8, String>, Esc> {
    if let Some(e) = c.data::<REnc>(v) { return Ok(Ok(e.id)); }
    let s = expect_str(c, v)?;
    let nm = String::from_utf8_lossy(c.str_bytes(s)).into_owned();
    Ok(match find_enc(c.str_bytes(s)) { Some(e) => Ok(e), None => Err(nm) })
}

// ---- compatibility --------------------------------------------------------------------------

/// rb_enc_compatible for two strings; None if incompatible.
pub fn compatible(c: &Ctx, a: Value, b: Value) -> Option<u8> {
    let (ra, rb) = (c.rstring(a), c.rstring(b));
    let (ea, eb) = (ra.enc, rb.enc);
    if ea == eb { return Some(ea); }
    if rb.b.is_empty() { return Some(ea); }
    if ra.b.is_empty() { return Some(if ascii_compat(ea) && ascii_compat(eb) && is_ascii(&rb.b) { ea } else { eb }); }
    if !ascii_compat(ea) || !ascii_compat(eb) { return None; }
    let (a7, b7) = (is_ascii(&ra.b), is_ascii(&rb.b));
    if b7 { return Some(ea); }
    if a7 { return Some(eb); }
    None
}
pub fn compat_error(c: &mut Ctx, ea: u8, eb: u8) -> Esc {
    raise_msg(c, c.vm().c_compat_error, &format!("incompatible character encodings: {} and {}", enc_name_msg(ea), enc_name_msg(eb)))
}
pub fn check_compatible(c: &mut Ctx, a: Value, b: Value) -> Result<u8, Esc> {
    match compatible(c, a, b) { Some(e) => Ok(e), None => { let (ea, eb) = (c.str_enc(a), c.str_enc(b)); Err(compat_error(c, ea, eb)) } }
}

// ---- Encoding class -------------------------------------------------------------------------

fn push_name(c: &mut Ctx, out: Value, name: &str) { let s = c.str_new_enc(name.as_bytes(), ENC_ASCII); c.ary_push(out, s); }
fn id_of(c: &Ctx, v: Value) -> u8 { c.data::<REnc>(v).map(|e| e.id).unwrap_or(ENC_BINARY) }
fn enc_to_s(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(c.str_new_enc(enc_name(id_of(c, slf)).as_bytes(), ENC_ASCII)) }
fn enc_inspect(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let id = id_of(c, slf);
    let s = format!("#<Encoding:{}{}>", enc_name_msg(id), if is_dummy(id) { " (dummy)" } else { "" });
    Ok(c.str_new_enc(s.as_bytes(), ENC_ASCII))
}
fn enc_names(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV {
    let id = id_of(c, slf);
    let d = &ENCS[id as usize];
    let out = c.ary_new();
    push_name(c, out, d.name);
    for a in d.aliases {
        if (*a == "locale" || *a == "external" || *a == "filesystem") && id != default_external() { continue; }
        push_name(c, out, a);
    }
    if id == default_external() && id != ENC_UTF8 { for a in ["locale", "external", "filesystem"] { push_name(c, out, a); } }
    Ok(out)
}
fn enc_dummy_p(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::bool(is_dummy(id_of(c, slf)))) }
fn enc_ascii_compat_p(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(Value::bool(ascii_compat(id_of(c, slf)))) }
fn enc_s_find(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    if c.data::<REnc>(a[0]).is_some() { return Ok(a[0]); }
    let s = expect_str(c, a[0])?;
    if c.str_bytes(s).eq_ignore_ascii_case(b"internal") { return Ok(match default_internal() { Some(e) => encoding_value(c, e), None => NIL }); }
    let id = to_enc(c, s)?;
    Ok(encoding_value(c, id))
}
fn list_order() -> Vec<u8> { let mut v: Vec<u8> = (0..NENC as u8).collect(); v.swap(0, 1); v }
fn enc_s_list(c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV {
    let out = c.ary_new();
    for i in list_order() { c.ary_push(out, encoding_value(c, i)); }
    Ok(out)
}
fn enc_s_name_list(c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV {
    let out = c.ary_new();
    for i in list_order() {
        let d = &ENCS[i as usize];
        push_name(c, out, d.name);
        for a in d.aliases { push_name(c, out, a); }
    }
    push_name(c, out, "internal");
    Ok(out)
}
fn enc_s_aliases(c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV {
    let h = c.hash_new();
    for i in list_order() {
        let d = &ENCS[i as usize];
        for a in d.aliases {
            let k = c.str_new_enc(a.as_bytes(), ENC_ASCII);
            let tgt = if matches!(*a, "locale" | "external" | "filesystem") { enc_name(default_external()) } else { d.name };
            let v = c.str_new_enc(tgt.as_bytes(), ENC_ASCII);
            crate::hash::hash_aset(c, h, k, v)?;
        }
    }
    Ok(h)
}
fn enc_s_default_external(c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(encoding_value(c, default_external())) }
fn enc_s_default_internal(c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(match default_internal() { Some(e) => encoding_value(c, e), None => NIL }) }
fn enc_s_set_default_external(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    if a[0] == NIL { return arg_err(c, "default external can not be nil"); }
    let e = to_enc(c, a[0])?;
    DEFAULT_EXTERNAL.store(e, Relaxed);
    Ok(a[0])
}
fn enc_s_set_default_internal(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    if a[0] == NIL { DEFAULT_INTERNAL.store(-1, Relaxed); return Ok(NIL); }
    let e = to_enc(c, a[0])?;
    DEFAULT_INTERNAL.store(e as i32, Relaxed);
    Ok(a[0])
}
fn enc_s_locale_charmap(c: &mut Ctx, _s: Value, _a: &[Value], _b: *const Block) -> RV { Ok(c.str_new_enc(b"UTF-8", ENC_ASCII)) }
/// Encoding of an object for compatible?: strings, symbols, regexps, encodings.
fn obj_enc(c: &mut Ctx, v: Value) -> Result<Option<Value>, Esc> {
    if is_string(v) { return Ok(Some(v)); }
    if c.data::<REnc>(v).is_some() { return Ok(Some(c.str_new_enc(b"", id_of(c, v)))); }
    if v.is_sym() { let b = crate::symbol::sym_bytes(v.as_sym()); return Ok(Some(c.str_new_enc(b, if is_ascii(b) { ENC_ASCII } else { ENC_UTF8 }))); }
    if crate::regexp::is_regexp(c, v) { let s = funcall(c, v, intern(b"source"), &[], std::ptr::null())?; return Ok(Some(s)); }
    Ok(None)
}
fn enc_s_compatible_p(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV {
    let (Some(x), Some(y)) = (obj_enc(c, a[0])?, obj_enc(c, a[1])?) else { return Ok(NIL) };
    // Encoding objects behave as empty strings that still need ASCII compatibility.
    if c.data::<REnc>(a[0]).is_some() || c.data::<REnc>(a[1]).is_some() {
        let (ex, ey) = (c.str_enc(x), c.str_enc(y));
        if ex == ey { return Ok(encoding_value(c, ex)); }
        if !ascii_compat(ex) || !ascii_compat(ey) { return Ok(NIL); }
        if c.data::<REnc>(a[1]).is_some() { if is_string(a[0]) && is_ascii(c.str_bytes(x)) { return Ok(encoding_value(c, ey)); } return Ok(if c.data::<REnc>(a[0]).is_some() { NIL } else { encoding_value(c, ex) }); }
        if is_ascii(c.str_bytes(y)) { return Ok(encoding_value(c, ex)); }
        return Ok(NIL);
    }
    Ok(match compatible(c, x, y) { Some(e) => encoding_value(c, e), None => NIL })
}
fn enc_dump(c: &mut Ctx, slf: Value, _a: &[Value], _b: *const Block) -> RV { Ok(c.str_new(enc_name(id_of(c, slf)).as_bytes())) }
fn enc_s_load(c: &mut Ctx, _s: Value, a: &[Value], _b: *const Block) -> RV { let e = to_enc(c, a[0])?; Ok(encoding_value(c, e)) }

/// Constant names for an encoding name (CRuby's set_encoding_const).
fn const_names(n: &str) -> Vec<String> {
    let first = n.as_bytes()[0];
    if !first.is_ascii_alphabetic() { return vec![]; }
    let mut s: String = n.chars().map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' }).collect();
    let mut out = vec![];
    if first.is_ascii_lowercase() { s = format!("{}{}", s[..1].to_ascii_uppercase(), &s[1..]); }
    out.push(s.clone());
    if s.bytes().any(|x| x.is_ascii_lowercase()) { out.push(s.to_ascii_uppercase()); }
    out
}

/// Boot: no context runs yet, so the Encoding objects are allocated directly.
pub fn init(vmr: &mut Vm) {
    let ek = vmr.c_encoding;
    for i in 0..NENC {
        let e = crate::gc::alloc(ek, Body::Data(Box::new(REnc { id: i as u8 })));
        obj_set_flags(e, FL_FROZEN);
        vmr.enc_objs.push(e);
        vmr.pinned.push(e);
    }
    for (i, d) in ENCS.iter().enumerate() {
        let ev = vmr.enc_objs[i];
        for n in std::iter::once(&d.name).chain(d.aliases.iter()) {
            if matches!(*n, "locale" | "external" | "filesystem") { continue; }
            for cn in const_names(n) { const_set(ek, intern(cn.as_bytes()), ev); }
        }
    }
    define_cfunc(ek, "to_s", enc_to_s, 0);
    define_cfunc(ek, "name", enc_to_s, 0);
    define_cfunc(ek, "inspect", enc_inspect, 0);
    define_cfunc(ek, "names", enc_names, 0);
    define_cfunc(ek, "dummy?", enc_dummy_p, 0);
    define_cfunc(ek, "ascii_compatible?", enc_ascii_compat_p, 0);
    define_cfunc(ek, "_dump", enc_dump, -1);
    define_singleton_cfunc(ek, "_load", enc_s_load, 1);
    define_singleton_cfunc(ek, "find", enc_s_find, 1);
    define_singleton_cfunc(ek, "list", enc_s_list, 0);
    define_singleton_cfunc(ek, "name_list", enc_s_name_list, 0);
    define_singleton_cfunc(ek, "aliases", enc_s_aliases, 0);
    define_singleton_cfunc(ek, "default_external", enc_s_default_external, 0);
    define_singleton_cfunc(ek, "default_internal", enc_s_default_internal, 0);
    define_singleton_cfunc(ek, "default_external=", enc_s_set_default_external, 1);
    define_singleton_cfunc(ek, "default_internal=", enc_s_set_default_internal, 1);
    define_singleton_cfunc(ek, "locale_charmap", enc_s_locale_charmap, 0);
    define_singleton_cfunc(ek, "compatible?", enc_s_compatible_p, 2);
}

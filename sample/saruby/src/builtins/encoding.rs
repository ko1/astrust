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
    Cesu8,           // UTF-8 with surrogate pairs (6 bytes) for U+10000.., no 4-byte forms
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
    e!("CESU-8", [], Cesu8),
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
// the locale's encoding (rb_locale_encindex) and nl_langinfo(CODESET), set once at boot
static LOCALE: AtomicU8 = AtomicU8::new(ENC_UTF8);
static LOCALE_CHARMAP: OnceLock<Vec<u8>> = OnceLock::new();
/// setlocale + nl_langinfo once; default_external starts as the locale's encoding (an unknown charmap is UTF-8).
pub fn init_locale() {
    let cs = LOCALE_CHARMAP.get_or_init(crate::builtins::sys::locale_codeset);
    let special = std::str::from_utf8(cs).is_ok_and(|n| ["locale", "external", "filesystem", "internal"].iter().any(|x| x.eq_ignore_ascii_case(n)));
    let e = if special { None } else { find_enc(cs) }.unwrap_or(ENC_UTF8);
    LOCALE.store(e, Relaxed);
    DEFAULT_EXTERNAL.store(e, Relaxed);
}
pub fn locale_enc() -> u8 { LOCALE.load(Relaxed) }
/// -E ext[:int] / -U (command line, after boot): unknown names are an error as in CRuby.
pub fn set_cmdline_encodings(ext: Option<&str>, int: Option<&str>) -> Result<(), String> {
    let f = |n: &str| find_enc(n.as_bytes()).ok_or_else(|| format!("unknown encoding name - {}", n));
    if let Some(e) = ext.filter(|e| !e.is_empty()) { DEFAULT_EXTERNAL.store(f(e)?, Relaxed); }
    if let Some(i) = int.filter(|i| !i.is_empty()) { DEFAULT_INTERNAL.store(f(i)? as i32, Relaxed); }
    Ok(())
}

#[inline] pub fn kind(enc: u8) -> EK { ENCS.get(enc as usize).map(|d| d.kind).unwrap_or(Bin) }
pub fn enc_name(enc: u8) -> &'static str { ENCS.get(enc as usize).map(|d| d.name).unwrap_or("ASCII-8BIT") }
/// Name as it appears in error messages ("BINARY (ASCII-8BIT)" for binary).
pub fn enc_name_msg(enc: u8) -> String { if enc == ENC_BINARY { "BINARY (ASCII-8BIT)".into() } else { enc_name(enc).into() } }
pub fn is_dummy(enc: u8) -> bool { kind(enc) == Dummy || enc == ENC_UTF16 || enc == ENC_UTF32 }
pub fn ascii_compat(enc: u8) -> bool { !matches!(kind(enc), U16LE | U16BE | U32LE | U32BE | Dummy) }
pub fn is_unicode(enc: u8) -> bool { matches!(kind(enc), Utf8 | Cesu8 | U16LE | U16BE | U32LE | U32BE) || enc == ENC_UTF16 || enc == ENC_UTF32 }
/// Every byte is one character.
#[inline] pub fn is_single_byte(enc: u8) -> bool { matches!(kind(enc), Bin | Ascii | Single | Dummy) }
pub fn min_len(enc: u8) -> usize { match kind(enc) { U16LE | U16BE => 2, U32LE | U32BE => 4, _ => 1 } }
pub fn max_len(enc: u8) -> usize { match kind(enc) { Cesu8 => 6, Utf8 | U32LE | U32BE | U16LE | U16BE | Gb18030 | EucTw | EmacsMule => 4, EucJp(_) => 3, Sjis(_) | Dbcs | Big5 | Gbk => 2, _ => 1 } }

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

/// rb_enc_isprint of the single byte `b` (>= 0x80) in `enc` (Onigmo's ctype table, generated from CRuby).
pub fn hi_byte_print(enc: u8, b: u8) -> bool {
    static IDX: OnceLock<[u8; NENC]> = OnceLock::new();
    let idx = IDX.get_or_init(|| {
        let mut t = [255u8; NENC];
        for (i, d) in ENCS.iter().enumerate() { if let Some(p) = tb::PRINT_HI.iter().position(|x| x.0 == d.name) { t[i] = p as u8; } }
        t
    });
    match idx.get(enc as usize) { Some(&i) if i != 255 && b >= 0x80 => tb::PRINT_HI[i as usize].1[((b - 0x80) >> 3) as usize] & (1 << (b & 7)) != 0, _ => false }
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
        Cesu8 => cesu8_precise(r),
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

/// enc/cesu_8.c's mbc_enc_len: UTF-8 up to 3 bytes (ED A0.. only as the first half of a pair), and
/// a high surrogate followed by a low one (6 bytes).
pub fn cesu8_precise(r: &[u8]) -> Option<usize> {
    let x = *r.first()?;
    if x == 0xed && r.len() >= 2 && (0xa0..=0xaf).contains(&r[1]) {
        let ok = r.len() >= 6 && r[2] & 0xc0 == 0x80 && r[3] == 0xed && (0xb0..=0xbf).contains(&r[4]) && r[5] & 0xc0 == 0x80;
        return if ok { Some(6) } else { None };
    }
    if (0xf0..=0xff).contains(&x) { return None; }
    utf8_precise(r)
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
        Cesu8 => {
            // the first half of a pair needs the rest of the 6 bytes
            if p[0] == 0xed && n >= 2 && (0xa0..=0xaf).contains(&p[1]) {
                let pat: [(u8, u8); 6] = [(0xed, 0xed), (0xa0, 0xaf), (0x80, 0xbf), (0xed, 0xed), (0xb0, 0xbf), (0x80, 0xbf)];
                return n < 6 && (0..n).all(|k| r(k, pat[k].0, pat[k].1));
            }
            p[0] < 0xf0 && needmore(ENC_UTF8, p)
        }
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
        Cesu8 => {
            let d3 = |p: &[u8]| ((p[0] as u32 & 0x0f) << 12) | ((p[1] as u32 & 0x3f) << 6) | (p[2] as u32 & 0x3f);
            match ch.len() { 6 => 0x10000 + ((d3(ch) & 0x3ff) << 10) + (d3(&ch[3..]) & 0x3ff), 3 => d3(ch), _ => decode_utf8(ch, 0).0 }
        }
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
/// rb_enc_codelen + rb_enc_mbcput for a multibyte encoding other than UTF-8 (Onigmo's code_to_mbclen /
/// code_to_mbc): the first `len` bytes it writes whenever the length is positive, even for an invalid character.
/// None: the length is an error.
pub fn onig_mbcput(enc: u8, cp: u32) -> Option<Vec<u8>> {
    let len: usize = match kind(enc) {
        Sjis(_) => {
            if cp <= 0x80 || (0xa0..=0xdf).contains(&cp) || (0xfd..=0xff).contains(&cp) { 1 }
            else if cp < 0x100 || cp > 0xffff || !matches!(cp & 0xff, 0x40..=0x7e | 0x80..=0xfc) { return None }
            else { 2 }
        }
        EucJp(_) => if cp < 0x80 { 1 } else if cp > 0xff_ffff { return None } else if cp & 0xff80_8080 == 0x80_8080 { 3 } else if cp & 0xffff_8080 == 0x8080 { 2 } else { return None },
        Dbcs | Big5 | Gbk => if cp <= 0xff { 1 } else if cp <= 0xffff { 2 } else { return None },
        Gb18030 | EucTw => if cp & 0xff00_0000 != 0 { 4 } else if cp & 0xff_0000 != 0 { 3 } else if cp & 0xff00 != 0 { 2 } else { 1 },
        EmacsMule => if cp < 0x80 { 1 } else if cp & 0xff00_0000 >= 0x8000_0000 { 4 } else if cp & 0xff_0000 >= 0x80_0000 { 3 } else if cp & 0xff00 >= 0x8000 { 2 } else { return None },
        U16LE | U16BE => {
            let ws: Vec<u16> = if cp > 0xffff { vec![((cp >> 10) + 0xd7c0) as u16, ((cp & 0x3ff) + 0xdc00) as u16] } else { vec![cp as u16] };
            return Some(ws.iter().flat_map(|w| if kind(enc) == U16LE { w.to_le_bytes() } else { w.to_be_bytes() }).collect());
        }
        U32LE => return Some(cp.to_le_bytes().to_vec()),
        U32BE => return Some(cp.to_be_bytes().to_vec()),
        _ => return code_to_bytes(enc, cp),
    };
    let b = cp.to_be_bytes();
    // code_to_mbc: mb4 writes from the highest non-zero byte on, the others each non-zero high byte
    let mut out: Vec<u8> = if matches!(kind(enc), Gb18030 | EucTw) { b[(cp.leading_zeros() as usize / 8).min(3)..].to_vec() }
        else { b[..3].iter().copied().filter(|&x| x != 0).chain([b[3]]).collect() };
    out.resize(len, 0);
    Some(out)
}
pub fn code_to_bytes(enc: u8, cp: u32) -> Option<Vec<u8>> {
    let mut out = vec![];
    match kind(enc) {
        Utf8 => { if cp > 0x10ffff { return None; } encode_utf8(cp, &mut out); }
        Cesu8 => {
            if cp > 0x10ffff || (0xd800..0xe000).contains(&cp) { return None; }
            if cp < 0x10000 { encode_utf8(cp, &mut out); } else { encode_utf8((cp >> 10) + 0xd7c0, &mut out); encode_utf8((cp & 0x3ff) + 0xdc00, &mut out); }
        }
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
        Utf8 | Cesu8 | U16LE | U16BE | U32LE | U32BE => Some(char_code(enc, ch)),
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
        Utf8 | Cesu8 | U16LE | U16BE | U32LE | U32BE => code_to_bytes(enc, u),
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

/// Convert `src` (in `from`) to `to` through the converter (transcode.rs); with from == to only
/// scrubbing and the decorators apply.
pub fn transcode(c: &mut Ctx, src: &[u8], from: u8, to: u8, o: &TcOpts) -> Result<Vec<u8>, Esc> {
    use crate::builtins::econv as ev;
    let mut f = 0;
    if o.invalid_replace { f |= ev::INVALID_REPLACE; }
    if o.undef_replace || o.replace.is_some() && !o.invalid_replace { f |= ev::UNDEF_REPLACE; }
    match o.xml { 1 => f |= ev::XML_TEXT_DECORATOR | ev::UNDEF_HEX_CHARREF, 2 => f |= ev::XML_ATTR_CONTENT_DECORATOR | ev::XML_ATTR_QUOTE_DECORATOR | ev::UNDEF_HEX_CHARREF, _ => {} }
    match o.newline { 1 => f |= ev::UNIVERSAL_NEWLINE_DECORATOR, 2 => f |= ev::CRLF_NEWLINE_DECORATOR, 3 => f |= ev::CR_NEWLINE_DECORATOR, 4 => f |= ev::LF_NEWLINE_DECORATOR, _ => {} }
    let opts = crate::builtins::transcode::Opts { replace: o.replace, fallback: o.fallback };
    crate::builtins::transcode::convert_bytes(c, src, from, to, f, &opts)
}

// ---- lookup ---------------------------------------------------------------------------------

pub fn find_enc(name: &[u8]) -> Option<u8> {
    let n = std::str::from_utf8(name).ok()?;
    if n.eq_ignore_ascii_case("internal") { return Some(default_internal().unwrap_or(ENC_BINARY)); }   // unset: BINARY (force_encoding)
    if n.eq_ignore_ascii_case("locale") { return Some(locale_enc()); }
    if n.eq_ignore_ascii_case("external") || n.eq_ignore_ascii_case("filesystem") { return Some(default_external()); }
    for (i, d) in ENCS.iter().enumerate() {
        if d.name.eq_ignore_ascii_case(n) || d.aliases.iter().any(|a| a.eq_ignore_ascii_case(n)) { return Some(i as u8); }
    }
    None
}
pub fn default_external() -> u8 { DEFAULT_EXTERNAL.load(Relaxed) }
pub fn default_internal() -> Option<u8> { let d = DEFAULT_INTERNAL.load(Relaxed); if d < 0 { None } else { Some(d as u8) } }
/// rb_external_str_new_with_enc: bytes in `enc`, converted to default_internal when one is set
/// (kept in `enc` when that conversion fails).
pub fn external_str(c: &mut Ctx, b: Vec<u8>, enc: u8) -> Value {
    if let Some(i) = default_internal() {
        if i != enc && enc != ENC_BINARY && ascii_compat(enc) {
            if crate::string::is_ascii(&b) && ascii_compat(i) { return c.str_from_vec_enc(b, i); }
            if let Some(o) = crate::builtins::econv::convert_string(enc_name(enc), enc_name(i), &b) { return c.str_from_vec_enc(o, i); }
        }
    }
    c.str_from_vec_enc(b, enc)
}

pub fn encoding_value(c: &Ctx, id: u8) -> Value { c.vm().enc_objs.borrow()[id as usize] }
pub fn is_encoding(c: &Ctx, v: Value) -> bool { c.data::<REnc>(v).is_some() }

/// Encoding object, name string or object with to_str -> encoding id.
pub fn to_enc(c: &mut Ctx, v: Value) -> Result<u8, Esc> {
    if let Some(e) = c.data::<REnc>(v) { return Ok(e.id); }
    let s = if is_string(v) { v } else { expect_str(c, v)? };
    if !ascii_compat(c.str_enc(s)) { return arg_err(c, "invalid encoding name (non ASCII)"); }
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
    let (ea, eb) = (ra.enc(), rb.enc());
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
    raise_msg(c, c.vm().c_compat_error.get(), &format!("incompatible character encodings: {} and {}", enc_name_msg(ea), enc_name_msg(eb)))
}
pub fn check_compatible(c: &mut Ctx, a: Value, b: Value) -> Result<u8, Esc> {
    match compatible(c, a, b) { Some(e) => Ok(e), None => { let (ea, eb) = (c.str_enc(a), c.str_enc(b)); Err(compat_error(c, ea, eb)) } }
}

// ---- Encoding class -------------------------------------------------------------------------

fn push_name(c: &mut Ctx, out: Value, name: &str) { let s = c.str_new_enc(name.as_bytes(), ENC_ASCII); c.ary_push(out, s); }
fn id_of(c: &Ctx, v: Value) -> u8 { c.data::<REnc>(v).map(|e| e.id).unwrap_or(ENC_BINARY) }
fn enc_to_s(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { Ok(c.str_new_enc(enc_name(id_of(c, slf)).as_bytes(), ENC_ASCII)) }
fn enc_inspect(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    let id = id_of(c, slf);
    let s = format!("#<Encoding:{}{}>", enc_name_msg(id), if is_dummy(id) { " (dummy)" } else { "" });
    Ok(c.str_new_enc(s.as_bytes(), ENC_ASCII))
}
fn enc_names(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    let id = id_of(c, slf);
    let d = &ENCS[id as usize];
    let out = c.ary_new();
    push_name(c, out, d.name);
    for a in d.aliases {
        if (*a == "external" || *a == "filesystem") && id != default_external() { continue; }
        push_name(c, out, a);
    }
    if id == default_external() && id != ENC_UTF8 { for a in ["external", "filesystem"] { push_name(c, out, a); } }
    Ok(out)
}
fn enc_dummy_p(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { Ok(Value::bool(is_dummy(id_of(c, slf)))) }
fn enc_ascii_compat_p(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { Ok(Value::bool(ascii_compat(id_of(c, slf)))) }
fn enc_s_find(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    if c.data::<REnc>(a[0]).is_some() { return Ok(a[0]); }
    let s = expect_str(c, a[0])?;
    if c.str_bytes(s).eq_ignore_ascii_case(b"internal") { return Ok(match default_internal() { Some(e) => encoding_value(c, e), None => NIL }); }
    let id = to_enc(c, s)?;
    Ok(encoding_value(c, id))
}
fn list_order() -> Vec<u8> { let mut v: Vec<u8> = (0..NENC as u8).collect(); v.swap(0, 1); v }
fn enc_s_list(c: &mut Ctx, _s: Value, _a: &[Value], _b: Blk) -> RV {
    let out = c.ary_new();
    for i in list_order() { c.ary_push(out, encoding_value(c, i)); }
    Ok(out)
}
fn enc_s_name_list(c: &mut Ctx, _s: Value, _a: &[Value], _b: Blk) -> RV {
    let out = c.ary_new();
    for i in list_order() {
        let d = &ENCS[i as usize];
        push_name(c, out, d.name);
        for a in d.aliases { push_name(c, out, a); }
    }
    push_name(c, out, "internal");
    Ok(out)
}
fn enc_s_aliases(c: &mut Ctx, _s: Value, _a: &[Value], _b: Blk) -> RV {
    let h = c.hash_new();
    for i in list_order() {
        let d = &ENCS[i as usize];
        for a in d.aliases {
            let k = c.str_new_enc(a.as_bytes(), ENC_ASCII);
            let tgt = if matches!(*a, "external" | "filesystem") { enc_name(default_external()) } else { d.name };
            let v = c.str_new_enc(tgt.as_bytes(), ENC_ASCII);
            crate::hash::hash_aset(c, h, k, v)?;
        }
    }
    Ok(h)
}
fn enc_s_default_external(c: &mut Ctx, _s: Value, _a: &[Value], _b: Blk) -> RV { Ok(encoding_value(c, default_external())) }
fn enc_s_default_internal(c: &mut Ctx, _s: Value, _a: &[Value], _b: Blk) -> RV { Ok(match default_internal() { Some(e) => encoding_value(c, e), None => NIL }) }
fn enc_s_set_default_external(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    if a[0] == NIL { return arg_err(c, "default external can not be nil"); }
    let e = to_enc(c, a[0])?;
    DEFAULT_EXTERNAL.store(e, Relaxed);
    Ok(a[0])
}
fn enc_s_set_default_internal(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    if a[0] == NIL { DEFAULT_INTERNAL.store(-1, Relaxed); return Ok(NIL); }
    let e = to_enc(c, a[0])?;
    DEFAULT_INTERNAL.store(e as i32, Relaxed);
    Ok(a[0])
}
fn enc_s_locale_charmap(c: &mut Ctx, _s: Value, _a: &[Value], _b: Blk) -> RV {
    let cs = LOCALE_CHARMAP.get().map(|v| v.as_slice()).unwrap_or(b"UTF-8");
    Ok(c.str_new_enc(cs, ENC_ASCII))
}
/// Encoding of an object for compatible?: strings, symbols, regexps, encodings.
/// rb_enc_get_index for Encoding.compatible?: (index, Some(String) when `v` is a String).
fn obj_enc(c: &mut Ctx, v: Value) -> Result<Option<(u8, Option<Value>)>, Esc> {
    if is_string(v) { return Ok(Some((c.str_enc(v), Some(v)))); }
    if c.data::<REnc>(v).is_some() { return Ok(Some((id_of(c, v), None))); }
    if v.is_sym() { return Ok(Some((crate::builtins::symbol_methods::sym_encoding(v.as_sym()), None))); }
    if crate::regexp::is_regexp(c, v) { let e = funcall(c, v, intern(b"encoding"), &[], std::ptr::null())?; return Ok(Some((id_of(c, e), None))); }
    Ok(None)
}
/// rb_enc_compatible / enc_compatible_latter, transcribed (the swap leaves enc1/enc2 as they were).
fn enc_s_compatible_p(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    let (Some((i1, s1)), Some((i2, s2))) = (obj_enc(c, a[0])?, obj_enc(c, a[1])?) else { return Ok(NIL) };
    let (e1, e2) = (i1, i2);
    if i1 == i2 { return Ok(encoding_value(c, e1)); }
    let seven = |c: &Ctx, s: Value| ascii_compat(c.str_enc(s)) && is_ascii(c.str_bytes(s));
    if let Some(y) = s2 { if c.str_bytes(y).is_empty() { return Ok(encoding_value(c, e1)); } }
    if let (Some(x), Some(y)) = (s1, s2) { if c.str_bytes(x).is_empty() { return Ok(encoding_value(c, if ascii_compat(e1) && seven(c, y) { e1 } else { e2 })); } }
    if !ascii_compat(e1) || !ascii_compat(e2) { return Ok(NIL); }
    if s2.is_none() && i2 == ENC_ASCII { return Ok(encoding_value(c, e1)); }
    if s1.is_none() && i1 == ENC_ASCII { return Ok(encoding_value(c, e2)); }
    let (s1, s2) = if s1.is_none() { (s2, s1) } else { (s1, s2) };
    if let Some(x) = s1 {
        let cr1 = seven(c, x);
        if let Some(y) = s2 {
            let cr2 = seven(c, y);
            if cr1 != cr2 { return Ok(encoding_value(c, if cr1 { e2 } else { e1 })); }
            if cr2 { return Ok(encoding_value(c, e1)); }
        }
        if cr1 { return Ok(encoding_value(c, e2)); }
    }
    Ok(NIL)
}
fn enc_dump(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV { Ok(c.str_new(enc_name(id_of(c, slf)).as_bytes())) }
fn enc_s_load(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV { let e = to_enc(c, a[0])?; Ok(encoding_value(c, e)) }

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

pub fn init(c: &mut Ctx) {
    let vmr = c.vm();
    let ek = vmr.c_encoding.get();
    for i in 0..NENC {
        let e = c.alloc(ek, Body::Data(Box::new(REnc { id: i as u8 })));
        obj_set_flags(e, FL_FROZEN);
        vmr.enc_objs.borrow_mut().push(e);
        vmr.pinned.borrow_mut().push(e);
    }
    for (i, d) in ENCS.iter().enumerate() {
        let ev = vmr.enc_objs.borrow()[i];
        for n in std::iter::once(&d.name).chain(d.aliases.iter()) {
            if matches!(*n, "locale" | "external" | "filesystem") { continue; }
            for cn in const_names(n) { const_set(c, ek, intern(cn.as_bytes()), ev); }
        }
    }
    let (u0, u1, u2) = char::UNICODE_VERSION;
    let uv = c.str_new_enc(format!("{}.{}.{}", u0, u1, u2).as_bytes(), ENC_ASCII); obj_set_flags(uv, FL_FROZEN);
    const_set(c, ek, intern(b"UNICODE_VERSION"), uv);
    define_cfunc(c, ek, "name", enc_to_s, 0);
    crate::builtins::object::alias_builtin(c, ek, "to_s", "name");
    define_cfunc(c, ek, "inspect", enc_inspect, 0);
    define_cfunc(c, ek, "names", enc_names, 0);
    define_cfunc(c, ek, "dummy?", enc_dummy_p, 0);
    define_cfunc(c, ek, "ascii_compatible?", enc_ascii_compat_p, 0);
    define_cfunc(c, ek, "_dump", enc_dump, -1);
    define_singleton_cfunc(c, ek, "_load", enc_s_load, 1);
    define_singleton_cfunc(c, ek, "find", enc_s_find, 1);
    define_singleton_cfunc(c, ek, "list", enc_s_list, 0);
    define_singleton_cfunc(c, ek, "name_list", enc_s_name_list, 0);
    define_singleton_cfunc(c, ek, "aliases", enc_s_aliases, 0);
    define_singleton_cfunc(c, ek, "default_external", enc_s_default_external, 0);
    define_singleton_cfunc(c, ek, "default_internal", enc_s_default_internal, 0);
    define_singleton_cfunc(c, ek, "default_external=", enc_s_set_default_external, 1);
    define_singleton_cfunc(c, ek, "default_internal=", enc_s_set_default_internal, 1);
    define_singleton_cfunc(c, ek, "locale_charmap", enc_s_locale_charmap, 0);
    define_singleton_cfunc(c, ek, "compatible?", enc_s_compatible_p, 2);
}

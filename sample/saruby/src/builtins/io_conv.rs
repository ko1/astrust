// IO encoding conversion: io.c's readconv / cbuf / writeconv on the engine of econv.rs.
// Reads convert first (rbuf -> readconv -> cbuf) and the char-oriented readers split the converted
// text; writes go through rb_str_encode and, for ASCII-incompatible or stateful externals, a
// writeconv that is finished at close.  Nothing here touches the Ruby heap: io.rs turns the
// results (CErr) into Strings and exceptions.
#![forbid(unsafe_code)]
use crate::builtins::econv::{self as ev, Econv, Res};
use crate::builtins::encoding as en;
use crate::builtins::io::{fill, RIo};
use crate::object::ENC_BINARY;

pub const STATEFUL_DECORATOR_MASK: u32 = 0x00f0_0000;
pub const NEWLINE_DECORATOR_READ_MASK: u32 = 0x0f00;
pub const NEWLINE_DECORATOR_WRITE_MASK: u32 = 0x7000;
/// IO_CBUF_CAPA_MIN
const CBUF_MIN: usize = 128 * 1024;

/// rb_io_t's readconv / cbuf / writeconv; made on first use.
#[derive(Default)]
pub struct IoConv {
    rc: Option<Box<Econv>>,
    pub cbuf: Vec<u8>,
    pub cpos: usize,
    ccap: usize,
    wc: Option<Box<Econv>>,
    wc_init: bool,
    wc_ascii: Option<&'static str>,
    wc_pre: u32,
}

/// Why a conversion step stopped short.
pub enum CErr {
    /// the converter reported an error (the bytes converted before it, for read_all's outbuf)
    Exc(Vec<u8>),
    /// a fill would block with other threads running: give these bytes back to cbuf, wait, retry
    Block(Vec<u8>),
    /// IOError with this message
    Io(&'static str),
    /// no converter for (source, destination, ecflags)
    Open(&'static str, &'static str, u32),
}
#[derive(PartialEq, Clone, Copy)]
pub enum More { Suspended, Finished }

/// NEED_READCONV (Unix: an internal encoding, or text mode).
#[inline]
pub fn need_readconv(io: &RIo) -> bool { io.enc2.is_some() || io.textmode }
/// NEED_WRITECONV
#[inline]
pub fn need_writeconv(io: &RIo) -> bool {
    matches!(io.enc, Some(e) if e != ENC_BINARY) || io.textmode || io.ecflags & (ev::DECORATOR_MASK | STATEFUL_DECORATOR_MASK) != 0
}
/// io_read_encoding
pub fn read_enc(io: &RIo) -> u8 { io.enc.unwrap_or_else(en::default_external) }
/// READ_CHAR_PENDING
#[inline]
pub fn char_pending(io: &RIo) -> bool { io.conv.as_ref().is_some_and(|v| v.cpos < v.cbuf.len()) }
fn cv(io: &mut RIo) -> &mut IoConv { io.conv.get_or_insert_with(Default::default) }

fn open_with_rep(io: &RIo, s: &str, d: &str, flags: u32) -> Option<Box<Econv>> {
    let mut ec = Econv::open(s, d, flags)?;
    if let Some(r) = &io.ecrep { if !ec.set_replacement(&r.0, r.1) { return None; } }
    Some(Box::new(ec))
}
fn is_error(r: Option<Res>) -> bool { matches!(r, Some(Res::InvalidByteSequence | Res::IncompleteInput | Res::UndefinedConversion)) }

/// make_readconv
pub fn make_readconv(io: &mut RIo, size: usize) -> Result<(), CErr> {
    if io.conv.as_ref().is_some_and(|v| v.rc.is_some()) { return Ok(()); }
    let flags = io.ecflags & !NEWLINE_DECORATOR_WRITE_MASK;
    let (s, d) = match io.enc2 { Some(e2) => (en::enc_name(e2), en::enc_name(read_enc(io))), None => ("", "") };
    let Some(rc) = open_with_rep(io, s, d, flags) else { return Err(CErr::Open(s, d, flags)) };
    let v = cv(io);
    v.rc = Some(rc);
    v.cbuf.clear(); v.cpos = 0;
    v.ccap = size.max(CBUF_MIN);
    Ok(())
}
/// clear_readconv
pub fn clear_readconv(io: &mut RIo) {
    if let Some(v) = io.conv.as_mut() { v.rc = None; v.cbuf = vec![]; v.cpos = 0; }
}
/// clear_codeconv (the writeconv is dropped unfinished, as CRuby's)
pub fn clear_codeconv(io: &mut RIo) { io.conv = None; }
/// The converter whose last error is to be raised.
pub fn readconv(io: &RIo) -> Option<&Econv> { io.conv.as_ref()?.rc.as_deref() }
pub fn writeconv(io: &RIo) -> Option<&Econv> { io.conv.as_ref()?.wc.as_deref() }

/// fill_cbuf: convert what rbuf holds (reading more when it is empty) into cbuf.
pub fn fill_cbuf(io: &mut RIo, ec_flags: u32) -> Result<More, CErr> {
    let flags = ec_flags | ev::PARTIAL_INPUT;
    let len0 = {
        let v = io.conv.as_mut().expect("readconv");
        if v.cbuf.len() - v.cpos >= v.ccap { return Ok(More::Suspended); }
        if v.cpos > 0 { v.cbuf.drain(..v.cpos); v.cpos = 0; }
        v.cbuf.len()
    };
    loop {
        let res = {
            let RIo { rbuf, rpos, conv, .. } = io;
            let v = conv.as_mut().expect("readconv");
            let rc = v.rc.as_mut().expect("readconv");
            let mut ip = 0;
            let res = rc.convert(&rbuf[*rpos..], &mut ip, &mut v.cbuf, v.ccap, flags);
            *rpos += ip;
            let pb = rc.putbackable();
            if pb > 0 {
                let b = rc.putback(pb);
                if *rpos >= b.len() { *rpos -= b.len(); rbuf[*rpos..*rpos + b.len()].copy_from_slice(&b); }
                else { let mut nb = b; nb.extend_from_slice(&rbuf[*rpos..]); *rbuf = nb; *rpos = 0; }
            }
            if is_error(rc.last_error.result) { return Err(CErr::Exc(vec![])); }
            if v.cbuf.len() != len0 { return Ok(More::Suspended); }
            res
        };
        if res == Res::Finished { return Ok(More::Finished); }
        if res == Res::SourceBufferEmpty && io.rpos >= io.rbuf.len() && !fill(io) {
            if io.wouldblock { return Err(CErr::Block(vec![])); }
            let v = io.conv.as_mut().expect("readconv");
            let rc = v.rc.as_mut().expect("readconv");
            let mut ip = 0;
            rc.convert(&[], &mut ip, &mut v.cbuf, v.ccap, 0);
            if is_error(rc.last_error.result) { return Err(CErr::Exc(vec![])); }
            break;
        }
    }
    let v = io.conv.as_ref().expect("readconv");
    Ok(if v.cbuf.len() != len0 { More::Suspended } else { More::Finished })
}
/// more_char
fn more_char(io: &mut RIo) -> Result<More, CErr> { fill_cbuf(io, ev::AFTER_OUTPUT) }

/// Put converted bytes back in front of cbuf (ungetc, and a read redone after a wait).
pub fn unget_cbuf(io: &mut RIo, b: &[u8]) {
    if b.is_empty() { return; }
    let v = cv(io);
    if v.cpos >= b.len() { v.cpos -= b.len(); let p = v.cpos; v.cbuf[p..p + b.len()].copy_from_slice(b); }
    else { let mut nb = b.to_vec(); nb.extend_from_slice(&v.cbuf[v.cpos..]); v.cbuf = nb; v.cpos = 0; }
}
/// rb_io_ungetc's readconv branch.
pub fn ungetc(io: &mut RIo, b: &[u8]) -> Result<(), CErr> {
    make_readconv(io, b.len())?;
    let v = cv(io);
    if v.ccap - (v.cbuf.len() - v.cpos) < b.len() { return Err(CErr::Io("ungetc failed")); }
    unget_cbuf(io, b);
    Ok(())
}
fn take(io: &mut RIo, n: usize) -> Vec<u8> {
    let v = cv(io);
    let r = v.cbuf[v.cpos..v.cpos + n].to_vec();
    v.cpos += n;
    if v.cpos == v.cbuf.len() { v.cbuf.clear(); v.cpos = 0; }
    r
}
fn skip(io: &mut RIo, n: usize) {
    let v = cv(io);
    v.cpos += n;
    if v.cpos == v.cbuf.len() { v.cbuf.clear(); v.cpos = 0; }
}
fn pending(io: &RIo) -> &[u8] { match &io.conv { Some(v) => &v.cbuf[v.cpos..], None => &[] } }

/// read_all without a converter.
pub fn read_all_raw(io: &mut RIo) -> Result<Vec<u8>, CErr> {
    let mut out = vec![];
    loop {
        let n = Raw::pending(io).len();
        if n > 0 { out.extend(Raw::take(io, n)); }
        match Raw::more(io) { Ok(true) => {} Ok(false) => return Ok(out), Err(_) => return Err(CErr::Block(out)) }
    }
}
/// read_all's readconv branch: everything up to EOF (the converter is dropped there).
pub fn read_all(io: &mut RIo) -> Result<Vec<u8>, CErr> {
    make_readconv(io, 0)?;
    let mut out = vec![];
    loop {
        let n = pending(io).len();
        if n > 0 { out.extend(take(io, n)); }
        match fill_cbuf(io, 0) {
            Ok(More::Suspended) => {}
            Ok(More::Finished) => { clear_readconv(io); return Ok(out); }
            Err(CErr::Exc(_)) => { let n = pending(io).len(); out.extend(take(io, n)); return Err(CErr::Exc(out)); }
            Err(CErr::Block(_)) => return Err(CErr::Block(out)),
            Err(e) => return Err(e),
        }
    }
}

enum Mb { Found(usize), More, Invalid }
/// rb_enc_precise_mbclen
fn precise(enc: u8, b: &[u8]) -> Mb {
    match en::precise_len(enc, b, 0) { Some(l) => Mb::Found(l), None => if en::needmore(enc, b) { Mb::More } else { Mb::Invalid } }
}

/// io_getc's readconv branch: the next character (broken ones are one unit long).
pub fn getc(io: &mut RIo) -> Result<Option<Vec<u8>>, CErr> {
    make_readconv(io, 0)?;
    let enc = read_enc(io);
    let r = loop {
        let p = pending(io);
        if !p.is_empty() {
            let r = precise(enc, p);
            if !matches!(r, Mb::More) { break r; }
            if p.len() >= io.conv.as_ref().map_or(0, |v| v.ccap) { return Err(CErr::Io("too long character")); }
        }
        if more_char(io)? == More::Finished {
            if pending(io).is_empty() { clear_readconv(io); return Ok(None); }
            // one unit of a character cut short at EOF
            let s = take(io, 1);
            if pending(io).is_empty() { clear_readconv(io); }
            return Ok(Some(s));
        }
    };
    let n = match r { Mb::Found(n) => n, _ => { let p = pending(io); en::mbclen(enc, p, 0) } };
    Ok(Some(take(io, n)))
}

/// Where the char-oriented readers take bytes from: the read buffer (no conversion) or cbuf.
pub trait Src {
    /// swallow compares characters (the converted path) or bytes (CRuby's raw path)
    const CHARWISE: bool;
    fn pending(io: &RIo) -> &[u8];
    fn take(io: &mut RIo, n: usize) -> Vec<u8>;
    fn skip(io: &mut RIo, n: usize);
    /// More bytes pending?  false at EOF.
    fn more(io: &mut RIo) -> Result<bool, CErr>;
    fn prepare(_io: &mut RIo) -> Result<(), CErr> { Ok(()) }
    fn at_eof(_io: &mut RIo) {}
    /// Give bytes back (a step redone after a wait).
    fn unget(io: &mut RIo, b: &[u8]);
    /// Everything up to EOF.
    fn read_all(io: &mut RIo) -> Result<Vec<u8>, CErr>;
}
pub struct Raw;
pub struct Conv;
impl Src for Raw {
    const CHARWISE: bool = false;
    fn pending(io: &RIo) -> &[u8] { &io.rbuf[io.rpos.min(io.rbuf.len())..] }
    fn take(io: &mut RIo, n: usize) -> Vec<u8> { let r = io.rbuf[io.rpos..io.rpos + n].to_vec(); io.rpos += n; r }
    fn skip(io: &mut RIo, n: usize) { io.rpos += n; }
    fn more(io: &mut RIo) -> Result<bool, CErr> { if fill(io) { Ok(true) } else if io.wouldblock { Err(CErr::Block(vec![])) } else { Ok(false) } }
    fn unget(io: &mut RIo, b: &[u8]) {
        if b.is_empty() { return; }
        let mut nb = b.to_vec(); nb.extend_from_slice(&io.rbuf[io.rpos.min(io.rbuf.len())..]);
        io.rbuf = nb; io.rpos = 0;
    }
    fn read_all(io: &mut RIo) -> Result<Vec<u8>, CErr> { read_all_raw(io) }
}
impl Src for Conv {
    const CHARWISE: bool = true;
    fn pending(io: &RIo) -> &[u8] { pending(io) }
    fn take(io: &mut RIo, n: usize) -> Vec<u8> { take(io, n) }
    fn skip(io: &mut RIo, n: usize) { skip(io, n) }
    fn more(io: &mut RIo) -> Result<bool, CErr> { Ok(more_char(io)? == More::Suspended) }
    fn prepare(io: &mut RIo) -> Result<(), CErr> { make_readconv(io, 0) }
    fn at_eof(io: &mut RIo) { clear_readconv(io); }
    fn unget(io: &mut RIo, b: &[u8]) { unget_cbuf(io, b); }
    fn read_all(io: &mut RIo) -> Result<Vec<u8>, CErr> { read_all(io) }
}

/// io_getc without a converter: one character of the read buffer; a character cut by the buffer
/// end is completed a byte at a time from the next reads.  `carry`: the length of such a prefix
/// handed back by a wait, so the retry goes on from it.
pub fn getc_raw(io: &mut RIo, carry: &mut usize) -> Result<Option<Vec<u8>>, CErr> {
    let enc = read_enc(io);
    let mut s = if *carry > 0 {
        let n = (*carry).min(Raw::pending(io).len());
        *carry = 0;
        Raw::take(io, n)
    } else {
        if !Raw::more(io)? { return Ok(None); }
        let p = Raw::pending(io);
        if en::ascii_compat(enc) && p[0] < 0x80 { return Ok(Some(Raw::take(io, 1))); }
        match precise(enc, p) {
            Mb::Found(n) => return Ok(Some(Raw::take(io, n))),
            Mb::Invalid => return Ok(Some(Raw::take(io, 1))),
            Mb::More => { let n = p.len(); Raw::take(io, n) }
        }
    };
    loop {
        match Raw::more(io) { Ok(true) => {} Ok(false) => break, Err(_) => { *carry = s.len(); return Err(CErr::Block(s)); } }
        s.extend(Raw::take(io, 1));
        if !matches!(precise(enc, &s), Mb::More) { break; }
    }
    Ok(Some(s))
}

/// The ASCII character at the start of `p` in `enc` and its length (rb_enc_ascget).
fn ascget(enc: u8, p: &[u8]) -> Option<(u8, usize)> {
    if en::min_len(enc) == 1 { return if p[0] < 0x80 { Some((p[0], 1)) } else { None }; }
    match precise(enc, p) { Mb::Found(n) => { let cp = en::char_code(enc, &p[..n]); if cp < 0x80 { Some((cp as u8, n)) } else { None } } _ => None }
}
/// swallow: skip characters equal to `term`; false at EOF.
fn swallow<S: Src>(io: &mut RIo, term: u8, enc: u8) -> Result<bool, CErr> {
    S::prepare(io)?;
    let needconv = S::CHARWISE && en::min_len(enc) != 1;
    loop {
        loop {
            let p = S::pending(io);
            if p.is_empty() { break; }
            let skip = if !needconv {
                if p[0] != term { return Ok(true); }
                p.iter().take_while(|&&b| b == term).count()
            } else {
                let mut i = 0;
                while i < p.len() { match ascget(enc, &p[i..]) { Some((ch, n)) if ch == term => i += n, _ => break } }
                if i == 0 { return Ok(true); }
                i
            };
            S::skip(io, skip);
        }
        if !S::more(io)? { return Ok(false); }
    }
}
/// search_delim: the end of the first `delim` character in `p`.
fn search_delim(p: &[u8], delim: u32, enc: u8) -> Option<usize> {
    if en::min_len(enc) == 1 { return p.iter().position(|&b| b as u32 == delim).map(|i| i + 1); }
    let mut i = 0;
    while i < p.len() {
        match precise(enc, &p[i..]) {
            Mb::Found(n) => { if en::char_code(enc, &p[i..i + n]) == delim { return Some(i + n); } i += n; }
            _ => i += en::min_len(enc),
        }
    }
    None
}
/// appendline: up to and including `delim`, or `limit` bytes (limit < 0: none); None at EOF.
fn appendline<S: Src>(io: &mut RIo, delim: Option<u32>, line: &mut Vec<u8>, limit: &mut i64, enc: u8) -> Result<Option<u32>, CErr> {
    S::prepare(io)?;
    loop {
        let p = S::pending(io);
        if !p.is_empty() {
            let mut n = p.len();
            if 0 < *limit && (*limit as usize) < n { n = *limit as usize; }
            if let Some(e) = delim.and_then(|d| search_delim(&p[..n], d, enc)) {
                line.extend_from_slice(&p[..e]);
                S::skip(io, e);
                *limit -= e as i64;
                return Ok(delim);
            }
            line.extend_from_slice(&p[..n]);
            S::skip(io, n);
            *limit -= n as i64;
            if *limit == 0 { return Ok(Some(*line.last().unwrap() as u32)); }
        }
        if !S::more(io)? { break; }
    }
    S::at_eof(io);
    Ok(None)
}
/// Is `p` the start of a character of `s` (rb_enc_left_char_head(s, p) == p)?
fn at_char_boundary(s: &[u8], p: usize, enc: u8) -> bool { p >= s.len() || left_adjust(s, p, enc) == p }
/// The start of the last character of `s` (rb_enc_prev_char from the end).
fn prev_char(s: &[u8], enc: u8) -> Option<usize> { if s.is_empty() { None } else { Some(left_adjust(s, s.len() - 1, enc)) } }
/// Onigmo's mbc_enc_len as the left_adjust functions use it: the raw precise result
/// (n found, -1-n needs n more, -1 invalid).
fn raw_mbc_len(enc: u8, p: &[u8]) -> isize {
    use en::EK::*;
    match en::precise_len(enc, p, 0) {
        Some(n) => n as isize,
        None if en::needmore(enc, p) => {
            let full = match en::kind(enc) { EucJp(_) if p[0] == 0x8f => 3, _ => 2 };
            -1 - (full as isize - p.len() as isize).max(0)
        }
        None => -1,
    }
}
/// ONIGENC_LEFT_ADJUST_CHAR_HEAD(s, pos): the start of the character holding byte `pos`, with the
/// encodings' own rules (EUC-JP and Shift_JIS step by the raw length, which goes back on a cut character).
fn left_adjust(s: &[u8], pos: usize, enc: u8) -> usize {
    use en::EK::*;
    if pos == 0 { return 0; }
    let adjust = |p: usize| -> usize {
        let len = raw_mbc_len(enc, &s[p..]);
        let (p, pos) = (p as isize, pos as isize);
        if p + len > pos { return p as usize; }
        let q = p + len;
        (q + ((pos - q) & !1)).max(0) as usize
    };
    match en::kind(enc) {
        Utf8 => { let mut p = pos; while s[p] & 0xc0 == 0x80 && p > 0 { p -= 1; } p }
        U16LE => { let mut p = pos - pos % 2; if s.get(p + 1).is_some_and(|&b| b & 0xfc == 0xdc) && p > 1 { p -= 2; } p }
        U16BE => { let mut p = pos - pos % 2; if s[p] & 0xfc == 0xdc && p > 1 { p -= 2; } p }
        U32LE | U32BE => pos - pos % 4,
        Bin | Ascii | Single | Dummy => pos,
        EucJp(_) => { let mut p = pos; while s[p].wrapping_sub(0xa1) <= 0x5d && p > 0 { p -= 1; } adjust(p) }
        Sjis(_) => {
            let first = |b: u8| (0x81..=0x9f).contains(&b) || (0xe0..=0xfc).contains(&b);
            let trail = |b: u8| (0x40..=0x7e).contains(&b) || (0x80..=0xfc).contains(&b);
            let mut p = pos;
            if trail(s[p]) { while p > 0 { p -= 1; if !first(s[p]) { p += 1; break; } } }
            adjust(p)
        }
        _ => {
            let mut i = 0; let mut last = 0;
            while i <= pos { last = i; i += en::mbclen(enc, s, i); }
            last
        }
    }
}

/// A gets separator prepared for the read encoding.
pub enum Sep { None, Para, Bytes(Vec<u8>) }
/// rb_io_getline_0's general branch: (line, whether the limit cut it).  `rs` is already in the
/// read encoding.  A step that would block hands back the partial line in CErr::Block.
pub fn getline<S: Src>(io: &mut RIo, rs: &Sep, limit: Option<usize>, chomp: bool) -> Result<Option<(Vec<u8>, bool)>, CErr> {
    let enc = read_enc(io);
    let mut limit: i64 = limit.map_or(-1, |l| l as i64);
    let (rsb, para): (Vec<u8>, bool) = match rs {
        Sep::None => (vec![], false),
        Sep::Para => {
            let nl = if en::ascii_compat(enc) { vec![10] } else { en::code_to_bytes(enc, 10).unwrap_or(vec![10]) };
            ([nl.clone(), nl].concat(), true)
        }
        Sep::Bytes(b) => (b.clone(), false),
    };
    let mut newline: Option<u32> = None;
    let mut chomp_cr = chomp;
    if !matches!(rs, Sep::None) {
        if para { swallow::<S>(io, b'\n', enc)?; newline = Some(10); }
        else if en::min_len(enc) == 1 { newline = Some(*rsb.last().unwrap() as u32); }
        else { let last = prev_char(&rsb, enc).unwrap_or(0); newline = Some(en::char_code(enc, &rsb[last..])); }
        chomp_cr = chomp && newline == Some(10) && rsb.len() == en::min_len(enc);
    }
    let mut line = vec![];
    let mut nolimit = false;
    let mut extra = 16;
    let mut c;
    loop {
        c = match appendline::<S>(io, newline, &mut line, &mut limit, enc) { Ok(c) => c, Err(CErr::Block(_)) => return Err(CErr::Block(line)), Err(e) => return Err(e) };
        let Some(ch) = c else { break };
        // 4.0.10: a rejected candidate still goes through the limit check
        if Some(ch) == newline && line.len() >= rsb.len() {
            let p = line.len() - rsb.len();
            if at_char_boundary(&line, p, enc) && line[p..] == rsb[..] {
                if chomp {
                    let mut q = p;
                    if chomp_cr && q > 0 && line[q - 1] == b'\r' { q -= 1; }
                    line.truncate(q);
                }
                break;
            }
        }
        if limit == 0 {
            if let Some(pp) = prev_char(&line, enc) {
                // relax the limit while the last character is incomplete
                if extra > 0 && matches!(precise(enc, &line[pp..]), Mb::More) { limit = 1; extra -= 1; continue; }
            }
            nolimit = true;
            break;
        }
    }
    if para && c.is_some() {
        match swallow::<S>(io, b'\n', enc) { Ok(_) | Err(CErr::Block(_)) => {} Err(e) => return Err(e) }
    }
    Ok(if line.is_empty() && c.is_none() { None } else { Some((line, nolimit)) })
}

// ---- writing ----------------------------------------------------------------------------------

/// make_writeconv
pub fn make_writeconv(io: &mut RIo) -> Result<(), CErr> {
    if io.conv.as_ref().is_some_and(|v| v.wc_init) { return Ok(()); }
    let ecflags = io.ecflags & !NEWLINE_DECORATOR_READ_MASK;
    let (pre, wc, ascii) = if io.enc.is_none() || (io.enc == Some(ENC_BINARY) && io.enc2.is_none()) {
        let Some(w) = open_with_rep(io, "", "", ecflags) else { return Err(CErr::Open("", "", ecflags)) };
        (0, Some(w), None)
    } else {
        let enc = io.enc2.or(io.enc).unwrap();
        let senc = ev::asciicompat_encoding(en::enc_name(enc));
        if senc.is_none() && io.ecflags & STATEFUL_DECORATOR_MASK == 0 { (ecflags, None, None) }
        else {
            let (s, d, a) = match senc { Some(s) => (s, en::enc_name(enc), s), None => ("", "", en::enc_name(enc)) };
            let f2 = io.ecflags & (ev::ERROR_HANDLER_MASK | STATEFUL_DECORATOR_MASK);
            let Some(w) = open_with_rep(io, s, d, f2) else { return Err(CErr::Open(s, d, f2)) };
            (ecflags & !STATEFUL_DECORATOR_MASK, Some(w), Some(a))
        }
    };
    let v = cv(io);
    v.wc_init = true; v.wc_pre = pre; v.wc = wc; v.wc_ascii = ascii;
    Ok(())
}
/// What do_writeconv does to a String of encoding `str_enc`: (rb_str_encode to this encoding name
/// with these ecflags, then the writeconv); Err: the text-mode ArgumentError.
pub fn write_plan(io: &RIo, str_enc: u8) -> Result<(Option<&'static str>, u32, bool), ()> {
    let v = io.conv.as_ref().expect("writeconv");
    if v.wc.is_some() {
        if let Some(a) = v.wc_ascii { return Ok((Some(a), v.wc_pre, true)); }
        if io.textmode && !en::ascii_compat(str_enc) { return Err(()); }
        return Ok((None, v.wc_pre, true));
    }
    let common = match io.enc2 { Some(e) => Some(e), None => io.enc.filter(|&e| e != ENC_BINARY) };
    Ok((common.map(en::enc_name), v.wc_pre, false))
}
/// rb_econv_str_convert(writeconv, s, ECONV_PARTIAL_INPUT); Err: the converter's error (rb_econv_check_error).
pub fn writeconv_convert(io: &mut RIo, s: &[u8]) -> Result<Vec<u8>, CErr> {
    let wc = io.conv.as_mut().and_then(|v| v.wc.as_mut()).expect("writeconv");
    let max = wc.last_tc.map(|i| wc.elems[i].tc.tr.max_out as usize).unwrap_or(1);
    let mut out = Vec::with_capacity(s.len() + max);
    let mut ip = 0;
    loop {
        let cap = out.len() + (s.len() - ip) + max;
        let r = wc.convert(s, &mut ip, &mut out, cap, ev::PARTIAL_INPUT);
        if is_error(wc.last_error.result) { return Err(CErr::Exc(vec![])); }
        if r != Res::DestinationBufferFull { return Ok(out); }
    }
}
/// finish_writeconv: the converter's closing bytes (ISO-2022-JP's ESC ( B); Err: its error.
pub fn finish_writeconv(io: &mut RIo) -> Result<Vec<u8>, ()> {
    let Some(wc) = io.conv.as_mut().and_then(|v| v.wc.as_mut()) else { return Ok(vec![]) };
    let mut out = vec![];
    loop {
        let mut ip = 0;
        let cap = out.len() + 1024;
        let r = wc.convert(&[], &mut ip, &mut out, cap, 0);
        if matches!(r, Res::InvalidByteSequence | Res::IncompleteInput | Res::UndefinedConversion) { return Err(()); }
        if r != Res::DestinationBufferFull { return Ok(out); }
    }
}
/// Drop the writeconv after finishing it (close).
pub fn close_writeconv(io: &mut RIo) { if let Some(v) = io.conv.as_mut() { v.wc = None; v.wc_init = false; } }
/// rb_io_binmode on the converters: drop their newline decorators.
pub fn binmode_convs(io: &mut RIo) {
    if let Some(v) = io.conv.as_mut() {
        if let Some(r) = v.rc.as_mut() { r.binmode(); }
        if let Some(w) = v.wc.as_mut() { w.binmode(); }
        v.wc_pre &= !ev::NEWLINE_DECORATOR_MASK;
    }
}

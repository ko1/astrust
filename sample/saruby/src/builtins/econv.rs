// The transcoding engine: a port of CRuby's transcode.c (rb_transcoding / rb_econv_t) running the
// transcoder tables of trans_tables.rs, and the functions those tables call (enc/trans/*.trans).
// No Ruby objects here; transcode.rs builds String#encode and Encoding::Converter on top.
#![forbid(unsafe_code)]
use crate::builtins::trans_tables as tt;
use tt::{TrDef, TrFile, TrFn};
use std::sync::OnceLock;

pub const INVALID_MASK: u32 = 0x0f;
pub const INVALID_REPLACE: u32 = 0x02;
pub const UNDEF_MASK: u32 = 0xf0;
pub const UNDEF_REPLACE: u32 = 0x20;
pub const UNDEF_HEX_CHARREF: u32 = 0x30;
pub const ERROR_HANDLER_MASK: u32 = 0xff;
pub const DECORATOR_MASK: u32 = 0x1ff00;
pub const NEWLINE_DECORATOR_MASK: u32 = 0x7f00;
pub const UNIVERSAL_NEWLINE_DECORATOR: u32 = 0x100;
pub const CRLF_NEWLINE_DECORATOR: u32 = 0x1000;
pub const CR_NEWLINE_DECORATOR: u32 = 0x2000;
pub const LF_NEWLINE_DECORATOR: u32 = 0x4000;
pub const XML_TEXT_DECORATOR: u32 = 0x8000;
pub const XML_ATTR_CONTENT_DECORATOR: u32 = 0x10000;
pub const XML_ATTR_QUOTE_DECORATOR: u32 = 0x100000;
pub const PARTIAL_INPUT: u32 = 0x20000;
pub const AFTER_OUTPUT: u32 = 0x40000;

/// rb_econv_result_t
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Res { InvalidByteSequence, UndefinedConversion, DestinationBufferFull, SourceBufferEmpty, Finished, AfterOutput, IncompleteInput }

// instruction kinds of an info word (transcode_data.h)
const NOMAP: u32 = 0x01;
const ONEBT: u32 = 0x02;
const TWOBT: u32 = 0x03;
const THREEBT: u32 = 0x05;
const FOURBT: u32 = 0x06;
const INVALID: u32 = 0x07;
const UNDEF: u32 = 0x09;
const ZEROBT: u32 = 0x0A;
const FUNII: u32 = 0x0B;
const FUNSI: u32 = 0x0D;
const FUNIO: u32 = 0x0E;
const FUNSO: u32 = 0x0F;
const STR1: u32 = 0x11;
const GB4BT: u32 = 0x12;
const FUNSIO: u32 = 0x13;

pub fn enc_eq(a: &str, b: &str) -> bool { a.eq_ignore_ascii_case(b) }
pub fn is_decorator(tr: &TrDef) -> bool { tr.src.is_empty() }

#[inline] fn word(f: &TrFile, idx: usize) -> u32 { let w = &f.words[idx * 4..idx * 4 + 4]; u32::from_le_bytes([w[0], w[1], w[2], w[3]]) }
/// The action for `byte` in table `table` (BL_ACTION, or INVALID outside BL_MIN..BL_MAX).
fn lookup(f: &TrFile, table: u32, byte: u8) -> u32 {
    let t = (table >> 2) as usize;
    let base = word(f, t) as usize;
    let info = (word(f, t + 1) >> 2) as usize;
    let (lo, hi) = (f.bytes[base], f.bytes[base + 1]);
    if byte < lo || hi < byte { return INVALID; }
    word(f, info + f.bytes[base + 2 + (byte - lo) as usize] as usize)
}

// ---- the registry: transdb.h declarations, searched like transcode_search_path ----------------

struct Registry { srcs: Vec<(&'static str, Vec<(&'static str, usize)>)> }
fn registry() -> &'static Registry {
    static R: OnceLock<Registry> = OnceLock::new();
    R.get_or_init(|| {
        let mut srcs: Vec<(&'static str, Vec<(&'static str, usize)>)> = vec![];
        let mut add = |s: &'static str, d: &'static str| {
            let Some(ti) = tt::TRANSCODERS.iter().position(|t| enc_eq(t.src, s) && enc_eq(t.dst, d)) else { return };
            let k = match srcs.iter().position(|(n, _)| enc_eq(n, s)) { Some(k) => k, None => { srcs.push((s, vec![])); srcs.len() - 1 } };
            if !srcs[k].1.iter().any(|(n, _)| enc_eq(n, d)) { srcs[k].1.push((d, ti)); }
        };
        // Init_newline registers before transdb.h declares the rest
        for d in ["universal_newline", "crlf_newline", "cr_newline", "lf_newline"] { add("", d); }
        for &(s, d, _) in tt::DECLS.iter() { add(s, d); }
        Registry { srcs }
    })
}
fn dests(s: &str) -> Option<&'static [(&'static str, usize)]> { registry().srcs.iter().find(|(n, _)| enc_eq(n, s)).map(|(_, v)| &v[..]) }
/// The transcoder for exactly this pair (get_transcoder_entry + load).
pub fn transcoder(s: &str, d: &str) -> Option<&'static TrDef> { dests(s)?.iter().find(|(n, _)| enc_eq(n, d)).map(|&(_, i)| &tt::TRANSCODERS[i]) }

/// Breadth-first search for a path of transcoders (transcode_search_path); None if there is none.
pub fn search_path(sname: &str, dname: &str) -> Option<Vec<&'static TrDef>> {
    type Cache = std::collections::HashMap<(String, String), Option<Vec<&'static TrDef>>>;
    static CACHE: OnceLock<std::sync::Mutex<Cache>> = OnceLock::new();
    let key = (sname.to_ascii_lowercase(), dname.to_ascii_lowercase());
    let m = CACHE.get_or_init(Default::default);
    if let Some(p) = m.lock().ok()?.get(&key) { return p.clone(); }
    let p = search_path0(sname, dname);
    m.lock().ok()?.insert(key, p.clone());
    p
}
fn search_path0(sname: &str, dname: &str) -> Option<Vec<&'static TrDef>> {
    if enc_eq(sname, dname) { return None; }
    let mut visited: Vec<(String, Option<String>)> = vec![(sname.to_string(), None)];
    let mut queue: std::collections::VecDeque<String> = [sname.to_string()].into();
    let seen = |v: &Vec<(String, Option<String>)>, n: &str| v.iter().position(|(k, _)| enc_eq(k, n));
    let mut found = false;
    while let Some(q) = queue.pop_front() {
        let Some(ds) = dests(&q) else { continue };
        if ds.iter().any(|(n, _)| enc_eq(n, dname)) { visited.push((dname.to_string(), Some(q))); found = true; break; }
        for (n, _) in ds { if seen(&visited, n).is_none() { visited.push((n.to_string(), Some(q.clone()))); queue.push_back(n.to_string()); } }
    }
    if !found { return None; }
    let mut path = vec![];
    let mut enc = dname.to_string();
    loop {
        let i = seen(&visited, &enc).unwrap();
        let Some(prev) = visited[i].1.clone() else { break };
        path.push(transcoder(&prev, &enc)?);
        enc = prev;
    }
    path.reverse();
    Some(path)
}

/// rb_econv_asciicompat_encoding: the ASCII-compatible encoding an ASCII-incompatible one decodes to.
pub fn asciicompat_encoding(name: &str) -> Option<&'static str> {
    let ds = dests(name)?;
    if ds.len() != 1 { return None; }
    let tr = &tt::TRANSCODERS[ds[0].1];
    if !is_decorator(tr) && tr.asciicompat == 1 { Some(tr.dst) } else { None }
}

// ---- one transcoder's run (rb_transcoding) -----------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Pc { Top, CheckIn, FollowByte, FollowInfo, NeedByte, InvB, InvSusp, Drain, DrainFin, Cleanup, Finished }

#[derive(Clone, Default)]
pub struct MacBuf { buf: [u8; 16], beg: usize, end: usize }

#[derive(Clone)]
pub struct Tc {
    pub tr: &'static TrDef,
    resume: Pc,
    next_table: u32,
    next_info: u32,
    next_byte: u8,
    pub recognized_len: usize,
    pub readagain_len: usize,
    pub readbuf: Vec<u8>,
    writebuf: Vec<u8>,
    writebuf_off: usize,
    chbuf: Vec<u8>,
    obuf: Vec<u8>,
    st: [u8; 3],
    mac: MacBuf,
}

impl Tc {
    pub fn new(tr: &'static TrDef) -> Tc {
        let mut t = Tc { tr, resume: Pc::Top, next_table: 0, next_info: 0, next_byte: 0, recognized_len: 0, readagain_len: 0,
                         readbuf: vec![], writebuf: vec![], writebuf_off: 0, chbuf: vec![], obuf: vec![], st: [0; 3], mac: MacBuf::default() };
        call_init(tr.state_init_func, &mut t);
        t
    }
    fn file(&self) -> &'static TrFile { &tt::FILES[self.tr.file as usize] }
    pub fn readagain(&self) -> &[u8] { self.readbuf.get(self.recognized_len..self.recognized_len + self.readagain_len).unwrap_or(&[]) }

    /// transcode_restartable0: a coroutine resumed at `self.resume`.
    fn run0(&mut self, inp: &[u8], ip: &mut usize, out: &mut Vec<u8>, ostop: usize, opt: u32) -> Res {
        let unitlen = self.tr.unit_len as usize;
        let mut readagain_len = 0usize;
        let mut in_p = *ip;
        let mut inchar_start = in_p;
        let out0 = out.len();
        let partial = opt & PARTIAL_INPUT != 0;
        let after = opt & AFTER_OUTPUT != 0;
        let f = self.file();
        macro_rules! suspend { ($ret:expr, $pc:expr) => {{
            self.resume = $pc;
            if in_p > inchar_start { self.readbuf.truncate(self.recognized_len); self.readbuf.extend_from_slice(&inp[inchar_start..in_p]); }
            *ip = in_p;
            self.recognized_len += in_p - inchar_start;
            if readagain_len != 0 { self.recognized_len -= readagain_len; self.readagain_len = readagain_len; }
            return $ret;
        }}}
        let mut pc = self.resume;
        loop {
            match pc {
                Pc::Top => {
                    inchar_start = in_p;
                    self.recognized_len = 0;
                    self.next_table = self.tr.tree_start;
                    if after && out.len() != out0 { suspend!(Res::AfterOutput, Pc::CheckIn); }
                    pc = Pc::CheckIn;
                }
                Pc::CheckIn => {
                    if in_p >= inp.len() {
                        if !partial { pc = Pc::Cleanup; continue; }
                        suspend!(Res::SourceBufferEmpty, Pc::Top);
                    }
                    self.next_byte = inp[in_p]; in_p += 1;
                    pc = Pc::FollowByte;
                }
                Pc::FollowByte => { self.next_info = lookup(f, self.next_table, self.next_byte); pc = Pc::FollowInfo; }
                Pc::FollowInfo => {
                    let ni = self.next_info;
                    // the character's bytes: those of earlier calls (readbuf) and this call's
                    let ch = |t: &mut Tc| { let mut v = std::mem::take(&mut t.chbuf); v.clear(); v.extend_from_slice(&t.readbuf[..t.recognized_len]); v.extend_from_slice(&inp[inchar_start..in_p]); v };
                    match ni & 0x1f {
                        NOMAP | ONEBT | TWOBT | THREEBT | FOURBT | GB4BT | STR1 => {
                            let b = |s: u32| (ni >> s) as u8;
                            let tmp: [u8; 4];
                            let bytes: &[u8] = match ni & 0x1f {
                                NOMAP => &inp[inchar_start..in_p],
                                ONEBT => { tmp = [b(8), 0, 0, 0]; &tmp[..1] }
                                TWOBT => { tmp = [b(8), b(16), 0, 0]; &tmp[..2] }
                                THREEBT => { tmp = [b(8), b(16), b(24), 0]; &tmp[..3] }
                                FOURBT => { tmp = [((ni >> 5) as u8 & 0x07) | 0xF0, b(8), b(16), b(24)]; &tmp }
                                GB4BT => { tmp = [b(8), ((ni >> 24) as u8 & 0x0F) | 0x30, b(16), ((ni >> 28) as u8 & 0x0F) | 0x30]; &tmp }
                                _ => { let bi = (ni >> 6) as usize; let len = f.bytes[bi] as usize + 4; &f.bytes[bi + 1..bi + 1 + len] }
                            };
                            // byte by byte with a check for room each, as CRuby; all at once when they fit
                            if out.len() + bytes.len() <= ostop { out.extend_from_slice(bytes); pc = Pc::Top; }
                            else { self.writebuf.clear(); self.writebuf.extend_from_slice(bytes); self.writebuf_off = 0; pc = Pc::Drain; }
                        }
                        0x00 | 0x04 | 0x08 | 0x0C | 0x10 | 0x14 | 0x18 | 0x1C => {
                            if after && out.len() != out0 { suspend!(Res::AfterOutput, Pc::NeedByte); }
                            pc = Pc::NeedByte;
                        }
                        ZEROBT => pc = Pc::Top,
                        FUNII | FUNIO => { self.next_info = INVALID; }
                        FUNSI => { let s = ch(self); self.next_info = call_si(self.tr.func_si, self, &s); self.chbuf = s; }
                        FUNSO | FUNSIO => {
                            if out.len() >= ostop { suspend!(Res::DestinationBufferFull, Pc::FollowInfo); }
                            let s = ch(self);
                            let mut o = std::mem::take(&mut self.obuf);
                            o.clear();
                            if ni & 0x1f == FUNSO { call_so(self.tr.func_so, self, &s, &mut o) } else { call_sio(self.tr.func_sio, self, &s, ni, &mut o) };
                            self.chbuf = s;
                            // with max_output bytes of room CRuby writes in place, even past the end
                            if self.tr.max_out as usize <= ostop - out.len() { out.extend_from_slice(&o); pc = Pc::Top; }
                            else {
                                // CRuby 4.0's CP50220 output can pass its writebuf (max(8, max_output) bytes) into the state
                                let wb = (self.tr.max_out as usize).max(8);
                                for (k, &b) in o.iter().skip(wb).take(3).enumerate() { self.st[k] = b; }
                                std::mem::swap(&mut self.writebuf, &mut o);
                                self.writebuf_off = 0; pc = Pc::Drain;
                            }
                            self.obuf = o;
                        }
                        INVALID => {
                            let rl = self.recognized_len + (in_p - inchar_start);
                            if rl <= unitlen {
                                if rl < unitlen && after && out.len() != out0 { suspend!(Res::AfterOutput, Pc::InvB); }
                                pc = Pc::InvB;
                            } else {
                                let discard = ((rl - 1) / unitlen) * unitlen;
                                readagain_len = rl - discard;
                                pc = Pc::InvSusp;
                            }
                        }
                        UNDEF => suspend!(Res::UndefinedConversion, Pc::Top),
                        _ => pc = Pc::InvSusp,
                    }
                }
                Pc::NeedByte => {
                    while in_p >= inp.len() {
                        if !partial { suspend!(Res::IncompleteInput, Pc::Top); }
                        suspend!(Res::SourceBufferEmpty, Pc::NeedByte);
                    }
                    self.next_byte = inp[in_p]; in_p += 1;
                    self.next_table = self.next_info;
                    pc = Pc::FollowByte;
                }
                Pc::InvB => {
                    while partial && self.recognized_len + (inp.len() - inchar_start) < unitlen {
                        in_p = inp.len();
                        suspend!(Res::SourceBufferEmpty, Pc::InvB);
                    }
                    if self.recognized_len + (inp.len() - inchar_start) <= unitlen { in_p = inp.len(); }
                    else { in_p = inchar_start + (unitlen - self.recognized_len); }
                    pc = Pc::InvSusp;
                }
                Pc::InvSusp => suspend!(Res::InvalidByteSequence, Pc::Top),
                Pc::Drain | Pc::DrainFin => {
                    while self.writebuf_off < self.writebuf.len() {
                        if out.len() >= ostop { suspend!(Res::DestinationBufferFull, pc); }
                        out.push(self.writebuf[self.writebuf_off]);
                        self.writebuf_off += 1;
                    }
                    pc = if pc == Pc::Drain { Pc::Top } else { Pc::Finished };
                }
                Pc::Cleanup => {
                    if self.tr.finish_func != TrFn::None {
                        if out.len() >= ostop { suspend!(Res::DestinationBufferFull, Pc::Cleanup); }
                        let o = call_finish(self.tr.finish_func, self);
                        if self.tr.max_out as usize <= ostop - out.len() { out.extend_from_slice(&o); pc = Pc::Finished; }
                        else { self.writebuf = o; self.writebuf_off = 0; pc = Pc::DrainFin; }
                    } else { pc = Pc::Finished; }
                }
                Pc::Finished => suspend!(Res::Finished, Pc::Finished),
            }
        }
    }

    /// transcode_restartable: bytes read again after an error go first.
    pub fn convert(&mut self, inp: &[u8], ip: &mut usize, out: &mut Vec<u8>, ostop: usize, opt: u32) -> Res {
        if self.readagain_len > 0 {
            let ra = self.readagain().to_vec();
            self.readagain_len = 0;
            let mut rp = 0;
            let res = self.run0(&ra, &mut rp, out, ostop, opt | PARTIAL_INPUT);
            if res != Res::SourceBufferEmpty {
                self.readbuf.truncate(self.recognized_len + self.readagain_len);
                self.readbuf.extend_from_slice(&ra[rp..]);
                self.readagain_len += ra.len() - rp;
                return res;
            }
        }
        self.run0(inp, ip, out, ostop, opt)
    }
}

// ---- the converter (rb_econv_t) -----------------------------------------------------------------

/// An intermediate buffer: data[start..] is pending, at most `cap` bytes after compaction.
#[derive(Clone, Default)]
pub struct Buf { pub data: Vec<u8>, pub start: usize, pub cap: usize }
impl Buf {
    fn compact(&mut self) { if self.start > 0 { self.data.drain(..self.start); self.start = 0; } }
    fn pending(&self) -> bool { self.start < self.data.len() }
}

#[derive(Clone)]
pub struct Elem { pub tc: Tc, pub buf: Buf, pub last_result: Res }

#[derive(Clone, Default)]
pub struct LastError {
    pub result: Option<Res>,
    pub error_tc: Option<usize>,
    pub src: &'static str,
    pub dst: &'static str,
    pub error_bytes: Vec<u8>,
    pub readagain: Vec<u8>,
}

#[derive(Clone)]
pub struct Econv {
    pub flags: u32,
    pub started: bool,
    pub sname: String,
    pub dname: String,
    pub replacement: Option<(Vec<u8>, String)>,
    pub in_buf: Option<Buf>,
    pub elems: Vec<Elem>,
    num_finished: usize,
    pub last_tc: Option<usize>,
    pub last_error: LastError,
}

const ELEM_BUFSIZE: usize = 4096;

impl Econv {
    pub fn empty() -> Econv {
        Econv { flags: 0, started: false, sname: String::new(), dname: String::new(), replacement: None, in_buf: None, elems: vec![],
                num_finished: 0, last_tc: None, last_error: LastError { result: Some(Res::SourceBufferEmpty), ..Default::default() } }
    }
    /// rb_econv_add_transcoder_at
    pub fn add_at(&mut self, tr: &'static TrDef, i: usize) {
        self.elems.insert(i, Elem { tc: Tc::new(tr), buf: Buf { data: vec![], start: 0, cap: ELEM_BUFSIZE }, last_result: Res::SourceBufferEmpty });
        if !is_decorator(tr) {
            if let Some(j) = (i..self.elems.len()).rev().find(|&j| !is_decorator(self.elems[j].tc.tr)) { self.last_tc = Some(j); }
        } else if let Some(l) = self.last_tc { if l >= i { self.last_tc = Some(l + 1); } }
    }
    /// rb_econv_add_converter (by name; fails once conversion has started)
    pub fn add_converter(&mut self, s: &str, d: &str, i: usize) -> bool {
        if self.started { return false; }
        match transcoder(s, d) { Some(tr) => { self.add_at(tr, i); true } None => false }
    }
    fn decorate_at_last(&mut self, name: &str) -> bool {
        let n = self.elems.len();
        if n == 0 { return self.add_converter("", name, 0); }
        let tr = self.elems[n - 1].tc.tr;
        if !is_decorator(tr) && tr.asciicompat == 2 { return self.add_converter("", name, n - 1); }
        self.add_converter("", name, n)
    }
    pub fn decorate_at_first(&mut self, name: &str) -> bool {
        if self.elems.is_empty() { return self.add_converter("", name, 0); }
        let tr = self.elems[0].tc.tr;
        if !is_decorator(tr) && tr.asciicompat == 1 { return self.add_converter("", name, 1); }
        self.add_converter("", name, 0)
    }

    /// rb_econv_open
    pub fn open(sname: &str, dname: &str, ecflags: u32) -> Option<Econv> {
        let decos = decorator_names(ecflags)?;
        let mut ec = Econv::open0(sname, dname, ecflags & ERROR_HANDLER_MASK)?;
        for d in decos { if !ec.decorate_at_last(d) { return None; } }
        ec.flags |= ecflags & !ERROR_HANDLER_MASK;
        Some(ec)
    }
    fn open0(sname: &str, dname: &str, ecflags: u32) -> Option<Econv> {
        let mut ec = Econv::empty();
        let (mut sname, mut dname) = (sname, dname);
        if sname.is_empty() && dname.is_empty() {
        } else {
            let path = search_path(sname, dname)?;
            // 4.0.10: the names are the transcoders' (WINDOWS-1252, not Windows-1252)
            if let (Some(f), Some(l)) = (path.first(), path.last()) { sname = f.src; dname = l.dst; }
            for tr in path { let n = ec.elems.len(); ec.add_at(tr, n); }
        }
        ec.flags = ecflags;
        ec.sname = sname.to_string();
        ec.dname = dname.to_string();
        Some(ec)
    }

    /// trans_sweep: run each transcoder until no progress; Some(i) where one needs reporting.
    fn trans_sweep(&mut self, input: &[u8], ip: &mut usize, out: &mut Vec<u8>, ostop: usize, mut flags: u32, mut start: usize) -> Option<usize> {
        let n = self.elems.len();
        let mut tried = true;
        while tried {
            tried = false;
            let mut i = start;
            while i < n {
                let mut f = flags;
                if self.num_finished != i { f |= PARTIAL_INPUT; }
                if i == 0 && flags & AFTER_OUTPUT != 0 { start = 1; flags &= !AFTER_OUTPUT; }
                if i != 0 { f &= !AFTER_OUTPUT; }
                let (before, rest) = self.elems.split_at_mut(i);
                let te = &mut rest[0];
                let last = i == n - 1;
                if !last { te.buf.compact(); }
                let res;
                let (moved_in, moved_out);
                if i == 0 {
                    let (iold, oold) = (*ip, if last { out.len() } else { te.buf.data.len() });
                    res = if last { te.tc.convert(input, ip, out, ostop, f) } else { let cap = te.buf.cap; te.tc.convert(input, ip, &mut te.buf.data, cap, f) };
                    moved_in = iold != *ip; moved_out = oold != if last { out.len() } else { te.buf.data.len() };
                } else {
                    let prev = &mut before[i - 1].buf;
                    let (iold, oold) = (prev.start, if last { out.len() } else { te.buf.data.len() });
                    let mut st = prev.start;
                    res = if last { te.tc.convert(&prev.data, &mut st, out, ostop, f) } else { let cap = te.buf.cap; te.tc.convert(&prev.data, &mut st, &mut te.buf.data, cap, f) };
                    prev.start = st;
                    moved_in = iold != st; moved_out = oold != if last { out.len() } else { te.buf.data.len() };
                }
                te.last_result = res;
                if moved_in || moved_out { tried = true; }
                match res {
                    Res::InvalidByteSequence | Res::IncompleteInput | Res::UndefinedConversion | Res::AfterOutput => return Some(i),
                    Res::DestinationBufferFull | Res::SourceBufferEmpty => {}
                    Res::Finished => self.num_finished = i + 1,
                }
                i += 1;
            }
        }
        None
    }

    /// rb_trans_conv
    fn trans_conv(&mut self, input: &[u8], ip: &mut usize, out: &mut Vec<u8>, ostop: usize, flags: u32, pos: &mut Option<usize>) -> Res {
        let n = self.elems.len();
        if self.elems[0].last_result == Res::AfterOutput { self.elems[0].last_result = Res::SourceBufferEmpty; }
        let mut sweep_start = None;
        for i in (0..n).rev() {
            match self.elems[i].last_result {
                Res::InvalidByteSequence | Res::IncompleteInput | Res::UndefinedConversion | Res::AfterOutput | Res::Finished => { sweep_start = Some(i + 1); break; }
                _ => {}
            }
        }
        let mut sweep_start = match sweep_start {
            Some(s) => s,
            None => {
                if self.elems[n - 1].last_result == Res::DestinationBufferFull && flags & AFTER_OUTPUT != 0 {
                    let mut z = 0;
                    let res = self.trans_conv(&[], &mut z, out, ostop, (flags & !AFTER_OUTPUT) | PARTIAL_INPUT, pos);
                    return if res == Res::SourceBufferEmpty { Res::AfterOutput } else { res };
                }
                0
            }
        };
        loop {
            let ni = self.trans_sweep(input, ip, out, ostop, flags, sweep_start);
            match ni { Some(k) if k != n - 1 => sweep_start = k + 1, _ => break }
        }
        for i in (0..n).rev() {
            let res = self.elems[i].last_result;
            if res != Res::SourceBufferEmpty {
                if matches!(res, Res::InvalidByteSequence | Res::IncompleteInput | Res::UndefinedConversion | Res::AfterOutput) {
                    self.elems[i].last_result = Res::SourceBufferEmpty;
                }
                *pos = Some(i);
                return res;
            }
        }
        *pos = None;
        Res::SourceBufferEmpty
    }

    /// rb_econv_convert0
    fn convert0(&mut self, input: &[u8], ip: &mut usize, out: &mut Vec<u8>, ostop: usize, mut flags: u32) -> Res {
        self.last_error = LastError::default();
        let mut pos = None;
        let res = 'got: {
            if self.elems.is_empty() {
                if let Some(b) = self.in_buf.as_mut() {
                    if b.pending() {
                        let len = b.data.len() - b.start;
                        let room = ostop.saturating_sub(out.len());
                        if room < len { out.extend_from_slice(&b.data[b.start..b.start + room]); b.start += room; break 'got Res::DestinationBufferFull; }
                        out.extend_from_slice(&b.data[b.start..]);
                        b.data.clear(); b.start = 0;
                        if flags & AFTER_OUTPUT != 0 { break 'got Res::AfterOutput; }
                    }
                }
                let room = ostop.saturating_sub(out.len());
                let len = room.min(input.len() - *ip);
                if len > 0 && flags & AFTER_OUTPUT != 0 { out.push(input[*ip]); *ip += 1; break 'got Res::AfterOutput; }
                out.extend_from_slice(&input[*ip..*ip + len]);
                *ip += len;
                break 'got if *ip != input.len() { Res::DestinationBufferFull } else if flags & PARTIAL_INPUT != 0 { Res::SourceBufferEmpty } else { Res::Finished };
            }
            let n = self.elems.len();
            let mut has_output = false;
            {
                let lb = &mut self.elems[n - 1].buf;
                if lb.pending() {
                    let len = lb.data.len() - lb.start;
                    let room = ostop.saturating_sub(out.len());
                    if room < len { out.extend_from_slice(&lb.data[lb.start..lb.start + room]); lb.start += room; break 'got Res::DestinationBufferFull; }
                    out.extend_from_slice(&lb.data[lb.start..]);
                    lb.data.clear(); lb.start = 0;
                    has_output = true;
                }
            }
            if self.in_buf.as_ref().is_some_and(|b| b.pending()) {
                let mut b = self.in_buf.take().unwrap();
                let mut st = b.start;
                let res = self.trans_conv(&b.data, &mut st, out, ostop, (flags & !AFTER_OUTPUT) | PARTIAL_INPUT, &mut pos);
                b.start = st;
                self.in_buf = Some(b);
                if res != Res::SourceBufferEmpty { break 'got res; }
            }
            if has_output && flags & AFTER_OUTPUT != 0 && *ip != input.len() {
                let stop = *ip;
                let res = self.trans_conv(&input[..stop], ip, out, ostop, flags, &mut pos);
                if res == Res::SourceBufferEmpty { Res::AfterOutput } else { res }
            } else if flags & AFTER_OUTPUT != 0 || n == 1 {
                self.trans_conv(input, ip, out, ostop, flags, &mut pos)
            } else {
                flags |= AFTER_OUTPUT;
                loop {
                    let r = self.trans_conv(input, ip, out, ostop, flags, &mut pos);
                    if r != Res::AfterOutput { break r; }
                }
            }
        };
        self.last_error.result = Some(res);
        if matches!(res, Res::InvalidByteSequence | Res::IncompleteInput | Res::UndefinedConversion) {
            if let Some(p) = pos {
                let tc = &self.elems[p].tc;
                self.last_error.error_tc = Some(p);
                self.last_error.src = tc.tr.src;
                self.last_error.dst = tc.tr.dst;
                self.last_error.error_bytes = tc.readbuf[..tc.recognized_len].to_vec();
                self.last_error.readagain = tc.readagain().to_vec();
            }
        }
        res
    }

    /// rb_econv_convert: errors are handled by the error-handler flags (replacement, hex charref).
    pub fn convert(&mut self, input: &[u8], ip: &mut usize, out: &mut Vec<u8>, ostop: usize, flags: u32) -> Res {
        self.started = true;
        loop {
            let ret = self.convert0(input, ip, out, ostop, flags);
            if matches!(ret, Res::InvalidByteSequence | Res::IncompleteInput) && self.flags & INVALID_MASK == INVALID_REPLACE
                && self.output_replacement() { continue; }
            if ret == Res::UndefinedConversion {
                match self.flags & UNDEF_MASK {
                    UNDEF_REPLACE => if self.output_replacement() { continue; },
                    UNDEF_HEX_CHARREF => if self.output_hex_charref() { continue; },
                    _ => {}
                }
            }
            return ret;
        }
    }

    /// rb_econv_encoding_to_insert_output
    pub fn insert_encoding(&self) -> &'static str {
        let Some(i) = self.last_tc else { return "" };
        let tr = self.elems[i].tc.tr;
        if tr.asciicompat == 2 { tr.src } else { tr.dst }
    }

    /// rb_econv_insert_output; false on failure.
    pub fn insert_output(&mut self, s: &[u8], s_enc: &str) -> bool {
        self.started = true;
        if s.is_empty() { return true; }
        let ins_enc = self.insert_encoding();
        let conv;
        let s = if enc_eq(ins_enc, s_enc) { s } else { match convert_string(s_enc, ins_enc, s) { Some(v) => { conv = v; &conv[..] } None => return false } };
        let n = self.elems.len();
        let encoder_last = n > 0 && self.elems[n - 1].tc.tr.asciicompat == 2;
        let tail: Vec<u8> = if encoder_last { let tc = &mut self.elems[n - 1].tc; let t = tc.readagain().to_vec(); tc.readagain_len = 0; t } else { vec![] };
        let b: &mut Buf = if n == 0 || encoder_last && n == 1 { self.in_buf.get_or_insert_with(Buf::default) }
                          else if encoder_last { &mut self.elems[n - 2].buf } else { &mut self.elems[n - 1].buf };
        b.compact();
        b.data.extend_from_slice(s);
        b.data.extend_from_slice(&tail);
        if b.cap < b.data.len() { b.cap = b.data.len(); }
        true
    }

    fn make_replacement(&mut self) {
        if self.replacement.is_some() { return; }
        let ins = self.insert_encoding();
        self.replacement = Some(if ins.is_empty() { (b"?".to_vec(), String::new()) }
                                else if enc_eq(ins, "UTF-8") { (vec![0xEF, 0xBF, 0xBD], "UTF-8".into()) } else { (b"?".to_vec(), "US-ASCII".into()) });
    }
    /// The replacement string and its encoding name (made on first use).
    pub fn replacement(&mut self) -> (Vec<u8>, String) { self.make_replacement(); self.replacement.clone().unwrap() }
    /// rb_econv_set_replacement; false when it cannot be converted.
    pub fn set_replacement(&mut self, s: &[u8], enc: &str) -> bool {
        let ins = self.insert_encoding();
        if ins.is_empty() || enc_eq(enc, ins) { self.replacement = Some((s.to_vec(), enc.to_string())); return true; }
        match convert_string(enc, ins, s) { Some(v) => { self.replacement = Some((v, ins.to_string())); true } None => false }
    }
    fn output_replacement(&mut self) -> bool {
        self.make_replacement();
        let (s, e) = self.replacement.clone().unwrap();
        self.insert_output(&s, &e)
    }
    fn output_hex_charref(&mut self) -> bool {
        let le = &self.last_error;
        let utf = if enc_eq(le.src, "UTF-32BE") { Some(le.error_bytes.clone()) } else { convert_string(le.src, "UTF-32BE", &le.error_bytes) };
        let Some(utf) = utf else { return false };
        if utf.len() % 4 != 0 { return false; }
        for w in utf.chunks(4) {
            let u = u32::from_be_bytes([w[0], w[1], w[2], w[3]]);
            if !self.insert_output(format!("&#x{:X};", u).as_bytes(), "US-ASCII") { return false; }
        }
        true
    }

    /// rb_econv_putbackable / rb_econv_putback
    pub fn putbackable(&self) -> usize { self.elems.first().map(|e| e.tc.readagain_len).unwrap_or(0) }
    pub fn putback(&mut self, n: usize) -> Vec<u8> {
        if self.elems.is_empty() || n == 0 { return vec![]; }
        let tc = &mut self.elems[0].tc;
        let end = tc.recognized_len + tc.readagain_len;
        let v = tc.readbuf[end - n..end].to_vec();
        tc.readagain_len -= n;
        v
    }

    /// rb_econv_binmode: drop the newline decorator.
    pub fn binmode(&mut self) {
        let d = match self.flags & NEWLINE_DECORATOR_MASK {
            UNIVERSAL_NEWLINE_DECORATOR => Some("universal_newline"), CRLF_NEWLINE_DECORATOR => Some("crlf_newline"),
            CR_NEWLINE_DECORATOR => Some("cr_newline"), LF_NEWLINE_DECORATOR => Some("lf_newline"), _ => None,
        };
        if let Some(d) = d {
            let lt = self.last_tc.map(|i| self.elems[i].tc.tr as *const TrDef);
            self.elems.retain(|e| !(is_decorator(e.tc.tr) && e.tc.tr.dst == d));
            self.last_tc = lt.and_then(|p| self.elems.iter().position(|e| std::ptr::eq(e.tc.tr, p)));
        }
        self.flags &= !NEWLINE_DECORATOR_MASK;
    }
}

/// decorator_names; None for contradictory flags.
pub fn decorator_names(ecflags: u32) -> Option<Vec<&'static str>> {
    match ecflags & NEWLINE_DECORATOR_MASK {
        UNIVERSAL_NEWLINE_DECORATOR | CRLF_NEWLINE_DECORATOR | CR_NEWLINE_DECORATOR | LF_NEWLINE_DECORATOR | 0 => {}
        _ => return None,
    }
    if ecflags & XML_TEXT_DECORATOR != 0 && ecflags & XML_ATTR_CONTENT_DECORATOR != 0 { return None; }
    let mut v = vec![];
    if ecflags & XML_TEXT_DECORATOR != 0 { v.push("xml_text_escape"); }
    if ecflags & XML_ATTR_CONTENT_DECORATOR != 0 { v.push("xml_attr_content_escape"); }
    if ecflags & XML_ATTR_QUOTE_DECORATOR != 0 { v.push("xml_attr_quote"); }
    if ecflags & CRLF_NEWLINE_DECORATOR != 0 { v.push("crlf_newline"); }
    if ecflags & CR_NEWLINE_DECORATOR != 0 { v.push("cr_newline"); }
    if ecflags & LF_NEWLINE_DECORATOR != 0 { v.push("lf_newline"); }
    if ecflags & UNIVERSAL_NEWLINE_DECORATOR != 0 { v.push("universal_newline"); }
    Some(v)
}

/// allocate_converted_string: the whole of `s` converted, None on any error.
pub fn convert_string(sname: &str, dname: &str, s: &[u8]) -> Option<Vec<u8>> {
    let mut ec = Econv::open(sname, dname, 0)?;
    let mut out = Vec::with_capacity(s.len().max(1));
    let mut ip = 0;
    let mut cap = s.len().max(1);
    loop {
        match ec.convert(s, &mut ip, &mut out, cap, 0) {
            Res::DestinationBufferFull => cap *= 2,
            Res::Finished => return Some(out),
            _ => return None,
        }
    }
}

// ---- transcoder functions (enc/trans/*.trans) ---------------------------------------------------

const G0_ASCII: u8 = 0;
const G0_JISX0208_1978: u8 = 1;
const G0_JISX0208_1983: u8 = 2;
const G0_JISX0201_KATAKANA: u8 = 3;
const EMACS_MULE_JISX0208_1978: u8 = 0o220;
const EMACS_MULE_JISX0208_1983: u8 = 0o222;

fn call_init(f: TrFn, t: &mut Tc) {
    match f {
        TrFn::FromUtf8MacInit => t.mac = MacBuf::default(),
        TrFn::None => {}
        _ => t.st = [0; 3],
    }
}

fn call_si(f: TrFn, t: &mut Tc, s: &[u8]) -> u32 {
    let sp = &mut t.st;
    match f {
        TrFn::FunSiIso2022jpDecoder => {
            if sp[0] == G0_ASCII { NOMAP } else if (0x21..=0x7e).contains(&s[0]) { tt::ISO2022JP_DECODER_JISX0208_REST } else { INVALID }
        }
        TrFn::FunSiIso2022jpKddiDecoder => {
            if sp[0] == G0_ASCII { NOMAP } else if (0x21..=0x7e).contains(&s[0]) { tt::ISO2022JP_KDDI_DECODER_JISX0208_REST } else { INVALID }
        }
        TrFn::FunSiCp50221Decoder => {
            match sp[0] {
                G0_ASCII => if (0xA1..=0xDF).contains(&s[0]) { FUNSO } else { NOMAP },
                G0_JISX0201_KATAKANA => { let c = s[0] & 0x7F; if (0x21..=0x5f).contains(&c) { FUNSO } else { INVALID } }
                G0_JISX0208_1978 => if (0x21..=0x28).contains(&s[0]) || (0x30..=0x74).contains(&s[0]) { tt::ISO2022JP_DECODER_JISX0208_REST } else { INVALID },
                G0_JISX0208_1983 => if (0x21..=0x28).contains(&s[0]) || s[0] == 0x2D || (0x30..=0x74).contains(&s[0]) || (0x79..=0x7C).contains(&s[0]) { tt::ISO2022JP_DECODER_JISX0208_REST } else { INVALID },
                _ => INVALID,
            }
        }
        TrFn::FunSiFromUtf16 => {
            match sp[0] {
                0 => {
                    if s[0] == 0xFE && s[1] == 0xFF { sp[0] = 1; return ZEROBT; }
                    if s[0] == 0xFF && s[1] == 0xFE { sp[0] = 2; return ZEROBT; }
                    INVALID
                }
                1 => if !(0xD8..=0xDF).contains(&s[0]) { FUNSO } else if s[0] <= 0xDB { tt::FROM_UTF_16BE_D8TODB_00TOFF } else { INVALID },
                2 => if !(0xD8..=0xDF).contains(&s[1]) { FUNSO } else if s[1] <= 0xDB { tt::FROM_UTF_16LE_00TOFF_D8TODB } else { INVALID },
                _ => INVALID,
            }
        }
        TrFn::FunSiFromUtf32 => {
            match sp[0] {
                0 => {
                    if s[0] == 0 && s[1] == 0 && s[2] == 0xFE && s[3] == 0xFF { sp[0] = 1; return ZEROBT; }
                    if s[0] == 0xFF && s[1] == 0xFE && s[2] == 0 && s[3] == 0 { sp[0] = 2; return ZEROBT; }
                    INVALID
                }
                1 => if s[0] == 0 && ((0 < s[1] && s[1] <= 0x10) || (s[1] == 0 && !(0xD8..=0xDF).contains(&s[2]))) { FUNSO } else { INVALID },
                2 => if s[3] == 0 && ((0 < s[2] && s[2] <= 0x10) || (s[2] == 0 && !(0xD8..=0xDF).contains(&s[1]))) { FUNSO } else { INVALID },
                _ => INVALID,
            }
        }
        _ => INVALID,
    }
}

fn iso2022jp_put_state(sp: &mut [u8; 3], o: &mut Vec<u8>, old: u8, new: u8) {
    if old != new {
        o.push(0x1b);
        o.extend_from_slice(match new { G0_ASCII => b"(B", G0_JISX0201_KATAKANA => b"(I", G0_JISX0208_1978 => b"$@", _ => b"$B" });
        sp[0] = new;
    }
}
const TBL0208: [[u8; 2]; 0xDF - 0xA1 + 1] = [
    [0x21, 0x23], [0x21, 0x56], [0x21, 0x57], [0x21, 0x22], [0x21, 0x26], [0x25, 0x72], [0x25, 0x21], [0x25, 0x23],
    [0x25, 0x25], [0x25, 0x27], [0x25, 0x29], [0x25, 0x63], [0x25, 0x65], [0x25, 0x67], [0x25, 0x43], [0x21, 0x3C],
    [0x25, 0x22], [0x25, 0x24], [0x25, 0x26], [0x25, 0x28], [0x25, 0x2A], [0x25, 0x2B], [0x25, 0x2D], [0x25, 0x2F],
    [0x25, 0x31], [0x25, 0x33], [0x25, 0x35], [0x25, 0x37], [0x25, 0x39], [0x25, 0x3B], [0x25, 0x3D], [0x25, 0x3F],
    [0x25, 0x41], [0x25, 0x44], [0x25, 0x46], [0x25, 0x48], [0x25, 0x4A], [0x25, 0x4B], [0x25, 0x4C], [0x25, 0x4D],
    [0x25, 0x4E], [0x25, 0x4F], [0x25, 0x52], [0x25, 0x55], [0x25, 0x58], [0x25, 0x5B], [0x25, 0x5E], [0x25, 0x5F],
    [0x25, 0x60], [0x25, 0x61], [0x25, 0x62], [0x25, 0x64], [0x25, 0x66], [0x25, 0x68], [0x25, 0x69], [0x25, 0x6A],
    [0x25, 0x6B], [0x25, 0x6C], [0x25, 0x6D], [0x25, 0x6F], [0x25, 0x73], [0x21, 0x2B], [0x21, 0x2C],
];

fn cp5022x_encoder(sp: &mut [u8; 3], s: &[u8], o: &mut Vec<u8>) {
    let (s, l, new) = if s.len() == 1 { (s, 1, G0_ASCII) } else if s[0] == 0x8E { (&s[1..], 1, G0_JISX0201_KATAKANA) } else { (s, 2, G0_JISX0208_1983) };
    let old = sp[0];
    iso2022jp_put_state(sp, o, old, new);
    if l == 1 { o.push(s[0] & 0x7f); } else { o.push(s[0] & 0x7f); o.push(s[1] & 0x7f); }
}

fn utf16be_from(s: &[u8], o: &mut Vec<u8>) {
    let (s0, s1) = (s[0] as u32, s[1] as u32);
    if s0 == 0 && s1 < 0x80 { o.push(s1 as u8); }
    else if s0 < 0x08 { o.extend_from_slice(&[(0xC0 | (s0 << 2) | (s1 >> 6)) as u8, (0x80 | (s1 & 0x3F)) as u8]); }
    else if (s0 & 0xF8) != 0xD8 { o.extend_from_slice(&[(0xE0 | (s0 >> 4)) as u8, (0x80 | ((s0 & 0x0F) << 2) | (s1 >> 6)) as u8, (0x80 | (s1 & 0x3F)) as u8]); }
    else {
        let (s2, s3) = (s[2] as u32, s[3] as u32);
        let u = (((s0 & 0x03) << 2) | (s1 >> 6)) + 1;
        o.extend_from_slice(&[(0xF0 | (u >> 2)) as u8, (0x80 | ((u & 0x03) << 4) | ((s1 >> 2) & 0x0F)) as u8,
                              (0x80 | ((s1 & 0x03) << 4) | ((s2 & 0x03) << 2) | (s3 >> 6)) as u8, (0x80 | (s3 & 0x3F)) as u8]);
    }
}
fn utf16le_from(s: &[u8], o: &mut Vec<u8>) {
    let sw = if s.len() >= 4 { [s[1], s[0], s[3], s[2]] } else { [s[1], s[0], 0, 0] };
    utf16be_from(&sw[..s.len().min(4)], o);
}
/// UTF-8 -> UTF-16BE code units (fun_so_to_utf_16be); `le` swaps each unit.
fn utf16_to(s: &[u8], o: &mut Vec<u8>, le: bool) {
    let s0 = s[0] as u32;
    let mut put = |a: u32, b: u32| if le { o.extend_from_slice(&[b as u8, a as u8]) } else { o.extend_from_slice(&[a as u8, b as u8]) };
    if s0 & 0x80 == 0 { put(0, s0); }
    else if (s0 & 0xE0) == 0xC0 { let s1 = s[1] as u32; put((s0 >> 2) & 0x07, ((s0 & 0x03) << 6) | (s1 & 0x3F)); }
    else if (s0 & 0xF0) == 0xE0 { let (s1, s2) = (s[1] as u32, s[2] as u32); put((s0 << 4) | ((s1 >> 2) ^ 0x20), (s1 << 6) | (s2 ^ 0x80)); }
    else {
        let (s1, s2, s3) = (s[1] as i32, s[2] as i32, s[3] as i32);
        let w = (((s0 as i32 & 0x07) << 2) | ((s1 >> 4) & 0x03)) - 1;
        put((0xD8 | (w >> 2)) as u32, ((w << 6) | ((s1 & 0x0F) << 2) | ((s2 >> 4) - 8)) as u32);
        put((0xDC | ((s2 >> 2) & 0x03)) as u32, ((s2 << 6) | (s3 & !0x80)) as u32);
    }
}
fn utf32be_from(s: &[u8], o: &mut Vec<u8>) {
    let (s1, s2, s3) = (s[1] as u32, s[2] as u32, s[3] as u32);
    if s1 == 0 {
        if s2 == 0 && s3 < 0x80 { o.push(s3 as u8); }
        else if s2 < 0x08 { o.extend_from_slice(&[(0xC0 | (s2 << 2) | (s3 >> 6)) as u8, (0x80 | (s3 & 0x3F)) as u8]); }
        else { o.extend_from_slice(&[(0xE0 | (s2 >> 4)) as u8, (0x80 | ((s2 & 0x0F) << 2) | (s3 >> 6)) as u8, (0x80 | (s3 & 0x3F)) as u8]); }
    } else {
        o.extend_from_slice(&[(0xF0 | (s1 >> 2)) as u8, (0x80 | ((s1 & 0x03) << 4) | (s2 >> 4)) as u8, (0x80 | ((s2 & 0x0F) << 2) | (s3 >> 6)) as u8, (0x80 | (s3 & 0x3F)) as u8]);
    }
}
fn utf32_to(s: &[u8], o: &mut Vec<u8>, le: bool) {
    let s0 = s[0] as u32;
    let b = if s0 & 0x80 == 0 { [0, 0, 0, s0] }
        else if (s0 & 0xE0) == 0xC0 { let s1 = s[1] as u32; [0, 0, (s0 >> 2) & 0x07, ((s0 & 0x03) << 6) | (s1 & 0x3F)] }
        else if (s0 & 0xF0) == 0xE0 { let (s1, s2) = (s[1] as u32, s[2] as u32); [0, 0, (s0 << 4) | ((s1 >> 2) ^ 0x20), (s1 << 6) | (s2 ^ 0x80)] }
        else { let (s1, s2, s3) = (s[1] as u32, s[2] as u32, s[3] as u32); [0, ((s0 & 0x07) << 2) | ((s1 >> 4) & 0x03), ((s1 & 0x0F) << 4) | ((s2 >> 2) & 0x0F), ((s2 & 0x03) << 6) | (s3 & 0x3F)] };
    let b = b.map(|x| x as u8);
    if le { o.extend_from_slice(&[b[3], b[2], b[1], b[0]]); } else { o.extend_from_slice(&b); }
}
fn utf8_4(u: u32, o: &mut Vec<u8>) { o.extend_from_slice(&[(0xF0 | (u >> 18)) as u8, (0x80 | ((u >> 12) & 0x3F)) as u8, (0x80 | ((u >> 6) & 0x3F)) as u8, (0x80 | (u & 0x3F)) as u8]); }

fn call_so(f: TrFn, t: &mut Tc, s: &[u8], o: &mut Vec<u8>) {
    let l = s.len();
    match f {
        TrFn::FunSoEucjp2sjis => {
            if s[0] == 0x8e { o.push(s[1]); }
            else {
                let m = (s[0] & 1) as i32;
                let mut h = (s[0] as i32 + m) >> 1;
                h += if s[0] < 0xdf { 0x30 } else { 0x70 };
                let mut lo = s[1] as i32 - m * 94 - 3;
                if 0x7f <= lo { lo += 1; }
                o.extend_from_slice(&[h as u8, lo as u8]);
            }
        }
        TrFn::FunSoSjis2eucjp => {
            if l == 1 { o.extend_from_slice(&[0x8e, s[0]]); }
            else {
                let mut h = s[0] as i32;
                let mut lo = s[1] as i32;
                if 0xe0 <= h { h -= 64; }
                lo += if lo < 0x80 { 0x61 } else { 0x60 };
                h = h * 2 - 0x61;
                if 0xfe < lo { lo -= 94; h += 1; }
                o.extend_from_slice(&[h as u8, lo as u8]);
            }
        }
        TrFn::FunSoIso2022jpDecoder | TrFn::FunSoIso2022jpKddiDecoder => {
            let sp = &mut t.st;
            if s[0] == 0x1b {
                if s[1] == b'(' { if matches!(s[l - 1], b'B' | b'J') { sp[0] = G0_ASCII; } }
                else { match s[l - 1] { b'@' => sp[0] = G0_JISX0208_1978, b'B' => sp[0] = G0_JISX0208_1983, _ => {} } }
            } else {
                o.push(if sp[0] == G0_JISX0208_1978 { EMACS_MULE_JISX0208_1978 } else { EMACS_MULE_JISX0208_1983 });
                o.extend_from_slice(&[s[0] | 0x80, s[1] | 0x80]);
            }
        }
        TrFn::FunSoIso2022jpEncoder | TrFn::FunSoIso2022jpKddiEncoder => {
            let new = if l == 1 { G0_ASCII } else if s[0] == EMACS_MULE_JISX0208_1978 { G0_JISX0208_1978 } else { G0_JISX0208_1983 };
            let old = t.st[0];
            iso2022jp_put_state(&mut t.st, o, old, new);
            if l == 1 { o.push(s[0] & 0x7f); } else { o.extend_from_slice(&[s[1] & 0x7f, s[2] & 0x7f]); }
        }
        TrFn::FunSoStatelessIso2022jpToEucjp => o.extend_from_slice(&[s[1], s[2]]),
        TrFn::FunSoEucjpToStatelessIso2022jp => o.extend_from_slice(&[EMACS_MULE_JISX0208_1983, s[0], s[1]]),
        TrFn::FunSoCp50221Decoder => {
            let sp = &mut t.st;
            match s[0] {
                0x1b => {
                    if s[1] == b'(' { match s[l - 1] { b'B' | b'J' => sp[0] = G0_ASCII, b'I' => sp[0] = G0_JISX0201_KATAKANA, _ => {} } }
                    else { match s[l - 1] { b'@' => sp[0] = G0_JISX0208_1978, b'B' => sp[0] = G0_JISX0208_1983, _ => {} } }
                }
                0x0E => sp[0] = G0_JISX0201_KATAKANA,
                0x0F => sp[0] = G0_ASCII,
                _ => {
                    if sp[0] == G0_JISX0201_KATAKANA || ((0xA1..=0xDF).contains(&s[0]) && sp[0] == G0_ASCII) { o.extend_from_slice(&[0x8E, s[0] | 0x80]); }
                    else { o.extend_from_slice(&[s[0] | 0x80, s[1] | 0x80]); }
                }
            }
        }
        TrFn::FunSoCp5022xEncoder => cp5022x_encoder(&mut t.st, s, o),
        TrFn::FunSoCp50220Encoder => {
            let sp = &mut t.st;
            if sp[0] == G0_JISX0201_KATAKANA && sp[2] != 0 {
                let c = (sp[2] & 0x7F) as usize;
                let p = TBL0208[c - 0x21];
                sp[2] = 0;
                let old = sp[1];
                iso2022jp_put_state(sp, o, old, G0_JISX0208_1983);
                sp[0] = G0_JISX0208_1983;
                o.push(p[0]);
                if l == 2 && s[0] == 0x8E {
                    if s[1] == 0xDE { o.push(p[1] + 1); return; }
                    if s[1] == 0xDF && (0x4A..=0x4E).contains(&c) { o.push(p[1] + 2); return; }
                }
                o.push(p[1]);
            }
            if l == 2 && s[0] == 0x8E {
                if (0xA1..=0xB5).contains(&s[1]) || (0xC5..=0xC9).contains(&s[1]) || (0xCF..=0xDF).contains(&s[1]) {
                    let p = TBL0208[(s[1] - 0xA1) as usize];
                    let old = sp[0];
                    iso2022jp_put_state(sp, o, old, G0_JISX0208_1983);
                    o.extend_from_slice(&p);
                    return;
                }
                if s[1] > 0xDF {
                    let old = sp[0];
                    iso2022jp_put_state(sp, o, old, G0_JISX0201_KATAKANA);
                    o.push(s[1] & 0x7F);
                    sp[2] = 0;
                    return;
                }
                sp[2] = s[1];
                sp[1] = sp[0];
                sp[0] = G0_JISX0201_KATAKANA;
                return;
            }
            cp5022x_encoder(sp, s, o);
        }
        TrFn::FunSoUniversalNewline => {
            let sp = &mut t.st;
            if s[0] == b'\n' { sp[1] |= if sp[0] == 0 { 1 } else { 2 }; o.push(b'\n'); sp[0] = 0; }
            else {
                if sp[0] == 1 { o.push(b'\n'); sp[1] |= 4; }
                if s[0] == b'\r' { sp[0] = 1; } else { o.push(s[0]); sp[0] = 0; }
            }
        }
        TrFn::FunSoEscapeXmlAttrQuote => { if t.st[0] == 0 { t.st[0] = 1; o.push(b'"'); } o.push(s[0]); }
        TrFn::FunSoFromCesu8 => {
            let u = ((((s[1] & 0x0F) as u32) << 16) | (((s[2] & 0x3F) as u32) << 10) | (((s[4] & 0x0F) as u32) << 6) | (s[5] & 0x3F) as u32) + 0x10000;
            utf8_4(u, o);
        }
        TrFn::FunSoToCesu8 => {
            let u = ((((s[0] & 0x07) as u32) << 18) | (((s[1] & 0x3F) as u32) << 12) | (((s[2] & 0x3F) as u32) << 6) | (s[3] & 0x3F) as u32) - 0x10000;
            o.extend_from_slice(&[0xED, (0xA0 | (u >> 16)) as u8, (0x80 | ((u >> 10) & 0x3F)) as u8, 0xED, (0xB0 | ((u >> 6) & 0x0F)) as u8, (0x80 | (u & 0x3F)) as u8]);
        }
        TrFn::FunSoFromGb18030 => {
            let u = (s[0] as u32 - 0x90) * 10 * 126 * 10 + (s[1] as u32 - 0x30) * 126 * 10 + (s[2] as u32 - 0x81) * 10 + (s[3] as u32 - 0x30) + 0x10000;
            utf8_4(u, o);
        }
        TrFn::FunSoToGb18030 => {
            let mut u = ((((s[0] & 0x07) as u32) << 18) | (((s[1] & 0x3F) as u32) << 12) | (((s[2] & 0x3F) as u32) << 6) | (s[3] & 0x3F) as u32) - 0x10000;
            let mut b = [0u8; 4];
            b[3] = (0x30 + u % 10) as u8; u /= 10;
            b[2] = (0x81 + u % 126) as u8; u /= 126;
            b[1] = (0x30 + u % 10) as u8;
            b[0] = (0x90 + u / 10) as u8;
            o.extend_from_slice(&b);
        }
        TrFn::FunSoFromUtf16be => utf16be_from(s, o),
        TrFn::FunSoFromUtf16le => utf16le_from(s, o),
        TrFn::FunSoToUtf16be => utf16_to(s, o, false),
        TrFn::FunSoToUtf16le => utf16_to(s, o, true),
        TrFn::FunSoFromUtf32be => utf32be_from(s, o),
        TrFn::FunSoFromUtf32le => utf32be_from(&[s[3], s[2], s[1], s[0]], o),
        TrFn::FunSoToUtf32be => utf32_to(s, o, false),
        TrFn::FunSoToUtf32le => utf32_to(s, o, true),
        TrFn::FunSoFromUtf16 => match t.st[0] { 1 => utf16be_from(s, o), 2 => utf16le_from(s, o), _ => {} },
        TrFn::FunSoFromUtf32 => match t.st[0] { 1 => utf32be_from(s, o), 2 => utf32be_from(&[s[3], s[2], s[1], s[0]], o), _ => {} },
        TrFn::FunSoToUtf16 => { if t.st[0] == 0 { o.extend_from_slice(&[0xFE, 0xFF]); t.st[0] = 1; } utf16_to(s, o, false); }
        TrFn::FunSoToUtf32 => { if t.st[0] == 0 { o.extend_from_slice(&[0, 0, 0xFE, 0xFF]); t.st[0] = 1; } utf32_to(s, o, false); }
        TrFn::FunSoFromUtf8Mac => {
            let file = t.file();
            let sp = &mut t.mac;
            match l {
                1 => mac_output_all(sp, o),
                4 => { mac_output_all(sp, o); o.extend_from_slice(&s[..4]); return; }
                _ => {}
            }
            mac_push(sp, s);
            mac_apply(file, sp, o);
        }
        _ => {}
    }
}

fn call_sio(f: TrFn, _t: &mut Tc, s: &[u8], info: u32, o: &mut Vec<u8>) {
    let diff = info >> 8;
    match f {
        TrFn::FunSioFromGb18030 => {
            let (s0, s1) = (s[0] as u32, s[1] as u32);
            let u = if diff & 0x20000 != 0 { (((s0 * 10 + s1) * 126 + s[2] as u32) * 10 + s[3] as u32).wrapping_sub(diff).wrapping_sub(0x170000) }
                    else { (s0 * 256 + s1 + 24055).wrapping_sub(diff) };
            o.extend_from_slice(&[(0xE0 | (u >> 12)) as u8, (0x80 | ((u >> 6) & 0x3F)) as u8, (0x80 | (u & 0x3F)) as u8]);
        }
        TrFn::FunSioToGb18030 => {
            let mut u = (((s[0] & 0x0F) as u32) << 12) | (((s[1] & 0x3F) as u32) << 6) | (s[2] & 0x3F) as u32;
            if diff & 0x20000 != 0 {
                u = u.wrapping_add(diff + 0x170000).wrapping_sub(1688980).wrapping_add(2);
                let mut b = [0u8; 4];
                b[3] = (0x30 + u % 10) as u8; u /= 10; u += 0x32;
                b[2] = (0x81 + u % 126) as u8; u /= 126; u += 1;
                b[1] = (0x30 + u % 10) as u8; u /= 10;
                b[0] = (0x81 + u) as u8;
                o.extend_from_slice(&b);
            } else {
                u = u.wrapping_add(diff).wrapping_sub(24055);
                o.extend_from_slice(&[(u / 256) as u8, (u % 256) as u8]);
            }
        }
        _ => {}
    }
}

fn call_finish(f: TrFn, t: &mut Tc) -> Vec<u8> {
    let mut o = vec![];
    match f {
        TrFn::FinishIso2022jpEncoder | TrFn::FinishIso2022jpKddiEncoder => {
            if t.st[0] != G0_ASCII { let old = t.st[0]; iso2022jp_put_state(&mut t.st, &mut o, old, G0_ASCII); }
        }
        TrFn::FinishCp50220Encoder => {
            let sp = &mut t.st;
            if sp[0] == G0_ASCII { return o; }
            if sp[0] == G0_JISX0201_KATAKANA && sp[2] != 0 {
                let c = (sp[2] & 0x7F) as usize;
                let p = TBL0208[c - 0x21];
                let old = sp[1];
                iso2022jp_put_state(sp, &mut o, old, G0_JISX0208_1983);
                sp[0] = G0_JISX0208_1983;
                o.extend_from_slice(&p);
            }
            let old = sp[0];
            iso2022jp_put_state(sp, &mut o, old, G0_ASCII);
        }
        TrFn::UniversalNewlineFinish => { if t.st[0] == 1 { o.push(b'\n'); t.st[1] |= 4; } t.st[0] = 0; }
        TrFn::EscapeXmlAttrQuoteFinish => { if t.st[0] == 0 { o.push(b'"'); } o.push(b'"'); t.st[0] = 0; }
        TrFn::FromUtf8MacFinish => mac_output_all(&mut t.mac, &mut o),
        _ => {}
    }
    o
}

// utf8_mac.trans: decomposed sequences are held until they can be composed (NFC for the Mac set)
const MAC_BUF: usize = 16;
fn mac_size(sp: &MacBuf) -> usize { (sp.end + MAC_BUF - sp.beg) % MAC_BUF }
fn mac_push(sp: &mut MacBuf, s: &[u8]) { for &b in s { sp.buf[sp.end] = b; sp.end = (sp.end + 1) % MAC_BUF; } }
fn mac_shift(sp: &mut MacBuf) -> u8 { let c = sp.buf[sp.beg]; sp.beg = (sp.beg + 1) % MAC_BUF; c }
fn mac_at(sp: &MacBuf, pos: usize) -> u8 { sp.buf[(pos + sp.beg) % MAC_BUF] }
fn mac_output_char(sp: &mut MacBuf, o: &mut Vec<u8>) {
    while sp.beg != sp.end { o.push(mac_shift(sp)); if sp.buf[sp.beg] & 0xC0 != 0x80 { break; } }
}
fn mac_output_all(sp: &mut MacBuf, o: &mut Vec<u8>) { while sp.beg != sp.end { o.push(mac_shift(sp)); } }
fn mac_apply(f: &TrFile, sp: &mut MacBuf, o: &mut Vec<u8>) {
    let n = mac_size(sp);
    if n < 3 || (n == 3 && mac_at(sp, 0) >= 0xE0) { return; }
    let mut info = tt::FROM_UTF8_MAC_NFC2;
    let mut pos = 0;
    while pos < mac_size(sp) {
        let b = mac_at(sp, pos); pos += 1;
        info = lookup(f, info, b);
        if info & 3 != 0 { break; }
    }
    match info & 0x1f {
        THREEBT | TWOBT => {
            let mut nb = vec![(info >> 8) as u8, (info >> 16) as u8];
            if info & 0x1f == THREEBT { nb.push((info >> 24) as u8); }
            sp.beg = 0; sp.end = 0;
            mac_push(sp, &nb);
        }
        _ => mac_output_char(sp, o),
    }
}

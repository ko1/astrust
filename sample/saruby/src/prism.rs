// prism (C) bridge: pm_serialize_parse → bytes → Rust AST (prism_ast.rs).
use std::os::raw::c_char;

#[repr(C)]
pub struct PmBuffer { _p: [u8; 0] }

extern "C" {
    fn pm_buffer_new() -> *mut PmBuffer;
    fn pm_buffer_free(b: *mut PmBuffer);
    fn pm_buffer_value(b: *const PmBuffer) -> *const c_char;
    fn pm_buffer_length(b: *const PmBuffer) -> usize;
    fn pm_serialize_parse(b: *mut PmBuffer, src: *const u8, size: usize, data: *const c_char);
    fn pm_serialize_lex(b: *mut PmBuffer, src: *const u8, size: usize, data: *const c_char);
    fn pm_serialize_parse_errors_format(b: *mut PmBuffer, src: *const u8, size: usize, data: *const c_char, format: i32) -> i8;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Loc { pub start: u32, pub len: u32 }

#[derive(Debug, Clone)]
pub struct PInt { pub neg: bool, pub words: Vec<u32> }

pub type Cst = u32;

pub struct Reader<'a> { pub buf: &'a [u8], pub pos: usize }

impl<'a> Reader<'a> {
    #[inline] pub fn byte(&mut self) -> u8 { let b = self.buf[self.pos]; self.pos += 1; b }
    pub fn skip(&mut self, n: usize) { self.pos += n; }
    pub fn varu(&mut self) -> u32 {
        let mut r: u32 = 0; let mut sh = 0;
        loop { let b = self.byte(); r |= ((b & 0x7f) as u32) << sh; sh += 7; if b & 0x80 == 0 { break; } }
        r
    }
    pub fn vars(&mut self) -> i32 { let v = self.varu(); ((v >> 1) as i32) ^ -((v & 1) as i32) }
    pub fn loc(&mut self) -> Loc { let start = self.varu(); let len = self.varu(); Loc { start, len } }
    pub fn opt_loc(&mut self) -> Option<Loc> { if self.byte() != 0 { Some(self.loc()) } else { None } }
    pub fn string(&mut self) -> Vec<u8> { let n = self.varu() as usize; let s = self.buf[self.pos..self.pos + n].to_vec(); self.pos += n; s }
    pub fn u32le(&mut self) -> u32 { let b = &self.buf[self.pos..self.pos + 4]; self.pos += 4; u32::from_le_bytes([b[0], b[1], b[2], b[3]]) }
    pub fn double(&mut self) -> f64 { let b = &self.buf[self.pos..self.pos + 8]; self.pos += 8; f64::from_ne_bytes(b.try_into().unwrap()) }
    pub fn integer(&mut self) -> PInt {
        let neg = self.byte() != 0;
        let n = self.varu();
        let words = (0..n).map(|_| self.varu()).collect();
        PInt { neg, words }
    }
    pub fn opt_node(&mut self) -> Option<Box<crate::prism_ast::Node>> {
        if self.buf[self.pos] == 0 { self.pos += 1; None } else { Some(Box::new(self.node())) }
    }
}

pub struct Diag { pub msg: String, pub loc: Loc, pub level: u8 }

pub struct ParseResult {
    pub root: crate::prism_ast::Node,
    pub constants: Vec<Vec<u8>>,
    pub line_offsets: Vec<u32>,
    pub start_line: i32,
    pub errors: Vec<Diag>,
    pub warnings: Vec<Diag>,
    pub data_loc: Option<Loc>,
    pub encoding: String,
}

impl ParseResult {
    pub fn line_of(&self, off: u32) -> u32 {
        let i = match self.line_offsets.binary_search(&off) { Ok(i) => i, Err(i) => i - 1 };
        (i as i32 + self.start_line) as u32
    }
    pub fn col_of(&self, off: u32) -> u32 {
        let i = match self.line_offsets.binary_search(&off) { Ok(i) => i, Err(i) => i - 1 };
        off - self.line_offsets[i]
    }
}

include!("prism_opts.rs");

pub fn parse(src: &[u8], o: &ParseOpts) -> ParseResult { read_serialized(&serialize(src, o)) }

/// pm_serialize_parse's output for `src`.
pub fn serialize(src: &[u8], o: &ParseOpts) -> Vec<u8> { call(src, o, 0).1 }

/// The error level and message prism (pm_errors_format) gives a source's errors, as CRuby's
/// pm_parse_process_error: `format` 1 plain, 2 bold, 3 color.  The source is parsed again (errors only).
pub fn format_errors(src: &[u8], o: &ParseOpts, format: i32) -> (u8, Vec<u8>, String) {
    let (lv, v) = call(src, o, format);
    // the buffer: the source encoding's name, NUL, the message
    let z = v.iter().position(|&b| b == 0).unwrap_or(0);
    (lv.max(0) as u8, v[(z + 1).min(v.len())..].to_vec(), String::from_utf8_lossy(&v[..z]).into_owned())
}

/// The tokens of pm_serialize_lex (the lexer's callback, in lexing order): (type, start, length, lex state).
pub fn lex(src: &[u8], o: &ParseOpts) -> Vec<(u32, u32, u32, u32)> {
    let v = call(src, o, -1).1;
    let mut r = Reader { buf: &v, pos: 0 };
    let mut t = vec![];
    loop {
        let ty = r.varu();
        if ty == 0 { break; }
        t.push((ty, r.varu(), r.varu(), r.varu()));
    }
    t
}
/// The messages of a parse's errors, in prism's order.
pub fn error_messages(src: &[u8], o: &ParseOpts) -> Vec<String> {
    let v = serialize(src, o);
    read_prefix(&mut Reader { buf: &v, pos: 0 }).errors.into_iter().map(|d| d.msg).collect()
}

/// `format` 0: pm_serialize_parse; -1: pm_serialize_lex; else pm_serialize_parse_errors_format.
fn call(src: &[u8], o: &ParseOpts, format: i32) -> (i8, Vec<u8>) {
    let opts = build_options(o);
    // SAFETY: prism reads `src` / `opts` and writes only the buffer it allocated, copied out before it is freed
    unsafe {
        let b = pm_buffer_new();
        let (s, n, d) = (src.as_ptr(), src.len(), opts.as_ptr() as *const c_char);
        let lv = match format { 0 => { pm_serialize_parse(b, s, n, d); 0 } -1 => { pm_serialize_lex(b, s, n, d); 0 } f => pm_serialize_parse_errors_format(b, s, n, d, f) };
        let v = std::slice::from_raw_parts(pm_buffer_value(b) as *const u8, pm_buffer_length(b)).to_vec();
        pm_buffer_free(b);
        (lv, v)
    }
}

/// What a serialization holds before its root node; `r` is left at the root.
pub struct Prefix { pub encoding: String, pub start_line: i32, pub line_offsets: Vec<u32>, pub data_loc: Option<Loc>,
    pub errors: Vec<Diag>, pub warnings: Vec<Diag>, pool_off: usize, pool_n: usize }
pub fn read_prefix(r: &mut Reader) -> Prefix {
    assert!(&r.buf[0..5] == b"PRISM", "prism: bad serialization");
    r.pos = 5;
    let _maj = r.byte(); let _min = r.byte(); let _pat = r.byte();
    let _sem = r.byte();
    let encoding = String::from_utf8_lossy(&r.string()).into_owned();
    let start_line = r.vars();
    let nl = r.varu();
    let line_offsets: Vec<u32> = (0..nl).map(|_| r.varu()).collect();
    let nc = r.varu();
    for _ in 0..nc { r.byte(); r.loc(); }
    let na = r.varu();
    for _ in 0..na { r.varu(); let k = r.varu(); for _ in 0..k { r.varu(); } }
    let nm = r.varu();
    for _ in 0..nm { r.loc(); r.loc(); }
    let data_loc = r.opt_loc();
    let mut errors = vec![];
    for _ in 0..r.varu() {
        r.varu(); let msg = String::from_utf8_lossy(&r.string()).into_owned(); let loc = r.loc(); let level = r.byte();
        errors.push(Diag { msg, loc, level });
    }
    let mut warnings = vec![];
    for _ in 0..r.varu() {
        r.varu(); let msg = String::from_utf8_lossy(&r.string()).into_owned(); let loc = r.loc(); let level = r.byte();
        warnings.push(Diag { msg, loc, level });
    }
    let _continuable = r.byte();
    let pool_off = r.u32le() as usize;
    let pool_n = r.varu() as usize;
    Prefix { encoding, start_line, line_offsets, data_loc, errors, warnings, pool_off, pool_n }
}
impl Prefix {
    /// The constant pool (the names of Cst).
    pub fn constants(&self, bytes: &[u8]) -> Vec<Vec<u8>> {
        let mut constants = Vec::with_capacity(self.pool_n);
        for i in 0..self.pool_n {
            let p = self.pool_off + i * 8;
            let st = u32::from_le_bytes(bytes[p..p + 4].try_into().unwrap()) as usize;
            let ln = u32::from_le_bytes(bytes[p + 4..p + 8].try_into().unwrap()) as usize;
            constants.push(bytes[st..st + ln].to_vec());
        }
        constants
    }
}

/// The result of a parse from its serialization (pm_serialize_parse's output).
pub fn read_serialized(bytes: &[u8]) -> ParseResult {
    let mut r = Reader { buf: bytes, pos: 0 };
    let p = read_prefix(&mut r);
    let root = r.node();
    let constants = p.constants(bytes);
    let Prefix { encoding, start_line, line_offsets, data_loc, errors, warnings, .. } = p;
    ParseResult { root, constants, line_offsets, start_line, errors, warnings, data_loc, encoding }
}

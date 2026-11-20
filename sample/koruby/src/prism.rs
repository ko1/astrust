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

/// Options for the parse: file path, starting line, and outer local scopes (for eval).
pub struct ParseOpts<'a> { pub filepath: &'a [u8], pub line: i32, pub scopes: Vec<Vec<Vec<u8>>>, pub main_script: bool, pub frozen_string_literal: i8 }

fn build_options(o: &ParseOpts) -> Vec<u8> {
    let mut d = vec![];
    d.extend((o.filepath.len() as u32).to_le_bytes()); d.extend(o.filepath);
    d.extend(o.line.to_le_bytes());
    d.extend(0u32.to_le_bytes());          // encoding
    d.push(o.frozen_string_literal as u8); // frozen_string_literal
    d.push(0);                             // command_line
    d.push(3);                             // version: CRuby 4.0 (the oracle; "latest" is 4.1 syntax)
    d.push(0);                             // encoding_locked
    d.push(o.main_script as u8);
    d.push(0);                             // partial_script
    d.push(0);                             // freeze
    d.push(0);                             // attach_comments
    d.extend((o.scopes.len() as u32).to_le_bytes());
    for sc in &o.scopes {
        d.extend((sc.len() as u32).to_le_bytes());
        d.push(0);
        for l in sc { d.extend((l.len() as u32).to_le_bytes()); d.extend(l); }
    }
    d
}

pub fn parse(src: &[u8], o: &ParseOpts) -> ParseResult {
    let opts = build_options(o);
    let bytes = unsafe {
        let b = pm_buffer_new();
        pm_serialize_parse(b, src.as_ptr(), src.len(), opts.as_ptr() as *const c_char);
        let v = std::slice::from_raw_parts(pm_buffer_value(b) as *const u8, pm_buffer_length(b)).to_vec();
        pm_buffer_free(b);
        v
    };
    let mut r = Reader { buf: &bytes, pos: 0 };
    assert!(&bytes[0..5] == b"PRISM", "prism: bad serialization");
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
    let root = r.node();
    let mut constants = Vec::with_capacity(pool_n);
    for i in 0..pool_n {
        let p = pool_off + i * 8;
        let st = u32::from_le_bytes(bytes[p..p + 4].try_into().unwrap()) as usize;
        let ln = u32::from_le_bytes(bytes[p + 4..p + 8].try_into().unwrap()) as usize;
        constants.push(bytes[st..st + ln].to_vec());
    }
    ParseResult { root, constants, line_offsets, start_line, errors, warnings, data_loc, encoding }
}

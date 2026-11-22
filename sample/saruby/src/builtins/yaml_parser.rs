// YAML reader, scanner and parser: a port of libyaml 0.2.5 (src/reader.c, scanner.c, parser.c).
//
// libyaml is
//   Copyright (c) 2017-2020 Ingy döt Net
//   Copyright (c) 2006-2016 Kirill Simonov
// and is distributed under the MIT License:
//
// Permission is hereby granted, free of charge, to any person obtaining a copy of
// this software and associated documentation files (the "Software"), to deal in
// the Software without restriction, including without limitation the rights to
// use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies
// of the Software, and to permit persons to whom the Software is furnished to do
// so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.
#![forbid(unsafe_code)]
use std::collections::VecDeque;

pub const ANY_ENCODING: i32 = 0;
pub const UTF8_ENCODING: i32 = 1;
pub const UTF16LE_ENCODING: i32 = 2;
pub const UTF16BE_ENCODING: i32 = 3;

pub const PLAIN: i32 = 1;
pub const SINGLE_QUOTED: i32 = 2;
pub const DOUBLE_QUOTED: i32 = 3;
pub const LITERAL: i32 = 4;
pub const FOLDED: i32 = 5;
pub const BLOCK_STYLE: i32 = 1;
pub const FLOW_STYLE: i32 = 2;

const RAW_SIZE: usize = 16384;

#[derive(Clone, Copy, Default, Debug)]
pub struct Mark { pub index: usize, pub line: usize, pub column: usize }

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ErrorKind { Reader, Scanner, Parser }

#[derive(Clone, Debug)]
pub struct YError {
    pub kind: ErrorKind,
    pub problem: &'static str,
    pub problem_offset: usize,
    pub problem_value: i64,
    pub problem_mark: Mark,
    pub context: Option<&'static str>,
    pub context_mark: Mark,
}

pub enum EventData {
    StreamStart { encoding: i32 },
    StreamEnd,
    DocumentStart { version: Option<(i32, i32)>, tags: Vec<(Vec<u8>, Vec<u8>)>, implicit: bool },
    DocumentEnd { implicit: bool },
    Alias { anchor: Vec<u8> },
    Scalar { anchor: Option<Vec<u8>>, tag: Option<Vec<u8>>, value: Vec<u8>, plain_implicit: bool, quoted_implicit: bool, style: i32 },
    SequenceStart { anchor: Option<Vec<u8>>, tag: Option<Vec<u8>>, implicit: bool, style: i32 },
    SequenceEnd,
    MappingStart { anchor: Option<Vec<u8>>, tag: Option<Vec<u8>>, implicit: bool, style: i32 },
    MappingEnd,
}
pub struct Event { pub data: EventData, pub start: Mark, pub end: Mark }

/// Where the raw bytes come from; `read` returns false on an input error.
pub trait Source { fn read(&mut self, max: usize, out: &mut Vec<u8>) -> bool; }

#[derive(PartialEq)]
enum Tk {
    StreamStart(i32), StreamEnd, VersionDirective(i32, i32), TagDirective(Vec<u8>, Vec<u8>), DocumentStart, DocumentEnd,
    BlockSequenceStart, BlockMappingStart, BlockEnd, FlowSequenceStart, FlowSequenceEnd, FlowMappingStart, FlowMappingEnd,
    BlockEntry, FlowEntry, Key, Value, Alias(Vec<u8>), Anchor(Vec<u8>), Tag(Vec<u8>, Vec<u8>), Scalar(Vec<u8>, i32),
}
struct Token { kind: Tk, start: Mark, end: Mark }
impl Token { fn new(kind: Tk, start: Mark, end: Mark) -> Token { Token { kind, start, end } } }

#[derive(Clone, Copy, Default)]
struct SimpleKey { possible: bool, required: bool, token_number: usize, mark: Mark }

#[derive(Clone, Copy, PartialEq, Eq)]
enum St {
    StreamStart, ImplicitDocumentStart, DocumentStart, DocumentContent, DocumentEnd, BlockNode, BlockNodeOrIndentlessSequence, FlowNode,
    BlockSequenceFirstEntry, BlockSequenceEntry, IndentlessSequenceEntry, BlockMappingFirstKey, BlockMappingKey, BlockMappingValue,
    FlowSequenceFirstEntry, FlowSequenceEntry, FlowSequenceEntryMappingKey, FlowSequenceEntryMappingValue, FlowSequenceEntryMappingEnd,
    FlowMappingFirstKey, FlowMappingKey, FlowMappingValue, FlowMappingEmptyValue, End,
}

type R<T> = Result<T, ()>;

pub struct Parser<S: Source> {
    pub src: S,
    pub error: Option<YError>,
    raw: Vec<u8>, raw_pos: usize, eof: bool, offset: usize, encoding: i32,
    buf: Vec<u8>, pos: usize, unread: usize,
    pub mark: Mark,
    stream_start_produced: bool, stream_end_produced: bool, flow_level: i32,
    tokens: VecDeque<Token>, tokens_parsed: usize, token_available: bool,
    indents: Vec<i64>, indent: i64, simple_key_allowed: bool, simple_keys: Vec<SimpleKey>,
    states: Vec<St>, state: St, marks: Vec<Mark>, tag_directives: Vec<(Vec<u8>, Vec<u8>)>,
}

fn width(b: u8) -> usize { if b & 0x80 == 0 { 1 } else if b & 0xE0 == 0xC0 { 2 } else if b & 0xF0 == 0xE0 { 3 } else if b & 0xF8 == 0xF0 { 4 } else { 0 } }
fn is_alpha(b: u8) -> bool { b.is_ascii_alphanumeric() || b == b'_' || b == b'-' }
fn as_hex(b: u8) -> u32 { (b as char).to_digit(16).unwrap_or(0) }
fn push_utf8(s: &mut Vec<u8>, v: u32) {
    if v <= 0x7F { s.push(v as u8); }
    else if v <= 0x7FF { s.push(0xC0 + (v >> 6) as u8); s.push(0x80 + (v & 0x3F) as u8); }
    else if v <= 0xFFFF { s.push(0xE0 + (v >> 12) as u8); s.push(0x80 + ((v >> 6) & 0x3F) as u8); s.push(0x80 + (v & 0x3F) as u8); }
    else { s.push(0xF0 + (v >> 18) as u8); s.push(0x80 + ((v >> 12) & 0x3F) as u8); s.push(0x80 + ((v >> 6) & 0x3F) as u8); s.push(0x80 + (v & 0x3F) as u8); }
}

impl<S: Source> Parser<S> {
    pub fn new(src: S, encoding: i32) -> Parser<S> {
        Parser { src, error: None, raw: Vec::new(), raw_pos: 0, eof: false, offset: 0, encoding, buf: Vec::new(), pos: 0, unread: 0,
            mark: Mark::default(), stream_start_produced: false, stream_end_produced: false, flow_level: 0, tokens: VecDeque::new(),
            tokens_parsed: 0, token_available: false, indents: Vec::new(), indent: 0, simple_key_allowed: false, simple_keys: Vec::new(),
            states: Vec::new(), state: St::StreamStart, marks: Vec::new(), tag_directives: Vec::new() }
    }

    // ---- reader ----

    fn reader_error(&mut self, problem: &'static str, offset: usize, value: i64) -> R<()> {
        self.error = Some(YError { kind: ErrorKind::Reader, problem, problem_offset: offset, problem_value: value, problem_mark: Mark::default(), context: None, context_mark: Mark::default() });
        Err(())
    }
    fn determine_encoding(&mut self) -> R<()> {
        while !self.eof && self.raw.len() - self.raw_pos < 3 { self.update_raw_buffer()?; }
        let r = &self.raw[self.raw_pos..];
        if r.len() >= 2 && r[..2] == [0xFF, 0xFE] { self.encoding = UTF16LE_ENCODING; self.raw_pos += 2; self.offset += 2; }
        else if r.len() >= 2 && r[..2] == [0xFE, 0xFF] { self.encoding = UTF16BE_ENCODING; self.raw_pos += 2; self.offset += 2; }
        else if r.len() >= 3 && r[..3] == [0xEF, 0xBB, 0xBF] { self.encoding = UTF8_ENCODING; self.raw_pos += 3; self.offset += 3; }
        else { self.encoding = UTF8_ENCODING; }
        Ok(())
    }
    fn update_raw_buffer(&mut self) -> R<()> {
        if self.raw_pos == 0 && self.raw.len() == RAW_SIZE { return Ok(()); }
        if self.eof { return Ok(()); }
        if self.raw_pos > 0 { self.raw.drain(..self.raw_pos); self.raw_pos = 0; }
        let before = self.raw.len();
        let want = RAW_SIZE - before;
        if !self.src.read(want, &mut self.raw) { let o = self.offset; return self.reader_error("input error", o, -1); }
        if self.raw.len() == before { self.eof = true; }
        Ok(())
    }
    fn update_buffer(&mut self, length: usize) -> R<()> {
        if self.eof && self.raw_pos == self.raw.len() { return Ok(()); }
        if self.unread >= length { return Ok(()); }
        if self.encoding == ANY_ENCODING { self.determine_encoding()?; }
        if self.pos > 0 { self.buf.drain(..self.pos); self.pos = 0; }
        let mut first = true;
        while self.unread < length {
            if !first || self.raw_pos == self.raw.len() { self.update_raw_buffer()?; }
            first = false;
            while self.raw_pos != self.raw.len() {
                let r = &self.raw[self.raw_pos..];
                let raw_unread = r.len();
                let value: u32;
                let w: usize;
                if self.encoding == UTF8_ENCODING {
                    let octet = r[0];
                    let wd = width(octet);
                    if wd == 0 { let o = self.offset; return self.reader_error("invalid leading UTF-8 octet", o, octet as i64); }
                    if wd > raw_unread {
                        if self.eof { let o = self.offset; return self.reader_error("incomplete UTF-8 octet sequence", o, -1); }
                        break;
                    }
                    let mut v: u32 = match wd { 1 => octet & 0x7F, 2 => octet & 0x1F, 3 => octet & 0x0F, _ => octet & 0x07 } as u32;
                    for k in 1..wd {
                        let o = r[k];
                        if o & 0xC0 != 0x80 { let off = self.offset + k; return self.reader_error("invalid trailing UTF-8 octet", off, o as i64); }
                        v = (v << 6) + (o & 0x3F) as u32;
                    }
                    if !(wd == 1 || (wd == 2 && v >= 0x80) || (wd == 3 && v >= 0x800) || (wd == 4 && v >= 0x10000)) {
                        let o = self.offset; return self.reader_error("invalid length of a UTF-8 sequence", o, -1);
                    }
                    if (0xD800..=0xDFFF).contains(&v) || v > 0x10FFFF { let o = self.offset; return self.reader_error("invalid Unicode character", o, v as i64); }
                    value = v; w = wd;
                } else {
                    let (low, high) = if self.encoding == UTF16LE_ENCODING { (0, 1) } else { (1, 0) };
                    if raw_unread < 2 {
                        if self.eof { let o = self.offset; return self.reader_error("incomplete UTF-16 character", o, -1); }
                        break;
                    }
                    let mut v = r[low] as u32 + ((r[high] as u32) << 8);
                    if v & 0xFC00 == 0xDC00 { let o = self.offset; return self.reader_error("unexpected low surrogate area", o, v as i64); }
                    if v & 0xFC00 == 0xD800 {
                        if raw_unread < 4 {
                            if self.eof { let o = self.offset; return self.reader_error("incomplete UTF-16 surrogate pair", o, -1); }
                            break;
                        }
                        let v2 = r[low + 2] as u32 + ((r[high + 2] as u32) << 8);
                        if v2 & 0xFC00 != 0xDC00 { let o = self.offset + 2; return self.reader_error("expected low surrogate area", o, v2 as i64); }
                        v = 0x10000 + ((v & 0x3FF) << 10) + (v2 & 0x3FF);
                        w = 4;
                    } else { w = 2; }
                    value = v;
                }
                if !(value == 0x09 || value == 0x0A || value == 0x0D || (0x20..=0x7E).contains(&value) || value == 0x85
                    || (0xA0..=0xD7FF).contains(&value) || (0xE000..=0xFFFD).contains(&value) || (0x10000..=0x10FFFF).contains(&value)) {
                    let o = self.offset; return self.reader_error("control characters are not allowed", o, value as i64);
                }
                self.raw_pos += w;
                self.offset += w;
                push_utf8(&mut self.buf, value);
                self.unread += 1;
            }
            if self.eof { self.buf.push(0); self.unread += 1; return Ok(()); }
        }
        Ok(())
    }

    // ---- buffer access ----

    #[inline] fn at(&self, k: usize) -> u8 { self.buf.get(self.pos + k).copied().unwrap_or(0) }
    #[inline] fn check(&self, c: u8) -> bool { self.at(0) == c }
    #[inline] fn cache(&mut self, n: usize) -> R<()> { if self.unread >= n { Ok(()) } else { self.update_buffer(n) } }
    fn is_break_at(&self, k: usize) -> bool {
        let b = self.at(k);
        b == b'\r' || b == b'\n' || (b == 0xC2 && self.at(k + 1) == 0x85) || (b == 0xE2 && self.at(k + 1) == 0x80 && (self.at(k + 2) == 0xA8 || self.at(k + 2) == 0xA9))
    }
    fn is_blank_at(&self, k: usize) -> bool { let b = self.at(k); b == b' ' || b == b'\t' }
    fn is_breakz_at(&self, k: usize) -> bool { self.is_break_at(k) || self.at(k) == 0 }
    fn is_blankz_at(&self, k: usize) -> bool { self.is_blank_at(k) || self.is_breakz_at(k) }
    fn is_break(&self) -> bool { self.is_break_at(0) }
    fn is_blank(&self) -> bool { self.is_blank_at(0) }
    fn is_breakz(&self) -> bool { self.is_breakz_at(0) }
    fn is_blankz(&self) -> bool { self.is_blankz_at(0) }
    fn skip(&mut self) {
        self.mark.index += 1; self.mark.column += 1; self.unread -= 1;
        self.pos += width(self.at(0));
    }
    fn skip_line(&mut self) {
        if self.at(0) == b'\r' && self.at(1) == b'\n' {
            self.mark.index += 2; self.mark.column = 0; self.mark.line += 1; self.unread -= 2; self.pos += 2;
        } else if self.is_break() {
            self.mark.index += 1; self.mark.column = 0; self.mark.line += 1; self.unread -= 1; self.pos += width(self.at(0));
        }
    }
    fn read(&mut self, s: &mut Vec<u8>) {
        let w = width(self.at(0));
        let end = (self.pos + w).min(self.buf.len());
        s.extend_from_slice(&self.buf[self.pos..end]);
        self.pos += w;
        self.mark.index += 1; self.mark.column += 1; self.unread -= 1;
    }
    fn read_line(&mut self, s: &mut Vec<u8>) {
        let (a, b, c) = (self.at(0), self.at(1), self.at(2));
        if a == b'\r' && b == b'\n' {
            s.push(b'\n'); self.pos += 2; self.mark.index += 2; self.mark.column = 0; self.mark.line += 1; self.unread -= 2;
        } else if a == b'\r' || a == b'\n' {
            s.push(b'\n'); self.pos += 1; self.mark.index += 1; self.mark.column = 0; self.mark.line += 1; self.unread -= 1;
        } else if a == 0xC2 && b == 0x85 {
            s.push(b'\n'); self.pos += 2; self.mark.index += 1; self.mark.column = 0; self.mark.line += 1; self.unread -= 1;
        } else if a == 0xE2 && b == 0x80 && (c == 0xA8 || c == 0xA9) {
            s.extend_from_slice(&[a, b, c]); self.pos += 3; self.mark.index += 1; self.mark.column = 0; self.mark.line += 1; self.unread -= 1;
        }
    }

    // ---- scanner ----

    fn scanner_error<T>(&mut self, context: Option<&'static str>, context_mark: Mark, problem: &'static str) -> R<T> {
        self.error = Some(YError { kind: ErrorKind::Scanner, problem, problem_offset: 0, problem_value: 0, problem_mark: self.mark, context, context_mark });
        Err(())
    }
    fn fetch_more_tokens(&mut self) -> R<()> {
        loop {
            let mut need = false;
            if self.tokens.is_empty() { need = true; } else {
                self.stale_simple_keys()?;
                let tp = self.tokens_parsed;
                if self.simple_keys.iter().any(|k| k.possible && k.token_number == tp) { need = true; }
            }
            if !need { break; }
            self.fetch_next_token()?;
        }
        self.token_available = true;
        Ok(())
    }
    fn fetch_next_token(&mut self) -> R<()> {
        self.cache(1)?;
        if !self.stream_start_produced { self.fetch_stream_start(); return Ok(()); }
        self.scan_to_next_token()?;
        self.stale_simple_keys()?;
        let col = self.mark.column as i64;
        self.unroll_indent(col);
        self.cache(4)?;
        let c = self.at(0);
        if c == 0 { return self.fetch_stream_end(); }
        if self.mark.column == 0 && c == b'%' { return self.fetch_directive(); }
        if self.mark.column == 0 && c == b'-' && self.at(1) == b'-' && self.at(2) == b'-' && self.is_blankz_at(3) { return self.fetch_document_indicator(Tk::DocumentStart); }
        if self.mark.column == 0 && c == b'.' && self.at(1) == b'.' && self.at(2) == b'.' && self.is_blankz_at(3) { return self.fetch_document_indicator(Tk::DocumentEnd); }
        match c {
            b'[' => return self.fetch_flow_collection_start(Tk::FlowSequenceStart),
            b'{' => return self.fetch_flow_collection_start(Tk::FlowMappingStart),
            b']' => return self.fetch_flow_collection_end(Tk::FlowSequenceEnd),
            b'}' => return self.fetch_flow_collection_end(Tk::FlowMappingEnd),
            b',' => return self.fetch_flow_entry(),
            _ => {}
        }
        if c == b'-' && self.is_blankz_at(1) { return self.fetch_block_entry(); }
        if c == b'?' && (self.flow_level > 0 || self.is_blankz_at(1)) { return self.fetch_key(); }
        if c == b':' && (self.flow_level > 0 || self.is_blankz_at(1)) { return self.fetch_value(); }
        if c == b'*' { return self.fetch_anchor(true); }
        if c == b'&' { return self.fetch_anchor(false); }
        if c == b'!' { return self.fetch_tag(); }
        if c == b'|' && self.flow_level == 0 { return self.fetch_block_scalar(true); }
        if c == b'>' && self.flow_level == 0 { return self.fetch_block_scalar(false); }
        if c == b'\'' { return self.fetch_flow_scalar(true); }
        if c == b'"' { return self.fetch_flow_scalar(false); }
        if !(self.is_blankz() || b"-?:,[]{}#&*!|>'\"%@`".contains(&c))
            || (c == b'-' && !self.is_blank_at(1))
            || (self.flow_level == 0 && (c == b'?' || c == b':') && !self.is_blankz_at(1)) {
            return self.fetch_plain_scalar();
        }
        let m = self.mark;
        self.scanner_error(Some("while scanning for the next token"), m, "found character that cannot start any token")
    }
    fn stale_simple_keys(&mut self) -> R<()> {
        let mark = self.mark;
        for i in 0..self.simple_keys.len() {
            let k = self.simple_keys[i];
            if k.possible && (k.mark.line < mark.line || k.mark.index + 1024 < mark.index) {
                if k.required { return self.scanner_error(Some("while scanning a simple key"), k.mark, "could not find expected ':'"); }
                self.simple_keys[i].possible = false;
            }
        }
        Ok(())
    }
    fn save_simple_key(&mut self) -> R<()> {
        let required = self.flow_level == 0 && self.indent == self.mark.column as i64;
        if self.simple_key_allowed {
            let sk = SimpleKey { possible: true, required, token_number: self.tokens_parsed + self.tokens.len(), mark: self.mark };
            self.remove_simple_key()?;
            *self.simple_keys.last_mut().unwrap() = sk;
        }
        Ok(())
    }
    fn remove_simple_key(&mut self) -> R<()> {
        let k = *self.simple_keys.last().unwrap();
        if k.possible && k.required { return self.scanner_error(Some("while scanning a simple key"), k.mark, "could not find expected ':'"); }
        self.simple_keys.last_mut().unwrap().possible = false;
        Ok(())
    }
    fn increase_flow_level(&mut self) { self.simple_keys.push(SimpleKey::default()); self.flow_level += 1; }
    fn decrease_flow_level(&mut self) { if self.flow_level > 0 { self.flow_level -= 1; self.simple_keys.pop(); } }
    fn roll_indent(&mut self, column: i64, number: Option<usize>, kind: Tk, mark: Mark) {
        if self.flow_level > 0 { return; }
        if self.indent < column {
            self.indents.push(self.indent);
            self.indent = column;
            let t = Token::new(kind, mark, mark);
            match number { None => self.tokens.push_back(t), Some(n) => self.tokens.insert(n - self.tokens_parsed, t) }
        }
    }
    fn unroll_indent(&mut self, column: i64) {
        if self.flow_level > 0 { return; }
        while self.indent > column {
            self.tokens.push_back(Token::new(Tk::BlockEnd, self.mark, self.mark));
            self.indent = self.indents.pop().unwrap_or(-1);
        }
    }
    fn fetch_stream_start(&mut self) {
        self.indent = -1;
        self.simple_keys.push(SimpleKey::default());
        self.simple_key_allowed = true;
        self.stream_start_produced = true;
        self.tokens.push_back(Token::new(Tk::StreamStart(self.encoding), self.mark, self.mark));
    }
    fn fetch_stream_end(&mut self) -> R<()> {
        if self.mark.column != 0 { self.mark.column = 0; self.mark.line += 1; }
        self.unroll_indent(-1);
        self.remove_simple_key()?;
        self.simple_key_allowed = false;
        self.tokens.push_back(Token::new(Tk::StreamEnd, self.mark, self.mark));
        Ok(())
    }
    fn fetch_directive(&mut self) -> R<()> {
        self.unroll_indent(-1);
        self.remove_simple_key()?;
        self.simple_key_allowed = false;
        let t = self.scan_directive()?;
        self.tokens.push_back(t);
        Ok(())
    }
    fn fetch_document_indicator(&mut self, kind: Tk) -> R<()> {
        self.unroll_indent(-1);
        self.remove_simple_key()?;
        self.simple_key_allowed = false;
        let s = self.mark;
        self.skip(); self.skip(); self.skip();
        self.tokens.push_back(Token::new(kind, s, self.mark));
        Ok(())
    }
    fn fetch_flow_collection_start(&mut self, kind: Tk) -> R<()> {
        self.save_simple_key()?;
        self.increase_flow_level();
        self.simple_key_allowed = true;
        let s = self.mark; self.skip();
        self.tokens.push_back(Token::new(kind, s, self.mark));
        Ok(())
    }
    fn fetch_flow_collection_end(&mut self, kind: Tk) -> R<()> {
        self.remove_simple_key()?;
        self.decrease_flow_level();
        self.simple_key_allowed = false;
        let s = self.mark; self.skip();
        self.tokens.push_back(Token::new(kind, s, self.mark));
        Ok(())
    }
    fn fetch_flow_entry(&mut self) -> R<()> {
        self.remove_simple_key()?;
        self.simple_key_allowed = true;
        let s = self.mark; self.skip();
        self.tokens.push_back(Token::new(Tk::FlowEntry, s, self.mark));
        Ok(())
    }
    fn fetch_block_entry(&mut self) -> R<()> {
        if self.flow_level == 0 {
            if !self.simple_key_allowed { let m = self.mark; return self.scanner_error(None, m, "block sequence entries are not allowed in this context"); }
            let (col, m) = (self.mark.column as i64, self.mark);
            self.roll_indent(col, None, Tk::BlockSequenceStart, m);
        }
        self.remove_simple_key()?;
        self.simple_key_allowed = true;
        let s = self.mark; self.skip();
        self.tokens.push_back(Token::new(Tk::BlockEntry, s, self.mark));
        Ok(())
    }
    fn fetch_key(&mut self) -> R<()> {
        if self.flow_level == 0 {
            if !self.simple_key_allowed { let m = self.mark; return self.scanner_error(None, m, "mapping keys are not allowed in this context"); }
            let (col, m) = (self.mark.column as i64, self.mark);
            self.roll_indent(col, None, Tk::BlockMappingStart, m);
        }
        self.remove_simple_key()?;
        self.simple_key_allowed = self.flow_level == 0;
        let s = self.mark; self.skip();
        self.tokens.push_back(Token::new(Tk::Key, s, self.mark));
        Ok(())
    }
    fn fetch_value(&mut self) -> R<()> {
        let sk = *self.simple_keys.last().unwrap();
        if sk.possible {
            self.tokens.insert(sk.token_number - self.tokens_parsed, Token::new(Tk::Key, sk.mark, sk.mark));
            self.roll_indent(sk.mark.column as i64, Some(sk.token_number), Tk::BlockMappingStart, sk.mark);
            self.simple_keys.last_mut().unwrap().possible = false;
            self.simple_key_allowed = false;
        } else {
            if self.flow_level == 0 {
                if !self.simple_key_allowed { let m = self.mark; return self.scanner_error(None, m, "mapping values are not allowed in this context"); }
                let (col, m) = (self.mark.column as i64, self.mark);
                self.roll_indent(col, None, Tk::BlockMappingStart, m);
            }
            self.simple_key_allowed = self.flow_level == 0;
        }
        let s = self.mark; self.skip();
        self.tokens.push_back(Token::new(Tk::Value, s, self.mark));
        Ok(())
    }
    fn fetch_anchor(&mut self, alias: bool) -> R<()> {
        self.save_simple_key()?;
        self.simple_key_allowed = false;
        let t = self.scan_anchor(alias)?;
        self.tokens.push_back(t);
        Ok(())
    }
    fn fetch_tag(&mut self) -> R<()> {
        self.save_simple_key()?;
        self.simple_key_allowed = false;
        let t = self.scan_tag()?;
        self.tokens.push_back(t);
        Ok(())
    }
    fn fetch_block_scalar(&mut self, literal: bool) -> R<()> {
        self.remove_simple_key()?;
        self.simple_key_allowed = true;
        let t = self.scan_block_scalar(literal)?;
        self.tokens.push_back(t);
        Ok(())
    }
    fn fetch_flow_scalar(&mut self, single: bool) -> R<()> {
        self.save_simple_key()?;
        self.simple_key_allowed = false;
        let t = self.scan_flow_scalar(single)?;
        self.tokens.push_back(t);
        Ok(())
    }
    fn fetch_plain_scalar(&mut self) -> R<()> {
        self.save_simple_key()?;
        self.simple_key_allowed = false;
        let t = self.scan_plain_scalar()?;
        self.tokens.push_back(t);
        Ok(())
    }
    fn scan_to_next_token(&mut self) -> R<()> {
        loop {
            self.cache(1)?;
            if self.mark.column == 0 && self.at(0) == 0xEF && self.at(1) == 0xBB && self.at(2) == 0xBF { self.skip(); }
            self.cache(1)?;
            while self.check(b' ') || ((self.flow_level > 0 || !self.simple_key_allowed) && self.check(b'\t')) { self.skip(); self.cache(1)?; }
            if self.check(b'#') { while !self.is_breakz() { self.skip(); self.cache(1)?; } }
            if self.is_break() {
                self.cache(2)?;
                self.skip_line();
                if self.flow_level == 0 { self.simple_key_allowed = true; }
            } else { break; }
        }
        Ok(())
    }
    fn skip_blanks(&mut self) -> R<()> { self.cache(1)?; while self.is_blank() { self.skip(); self.cache(1)?; } Ok(()) }
    fn scan_directive(&mut self) -> R<Token> {
        let start = self.mark;
        self.skip();
        let name = self.scan_directive_name(start)?;
        let kind = if name == b"YAML" {
            let (major, minor) = self.scan_version_directive_value(start)?;
            Tk::VersionDirective(major, minor)
        } else if name == b"TAG" {
            let (h, p) = self.scan_tag_directive_value(start)?;
            Tk::TagDirective(h, p)
        } else {
            return self.scanner_error(Some("while scanning a directive"), start, "found unknown directive name");
        };
        let end = self.mark;
        self.skip_blanks()?;
        if self.check(b'#') { while !self.is_breakz() { self.skip(); self.cache(1)?; } }
        if !self.is_breakz() { return self.scanner_error(Some("while scanning a directive"), start, "did not find expected comment or line break"); }
        if self.is_break() { self.cache(2)?; self.skip_line(); }
        Ok(Token::new(kind, start, end))
    }
    fn scan_directive_name(&mut self, start: Mark) -> R<Vec<u8>> {
        let mut s = Vec::new();
        self.cache(1)?;
        while is_alpha(self.at(0)) { self.read(&mut s); self.cache(1)?; }
        if s.is_empty() { return self.scanner_error(Some("while scanning a directive"), start, "could not find expected directive name"); }
        if !self.is_blankz() { return self.scanner_error(Some("while scanning a directive"), start, "found unexpected non-alphabetical character"); }
        Ok(s)
    }
    fn scan_version_directive_value(&mut self, start: Mark) -> R<(i32, i32)> {
        self.skip_blanks()?;
        let major = self.scan_version_directive_number(start)?;
        if !self.check(b'.') { return self.scanner_error(Some("while scanning a %YAML directive"), start, "did not find expected digit or '.' character"); }
        self.skip();
        let minor = self.scan_version_directive_number(start)?;
        Ok((major, minor))
    }
    fn scan_version_directive_number(&mut self, start: Mark) -> R<i32> {
        let mut value: i32 = 0;
        let mut length = 0;
        self.cache(1)?;
        while self.at(0).is_ascii_digit() {
            length += 1;
            if length > 9 { return self.scanner_error(Some("while scanning a %YAML directive"), start, "found extremely long version number"); }
            value = value * 10 + (self.at(0) - b'0') as i32;
            self.skip();
            self.cache(1)?;
        }
        if length == 0 { return self.scanner_error(Some("while scanning a %YAML directive"), start, "did not find expected version number"); }
        Ok(value)
    }
    fn scan_tag_directive_value(&mut self, start: Mark) -> R<(Vec<u8>, Vec<u8>)> {
        self.skip_blanks()?;
        let handle = self.scan_tag_handle(true, start)?;
        self.cache(1)?;
        if !self.is_blank() { return self.scanner_error(Some("while scanning a %TAG directive"), start, "did not find expected whitespace"); }
        while self.is_blank() { self.skip(); self.cache(1)?; }
        let prefix = self.scan_tag_uri(true, true, None, start)?;
        self.cache(1)?;
        if !self.is_blankz() { return self.scanner_error(Some("while scanning a %TAG directive"), start, "did not find expected whitespace or line break"); }
        Ok((handle, prefix))
    }
    fn scan_anchor(&mut self, alias: bool) -> R<Token> {
        let mut s = Vec::new();
        let start = self.mark;
        self.skip();
        self.cache(1)?;
        while is_alpha(self.at(0)) { self.read(&mut s); self.cache(1)?; }
        let end = self.mark;
        let c = self.at(0);
        if s.is_empty() || !(self.is_blankz() || b"?:,]}%@`".contains(&c)) {
            return self.scanner_error(Some(if alias { "while scanning an alias" } else { "while scanning an anchor" }), start, "did not find expected alphabetic or numeric character");
        }
        Ok(Token::new(if alias { Tk::Alias(s) } else { Tk::Anchor(s) }, start, end))
    }
    fn scan_tag(&mut self) -> R<Token> {
        let start = self.mark;
        self.cache(2)?;
        let (handle, suffix);
        if self.at(1) == b'<' {
            self.skip(); self.skip();
            let sfx = self.scan_tag_uri(true, false, None, start)?;
            if !self.check(b'>') { return self.scanner_error(Some("while scanning a tag"), start, "did not find the expected '>'"); }
            self.skip();
            handle = Vec::new(); suffix = sfx;
        } else {
            let h = self.scan_tag_handle(false, start)?;
            if h.len() > 1 && h[0] == b'!' && h[h.len() - 1] == b'!' {
                suffix = self.scan_tag_uri(false, false, None, start)?;
                handle = h;
            } else {
                let sfx = self.scan_tag_uri(false, false, Some(&h), start)?;
                if sfx.is_empty() { handle = sfx; suffix = b"!".to_vec(); } else { handle = b"!".to_vec(); suffix = sfx; }
            }
        }
        self.cache(1)?;
        if !self.is_blankz() && (self.flow_level == 0 || !self.check(b',')) {
            return self.scanner_error(Some("while scanning a tag"), start, "did not find expected whitespace or line break");
        }
        Ok(Token::new(Tk::Tag(handle, suffix), start, self.mark))
    }
    fn scan_tag_handle(&mut self, directive: bool, start: Mark) -> R<Vec<u8>> {
        let mut s = Vec::new();
        self.cache(1)?;
        if !self.check(b'!') { return self.scanner_error(Some(if directive { "while scanning a tag directive" } else { "while scanning a tag" }), start, "did not find expected '!'"); }
        self.read(&mut s);
        self.cache(1)?;
        while is_alpha(self.at(0)) { self.read(&mut s); self.cache(1)?; }
        if self.check(b'!') { self.read(&mut s); }
        else if directive && s != b"!" { return self.scanner_error(Some("while parsing a tag directive"), start, "did not find expected '!'"); }
        Ok(s)
    }
    fn scan_tag_uri(&mut self, uri_char: bool, directive: bool, head: Option<&[u8]>, start: Mark) -> R<Vec<u8>> {
        let mut length = head.map(|h| h.len()).unwrap_or(0);
        let mut s = Vec::new();
        if length > 1 { s.extend_from_slice(&head.unwrap()[1..]); }
        self.cache(1)?;
        loop {
            let c = self.at(0);
            if !(is_alpha(c) || b";/?:@&=+$.%!~*'()".contains(&c) || (uri_char && (c == b',' || c == b'[' || c == b']'))) { break; }
            if c == b'%' { self.scan_uri_escapes(directive, start, &mut s)?; } else { self.read(&mut s); }
            length += 1;
            self.cache(1)?;
        }
        if length == 0 { return self.scanner_error(Some(if directive { "while parsing a %TAG directive" } else { "while parsing a tag" }), start, "did not find expected tag URI"); }
        Ok(s)
    }
    fn scan_uri_escapes(&mut self, directive: bool, start: Mark, s: &mut Vec<u8>) -> R<()> {
        let ctx = if directive { "while parsing a %TAG directive" } else { "while parsing a tag" };
        let mut w = 0;
        loop {
            self.cache(3)?;
            if !(self.check(b'%') && self.at(1).is_ascii_hexdigit() && self.at(2).is_ascii_hexdigit()) { return self.scanner_error(Some(ctx), start, "did not find URI escaped octet"); }
            let octet = ((as_hex(self.at(1)) << 4) + as_hex(self.at(2))) as u8;
            if w == 0 {
                w = width(octet);
                if w == 0 { return self.scanner_error(Some(ctx), start, "found an incorrect leading UTF-8 octet"); }
            } else if octet & 0xC0 != 0x80 { return self.scanner_error(Some(ctx), start, "found an incorrect trailing UTF-8 octet"); }
            s.push(octet);
            self.skip(); self.skip(); self.skip();
            w -= 1;
            if w == 0 { break; }
        }
        Ok(())
    }
    fn scan_block_scalar(&mut self, literal: bool) -> R<Token> {
        let mut s = Vec::new();
        let mut leading_break = Vec::new();
        let mut trailing_breaks = Vec::new();
        let mut chomping = 0;
        let mut increment: i64 = 0;
        let mut indent: i64 = 0;
        let mut leading_blank = false;
        let start = self.mark;
        self.skip();
        self.cache(1)?;
        if self.check(b'+') || self.check(b'-') {
            chomping = if self.check(b'+') { 1 } else { -1 };
            self.skip();
            self.cache(1)?;
            if self.at(0).is_ascii_digit() {
                if self.check(b'0') { return self.scanner_error(Some("while scanning a block scalar"), start, "found an indentation indicator equal to 0"); }
                increment = (self.at(0) - b'0') as i64;
                self.skip();
            }
        } else if self.at(0).is_ascii_digit() {
            if self.check(b'0') { return self.scanner_error(Some("while scanning a block scalar"), start, "found an indentation indicator equal to 0"); }
            increment = (self.at(0) - b'0') as i64;
            self.skip();
            self.cache(1)?;
            if self.check(b'+') || self.check(b'-') { chomping = if self.check(b'+') { 1 } else { -1 }; self.skip(); }
        }
        self.skip_blanks()?;
        if self.check(b'#') { while !self.is_breakz() { self.skip(); self.cache(1)?; } }
        if !self.is_breakz() { return self.scanner_error(Some("while scanning a block scalar"), start, "did not find expected comment or line break"); }
        if self.is_break() { self.cache(2)?; self.skip_line(); }
        let mut end = self.mark;
        if increment != 0 { indent = if self.indent >= 0 { self.indent + increment } else { increment }; }
        self.scan_block_scalar_breaks(&mut indent, &mut trailing_breaks, start, &mut end)?;
        self.cache(1)?;
        while self.mark.column as i64 == indent && self.at(0) != 0 {
            let trailing_blank = self.is_blank();
            if !literal && leading_break.first() == Some(&b'\n') && !leading_blank && !trailing_blank {
                if trailing_breaks.is_empty() { s.push(b' '); }
                leading_break.clear();
            } else {
                s.append(&mut leading_break);
            }
            s.append(&mut trailing_breaks);
            leading_blank = self.is_blank();
            while !self.is_breakz() { self.read(&mut s); self.cache(1)?; }
            self.cache(2)?;
            self.read_line(&mut leading_break);
            self.scan_block_scalar_breaks(&mut indent, &mut trailing_breaks, start, &mut end)?;
        }
        if chomping != -1 { s.append(&mut leading_break); }
        if chomping == 1 { s.append(&mut trailing_breaks); }
        Ok(Token::new(Tk::Scalar(s, if literal { LITERAL } else { FOLDED }), start, end))
    }
    fn scan_block_scalar_breaks(&mut self, indent: &mut i64, breaks: &mut Vec<u8>, start: Mark, end: &mut Mark) -> R<()> {
        let mut max_indent: i64 = 0;
        *end = self.mark;
        loop {
            self.cache(1)?;
            while (*indent == 0 || (self.mark.column as i64) < *indent) && self.check(b' ') { self.skip(); self.cache(1)?; }
            if self.mark.column as i64 > max_indent { max_indent = self.mark.column as i64; }
            if (*indent == 0 || (self.mark.column as i64) < *indent) && self.check(b'\t') {
                return self.scanner_error(Some("while scanning a block scalar"), start, "found a tab character where an indentation space is expected");
            }
            if !self.is_break() { break; }
            self.cache(2)?;
            self.read_line(breaks);
            *end = self.mark;
        }
        if *indent == 0 {
            *indent = max_indent;
            if *indent < self.indent + 1 { *indent = self.indent + 1; }
            if *indent < 1 { *indent = 1; }
        }
        Ok(())
    }
    fn at_document_indicator(&self) -> bool {
        self.mark.column == 0 && ((self.at(0) == b'-' && self.at(1) == b'-' && self.at(2) == b'-') || (self.at(0) == b'.' && self.at(1) == b'.' && self.at(2) == b'.')) && self.is_blankz_at(3)
    }
    fn scan_flow_scalar(&mut self, single: bool) -> R<Token> {
        let mut s = Vec::new();
        let mut leading_break = Vec::new();
        let mut trailing_breaks = Vec::new();
        let mut whitespaces = Vec::new();
        let start = self.mark;
        let q = if single { b'\'' } else { b'"' };
        self.skip();
        loop {
            self.cache(4)?;
            if self.at_document_indicator() { return self.scanner_error(Some("while scanning a quoted scalar"), start, "found unexpected document indicator"); }
            if self.at(0) == 0 { return self.scanner_error(Some("while scanning a quoted scalar"), start, "found unexpected end of stream"); }
            self.cache(2)?;
            let mut leading_blanks = false;
            while !self.is_blankz() {
                if single && self.at(0) == b'\'' && self.at(1) == b'\'' {
                    s.push(b'\''); self.skip(); self.skip();
                } else if self.check(q) {
                    break;
                } else if !single && self.check(b'\\') && self.is_break_at(1) {
                    self.cache(3)?;
                    self.skip(); self.skip_line();
                    leading_blanks = true;
                    break;
                } else if !single && self.check(b'\\') {
                    let mut code_length = 0;
                    match self.at(1) {
                        b'0' => s.push(0), b'a' => s.push(7), b'b' => s.push(8), b't' | b'\t' => s.push(9), b'n' => s.push(10),
                        b'v' => s.push(11), b'f' => s.push(12), b'r' => s.push(13), b'e' => s.push(0x1B), b' ' => s.push(b' '),
                        b'"' => s.push(b'"'), b'/' => s.push(b'/'), b'\\' => s.push(b'\\'),
                        b'N' => s.extend_from_slice(&[0xC2, 0x85]), b'_' => s.extend_from_slice(&[0xC2, 0xA0]),
                        b'L' => s.extend_from_slice(&[0xE2, 0x80, 0xA8]), b'P' => s.extend_from_slice(&[0xE2, 0x80, 0xA9]),
                        b'x' => code_length = 2, b'u' => code_length = 4, b'U' => code_length = 8,
                        _ => return self.scanner_error(Some("while parsing a quoted scalar"), start, "found unknown escape character"),
                    }
                    self.skip(); self.skip();
                    if code_length > 0 {
                        self.cache(code_length)?;
                        let mut value: u32 = 0;
                        for k in 0..code_length {
                            if !self.at(k).is_ascii_hexdigit() { return self.scanner_error(Some("while parsing a quoted scalar"), start, "did not find expected hexdecimal number"); }
                            value = (value << 4).wrapping_add(as_hex(self.at(k)));
                        }
                        if (0xD800..=0xDFFF).contains(&value) || value > 0x10FFFF { return self.scanner_error(Some("while parsing a quoted scalar"), start, "found invalid Unicode character escape code"); }
                        push_utf8(&mut s, value);
                        for _ in 0..code_length { self.skip(); }
                    }
                } else {
                    self.read(&mut s);
                }
                self.cache(2)?;
            }
            self.cache(1)?;
            if self.check(q) { break; }
            self.cache(1)?;
            while self.is_blank() || self.is_break() {
                if self.is_blank() {
                    if !leading_blanks { self.read(&mut whitespaces); } else { self.skip(); }
                } else {
                    self.cache(2)?;
                    if !leading_blanks { whitespaces.clear(); self.read_line(&mut leading_break); leading_blanks = true; }
                    else { self.read_line(&mut trailing_breaks); }
                }
                self.cache(1)?;
            }
            if leading_blanks {
                if leading_break.first() == Some(&b'\n') {
                    if trailing_breaks.is_empty() { s.push(b' '); } else { s.append(&mut trailing_breaks); }
                    leading_break.clear();
                } else {
                    s.append(&mut leading_break);
                    s.append(&mut trailing_breaks);
                }
            } else {
                s.append(&mut whitespaces);
            }
        }
        self.skip();
        Ok(Token::new(Tk::Scalar(s, if single { SINGLE_QUOTED } else { DOUBLE_QUOTED }), start, self.mark))
    }
    fn scan_plain_scalar(&mut self) -> R<Token> {
        let mut s = Vec::new();
        let mut leading_break = Vec::new();
        let mut trailing_breaks = Vec::new();
        let mut whitespaces = Vec::new();
        let mut leading_blanks = false;
        let indent = self.indent + 1;
        let start = self.mark;
        let mut end = self.mark;
        loop {
            self.cache(4)?;
            if self.at_document_indicator() { break; }
            if self.check(b'#') { break; }
            while !self.is_blankz() {
                let c = self.at(0);
                if self.flow_level > 0 && c == b':' && b",?[]{}".contains(&self.at(1)) && self.at(1) != 0 {
                    return self.scanner_error(Some("while scanning a plain scalar"), start, "found unexpected ':'");
                }
                if (c == b':' && self.is_blankz_at(1)) || (self.flow_level > 0 && b",[]{}".contains(&c)) { break; }
                if leading_blanks || !whitespaces.is_empty() {
                    if leading_blanks {
                        if leading_break.first() == Some(&b'\n') {
                            if trailing_breaks.is_empty() { s.push(b' '); } else { s.append(&mut trailing_breaks); }
                            leading_break.clear();
                        } else {
                            s.append(&mut leading_break);
                            s.append(&mut trailing_breaks);
                        }
                        leading_blanks = false;
                    } else {
                        s.append(&mut whitespaces);
                    }
                }
                self.read(&mut s);
                end = self.mark;
                self.cache(2)?;
            }
            if !(self.is_blank() || self.is_break()) { break; }
            self.cache(1)?;
            while self.is_blank() || self.is_break() {
                if self.is_blank() {
                    if leading_blanks && (self.mark.column as i64) < indent && self.check(b'\t') {
                        return self.scanner_error(Some("while scanning a plain scalar"), start, "found a tab character that violates indentation");
                    }
                    if !leading_blanks { self.read(&mut whitespaces); } else { self.skip(); }
                } else {
                    self.cache(2)?;
                    if !leading_blanks { whitespaces.clear(); self.read_line(&mut leading_break); leading_blanks = true; }
                    else { self.read_line(&mut trailing_breaks); }
                }
                self.cache(1)?;
            }
            if self.flow_level == 0 && (self.mark.column as i64) < indent { break; }
        }
        if leading_blanks { self.simple_key_allowed = true; }
        Ok(Token::new(Tk::Scalar(s, PLAIN), start, end))
    }

    // ---- parser ----

    fn peek(&mut self) -> R<()> { if self.token_available { Ok(()) } else { self.fetch_more_tokens() } }
    fn tok(&self) -> &Token { &self.tokens[0] }
    fn skip_token(&mut self) -> Token {
        self.token_available = false;
        self.tokens_parsed += 1;
        let t = self.tokens.pop_front().unwrap();
        self.stream_end_produced = t.kind == Tk::StreamEnd;
        t
    }
    fn parser_error<T>(&mut self, problem: &'static str, problem_mark: Mark) -> R<T> {
        self.error = Some(YError { kind: ErrorKind::Parser, problem, problem_offset: 0, problem_value: 0, problem_mark, context: None, context_mark: Mark::default() });
        Err(())
    }
    fn parser_error_ctx<T>(&mut self, context: &'static str, context_mark: Mark, problem: &'static str, problem_mark: Mark) -> R<T> {
        self.error = Some(YError { kind: ErrorKind::Parser, problem, problem_offset: 0, problem_value: 0, problem_mark, context: Some(context), context_mark });
        Err(())
    }
    /// The next event; None once the stream has ended (the caller stops at StreamEnd).
    pub fn parse(&mut self) -> R<Option<Event>> {
        if self.stream_end_produced || self.error.is_some() || self.state == St::End { return Ok(None); }
        self.state_machine().map(Some)
    }
    fn pop_state(&mut self) -> St { self.states.pop().unwrap_or(St::End) }
    fn state_machine(&mut self) -> R<Event> {
        match self.state {
            St::StreamStart => self.parse_stream_start(),
            St::ImplicitDocumentStart => self.parse_document_start(true),
            St::DocumentStart => self.parse_document_start(false),
            St::DocumentContent => self.parse_document_content(),
            St::DocumentEnd => self.parse_document_end(),
            St::BlockNode => self.parse_node(true, false),
            St::BlockNodeOrIndentlessSequence => self.parse_node(true, true),
            St::FlowNode => self.parse_node(false, false),
            St::BlockSequenceFirstEntry => self.parse_block_sequence_entry(true),
            St::BlockSequenceEntry => self.parse_block_sequence_entry(false),
            St::IndentlessSequenceEntry => self.parse_indentless_sequence_entry(),
            St::BlockMappingFirstKey => self.parse_block_mapping_key(true),
            St::BlockMappingKey => self.parse_block_mapping_key(false),
            St::BlockMappingValue => self.parse_block_mapping_value(),
            St::FlowSequenceFirstEntry => self.parse_flow_sequence_entry(true),
            St::FlowSequenceEntry => self.parse_flow_sequence_entry(false),
            St::FlowSequenceEntryMappingKey => self.parse_flow_sequence_entry_mapping_key(),
            St::FlowSequenceEntryMappingValue => self.parse_flow_sequence_entry_mapping_value(),
            St::FlowSequenceEntryMappingEnd => self.parse_flow_sequence_entry_mapping_end(),
            St::FlowMappingFirstKey => self.parse_flow_mapping_key(true),
            St::FlowMappingKey => self.parse_flow_mapping_key(false),
            St::FlowMappingValue => self.parse_flow_mapping_value(false),
            St::FlowMappingEmptyValue => self.parse_flow_mapping_value(true),
            St::End => Err(()),
        }
    }
    fn parse_stream_start(&mut self) -> R<Event> {
        self.peek()?;
        let (s, enc) = match self.tok().kind { Tk::StreamStart(e) => (self.tok().start, e), _ => { let m = self.tok().start; return self.parser_error("did not find expected <stream-start>", m); } };
        self.state = St::ImplicitDocumentStart;
        self.skip_token();
        Ok(Event { data: EventData::StreamStart { encoding: enc }, start: s, end: s })
    }
    fn parse_document_start(&mut self, implicit: bool) -> R<Event> {
        self.peek()?;
        if !implicit { while self.tok().kind == Tk::DocumentEnd { self.skip_token(); self.peek()?; } }
        let k = &self.tok().kind;
        let is_dir = matches!(k, Tk::VersionDirective(..) | Tk::TagDirective(..));
        if implicit && !is_dir && *k != Tk::DocumentStart && *k != Tk::StreamEnd {
            self.process_directives()?;
            self.states.push(St::DocumentEnd);
            self.state = St::BlockNode;
            let m = self.tok().start;
            Ok(Event { data: EventData::DocumentStart { version: None, tags: vec![], implicit: true }, start: m, end: m })
        } else if *k != Tk::StreamEnd {
            let start = self.tok().start;
            let (version, tags) = self.process_directives()?;
            if self.tok().kind != Tk::DocumentStart { let m = self.tok().start; return self.parser_error("did not find expected <document start>", m); }
            self.states.push(St::DocumentEnd);
            self.state = St::DocumentContent;
            let end = self.tok().end;
            self.skip_token();
            Ok(Event { data: EventData::DocumentStart { version, tags, implicit: false }, start, end })
        } else {
            self.state = St::End;
            let t = self.skip_token();
            Ok(Event { data: EventData::StreamEnd, start: t.start, end: t.end })
        }
    }
    fn parse_document_content(&mut self) -> R<Event> {
        self.peek()?;
        let k = &self.tok().kind;
        if matches!(k, Tk::VersionDirective(..) | Tk::TagDirective(..) | Tk::DocumentStart | Tk::DocumentEnd | Tk::StreamEnd) {
            self.state = self.pop_state();
            let m = self.tok().start;
            Ok(empty_scalar(m))
        } else { self.parse_node(true, false) }
    }
    fn parse_document_end(&mut self) -> R<Event> {
        self.peek()?;
        let start = self.tok().start;
        let mut end = start;
        let mut implicit = true;
        if self.tok().kind == Tk::DocumentEnd { end = self.tok().end; self.skip_token(); implicit = false; }
        self.tag_directives.clear();
        self.state = St::DocumentStart;
        Ok(Event { data: EventData::DocumentEnd { implicit }, start, end })
    }
    fn parse_node(&mut self, block: bool, indentless_sequence: bool) -> R<Event> {
        self.peek()?;
        if let Tk::Alias(_) = self.tok().kind {
            self.state = self.pop_state();
            let t = self.skip_token();
            let Tk::Alias(a) = t.kind else { unreachable!() };
            return Ok(Event { data: EventData::Alias { anchor: a }, start: t.start, end: t.end });
        }
        let mut anchor: Option<Vec<u8>> = None;
        let mut tag_hs: Option<(Vec<u8>, Vec<u8>)> = None;
        let mut start = self.tok().start;
        let mut end = start;
        let mut tag_mark = Mark::default();
        if let Tk::Anchor(_) = self.tok().kind {
            let t = self.skip_token();
            start = t.start; end = t.end;
            if let Tk::Anchor(a) = t.kind { anchor = Some(a); }
            self.peek()?;
            if let Tk::Tag(..) = self.tok().kind {
                let t = self.skip_token();
                tag_mark = t.start; end = t.end;
                if let Tk::Tag(h, s) = t.kind { tag_hs = Some((h, s)); }
                self.peek()?;
            }
        } else if let Tk::Tag(..) = self.tok().kind {
            let t = self.skip_token();
            start = t.start; tag_mark = t.start; end = t.end;
            if let Tk::Tag(h, s) = t.kind { tag_hs = Some((h, s)); }
            self.peek()?;
            if let Tk::Anchor(_) = self.tok().kind {
                let t = self.skip_token();
                end = t.end;
                if let Tk::Anchor(a) = t.kind { anchor = Some(a); }
                self.peek()?;
            }
        }
        let mut tag: Option<Vec<u8>> = None;
        if let Some((h, s)) = tag_hs {
            if h.is_empty() { tag = Some(s); } else {
                match self.tag_directives.iter().find(|d| d.0 == h) {
                    Some(d) => { let mut t = d.1.clone(); t.extend_from_slice(&s); tag = Some(t); }
                    None => return self.parser_error_ctx("while parsing a node", start, "found undefined tag handle", tag_mark),
                }
            }
        }
        let implicit = tag.as_ref().map(|t| t.is_empty()).unwrap_or(true);
        let tk = &self.tok().kind;
        if indentless_sequence && *tk == Tk::BlockEntry {
            end = self.tok().end;
            self.state = St::IndentlessSequenceEntry;
            return Ok(Event { data: EventData::SequenceStart { anchor, tag, implicit, style: BLOCK_STYLE }, start, end });
        }
        match tk {
            Tk::Scalar(_, style) => {
                let style = *style;
                let (mut plain_implicit, mut quoted_implicit) = (false, false);
                if (style == PLAIN && tag.is_none()) || tag.as_deref() == Some(b"!") { plain_implicit = true; } else if tag.is_none() { quoted_implicit = true; }
                self.state = self.pop_state();
                let t = self.skip_token();
                let Tk::Scalar(value, _) = t.kind else { unreachable!() };
                Ok(Event { data: EventData::Scalar { anchor, tag, value, plain_implicit, quoted_implicit, style }, start, end: t.end })
            }
            Tk::FlowSequenceStart => {
                end = self.tok().end; self.state = St::FlowSequenceFirstEntry;
                Ok(Event { data: EventData::SequenceStart { anchor, tag, implicit, style: FLOW_STYLE }, start, end })
            }
            Tk::FlowMappingStart => {
                end = self.tok().end; self.state = St::FlowMappingFirstKey;
                Ok(Event { data: EventData::MappingStart { anchor, tag, implicit, style: FLOW_STYLE }, start, end })
            }
            Tk::BlockSequenceStart if block => {
                end = self.tok().end; self.state = St::BlockSequenceFirstEntry;
                Ok(Event { data: EventData::SequenceStart { anchor, tag, implicit, style: BLOCK_STYLE }, start, end })
            }
            Tk::BlockMappingStart if block => {
                end = self.tok().end; self.state = St::BlockMappingFirstKey;
                Ok(Event { data: EventData::MappingStart { anchor, tag, implicit, style: BLOCK_STYLE }, start, end })
            }
            _ if anchor.is_some() || tag.is_some() => {
                self.state = self.pop_state();
                Ok(Event { data: EventData::Scalar { anchor, tag, value: vec![], plain_implicit: implicit, quoted_implicit: false, style: PLAIN }, start, end })
            }
            _ => {
                let m = self.tok().start;
                self.parser_error_ctx(if block { "while parsing a block node" } else { "while parsing a flow node" }, start, "did not find expected node content", m)
            }
        }
    }
    fn first_mark(&mut self) {
        let m = self.tok().start;
        self.marks.push(m);
        self.skip_token();
    }
    fn pop_mark(&mut self) -> Mark { self.marks.pop().unwrap_or_default() }
    fn parse_block_sequence_entry(&mut self, first: bool) -> R<Event> {
        if first { self.peek()?; self.first_mark(); }
        self.peek()?;
        if self.tok().kind == Tk::BlockEntry {
            let mark = self.tok().end;
            self.skip_token();
            self.peek()?;
            if self.tok().kind != Tk::BlockEntry && self.tok().kind != Tk::BlockEnd {
                self.states.push(St::BlockSequenceEntry);
                self.parse_node(true, false)
            } else { self.state = St::BlockSequenceEntry; Ok(empty_scalar(mark)) }
        } else if self.tok().kind == Tk::BlockEnd {
            self.state = self.pop_state();
            self.pop_mark();
            let t = self.skip_token();
            Ok(Event { data: EventData::SequenceEnd, start: t.start, end: t.end })
        } else {
            let (cm, m) = (self.pop_mark(), self.tok().start);
            self.parser_error_ctx("while parsing a block collection", cm, "did not find expected '-' indicator", m)
        }
    }
    fn parse_indentless_sequence_entry(&mut self) -> R<Event> {
        self.peek()?;
        if self.tok().kind == Tk::BlockEntry {
            let mark = self.tok().end;
            self.skip_token();
            self.peek()?;
            let k = &self.tok().kind;
            if *k != Tk::BlockEntry && *k != Tk::Key && *k != Tk::Value && *k != Tk::BlockEnd {
                self.states.push(St::IndentlessSequenceEntry);
                self.parse_node(true, false)
            } else { self.state = St::IndentlessSequenceEntry; Ok(empty_scalar(mark)) }
        } else {
            self.state = self.pop_state();
            let m = self.tok().start;
            Ok(Event { data: EventData::SequenceEnd, start: m, end: m })
        }
    }
    fn parse_block_mapping_key(&mut self, first: bool) -> R<Event> {
        if first { self.peek()?; self.first_mark(); }
        self.peek()?;
        if self.tok().kind == Tk::Key {
            let mark = self.tok().end;
            self.skip_token();
            self.peek()?;
            let k = &self.tok().kind;
            if *k != Tk::Key && *k != Tk::Value && *k != Tk::BlockEnd {
                self.states.push(St::BlockMappingValue);
                self.parse_node(true, true)
            } else { self.state = St::BlockMappingValue; Ok(empty_scalar(mark)) }
        } else if self.tok().kind == Tk::BlockEnd {
            self.state = self.pop_state();
            self.pop_mark();
            let t = self.skip_token();
            Ok(Event { data: EventData::MappingEnd, start: t.start, end: t.end })
        } else {
            let (cm, m) = (self.pop_mark(), self.tok().start);
            self.parser_error_ctx("while parsing a block mapping", cm, "did not find expected key", m)
        }
    }
    fn parse_block_mapping_value(&mut self) -> R<Event> {
        self.peek()?;
        if self.tok().kind == Tk::Value {
            let mark = self.tok().end;
            self.skip_token();
            self.peek()?;
            let k = &self.tok().kind;
            if *k != Tk::Key && *k != Tk::Value && *k != Tk::BlockEnd {
                self.states.push(St::BlockMappingKey);
                self.parse_node(true, true)
            } else { self.state = St::BlockMappingKey; Ok(empty_scalar(mark)) }
        } else {
            self.state = St::BlockMappingKey;
            let m = self.tok().start;
            Ok(empty_scalar(m))
        }
    }
    fn parse_flow_sequence_entry(&mut self, first: bool) -> R<Event> {
        if first { self.peek()?; self.first_mark(); }
        self.peek()?;
        if self.tok().kind != Tk::FlowSequenceEnd {
            if !first {
                if self.tok().kind == Tk::FlowEntry { self.skip_token(); self.peek()?; }
                else { let (cm, m) = (self.pop_mark(), self.tok().start); return self.parser_error_ctx("while parsing a flow sequence", cm, "did not find expected ',' or ']'", m); }
            }
            if self.tok().kind == Tk::Key {
                self.state = St::FlowSequenceEntryMappingKey;
                let t = self.skip_token();
                return Ok(Event { data: EventData::MappingStart { anchor: None, tag: None, implicit: true, style: FLOW_STYLE }, start: t.start, end: t.end });
            } else if self.tok().kind != Tk::FlowSequenceEnd {
                self.states.push(St::FlowSequenceEntry);
                return self.parse_node(false, false);
            }
        }
        self.state = self.pop_state();
        self.pop_mark();
        let t = self.skip_token();
        Ok(Event { data: EventData::SequenceEnd, start: t.start, end: t.end })
    }
    fn parse_flow_sequence_entry_mapping_key(&mut self) -> R<Event> {
        self.peek()?;
        let k = &self.tok().kind;
        if *k != Tk::Value && *k != Tk::FlowEntry && *k != Tk::FlowSequenceEnd {
            self.states.push(St::FlowSequenceEntryMappingValue);
            self.parse_node(false, false)
        } else {
            let mark = self.tok().end;
            self.skip_token();
            self.state = St::FlowSequenceEntryMappingValue;
            Ok(empty_scalar(mark))
        }
    }
    fn parse_flow_sequence_entry_mapping_value(&mut self) -> R<Event> {
        self.peek()?;
        if self.tok().kind == Tk::Value {
            self.skip_token();
            self.peek()?;
            let k = &self.tok().kind;
            if *k != Tk::FlowEntry && *k != Tk::FlowSequenceEnd {
                self.states.push(St::FlowSequenceEntryMappingEnd);
                return self.parse_node(false, false);
            }
        }
        self.state = St::FlowSequenceEntryMappingEnd;
        let m = self.tok().start;
        Ok(empty_scalar(m))
    }
    fn parse_flow_sequence_entry_mapping_end(&mut self) -> R<Event> {
        self.peek()?;
        self.state = St::FlowSequenceEntry;
        let m = self.tok().start;
        Ok(Event { data: EventData::MappingEnd, start: m, end: m })
    }
    fn parse_flow_mapping_key(&mut self, first: bool) -> R<Event> {
        if first { self.peek()?; self.first_mark(); }
        self.peek()?;
        if self.tok().kind != Tk::FlowMappingEnd {
            if !first {
                if self.tok().kind == Tk::FlowEntry { self.skip_token(); self.peek()?; }
                else { let (cm, m) = (self.pop_mark(), self.tok().start); return self.parser_error_ctx("while parsing a flow mapping", cm, "did not find expected ',' or '}'", m); }
            }
            if self.tok().kind == Tk::Key {
                self.skip_token();
                self.peek()?;
                let k = &self.tok().kind;
                if *k != Tk::Value && *k != Tk::FlowEntry && *k != Tk::FlowMappingEnd {
                    self.states.push(St::FlowMappingValue);
                    return self.parse_node(false, false);
                } else {
                    self.state = St::FlowMappingValue;
                    let m = self.tok().start;
                    return Ok(empty_scalar(m));
                }
            } else if self.tok().kind != Tk::FlowMappingEnd {
                self.states.push(St::FlowMappingEmptyValue);
                return self.parse_node(false, false);
            }
        }
        self.state = self.pop_state();
        self.pop_mark();
        let t = self.skip_token();
        Ok(Event { data: EventData::MappingEnd, start: t.start, end: t.end })
    }
    fn parse_flow_mapping_value(&mut self, empty: bool) -> R<Event> {
        self.peek()?;
        if empty {
            self.state = St::FlowMappingKey;
            let m = self.tok().start;
            return Ok(empty_scalar(m));
        }
        if self.tok().kind == Tk::Value {
            self.skip_token();
            self.peek()?;
            let k = &self.tok().kind;
            if *k != Tk::FlowEntry && *k != Tk::FlowMappingEnd {
                self.states.push(St::FlowMappingKey);
                return self.parse_node(false, false);
            }
        }
        self.state = St::FlowMappingKey;
        let m = self.tok().start;
        Ok(empty_scalar(m))
    }
    #[allow(clippy::type_complexity)]
    fn process_directives(&mut self) -> R<(Option<(i32, i32)>, Vec<(Vec<u8>, Vec<u8>)>)> {
        let mut version: Option<(i32, i32)> = None;
        let mut tags: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
        self.peek()?;
        loop {
            match &self.tok().kind {
                Tk::VersionDirective(major, minor) => {
                    let (major, minor, m) = (*major, *minor, self.tok().start);
                    if version.is_some() { return self.parser_error("found duplicate %YAML directive", m); }
                    if major != 1 || (minor != 1 && minor != 2) { return self.parser_error("found incompatible YAML document", m); }
                    version = Some((major, minor));
                }
                Tk::TagDirective(h, p) => {
                    let (h, p, m) = (h.clone(), p.clone(), self.tok().start);
                    self.append_tag_directive(&h, &p, false, m)?;
                    tags.push((h, p));
                }
                _ => break,
            }
            self.skip_token();
            self.peek()?;
        }
        let m = self.tok().start;
        self.append_tag_directive(b"!", b"!", true, m)?;
        self.append_tag_directive(b"!!", b"tag:yaml.org,2002:", true, m)?;
        Ok((version, tags))
    }
    fn append_tag_directive(&mut self, handle: &[u8], prefix: &[u8], allow_duplicates: bool, mark: Mark) -> R<()> {
        if self.tag_directives.iter().any(|d| d.0 == handle) {
            if allow_duplicates { return Ok(()); }
            return self.parser_error("found duplicate %TAG directive", mark);
        }
        self.tag_directives.push((handle.to_vec(), prefix.to_vec()));
        Ok(())
    }
}

fn empty_scalar(m: Mark) -> Event {
    Event { data: EventData::Scalar { anchor: None, tag: None, value: vec![], plain_implicit: true, quoted_implicit: false, style: PLAIN }, start: m, end: m }
}

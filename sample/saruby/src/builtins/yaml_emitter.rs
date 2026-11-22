// YAML emitter: a port of libyaml 0.2.5 (src/emitter.c, writer.c).
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
use crate::builtins::yaml_parser::{EventData, DOUBLE_QUOTED, FLOW_STYLE, FOLDED, LITERAL, PLAIN, SINGLE_QUOTED, UTF16LE_ENCODING, UTF8_ENCODING};
use std::collections::VecDeque;

const OUTPUT_BUFFER_SIZE: usize = 16384;

#[derive(Clone, Copy, PartialEq, Eq)]
enum ESt {
    StreamStart, FirstDocumentStart, DocumentStart, DocumentContent, DocumentEnd, FlowSequenceFirstItem, FlowSequenceItem,
    FlowMappingFirstKey, FlowMappingKey, FlowMappingSimpleValue, FlowMappingValue, BlockSequenceFirstItem, BlockSequenceItem,
    BlockMappingFirstKey, BlockMappingKey, BlockMappingSimpleValue, BlockMappingValue, End,
}

#[derive(Default)]
struct ScalarData { value: Vec<u8>, multiline: bool, flow_plain_allowed: bool, block_plain_allowed: bool, single_quoted_allowed: bool, block_allowed: bool, style: i32 }

pub struct Emitter {
    /// Flushed output, written to the IO by the caller after each emit.
    pub out: Vec<Vec<u8>>,
    buffer: Vec<u8>,
    encoding: i32,
    pub canonical: bool,
    pub best_indent: i32,
    pub best_width: i32,
    unicode: bool,
    states: Vec<ESt>, state: ESt,
    events: VecDeque<EventData>,
    indents: Vec<i32>,
    tag_directives: Vec<(Vec<u8>, Vec<u8>)>,
    indent: i32, flow_level: i32,
    root_context: bool, sequence_context: bool, mapping_context: bool, simple_key_context: bool,
    line: i32, column: i32, whitespace: bool, indention: bool, open_ended: i32,
    anchor: Option<(Vec<u8>, bool)>,
    tag_handle: Option<Vec<u8>>, tag_suffix: Option<Vec<u8>>,
    scalar: ScalarData,
    pub problem: Option<&'static str>,
}

type R = Result<(), ()>;

fn width(b: u8) -> usize { if b & 0x80 == 0 { 1 } else if b & 0xE0 == 0xC0 { 2 } else if b & 0xF0 == 0xE0 { 3 } else if b & 0xF8 == 0xF0 { 4 } else { 0 } }
fn at(s: &[u8], i: usize) -> u8 { s.get(i).copied().unwrap_or(0) }
fn is_alpha(b: u8) -> bool { b.is_ascii_alphanumeric() || b == b'_' || b == b'-' }
fn is_break_at(s: &[u8], i: usize) -> bool {
    let b = at(s, i);
    b == b'\r' || b == b'\n' || (b == 0xC2 && at(s, i + 1) == 0x85) || (b == 0xE2 && at(s, i + 1) == 0x80 && (at(s, i + 2) == 0xA8 || at(s, i + 2) == 0xA9))
}
fn is_blankz_at(s: &[u8], i: usize) -> bool { let b = at(s, i); b == b' ' || b == b'\t' || b == 0 || is_break_at(s, i) }
fn is_printable_at(s: &[u8], i: usize) -> bool {
    let (b, b1, b2) = (at(s, i), at(s, i + 1), at(s, i + 2));
    b == 0x0A || (0x20..=0x7E).contains(&b) || (b == 0xC2 && b1 >= 0xA0) || (b > 0xC2 && b < 0xED) || (b == 0xED && b1 < 0xA0) || b == 0xEE
        || (b == 0xEF && !(b1 == 0xBB && b2 == 0xBF) && !(b1 == 0xBF && (b2 == 0xBE || b2 == 0xBF)))
}
/// A char's width, never 0 so that malformed input cannot stall the loops.
fn w1(s: &[u8], i: usize) -> usize { width(at(s, i)).max(1) }
fn check_utf8(s: &[u8]) -> bool { std::str::from_utf8(s).is_ok() || { let mut p = 0; let mut ok = true; while p < s.len() { let w = width(s[p]); if w == 0 || p + w > s.len() || s[p + 1..p + w].iter().any(|o| o & 0xC0 != 0x80) { ok = false; break; } p += w; } ok } }

impl Default for Emitter { fn default() -> Self { Emitter::new() } }

impl Emitter {
    pub fn new() -> Emitter {
        Emitter { out: Vec::new(), buffer: Vec::new(), encoding: 0, canonical: false, best_indent: 2, best_width: 0, unicode: true,
            states: Vec::new(), state: ESt::StreamStart, events: VecDeque::new(), indents: Vec::new(), tag_directives: Vec::new(),
            indent: 0, flow_level: 0, root_context: false, sequence_context: false, mapping_context: false, simple_key_context: false,
            line: 0, column: 0, whitespace: false, indention: false, open_ended: 0, anchor: None, tag_handle: None, tag_suffix: None,
            scalar: ScalarData::default(), problem: None }
    }
    pub fn set_indent(&mut self, i: i64) { self.best_indent = if 1 < i && i < 10 { i as i32 } else { 2 }; }
    pub fn set_width(&mut self, w: i64) { self.best_width = if w >= 0 { w.min(i32::MAX as i64) as i32 } else { -1 }; }

    /// Validity checks of the yaml_*_event_initialize functions.
    pub fn event_ok(ev: &EventData) -> bool {
        let ok = |o: &Option<Vec<u8>>| o.as_ref().map(|v| check_utf8(v)).unwrap_or(true);
        match ev {
            EventData::DocumentStart { tags, .. } => tags.iter().all(|(h, p)| check_utf8(h) && check_utf8(p)),
            EventData::Alias { anchor } => check_utf8(anchor),
            EventData::Scalar { anchor, tag, value, .. } => ok(anchor) && ok(tag) && check_utf8(value),
            EventData::SequenceStart { anchor, tag, .. } | EventData::MappingStart { anchor, tag, .. } => ok(anchor) && ok(tag),
            _ => true,
        }
    }

    fn error(&mut self, problem: &'static str) -> R { self.problem = Some(problem); Err(()) }

    pub fn emit(&mut self, ev: EventData) -> R {
        self.events.push_back(ev);
        while !self.need_more_events() {
            self.analyze_event()?;
            self.state_machine()?;
            self.events.pop_front();
        }
        Ok(())
    }
    fn need_more_events(&self) -> bool {
        let Some(head) = self.events.front() else { return true };
        let accumulate = match head { EventData::DocumentStart { .. } => 1, EventData::SequenceStart { .. } => 2, EventData::MappingStart { .. } => 3, _ => return false };
        if self.events.len() > accumulate { return false; }
        let mut level = 0;
        for e in &self.events {
            match e {
                EventData::StreamStart { .. } | EventData::DocumentStart { .. } | EventData::SequenceStart { .. } | EventData::MappingStart { .. } => level += 1,
                EventData::StreamEnd | EventData::DocumentEnd { .. } | EventData::SequenceEnd | EventData::MappingEnd => level -= 1,
                _ => {}
            }
            if level == 0 { return false; }
        }
        true
    }
    fn append_tag_directive(&mut self, h: &[u8], p: &[u8], allow_duplicates: bool) -> R {
        if self.tag_directives.iter().any(|d| d.0 == h) {
            if allow_duplicates { return Ok(()); }
            return self.error("duplicate %TAG directive");
        }
        self.tag_directives.push((h.to_vec(), p.to_vec()));
        Ok(())
    }
    fn increase_indent(&mut self, flow: bool, indentless: bool) {
        self.indents.push(self.indent);
        if self.indent < 0 { self.indent = if flow { self.best_indent } else { 0 }; }
        else if !indentless { self.indent += self.best_indent; }
    }
    fn pop_indent(&mut self) { self.indent = self.indents.pop().unwrap_or(-1); }
    fn pop_state(&mut self) { self.state = self.states.pop().unwrap_or(ESt::End); }
    fn head(&self) -> &EventData { &self.events[0] }
    fn state_machine(&mut self) -> R {
        match self.state {
            ESt::StreamStart => self.emit_stream_start(),
            ESt::FirstDocumentStart => self.emit_document_start(true),
            ESt::DocumentStart => self.emit_document_start(false),
            ESt::DocumentContent => { self.states.push(ESt::DocumentEnd); self.emit_node(true, false, false, false) }
            ESt::DocumentEnd => self.emit_document_end(),
            ESt::FlowSequenceFirstItem => self.emit_flow_sequence_item(true),
            ESt::FlowSequenceItem => self.emit_flow_sequence_item(false),
            ESt::FlowMappingFirstKey => self.emit_flow_mapping_key(true),
            ESt::FlowMappingKey => self.emit_flow_mapping_key(false),
            ESt::FlowMappingSimpleValue => self.emit_flow_mapping_value(true),
            ESt::FlowMappingValue => self.emit_flow_mapping_value(false),
            ESt::BlockSequenceFirstItem => self.emit_block_sequence_item(true),
            ESt::BlockSequenceItem => self.emit_block_sequence_item(false),
            ESt::BlockMappingFirstKey => self.emit_block_mapping_key(true),
            ESt::BlockMappingKey => self.emit_block_mapping_key(false),
            ESt::BlockMappingSimpleValue => self.emit_block_mapping_value(true),
            ESt::BlockMappingValue => self.emit_block_mapping_value(false),
            ESt::End => self.error("expected nothing after STREAM-END"),
        }
    }
    fn emit_stream_start(&mut self) -> R {
        self.open_ended = 0;
        if let EventData::StreamStart { encoding } = *self.head() {
            if self.encoding == 0 { self.encoding = encoding; }
            if self.encoding == 0 { self.encoding = UTF8_ENCODING; }
            if self.best_indent < 2 || self.best_indent > 9 { self.best_indent = 2; }
            if self.best_width >= 0 && self.best_width <= self.best_indent * 2 { self.best_width = 80; }
            if self.best_width < 0 { self.best_width = i32::MAX; }
            self.indent = -1;
            self.line = 0; self.column = 0; self.whitespace = true; self.indention = true;
            if self.encoding != UTF8_ENCODING { self.flush_check(); self.buffer.extend_from_slice(&[0xEF, 0xBB, 0xBF]); }
            self.state = ESt::FirstDocumentStart;
            return Ok(());
        }
        self.error("expected STREAM-START")
    }
    fn emit_document_start(&mut self, first: bool) -> R {
        match self.head() {
            EventData::DocumentStart { version, tags, implicit } => {
                let (version, tags, mut implicit) = (*version, tags.clone(), *implicit);
                if let Some((major, minor)) = version { if major != 1 || (minor != 1 && minor != 2) { return self.error("incompatible %YAML directive"); } }
                for (h, p) in &tags {
                    self.analyze_tag_directive(h, p)?;
                    self.append_tag_directive(h, p, false)?;
                }
                self.append_tag_directive(b"!", b"!", true)?;
                self.append_tag_directive(b"!!", b"tag:yaml.org,2002:", true)?;
                if !first || self.canonical { implicit = false; }
                if (version.is_some() || !tags.is_empty()) && self.open_ended != 0 {
                    self.write_indicator(b"...", true, false, false);
                    self.write_indent();
                }
                self.open_ended = 0;
                if let Some((_, minor)) = version {
                    implicit = false;
                    self.write_indicator(b"%YAML", true, false, false);
                    self.write_indicator(if minor == 1 { b"1.1" } else { b"1.2" }, true, false, false);
                    self.write_indent();
                }
                if !tags.is_empty() {
                    implicit = false;
                    for (h, p) in &tags {
                        self.write_indicator(b"%TAG", true, false, false);
                        self.write_tag_handle(h);
                        self.write_tag_content(p, true);
                        self.write_indent();
                    }
                }
                if !implicit {
                    self.write_indent();
                    self.write_indicator(b"---", true, false, false);
                    if self.canonical { self.write_indent(); }
                }
                self.state = ESt::DocumentContent;
                self.open_ended = 0;
                Ok(())
            }
            EventData::StreamEnd => {
                if self.open_ended == 2 {
                    self.write_indicator(b"...", true, false, false);
                    self.open_ended = 0;
                    self.write_indent();
                }
                self.flush();
                self.state = ESt::End;
                Ok(())
            }
            _ => self.error("expected DOCUMENT-START or STREAM-END"),
        }
    }
    fn emit_document_end(&mut self) -> R {
        if let EventData::DocumentEnd { implicit } = *self.head() {
            self.write_indent();
            if !implicit {
                self.write_indicator(b"...", true, false, false);
                self.open_ended = 0;
                self.write_indent();
            } else if self.open_ended == 0 { self.open_ended = 1; }
            self.flush();
            self.state = ESt::DocumentStart;
            self.tag_directives.clear();
            return Ok(());
        }
        self.error("expected DOCUMENT-END")
    }
    fn emit_flow_sequence_item(&mut self, first: bool) -> R {
        if first { self.write_indicator(b"[", true, true, false); self.increase_indent(true, false); self.flow_level += 1; }
        if let EventData::SequenceEnd = self.head() {
            self.flow_level -= 1;
            self.pop_indent();
            if self.canonical && !first { self.write_indicator(b",", false, false, false); self.write_indent(); }
            self.write_indicator(b"]", false, false, false);
            self.pop_state();
            return Ok(());
        }
        if !first { self.write_indicator(b",", false, false, false); }
        if self.canonical || self.column > self.best_width { self.write_indent(); }
        self.states.push(ESt::FlowSequenceItem);
        self.emit_node(false, true, false, false)
    }
    fn emit_flow_mapping_key(&mut self, first: bool) -> R {
        if first { self.write_indicator(b"{", true, true, false); self.increase_indent(true, false); self.flow_level += 1; }
        if let EventData::MappingEnd = self.head() {
            self.flow_level -= 1;
            self.pop_indent();
            if self.canonical && !first { self.write_indicator(b",", false, false, false); self.write_indent(); }
            self.write_indicator(b"}", false, false, false);
            self.pop_state();
            return Ok(());
        }
        if !first { self.write_indicator(b",", false, false, false); }
        if self.canonical || self.column > self.best_width { self.write_indent(); }
        if !self.canonical && self.check_simple_key() {
            self.states.push(ESt::FlowMappingSimpleValue);
            self.emit_node(false, false, true, true)
        } else {
            self.write_indicator(b"?", true, false, false);
            self.states.push(ESt::FlowMappingValue);
            self.emit_node(false, false, true, false)
        }
    }
    fn emit_flow_mapping_value(&mut self, simple: bool) -> R {
        if simple { self.write_indicator(b":", false, false, false); } else {
            if self.canonical || self.column > self.best_width { self.write_indent(); }
            self.write_indicator(b":", true, false, false);
        }
        self.states.push(ESt::FlowMappingKey);
        self.emit_node(false, false, true, false)
    }
    fn emit_block_sequence_item(&mut self, first: bool) -> R {
        if first { let il = self.mapping_context && !self.indention; self.increase_indent(false, il); }
        if let EventData::SequenceEnd = self.head() { self.pop_indent(); self.pop_state(); return Ok(()); }
        self.write_indent();
        self.write_indicator(b"-", true, false, true);
        self.states.push(ESt::BlockSequenceItem);
        self.emit_node(false, true, false, false)
    }
    fn emit_block_mapping_key(&mut self, first: bool) -> R {
        if first { self.increase_indent(false, false); }
        if let EventData::MappingEnd = self.head() { self.pop_indent(); self.pop_state(); return Ok(()); }
        self.write_indent();
        if self.check_simple_key() {
            self.states.push(ESt::BlockMappingSimpleValue);
            self.emit_node(false, false, true, true)
        } else {
            self.write_indicator(b"?", true, false, true);
            self.states.push(ESt::BlockMappingValue);
            self.emit_node(false, false, true, false)
        }
    }
    fn emit_block_mapping_value(&mut self, simple: bool) -> R {
        if simple { self.write_indicator(b":", false, false, false); } else {
            self.write_indent();
            self.write_indicator(b":", true, false, true);
        }
        self.states.push(ESt::BlockMappingKey);
        self.emit_node(false, false, true, false)
    }
    fn emit_node(&mut self, root: bool, sequence: bool, mapping: bool, simple_key: bool) -> R {
        self.root_context = root; self.sequence_context = sequence; self.mapping_context = mapping; self.simple_key_context = simple_key;
        match self.head() {
            EventData::Alias { .. } => {
                self.process_anchor();
                if self.simple_key_context { self.put(b' '); }
                self.pop_state();
                Ok(())
            }
            EventData::Scalar { .. } => {
                self.select_scalar_style()?;
                self.process_anchor();
                self.process_tag();
                self.increase_indent(true, false);
                self.process_scalar();
                self.pop_indent();
                self.pop_state();
                Ok(())
            }
            EventData::SequenceStart { style, .. } => {
                let style = *style;
                self.process_anchor();
                self.process_tag();
                self.state = if self.flow_level > 0 || self.canonical || style == FLOW_STYLE || self.check_empty_sequence() { ESt::FlowSequenceFirstItem } else { ESt::BlockSequenceFirstItem };
                Ok(())
            }
            EventData::MappingStart { style, .. } => {
                let style = *style;
                self.process_anchor();
                self.process_tag();
                self.state = if self.flow_level > 0 || self.canonical || style == FLOW_STYLE || self.check_empty_mapping() { ESt::FlowMappingFirstKey } else { ESt::BlockMappingFirstKey };
                Ok(())
            }
            _ => self.error("expected SCALAR, SEQUENCE-START, MAPPING-START, or ALIAS"),
        }
    }
    fn check_empty_sequence(&self) -> bool { self.events.len() >= 2 && matches!(self.events[0], EventData::SequenceStart { .. }) && matches!(self.events[1], EventData::SequenceEnd) }
    fn check_empty_mapping(&self) -> bool { self.events.len() >= 2 && matches!(self.events[0], EventData::MappingStart { .. }) && matches!(self.events[1], EventData::MappingEnd) }
    fn check_simple_key(&self) -> bool {
        let al = self.anchor.as_ref().map(|a| a.0.len()).unwrap_or(0);
        let tl = self.tag_handle.as_ref().map(|t| t.len()).unwrap_or(0) + self.tag_suffix.as_ref().map(|t| t.len()).unwrap_or(0);
        let length = match self.head() {
            EventData::Alias { .. } => al,
            EventData::Scalar { .. } => { if self.scalar.multiline { return false; } al + tl + self.scalar.value.len() }
            EventData::SequenceStart { .. } => { if !self.check_empty_sequence() { return false; } al + tl }
            EventData::MappingStart { .. } => { if !self.check_empty_mapping() { return false; } al + tl }
            _ => return false,
        };
        length <= 128
    }
    fn select_scalar_style(&mut self) -> R {
        let EventData::Scalar { style, plain_implicit, quoted_implicit, .. } = *self.head() else { return Ok(()) };
        let mut style = style;
        let no_tag = self.tag_handle.is_none() && self.tag_suffix.is_none();
        if no_tag && !plain_implicit && !quoted_implicit { return self.error("neither tag nor implicit flags are specified"); }
        if style == 0 { style = PLAIN; }
        if self.canonical { style = DOUBLE_QUOTED; }
        if self.simple_key_context && self.scalar.multiline { style = DOUBLE_QUOTED; }
        if style == PLAIN {
            if (self.flow_level > 0 && !self.scalar.flow_plain_allowed) || (self.flow_level == 0 && !self.scalar.block_plain_allowed) { style = SINGLE_QUOTED; }
            if self.scalar.value.is_empty() && (self.flow_level > 0 || self.simple_key_context) { style = SINGLE_QUOTED; }
            if no_tag && !plain_implicit { style = SINGLE_QUOTED; }
        }
        if style == SINGLE_QUOTED && !self.scalar.single_quoted_allowed { style = DOUBLE_QUOTED; }
        if (style == LITERAL || style == FOLDED) && (!self.scalar.block_allowed || self.flow_level > 0 || self.simple_key_context) { style = DOUBLE_QUOTED; }
        if no_tag && !quoted_implicit && style != PLAIN { self.tag_handle = Some(b"!".to_vec()); }
        self.scalar.style = style;
        Ok(())
    }
    fn process_anchor(&mut self) {
        let Some((a, alias)) = self.anchor.take() else { return };
        self.write_indicator(if alias { b"*" } else { b"&" }, true, false, false);
        self.write_anchor(&a);
        self.anchor = Some((a, alias));
    }
    fn process_tag(&mut self) {
        if self.tag_handle.is_none() && self.tag_suffix.is_none() { return; }
        let (h, s) = (self.tag_handle.clone(), self.tag_suffix.clone());
        if let Some(h) = h {
            self.write_tag_handle(&h);
            if let Some(s) = s { self.write_tag_content(&s, false); }
        } else {
            self.write_indicator(b"!<", true, false, false);
            self.write_tag_content(&s.unwrap_or_default(), false);
            self.write_indicator(b">", false, false, false);
        }
    }
    fn process_scalar(&mut self) {
        let v = std::mem::take(&mut self.scalar.value);
        let ab = !self.simple_key_context;
        match self.scalar.style {
            PLAIN => self.write_plain_scalar(&v, ab),
            SINGLE_QUOTED => self.write_single_quoted_scalar(&v, ab),
            DOUBLE_QUOTED => self.write_double_quoted_scalar(&v, ab),
            LITERAL => self.write_literal_scalar(&v),
            FOLDED => self.write_folded_scalar(&v),
            _ => {}
        }
        self.scalar.value = v;
    }
    fn analyze_tag_directive(&mut self, h: &[u8], p: &[u8]) -> R {
        if h.is_empty() { return self.error("tag handle must not be empty"); }
        if h[0] != b'!' { return self.error("tag handle must start with '!'"); }
        if h[h.len() - 1] != b'!' { return self.error("tag handle must end with '!'"); }
        let mut i = 1;
        while i + 1 < h.len() {
            if !is_alpha(h[i]) { return self.error("tag handle must contain alphanumerical characters only"); }
            i += w1(h, i);
        }
        if p.is_empty() { return self.error("tag prefix must not be empty"); }
        Ok(())
    }
    fn analyze_anchor(&mut self, a: &[u8], alias: bool) -> R {
        if a.is_empty() { return self.error(if alias { "alias value must not be empty" } else { "anchor value must not be empty" }); }
        let mut i = 0;
        while i < a.len() {
            if !is_alpha(a[i]) { return self.error(if alias { "alias value must contain alphanumerical characters only" } else { "anchor value must contain alphanumerical characters only" }); }
            i += w1(a, i);
        }
        self.anchor = Some((a.to_vec(), alias));
        Ok(())
    }
    fn analyze_tag(&mut self, t: &[u8]) -> R {
        if t.is_empty() { return self.error("tag value must not be empty"); }
        for (h, p) in &self.tag_directives {
            if p.len() < t.len() && t.starts_with(p) {
                self.tag_handle = Some(h.clone());
                self.tag_suffix = Some(t[p.len()..].to_vec());
                return Ok(());
            }
        }
        self.tag_suffix = Some(t.to_vec());
        Ok(())
    }
    fn analyze_scalar(&mut self, s: &[u8]) {
        let sd = &mut self.scalar;
        sd.value = s.to_vec();
        if s.is_empty() {
            sd.multiline = false; sd.flow_plain_allowed = false; sd.block_plain_allowed = true; sd.single_quoted_allowed = true; sd.block_allowed = false;
            return;
        }
        let (mut block_indicators, mut flow_indicators, mut line_breaks, mut special_characters) = (false, false, false, false);
        let (mut leading_space, mut leading_break, mut trailing_space, mut trailing_break) = (false, false, false, false);
        let (mut break_space, mut space_break, mut previous_space, mut previous_break) = (false, false, false, false);
        if s.starts_with(b"---") || s.starts_with(b"...") { block_indicators = true; flow_indicators = true; }
        let mut preceded_by_whitespace = true;
        let mut followed_by_whitespace = is_blankz_at(s, w1(s, 0));
        let mut i = 0;
        while i < s.len() {
            let c = s[i];
            if i == 0 {
                if b"#,[]{}&*!|>'\"%@`".contains(&c) { flow_indicators = true; block_indicators = true; }
                if c == b'?' || c == b':' { flow_indicators = true; if followed_by_whitespace { block_indicators = true; } }
                if c == b'-' && followed_by_whitespace { flow_indicators = true; block_indicators = true; }
            } else {
                if b",?[]{}".contains(&c) { flow_indicators = true; }
                if c == b':' { flow_indicators = true; if followed_by_whitespace { block_indicators = true; } }
                if c == b'#' && preceded_by_whitespace { flow_indicators = true; block_indicators = true; }
            }
            if !is_printable_at(s, i) || (c > 0x7F && !self.unicode) { special_characters = true; }
            let brk = is_break_at(s, i);
            if brk { line_breaks = true; }
            let w = w1(s, i);
            if c == b' ' {
                if i == 0 { leading_space = true; }
                if i + w == s.len() { trailing_space = true; }
                if previous_break { break_space = true; }
                previous_space = true; previous_break = false;
            } else if brk {
                if i == 0 { leading_break = true; }
                if i + w == s.len() { trailing_break = true; }
                if previous_space { space_break = true; }
                previous_space = false; previous_break = true;
            } else { previous_space = false; previous_break = false; }
            preceded_by_whitespace = is_blankz_at(s, i);
            i += w;
            if i < s.len() { followed_by_whitespace = is_blankz_at(s, i + w1(s, i)); }
        }
        let sd = &mut self.scalar;
        sd.multiline = line_breaks;
        sd.flow_plain_allowed = true; sd.block_plain_allowed = true; sd.single_quoted_allowed = true; sd.block_allowed = true;
        if leading_space || leading_break || trailing_space || trailing_break { sd.flow_plain_allowed = false; sd.block_plain_allowed = false; }
        if trailing_space { sd.block_allowed = false; }
        if break_space { sd.flow_plain_allowed = false; sd.block_plain_allowed = false; sd.single_quoted_allowed = false; }
        if space_break || special_characters { sd.flow_plain_allowed = false; sd.block_plain_allowed = false; sd.single_quoted_allowed = false; sd.block_allowed = false; }
        if line_breaks { sd.flow_plain_allowed = false; sd.block_plain_allowed = false; }
        if flow_indicators { sd.flow_plain_allowed = false; }
        if block_indicators { sd.block_plain_allowed = false; }
    }
    fn analyze_event(&mut self) -> R {
        self.anchor = None; self.tag_handle = None; self.tag_suffix = None; self.scalar.value.clear();
        let canonical = self.canonical;
        let (anchor, alias, tag, value): (Option<Vec<u8>>, bool, Option<Vec<u8>>, Option<Vec<u8>>) = match self.head() {
            EventData::Alias { anchor } => (Some(anchor.clone()), true, None, None),
            EventData::Scalar { anchor, tag, value, plain_implicit, quoted_implicit, .. } =>
                (anchor.clone(), false, if canonical || (!plain_implicit && !quoted_implicit) { tag.clone() } else { None }, Some(value.clone())),
            EventData::SequenceStart { anchor, tag, implicit, .. } | EventData::MappingStart { anchor, tag, implicit, .. } =>
                (anchor.clone(), false, if canonical || !implicit { tag.clone() } else { None }, None),
            _ => return Ok(()),
        };
        if let Some(a) = anchor { self.analyze_anchor(&a, alias)?; }
        if let Some(t) = tag { self.analyze_tag(&t)?; }
        if let Some(v) = value { self.analyze_scalar(&v); }
        Ok(())
    }

    // ---- output ----

    fn flush(&mut self) {
        if self.buffer.is_empty() { return; }
        let b = std::mem::take(&mut self.buffer);
        if self.encoding == UTF8_ENCODING { self.out.push(b); return; }
        let (low, high) = if self.encoding == UTF16LE_ENCODING { (0, 1) } else { (1, 0) };
        let mut raw = Vec::with_capacity(b.len() * 2);
        let mut i = 0;
        while i < b.len() {
            let w = w1(&b, i);
            let o = b[i];
            let mut v: u32 = match w { 1 => o & 0x7F, 2 => o & 0x1F, 3 => o & 0x0F, _ => o & 0x07 } as u32;
            for k in 1..w { v = (v << 6) + (at(&b, i + k) & 0x3F) as u32; }
            i += w;
            let mut put = |x: u32| { let mut p = [0u8; 2]; p[high] = (x >> 8) as u8; p[low] = (x & 0xFF) as u8; raw.extend_from_slice(&p); };
            if v < 0x10000 { put(v); } else { let v = v - 0x10000; put(0xD800 + (v >> 10)); put(0xDC00 + (v & 0x3FF)); }
        }
        self.out.push(raw);
    }
    fn flush_check(&mut self) { if self.buffer.len() + 5 >= OUTPUT_BUFFER_SIZE { self.flush(); } }
    fn put(&mut self, c: u8) { self.flush_check(); self.buffer.push(c); self.column += 1; }
    fn put_break(&mut self) { self.flush_check(); self.buffer.push(b'\n'); self.column = 0; self.line += 1; }
    /// WRITE: one character of s at *i.
    fn write(&mut self, s: &[u8], i: &mut usize) {
        self.flush_check();
        let w = w1(s, *i);
        let e = (*i + w).min(s.len());
        self.buffer.extend_from_slice(&s[*i..e]);
        *i += w;
        self.column += 1;
    }
    fn write_break(&mut self, s: &[u8], i: &mut usize) {
        self.flush_check();
        if s[*i] == b'\n' { self.put_break(); *i += 1; } else {
            let w = w1(s, *i);
            let e = (*i + w).min(s.len());
            self.buffer.extend_from_slice(&s[*i..e]);
            *i += w;
            self.column = 0; self.line += 1;
        }
    }
    fn write_indent(&mut self) {
        let indent = if self.indent >= 0 { self.indent } else { 0 };
        if !self.indention || self.column > indent || (self.column == indent && !self.whitespace) { self.put_break(); }
        while self.column < indent { self.put(b' '); }
        self.whitespace = true;
        self.indention = true;
    }
    fn write_indicator(&mut self, ind: &[u8], need_whitespace: bool, is_whitespace: bool, is_indention: bool) {
        if need_whitespace && !self.whitespace { self.put(b' '); }
        let mut i = 0;
        while i < ind.len() { self.write(ind, &mut i); }
        self.whitespace = is_whitespace;
        self.indention = self.indention && is_indention;
    }
    fn write_anchor(&mut self, v: &[u8]) {
        let mut i = 0;
        while i < v.len() { self.write(v, &mut i); }
        self.whitespace = false; self.indention = false;
    }
    fn write_tag_handle(&mut self, v: &[u8]) {
        if !self.whitespace { self.put(b' '); }
        let mut i = 0;
        while i < v.len() { self.write(v, &mut i); }
        self.whitespace = false; self.indention = false;
    }
    fn write_tag_content(&mut self, v: &[u8], need_whitespace: bool) {
        if need_whitespace && !self.whitespace { self.put(b' '); }
        let mut i = 0;
        while i < v.len() {
            let c = v[i];
            if is_alpha(c) || b";/?:@&=+$,_.~*'()[]".contains(&c) { self.write(v, &mut i); } else {
                let mut w = w1(v, i);
                while w > 0 && i < v.len() {
                    let value = v[i];
                    i += 1;
                    w -= 1;
                    let hex = |d: u8| if d < 10 { b'0' + d } else { b'A' + d - 10 };
                    self.put(b'%'); self.put(hex(value >> 4)); self.put(hex(value & 0x0F));
                }
            }
        }
        self.whitespace = false; self.indention = false;
    }
    fn write_plain_scalar(&mut self, s: &[u8], allow_breaks: bool) {
        let (mut spaces, mut breaks) = (false, false);
        if !self.whitespace && (!s.is_empty() || self.flow_level > 0) { self.put(b' '); }
        let mut i = 0;
        while i < s.len() {
            if s[i] == b' ' {
                if allow_breaks && !spaces && self.column > self.best_width && at(s, i + 1) != b' ' { self.write_indent(); i += 1; } else { self.write(s, &mut i); }
                spaces = true;
            } else if is_break_at(s, i) {
                if !breaks && s[i] == b'\n' { self.put_break(); }
                self.write_break(s, &mut i);
                self.indention = true;
                breaks = true;
            } else {
                if breaks { self.write_indent(); }
                self.write(s, &mut i);
                self.indention = false;
                spaces = false; breaks = false;
            }
        }
        self.whitespace = false; self.indention = false;
    }
    fn write_single_quoted_scalar(&mut self, s: &[u8], allow_breaks: bool) {
        let (mut spaces, mut breaks) = (false, false);
        self.write_indicator(b"'", true, false, false);
        let mut i = 0;
        while i < s.len() {
            if s[i] == b' ' {
                if allow_breaks && !spaces && self.column > self.best_width && i != 0 && i != s.len() - 1 && at(s, i + 1) != b' ' { self.write_indent(); i += 1; } else { self.write(s, &mut i); }
                spaces = true;
            } else if is_break_at(s, i) {
                if !breaks && s[i] == b'\n' { self.put_break(); }
                self.write_break(s, &mut i);
                self.indention = true;
                breaks = true;
            } else {
                if breaks { self.write_indent(); }
                if s[i] == b'\'' { self.put(b'\''); }
                self.write(s, &mut i);
                self.indention = false;
                spaces = false; breaks = false;
            }
        }
        if breaks { self.write_indent(); }
        self.write_indicator(b"'", false, false, false);
        self.whitespace = false; self.indention = false;
    }
    fn write_double_quoted_scalar(&mut self, s: &[u8], allow_breaks: bool) {
        let mut spaces = false;
        self.write_indicator(b"\"", true, false, false);
        let mut i = 0;
        while i < s.len() {
            let c = s[i];
            let bom = c == 0xEF && at(s, i + 1) == 0xBB && at(s, i + 2) == 0xBF;
            if !is_printable_at(s, i) || (!self.unicode && c > 0x7F) || bom || is_break_at(s, i) || c == b'"' || c == b'\\' {
                let w = width(c);
                let mut value: u32 = match w { 1 => c & 0x7F, 2 => c & 0x1F, 3 => c & 0x0F, 4 => c & 0x07, _ => 0 } as u32;
                for k in 1..w { value = (value << 6) + (at(s, i + k) & 0x3F) as u32; }
                i += w.max(1);
                self.put(b'\\');
                match value {
                    0x00 => self.put(b'0'), 0x07 => self.put(b'a'), 0x08 => self.put(b'b'), 0x09 => self.put(b't'), 0x0A => self.put(b'n'),
                    0x0B => self.put(b'v'), 0x0C => self.put(b'f'), 0x0D => self.put(b'r'), 0x1B => self.put(b'e'), 0x22 => self.put(b'"'),
                    0x5C => self.put(b'\\'), 0x85 => self.put(b'N'), 0xA0 => self.put(b'_'), 0x2028 => self.put(b'L'), 0x2029 => self.put(b'P'),
                    _ => {
                        let width = if value <= 0xFF { self.put(b'x'); 2 } else if value <= 0xFFFF { self.put(b'u'); 4 } else { self.put(b'U'); 8 };
                        let mut k: i32 = (width - 1) * 4;
                        while k >= 0 {
                            let d = ((value >> k) & 0x0F) as u8;
                            self.put(if d < 10 { b'0' + d } else { b'A' + d - 10 });
                            k -= 4;
                        }
                    }
                }
                spaces = false;
            } else if c == b' ' {
                if allow_breaks && !spaces && self.column > self.best_width && i != 0 && i != s.len() - 1 {
                    self.write_indent();
                    if at(s, i + 1) == b' ' { self.put(b'\\'); }
                    i += 1;
                } else { self.write(s, &mut i); }
                spaces = true;
            } else {
                self.write(s, &mut i);
                spaces = false;
            }
        }
        self.write_indicator(b"\"", false, false, false);
        self.whitespace = false; self.indention = false;
    }
    fn write_block_scalar_hints(&mut self, s: &[u8]) {
        let mut chomp: Option<&[u8]> = None;
        if at(s, 0) == b' ' || is_break_at(s, 0) {
            let hint = [b'0' + self.best_indent as u8];
            self.write_indicator(&hint, false, false, false);
        }
        self.open_ended = 0;
        if s.is_empty() { chomp = Some(b"-"); } else {
            let mut p = s.len();
            loop { p -= 1; if p == 0 || s[p] & 0xC0 != 0x80 { break; } }
            if !is_break_at(s, p) { chomp = Some(b"-"); }
            else if p == 0 { chomp = Some(b"+"); self.open_ended = 2; }
            else {
                loop { p -= 1; if p == 0 || s[p] & 0xC0 != 0x80 { break; } }
                if is_break_at(s, p) { chomp = Some(b"+"); self.open_ended = 2; }
            }
        }
        if let Some(c) = chomp { self.write_indicator(c, false, false, false); }
    }
    fn write_literal_scalar(&mut self, s: &[u8]) {
        let mut breaks = true;
        self.write_indicator(b"|", true, false, false);
        self.write_block_scalar_hints(s);
        self.put_break();
        self.indention = true; self.whitespace = true;
        let mut i = 0;
        while i < s.len() {
            if is_break_at(s, i) {
                self.write_break(s, &mut i);
                self.indention = true;
                breaks = true;
            } else {
                if breaks { self.write_indent(); }
                self.write(s, &mut i);
                self.indention = false;
                breaks = false;
            }
        }
    }
    fn write_folded_scalar(&mut self, s: &[u8]) {
        let (mut breaks, mut leading_spaces) = (true, true);
        self.write_indicator(b">", true, false, false);
        self.write_block_scalar_hints(s);
        self.put_break();
        self.indention = true; self.whitespace = true;
        let mut i = 0;
        while i < s.len() {
            if is_break_at(s, i) {
                if !breaks && !leading_spaces && s[i] == b'\n' {
                    let mut k = 0;
                    while is_break_at(s, i + k) { k += w1(s, i + k); }
                    if !is_blankz_at(s, i + k) { self.put_break(); }
                }
                self.write_break(s, &mut i);
                self.indention = true;
                breaks = true;
            } else {
                if breaks { self.write_indent(); leading_spaces = s[i] == b' ' || s[i] == b'\t'; }
                if !breaks && s[i] == b' ' && at(s, i + 1) != b' ' && self.column > self.best_width { self.write_indent(); i += 1; } else { self.write(s, &mut i); }
                self.indention = false;
                breaks = false;
            }
        }
    }
}

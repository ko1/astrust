// Backtracking regex engine following Ruby (Onigmo) syntax and semantics.
// Pattern -> AST -> instruction program; matching uses an explicit backtrack
// stack so subject length never drives native recursion.

pub const OPT_I: u32 = 1;
pub const OPT_X: u32 = 2;
pub const OPT_M: u32 = 4;

const NONE: usize = usize::MAX;
const INF: u32 = u32::MAX;
// Invalid UTF-8 byte b is represented as the pseudo char BAD + b.
const BAD: u32 = 0x110000;
const MAX_REPEAT: u32 = 100000;

// ---------------------------------------------------------------------------
// UTF-8 helpers

#[inline]
fn decode(s: &[u8], i: usize) -> (u32, usize) {
    let b0 = s[i];
    if b0 < 0x80 {
        return (b0 as u32, 1);
    }
    let (len, init) = if b0 & 0xE0 == 0xC0 {
        (2, (b0 & 0x1F) as u32)
    } else if b0 & 0xF0 == 0xE0 {
        (3, (b0 & 0x0F) as u32)
    } else if b0 & 0xF8 == 0xF0 {
        (4, (b0 & 0x07) as u32)
    } else {
        return (BAD + b0 as u32, 1);
    };
    if i + len > s.len() {
        return (BAD + b0 as u32, 1);
    }
    let mut c = init;
    for k in 1..len {
        let b = s[i + k];
        if b & 0xC0 != 0x80 {
            return (BAD + b0 as u32, 1);
        }
        c = (c << 6) | (b & 0x3F) as u32;
    }
    let ok = match len {
        2 => c >= 0x80,
        3 => c >= 0x800 && !(0xD800..=0xDFFF).contains(&c),
        _ => (0x10000..=0x10FFFF).contains(&c),
    };
    if ok { (c, len) } else { (BAD + b0 as u32, 1) }
}

#[inline]
fn char_len(s: &[u8], i: usize) -> usize {
    if s[i] < 0x80 { 1 } else { decode(s, i).1 }
}

/// Start of the char ending at `i` (requires i > lo; never goes below lo).
#[inline]
fn prev_start(s: &[u8], i: usize, lo: usize) -> usize {
    let mut j = i - 1;
    if s[j] < 0x80 {
        return j;
    }
    while j > lo && i - j < 4 && s[j] & 0xC0 == 0x80 {
        j -= 1;
    }
    if j < i - 1 && decode(s, j).1 == i - j { j } else { i - 1 }
}

fn push_char(v: &mut Vec<u8>, c: u32) {
    if c >= BAD {
        v.push((c - BAD) as u8);
    } else if let Some(ch) = char::from_u32(c) {
        let mut buf = [0u8; 4];
        v.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
    }
}

fn first_byte(c: u32) -> u8 {
    let mut v = Vec::new();
    push_char(&mut v, c);
    v[0]
}

// ---------------------------------------------------------------------------
// Case folding

fn lower1(c: u32) -> u32 {
    if c < 128 {
        return (c as u8).to_ascii_lowercase() as u32;
    }
    match char::from_u32(c) {
        Some(ch) => {
            let mut it = ch.to_lowercase();
            let a = it.next().unwrap();
            if it.next().is_none() { a as u32 } else { c }
        }
        None => c,
    }
}

fn upper1(c: u32) -> u32 {
    if c < 128 {
        return (c as u8).to_ascii_uppercase() as u32;
    }
    match char::from_u32(c) {
        Some(ch) => {
            let mut it = ch.to_uppercase();
            let a = it.next().unwrap();
            if it.next().is_none() { a as u32 } else { c }
        }
        None => c,
    }
}

#[inline]
fn fold(c: u32) -> u32 {
    if c < 128 {
        return (c as u8).to_ascii_lowercase() as u32;
    }
    lower1(upper1(c))
}

fn has_case(c: u32) -> bool {
    fold(c) != c || upper1(c) != c || lower1(c) != c || c == 0xDF
}

const SHARP_S: [u32; 2] = [0xDF, 0x1E9E];

// ---------------------------------------------------------------------------
// Character properties

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Prop {
    Digit, // ASCII \d
    Word,  // ASCII \w
    Space, // ASCII \s
    Hex,   // \h
    Alpha,
    UDigit,
    USpace,
    Upper,
    Lower,
    Alnum,
    UWord,
    Punct,
    Cntrl,
    Graph,
    Print,
    Blank,
    Ascii,
    Any,
    Letter,
    Mark,
    Number,
    Hiragana,
    Katakana,
    Han,
    Latin,
    Greek,
    Cyrillic,
    Hangul,
    Arabic,
    Hebrew,
    Thai,
    Emoji,
}

const ND_STARTS: &[u32] = &[
    0x30, 0x660, 0x6F0, 0x7C0, 0x966, 0x9E6, 0xA66, 0xAE6, 0xB66, 0xBE6, 0xC66, 0xCE6, 0xD66,
    0xDE6, 0xE50, 0xED0, 0xF20, 0x1040, 0x1090, 0x17E0, 0x1810, 0x1946, 0x19D0, 0x1A80, 0x1A90,
    0x1B50, 0x1BB0, 0x1C40, 0x1C50, 0xA620, 0xA8D0, 0xA900, 0xA9D0, 0xA9F0, 0xAA50, 0xABF0,
    0xFF10, 0x104A0, 0x10D30, 0x11066, 0x110F0, 0x11136, 0x111D0, 0x112F0, 0x11450, 0x114D0,
    0x11650, 0x116C0, 0x11730, 0x118E0, 0x11950, 0x11C50, 0x11D50, 0x11DA0, 0x16A60, 0x16B50,
    0x1E140, 0x1E2F0, 0x1E950, 0x1FBF0,
];

fn is_nd(c: u32) -> bool {
    if c < 128 {
        return (b'0' as u32..=b'9' as u32).contains(&c);
    }
    if (0x1D7CE..=0x1D7FF).contains(&c) {
        return true;
    }
    ND_STARTS.iter().any(|&s| c >= s && c < s + 10)
}

const MARK_RANGES: &[(u32, u32)] = &[
    (0x300, 0x36F), (0x483, 0x489), (0x591, 0x5BD), (0x5BF, 0x5BF), (0x5C1, 0x5C2), (0x5C4, 0x5C5),
    (0x5C7, 0x5C7), (0x610, 0x61A), (0x64B, 0x65F), (0x670, 0x670), (0x6D6, 0x6DC), (0x6DF, 0x6E4),
    (0x6E7, 0x6E8), (0x6EA, 0x6ED), (0x711, 0x711), (0x730, 0x74A), (0x7A6, 0x7B0), (0x7EB, 0x7F3),
    (0x816, 0x82D), (0x859, 0x85B), (0x8D3, 0x903), (0x93A, 0x93C), (0x93E, 0x94F), (0x951, 0x957),
    (0x962, 0x963), (0x981, 0x983), (0x9BC, 0x9BC), (0x9BE, 0x9CD), (0x9D7, 0x9D7), (0xA01, 0xA03),
    (0xA3C, 0xA51), (0xA70, 0xA71), (0xA81, 0xA83), (0xABC, 0xABC), (0xABE, 0xACD), (0xB01, 0xB03),
    (0xB3C, 0xB3C), (0xB3E, 0xB57), (0xB82, 0xB82), (0xBBE, 0xBCD), (0xBD7, 0xBD7), (0xC00, 0xC04),
    (0xC3E, 0xC56), (0xC81, 0xC83), (0xCBC, 0xCBC), (0xCBE, 0xCD6), (0xD00, 0xD03), (0xD3E, 0xD4D),
    (0xD57, 0xD57), (0xE31, 0xE31), (0xE34, 0xE3A), (0xE47, 0xE4E), (0xEB1, 0xEB1), (0xEB4, 0xEBC),
    (0xEC8, 0xECD), (0xF18, 0xF19), (0xF35, 0xF35), (0xF37, 0xF37), (0xF39, 0xF39), (0xF3E, 0xF3F),
    (0xF71, 0xF84), (0x102B, 0x103E), (0x1AB0, 0x1AFF), (0x1DC0, 0x1DFF), (0x20D0, 0x20F0),
    (0x302A, 0x302F), (0x3099, 0x309A), (0xFE00, 0xFE0F), (0xFE20, 0xFE2F), (0x1F3FB, 0x1F3FF),
    (0xE0100, 0xE01EF),
];

fn is_mark(c: u32) -> bool {
    c >= 0x300 && MARK_RANGES.iter().any(|&(a, b)| c >= a && c <= b)
}

fn is_zs(c: u32) -> bool {
    matches!(c, 0x20 | 0xA0 | 0x1680 | 0x2000..=0x200A | 0x202F | 0x205F | 0x3000)
}

fn is_uword(c: u32) -> bool {
    if c < 128 {
        let b = c as u8;
        return b.is_ascii_alphanumeric() || b == b'_';
    }
    match char::from_u32(c) {
        Some(ch) => ch.is_alphanumeric() || is_mark(c) || matches!(c, 0x203F | 0x2040 | 0x2054 | 0xFE33 | 0xFE34 | 0xFE4D..=0xFE4F | 0xFF3F),
        None => false,
    }
}

fn is_upunct(c: u32) -> bool {
    if c < 128 {
        return (c as u8).is_ascii_punctuation();
    }
    matches!(c,
        0xA1 | 0xA7 | 0xAB | 0xB6 | 0xB7 | 0xBB | 0xBF | 0x37E | 0x387 | 0x55A..=0x55F | 0x589 | 0x58A
        | 0x5BE | 0x5C0 | 0x5C3 | 0x5C6 | 0x5F3 | 0x5F4 | 0x609 | 0x60A | 0x60C | 0x60D | 0x61B
        | 0x61E | 0x61F | 0x66A..=0x66D | 0x6D4 | 0x964 | 0x965 | 0x970 | 0xE4F | 0xE5A | 0xE5B
        | 0x2010..=0x2027 | 0x2030..=0x2043 | 0x2045..=0x2051 | 0x2053..=0x205E | 0x207D | 0x207E
        | 0x208D | 0x208E | 0x2308..=0x230B | 0x2329 | 0x232A | 0x2768..=0x2775 | 0x27C5 | 0x27C6
        | 0x27E6..=0x27EF | 0x2983..=0x2998 | 0x29D8..=0x29DB | 0x29FC | 0x29FD | 0x2E00..=0x2E4F
        | 0x3001..=0x3003 | 0x3008..=0x3011 | 0x3014..=0x301F | 0x3030 | 0x303D | 0x30A0 | 0x30FB
        | 0xFE10..=0xFE19 | 0xFE30..=0xFE52 | 0xFE54..=0xFE61 | 0xFE63 | 0xFE68 | 0xFE6A | 0xFE6B
        | 0xFF01..=0xFF03 | 0xFF05..=0xFF0A | 0xFF0C..=0xFF0F | 0xFF1A | 0xFF1B | 0xFF1F | 0xFF20
        | 0xFF3B..=0xFF3D | 0xFF3F | 0xFF5B | 0xFF5D | 0xFF5F..=0xFF65)
}

fn prop_match(p: Prop, c: u32) -> bool {
    if c >= BAD {
        return matches!(p, Prop::Any);
    }
    let ch = match char::from_u32(c) {
        Some(ch) => ch,
        None => return false,
    };
    match p {
        Prop::Digit => ch.is_ascii_digit(),
        Prop::Word => ch.is_ascii_alphanumeric() || ch == '_',
        Prop::Space => matches!(c, 0x20 | 0x09..=0x0D),
        Prop::Hex => ch.is_ascii_hexdigit(),
        Prop::Alpha => ch.is_alphabetic(),
        Prop::UDigit => is_nd(c),
        Prop::USpace => ch.is_whitespace(),
        Prop::Upper => ch.is_uppercase(),
        Prop::Lower => ch.is_lowercase(),
        Prop::Alnum => ch.is_alphabetic() || is_nd(c),
        Prop::UWord => is_uword(c),
        Prop::Punct => is_upunct(c),
        Prop::Cntrl => ch.is_control(),
        Prop::Graph => !ch.is_whitespace() && !ch.is_control() && !(0xD800..=0xDFFF).contains(&c),
        Prop::Print => (!ch.is_whitespace() && !ch.is_control()) || is_zs(c),
        Prop::Blank => c == 9 || is_zs(c),
        Prop::Ascii => c < 128,
        Prop::Any => true,
        Prop::Letter => ch.is_alphabetic() && !is_mark(c) && !(0x2160..=0x2188).contains(&c) && !(0x24B6..=0x24E9).contains(&c),
        Prop::Mark => is_mark(c),
        Prop::Number => ch.is_numeric(),
        Prop::Hiragana => (0x3041..=0x3096).contains(&c) || (0x309D..=0x309F).contains(&c) || c == 0x1B001 || c == 0x1F200,
        Prop::Katakana => (0x30A1..=0x30FA).contains(&c) || (0x30FD..=0x30FF).contains(&c) || (0x31F0..=0x31FF).contains(&c)
            || (0x32D0..=0x32FE).contains(&c) || (0x3300..=0x3357).contains(&c) || (0xFF66..=0xFF6F).contains(&c)
            || (0xFF71..=0xFF9D).contains(&c) || c == 0x1B000,
        Prop::Han => matches!(c, 0x2E80..=0x2E99 | 0x2E9B..=0x2EF3 | 0x2F00..=0x2FD5 | 0x3005 | 0x3007
            | 0x3021..=0x3029 | 0x3038..=0x303B | 0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xF900..=0xFA6D
            | 0xFA70..=0xFAD9 | 0x20000..=0x3134A),
        Prop::Latin => (ch.is_ascii_alphabetic()) || matches!(c, 0xAA | 0xBA | 0xC0..=0xD6 | 0xD8..=0xF6
            | 0xF8..=0x2B8 | 0x2E0..=0x2E4 | 0x1D00..=0x1D25 | 0x1E00..=0x1EFF | 0x2C60..=0x2C7F
            | 0xA722..=0xA7FF | 0xFB00..=0xFB06 | 0xFF21..=0xFF3A | 0xFF41..=0xFF5A),
        Prop::Greek => matches!(c, 0x370..=0x373 | 0x375..=0x377 | 0x37A..=0x37D | 0x37F | 0x384 | 0x386
            | 0x388..=0x3E1 | 0x3F0..=0x3FF | 0x1D26..=0x1D2A | 0x1F00..=0x1FFE | 0x2126),
        Prop::Cyrillic => matches!(c, 0x400..=0x52F | 0x1C80..=0x1C88 | 0x2DE0..=0x2DFF | 0xA640..=0xA69F),
        Prop::Hangul => matches!(c, 0x1100..=0x11FF | 0x3131..=0x318E | 0xA960..=0xA97C | 0xAC00..=0xD7A3 | 0xD7B0..=0xD7FB | 0xFFA0..=0xFFDC),
        Prop::Arabic => matches!(c, 0x600..=0x604 | 0x606..=0x60B | 0x60D..=0x61A | 0x61E | 0x620..=0x63F | 0x641..=0x64A
            | 0x656..=0x66F | 0x671..=0x6DC | 0x6DE..=0x6FF | 0x750..=0x77F | 0x8A0..=0x8FF | 0xFB50..=0xFDFF | 0xFE70..=0xFEFC),
        Prop::Hebrew => matches!(c, 0x591..=0x5C7 | 0x5D0..=0x5EA | 0x5EF..=0x5F4 | 0xFB1D..=0xFB4F),
        Prop::Thai => matches!(c, 0xE01..=0xE3A | 0xE40..=0xE5B),
        Prop::Emoji => matches!(c, 0x23 | 0x2A | 0x30..=0x39 | 0xA9 | 0xAE | 0x203C | 0x2049 | 0x2122 | 0x2139
            | 0x2194..=0x2199 | 0x21A9 | 0x21AA | 0x231A | 0x231B | 0x2328 | 0x23CF | 0x23E9..=0x23F3
            | 0x23F8..=0x23FA | 0x24C2 | 0x25AA | 0x25AB | 0x25B6 | 0x25C0 | 0x25FB..=0x25FE
            | 0x2600..=0x27BF | 0x2934 | 0x2935 | 0x2B05..=0x2B07 | 0x2B1B | 0x2B1C | 0x2B50 | 0x2B55
            | 0x3030 | 0x303D | 0x3297 | 0x3299 | 0x1F000..=0x1FAFF),
    }
}

fn prop_by_name(name: &str) -> Option<Prop> {
    let n: String = name.chars().filter(|c| !matches!(c, ' ' | '_' | '-')).collect::<String>().to_ascii_lowercase();
    Some(match n.as_str() {
        "alpha" | "alphabetic" => Prop::Alpha,
        "digit" | "nd" | "decimalnumber" => Prop::UDigit,
        "space" | "whitespace" => Prop::USpace,
        "upper" | "uppercase" | "lu" | "uppercaseletter" => Prop::Upper,
        "lower" | "lowercase" | "ll" | "lowercaseletter" => Prop::Lower,
        "alnum" => Prop::Alnum,
        "word" => Prop::UWord,
        "punct" | "p" | "punctuation" => Prop::Punct,
        "cntrl" | "cc" | "control" => Prop::Cntrl,
        "graph" => Prop::Graph,
        "print" => Prop::Print,
        "xdigit" | "asciihexdigit" => Prop::Hex,
        "blank" => Prop::Blank,
        "ascii" => Prop::Ascii,
        "any" => Prop::Any,
        "l" | "letter" => Prop::Letter,
        "m" | "mark" | "combiningmark" | "mn" => Prop::Mark,
        "n" | "number" => Prop::Number,
        "hiragana" | "hira" => Prop::Hiragana,
        "katakana" | "kana" => Prop::Katakana,
        "han" | "hani" => Prop::Han,
        "latin" | "latn" => Prop::Latin,
        "greek" | "grek" => Prop::Greek,
        "cyrillic" | "cyrl" => Prop::Cyrillic,
        "hangul" | "hang" => Prop::Hangul,
        "arabic" | "arab" => Prop::Arabic,
        "hebrew" | "hebr" => Prop::Hebrew,
        "thai" => Prop::Thai,
        "emoji" => Prop::Emoji,
        _ => return None,
    })
}

fn posix_by_name(n: &str) -> Option<Prop> {
    Some(match n {
        "alnum" => Prop::Alnum,
        "alpha" => Prop::Alpha,
        "ascii" => Prop::Ascii,
        "blank" => Prop::Blank,
        "cntrl" => Prop::Cntrl,
        "digit" => Prop::UDigit,
        "graph" => Prop::Graph,
        "lower" => Prop::Lower,
        "print" => Prop::Print,
        "punct" => Prop::Punct,
        "space" => Prop::USpace,
        "upper" => Prop::Upper,
        "xdigit" => Prop::Hex,
        "word" => Prop::UWord,
        _ => return None,
    })
}

// ---------------------------------------------------------------------------
// Character classes

#[derive(Clone, Debug)]
enum CItem {
    Range(u32, u32),
    Prop(Prop, bool),
    Class(Box<CClass>),
}

#[derive(Clone, Debug)]
struct CClass {
    neg: bool,
    icase: bool,
    parts: Vec<Vec<CItem>>, // intersection of unions
    bits: [u64; 2],         // precomputed result for ASCII
}

impl CClass {
    fn raw(&self, c: u32) -> bool {
        for part in &self.parts {
            if part.is_empty() {
                continue;
            }
            let hit = part.iter().any(|it| match it {
                CItem::Range(a, b) => c >= *a && c <= *b,
                CItem::Prop(p, n) => prop_match(*p, c) != *n,
                CItem::Class(k) => k.test(c),
            });
            if !hit {
                return false;
            }
        }
        true
    }

    fn test_slow(&self, c: u32) -> bool {
        let mut h = self.raw(c);
        if !h && self.icase && c < BAD {
            for v in [fold(c), lower1(c), upper1(c)] {
                if v != c && self.raw(v) {
                    h = true;
                    break;
                }
            }
        }
        h != self.neg
    }

    #[inline]
    fn test(&self, c: u32) -> bool {
        if c < 128 { (self.bits[(c >> 6) as usize] >> (c & 63)) & 1 != 0 } else { self.test_slow(c) }
    }

    #[inline]
    fn bit(&self, b: u8) -> bool {
        (self.bits[(b >> 6) as usize] >> (b & 63)) & 1 != 0
    }

    fn finalize(&mut self, icase: bool) {
        self.icase = icase;
        for part in self.parts.iter_mut() {
            for it in part.iter_mut() {
                if let CItem::Class(k) = it {
                    k.finalize(icase);
                }
            }
        }
        self.bits = [0; 2];
        for c in 0..128u32 {
            if self.test_slow(c) {
                self.bits[(c >> 6) as usize] |= 1 << (c & 63);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// AST

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum As {
    Bol,
    Eol,
    StrBegin,
    StrEnd,
    StrEndNl,
    WordB,
    NotWordB,
    Gpos,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    Greedy,
    Lazy,
    Possessive,
}

#[derive(Clone, Debug)]
enum BRef {
    Num(usize),
    BadCond(Box<BRef>), // conditional with more than two branches; reported after the ref is checked
    Name(String),
    Groups(Vec<usize>),
}

#[derive(Clone, Debug)]
enum Node {
    Empty,
    Char(u32, bool),
    Any(bool),
    Class(Box<CClass>),
    Seq(Vec<Node>),
    Alt(Vec<Node>),
    Group(usize, Box<Node>),
    Repeat(Box<Node>, u32, u32, Kind),
    Assert(As),
    Look(bool, bool, Box<Node>), // (behind, negative, body)
    Atomic(Box<Node>),
    Backref(BRef, bool),
    Keep,
    Grapheme,
    Cond(BRef, Box<Node>, Box<Node>),
}

// ---------------------------------------------------------------------------
// Parser

struct Parser<'a> {
    p: &'a [u8],
    i: usize,
    icase: bool,
    ext: bool,
    ml: bool,
    ngroups: usize,
    group_names: Vec<Option<String>>, // by parse-order group number - 1
    pending_u_list: bool,             // \u{61 62}: more code points follow
    u_list_prefix: Vec<Node>,         // leading code points of a \u{...} list, emitted before the atom
}

enum Lit {
    Ch(u32),
    Item(CItem),
}

fn err<T>(m: &str) -> Result<T, String> {
    Err(m.to_string())
}

impl<'a> Parser<'a> {
    #[inline]
    fn peek(&self) -> Option<u8> {
        self.p.get(self.i).copied()
    }
    #[inline]
    fn peek_at(&self, k: usize) -> Option<u8> {
        self.p.get(self.i + k).copied()
    }
    fn at(&self, s: &[u8]) -> bool {
        self.p[self.i..].starts_with(s)
    }

    fn skip_ext(&mut self) {
        if !self.ext {
            return;
        }
        while let Some(c) = self.peek() {
            match c {
                b' ' | b'\t' | b'\n' | b'\r' | 0x0c | 0x0b => self.i += 1,
                b'#' => {
                    while let Some(c) = self.peek() {
                        self.i += 1;
                        if c == b'\n' {
                            break;
                        }
                    }
                }
                _ => break,
            }
        }
    }

    fn parse_alt(&mut self) -> Result<Node, String> {
        let mut alts = vec![self.parse_seq()?];
        while self.peek() == Some(b'|') {
            self.i += 1;
            alts.push(self.parse_seq()?);
        }
        Ok(if alts.len() == 1 { alts.pop().unwrap() } else { Node::Alt(alts) })
    }

    fn parse_seq(&mut self) -> Result<Node, String> {
        let mut items: Vec<Node> = Vec::new();
        loop {
            self.skip_ext();
            let c = match self.peek() {
                None => break,
                Some(c) => c,
            };
            let atom = match c {
                b'|' | b')' => break,
                b'*' | b'+' | b'?' => return err("target of repeat operator is not specified"),
                b'{' => {
                    if self.try_brace()?.is_some() {
                        return err("target of repeat operator is not specified");
                    }
                    self.i += 1;
                    Node::Char(b'{' as u32, false)
                }
                b'(' => {
                    if self.at(b"(?#") {
                        self.i += 3;
                        loop {
                            match self.peek() {
                                None => return err("end pattern in group"),
                                Some(b')') => {
                                    self.i += 1;
                                    break;
                                }
                                Some(b'\\') => self.i += 2,
                                _ => self.i += 1,
                            }
                        }
                        continue;
                    }
                    if self.at(b"(?") && matches!(self.peek_at(2), Some(b'i' | b'm' | b'x' | b'-' | b'a' | b'd' | b'u')) {
                        self.i += 2;
                        let (ic, ex, ml) = (self.icase, self.ext, self.ml);
                        let mut on = true;
                        let term;
                        loop {
                            match self.peek() {
                                None => return err("end pattern in group"),
                                Some(b'i') => self.icase = on,
                                Some(b'm') => self.ml = on,
                                Some(b'x') => self.ext = on,
                                Some(b'a' | b'd' | b'u') => {}
                                Some(b'-') => on = false,
                                Some(b':') => {
                                    term = b':';
                                    self.i += 1;
                                    break;
                                }
                                Some(b')') => {
                                    term = b')';
                                    self.i += 1;
                                    break;
                                }
                                _ => return err("undefined group option"),
                            }
                            self.i += 1;
                        }
                        if term == b')' {
                            // (?i) extends to the end of the enclosing group, alternatives included.
                            let rest = self.parse_alt()?;
                            items.push(rest);
                            break;
                        }
                        let inner = self.parse_alt()?;
                        if self.peek() != Some(b')') {
                            return err("end pattern with unmatched parenthesis");
                        }
                        self.i += 1;
                        self.icase = ic;
                        self.ext = ex;
                        self.ml = ml;
                        inner
                    } else {
                        self.parse_group()?
                    }
                }
                b'[' => {
                    self.i += 1;
                    let mut cc = self.parse_class()?;
                    cc.finalize(self.icase);
                    Node::Class(Box::new(cc))
                }
                b'.' => {
                    self.i += 1;
                    Node::Any(self.ml)
                }
                b'^' => {
                    self.i += 1;
                    Node::Assert(As::Bol)
                }
                b'$' => {
                    self.i += 1;
                    Node::Assert(As::Eol)
                }
                b'\\' => {
                    self.i += 1;
                    self.parse_escape()?
                }
                _ => {
                    let (ch, l) = decode(self.p, self.i);
                    self.i += l;
                    self.mk_char(ch)
                }
            };
            items.append(&mut self.u_list_prefix);
            let atom = self.parse_quant(atom)?;
            items.push(atom);
        }
        if items.len() >= 2 {
            items = fold_ss(items);
        }
        Ok(match items.len() {
            0 => Node::Empty,
            1 => items.pop().unwrap(),
            _ => Node::Seq(items),
        })
    }

    /// Ruby resolves named references while parsing: the group must already be defined.
    fn check_name_defined(&self, name: &str) -> Result<(), String> {
        if self.group_names.iter().any(|n| n.as_deref() == Some(name)) {
            Ok(())
        } else {
            Err(format!("undefined name <{}> reference", name))
        }
    }

    fn mk_char(&self, c: u32) -> Node {
        if self.icase && SHARP_S.contains(&c) {
            return sharp_s_node();
        }
        Node::Char(c, self.icase && has_case(c))
    }

    fn read_num(&mut self) -> Option<u32> {
        let st = self.i;
        let mut v: u64 = 0;
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                v = (v * 10 + (c - b'0') as u64).min(u32::MAX as u64 - 1);
                self.i += 1;
            } else {
                break;
            }
        }
        if self.i == st { None } else { Some(v as u32) }
    }

    /// Parses `{n}`, `{n,}`, `{,m}`, `{n,m}` at the current `{`; leaves position unchanged and
    /// returns None when it does not form a quantifier (then `{` is a literal).
    fn try_brace(&mut self) -> Result<Option<(u32, u32)>, String> {
        let save = self.i;
        self.i += 1;
        let a = self.read_num();
        let (min, max);
        if self.peek() == Some(b',') {
            self.i += 1;
            let b = self.read_num();
            if a.is_none() && b.is_none() {
                self.i = save;
                return Ok(None);
            }
            min = a.unwrap_or(0);
            max = b.unwrap_or(INF);
        } else {
            match a {
                None => {
                    self.i = save;
                    return Ok(None);
                }
                Some(n) => {
                    min = n;
                    max = n;
                }
            }
        }
        if self.peek() != Some(b'}') {
            self.i = save;
            return Ok(None);
        }
        self.i += 1;
        if min > MAX_REPEAT || (max != INF && max > MAX_REPEAT) {
            return err("too big number for repeat range");
        }
        if max < min {
            return err("upper is smaller than lower in repeat range");
        }
        Ok(Some((min, max)))
    }

    fn parse_quant(&mut self, mut atom: Node) -> Result<Node, String> {
        loop {
            self.skip_ext();
            let (min, max, brace) = match self.peek() {
                Some(b'*') => {
                    self.i += 1;
                    (0, INF, false)
                }
                Some(b'+') => {
                    self.i += 1;
                    (1, INF, false)
                }
                Some(b'?') => {
                    self.i += 1;
                    (0, 1, false)
                }
                Some(b'{') => match self.try_brace()? {
                    Some((a, b)) => (a, b, true),
                    None => break,
                },
                _ => break,
            };
            let mut kind = Kind::Greedy;
            match self.peek() {
                Some(b'?') => {
                    self.i += 1;
                    if brace && min == max {
                        // Ruby: a{n}? is (?:a{n})?, not a lazy repeat.
                        atom = Node::Repeat(Box::new(Node::Repeat(Box::new(atom), min, max, Kind::Greedy)), 0, 1, Kind::Greedy);
                        continue;
                    }
                    kind = Kind::Lazy;
                }
                Some(b'+') if !brace => {
                    self.i += 1;
                    kind = Kind::Possessive;
                }
                _ => {}
            }
            atom = reduce_repeat(atom, min, max, kind);
        }
        Ok(atom)
    }

    fn read_name(&mut self, close: u8) -> Result<String, String> {
        let st = self.i;
        while let Some(c) = self.peek() {
            if c == close {
                break;
            }
            self.i += 1;
        }
        if self.peek() != Some(close) {
            return err("invalid group name <>");
        }
        let name = String::from_utf8_lossy(&self.p[st..self.i]).into_owned();
        self.i += 1;
        Ok(name)
    }

    fn parse_group(&mut self) -> Result<Node, String> {
        self.i += 1; // '('
        let (ic, ex, ml) = (self.icase, self.ext, self.ml);
        enum G {
            Cap(usize),
            NonCap,
            Look(bool, bool),
            Atomic,
        }
        let g = if self.peek() == Some(b'?') {
            self.i += 1;
            match self.peek() {
                Some(b':') => {
                    self.i += 1;
                    G::NonCap
                }
                Some(b'=') => {
                    self.i += 1;
                    G::Look(false, false)
                }
                Some(b'!') => {
                    self.i += 1;
                    G::Look(false, true)
                }
                Some(b'>') => {
                    self.i += 1;
                    G::Atomic
                }
                Some(b'~') => return err("absent operator is not supported"),
                Some(b'(') => {
                    self.i += 1;
                    let st = self.i;
                    while let Some(c) = self.peek() {
                        if c == b')' {
                            break;
                        }
                        self.i += 1;
                    }
                    if self.peek() != Some(b')') {
                        return err("invalid conditional pattern");
                    }
                    let raw = String::from_utf8_lossy(&self.p[st..self.i]).into_owned();
                    self.i += 1;
                    let name = if (raw.starts_with('<') && raw.ends_with('>')) || (raw.starts_with('\'') && raw.ends_with('\'')) && raw.len() >= 2 {
                        raw[1..raw.len() - 1].to_string()
                    } else {
                        raw
                    };
                    if name.is_empty() {
                        return err("invalid conditional pattern");
                    }
                    let r = if name.bytes().all(|b| b.is_ascii_digit()) {
                        BRef::Num(name.parse().map_err(|_| "invalid backref number/name".to_string())?)
                    } else {
                        self.check_name_defined(&name)?;
                        BRef::Name(name)
                    };
                    let body = self.parse_alt()?;
                    if self.peek() != Some(b')') {
                        return err("end pattern with unmatched parenthesis");
                    }
                    self.i += 1;
                    self.icase = ic;
                    self.ext = ex;
                    self.ml = ml;
                    let mut r = r;
                    let (yes, no) = match body {
                        Node::Alt(mut v) => {
                            if v.len() > 2 {
                                r = BRef::BadCond(Box::new(r));
                                v.truncate(2);
                            }
                            let n = v.pop().unwrap();
                            (v.pop().unwrap(), n)
                        }
                        b => (b, Node::Empty),
                    };
                    return Ok(Node::Cond(r, Box::new(yes), Box::new(no)));
                }
                Some(b'<') if self.peek_at(1) == Some(b'=') => {
                    self.i += 2;
                    G::Look(true, false)
                }
                Some(b'<') if self.peek_at(1) == Some(b'!') => {
                    self.i += 2;
                    G::Look(true, true)
                }
                Some(q @ (b'<' | b'\'')) => {
                    self.i += 1;
                    let close = if q == b'<' { b'>' } else { b'\'' };
                    let name = self.read_name(close)?;
                    if name.is_empty() {
                        return err("group name is empty");
                    }
                    // Onigmo only rejects names that look like (relative) group numbers.
                    let valid = !matches!(name.as_bytes()[0], b'0'..=b'9' | b'-' | b'+');
                    if !valid {
                        return Err(format!("invalid group name <{}>", name));
                    }
                    self.ngroups += 1;
                    self.group_names.push(Some(name));
                    G::Cap(self.ngroups)
                }
                None => return err("end pattern in group"),
                _ => return err("undefined group option"),
            }
        } else {
            self.ngroups += 1;
            self.group_names.push(None);
            G::Cap(self.ngroups)
        };
        let inner = self.parse_alt()?;
        if self.peek() != Some(b')') {
            return err("end pattern with unmatched parenthesis");
        }
        self.i += 1;
        self.icase = ic;
        self.ext = ex;
        self.ml = ml;
        Ok(match g {
            G::Cap(n) => Node::Group(n, Box::new(inner)),
            G::NonCap => inner,
            G::Look(b, n) => Node::Look(b, n, Box::new(inner)),
            G::Atomic => Node::Atomic(Box::new(inner)),
        })
    }

    fn hex_digits(&mut self, maxn: usize) -> Option<u32> {
        let mut v: u32 = 0;
        let mut n = 0;
        while n < maxn {
            match self.peek() {
                Some(c) if c.is_ascii_hexdigit() => {
                    v = v * 16 + (c as char).to_digit(16).unwrap();
                    self.i += 1;
                    n += 1;
                }
                _ => break,
            }
        }
        if n == 0 { None } else { Some(v) }
    }

    fn octal(&mut self, maxn: usize, init: u32) -> u32 {
        let mut v = init;
        let mut n = 0;
        while n < maxn {
            match self.peek() {
                Some(c @ b'0'..=b'7') => {
                    v = v * 8 + (c - b'0') as u32;
                    self.i += 1;
                    n += 1;
                }
                _ => break,
            }
        }
        v
    }

    /// A raw byte >= 0x80 from \x or octal: try to complete a UTF-8 sequence from following \xHH.
    fn finish_byte(&mut self, b: u8) -> u32 {
        let need = if b & 0xE0 == 0xC0 { 2 } else if b & 0xF0 == 0xE0 { 3 } else if b & 0xF8 == 0xF0 { 4 } else { 1 };
        if need == 1 {
            return BAD + b as u32;
        }
        let save = self.i;
        let mut bytes = vec![b];
        while bytes.len() < need && self.at(b"\\x") {
            self.i += 2;
            match self.hex_digits(2) {
                Some(v) if v & 0xC0 == 0x80 => bytes.push(v as u8),
                _ => break,
            }
        }
        if bytes.len() == need {
            let (c, l) = decode(&bytes, 0);
            if l == need {
                return c;
            }
        }
        self.i = save;
        BAD + b as u32
    }

    fn code_point(&self, v: u32) -> Result<u32, String> {
        if v > 0x10FFFF || (0xD800..=0xDFFF).contains(&v) {
            return err("invalid Unicode range");
        }
        Ok(v)
    }

    /// Escapes that denote a single char or a char-set item (shared by classes and atoms).
    /// `c` is the byte after the backslash, already consumed.
    fn lit_escape(&mut self, c: u8, in_class: bool) -> Result<Lit, String> {
        Ok(Lit::Ch(match c {
            b't' => 9,
            b'n' => 10,
            b'r' => 13,
            b'f' => 12,
            b'v' => 11,
            b'a' => 7,
            b'e' => 0x1b,
            b'd' => return Ok(Lit::Item(CItem::Prop(Prop::Digit, false))),
            b'D' => return Ok(Lit::Item(CItem::Prop(Prop::Digit, true))),
            b'w' => return Ok(Lit::Item(CItem::Prop(Prop::Word, false))),
            b'W' => return Ok(Lit::Item(CItem::Prop(Prop::Word, true))),
            b's' => return Ok(Lit::Item(CItem::Prop(Prop::Space, false))),
            b'S' => return Ok(Lit::Item(CItem::Prop(Prop::Space, true))),
            b'h' => return Ok(Lit::Item(CItem::Prop(Prop::Hex, false))),
            b'H' => return Ok(Lit::Item(CItem::Prop(Prop::Hex, true))),
            b'p' | b'P' => {
                if self.peek() != Some(b'{') {
                    c as u32
                } else {
                    self.i += 1;
                    let mut neg = c == b'P';
                    if self.peek() == Some(b'^') {
                        self.i += 1;
                        neg = !neg;
                    }
                    let st = self.i;
                    while let Some(ch) = self.peek() {
                        if ch == b'}' {
                            break;
                        }
                        self.i += 1;
                    }
                    let name = String::from_utf8_lossy(&self.p[st..self.i]).into_owned();
                    if self.peek() != Some(b'}') {
                        return Err(format!("invalid character property name {{{}}}", name));
                    }
                    self.i += 1;
                    match prop_by_name(&name) {
                        Some(p) => return Ok(Lit::Item(CItem::Prop(p, neg))),
                        None => return Err(format!("invalid character property name {{{}}}", name)),
                    }
                }
            }
            b'x' => {
                let v = self.hex_digits(2).ok_or("invalid hex escape")?;
                if v >= 0x80 { self.finish_byte(v as u8) } else { v }
            }
            b'u' => {
                if self.peek() == Some(b'{') {
                    self.i += 1;
                    while self.peek() == Some(b' ') {
                        self.i += 1;
                    }
                    let v = self.hex_digits(8).ok_or("invalid Unicode escape")?;
                    while self.peek() == Some(b' ') {
                        self.i += 1;
                    }
                    if self.peek() != Some(b'}') {
                        // \u{61 62}: the caller reads the remaining code points.
                        if !self.peek().map_or(false, |c| c.is_ascii_hexdigit()) {
                            return err("invalid Unicode list");
                        }
                        self.pending_u_list = true;
                        return Ok(Lit::Ch(self.code_point(v)?));
                    }
                    self.i += 1;
                    self.code_point(v)?
                } else {
                    let st = self.i;
                    let v = self.hex_digits(4).ok_or("invalid Unicode escape")?;
                    if self.i - st != 4 {
                        return err("invalid Unicode escape");
                    }
                    self.code_point(v)?
                }
            }
            b'c' => {
                let x = self.peek().ok_or("end pattern at control")?;
                self.i += 1;
                if x == b'?' { 0x7f } else { (x & 0x1f) as u32 }
            }
            b'C' => {
                if self.peek() != Some(b'-') {
                    return err("invalid control-code syntax");
                }
                self.i += 1;
                let x = self.peek().ok_or("end pattern at control")?;
                self.i += 1;
                if x == b'?' { 0x7f } else { (x & 0x1f) as u32 }
            }
            b'M' => {
                if self.peek() != Some(b'-') {
                    return err("invalid meta-code syntax");
                }
                // Meta chars are single bytes >= 0x80, which cannot stand alone in a UTF-8 regexp.
                return err("too short escaped multibyte character");
            }
            b'0'..=b'7' if in_class => {
                let v = self.octal(2, (c - b'0') as u32);
                if v >= 0x80 { self.finish_byte(v as u8) } else { v }
            }
            b'b' if in_class => 8,
            _ => {
                self.i -= 1;
                let (ch, l) = decode(self.p, self.i);
                self.i += l;
                ch
            }
        }))
    }

    fn parse_escape(&mut self) -> Result<Node, String> {
        let c = match self.peek() {
            None => return err("too short escape sequence"),
            Some(c) => c,
        };
        self.i += 1;
        Ok(match c {
            b'A' => Node::Assert(As::StrBegin),
            b'z' => Node::Assert(As::StrEnd),
            b'Z' => Node::Assert(As::StrEndNl),
            b'b' => Node::Assert(As::WordB),
            b'B' => Node::Assert(As::NotWordB),
            b'G' => Node::Assert(As::Gpos),
            b'K' => Node::Keep,
            b'X' => Node::Grapheme,
            b'R' => {
                let mut cc = CClass { neg: false, icase: false, parts: vec![vec![CItem::Range(0x0a, 0x0d), CItem::Range(0x85, 0x85), CItem::Range(0x2028, 0x2029)]], bits: [0; 2] };
                cc.finalize(false);
                Node::Atomic(Box::new(Node::Alt(vec![
                    Node::Seq(vec![Node::Char(13, false), Node::Char(10, false)]),
                    Node::Class(Box::new(cc)),
                ])))
            }
            b'k' if matches!(self.peek(), Some(b'<' | b'\'')) => {
                let close = if self.peek() == Some(b'<') { b'>' } else { b'\'' };
                self.i += 1;
                let name = self.read_name(close).map_err(|_| "invalid backref number/name".to_string())?;
                if name.is_empty() {
                    return err("group name is empty");
                }
                let r = if let Some(rest) = name.strip_prefix('-') {
                    let n: usize = rest.parse().map_err(|_| "invalid backref number/name".to_string())?;
                    if n == 0 || n > self.ngroups {
                        return err("invalid backref number/name");
                    }
                    BRef::Num(self.ngroups + 1 - n)
                } else if name.as_bytes()[0].is_ascii_digit() || name.starts_with('+') {
                    let n: usize = name.trim_start_matches('+').parse().map_err(|_| "invalid backref number/name".to_string())?;
                    BRef::Num(n)
                } else {
                    self.check_name_defined(&name)?;
                    BRef::Name(name)
                };
                Node::Backref(r, self.icase)
            }
            b'g' if matches!(self.peek(), Some(b'<' | b'\'')) => return err("subexpression call is not supported"),
            b'1'..=b'9' => {
                let st = self.i - 1;
                self.i = st;
                let n = self.read_num().unwrap() as usize;
                if n <= 9 || n <= self.ngroups {
                    Node::Backref(BRef::Num(n), self.icase)
                } else {
                    self.i = st;
                    if c >= b'8' {
                        self.i += 1;
                        Node::Char(c as u32, false)
                    } else {
                        let v = self.octal(3, 0);
                        let v = if v >= 0x80 { self.finish_byte((v & 0xff) as u8) } else { v };
                        self.mk_char(v)
                    }
                }
            }
            b'0' => {
                let v = self.octal(2, 0);
                let v = if v >= 0x80 { self.finish_byte(v as u8) } else { v };
                self.mk_char(v)
            }
            _ => match self.lit_escape(c, false)? {
                Lit::Ch(v) => {
                    if self.pending_u_list {
                        self.pending_u_list = false;
                        let mut items = vec![self.mk_char(v)];
                        loop {
                            while self.peek() == Some(b' ') {
                                self.i += 1;
                            }
                            if self.peek() == Some(b'}') {
                                self.i += 1;
                                break;
                            }
                            let v = self.hex_digits(8).ok_or("invalid Unicode list")?;
                            items.push(self.mk_char(self.code_point(v)?));
                        }
                        // Like separate literals: a following quantifier binds to the last one.
                        let last = items.pop().unwrap();
                        self.u_list_prefix = items;
                        last
                    } else {
                        self.mk_char(v)
                    }
                }
                Lit::Item(it) => {
                    let mut cc = CClass { neg: false, icase: false, parts: vec![vec![it]], bits: [0; 2] };
                    cc.finalize(self.icase);
                    Node::Class(Box::new(cc))
                }
            },
        })
    }

    fn class_atom(&mut self) -> Result<Lit, String> {
        let c = self.peek().ok_or("premature end of char-class")?;
        if c == b'\\' {
            self.i += 1;
            let e = self.peek().ok_or("premature end of char-class")?;
            self.i += 1;
            return self.lit_escape(e, true);
        }
        let (ch, l) = decode(self.p, self.i);
        self.i += l;
        Ok(Lit::Ch(ch))
    }

    fn try_posix(&mut self) -> Result<Option<CItem>, String> {
        if !self.at(b"[:") {
            return Ok(None);
        }
        let mut j = self.i + 2;
        let neg = self.p.get(j) == Some(&b'^');
        if neg {
            j += 1;
        }
        let st = j;
        while j < self.p.len() && self.p[j].is_ascii_alphabetic() {
            j += 1;
        }
        if !self.p[j..].starts_with(b":]") {
            return Ok(None);
        }
        let name = std::str::from_utf8(&self.p[st..j]).unwrap();
        match posix_by_name(name) {
            Some(p) => {
                self.i = j + 2;
                Ok(Some(CItem::Prop(p, neg)))
            }
            None => err("invalid POSIX bracket type"),
        }
    }

    /// Parses a bracket class; position is just after '['.
    fn parse_class(&mut self) -> Result<CClass, String> {
        let mut neg = false;
        if self.peek() == Some(b'^') {
            self.i += 1;
            neg = true;
        }
        let mut parts: Vec<Vec<CItem>> = vec![vec![]];
        let mut first = true;
        loop {
            let c = self.peek().ok_or("premature end of char-class")?;
            if c == b']' {
                if first {
                    if self.p[self.i + 1..].contains(&b']') {
                        self.i += 1;
                        first = false;
                        parts.last_mut().unwrap().push(CItem::Range(b']' as u32, b']' as u32));
                        continue;
                    }
                    return err("empty char-class");
                }
                self.i += 1;
                break;
            }
            first = false;
            if c == b'&' && self.peek_at(1) == Some(b'&') {
                self.i += 2;
                parts.push(vec![]);
                continue;
            }
            if c == b'[' {
                if let Some(it) = self.try_posix()? {
                    parts.last_mut().unwrap().push(it);
                    continue;
                }
                self.i += 1;
                let sub = self.parse_class()?;
                parts.last_mut().unwrap().push(CItem::Class(Box::new(sub)));
                continue;
            }
            match self.class_atom()? {
                Lit::Item(it) => {
                    if self.peek() == Some(b'-') && !matches!(self.peek_at(1), Some(b']') | None) && !(self.peek_at(1) == Some(b'&') && self.peek_at(2) == Some(b'&')) && self.peek_at(1) != Some(b'[') {
                        return err("char-class value at end of range");
                    }
                    parts.last_mut().unwrap().push(it)
                }
                Lit::Ch(lo) if self.pending_u_list => {
                    self.pending_u_list = false;
                    let part = parts.last_mut().unwrap();
                    part.push(CItem::Range(lo, lo));
                    loop {
                        while self.peek() == Some(b' ') {
                            self.i += 1;
                        }
                        if self.peek() == Some(b'}') {
                            self.i += 1;
                            break;
                        }
                        let v = self.hex_digits(8).ok_or("invalid Unicode list")?;
                        let v = self.code_point(v)?;
                        part.push(CItem::Range(v, v));
                    }
                }
                Lit::Ch(lo) => {
                    if self.peek() == Some(b'-') && !matches!(self.peek_at(1), Some(b']') | None | Some(b'['))
                        && !(self.peek_at(1) == Some(b'&') && self.peek_at(2) == Some(b'&'))
                    {
                        self.i += 1;
                        match self.class_atom()? {
                            Lit::Ch(hi) => {
                                if hi < lo {
                                    return err("empty range in char class");
                                }
                                parts.last_mut().unwrap().push(CItem::Range(lo, hi));
                            }
                            Lit::Item(_) => return err("char-class value at end of range"),
                        }
                    } else {
                        parts.last_mut().unwrap().push(CItem::Range(lo, lo));
                    }
                }
            }
        }
        Ok(CClass { neg, icase: false, parts, bits: [0; 2] })
    }
}

/// Case-insensitive ß: matches ß, ẞ, or "ss" in any case.
fn sharp_s_node() -> Node {
    let mut cc = CClass { neg: false, icase: false, parts: vec![vec![CItem::Range(0xDF, 0xDF), CItem::Range(0x1E9E, 0x1E9E)]], bits: [0; 2] };
    cc.finalize(false);
    Node::Alt(vec![Node::Class(Box::new(cc)), Node::Seq(vec![Node::Char('s' as u32, true), Node::Char('s' as u32, true)])])
}

/// Under /i a literal "ss" also matches ß (multi-char case folding).
fn fold_ss(items: Vec<Node>) -> Vec<Node> {
    let is_s = |n: &Node| matches!(n, Node::Char(c, true) if *c == 's' as u32 || *c == 'S' as u32);
    let mut out = Vec::with_capacity(items.len());
    let mut it = items.into_iter().peekable();
    while let Some(n) = it.next() {
        if is_s(&n) && it.peek().map_or(false, |x| is_s(x)) {
            let n2 = it.next().unwrap();
            let mut cc = CClass { neg: false, icase: false, parts: vec![vec![CItem::Range(0xDF, 0xDF), CItem::Range(0x1E9E, 0x1E9E)]], bits: [0; 2] };
            cc.finalize(false);
            out.push(Node::Alt(vec![Node::Seq(vec![n, n2]), Node::Class(Box::new(cc))]));
        } else {
            out.push(n);
        }
    }
    out
}

/// Onigmo-style reduction of nested simple quantifiers (e.g. (?:a?)* -> a*).
fn reduce_repeat(atom: Node, min: u32, max: u32, kind: Kind) -> Node {
    if let Node::Repeat(ref b, imin, imax, ik) = atom {
        let simple = |a: u32, m: u32| (a == 0 || a == 1) && (m == 1 || m == INF) && !(a == 1 && m == 1);
        if ik == kind && kind != Kind::Possessive && simple(imin, imax) && simple(min, max) {
            let (nmin, nmax) = match ((imin, imax), (min, max)) {
                ((0, 1), (0, 1)) => (0, 1),
                ((1, INF), (1, INF)) => (1, INF),
                _ => (0, INF),
            };
            return Node::Repeat(b.clone(), nmin, nmax, kind);
        }
    }
    Node::Repeat(Box::new(atom), min, max, kind)
}

impl<'a> Parser<'a> {
    fn new(p: &'a [u8], opts: u32) -> Parser<'a> {
        Parser {
            p,
            i: 0,
            icase: opts & OPT_I != 0,
            ext: opts & OPT_X != 0,
            ml: opts & OPT_M != 0,
            ngroups: 0,
            group_names: Vec::new(),
            pending_u_list: false,
            u_list_prefix: Vec::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Post-parse: renumber groups when named groups exist and resolve backrefs.

struct Resolver<'a> {
    map: Vec<Option<usize>>, // parse-order group -> final group number
    has_named: bool,
    total: usize,
    names: &'a [(String, Vec<usize>)],
}

impl<'a> Resolver<'a> {
    fn walk(&self, n: Node) -> Result<Node, String> {
        Ok(match n {
            Node::Seq(v) => Node::Seq(v.into_iter().map(|x| self.walk(x)).collect::<Result<_, _>>()?),
            Node::Alt(v) => Node::Alt(v.into_iter().map(|x| self.walk(x)).collect::<Result<_, _>>()?),
            Node::Group(g, b) => {
                let inner = self.walk(*b)?;
                match self.map[g - 1] {
                    Some(k) => Node::Group(k, Box::new(inner)),
                    None => inner,
                }
            }
            Node::Repeat(b, a, m, k) => Node::Repeat(Box::new(self.walk(*b)?), a, m, k),
            Node::Look(be, ne, b) => Node::Look(be, ne, Box::new(self.walk(*b)?)),
            Node::Atomic(b) => Node::Atomic(Box::new(self.walk(*b)?)),
            Node::Cond(r, y, n) => {
                let (r, bad) = match r {
                    BRef::BadCond(b) => (*b, true),
                    r => (r, false),
                };
                let r = match self.walk(Node::Backref(r, false))? {
                    Node::Backref(r, _) => r,
                    _ => unreachable!(),
                };
                if bad {
                    return err("invalid conditional pattern");
                }
                Node::Cond(r, Box::new(self.walk(*y)?), Box::new(self.walk(*n)?))
            }
            Node::Backref(r, ic) => {
                let gs = match r {
                    BRef::Num(k) => {
                        if self.has_named {
                            return err("numbered backref/call is not allowed. (use name)");
                        }
                        if k == 0 || k > self.total {
                            return err("invalid backref number/name");
                        }
                        vec![k]
                    }
                    BRef::Name(nm) => match self.names.iter().find(|(x, _)| *x == nm) {
                        Some((_, v)) => v.clone(),
                        None => return Err(format!("undefined name <{}> reference", nm)),
                    },
                    BRef::Groups(v) => v,
                    BRef::BadCond(_) => unreachable!(),
                };
                Node::Backref(BRef::Groups(gs), ic)
            }
            other => other,
        })
    }
}

/// Length range in chars; None when unbounded or not fixed-computable (backrefs, \X).
fn len_range(n: &Node) -> Option<(u32, u32)> {
    Some(match n {
        Node::Empty | Node::Assert(_) | Node::Keep | Node::Look(..) => (0, 0),
        Node::Char(..) | Node::Any(_) | Node::Class(_) => (1, 1),
        Node::Seq(v) => {
            let (mut a, mut b) = (0u32, 0u32);
            for x in v {
                let (p, q) = len_range(x)?;
                a = a.saturating_add(p);
                b = b.saturating_add(q);
            }
            (a, b)
        }
        Node::Alt(v) => {
            let (mut a, mut b) = (u32::MAX, 0u32);
            for x in v {
                let (p, q) = len_range(x)?;
                a = a.min(p);
                b = b.max(q);
            }
            (a, b)
        }
        Node::Group(_, b) | Node::Atomic(b) => len_range(b)?,
        Node::Repeat(b, mn, mx, _) => {
            if *mx == 0 {
                return Some((0, 0));
            }
            let (p, q) = len_range(b)?;
            if *mx == INF {
                if q == 0 {
                    return Some((0, 0));
                }
                return None;
            }
            (p.saturating_mul(*mn), q.saturating_mul(*mx))
        }
        Node::Backref(..) | Node::Grapheme => return None,
        Node::Cond(_, y, n) => {
            let (a, b) = len_range(y)?;
            let (c, d) = len_range(n)?;
            (a.min(c), b.max(d))
        }
    })
}

// ---------------------------------------------------------------------------
// Program

#[derive(Clone, Debug)]
enum Single {
    Ch(u32),
    ChIC(u32),
    Any,
    AnyNL,
    Class(Box<CClass>),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum SubKind {
    Atomic,
    Ahead,
    NegAhead,
    Behind,
    NegBehind,
}

#[derive(Clone, Debug)]
enum Inst {
    Str(Box<[u8]>),
    One(Single),
    Split(usize, usize), // continue at .0, backtrack to .1
    Jmp(usize),
    Save(usize),
    SaveNR(usize), // set a capture slot without restoring it on backtrack
    Assert(As),
    RepChar { m: Single, min: u32, max: u32, greedy: bool, nb: i16 },
    SetReg(usize),
    RegZero(usize),
    CountCheck { reg: usize, min: u32, max: u32, greedy: bool, exit: usize },
    CountInc(usize),
    EmptyCheck { reg: usize, exit: usize, mem: bool },
    Sub { kind: SubKind, end: usize, minlen: u32, maxlen: u32 },
    SubEnd,
    Backref { groups: Box<[usize]>, icase: bool },
    CondJmp { groups: Box<[usize]>, no: usize },
    Keep,
    Grapheme,
    Match,
}

struct Compiler {
    prog: Vec<Inst>,
    nregs: usize,
    bt_depth: usize,
    backrefed: Vec<usize>,
}

fn single_of(n: &Node) -> Option<Single> {
    match n {
        Node::Char(c, false) => Some(Single::Ch(*c)),
        Node::Char(c, true) => Some(Single::ChIC(fold(*c))),
        Node::Any(false) => Some(Single::Any),
        Node::Any(true) => Some(Single::AnyNL),
        Node::Class(cc) => Some(Single::Class(cc.clone())),
        _ => None,
    }
}

impl Compiler {
    fn pc(&self) -> usize {
        self.prog.len()
    }
    fn emit_i(&mut self, i: Inst) -> usize {
        self.prog.push(i);
        self.prog.len() - 1
    }
    fn reg(&mut self) -> usize {
        self.nregs += 1;
        self.nregs - 1
    }

    /// Emits `n` in a context where Onigmo uses backtrackable (push) captures.
    fn emit_bt(&mut self, n: &Node) -> Result<(), String> {
        self.bt_depth += 1;
        let r = self.emit(n);
        self.bt_depth -= 1;
        r
    }

    fn emit(&mut self, n: &Node) -> Result<(), String> {
        match n {
            Node::Empty => {}
            Node::Char(c, false) => {
                let mut v = Vec::new();
                push_char(&mut v, *c);
                self.emit_i(Inst::Str(v.into_boxed_slice()));
            }
            Node::Char(..) | Node::Any(_) | Node::Class(_) => {
                let m = single_of(n).unwrap();
                self.emit_i(Inst::One(m));
            }
            Node::Seq(items) => {
                let mut buf: Vec<u8> = Vec::new();
                for it in items {
                    if let Node::Char(c, false) = it {
                        push_char(&mut buf, *c);
                        continue;
                    }
                    if !buf.is_empty() {
                        self.emit_i(Inst::Str(std::mem::take(&mut buf).into_boxed_slice()));
                    }
                    self.emit(it)?;
                }
                if !buf.is_empty() {
                    self.emit_i(Inst::Str(buf.into_boxed_slice()));
                }
            }
            Node::Alt(alts) => {
                let mut jmps = Vec::new();
                for (k, a) in alts.iter().enumerate() {
                    if k + 1 < alts.len() {
                        let sp = self.emit_i(Inst::Split(0, 0));
                        self.emit_bt(a)?;
                        jmps.push(self.emit_i(Inst::Jmp(0)));
                        let next = self.pc();
                        self.prog[sp] = Inst::Split(sp + 1, next);
                    } else {
                        self.emit_bt(a)?;
                    }
                }
                let end = self.pc();
                for j in jmps {
                    self.prog[j] = Inst::Jmp(end);
                }
            }
            Node::Group(g, b) => {
                // Onigmo pushes (backtrackable) group starts only inside alternation, variable
                // repeat or negative lookaround, or for groups referenced by backrefs.
                if self.bt_depth > 0 || self.backrefed.contains(g) {
                    self.emit_i(Inst::Save(2 * g));
                    self.emit(b)?;
                    self.emit_i(Inst::Save(2 * g + 1));
                } else {
                    self.emit_i(Inst::SaveNR(2 * g));
                    self.emit(b)?;
                    self.emit_i(Inst::SaveNR(2 * g + 1));
                }
            }
            Node::Repeat(b, mn, mx, k) => self.emit_repeat(b, *mn, *mx, *k)?,
            Node::Assert(a) => {
                self.emit_i(Inst::Assert(*a));
            }
            Node::Look(behind, neg, b) => {
                let (minlen, maxlen) = if *behind {
                    len_range(b).ok_or("invalid pattern in look-behind")?
                } else {
                    (0, 0)
                };
                let kind = match (behind, neg) {
                    (false, false) => SubKind::Ahead,
                    (false, true) => SubKind::NegAhead,
                    (true, false) => SubKind::Behind,
                    (true, true) => SubKind::NegBehind,
                };
                if *neg {
                    self.bt_depth += 1;
                }
                self.emit_sub(kind, b, minlen, maxlen)?;
                if *neg {
                    self.bt_depth -= 1;
                }
            }
            Node::Atomic(b) => self.emit_sub(SubKind::Atomic, b, 0, 0)?,
            Node::Backref(BRef::Groups(g), ic) => {
                self.emit_i(Inst::Backref { groups: g.clone().into_boxed_slice(), icase: *ic });
            }
            Node::Cond(BRef::Groups(g), y, no) => {
                let cj = self.emit_i(Inst::CondJmp { groups: g.clone().into_boxed_slice(), no: 0 });
                self.emit_bt(y)?;
                let j = self.emit_i(Inst::Jmp(0));
                let nopc = self.pc();
                self.emit_bt(no)?;
                let end = self.pc();
                self.prog[cj] = Inst::CondJmp { groups: g.clone().into_boxed_slice(), no: nopc };
                self.prog[j] = Inst::Jmp(end);
            }
            Node::Backref(..) | Node::Cond(..) => unreachable!(),
            Node::Keep => {
                self.emit_i(Inst::Keep);
            }
            Node::Grapheme => {
                self.emit_i(Inst::Grapheme);
            }
        }
        Ok(())
    }

    fn emit_sub(&mut self, kind: SubKind, b: &Node, minlen: u32, maxlen: u32) -> Result<(), String> {
        let at = self.emit_i(Inst::Sub { kind, end: 0, minlen, maxlen });
        self.emit(b)?;
        self.emit_i(Inst::SubEnd);
        let end = self.pc();
        self.prog[at] = Inst::Sub { kind, end, minlen, maxlen };
        Ok(())
    }

    fn emit_repeat(&mut self, b: &Node, min: u32, max: u32, kind: Kind) -> Result<(), String> {
        if max == 0 {
            return Ok(());
        }
        if len_range(b) == Some((0, 0)) && !has_any_group(b) {
            // Zero-width body without captures: iterating cannot change anything, so a loop
            // would only duplicate backtracking paths.
            return if min == 0 { Ok(()) } else { self.emit(b) };
        }
        if min != max {
            self.bt_depth += 1;
            let r = self.emit_repeat_inner(b, min, max, kind);
            self.bt_depth -= 1;
            return r;
        }
        self.emit_repeat_inner(b, min, max, kind)
    }

    fn emit_repeat_inner(&mut self, b: &Node, min: u32, max: u32, kind: Kind) -> Result<(), String> {
        if kind == Kind::Possessive {
            let r = Node::Repeat(Box::new(b.clone()), min, max, Kind::Greedy);
            return self.emit_sub(SubKind::Atomic, &r, 0, 0);
        }
        let greedy = kind == Kind::Greedy;
        let mem = has_group(b);
        if let Some(m) = single_of(b) {
            self.emit_i(Inst::RepChar { m, min, max, greedy, nb: -1 });
            return Ok(());
        }
        if min == 1 && max == 1 {
            return self.emit(b);
        }
        if min == 0 && max == 1 {
            let sp = self.emit_i(Inst::Split(0, 0));
            self.emit(b)?;
            let end = self.pc();
            self.prog[sp] = if greedy { Inst::Split(sp + 1, end) } else { Inst::Split(end, sp + 1) };
            return Ok(());
        }
        if max == INF && min == 0 {
            let r = self.reg();
            let top = self.emit_i(Inst::Split(0, 0));
            self.emit_i(Inst::SetReg(r));
            self.emit(b)?;
            let ec = self.emit_i(Inst::EmptyCheck { reg: r, exit: 0, mem });
            self.emit_i(Inst::Jmp(top));
            let exit = self.pc();
            self.prog[ec] = Inst::EmptyCheck { reg: r, exit, mem };
            self.prog[top] = if greedy { Inst::Split(top + 1, exit) } else { Inst::Split(exit, top + 1) };
            return Ok(());
        }
        // Onigmo expands small repeats into plain copies, which are not null-checked; larger
        // ones loop with a null check on every iteration. Its choice depends on the compiled
        // size of the body, approximated by onig_len.
        let tlen = onig_len(b);
        if max == INF && (min == 1 && tlen <= ONIG_EXPAND_LIMIT || min >= 2 && tlen.saturating_mul(min as usize) <= ONIG_EXPAND_LIMIT) {
            for _ in 0..min {
                self.emit(b)?;
            }
            return self.emit_repeat_inner(b, 0, INF, kind);
        }
        if max == INF && min == 1 {
            let r = self.reg();
            let top = self.emit_i(Inst::SetReg(r));
            self.emit(b)?;
            let ec = self.emit_i(Inst::EmptyCheck { reg: r, exit: 0, mem });
            let sp = self.emit_i(Inst::Split(0, 0));
            let exit = self.pc();
            self.prog[ec] = Inst::EmptyCheck { reg: r, exit, mem };
            self.prog[sp] = if greedy { Inst::Split(top, exit) } else { Inst::Split(exit, top) };
            return Ok(());
        }
        let checked = !(max != INF && greedy && (tlen + 5).saturating_mul(max as usize) <= ONIG_EXPAND_LIMIT);
        let c = self.reg();
        let p = self.reg();
        self.emit_i(Inst::RegZero(c));
        let top = self.emit_i(Inst::CountCheck { reg: c, min, max, greedy, exit: 0 });
        self.emit_i(Inst::SetReg(p));
        self.emit(b)?;
        self.emit_i(Inst::CountInc(c));
        let ec = if checked { Some(self.emit_i(Inst::EmptyCheck { reg: p, exit: 0, mem })) } else { None };
        self.emit_i(Inst::Jmp(top));
        let exit = self.pc();
        self.prog[top] = Inst::CountCheck { reg: c, min, max, greedy, exit };
        if let Some(ec) = ec {
            self.prog[ec] = Inst::EmptyCheck { reg: p, exit, mem };
        }
        Ok(())
    }
}

/// Onigmo's QUANTIFIER_EXPAND_LIMIT_SIZE.
const ONIG_EXPAND_LIMIT: usize = 50;

/// Rough size in bytes of Onigmo's compiled code for `n` (compile_length_tree).
fn onig_len(n: &Node) -> usize {
    match n {
        Node::Empty => 0,
        Node::Char(c, _) => 1 + if *c < 0x80 { 1 } else if *c < 0x800 { 2 } else if *c < 0x10000 { 3 } else { 4 },
        Node::Any(_) | Node::Assert(_) | Node::Keep | Node::Grapheme => 1,
        Node::Class(cc) => {
            let simple_word = cc.parts.len() == 1 && cc.parts[0].len() == 1 && matches!(cc.parts[0][0], CItem::Prop(Prop::Word, _));
            let ascii_only = cc.parts.iter().flatten().all(|it| matches!(it, CItem::Range(_, hi) if *hi < 128)) && !cc.neg;
            let single = !cc.neg && cc.parts.len() == 1 && cc.parts[0].len() == 1 && matches!(cc.parts[0][0], CItem::Range(a, b) if a == b);
            if simple_word { 1 } else if single { 2 } else if ascii_only { 33 } else { 80 }
        }
        Node::Seq(v) => {
            let mut total = 0;
            let mut run = 0;
            for x in v {
                if let Node::Char(c, false) = x {
                    run += onig_len(&Node::Char(*c, false)) - 1;
                    continue;
                }
                if run > 0 {
                    total += 1 + run + if run > 5 { 4 } else { 0 };
                    run = 0;
                }
                total += onig_len(x);
            }
            if run > 0 {
                total += 1 + run + if run > 5 { 4 } else { 0 };
            }
            total
        }
        Node::Alt(v) => v.iter().map(onig_len).sum::<usize>() + (v.len() - 1) * 10,
        Node::Group(_, b) => 6 + onig_len(b),
        Node::Repeat(b, mn, mx, _) => {
            let t = onig_len(b);
            if *mx == INF {
                let copies = if *mn == 1 && t > ONIG_EXPAND_LIMIT { 0 } else { *mn as usize };
                let null_check = if first_set(b, &mut [false; 256]) { 6 } else { 0 };
                t.saturating_mul(copies + 1) + 10 + null_check
            } else {
                t.saturating_mul(*mx as usize) + 5 * (*mx - *mn) as usize
            }
        }
        Node::Look(behind, neg, b) => onig_len(b) + match (behind, neg) {
            (false, false) => 2,
            (false, true) => 6,
            (true, false) => 5,
            (true, true) => 10,
        },
        Node::Atomic(b) => 2 + onig_len(b),
        Node::Backref(..) => 3,
        Node::Cond(_, y, n) => 10 + onig_len(y) + onig_len(n),
    }
}

// First-byte analysis: returns whether the node can match empty.
fn first_set(n: &Node, set: &mut [bool; 256]) -> bool {
    match n {
        Node::Empty | Node::Assert(_) | Node::Keep | Node::Look(..) => true,
        Node::Char(c, ic) => {
            set[first_byte(*c) as usize] = true;
            if *ic {
                for v in [fold(*c), lower1(*c), upper1(*c)] {
                    set[first_byte(v) as usize] = true;
                }
                for b in 0x80..256 {
                    set[b] = true;
                }
            }
            false
        }
        Node::Any(ml) => {
            for b in 0..256 {
                if *ml || b != b'\n' as usize {
                    set[b] = true;
                }
            }
            false
        }
        Node::Class(cc) => {
            for b in 0..128u8 {
                if cc.bit(b) {
                    set[b as usize] = true;
                }
            }
            for b in 0x80..256 {
                set[b] = true;
            }
            false
        }
        Node::Seq(v) => {
            for x in v {
                if !first_set(x, set) {
                    return false;
                }
            }
            true
        }
        Node::Alt(v) => {
            let mut e = false;
            for x in v {
                if first_set(x, set) {
                    e = true;
                }
            }
            e
        }
        Node::Group(_, b) | Node::Atomic(b) => first_set(b, set),
        Node::Repeat(b, mn, mx, _) => {
            if *mx == 0 {
                return true;
            }
            let e = first_set(b, set);
            e || *mn == 0
        }
        Node::Backref(..) => {
            for b in 0..256 {
                set[b] = true;
            }
            true
        }
        Node::Grapheme => {
            for b in 0..256 {
                set[b] = true;
            }
            false
        }
        Node::Cond(_, y, n) => {
            let a = first_set(y, set);
            let b = first_set(n, set);
            a || b
        }
    }
}

fn collect_backrefed(n: &Node, out: &mut Vec<usize>) {
    match n {
        Node::Backref(BRef::Groups(g), _) => out.extend_from_slice(g),
        Node::Seq(v) | Node::Alt(v) => v.iter().for_each(|x| collect_backrefed(x, out)),
        Node::Group(_, b) | Node::Repeat(b, ..) | Node::Look(_, _, b) | Node::Atomic(b) => collect_backrefed(b, out),
        Node::Cond(_, y, n) => {
            collect_backrefed(y, out);
            collect_backrefed(n, out);
        }
        _ => {}
    }
}

fn has_any_group(n: &Node) -> bool {
    match n {
        Node::Group(..) => true,
        Node::Seq(v) | Node::Alt(v) => v.iter().any(has_any_group),
        Node::Repeat(b, ..) | Node::Look(_, _, b) | Node::Atomic(b) => has_any_group(b),
        Node::Cond(_, y, n) => has_any_group(y) || has_any_group(n),
        _ => false,
    }
}

fn has_group(n: &Node) -> bool {
    match n {
        Node::Group(..) => true,
        Node::Seq(v) | Node::Alt(v) => v.iter().any(has_group),
        // Like Onigmo's quantifiers_memory_node_info: captures inside lookarounds do not count.
        Node::Repeat(b, _, mx, _) => *mx != 0 && has_group(b),
        Node::Atomic(b) => has_group(b),
        Node::Cond(_, y, n) => has_group(y) || has_group(n),
        _ => false,
    }
}

/// A literal every match must contain: bytes plus whether it compares ASCII-case-insensitively.
type MustLit = (Vec<u8>, bool);

fn lit_char(n: &Node) -> Option<(u32, bool)> {
    match n {
        Node::Char(c, false) => Some((*c, false)),
        Node::Char(c, true) if *c < 128 => Some(((*c as u8).to_ascii_lowercase() as u32, true)),
        _ => None,
    }
}

/// Collects literals every match must contain (lookarounds excluded).
fn must_lits(n: &Node, out: &mut Vec<MustLit>) {
    if let Some((c, ic)) = lit_char(n) {
        let mut v = Vec::new();
        push_char(&mut v, c);
        out.push((v, ic));
        return;
    }
    match n {
        Node::Seq(items) => {
            let mut run: MustLit = (Vec::new(), false);
            for it in items {
                if let Some((c, ic)) = lit_char(it) {
                    if !run.0.is_empty() && run.1 != ic {
                        out.push(std::mem::take(&mut run));
                    }
                    run.1 = ic;
                    push_char(&mut run.0, c);
                    continue;
                }
                if !run.0.is_empty() {
                    out.push(std::mem::take(&mut run));
                }
                must_lits(it, out);
            }
            if !run.0.is_empty() {
                out.push(run);
            }
        }
        Node::Group(_, b) | Node::Atomic(b) => must_lits(b, out),
        Node::Repeat(b, mn, _, _) if *mn >= 1 => must_lits(b, out),
        _ => {}
    }
}

#[inline]
fn lit_eq(a: &[u8], lit: &[u8], ic: bool) -> bool {
    if ic { a.eq_ignore_ascii_case(lit) } else { a == lit }
}

fn find_must(h: &[u8], m: &MustLit, from: usize) -> Option<usize> {
    if !m.1 {
        return find_sub(h, &m.0, from);
    }
    let n = m.0.len();
    (from..(h.len() + 1).saturating_sub(n)).find(|&i| lit_eq(&h[i..i + n], &m.0, true))
}

fn rfind_must(h: &[u8], m: &MustLit) -> Option<usize> {
    if !m.1 {
        return rfind_sub(h, &m.0);
    }
    let n = m.0.len();
    (0..(h.len() + 1).saturating_sub(n)).rev().find(|&i| lit_eq(&h[i..i + n], &m.0, true))
}

fn flatten_head<'a>(n: &'a Node, out: &mut Vec<&'a Node>, backrefed: &[usize]) {
    match n {
        Node::Seq(v) => v.iter().for_each(|x| flatten_head(x, out, backrefed)),
        Node::Group(g, b) if !backrefed.contains(g) => flatten_head(b, out, backrefed),
        Node::Atomic(b) => flatten_head(b, out, backrefed),
        _ => out.push(n),
    }
}

/// Where a match can start, from the pattern head. Also reproduces Onigmo's implicit /m `.*`
/// anchor, which it keeps even after some zero-width prefixes (e.g. /(?=a).*/m only tries the
/// search start).
fn head_anchor(ast: &Node, backrefed: &[usize]) -> Anchor {
    let mut items = Vec::new();
    flatten_head(ast, &mut items, backrefed);
    let star = |n: &Node| match n {
        Node::Repeat(b, 0, INF, k) if *k != Kind::Lazy => match **b {
            Node::Any(ml) => Some(ml),
            _ => None,
        },
        _ => None,
    };
    match items.first() {
        None => return Anchor::None,
        Some(Node::Assert(As::StrBegin)) => return Anchor::StrBegin,
        Some(Node::Assert(As::Gpos)) => return Anchor::Start,
        Some(Node::Assert(As::Bol)) => return Anchor::Line,
        Some(n) => {
            if let Some(ml) = star(n) {
                return if ml { Anchor::Start } else { Anchor::Line };
            }
        }
    }
    let (mut trigger, mut cancel) = (false, false);
    for n in &items {
        let n = match n {
            Node::Repeat(b, mn, _, _) if matches!(**b, Node::Look(..) | Node::Assert(_)) => {
                if *mn == 0 {
                    continue;
                }
                &**b
            }
            n => *n,
        };
        match n {
            Node::Look(false, false, _) | Node::Assert(As::Eol | As::StrEnd | As::StrEndNl) => trigger = true,
            Node::Look(true, false, _) => cancel = true,
            Node::Look(..) | Node::Assert(_) | Node::Keep | Node::Empty => {}
            _ => {
                if star(n) == Some(true) && trigger && !cancel {
                    return Anchor::Start;
                }
                break;
            }
        }
    }
    Anchor::None
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Anchor {
    None,
    StrBegin, // only at 0
    Start,    // only at the search start
    Line,     // at the search start or after "\n"
}

pub struct Regex {
    prog: Vec<Inst>,
    ngroups: usize,
    nregs: usize,
    names: Vec<(String, Vec<usize>)>,
    anchor: Anchor,
    prefix: Option<Box<[u8]>>,
    first: Option<Box<[bool; 256]>>,
    must: Vec<MustLit>,
}

#[derive(Clone, Copy)]
enum Bt {
    Br { pc: usize, pos: usize },
    Slot { idx: usize, old: usize },
    Reg { idx: usize, old: usize },
    Greedy { pc: usize, minpos: usize, pos: usize, nb: i16 },
    Lazy { pc: usize, pos: usize, count: u32 },
    SprevBug { old: usize },
}

struct Ctx<'a> {
    s: &'a [u8],
    slots: Vec<usize>,
    regs: Vec<usize>,
    stack: Vec<Bt>,
    gpos: usize,
    sprev_bug: usize, // position right after an empty backref (see Inst::Backref)
}

#[inline]
fn one(m: &Single, s: &[u8], pos: usize) -> Option<usize> {
    if pos >= s.len() {
        return None;
    }
    let b = s[pos];
    match m {
        Single::Ch(c) => {
            if *c < 128 {
                if b as u32 == *c { Some(pos + 1) } else { None }
            } else {
                let (d, l) = decode(s, pos);
                if d == *c { Some(pos + l) } else { None }
            }
        }
        Single::ChIC(f) => {
            if b < 128 {
                if b.to_ascii_lowercase() as u32 == *f { Some(pos + 1) } else { None }
            } else {
                let (d, l) = decode(s, pos);
                if fold(d) == *f { Some(pos + l) } else { None }
            }
        }
        Single::Any => {
            if b == b'\n' {
                None
            } else if b < 128 {
                Some(pos + 1)
            } else {
                Some(pos + decode(s, pos).1)
            }
        }
        Single::AnyNL => Some(pos + char_len(s, pos)),
        Single::Class(cc) => {
            if b < 128 {
                if cc.bit(b) { Some(pos + 1) } else { None }
            } else {
                let (d, l) = decode(s, pos);
                if cc.test(d) { Some(pos + l) } else { None }
            }
        }
    }
}

/// Greedily consumes up to `rem` more chars matching `m`.
#[inline]
fn scan_greedy(m: &Single, s: &[u8], mut p: usize, rem: u32) -> usize {
    let len = s.len();
    if rem == INF {
        match m {
            Single::Any => {
                return match s[p..].iter().position(|&b| b == b'\n') {
                    Some(k) => p + k,
                    None => len,
                };
            }
            Single::AnyNL => return len,
            Single::Ch(c) if *c < 128 => {
                let c = *c as u8;
                while p < len && s[p] == c {
                    p += 1;
                }
                return p;
            }
            Single::Class(cc) => {
                while p < len {
                    let b = s[p];
                    if b < 128 {
                        if !cc.bit(b) {
                            break;
                        }
                        p += 1;
                    } else {
                        let (d, l) = decode(s, p);
                        if !cc.test(d) {
                            break;
                        }
                        p += l;
                    }
                }
                return p;
            }
            _ => {}
        }
    }
    let mut n = 0u32;
    while n < rem {
        match one(m, s, p) {
            Some(np) => {
                p = np;
                n += 1;
            }
            None => break,
        }
    }
    p
}

/// Largest q in [lo, hi] with s[q] == b.
#[inline]
fn find_back(s: &[u8], lo: usize, hi: usize, b: u8) -> Option<usize> {
    let hi = hi.min(s.len().wrapping_sub(1));
    if s.is_empty() || hi < lo {
        return None;
    }
    s[lo..=hi].iter().rposition(|&x| x == b).map(|k| lo + k)
}

fn find_sub(h: &[u8], n: &[u8], from: usize) -> Option<usize> {
    if n.is_empty() {
        return Some(from);
    }
    let f = n[0];
    let mut i = from;
    while i + n.len() <= h.len() {
        match h[i..h.len() - n.len() + 1].iter().position(|&b| b == f) {
            None => return None,
            Some(k) => {
                i += k;
                if &h[i..i + n.len()] == n {
                    return Some(i);
                }
                i += 1;
            }
        }
    }
    None
}

fn rfind_sub(h: &[u8], n: &[u8]) -> Option<usize> {
    if n.len() > h.len() {
        return None;
    }
    let l = *n.last().unwrap();
    let mut end = h.len();
    while end >= n.len() {
        match h[n.len() - 1..end].iter().rposition(|&b| b == l) {
            None => return None,
            Some(k) => {
                let e = n.len() - 1 + k + 1;
                if &h[e - n.len()..e] == n {
                    return Some(e - n.len());
                }
                end = e - 1;
            }
        }
    }
    None
}

#[inline]
fn word_at(s: &[u8], pos: usize) -> bool {
    pos < s.len() && is_uword(decode(s, pos).0)
}

#[inline]
fn word_before(s: &[u8], pos: usize) -> bool {
    pos > 0 && is_uword(decode(s, prev_start(s, pos, 0)).0)
}

fn is_extender(c: u32) -> bool {
    is_mark(c) || c == 0x200C || (0xE0020..=0xE007F).contains(&c)
}

fn grapheme(s: &[u8], pos: usize) -> Option<usize> {
    let len = s.len();
    if pos >= len {
        return None;
    }
    if s[pos] == b'\r' {
        return Some(if pos + 1 < len && s[pos + 1] == b'\n' { pos + 2 } else { pos + 1 });
    }
    let (c0, l) = decode(s, pos);
    let mut p = pos + l;
    if (0x1F1E6..=0x1F1FF).contains(&c0) && p < len {
        let (c1, l1) = decode(s, p);
        if (0x1F1E6..=0x1F1FF).contains(&c1) {
            p += l1;
        }
    }
    while p < len {
        let (c, l) = decode(s, p);
        if is_extender(c) {
            p += l;
        } else if c == 0x200D {
            p += l;
            if p < len {
                p += decode(s, p).1;
            }
        } else {
            break;
        }
    }
    Some(p)
}

impl Regex {
    pub fn new(pat: &[u8], opts: u32) -> Result<Regex, String> {
        let mut ps = Parser::new(pat, opts);
        let ast = ps.parse_alt()?;
        if ps.i < pat.len() {
            // Only an unmatched ')' can stop the top-level parse early.
            return err("unmatched close parenthesis");
        }
        let has_named = ps.group_names.iter().any(|x| x.is_some());
        let mut map = Vec::with_capacity(ps.group_names.len());
        let mut names: Vec<(String, Vec<usize>)> = Vec::new();
        let mut k = 0;
        for g in &ps.group_names {
            match g {
                Some(nm) => {
                    k += 1;
                    map.push(Some(k));
                    match names.iter_mut().find(|(x, _)| x == nm) {
                        Some(e) => e.1.push(k),
                        None => names.push((nm.clone(), vec![k])),
                    }
                }
                None => {
                    if has_named {
                        map.push(None);
                    } else {
                        k += 1;
                        map.push(Some(k));
                    }
                }
            }
        }
        let ngroups = k;
        let ast = Resolver { map, has_named, total: ngroups, names: &names }.walk(ast)?;
        check_lookbehind(&ast, false)?;

        let mut backrefed = Vec::new();
        collect_backrefed(&ast, &mut backrefed);
        let c_backrefed = backrefed.clone();
        let mut c = Compiler { prog: Vec::new(), nregs: 0, bt_depth: 0, backrefed };
        c.emit(&ast)?;
        c.emit_i(Inst::Match);
        let mut prog = c.prog;
        // Peephole: a single-char loop followed by a literal only stops where that literal's first byte is.
        for pc in 0..prog.len() {
            if let Inst::RepChar { .. } = prog[pc] {
                let mut q = pc + 1;
                while let Inst::Save(_) | Inst::SaveNR(_) = prog[q] {
                    q += 1;
                }
                let b = if let Inst::Str(bytes) = &prog[q] { bytes[0] as i16 } else { -1 };
                if let Inst::RepChar { nb, .. } = &mut prog[pc] {
                    *nb = b;
                }
            }
        }

        let mut q = 0;
        while let Inst::Save(_) | Inst::SaveNR(_) = prog[q] {
            q += 1;
        }
        let anchor = head_anchor(&ast, &c_backrefed);
        let prefix = match &prog[q] {
            Inst::Str(b) => Some(b.clone()),
            _ => None,
        };
        let mut set = [false; 256];
        let first = if prefix.is_none() && !first_set(&ast, &mut set) { Some(Box::new(set)) } else { None };
        let mut ml = Vec::new();
        must_lits(&ast, &mut ml);
        ml.sort();
        ml.dedup();
        ml.sort_by_key(|m| std::cmp::Reverse(m.0.len()));
        ml.truncate(4);
        let must = ml;
        Ok(Regex { prog, ngroups, nregs: c.nregs, names, anchor, prefix, first, must })
    }

    pub fn group_count(&self) -> usize {
        self.ngroups
    }

    pub fn names(&self) -> Vec<(String, Vec<usize>)> {
        self.names.clone()
    }

    pub fn group_index(&self, name: &str) -> Vec<usize> {
        self.names.iter().find(|(n, _)| n == name).map(|(_, v)| v.clone()).unwrap_or_default()
    }

    fn ctx<'a>(&self, s: &'a [u8], gpos: usize) -> Ctx<'a> {
        Ctx { s, slots: vec![NONE; 2 * (self.ngroups + 1)], regs: vec![0; self.nregs], stack: Vec::new(), gpos, sprev_bug: NONE }
    }

    fn try_at(&self, cx: &mut Ctx, p: usize) -> Option<Vec<Option<(usize, usize)>>> {
        for x in cx.slots.iter_mut() {
            *x = NONE;
        }
        cx.stack.clear();
        cx.sprev_bug = NONE;
        cx.slots[0] = p;
        let e = self.run(cx, 0, p, NONE)?;
        cx.slots[1] = e;
        if cx.slots[0] > e {
            cx.slots[0] = e; // \K inside a lookahead can point past the match end
        }
        cx.stack.clear();
        let mut out = Vec::with_capacity(self.ngroups + 1);
        for g in 0..=self.ngroups {
            let (a, b) = (cx.slots[2 * g], cx.slots[2 * g + 1]);
            out.push(if a != NONE && b != NONE && a <= b { Some((a, b)) } else { None });
        }
        Some(out)
    }

    pub fn match_at(&self, s: &[u8], pos: usize) -> Option<Vec<Option<(usize, usize)>>> {
        if pos > s.len() {
            return None;
        }
        for m in &self.must {
            if find_must(s, m, pos).is_none() {
                return None;
            }
        }
        let mut cx = self.ctx(s, pos);
        self.try_at(&mut cx, pos)
    }

    pub fn search(&self, s: &[u8], start: usize) -> Option<Vec<Option<(usize, usize)>>> {
        let len = s.len();
        if start > len {
            return None;
        }
        // A match starting at p contains the required literal at some offset >= p.
        let mut last_must = len;
        for m in &self.must {
            match rfind_must(&s[start..], m) {
                Some(k) => last_must = last_must.min(start + k),
                None => return None,
            }
        }
        let mut cx = self.ctx(s, start);
        let mut p = start;
        loop {
            if p > last_must {
                return None;
            }
            match self.anchor {
                Anchor::StrBegin => {
                    if p != 0 {
                        return None;
                    }
                }
                Anchor::Start => {
                    if p != start {
                        return None;
                    }
                }
                Anchor::Line => {
                    if p != start && s[p - 1] != b'\n' {
                        match s[p..].iter().position(|&b| b == b'\n') {
                            Some(k) => p += k + 1,
                            None => return None,
                        }
                    }
                }
                Anchor::None => {
                    if let Some(pre) = &self.prefix {
                        match find_sub(s, pre, p) {
                            Some(q) => p = q,
                            None => return None,
                        }
                    } else if let Some(fs) = &self.first {
                        while p < len && !fs[s[p] as usize] {
                            p += char_len(s, p);
                        }
                        if p >= len {
                            return None;
                        }
                    }
                }
            }
            if let Some(r) = self.try_at(&mut cx, p) {
                return Some(r);
            }
            if p >= len {
                return None;
            }
            p += char_len(s, p);
        }
    }

    #[inline]
    fn word_prev(&self, s: &[u8], pos: usize, sprev_bug: usize) -> bool {
        if pos == sprev_bug && pos > 0 { word_at(s, pos) } else { word_before(s, pos) }
    }

    fn assert(&self, a: As, s: &[u8], pos: usize, gpos: usize, sprev_bug: usize) -> bool {
        let len = s.len();
        match a {
            As::Bol => pos == 0 || (pos < len && s[pos - 1] == b'\n'),
            As::Eol => pos == len || s[pos] == b'\n',
            As::StrBegin => pos == 0,
            As::StrEnd => pos == len,
            As::StrEndNl => pos == len || (pos + 1 == len && s[pos] == b'\n'),
            As::WordB => self.word_prev(s, pos, sprev_bug) != word_at(s, pos),
            As::NotWordB => self.word_prev(s, pos, sprev_bug) == word_at(s, pos),
            As::Gpos => pos == gpos,
        }
    }

    /// Onigmo's NULL_CHECK_END_MEMST for an iteration that consumed nothing, judged by the span
    /// each group had before being opened in it. 0: exit loop, 1: keep looping, 2: fail.
    fn empty_iter_mem(&self, cx: &Ctx, reg: usize, pos: usize) -> u8 {
        let ncommit = 2 * (self.ngroups + 1);
        let st = cx.stack.iter().rposition(|e| matches!(*e, Bt::Reg { idx, .. } if idx == reg)).map_or(0, |k| k + 1);
        let mut res = 0;
        let mut k = st;
        while k + 1 < cx.stack.len() {
            if let (Bt::Slot { idx: i0, old: os }, Bt::Slot { idx: i1, old: oe }) = (cx.stack[k], cx.stack[k + 1]) {
                if i0 >= 2 && i0 < ncommit && i0 & 1 == 0 && i1 == i0 + 1 {
                    k += 2;
                    if cx.slots[i0] == os && cx.slots[i1] == oe {
                        continue; // unchanged (e.g. set inside a lookahead): treating it as progress could loop forever
                    }
                    if oe == NONE || os != oe {
                        return 1;
                    }
                    if oe != pos {
                        res = 2;
                    }
                    continue;
                }
            }
            k += 1;
        }
        res
    }

    fn compact(cx: &mut Ctx, base: usize) {
        let mut w = base;
        for r in base..cx.stack.len() {
            if matches!(cx.stack[r], Bt::Slot { .. } | Bt::Reg { .. } | Bt::SprevBug { .. }) {
                cx.stack[w] = cx.stack[r];
                w += 1;
            }
        }
        cx.stack.truncate(w);
    }

    fn unwind(cx: &mut Ctx, base: usize) {
        while cx.stack.len() > base {
            match cx.stack.pop().unwrap() {
                Bt::Slot { idx, old } => cx.slots[idx] = old,
                Bt::Reg { idx, old } => cx.regs[idx] = old,
                Bt::SprevBug { old } => cx.sprev_bug = old,
                _ => {}
            }
        }
    }

    fn run(&self, cx: &mut Ctx, mut pc: usize, mut pos: usize, target: usize) -> Option<usize> {
        let base = cx.stack.len();
        let s = cx.s;
        let len = s.len();
        loop {
            let ok = match &self.prog[pc] {
                Inst::Str(b) => {
                    if len - pos >= b.len() && s[pos..pos + b.len()] == b[..] {
                        pos += b.len();
                        pc += 1;
                        true
                    } else {
                        false
                    }
                }
                Inst::One(m) => match one(m, s, pos) {
                    Some(np) => {
                        pos = np;
                        pc += 1;
                        true
                    }
                    None => false,
                },
                Inst::Split(a, b) => {
                    cx.stack.push(Bt::Br { pc: *b, pos });
                    pc = *a;
                    true
                }
                Inst::Jmp(a) => {
                    pc = *a;
                    true
                }
                Inst::Save(k) => {
                    // Onigmo MEMORY_START_PUSH / MEMORY_END: opening saves the old span (restored
                    // on backtrack) and unsets the end; closing is never undone, so a backtrack
                    // inside the group leaves a stale end visible to backrefs.
                    if k & 1 == 0 {
                        cx.stack.push(Bt::Slot { idx: *k, old: cx.slots[*k] });
                        cx.stack.push(Bt::Slot { idx: k + 1, old: cx.slots[k + 1] });
                        cx.slots[k + 1] = NONE;
                    }
                    cx.slots[*k] = pos;
                    pc += 1;
                    true
                }
                Inst::SaveNR(k) => {
                    // Onigmo MEMORY_START (group outside alternation/repeat and never backrefed).
                    if k & 1 == 0 {
                        cx.slots[k + 1] = NONE;
                    }
                    cx.slots[*k] = pos;
                    pc += 1;
                    true
                }
                Inst::Assert(a) => {
                    if self.assert(*a, s, pos, cx.gpos, cx.sprev_bug) {
                        pc += 1;
                        true
                    } else {
                        false
                    }
                }
                Inst::RepChar { m, min, max, greedy, nb } => {
                    let mut p = pos;
                    let mut cnt = 0u32;
                    while cnt < *min {
                        match one(m, s, p) {
                            Some(np) => {
                                p = np;
                                cnt += 1;
                            }
                            None => break,
                        }
                    }
                    if cnt < *min {
                        false
                    } else if *greedy {
                        let minpos = p;
                        let rem = if *max == INF { INF } else { *max - *min };
                        let maxpos = scan_greedy(m, s, p, rem);
                        let cand = if *nb >= 0 { find_back(s, minpos, maxpos, *nb as u8) } else { Some(maxpos) };
                        match cand {
                            Some(q) => {
                                if q > minpos {
                                    cx.stack.push(Bt::Greedy { pc: pc + 1, minpos, pos: q, nb: *nb });
                                }
                                pos = q;
                                pc += 1;
                                true
                            }
                            None => false,
                        }
                    } else {
                        match lazy_advance(m, s, p, cnt, *max, *nb) {
                            Some((q, c2)) => {
                                if c2 < *max {
                                    cx.stack.push(Bt::Lazy { pc, pos: q, count: c2 });
                                }
                                pos = q;
                                pc += 1;
                                true
                            }
                            None => false,
                        }
                    }
                }
                Inst::SetReg(r) => {
                    cx.stack.push(Bt::Reg { idx: *r, old: cx.regs[*r] });
                    cx.regs[*r] = pos;
                    pc += 1;
                    true
                }
                Inst::RegZero(r) => {
                    cx.stack.push(Bt::Reg { idx: *r, old: cx.regs[*r] });
                    cx.regs[*r] = 0;
                    pc += 1;
                    true
                }
                Inst::CountCheck { reg, min, max, greedy, exit } => {
                    let c = cx.regs[*reg] as u32;
                    if c < *min {
                        pc += 1;
                    } else if c >= *max {
                        pc = *exit;
                    } else if *greedy {
                        cx.stack.push(Bt::Br { pc: *exit, pos });
                        pc += 1;
                    } else {
                        cx.stack.push(Bt::Br { pc: pc + 1, pos });
                        pc = *exit;
                    }
                    true
                }
                Inst::CountInc(r) => {
                    cx.stack.push(Bt::Reg { idx: *r, old: cx.regs[*r] });
                    cx.regs[*r] += 1;
                    pc += 1;
                    true
                }
                Inst::EmptyCheck { reg, exit, mem } => {
                    if cx.regs[*reg] != pos {
                        pc += 1;
                        true
                    } else if !*mem {
                        pc = *exit;
                        true
                    } else {
                        match self.empty_iter_mem(cx, *reg, pos) {
                            0 => {
                                pc = *exit;
                                true
                            }
                            1 => {
                                pc += 1;
                                true
                            }
                            _ => false,
                        }
                    }
                }
                Inst::Sub { kind, end, minlen, maxlen } => {
                    let b = cx.stack.len();
                    match kind {
                        SubKind::Atomic | SubKind::Ahead => match self.run(cx, pc + 1, pos, NONE) {
                            Some(np) => {
                                Self::compact(cx, b);
                                if *kind == SubKind::Atomic {
                                    pos = np;
                                }
                                pc = *end;
                                true
                            }
                            None => false,
                        },
                        SubKind::NegAhead => {
                            if self.run(cx, pc + 1, pos, NONE).is_some() {
                                Self::unwind(cx, b);
                                false
                            } else {
                                pc = *end;
                                true
                            }
                        }
                        SubKind::Behind | SubKind::NegBehind => {
                            let mut found = false;
                            let mut q = pos;
                            let mut k = 0u32;
                            loop {
                                if k >= *minlen && self.run(cx, pc + 1, q, pos).is_some() {
                                    found = true;
                                    break;
                                }
                                if k >= *maxlen || q == 0 {
                                    break;
                                }
                                q = prev_start(s, q, 0);
                                k += 1;
                            }
                            if *kind == SubKind::Behind {
                                if found {
                                    Self::compact(cx, b);
                                    pc = *end;
                                }
                                found
                            } else if found {
                                Self::unwind(cx, b);
                                false
                            } else {
                                pc = *end;
                                true
                            }
                        }
                    }
                }
                Inst::SubEnd => {
                    if target == NONE || pos == target {
                        return Some(pos);
                    }
                    false
                }
                Inst::Backref { groups, icase } => {
                    let mut hit = None;
                    for &g in groups.iter().rev() {
                        let (a, e) = (cx.slots[2 * g], cx.slots[2 * g + 1]);
                        if a == NONE || e == NONE {
                            continue;
                        }
                        // A stale end before the start compares as empty in Onigmo.
                        let sub = if a > e { &s[0..0] } else { &s[a..e] };
                        if *icase {
                            if let Some(np) = match_ic(s, pos, sub) {
                                hit = Some(np);
                                break;
                            }
                        } else if len - pos >= sub.len() && &s[pos..pos + sub.len()] == sub {
                            hit = Some(pos + sub.len());
                            break;
                        }
                    }
                    match hit {
                        Some(np) => {
                            if np == pos && cx.sprev_bug != pos {
                                // Onigmo leaves its previous-char pointer at the current position
                                // after an empty backref, which changes what \b and \B see next.
                                cx.stack.push(Bt::SprevBug { old: cx.sprev_bug });
                                cx.sprev_bug = pos;
                            }
                            pos = np;
                            pc += 1;
                            true
                        }
                        None => false,
                    }
                }
                Inst::CondJmp { groups, no } => {
                    if groups.iter().any(|&g| cx.slots[2 * g] != NONE && cx.slots[2 * g + 1] != NONE) {
                        pc += 1;
                    } else {
                        pc = *no;
                    }
                    true
                }
                Inst::Keep => {
                    cx.stack.push(Bt::Slot { idx: 0, old: cx.slots[0] });
                    cx.slots[0] = pos;
                    pc += 1;
                    true
                }
                Inst::Grapheme => match grapheme(s, pos) {
                    Some(np) => {
                        pos = np;
                        pc += 1;
                        true
                    }
                    None => false,
                },
                Inst::Match => return Some(pos),
            };
            if ok {
                continue;
            }
            // Backtrack.
            loop {
                if cx.stack.len() == base {
                    return None;
                }
                match cx.stack.pop().unwrap() {
                    Bt::Br { pc: p, pos: q } => {
                        pc = p;
                        pos = q;
                        break;
                    }
                    Bt::Slot { idx, old } => cx.slots[idx] = old,
                    Bt::Reg { idx, old } => cx.regs[idx] = old,
                    Bt::SprevBug { old } => cx.sprev_bug = old,
                    Bt::Greedy { pc: p, minpos, pos: q, nb } => {
                        let np = if nb >= 0 { find_back(s, minpos, q - 1, nb as u8) } else { Some(prev_start(s, q, minpos)) };
                        if let Some(np) = np {
                            if np > minpos {
                                cx.stack.push(Bt::Greedy { pc: p, minpos, pos: np, nb });
                            }
                            pc = p;
                            pos = np;
                            break;
                        }
                    }
                    Bt::Lazy { pc: p, pos: q, count } => {
                        if let Inst::RepChar { m, max, nb, .. } = &self.prog[p] {
                            if let Some(np) = one(m, s, q) {
                                if let Some((r, c2)) = lazy_advance(m, s, np, count + 1, *max, *nb) {
                                    if c2 < *max {
                                        cx.stack.push(Bt::Lazy { pc: p, pos: r, count: c2 });
                                    }
                                    pc = p + 1;
                                    pos = r;
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// From (p, cnt), consumes further chars until the position could start the following literal.
#[inline]
fn lazy_advance(m: &Single, s: &[u8], mut p: usize, mut cnt: u32, max: u32, nb: i16) -> Option<(usize, u32)> {
    if nb >= 0 {
        let b = nb as u8;
        while !(p < s.len() && s[p] == b) {
            if cnt >= max {
                return None;
            }
            p = one(m, s, p)?;
            cnt += 1;
        }
    }
    Some((p, cnt))
}

fn match_ic(s: &[u8], pos: usize, sub: &[u8]) -> Option<usize> {
    let mut i = 0;
    let mut p = pos;
    while i < sub.len() {
        if p >= s.len() {
            return None;
        }
        let (a, la) = decode(sub, i);
        let (b, lb) = decode(s, p);
        if a != b && fold(a) != fold(b) {
            return None;
        }
        i += la;
        p += lb;
    }
    Some(p)
}

fn check_lookbehind(n: &Node, inside: bool) -> Result<(), String> {
    match n {
        Node::Seq(v) | Node::Alt(v) => v.iter().try_for_each(|x| check_lookbehind(x, inside)),
        Node::Group(_, b) | Node::Atomic(b) | Node::Repeat(b, ..) => check_lookbehind(b, inside),
        Node::Cond(_, y, n) => {
            check_lookbehind(y, inside)?;
            check_lookbehind(n, inside)
        }
        Node::Look(behind, _, b) => {
            if inside && !behind {
                return err("invalid pattern in look-behind");
            }
            if *behind && len_range(b).is_none() {
                return err("invalid pattern in look-behind");
            }
            check_lookbehind(b, inside || *behind)
        }
        Node::Backref(..) | Node::Keep if inside => err("invalid pattern in look-behind"),
        _ => Ok(()),
    }
}

// prism's options blob; also included by build.rs (the prelude is parsed at build time).
/// Options for the parse: file path, starting line, and outer local scopes (for eval).
/// `encoding`: the default source encoding (an eval'd String's), empty for UTF-8.
pub struct ParseOpts<'a> { pub filepath: &'a [u8], pub line: i32, pub scopes: Vec<Vec<Vec<u8>>>, pub main_script: bool, pub frozen_string_literal: i8, pub encoding: &'a [u8] }

fn build_options(o: &ParseOpts) -> Vec<u8> {
    let mut d = vec![];
    d.extend((o.filepath.len() as u32).to_le_bytes()); d.extend(o.filepath);
    d.extend(o.line.to_le_bytes());
    d.extend((o.encoding.len() as u32).to_le_bytes()); d.extend(o.encoding);
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
        // forwarding flags: the scope's method took anonymous *, **, & (all three: `...`)
        let has = |n: &[u8]| sc.iter().any(|l| l == n);
        let f = has(b"__fwd_rest") as u8 | (has(b"__fwd_kw") as u8) << 1 | (has(b"__fwd_blk") as u8) << 2;
        d.push(if f == 7 { 0xf } else { f });
        for l in sc { d.extend((l.len() as u32).to_le_bytes()); d.extend(l); }
    }
    d
}

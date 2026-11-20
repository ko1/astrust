// Node execution counts per kind, and call / block counts by shape (cargo feature `nodeprof`, interpreter only).
use std::cell::RefCell;
use std::collections::HashMap;

thread_local! {
    static COUNTS: RefCell<HashMap<&'static str, u64>> = RefCell::new(HashMap::new());
    static SHAPES: RefCell<HashMap<usize, &'static str>> = RefCell::new(HashMap::new());
}

pub fn hit(kind: &'static str) { COUNTS.with(|c| *c.borrow_mut().entry(kind).or_insert(0) += 1); }

/// Count a method or block body under `tag` and what its dump contains: p<params>, +blk (called with a
/// block), E(nv), B(lock literal), R(eturn), Y(ield), leaf (nothing that can call or raise).
pub fn hit_body(tag: &str, body: crate::node::NodePtr, env: bool, nparams: usize, blk: bool) {
    let key = body as usize ^ if blk { 1 } else { 0 };
    let s = SHAPES.with(|m| *m.borrow_mut().entry(key).or_insert_with(|| {
        let mut d = String::new();
        crate::node::dump_node(&mut d, body);
        let has = |pats: &[&str]| pats.iter().any(|p| d.contains(p));
        let leaf = !d.split(|ch: char| !(ch.is_alphanumeric() || ch == '_')).filter(|w| w.starts_with("node_"))
            .any(|w| !matches!(w, "node_method_entry" | "node_nil" | "node_true" | "node_false" | "node_self" | "node_lit" | "node_lvar"
                               | "node_lasgn" | "node_ivar" | "node_block" | "node_line" | "node_if" | "node_and" | "node_or"));
        let mut k = format!("{} p{}", tag, nparams);
        if blk { k.push_str(" +blk"); }
        if env { k.push_str(" E"); }
        if has(&["node_call_blk", "node_lambda", "node_super", "node_call_x", "node_special"]) { k.push_str(" B"); }
        if has(&["node_return"]) { k.push_str(" R"); }
        if has(&["node_yield", "node_block_given"]) { k.push_str(" Y"); }
        if leaf { k.push_str(" leaf"); }
        Box::leak(k.into_boxed_str())
    }));
    hit(s);
}

/// Print the table (most executed first) when SARUBY_NODEPROF is set.
pub fn report() {
    if std::env::var_os("SARUBY_NODEPROF").is_none() { return; }
    COUNTS.with(|c| {
        let mut v: Vec<_> = c.borrow().iter().map(|(k, n)| (*n, *k)).collect();
        v.sort_by(|a, b| b.cmp(a));
        for (n, k) in v { eprintln!("{:>14} {}", n, k); }
    });
}

// This file is auto-generated from node.def by astrustgen. Do not edit.

pub type DispatchFn = fn(&mut Ctx, NodePtr, usize) -> u32;
pub type NodePtr = *mut Node;
pub type NodeList = &'static [NodePtr];

pub struct NodeKind {
    pub name: &'static str,
    pub default_dispatcher: DispatchFn,
    pub hash_func: fn(NodePtr) -> u64,
    pub specializer: fn(&mut SpecCtx, NodePtr) -> String,
    pub dumper: fn(&mut String, NodePtr),
    pub replacer: fn(NodePtr, NodePtr, NodePtr),
    pub no_inline: bool,
}

pub const NF_HAS_HASH: u32 = 1;
pub const NF_SPECIALIZING: u32 = 2;
pub const NF_NO_INLINE: u32 = 4;

#[repr(C)]
pub struct NodeHead {
    pub dispatcher: DispatchFn,
    pub kind: &'static NodeKind,
    pub hash: u64,
    pub flags: u32,

}
#[repr(C)]
pub struct Node { pub head: NodeHead }
#[repr(C)]
pub struct NodeOf<T> { pub head: NodeHead, pub u: T }

pub fn node_allocate<T>(kind: &'static NodeKind, u: T) -> NodePtr {
    let n = Box::into_raw(Box::new(NodeOf { head: NodeHead {
        dispatcher: kind.default_dispatcher, kind, hash: 0,
        flags: if kind.no_inline { NF_NO_INLINE } else { 0 },
        
    }, u })) as NodePtr;
    node_allocated(n);
    n
}

/// Structural (Merkle) hash, cached in the head.
pub fn hash_node(n: NodePtr) -> u64 {
    if n.is_null() { return hash_str("null"); }
    unsafe {
        if (*n).head.flags & NF_HAS_HASH != 0 { return (*n).head.hash; }
        let h = ((*n).head.kind.hash_func)(n);
        (*n).head.hash = h;
        (*n).head.flags |= NF_HAS_HASH;
        h
    }
}
pub fn hash_node_list(ns: NodeList) -> u64 {
    let mut h = hash_u64(ns.len() as u64);
    for x in ns.iter() { h = hash_merge(h, hash_node(*x)); }
    h
}
pub fn dump_node(out: &mut String, n: NodePtr) {
    if n.is_null() { out.push_str("null"); return; }
    unsafe { ((*n).head.kind.dumper)(out, n) }
}
pub fn node_replace_child(parent: NodePtr, old: NodePtr, new: NodePtr) {
    unsafe { ((*parent).head.kind.replacer)(parent, old, new) }
}
/// SD type name for the subtree rooted at n ("Dyn" when it must stay dynamic).
pub fn specialize_node(sc: &mut SpecCtx, n: NodePtr) -> String {
    unsafe {
        if n.is_null() || (*n).head.flags & (NF_NO_INLINE | NF_SPECIALIZING) != 0 { return "Dyn".to_string(); }
        (*n).head.flags |= NF_SPECIALIZING;
        let t = ((*n).head.kind.specializer)(sc, n);
        (*n).head.flags &= !NF_SPECIALIZING;
        t
    }
}
/// Specialize n as a public entry: an exported `SD_<hash>` with the dispatcher's signature.
pub fn specialize_entry(sc: &mut SpecCtx, n: NodePtr) -> String {
    let fname = format!("SD_{:016x}", hash_node(n));
    if sc.emitted.contains(&fname) { return fname; }
    let t = specialize_node(sc, n);
    sc.out.push_str(&format!("#[no_mangle]
pub fn {}(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {{ <{} as Disp>::call(c, n, pos) }}
", fname, t));
    sc.emitted.insert(fname.clone());
    fname
}

/// A dispatcher as a type: `Dyn` calls through the node head, an SD type statically.
pub trait Disp { fn call(c: &mut Ctx, n: NodePtr, pos: usize) -> u32; }
/// A list of children: `DynList` loops over the heads, an SD list type is unrolled.
pub trait DispList {
    fn len(ns: NodeList) -> usize;
    fn nth(c: &mut Ctx, ns: NodeList, i: usize, pos: usize) -> u32;
}
pub struct Dyn;
impl Disp for Dyn {
    #[inline(always)]
    fn call(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 { unsafe { ((*n).head.dispatcher)(c, n, pos) } }
}
pub struct DynList;
impl DispList for DynList {
    #[inline(always)] fn len(ns: NodeList) -> usize { ns.len() }
    #[inline(always)]
    fn nth(c: &mut Ctx, ns: NodeList, i: usize, pos: usize) -> u32 {
        let x = ns[i];
        unsafe { ((*x).head.dispatcher)(c, x, pos) }
    }
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_re_succ_struct {

}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_re_sub_succ_struct {

}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_re_sub_succ_at_struct {

}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_re_byte_struct {
    pub b: u8,
    pub next: NodePtr,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_re_lit_struct {
    pub bytes: Bytes,
    pub next: NodePtr,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_re_one_struct {
    pub kind: u32,
    pub ch: u32,
    pub cls: ClassRef,
    pub next: NodePtr,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_re_grapheme_struct {
    pub next: NodePtr,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_re_rep1_struct {
    pub kind: u32,
    pub ch: u32,
    pub cls: ClassRef,
    pub min: u32,
    pub max: u32,
    pub greedy: bool,
    pub nb: i32,
    pub next: NodePtr,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_re_assert_struct {
    pub kind: u32,
    pub next: NodePtr,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_re_keep_struct {
    pub next: NodePtr,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_re_open_struct {
    pub g: u32,
    pub next: NodePtr,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_re_open_nr_struct {
    pub g: u32,
    pub next: NodePtr,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_re_close_struct {
    pub g: u32,
    pub next: NodePtr,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_re_backref_struct {
    pub groups: Groups,
    pub icase: bool,
    pub next: NodePtr,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_re_cond_struct {
    pub groups: Groups,
    pub yes: NodePtr,
    pub no: NodePtr,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_re_alt_struct {
    pub a: NodePtr,
    pub b: NodePtr,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_re_opt_struct {
    pub body: NodePtr,
    pub skip: NodePtr,
    pub greedy: bool,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_re_rep_struct {
    pub body: NodePtr,
    pub outer: NodePtr,
    pub min: u32,
    pub max: u32,
    pub greedy: bool,
    pub mem: bool,
    pub shape: u32,
    pub cost: u32,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_re_rep_cont_struct {

}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_re_look_struct {
    pub kind: u32,
    pub body: NodePtr,
    pub next: NodePtr,
    pub minlen: u32,
    pub maxlen: u32,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_search_struct {
    pub body: NodePtr,
    pub anchor: u32,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_search_memchr_struct {
    pub body: NodePtr,
    pub b: u8,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_search_memmem_struct {
    pub body: NodePtr,
    pub needle: Bytes,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_search_byteset_struct {
    pub body: NodePtr,
    pub set: ByteSet,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_match_at_struct {
    pub body: NodePtr,
}

#[allow(non_snake_case)]
pub fn ALLOC_node_re_succ() -> NodePtr {
    node_allocate(&KIND_node_re_succ, node_re_succ_struct {  })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_re_sub_succ() -> NodePtr {
    node_allocate(&KIND_node_re_sub_succ, node_re_sub_succ_struct {  })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_re_sub_succ_at() -> NodePtr {
    node_allocate(&KIND_node_re_sub_succ_at, node_re_sub_succ_at_struct {  })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_re_byte(b: u8, next: NodePtr) -> NodePtr {
    node_allocate(&KIND_node_re_byte, node_re_byte_struct { b, next })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_re_lit(bytes: Bytes, next: NodePtr) -> NodePtr {
    node_allocate(&KIND_node_re_lit, node_re_lit_struct { bytes, next })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_re_one(kind: u32, ch: u32, cls: ClassRef, next: NodePtr) -> NodePtr {
    node_allocate(&KIND_node_re_one, node_re_one_struct { kind, ch, cls, next })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_re_grapheme(next: NodePtr) -> NodePtr {
    node_allocate(&KIND_node_re_grapheme, node_re_grapheme_struct { next })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_re_rep1(kind: u32, ch: u32, cls: ClassRef, min: u32, max: u32, greedy: bool, nb: i32, next: NodePtr) -> NodePtr {
    node_allocate(&KIND_node_re_rep1, node_re_rep1_struct { kind, ch, cls, min, max, greedy, nb, next })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_re_assert(kind: u32, next: NodePtr) -> NodePtr {
    node_allocate(&KIND_node_re_assert, node_re_assert_struct { kind, next })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_re_keep(next: NodePtr) -> NodePtr {
    node_allocate(&KIND_node_re_keep, node_re_keep_struct { next })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_re_open(g: u32, next: NodePtr) -> NodePtr {
    node_allocate(&KIND_node_re_open, node_re_open_struct { g, next })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_re_open_nr(g: u32, next: NodePtr) -> NodePtr {
    node_allocate(&KIND_node_re_open_nr, node_re_open_nr_struct { g, next })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_re_close(g: u32, next: NodePtr) -> NodePtr {
    node_allocate(&KIND_node_re_close, node_re_close_struct { g, next })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_re_backref(groups: Groups, icase: bool, next: NodePtr) -> NodePtr {
    node_allocate(&KIND_node_re_backref, node_re_backref_struct { groups, icase, next })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_re_cond(groups: Groups, yes: NodePtr, no: NodePtr) -> NodePtr {
    node_allocate(&KIND_node_re_cond, node_re_cond_struct { groups, yes, no })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_re_alt(a: NodePtr, b: NodePtr) -> NodePtr {
    node_allocate(&KIND_node_re_alt, node_re_alt_struct { a, b })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_re_opt(body: NodePtr, skip: NodePtr, greedy: bool) -> NodePtr {
    node_allocate(&KIND_node_re_opt, node_re_opt_struct { body, skip, greedy })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_re_rep(body: NodePtr, outer: NodePtr, min: u32, max: u32, greedy: bool, mem: bool, shape: u32, cost: u32) -> NodePtr {
    node_allocate(&KIND_node_re_rep, node_re_rep_struct { body, outer, min, max, greedy, mem, shape, cost })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_re_rep_cont() -> NodePtr {
    node_allocate(&KIND_node_re_rep_cont, node_re_rep_cont_struct {  })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_re_look(kind: u32, body: NodePtr, next: NodePtr, minlen: u32, maxlen: u32) -> NodePtr {
    node_allocate(&KIND_node_re_look, node_re_look_struct { kind, body, next, minlen, maxlen })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_search(body: NodePtr, anchor: u32) -> NodePtr {
    node_allocate(&KIND_node_search, node_search_struct { body, anchor })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_search_memchr(body: NodePtr, b: u8) -> NodePtr {
    node_allocate(&KIND_node_search_memchr, node_search_memchr_struct { body, b })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_search_memmem(body: NodePtr, needle: Bytes) -> NodePtr {
    node_allocate(&KIND_node_search_memmem, node_search_memmem_struct { body, needle })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_search_byteset(body: NodePtr, set: ByteSet) -> NodePtr {
    node_allocate(&KIND_node_search_byteset, node_search_byteset_struct { body, set })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_match_at(body: NodePtr) -> NodePtr {
    node_allocate(&KIND_node_match_at, node_match_at_struct { body })
}

#[allow(non_upper_case_globals)]
pub static KIND_node_re_succ: NodeKind = NodeKind {
    name: "node_re_succ",
    default_dispatcher: DISPATCH_node_re_succ,
    hash_func: HASH_node_re_succ,
    specializer: SPECIALIZE_node_re_succ,
    dumper: DUMP_node_re_succ,
    replacer: REPLACE_node_re_succ,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_re_sub_succ: NodeKind = NodeKind {
    name: "node_re_sub_succ",
    default_dispatcher: DISPATCH_node_re_sub_succ,
    hash_func: HASH_node_re_sub_succ,
    specializer: SPECIALIZE_node_re_sub_succ,
    dumper: DUMP_node_re_sub_succ,
    replacer: REPLACE_node_re_sub_succ,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_re_sub_succ_at: NodeKind = NodeKind {
    name: "node_re_sub_succ_at",
    default_dispatcher: DISPATCH_node_re_sub_succ_at,
    hash_func: HASH_node_re_sub_succ_at,
    specializer: SPECIALIZE_node_re_sub_succ_at,
    dumper: DUMP_node_re_sub_succ_at,
    replacer: REPLACE_node_re_sub_succ_at,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_re_byte: NodeKind = NodeKind {
    name: "node_re_byte",
    default_dispatcher: DISPATCH_node_re_byte,
    hash_func: HASH_node_re_byte,
    specializer: SPECIALIZE_node_re_byte,
    dumper: DUMP_node_re_byte,
    replacer: REPLACE_node_re_byte,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_re_lit: NodeKind = NodeKind {
    name: "node_re_lit",
    default_dispatcher: DISPATCH_node_re_lit,
    hash_func: HASH_node_re_lit,
    specializer: SPECIALIZE_node_re_lit,
    dumper: DUMP_node_re_lit,
    replacer: REPLACE_node_re_lit,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_re_one: NodeKind = NodeKind {
    name: "node_re_one",
    default_dispatcher: DISPATCH_node_re_one,
    hash_func: HASH_node_re_one,
    specializer: SPECIALIZE_node_re_one,
    dumper: DUMP_node_re_one,
    replacer: REPLACE_node_re_one,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_re_grapheme: NodeKind = NodeKind {
    name: "node_re_grapheme",
    default_dispatcher: DISPATCH_node_re_grapheme,
    hash_func: HASH_node_re_grapheme,
    specializer: SPECIALIZE_node_re_grapheme,
    dumper: DUMP_node_re_grapheme,
    replacer: REPLACE_node_re_grapheme,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_re_rep1: NodeKind = NodeKind {
    name: "node_re_rep1",
    default_dispatcher: DISPATCH_node_re_rep1,
    hash_func: HASH_node_re_rep1,
    specializer: SPECIALIZE_node_re_rep1,
    dumper: DUMP_node_re_rep1,
    replacer: REPLACE_node_re_rep1,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_re_assert: NodeKind = NodeKind {
    name: "node_re_assert",
    default_dispatcher: DISPATCH_node_re_assert,
    hash_func: HASH_node_re_assert,
    specializer: SPECIALIZE_node_re_assert,
    dumper: DUMP_node_re_assert,
    replacer: REPLACE_node_re_assert,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_re_keep: NodeKind = NodeKind {
    name: "node_re_keep",
    default_dispatcher: DISPATCH_node_re_keep,
    hash_func: HASH_node_re_keep,
    specializer: SPECIALIZE_node_re_keep,
    dumper: DUMP_node_re_keep,
    replacer: REPLACE_node_re_keep,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_re_open: NodeKind = NodeKind {
    name: "node_re_open",
    default_dispatcher: DISPATCH_node_re_open,
    hash_func: HASH_node_re_open,
    specializer: SPECIALIZE_node_re_open,
    dumper: DUMP_node_re_open,
    replacer: REPLACE_node_re_open,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_re_open_nr: NodeKind = NodeKind {
    name: "node_re_open_nr",
    default_dispatcher: DISPATCH_node_re_open_nr,
    hash_func: HASH_node_re_open_nr,
    specializer: SPECIALIZE_node_re_open_nr,
    dumper: DUMP_node_re_open_nr,
    replacer: REPLACE_node_re_open_nr,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_re_close: NodeKind = NodeKind {
    name: "node_re_close",
    default_dispatcher: DISPATCH_node_re_close,
    hash_func: HASH_node_re_close,
    specializer: SPECIALIZE_node_re_close,
    dumper: DUMP_node_re_close,
    replacer: REPLACE_node_re_close,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_re_backref: NodeKind = NodeKind {
    name: "node_re_backref",
    default_dispatcher: DISPATCH_node_re_backref,
    hash_func: HASH_node_re_backref,
    specializer: SPECIALIZE_node_re_backref,
    dumper: DUMP_node_re_backref,
    replacer: REPLACE_node_re_backref,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_re_cond: NodeKind = NodeKind {
    name: "node_re_cond",
    default_dispatcher: DISPATCH_node_re_cond,
    hash_func: HASH_node_re_cond,
    specializer: SPECIALIZE_node_re_cond,
    dumper: DUMP_node_re_cond,
    replacer: REPLACE_node_re_cond,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_re_alt: NodeKind = NodeKind {
    name: "node_re_alt",
    default_dispatcher: DISPATCH_node_re_alt,
    hash_func: HASH_node_re_alt,
    specializer: SPECIALIZE_node_re_alt,
    dumper: DUMP_node_re_alt,
    replacer: REPLACE_node_re_alt,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_re_opt: NodeKind = NodeKind {
    name: "node_re_opt",
    default_dispatcher: DISPATCH_node_re_opt,
    hash_func: HASH_node_re_opt,
    specializer: SPECIALIZE_node_re_opt,
    dumper: DUMP_node_re_opt,
    replacer: REPLACE_node_re_opt,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_re_rep: NodeKind = NodeKind {
    name: "node_re_rep",
    default_dispatcher: DISPATCH_node_re_rep,
    hash_func: HASH_node_re_rep,
    specializer: SPECIALIZE_node_re_rep,
    dumper: DUMP_node_re_rep,
    replacer: REPLACE_node_re_rep,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_re_rep_cont: NodeKind = NodeKind {
    name: "node_re_rep_cont",
    default_dispatcher: DISPATCH_node_re_rep_cont,
    hash_func: HASH_node_re_rep_cont,
    specializer: SPECIALIZE_node_re_rep_cont,
    dumper: DUMP_node_re_rep_cont,
    replacer: REPLACE_node_re_rep_cont,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_re_look: NodeKind = NodeKind {
    name: "node_re_look",
    default_dispatcher: DISPATCH_node_re_look,
    hash_func: HASH_node_re_look,
    specializer: SPECIALIZE_node_re_look,
    dumper: DUMP_node_re_look,
    replacer: REPLACE_node_re_look,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_search: NodeKind = NodeKind {
    name: "node_search",
    default_dispatcher: DISPATCH_node_search,
    hash_func: HASH_node_search,
    specializer: SPECIALIZE_node_search,
    dumper: DUMP_node_search,
    replacer: REPLACE_node_search,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_search_memchr: NodeKind = NodeKind {
    name: "node_search_memchr",
    default_dispatcher: DISPATCH_node_search_memchr,
    hash_func: HASH_node_search_memchr,
    specializer: SPECIALIZE_node_search_memchr,
    dumper: DUMP_node_search_memchr,
    replacer: REPLACE_node_search_memchr,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_search_memmem: NodeKind = NodeKind {
    name: "node_search_memmem",
    default_dispatcher: DISPATCH_node_search_memmem,
    hash_func: HASH_node_search_memmem,
    specializer: SPECIALIZE_node_search_memmem,
    dumper: DUMP_node_search_memmem,
    replacer: REPLACE_node_search_memmem,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_search_byteset: NodeKind = NodeKind {
    name: "node_search_byteset",
    default_dispatcher: DISPATCH_node_search_byteset,
    hash_func: HASH_node_search_byteset,
    specializer: SPECIALIZE_node_search_byteset,
    dumper: DUMP_node_search_byteset,
    replacer: REPLACE_node_search_byteset,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_match_at: NodeKind = NodeKind {
    name: "node_match_at",
    default_dispatcher: DISPATCH_node_match_at,
    hash_func: HASH_node_match_at,
    specializer: SPECIALIZE_node_match_at,
    dumper: DUMP_node_match_at,
    replacer: REPLACE_node_match_at,
    no_inline: false,
};


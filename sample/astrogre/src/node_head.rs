// This file is auto-generated from node.def by astrustgen. Do not edit.

pub type DispatchFn = fn(&mut Ctx, NodePtr, usize) -> u32;
pub type NodePtr = *mut Node;
pub type NodeList = &'static [NodePtr];

pub struct NodeKind {
    pub name: &'static str,
    pub default_dispatcher: DispatchFn,
    pub hash_func: fn(NodePtr) -> u64,
    pub profile: fn(&mut ProfCtx, NodePtr) -> u64,
    pub specializer: fn(&mut SpecCtx, NodePtr) -> String,
    pub dumper: fn(&mut String, NodePtr),
    pub replacer: fn(NodePtr, NodePtr, NodePtr),
    pub no_inline: bool,
    pub size: usize,
    pub children: fn(NodePtr, bool, &mut dyn FnMut(&mut NodePtr)),
}

pub const NF_HAS_HASH: u32 = 1;
pub const NF_SPECIALIZING: u32 = 2;
pub const NF_NO_INLINE: u32 = 4;
pub const NF_LAID: u32 = 8;        // part of a block made by layout_tree

#[repr(C)]
pub struct NodeHead {

    pub flags: u32,
    pub dispatcher: DispatchFn,
    pub kind: &'static NodeKind,
    pub hash: u64,

}
#[repr(C)]
pub struct Node { pub head: NodeHead }
#[repr(C)]
pub struct NodeOf<T> { pub head: NodeHead, pub u: T }

pub fn node_allocate<T>(kind: &'static NodeKind, u: T) -> NodePtr { node_allocate_with(kind, u, |_| {}) }
/// node_allocate, `init` setting head fields (head holes) before the node is shared.
pub fn node_allocate_with<T>(kind: &'static NodeKind, u: T, init: impl FnOnce(&mut NodeHead)) -> NodePtr {
    let mut b = Box::new(NodeOf { head: NodeHead {
        dispatcher: kind.default_dispatcher, kind, hash: 0,
        flags: if kind.no_inline { NF_NO_INLINE } else { 0 },
        
    }, u });
    init(&mut b.head);
    let n = Box::into_raw(b) as NodePtr;
    node_allocated(n);
    n
}

// ---- node access: the generated code's only unsafe (sample/saruby/docs/unsafe_catalog.md "generated code") ----
// Contract on every NodePtr given to these (the generator's manual invariant): non-null, made by
// node_allocate / layout_tree, and live for as long as the returned borrow is used.
/// The operand struct type of a node kind.
pub trait NodeStruct { fn kind() -> &'static NodeKind; }
/// n's operand struct; n is of T's kind (checked in debug builds).  Operand fields other than the Cell state of @ref
/// are not written while a borrow lives (only CHILDREN / REPLACE write, through node_u_mut).
#[inline(always)]
pub fn node_u<'a, T: NodeStruct>(n: NodePtr) -> &'a T {
    debug_assert!(std::ptr::eq(node_kind(n), T::kind()));
    // SAFETY: the contract above
    unsafe { &(*(n as *const NodeOf<T>)).u }
}
/// n's operand struct to rewrite (a copy in layout_tree, REPLACE): no other borrow of it lives.
#[inline(always)]
pub fn node_u_mut<'a, T: NodeStruct>(n: NodePtr) -> &'a mut T {
    debug_assert!(std::ptr::eq(node_kind(n), T::kind()));
    // SAFETY: the contract above, and unique
    unsafe { &mut (*(n as *mut NodeOf<T>)).u }
}
// head fields one at a time, so no borrow of the head overlaps a write of another field
#[inline(always)] pub fn node_kind(n: NodePtr) -> &'static NodeKind { unsafe { (*n).head.kind } }
#[inline(always)] pub fn node_dispatcher(n: NodePtr) -> DispatchFn { unsafe { (*n).head.dispatcher } }
#[inline(always)] pub fn node_flags(n: NodePtr) -> u32 { unsafe { (*n).head.flags } }
#[inline(always)] pub fn node_set_flags(n: NodePtr, f: u32) { unsafe { (*n).head.flags = f; } }
#[inline(always)] pub fn node_hash_set(n: NodePtr, h: u64) { unsafe { (*n).head.hash = h; } }
#[inline(always)] pub fn node_hash_get(n: NodePtr) -> u64 { unsafe { (*n).head.hash } }

/// Copy the tree under `root` into one block, parents before children in operand order, and
/// return the copy.  The original stays valid (other holders of its nodes are unaffected).
pub fn layout_tree(root: NodePtr) -> NodePtr {
    fn size(n: NodePtr) -> usize {
        if n.is_null() { return 0; }
        let k = node_kind(n);
        let mut s = k.size;
        (k.children)(n, false, &mut |c| s += size(*c));
        s
    }
    fn copy(n: NodePtr, at: &mut *mut u8) -> NodePtr {
        let k = node_kind(n);
        let d = *at;
        // SAFETY: n is k.size bytes; d has k.size bytes left of the block size() measured
        unsafe { std::ptr::copy_nonoverlapping(n as *const u8, d, k.size); }
        *at = d.wrapping_add(k.size);
        let d = d as NodePtr;
        node_set_flags(d, node_flags(d) | NF_LAID);
        (k.children)(d, true, &mut |c| if !c.is_null() { *c = copy(*c, at); });
        d
    }
    if root.is_null() { return root; }
    let mut at = Box::leak(vec![0u64; size(root) / 8].into_boxed_slice()).as_mut_ptr() as *mut u8;
    copy(root, &mut at)
}
pub fn node_laid(n: NodePtr) -> bool { !n.is_null() && node_flags(n) & NF_LAID != 0 }
/// Specialized code's pointer to child `c` of `n` (contiguous trees): an address only, node_u reads it.
pub fn child_ptr_expr(n: NodePtr, c: NodePtr) -> String {
    if c.is_null() { "std::ptr::null_mut()".to_string() } else { format!("n.wrapping_byte_offset({}isize)", c as isize - n as isize) }
}
/// A list operand in specialized code over a contiguous tree: no elements, the parent's address.
#[inline(always)]
pub fn list_base(n: NodePtr) -> NodeList { unsafe { std::slice::from_raw_parts(n as *const NodePtr, 0) } }

/// Structural (Merkle) hash, cached in the head.
pub fn hash_node(n: NodePtr) -> u64 {
    if n.is_null() { return hash_str("null"); }
    if node_flags(n) & NF_HAS_HASH != 0 { return node_hash_get(n); }
    let h = (node_kind(n).hash_func)(n);
    node_hash_set(n, h);
    node_set_flags(n, node_flags(n) | NF_HAS_HASH);
    h
}
/// What specialization bakes into the subtree at n (PROF_<name>); each node once per ProfCtx.
pub fn profile_node(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    if n.is_null() || !pc.seen.insert(n as usize) { return 0; }
    (node_kind(n).profile)(pc, n)
}
pub fn hash_node_list(ns: NodeList) -> u64 {
    let mut h = hash_u64(ns.len() as u64);
    for x in ns.iter() { h = hash_merge(h, hash_node(*x)); }
    h
}
pub fn dump_node(out: &mut String, n: NodePtr) {
    if n.is_null() { out.push_str("null"); return; }
    (node_kind(n).dumper)(out, n)
}
pub fn node_replace_child(parent: NodePtr, old: NodePtr, new: NodePtr) {
    (node_kind(parent).replacer)(parent, old, new)
}
/// SD type name for the subtree rooted at n ("Dyn" when it must stay dynamic).
pub fn specialize_node(sc: &mut SpecCtx, n: NodePtr) -> String {
    if n.is_null() { return "Dyn".to_string(); }
    let f = node_flags(n);
    if f & NF_NO_INLINE != 0 || (f & NF_SPECIALIZING != 0 && sc.reenter == 0) { return "Dyn".to_string(); }
    node_set_flags(n, f | NF_SPECIALIZING);
    let t = (node_kind(n).specializer)(sc, n);
    node_set_flags(n, node_flags(n) & !NF_SPECIALIZING);
    t
}
/// specialize_node for a node being specialized already (a recursion target compiled into itself):
/// its subtree again; the copy's recursive sites see the target in sc.unrolled and call it instead.
pub fn specialize_open(sc: &mut SpecCtx, n: NodePtr) -> String {
    sc.reenter += 1;
    let t = specialize_node(sc, n);
    sc.reenter -= 1;
    node_set_flags(n, node_flags(n) | NF_SPECIALIZING);
    t
}
/// Specialize n as a public entry: an exported `SD_<hash>` with the dispatcher's signature.
pub fn specialize_entry(sc: &mut SpecCtx, n: NodePtr) -> String {
    let fname = format!("SD_{:016x}", sc.entry_name(hash_node(n)));
    if sc.emitted.contains(&fname) { return fname; }
    sc.holes.clear();
    sc.begin_entry(n as usize, hash_node(n));
    sc.tag = rec_tag(sc, n);
    let t = specialize_node(sc, n);
    let p = sc.rec_suffix(hash_node(n));
    let s = format!("S_{:016x}{}{}", hash_node(n), sc.tag, p);
    if sc.emitted.contains(&format!("R_{:016x}{}{}", hash_node(n), sc.tag, p)) && !sc.emitted.contains(&s) {
        // n calls itself: its one body is S_<hash>::run (R_ calls it directly); SD_ passes the spec-in value on
        specialize_run(sc, &s, &t);
        sc.out.push_str(&format!("#[no_mangle]
pub fn {}(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {{ {}::run(c, n, pos) }}
", fname, s));
    } else {
        sc.out.push_str(&format!("#[no_mangle]
pub fn {}(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {{ <{} as Disp>::call(c, n, pos) }}
", fname, t));
    }
    // binding descriptor: (offset from n, size) of each bound operand the entry reads
    if !sc.holes.is_empty() {
        let d: Vec<String> = sc.holes.iter().map(|&(p, sz)| format!("{},{}", p - n as usize, sz)).collect();
        sc.out.push_str(&format!("#[no_mangle]
pub static {}_HOLES: [u32; {}] = [{}];
", fname, 2 * d.len(), d.join(",")));
    }
    sc.begin_entry(0, 0);
    sc.emitted.insert(fname.clone());
    fname
}
/// R_ / S_ suffix of q: its own profile (what its body bakes), not the entry's: entries reaching q share them.
pub fn rec_tag(sc: &mut SpecCtx, q: NodePtr) -> String {
    if !sc.profiled { return String::new(); }
    let h = *sc.own.entry(q as usize).or_insert_with(|| hash_merge(hash_node(q), profile_node(&mut ProfCtx::new(), q)));
    format!("_{:016x}", h)
}
/// A call from inside the specialization of q to q (recursion): `R_<hash>`, a direct call of
/// `S_<hash>::run`, q's entry taking the spec-in value as a parameter (specialize_entry / _finish emit it).
pub fn specialize_rec(sc: &mut SpecCtx, q: NodePtr) -> String {
    let h = hash_node(q);
    let p = sc.rec_suffix(h);
    // P calls the bound entry's own body (S_<hash>P): the entry's suffix
    let tag = if p.is_empty() { rec_tag(sc, q) } else { sc.tag.clone() };
    let name = format!("R_{:016x}{}{}", h, tag, p);
    if !p.is_empty() {
        // the entry's patched body (S_<hash>P::run) holds the values of the one node it was bound to
        if sc.emitted.insert(name.clone()) {
            sc.out.push_str(&format!("pub struct {0};\nimpl Disp for {0} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {{ if n as usize == &raw const ASTRUST_ROOT as usize {{ S_{1:016x}{2}P::run(c, n, pos) }} else {{ std::hint::cold_path(); <Dyn as Disp>::call(c, n, pos) }} }}\n    #[inline(always)]\n    fn spec(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {{ if n as usize == &raw const ASTRUST_ROOT as usize {{ S_{1:016x}{2}P::run(c, n, pos) }} else {{ std::hint::cold_path(); <Dyn as Disp>::spec(c, n, pos) }} }}\n}}\n", name, h, tag));
        }
        return name;
    }
    if sc.emitted.insert(name.clone()) {
        sc.rec.push((q as usize, tag.clone()));
        sc.out.push_str(&format!("pub struct {0};\nimpl Disp for {0} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {{ S_{1:016x}{2}::run(c, n, pos) }}\n    #[inline(always)]\n    fn spec(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {{ S_{1:016x}{2}::run(c, n, pos) }}\n}}\n", name, h, tag));
    }
    name
}
fn specialize_run(sc: &mut SpecCtx, s: &str, t: &str) {
    sc.out.push_str(&format!("pub struct {0};\nimpl {0} {{\n    #[inline(never)]\n    pub fn run(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {{ <{1} as Disp>::spec(c, n, pos) }}\n}}\n", s, t));
    sc.emitted.insert(s.to_string());
}
/// After the entries: S_<hash>::run of the recursion targets whose SD_ was not the one to provide it.
pub fn specialize_finish(sc: &mut SpecCtx) {
    while let Some((q, tag)) = sc.rec.pop() {
        let q = q as NodePtr;
        let s = format!("S_{:016x}{}", hash_node(q), tag);
        if sc.emitted.contains(&s) { continue; }
        let t = specialize_node(sc, q);
        specialize_run(sc, &s, &t);
    }
}

/// A dispatcher as a type: `Dyn` calls through the node head, an SD type statically.
pub trait Disp {
    fn call(c: &mut Ctx, n: NodePtr, pos: usize) -> u32;
    /// call() across a speculation boundary (the embedder only speculates small callees, to a bounded depth).
    #[inline(always)]
    fn spec(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 { Self::call(c, n, pos) }

}
/// A list of children: `DynList` loops over the heads, an SD list type is unrolled.
pub trait DispList {
    fn len(ns: NodeList) -> usize;
    fn nth(c: &mut Ctx, ns: NodeList, i: usize, pos: usize) -> u32;
    /// Every child in order, its result to f (EVAL_EACH!).
    fn each<F: FnMut(&mut Ctx, usize, u32) -> ()>(c: &mut Ctx, ns: NodeList, pos: usize, f: F) -> ();
}
pub struct Dyn;
impl Disp for Dyn {
    #[inline(always)]
    fn call(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 { node_dispatcher(n)(c, n, pos) }
}
pub struct DynList;
impl DispList for DynList {
    #[inline(always)] fn len(ns: NodeList) -> usize { ns.len() }
    #[inline(always)]
    fn nth(c: &mut Ctx, ns: NodeList, i: usize, pos: usize) -> u32 {
        let x = ns[i];
        node_dispatcher(x)(c, x, pos)
    }
    #[inline(always)]
    fn each<F: FnMut(&mut Ctx, usize, u32) -> ()>(c: &mut Ctx, ns: NodeList, pos: usize, mut f: F) -> () {
        // the first ones in a row, as ASTroGen's per-arity dispatchers (short lists are the common case)
        if ns.len() > 0 { let x = ns[0]; let r = node_dispatcher(x)(c, x, pos); f(c, 0, r); }
        if ns.len() > 1 { let x = ns[1]; let r = node_dispatcher(x)(c, x, pos); f(c, 1, r); }
        if ns.len() > 2 { let x = ns[2]; let r = node_dispatcher(x)(c, x, pos); f(c, 2, r); }
        for i in 3..ns.len() {
            let x = ns[i];
            let r = node_dispatcher(x)(c, x, pos);
            f(c, i, r);
        }
        ()
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
pub struct node_re_open_rec_struct {
    pub g: u32,
    pub next: NodePtr,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_re_close_rec_struct {
    pub g: u32,
    pub next: NodePtr,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_re_call_struct {
    pub g: u32,
    pub next: NodePtr,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_re_ret_struct {

}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_re_backref_lv_struct {
    pub groups: Groups,
    pub icase: bool,
    pub nest: i32,
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
pub struct node_re_absent_struct {
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
    node_allocate_with(&KIND_node_re_succ, node_re_succ_struct {  }, |_| { })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_re_sub_succ() -> NodePtr {
    node_allocate_with(&KIND_node_re_sub_succ, node_re_sub_succ_struct {  }, |_| { })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_re_sub_succ_at() -> NodePtr {
    node_allocate_with(&KIND_node_re_sub_succ_at, node_re_sub_succ_at_struct {  }, |_| { })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_re_byte(b: u8, next: NodePtr) -> NodePtr {
    node_allocate_with(&KIND_node_re_byte, node_re_byte_struct { b, next }, |_| { })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_re_lit(bytes: Bytes, next: NodePtr) -> NodePtr {
    node_allocate_with(&KIND_node_re_lit, node_re_lit_struct { bytes, next }, |_| { })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_re_one(kind: u32, ch: u32, cls: ClassRef, next: NodePtr) -> NodePtr {
    node_allocate_with(&KIND_node_re_one, node_re_one_struct { kind, ch, cls, next }, |_| { })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_re_grapheme(next: NodePtr) -> NodePtr {
    node_allocate_with(&KIND_node_re_grapheme, node_re_grapheme_struct { next }, |_| { })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_re_rep1(kind: u32, ch: u32, cls: ClassRef, min: u32, max: u32, greedy: bool, nb: i32, next: NodePtr) -> NodePtr {
    node_allocate_with(&KIND_node_re_rep1, node_re_rep1_struct { kind, ch, cls, min, max, greedy, nb, next }, |_| { })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_re_assert(kind: u32, next: NodePtr) -> NodePtr {
    node_allocate_with(&KIND_node_re_assert, node_re_assert_struct { kind, next }, |_| { })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_re_keep(next: NodePtr) -> NodePtr {
    node_allocate_with(&KIND_node_re_keep, node_re_keep_struct { next }, |_| { })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_re_open(g: u32, next: NodePtr) -> NodePtr {
    node_allocate_with(&KIND_node_re_open, node_re_open_struct { g, next }, |_| { })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_re_open_nr(g: u32, next: NodePtr) -> NodePtr {
    node_allocate_with(&KIND_node_re_open_nr, node_re_open_nr_struct { g, next }, |_| { })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_re_close(g: u32, next: NodePtr) -> NodePtr {
    node_allocate_with(&KIND_node_re_close, node_re_close_struct { g, next }, |_| { })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_re_backref(groups: Groups, icase: bool, next: NodePtr) -> NodePtr {
    node_allocate_with(&KIND_node_re_backref, node_re_backref_struct { groups, icase, next }, |_| { })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_re_open_rec(g: u32, next: NodePtr) -> NodePtr {
    node_allocate_with(&KIND_node_re_open_rec, node_re_open_rec_struct { g, next }, |_| { })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_re_close_rec(g: u32, next: NodePtr) -> NodePtr {
    node_allocate_with(&KIND_node_re_close_rec, node_re_close_rec_struct { g, next }, |_| { })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_re_call(g: u32, next: NodePtr) -> NodePtr {
    node_allocate_with(&KIND_node_re_call, node_re_call_struct { g, next }, |_| { })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_re_ret() -> NodePtr {
    node_allocate_with(&KIND_node_re_ret, node_re_ret_struct {  }, |_| { })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_re_backref_lv(groups: Groups, icase: bool, nest: i32, next: NodePtr) -> NodePtr {
    node_allocate_with(&KIND_node_re_backref_lv, node_re_backref_lv_struct { groups, icase, nest, next }, |_| { })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_re_cond(groups: Groups, yes: NodePtr, no: NodePtr) -> NodePtr {
    node_allocate_with(&KIND_node_re_cond, node_re_cond_struct { groups, yes, no }, |_| { })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_re_alt(a: NodePtr, b: NodePtr) -> NodePtr {
    node_allocate_with(&KIND_node_re_alt, node_re_alt_struct { a, b }, |_| { })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_re_opt(body: NodePtr, skip: NodePtr, greedy: bool) -> NodePtr {
    node_allocate_with(&KIND_node_re_opt, node_re_opt_struct { body, skip, greedy }, |_| { })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_re_rep(body: NodePtr, outer: NodePtr, min: u32, max: u32, greedy: bool, mem: bool, shape: u32, cost: u32) -> NodePtr {
    node_allocate_with(&KIND_node_re_rep, node_re_rep_struct { body, outer, min, max, greedy, mem, shape, cost }, |_| { })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_re_rep_cont() -> NodePtr {
    node_allocate_with(&KIND_node_re_rep_cont, node_re_rep_cont_struct {  }, |_| { })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_re_look(kind: u32, body: NodePtr, next: NodePtr, minlen: u32, maxlen: u32) -> NodePtr {
    node_allocate_with(&KIND_node_re_look, node_re_look_struct { kind, body, next, minlen, maxlen }, |_| { })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_re_absent(body: NodePtr, next: NodePtr, minlen: u32, maxlen: u32) -> NodePtr {
    node_allocate_with(&KIND_node_re_absent, node_re_absent_struct { body, next, minlen, maxlen }, |_| { })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_search(body: NodePtr, anchor: u32) -> NodePtr {
    node_allocate_with(&KIND_node_search, node_search_struct { body, anchor }, |_| { })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_search_memchr(body: NodePtr, b: u8) -> NodePtr {
    node_allocate_with(&KIND_node_search_memchr, node_search_memchr_struct { body, b }, |_| { })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_search_memmem(body: NodePtr, needle: Bytes) -> NodePtr {
    node_allocate_with(&KIND_node_search_memmem, node_search_memmem_struct { body, needle }, |_| { })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_search_byteset(body: NodePtr, set: ByteSet) -> NodePtr {
    node_allocate_with(&KIND_node_search_byteset, node_search_byteset_struct { body, set }, |_| { })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_match_at(body: NodePtr) -> NodePtr {
    node_allocate_with(&KIND_node_match_at, node_match_at_struct { body }, |_| { })
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_re_succ(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = node_u_mut::<node_re_succ_struct>(n);

}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_re_sub_succ(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = node_u_mut::<node_re_sub_succ_struct>(n);

}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_re_sub_succ_at(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = node_u_mut::<node_re_sub_succ_at_struct>(n);

}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_re_byte(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = node_u_mut::<node_re_byte_struct>(n);
    f(&mut u.next);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_re_lit(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = node_u_mut::<node_re_lit_struct>(n);
    f(&mut u.next);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_re_one(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = node_u_mut::<node_re_one_struct>(n);
    f(&mut u.next);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_re_grapheme(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = node_u_mut::<node_re_grapheme_struct>(n);
    f(&mut u.next);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_re_rep1(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = node_u_mut::<node_re_rep1_struct>(n);
    f(&mut u.next);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_re_assert(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = node_u_mut::<node_re_assert_struct>(n);
    f(&mut u.next);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_re_keep(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = node_u_mut::<node_re_keep_struct>(n);
    f(&mut u.next);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_re_open(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = node_u_mut::<node_re_open_struct>(n);
    f(&mut u.next);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_re_open_nr(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = node_u_mut::<node_re_open_nr_struct>(n);
    f(&mut u.next);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_re_close(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = node_u_mut::<node_re_close_struct>(n);
    f(&mut u.next);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_re_backref(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = node_u_mut::<node_re_backref_struct>(n);
    f(&mut u.next);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_re_open_rec(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = node_u_mut::<node_re_open_rec_struct>(n);
    f(&mut u.next);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_re_close_rec(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = node_u_mut::<node_re_close_rec_struct>(n);
    f(&mut u.next);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_re_call(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = node_u_mut::<node_re_call_struct>(n);
    f(&mut u.next);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_re_ret(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = node_u_mut::<node_re_ret_struct>(n);

}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_re_backref_lv(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = node_u_mut::<node_re_backref_lv_struct>(n);
    f(&mut u.next);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_re_cond(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = node_u_mut::<node_re_cond_struct>(n);
    f(&mut u.yes);
    f(&mut u.no);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_re_alt(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = node_u_mut::<node_re_alt_struct>(n);
    f(&mut u.a);
    f(&mut u.b);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_re_opt(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = node_u_mut::<node_re_opt_struct>(n);
    f(&mut u.body);
    f(&mut u.skip);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_re_rep(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = node_u_mut::<node_re_rep_struct>(n);
    f(&mut u.body);
    f(&mut u.outer);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_re_rep_cont(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = node_u_mut::<node_re_rep_cont_struct>(n);

}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_re_look(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = node_u_mut::<node_re_look_struct>(n);
    f(&mut u.body);
    f(&mut u.next);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_re_absent(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = node_u_mut::<node_re_absent_struct>(n);
    f(&mut u.body);
    f(&mut u.next);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_search(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = node_u_mut::<node_search_struct>(n);
    f(&mut u.body);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_search_memchr(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = node_u_mut::<node_search_memchr_struct>(n);
    f(&mut u.body);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_search_memmem(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = node_u_mut::<node_search_memmem_struct>(n);
    f(&mut u.body);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_search_byteset(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = node_u_mut::<node_search_byteset_struct>(n);
    f(&mut u.body);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_match_at(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = node_u_mut::<node_match_at_struct>(n);
    f(&mut u.body);
}

#[allow(non_upper_case_globals)]
pub static KIND_node_re_succ: NodeKind = NodeKind {
    name: "node_re_succ",
    default_dispatcher: DISPATCH_node_re_succ,
    hash_func: HASH_node_re_succ,
    profile: PROF_node_re_succ,
    specializer: SPECIALIZE_node_re_succ,
    dumper: DUMP_node_re_succ,
    replacer: REPLACE_node_re_succ,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_re_succ_struct>>(),
    children: CHILDREN_node_re_succ,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_re_succ_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_re_succ_struct>>() <= 8);
impl NodeStruct for node_re_succ_struct { #[inline(always)] fn kind() -> &'static NodeKind { &KIND_node_re_succ } }

#[allow(non_upper_case_globals)]
pub static KIND_node_re_sub_succ: NodeKind = NodeKind {
    name: "node_re_sub_succ",
    default_dispatcher: DISPATCH_node_re_sub_succ,
    hash_func: HASH_node_re_sub_succ,
    profile: PROF_node_re_sub_succ,
    specializer: SPECIALIZE_node_re_sub_succ,
    dumper: DUMP_node_re_sub_succ,
    replacer: REPLACE_node_re_sub_succ,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_re_sub_succ_struct>>(),
    children: CHILDREN_node_re_sub_succ,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_re_sub_succ_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_re_sub_succ_struct>>() <= 8);
impl NodeStruct for node_re_sub_succ_struct { #[inline(always)] fn kind() -> &'static NodeKind { &KIND_node_re_sub_succ } }

#[allow(non_upper_case_globals)]
pub static KIND_node_re_sub_succ_at: NodeKind = NodeKind {
    name: "node_re_sub_succ_at",
    default_dispatcher: DISPATCH_node_re_sub_succ_at,
    hash_func: HASH_node_re_sub_succ_at,
    profile: PROF_node_re_sub_succ_at,
    specializer: SPECIALIZE_node_re_sub_succ_at,
    dumper: DUMP_node_re_sub_succ_at,
    replacer: REPLACE_node_re_sub_succ_at,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_re_sub_succ_at_struct>>(),
    children: CHILDREN_node_re_sub_succ_at,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_re_sub_succ_at_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_re_sub_succ_at_struct>>() <= 8);
impl NodeStruct for node_re_sub_succ_at_struct { #[inline(always)] fn kind() -> &'static NodeKind { &KIND_node_re_sub_succ_at } }

#[allow(non_upper_case_globals)]
pub static KIND_node_re_byte: NodeKind = NodeKind {
    name: "node_re_byte",
    default_dispatcher: DISPATCH_node_re_byte,
    hash_func: HASH_node_re_byte,
    profile: PROF_node_re_byte,
    specializer: SPECIALIZE_node_re_byte,
    dumper: DUMP_node_re_byte,
    replacer: REPLACE_node_re_byte,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_re_byte_struct>>(),
    children: CHILDREN_node_re_byte,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_re_byte_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_re_byte_struct>>() <= 8);
impl NodeStruct for node_re_byte_struct { #[inline(always)] fn kind() -> &'static NodeKind { &KIND_node_re_byte } }

#[allow(non_upper_case_globals)]
pub static KIND_node_re_lit: NodeKind = NodeKind {
    name: "node_re_lit",
    default_dispatcher: DISPATCH_node_re_lit,
    hash_func: HASH_node_re_lit,
    profile: PROF_node_re_lit,
    specializer: SPECIALIZE_node_re_lit,
    dumper: DUMP_node_re_lit,
    replacer: REPLACE_node_re_lit,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_re_lit_struct>>(),
    children: CHILDREN_node_re_lit,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_re_lit_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_re_lit_struct>>() <= 8);
impl NodeStruct for node_re_lit_struct { #[inline(always)] fn kind() -> &'static NodeKind { &KIND_node_re_lit } }

#[allow(non_upper_case_globals)]
pub static KIND_node_re_one: NodeKind = NodeKind {
    name: "node_re_one",
    default_dispatcher: DISPATCH_node_re_one,
    hash_func: HASH_node_re_one,
    profile: PROF_node_re_one,
    specializer: SPECIALIZE_node_re_one,
    dumper: DUMP_node_re_one,
    replacer: REPLACE_node_re_one,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_re_one_struct>>(),
    children: CHILDREN_node_re_one,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_re_one_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_re_one_struct>>() <= 8);
impl NodeStruct for node_re_one_struct { #[inline(always)] fn kind() -> &'static NodeKind { &KIND_node_re_one } }

#[allow(non_upper_case_globals)]
pub static KIND_node_re_grapheme: NodeKind = NodeKind {
    name: "node_re_grapheme",
    default_dispatcher: DISPATCH_node_re_grapheme,
    hash_func: HASH_node_re_grapheme,
    profile: PROF_node_re_grapheme,
    specializer: SPECIALIZE_node_re_grapheme,
    dumper: DUMP_node_re_grapheme,
    replacer: REPLACE_node_re_grapheme,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_re_grapheme_struct>>(),
    children: CHILDREN_node_re_grapheme,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_re_grapheme_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_re_grapheme_struct>>() <= 8);
impl NodeStruct for node_re_grapheme_struct { #[inline(always)] fn kind() -> &'static NodeKind { &KIND_node_re_grapheme } }

#[allow(non_upper_case_globals)]
pub static KIND_node_re_rep1: NodeKind = NodeKind {
    name: "node_re_rep1",
    default_dispatcher: DISPATCH_node_re_rep1,
    hash_func: HASH_node_re_rep1,
    profile: PROF_node_re_rep1,
    specializer: SPECIALIZE_node_re_rep1,
    dumper: DUMP_node_re_rep1,
    replacer: REPLACE_node_re_rep1,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_re_rep1_struct>>(),
    children: CHILDREN_node_re_rep1,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_re_rep1_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_re_rep1_struct>>() <= 8);
impl NodeStruct for node_re_rep1_struct { #[inline(always)] fn kind() -> &'static NodeKind { &KIND_node_re_rep1 } }

#[allow(non_upper_case_globals)]
pub static KIND_node_re_assert: NodeKind = NodeKind {
    name: "node_re_assert",
    default_dispatcher: DISPATCH_node_re_assert,
    hash_func: HASH_node_re_assert,
    profile: PROF_node_re_assert,
    specializer: SPECIALIZE_node_re_assert,
    dumper: DUMP_node_re_assert,
    replacer: REPLACE_node_re_assert,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_re_assert_struct>>(),
    children: CHILDREN_node_re_assert,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_re_assert_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_re_assert_struct>>() <= 8);
impl NodeStruct for node_re_assert_struct { #[inline(always)] fn kind() -> &'static NodeKind { &KIND_node_re_assert } }

#[allow(non_upper_case_globals)]
pub static KIND_node_re_keep: NodeKind = NodeKind {
    name: "node_re_keep",
    default_dispatcher: DISPATCH_node_re_keep,
    hash_func: HASH_node_re_keep,
    profile: PROF_node_re_keep,
    specializer: SPECIALIZE_node_re_keep,
    dumper: DUMP_node_re_keep,
    replacer: REPLACE_node_re_keep,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_re_keep_struct>>(),
    children: CHILDREN_node_re_keep,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_re_keep_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_re_keep_struct>>() <= 8);
impl NodeStruct for node_re_keep_struct { #[inline(always)] fn kind() -> &'static NodeKind { &KIND_node_re_keep } }

#[allow(non_upper_case_globals)]
pub static KIND_node_re_open: NodeKind = NodeKind {
    name: "node_re_open",
    default_dispatcher: DISPATCH_node_re_open,
    hash_func: HASH_node_re_open,
    profile: PROF_node_re_open,
    specializer: SPECIALIZE_node_re_open,
    dumper: DUMP_node_re_open,
    replacer: REPLACE_node_re_open,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_re_open_struct>>(),
    children: CHILDREN_node_re_open,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_re_open_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_re_open_struct>>() <= 8);
impl NodeStruct for node_re_open_struct { #[inline(always)] fn kind() -> &'static NodeKind { &KIND_node_re_open } }

#[allow(non_upper_case_globals)]
pub static KIND_node_re_open_nr: NodeKind = NodeKind {
    name: "node_re_open_nr",
    default_dispatcher: DISPATCH_node_re_open_nr,
    hash_func: HASH_node_re_open_nr,
    profile: PROF_node_re_open_nr,
    specializer: SPECIALIZE_node_re_open_nr,
    dumper: DUMP_node_re_open_nr,
    replacer: REPLACE_node_re_open_nr,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_re_open_nr_struct>>(),
    children: CHILDREN_node_re_open_nr,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_re_open_nr_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_re_open_nr_struct>>() <= 8);
impl NodeStruct for node_re_open_nr_struct { #[inline(always)] fn kind() -> &'static NodeKind { &KIND_node_re_open_nr } }

#[allow(non_upper_case_globals)]
pub static KIND_node_re_close: NodeKind = NodeKind {
    name: "node_re_close",
    default_dispatcher: DISPATCH_node_re_close,
    hash_func: HASH_node_re_close,
    profile: PROF_node_re_close,
    specializer: SPECIALIZE_node_re_close,
    dumper: DUMP_node_re_close,
    replacer: REPLACE_node_re_close,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_re_close_struct>>(),
    children: CHILDREN_node_re_close,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_re_close_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_re_close_struct>>() <= 8);
impl NodeStruct for node_re_close_struct { #[inline(always)] fn kind() -> &'static NodeKind { &KIND_node_re_close } }

#[allow(non_upper_case_globals)]
pub static KIND_node_re_backref: NodeKind = NodeKind {
    name: "node_re_backref",
    default_dispatcher: DISPATCH_node_re_backref,
    hash_func: HASH_node_re_backref,
    profile: PROF_node_re_backref,
    specializer: SPECIALIZE_node_re_backref,
    dumper: DUMP_node_re_backref,
    replacer: REPLACE_node_re_backref,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_re_backref_struct>>(),
    children: CHILDREN_node_re_backref,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_re_backref_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_re_backref_struct>>() <= 8);
impl NodeStruct for node_re_backref_struct { #[inline(always)] fn kind() -> &'static NodeKind { &KIND_node_re_backref } }

#[allow(non_upper_case_globals)]
pub static KIND_node_re_open_rec: NodeKind = NodeKind {
    name: "node_re_open_rec",
    default_dispatcher: DISPATCH_node_re_open_rec,
    hash_func: HASH_node_re_open_rec,
    profile: PROF_node_re_open_rec,
    specializer: SPECIALIZE_node_re_open_rec,
    dumper: DUMP_node_re_open_rec,
    replacer: REPLACE_node_re_open_rec,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_re_open_rec_struct>>(),
    children: CHILDREN_node_re_open_rec,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_re_open_rec_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_re_open_rec_struct>>() <= 8);
impl NodeStruct for node_re_open_rec_struct { #[inline(always)] fn kind() -> &'static NodeKind { &KIND_node_re_open_rec } }

#[allow(non_upper_case_globals)]
pub static KIND_node_re_close_rec: NodeKind = NodeKind {
    name: "node_re_close_rec",
    default_dispatcher: DISPATCH_node_re_close_rec,
    hash_func: HASH_node_re_close_rec,
    profile: PROF_node_re_close_rec,
    specializer: SPECIALIZE_node_re_close_rec,
    dumper: DUMP_node_re_close_rec,
    replacer: REPLACE_node_re_close_rec,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_re_close_rec_struct>>(),
    children: CHILDREN_node_re_close_rec,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_re_close_rec_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_re_close_rec_struct>>() <= 8);
impl NodeStruct for node_re_close_rec_struct { #[inline(always)] fn kind() -> &'static NodeKind { &KIND_node_re_close_rec } }

#[allow(non_upper_case_globals)]
pub static KIND_node_re_call: NodeKind = NodeKind {
    name: "node_re_call",
    default_dispatcher: DISPATCH_node_re_call,
    hash_func: HASH_node_re_call,
    profile: PROF_node_re_call,
    specializer: SPECIALIZE_node_re_call,
    dumper: DUMP_node_re_call,
    replacer: REPLACE_node_re_call,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_re_call_struct>>(),
    children: CHILDREN_node_re_call,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_re_call_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_re_call_struct>>() <= 8);
impl NodeStruct for node_re_call_struct { #[inline(always)] fn kind() -> &'static NodeKind { &KIND_node_re_call } }

#[allow(non_upper_case_globals)]
pub static KIND_node_re_ret: NodeKind = NodeKind {
    name: "node_re_ret",
    default_dispatcher: DISPATCH_node_re_ret,
    hash_func: HASH_node_re_ret,
    profile: PROF_node_re_ret,
    specializer: SPECIALIZE_node_re_ret,
    dumper: DUMP_node_re_ret,
    replacer: REPLACE_node_re_ret,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_re_ret_struct>>(),
    children: CHILDREN_node_re_ret,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_re_ret_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_re_ret_struct>>() <= 8);
impl NodeStruct for node_re_ret_struct { #[inline(always)] fn kind() -> &'static NodeKind { &KIND_node_re_ret } }

#[allow(non_upper_case_globals)]
pub static KIND_node_re_backref_lv: NodeKind = NodeKind {
    name: "node_re_backref_lv",
    default_dispatcher: DISPATCH_node_re_backref_lv,
    hash_func: HASH_node_re_backref_lv,
    profile: PROF_node_re_backref_lv,
    specializer: SPECIALIZE_node_re_backref_lv,
    dumper: DUMP_node_re_backref_lv,
    replacer: REPLACE_node_re_backref_lv,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_re_backref_lv_struct>>(),
    children: CHILDREN_node_re_backref_lv,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_re_backref_lv_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_re_backref_lv_struct>>() <= 8);
impl NodeStruct for node_re_backref_lv_struct { #[inline(always)] fn kind() -> &'static NodeKind { &KIND_node_re_backref_lv } }

#[allow(non_upper_case_globals)]
pub static KIND_node_re_cond: NodeKind = NodeKind {
    name: "node_re_cond",
    default_dispatcher: DISPATCH_node_re_cond,
    hash_func: HASH_node_re_cond,
    profile: PROF_node_re_cond,
    specializer: SPECIALIZE_node_re_cond,
    dumper: DUMP_node_re_cond,
    replacer: REPLACE_node_re_cond,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_re_cond_struct>>(),
    children: CHILDREN_node_re_cond,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_re_cond_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_re_cond_struct>>() <= 8);
impl NodeStruct for node_re_cond_struct { #[inline(always)] fn kind() -> &'static NodeKind { &KIND_node_re_cond } }

#[allow(non_upper_case_globals)]
pub static KIND_node_re_alt: NodeKind = NodeKind {
    name: "node_re_alt",
    default_dispatcher: DISPATCH_node_re_alt,
    hash_func: HASH_node_re_alt,
    profile: PROF_node_re_alt,
    specializer: SPECIALIZE_node_re_alt,
    dumper: DUMP_node_re_alt,
    replacer: REPLACE_node_re_alt,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_re_alt_struct>>(),
    children: CHILDREN_node_re_alt,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_re_alt_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_re_alt_struct>>() <= 8);
impl NodeStruct for node_re_alt_struct { #[inline(always)] fn kind() -> &'static NodeKind { &KIND_node_re_alt } }

#[allow(non_upper_case_globals)]
pub static KIND_node_re_opt: NodeKind = NodeKind {
    name: "node_re_opt",
    default_dispatcher: DISPATCH_node_re_opt,
    hash_func: HASH_node_re_opt,
    profile: PROF_node_re_opt,
    specializer: SPECIALIZE_node_re_opt,
    dumper: DUMP_node_re_opt,
    replacer: REPLACE_node_re_opt,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_re_opt_struct>>(),
    children: CHILDREN_node_re_opt,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_re_opt_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_re_opt_struct>>() <= 8);
impl NodeStruct for node_re_opt_struct { #[inline(always)] fn kind() -> &'static NodeKind { &KIND_node_re_opt } }

#[allow(non_upper_case_globals)]
pub static KIND_node_re_rep: NodeKind = NodeKind {
    name: "node_re_rep",
    default_dispatcher: DISPATCH_node_re_rep,
    hash_func: HASH_node_re_rep,
    profile: PROF_node_re_rep,
    specializer: SPECIALIZE_node_re_rep,
    dumper: DUMP_node_re_rep,
    replacer: REPLACE_node_re_rep,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_re_rep_struct>>(),
    children: CHILDREN_node_re_rep,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_re_rep_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_re_rep_struct>>() <= 8);
impl NodeStruct for node_re_rep_struct { #[inline(always)] fn kind() -> &'static NodeKind { &KIND_node_re_rep } }

#[allow(non_upper_case_globals)]
pub static KIND_node_re_rep_cont: NodeKind = NodeKind {
    name: "node_re_rep_cont",
    default_dispatcher: DISPATCH_node_re_rep_cont,
    hash_func: HASH_node_re_rep_cont,
    profile: PROF_node_re_rep_cont,
    specializer: SPECIALIZE_node_re_rep_cont,
    dumper: DUMP_node_re_rep_cont,
    replacer: REPLACE_node_re_rep_cont,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_re_rep_cont_struct>>(),
    children: CHILDREN_node_re_rep_cont,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_re_rep_cont_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_re_rep_cont_struct>>() <= 8);
impl NodeStruct for node_re_rep_cont_struct { #[inline(always)] fn kind() -> &'static NodeKind { &KIND_node_re_rep_cont } }

#[allow(non_upper_case_globals)]
pub static KIND_node_re_look: NodeKind = NodeKind {
    name: "node_re_look",
    default_dispatcher: DISPATCH_node_re_look,
    hash_func: HASH_node_re_look,
    profile: PROF_node_re_look,
    specializer: SPECIALIZE_node_re_look,
    dumper: DUMP_node_re_look,
    replacer: REPLACE_node_re_look,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_re_look_struct>>(),
    children: CHILDREN_node_re_look,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_re_look_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_re_look_struct>>() <= 8);
impl NodeStruct for node_re_look_struct { #[inline(always)] fn kind() -> &'static NodeKind { &KIND_node_re_look } }

#[allow(non_upper_case_globals)]
pub static KIND_node_re_absent: NodeKind = NodeKind {
    name: "node_re_absent",
    default_dispatcher: DISPATCH_node_re_absent,
    hash_func: HASH_node_re_absent,
    profile: PROF_node_re_absent,
    specializer: SPECIALIZE_node_re_absent,
    dumper: DUMP_node_re_absent,
    replacer: REPLACE_node_re_absent,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_re_absent_struct>>(),
    children: CHILDREN_node_re_absent,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_re_absent_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_re_absent_struct>>() <= 8);
impl NodeStruct for node_re_absent_struct { #[inline(always)] fn kind() -> &'static NodeKind { &KIND_node_re_absent } }

#[allow(non_upper_case_globals)]
pub static KIND_node_search: NodeKind = NodeKind {
    name: "node_search",
    default_dispatcher: DISPATCH_node_search,
    hash_func: HASH_node_search,
    profile: PROF_node_search,
    specializer: SPECIALIZE_node_search,
    dumper: DUMP_node_search,
    replacer: REPLACE_node_search,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_search_struct>>(),
    children: CHILDREN_node_search,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_search_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_search_struct>>() <= 8);
impl NodeStruct for node_search_struct { #[inline(always)] fn kind() -> &'static NodeKind { &KIND_node_search } }

#[allow(non_upper_case_globals)]
pub static KIND_node_search_memchr: NodeKind = NodeKind {
    name: "node_search_memchr",
    default_dispatcher: DISPATCH_node_search_memchr,
    hash_func: HASH_node_search_memchr,
    profile: PROF_node_search_memchr,
    specializer: SPECIALIZE_node_search_memchr,
    dumper: DUMP_node_search_memchr,
    replacer: REPLACE_node_search_memchr,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_search_memchr_struct>>(),
    children: CHILDREN_node_search_memchr,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_search_memchr_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_search_memchr_struct>>() <= 8);
impl NodeStruct for node_search_memchr_struct { #[inline(always)] fn kind() -> &'static NodeKind { &KIND_node_search_memchr } }

#[allow(non_upper_case_globals)]
pub static KIND_node_search_memmem: NodeKind = NodeKind {
    name: "node_search_memmem",
    default_dispatcher: DISPATCH_node_search_memmem,
    hash_func: HASH_node_search_memmem,
    profile: PROF_node_search_memmem,
    specializer: SPECIALIZE_node_search_memmem,
    dumper: DUMP_node_search_memmem,
    replacer: REPLACE_node_search_memmem,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_search_memmem_struct>>(),
    children: CHILDREN_node_search_memmem,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_search_memmem_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_search_memmem_struct>>() <= 8);
impl NodeStruct for node_search_memmem_struct { #[inline(always)] fn kind() -> &'static NodeKind { &KIND_node_search_memmem } }

#[allow(non_upper_case_globals)]
pub static KIND_node_search_byteset: NodeKind = NodeKind {
    name: "node_search_byteset",
    default_dispatcher: DISPATCH_node_search_byteset,
    hash_func: HASH_node_search_byteset,
    profile: PROF_node_search_byteset,
    specializer: SPECIALIZE_node_search_byteset,
    dumper: DUMP_node_search_byteset,
    replacer: REPLACE_node_search_byteset,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_search_byteset_struct>>(),
    children: CHILDREN_node_search_byteset,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_search_byteset_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_search_byteset_struct>>() <= 8);
impl NodeStruct for node_search_byteset_struct { #[inline(always)] fn kind() -> &'static NodeKind { &KIND_node_search_byteset } }

#[allow(non_upper_case_globals)]
pub static KIND_node_match_at: NodeKind = NodeKind {
    name: "node_match_at",
    default_dispatcher: DISPATCH_node_match_at,
    hash_func: HASH_node_match_at,
    profile: PROF_node_match_at,
    specializer: SPECIALIZE_node_match_at,
    dumper: DUMP_node_match_at,
    replacer: REPLACE_node_match_at,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_match_at_struct>>(),
    children: CHILDREN_node_match_at,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_match_at_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_match_at_struct>>() <= 8);
impl NodeStruct for node_match_at_struct { #[inline(always)] fn kind() -> &'static NodeKind { &KIND_node_match_at } }


// This file is auto-generated from node.def by astrustgen. Do not edit.

pub type DispatchFn = fn(&mut Ctx, NodePtr) -> i64;
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

pub fn node_allocate<T>(kind: &'static NodeKind, u: T) -> NodePtr {
    let n = Box::into_raw(Box::new(NodeOf { head: NodeHead {
        dispatcher: kind.default_dispatcher, kind, hash: 0,
        flags: if kind.no_inline { NF_NO_INLINE } else { 0 },
        
    }, u })) as NodePtr;
    node_allocated(n);
    n
}

/// Copy the tree under `root` into one block, parents before children in operand order, and
/// return the copy.  The original stays valid (other holders of its nodes are unaffected).
pub fn layout_tree(root: NodePtr) -> NodePtr {
    fn size(n: NodePtr) -> usize {
        if n.is_null() { return 0; }
        let k = unsafe { (*n).head.kind };
        let mut s = k.size;
        (k.children)(n, false, &mut |c| s += size(*c));
        s
    }
    fn copy(n: NodePtr, at: &mut *mut u8) -> NodePtr {
        unsafe {
            let k = (*n).head.kind;
            let d = *at;
            std::ptr::copy_nonoverlapping(n as *const u8, d, k.size);
            *at = d.add(k.size);
            let d = d as NodePtr;
            (*d).head.flags |= NF_LAID;
            (k.children)(d, true, &mut |c| if !c.is_null() { *c = copy(*c, at); });
            d
        }
    }
    if root.is_null() { return root; }
    let mut at = Box::leak(vec![0u64; size(root) / 8].into_boxed_slice()).as_mut_ptr() as *mut u8;
    copy(root, &mut at)
}
pub fn node_laid(n: NodePtr) -> bool { !n.is_null() && unsafe { (*n).head.flags & NF_LAID != 0 } }
/// Specialized code's pointer to child `c` of `n` (contiguous trees).
pub fn child_ptr_expr(n: NodePtr, c: NodePtr) -> String {
    if c.is_null() { "std::ptr::null_mut()".to_string() } else { format!("n.byte_offset({}isize)", c as isize - n as isize) }
}
/// A list operand in specialized code over a contiguous tree: no elements, the parent's address.
#[inline(always)]
pub fn list_base(n: NodePtr) -> NodeList { unsafe { std::slice::from_raw_parts(n as *const NodePtr, 0) } }

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
/// What specialization bakes into the subtree at n (PROF_<name>); each node once per ProfCtx.
pub fn profile_node(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    if n.is_null() || !pc.seen.insert(n as usize) { return 0; }
    unsafe { ((*n).head.kind.profile)(pc, n) }
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
        if n.is_null() || (*n).head.flags & NF_NO_INLINE != 0 || ((*n).head.flags & NF_SPECIALIZING != 0 && sc.reenter == 0) { return "Dyn".to_string(); }
        (*n).head.flags |= NF_SPECIALIZING;
        let t = ((*n).head.kind.specializer)(sc, n);
        (*n).head.flags &= !NF_SPECIALIZING;
        t
    }
}
/// specialize_node for a node being specialized already (a recursion target compiled into itself):
/// its subtree again; the copy's recursive sites see the target in sc.unrolled and call it instead.
pub fn specialize_open(sc: &mut SpecCtx, n: NodePtr) -> String {
    sc.reenter += 1;
    let t = specialize_node(sc, n);
    sc.reenter -= 1;
    unsafe { (*n).head.flags |= NF_SPECIALIZING; }
    t
}
/// Specialize n as a public entry: an exported `SD_<hash>` with the dispatcher's signature.
pub fn specialize_entry(sc: &mut SpecCtx, n: NodePtr) -> String {
    let fname = format!("SD_{:016x}", sc.entry_name(hash_node(n)));
    if sc.emitted.contains(&fname) { return fname; }
    sc.holes.clear();
    sc.begin_entry(n as usize, hash_node(n));
    let t = specialize_node(sc, n);
    let p = sc.rec_suffix(hash_node(n));
    let s = format!("S_{:016x}{}{}", hash_node(n), sc.tag, p);
    if sc.emitted.contains(&format!("R_{:016x}{}{}", hash_node(n), sc.tag, p)) && !sc.emitted.contains(&s) {
        // n calls itself: its one body is S_<hash>::run (R_ calls it directly); SD_ passes the spec-in value on
        specialize_run(sc, &s, &t);
        sc.out.push_str(&format!("#[no_mangle]
pub fn {}(c: &mut Ctx, n: NodePtr) -> i64 {{ {}::run(c, n) }}
", fname, s));
    } else {
        sc.out.push_str(&format!("#[no_mangle]
pub fn {}(c: &mut Ctx, n: NodePtr) -> i64 {{ <{} as Disp>::call(c, n) }}
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
/// A call from inside the specialization of q to q (recursion): `R_<hash>`, a direct call of
/// `S_<hash>::run`, q's entry taking the spec-in value as a parameter (specialize_entry / _finish emit it).
pub fn specialize_rec(sc: &mut SpecCtx, q: NodePtr) -> String {
    let h = hash_node(q);
    let p = sc.rec_suffix(h);
    let name = format!("R_{:016x}{}{}", h, sc.tag, p);
    if !p.is_empty() {
        // the entry's patched body (S_<hash>P::run) holds the values of the one node it was bound to
        if sc.emitted.insert(name.clone()) {
            sc.out.push_str(&format!("pub struct {0};\nimpl Disp for {0} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr) -> i64 {{ if n as usize == &raw const ASTRUST_ROOT as usize {{ S_{1:016x}{2}P::run(c, n) }} else {{ std::hint::cold_path(); <Dyn as Disp>::call(c, n) }} }}\n    #[inline(always)]\n    fn spec(c: &mut Ctx, n: NodePtr) -> i64 {{ if n as usize == &raw const ASTRUST_ROOT as usize {{ S_{1:016x}{2}P::run(c, n) }} else {{ std::hint::cold_path(); <Dyn as Disp>::spec(c, n) }} }}\n}}\n", name, h, sc.tag));
        }
        return name;
    }
    if sc.emitted.insert(name.clone()) {
        sc.rec.push((q as usize, sc.tag.clone()));
        sc.out.push_str(&format!("pub struct {0};\nimpl Disp for {0} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr) -> i64 {{ S_{1:016x}{2}::run(c, n) }}\n    #[inline(always)]\n    fn spec(c: &mut Ctx, n: NodePtr) -> i64 {{ S_{1:016x}{2}::run(c, n) }}\n}}\n", name, h, sc.tag));
    }
    name
}
fn specialize_run(sc: &mut SpecCtx, s: &str, t: &str) {
    sc.out.push_str(&format!("pub struct {0};\nimpl {0} {{\n    #[inline(never)]\n    pub fn run(c: &mut Ctx, n: NodePtr) -> i64 {{ <{1} as Disp>::spec(c, n) }}\n}}\n", s, t));
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
    fn call(c: &mut Ctx, n: NodePtr) -> i64;
    /// call() across a speculation boundary (the embedder only speculates small callees, to a bounded depth).
    #[inline(always)]
    fn spec(c: &mut Ctx, n: NodePtr) -> i64 { Self::call(c, n) }

}
/// A list of children: `DynList` loops over the heads, an SD list type is unrolled.
pub trait DispList {
    fn len(ns: NodeList) -> usize;
    fn nth(c: &mut Ctx, ns: NodeList, i: usize) -> i64;
}
pub struct Dyn;
impl Disp for Dyn {
    #[inline(always)]
    fn call(c: &mut Ctx, n: NodePtr) -> i64 { unsafe { ((*n).head.dispatcher)(c, n) } }
}
pub struct DynList;
impl DispList for DynList {
    #[inline(always)] fn len(ns: NodeList) -> usize { ns.len() }
    #[inline(always)]
    fn nth(c: &mut Ctx, ns: NodeList, i: usize) -> i64 {
        let x = ns[i];
        unsafe { ((*x).head.dispatcher)(c, x) }
    }
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_num_struct {
    pub v: i64,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_add_struct {
    pub l: NodePtr,
    pub r: NodePtr,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_sub_struct {
    pub l: NodePtr,
    pub r: NodePtr,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_mul_struct {
    pub l: NodePtr,
    pub r: NodePtr,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_mod_struct {
    pub l: NodePtr,
    pub r: NodePtr,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_var_struct {
    pub idx: u32,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_set_struct {
    pub idx: u32,
    pub v: NodePtr,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_seq_struct {
    pub xs: NodeList,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_times_struct {
    pub cnt: NodePtr,
    pub body: NodePtr,
}

#[allow(non_snake_case)]
pub fn ALLOC_node_num(v: i64) -> NodePtr {
    let n = node_allocate(&KIND_node_num, node_num_struct { v });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_add(l: NodePtr, r: NodePtr) -> NodePtr {
    let n = node_allocate(&KIND_node_add, node_add_struct { l, r });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_sub(l: NodePtr, r: NodePtr) -> NodePtr {
    let n = node_allocate(&KIND_node_sub, node_sub_struct { l, r });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_mul(l: NodePtr, r: NodePtr) -> NodePtr {
    let n = node_allocate(&KIND_node_mul, node_mul_struct { l, r });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_mod(l: NodePtr, r: NodePtr) -> NodePtr {
    let n = node_allocate(&KIND_node_mod, node_mod_struct { l, r });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_var(idx: u32) -> NodePtr {
    let n = node_allocate(&KIND_node_var, node_var_struct { idx });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_set(idx: u32, v: NodePtr) -> NodePtr {
    let n = node_allocate(&KIND_node_set, node_set_struct { idx, v });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_seq(xs: NodeList) -> NodePtr {
    let n = node_allocate(&KIND_node_seq, node_seq_struct { xs });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_times(cnt: NodePtr, body: NodePtr) -> NodePtr {
    let n = node_allocate(&KIND_node_times, node_times_struct { cnt, body });
    n
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_num(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_num_struct>)).u };

}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_add(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_add_struct>)).u };
    f(&mut u.l);
    f(&mut u.r);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_sub(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_sub_struct>)).u };
    f(&mut u.l);
    f(&mut u.r);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_mul(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_mul_struct>)).u };
    f(&mut u.l);
    f(&mut u.r);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_mod(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_mod_struct>)).u };
    f(&mut u.l);
    f(&mut u.r);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_var(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_var_struct>)).u };

}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_set(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_set_struct>)).u };
    f(&mut u.v);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_seq(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_seq_struct>)).u };
    { if own { u.xs = Box::leak(u.xs.to_vec().into_boxed_slice()); } let l = u.xs; let p = l.as_ptr() as *mut NodePtr; for i in 0..l.len() { unsafe { f(&mut *p.add(i)); } } }
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_times(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_times_struct>)).u };
    f(&mut u.cnt);
    f(&mut u.body);
}

#[allow(non_upper_case_globals)]
pub static KIND_node_num: NodeKind = NodeKind {
    name: "node_num",
    default_dispatcher: DISPATCH_node_num,
    hash_func: HASH_node_num,
    profile: PROF_node_num,
    specializer: SPECIALIZE_node_num,
    dumper: DUMP_node_num,
    replacer: REPLACE_node_num,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_num_struct>>(),
    children: CHILDREN_node_num,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_num_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_num_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_add: NodeKind = NodeKind {
    name: "node_add",
    default_dispatcher: DISPATCH_node_add,
    hash_func: HASH_node_add,
    profile: PROF_node_add,
    specializer: SPECIALIZE_node_add,
    dumper: DUMP_node_add,
    replacer: REPLACE_node_add,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_add_struct>>(),
    children: CHILDREN_node_add,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_add_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_add_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_sub: NodeKind = NodeKind {
    name: "node_sub",
    default_dispatcher: DISPATCH_node_sub,
    hash_func: HASH_node_sub,
    profile: PROF_node_sub,
    specializer: SPECIALIZE_node_sub,
    dumper: DUMP_node_sub,
    replacer: REPLACE_node_sub,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_sub_struct>>(),
    children: CHILDREN_node_sub,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_sub_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_sub_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_mul: NodeKind = NodeKind {
    name: "node_mul",
    default_dispatcher: DISPATCH_node_mul,
    hash_func: HASH_node_mul,
    profile: PROF_node_mul,
    specializer: SPECIALIZE_node_mul,
    dumper: DUMP_node_mul,
    replacer: REPLACE_node_mul,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_mul_struct>>(),
    children: CHILDREN_node_mul,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_mul_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_mul_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_mod: NodeKind = NodeKind {
    name: "node_mod",
    default_dispatcher: DISPATCH_node_mod,
    hash_func: HASH_node_mod,
    profile: PROF_node_mod,
    specializer: SPECIALIZE_node_mod,
    dumper: DUMP_node_mod,
    replacer: REPLACE_node_mod,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_mod_struct>>(),
    children: CHILDREN_node_mod,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_mod_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_mod_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_var: NodeKind = NodeKind {
    name: "node_var",
    default_dispatcher: DISPATCH_node_var,
    hash_func: HASH_node_var,
    profile: PROF_node_var,
    specializer: SPECIALIZE_node_var,
    dumper: DUMP_node_var,
    replacer: REPLACE_node_var,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_var_struct>>(),
    children: CHILDREN_node_var,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_var_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_var_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_set: NodeKind = NodeKind {
    name: "node_set",
    default_dispatcher: DISPATCH_node_set,
    hash_func: HASH_node_set,
    profile: PROF_node_set,
    specializer: SPECIALIZE_node_set,
    dumper: DUMP_node_set,
    replacer: REPLACE_node_set,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_set_struct>>(),
    children: CHILDREN_node_set,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_set_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_set_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_seq: NodeKind = NodeKind {
    name: "node_seq",
    default_dispatcher: DISPATCH_node_seq,
    hash_func: HASH_node_seq,
    profile: PROF_node_seq,
    specializer: SPECIALIZE_node_seq,
    dumper: DUMP_node_seq,
    replacer: REPLACE_node_seq,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_seq_struct>>(),
    children: CHILDREN_node_seq,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_seq_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_seq_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_times: NodeKind = NodeKind {
    name: "node_times",
    default_dispatcher: DISPATCH_node_times,
    hash_func: HASH_node_times,
    profile: PROF_node_times,
    specializer: SPECIALIZE_node_times,
    dumper: DUMP_node_times,
    replacer: REPLACE_node_times,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_times_struct>>(),
    children: CHILDREN_node_times,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_times_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_times_struct>>() <= 8);


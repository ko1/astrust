// This file is auto-generated from node.def by astrustgen. Do not edit.

pub type DispatchFn = fn(&mut Ctx, NodePtr, Fp) -> RV;
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
    pub line: u32,
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
        line: 0,
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
        if n.is_null() || (*n).head.flags & NF_NO_INLINE != 0 || ((*n).head.flags & NF_SPECIALIZING != 0 && sc.reenter == 0) || (*n).head.flags & NF_LAID == 0 { return "Dyn".to_string(); }
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
    if sc.emitted.contains(&fname) || !node_laid(n) { return fname; }
    sc.holes.clear();
    sc.begin_entry(n as usize, hash_node(n));
    let t = specialize_node(sc, n);
    let p = sc.rec_suffix(hash_node(n));
    let s = format!("S_{:016x}{}{}", hash_node(n), sc.tag, p);
    if sc.emitted.contains(&format!("R_{:016x}{}{}", hash_node(n), sc.tag, p)) && !sc.emitted.contains(&s) {
        // n calls itself: its one body is S_<hash>::run (R_ calls it directly); SD_ passes the spec-in value on
        specialize_run(sc, &s, &t);
        sc.out.push_str(&format!("#[no_mangle]
pub fn {}(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ {}::run(c, n, fp, crate::vm::CallArgs::load(c).pass()) }}
", fname, s));
    } else {
        sc.out.push_str(&format!("#[no_mangle]
pub fn {}(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ <{} as Disp>::call(c, n, fp) }}
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
            sc.out.push_str(&format!("pub struct {0};\nimpl Disp for {0} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ crate::vm::native_check(c)?; if n as usize == &raw const ASTRUST_ROOT as usize {{ S_{1:016x}{2}P::run(c, n, fp, crate::vm::CallArgs::load(c).pass()) }} else {{ std::hint::cold_path(); <Dyn as Disp>::call(c, n, fp) }} }}\n    #[inline(always)]\n    fn spec(c: &mut Ctx, n: NodePtr, fp: Fp, a: CallArgs) -> RV {{ crate::vm::native_check(c)?; if n as usize == &raw const ASTRUST_ROOT as usize {{ S_{1:016x}{2}P::run(c, n, fp, a.pass()) }} else {{ std::hint::cold_path(); <Dyn as Disp>::spec(c, n, fp, a) }} }}\n}}\n", name, h, sc.tag));
        }
        return name;
    }
    if sc.emitted.insert(name.clone()) {
        sc.rec.push((q as usize, sc.tag.clone()));
        sc.out.push_str(&format!("pub struct {0};\nimpl Disp for {0} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ crate::vm::native_check(c)?; S_{1:016x}{2}::run(c, n, fp, crate::vm::CallArgs::load(c).pass()) }}\n    #[inline(always)]\n    fn spec(c: &mut Ctx, n: NodePtr, fp: Fp, a: CallArgs) -> RV {{ crate::vm::native_check(c)?; S_{1:016x}{2}::run(c, n, fp, a.pass()) }}\n}}\n", name, h, sc.tag));
    }
    name
}
fn specialize_run(sc: &mut SpecCtx, s: &str, t: &str) {
    sc.out.push_str(&format!("pub struct {0};\nimpl {0} {{\n    #[inline(never)]\n    pub fn run(c: &mut Ctx, n: NodePtr, fp: Fp, a: crate::vm::CallPass) -> RV {{ <{1} as Disp>::spec(c, n, fp, a.unpass()) }}\n}}\n", s, t));
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
    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV;
    /// call() across a speculation boundary (the embedder only speculates small callees, to a bounded depth).
    #[inline(always)]
    fn spec(c: &mut Ctx, n: NodePtr, fp: Fp, a: CallArgs) -> RV { a.store(c); Self::call(c, n, fp) }
    #[inline(always)]
    fn test(c: &mut Ctx, n: NodePtr, fp: Fp) -> Result<bool, Esc> { Ok(Self::call(c, n, fp)?.truthy()) }
}
/// A list of children: `DynList` loops over the heads, an SD list type is unrolled.
pub trait DispList {
    fn len(ns: NodeList) -> usize;
    fn nth(c: &mut Ctx, ns: NodeList, i: usize, fp: Fp) -> RV;
    /// Every child in order, its result to f (EVAL_EACH!).
    fn each<F: FnMut(&mut Ctx, usize, Value) -> Result<(), Esc>>(c: &mut Ctx, ns: NodeList, fp: Fp, f: F) -> Result<(), Esc>;
}
pub struct Dyn;
impl Disp for Dyn {
    #[inline(always)]
    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV { unsafe { ((*n).head.dispatcher)(c, n, fp) } }
}
pub struct DynList;
impl DispList for DynList {
    #[inline(always)] fn len(ns: NodeList) -> usize { ns.len() }
    #[inline(always)]
    fn nth(c: &mut Ctx, ns: NodeList, i: usize, fp: Fp) -> RV {
        let x = ns[i];
        unsafe { ((*x).head.dispatcher)(c, x, fp) }
    }
    #[inline(always)]
    fn each<F: FnMut(&mut Ctx, usize, Value) -> Result<(), Esc>>(c: &mut Ctx, ns: NodeList, fp: Fp, mut f: F) -> Result<(), Esc> {
        // the first ones in a row, as ASTroGen's per-arity dispatchers (short lists are the common case)
        if ns.len() > 0 { let x = ns[0]; let r = unsafe { ((*x).head.dispatcher)(c, x, fp) }?; f(c, 0, r)?; }
        if ns.len() > 1 { let x = ns[1]; let r = unsafe { ((*x).head.dispatcher)(c, x, fp) }?; f(c, 1, r)?; }
        if ns.len() > 2 { let x = ns[2]; let r = unsafe { ((*x).head.dispatcher)(c, x, fp) }?; f(c, 2, r)?; }
        for i in 3..ns.len() {
            let x = ns[i];
            let r = unsafe { ((*x).head.dispatcher)(c, x, fp) }?;
            f(c, i, r)?;
        }
        Ok(())
    }
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_nil_struct {

}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_true_struct {

}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_false_struct {

}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_self_struct {

}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_lit_struct {
    pub v: Value,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_float_struct {
    pub v: f64,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_str_struct {
    pub s: Str,
    pub enc: u32,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_fstr_struct {
    pub s: Str,
    pub enc: u32,
    pub lc: LitCache,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_dstr_struct {
    pub parts: NodeList,
    pub enc: u32,
    pub cap: u32,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_tostr_struct {
    pub v: NodePtr,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_dsym_struct {
    pub s: NodePtr,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_regex_struct {
    pub src: Str,
    pub opts: u32,
    pub lc: LitCache,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_dregex_struct {
    pub s: NodePtr,
    pub opts: u32,
    pub once: bool,
    pub lc: LitCache,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_biglit_struct {
    pub digits: Str,
    pub lc: LitCache,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_ary_struct {
    pub elems: NodeList,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_ary_build_struct {
    pub elems: NodeList,
    pub kinds: Str,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_hash_struct {
    pub items: NodeList,
    pub kinds: Str,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_range_struct {
    pub b: NodePtr,
    pub e: NodePtr,
    pub excl: bool,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_range_lit_struct {
    pub b: Value,
    pub e: Value,
    pub excl: bool,
    pub lc: LitCache,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_lvar_struct {
    pub idx: u32,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_lasgn_struct {
    pub idx: u32,
    pub v: NodePtr,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_dvar_struct {
    pub hops: u32,
    pub idx: u32,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_dasgn_struct {
    pub hops: u32,
    pub idx: u32,
    pub v: NodePtr,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_ivar_struct {
    pub name: Sym,
    pub ic: IvarCache,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_iasgn_struct {
    pub name: Sym,
    pub v: NodePtr,
    pub ic: IvarCache,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_ivar_arith_struct {
    pub name: Sym,
    pub sub: bool,
    pub v: NodePtr,
    pub ic: IvarCache,
    pub cc: OpCc,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_gvar_struct {
    pub name: Sym,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_gasgn_struct {
    pub name: Sym,
    pub v: NodePtr,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_cvar_struct {
    pub name: Sym,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_cvasgn_struct {
    pub name: Sym,
    pub v: NodePtr,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_const_struct {
    pub name: Sym,
    pub cc: ConstCache,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_const_path_struct {
    pub parent: NodePtr,
    pub name: Sym,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_top_const_struct {
    pub name: Sym,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_cdecl_struct {
    pub name: Sym,
    pub v: NodePtr,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_cdecl_path_struct {
    pub parent: NodePtr,
    pub name: Sym,
    pub v: NodePtr,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_errinfo_struct {

}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_nth_ref_struct {
    pub nth: u32,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_back_ref_struct {
    pub kind: u32,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_block_struct {
    pub stmts: NodeList,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_line_struct {
    pub body: NodePtr,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_trace_struct {
    pub ev: u32,
    pub body: NodePtr,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_if_struct {
    pub cond: NodePtr,
    pub then_b: NodePtr,
    pub else_b: NodePtr,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_and_struct {
    pub a: NodePtr,
    pub b: NodePtr,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_or_struct {
    pub a: NodePtr,
    pub b: NodePtr,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_not_struct {
    pub v: NodePtr,
    pub cc: OpCc,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_while_struct {
    pub cond: NodePtr,
    pub body: NodePtr,
    pub until: bool,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_dowhile_struct {
    pub cond: NodePtr,
    pub body: NodePtr,
    pub until: bool,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_loop_break_struct {
    pub v: NodePtr,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_loop_next_struct {
    pub v: NodePtr,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_break_struct {
    pub v: NodePtr,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_next_struct {
    pub v: NodePtr,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_return_struct {
    pub v: NodePtr,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_retry_struct {

}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_redo_struct {

}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_rescue_struct {
    pub body: NodePtr,
    pub clauses: NodeList,
    pub else_b: NodePtr,
    pub has_else: bool,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_resbody_struct {
    pub classes: NodeList,
    pub kinds: Str,
    pub assign: NodePtr,
    pub body: NodePtr,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_ensure_struct {
    pub body: NodePtr,
    pub ens: NodePtr,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_when_struct {
    pub subj_slot: u32,
    pub pats: NodeList,
    pub kinds: Str,
    pub cc: CallCache,
    pub km: KindMemo,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_when_lit_struct {
    pub subj_slot: u32,
    pub v: Value,
    pub cc: CallCache,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_defined_struct {
    pub kind: u32,
    pub name: Sym,
    pub recv: NodePtr,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_call_struct {
    pub recv: NodePtr,
    pub args: NodeList,
    pub mid: Sym,
    pub flags: u32,
    pub cc: CallCache,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_call_blk_struct {
    pub recv: NodePtr,
    pub args: NodeList,
    pub mid: Sym,
    pub flags: u32,
    pub blk: IseqRef,
    pub cc: CallCache,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_call_kw_struct {
    pub recv: NodePtr,
    pub args: NodeList,
    pub mid: Sym,
    pub flags: u32,
    pub ci: CiRef,
    pub cc: CallCache,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_call_x_struct {
    pub recv: NodePtr,
    pub args: NodeList,
    pub mid: Sym,
    pub flags: u32,
    pub ci: CiRef,
    pub cc: CallCache,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_send_sym_struct {
    pub recv: NodePtr,
    pub smid: Sym,
    pub call: NodePtr,
    pub general: NodePtr,
    pub g: SendGuard,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_attrasgn_struct {
    pub recv: NodePtr,
    pub args: NodeList,
    pub mid: Sym,
    pub flags: u32,
    pub cc: CallCache,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_super_struct {
    pub args: NodeList,
    pub ci: CiRef,
    pub zsuper: bool,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_yield_struct {
    pub args: NodeList,
    pub yc: YieldCache,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_yield_x_struct {
    pub args: NodeList,
    pub ci: CiRef,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_block_given_struct {

}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_lambda_struct {
    pub blk: IseqRef,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_opt_plus_struct {
    pub a: NodePtr,
    pub b: NodePtr,
    pub cc: OpCc,
    pub tf: TyFb,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_opt_minus_struct {
    pub a: NodePtr,
    pub b: NodePtr,
    pub cc: OpCc,
    pub tf: TyFb,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_opt_mul_struct {
    pub a: NodePtr,
    pub b: NodePtr,
    pub cc: OpCc,
    pub tf: TyFb,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_opt_div_struct {
    pub a: NodePtr,
    pub b: NodePtr,
    pub cc: OpCc,
    pub tf: TyFb,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_opt_pow_struct {
    pub a: NodePtr,
    pub b: NodePtr,
    pub cc: OpCc,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_opt_mod_struct {
    pub a: NodePtr,
    pub b: NodePtr,
    pub cc: OpCc,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_opt_lt_struct {
    pub a: NodePtr,
    pub b: NodePtr,
    pub cc: OpCc,
    pub tf: TyFb,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_opt_le_struct {
    pub a: NodePtr,
    pub b: NodePtr,
    pub cc: OpCc,
    pub tf: TyFb,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_opt_gt_struct {
    pub a: NodePtr,
    pub b: NodePtr,
    pub cc: OpCc,
    pub tf: TyFb,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_opt_ge_struct {
    pub a: NodePtr,
    pub b: NodePtr,
    pub cc: OpCc,
    pub tf: TyFb,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_opt_eq_struct {
    pub a: NodePtr,
    pub b: NodePtr,
    pub cc: OpCc,
    pub tf: TyFb,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_opt_neq_struct {
    pub a: NodePtr,
    pub b: NodePtr,
    pub cc: OpCc,
    pub tf: TyFb,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_opt_ltlt_struct {
    pub a: NodePtr,
    pub b: NodePtr,
    pub cc: OpCc,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_opt_gtgt_struct {
    pub a: NodePtr,
    pub b: NodePtr,
    pub cc: OpCc,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_opt_xor_struct {
    pub a: NodePtr,
    pub b: NodePtr,
    pub cc: OpCc,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_opt_and_struct {
    pub a: NodePtr,
    pub b: NodePtr,
    pub cc: OpCc,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_opt_or_struct {
    pub a: NodePtr,
    pub b: NodePtr,
    pub cc: OpCc,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_opt_aref_struct {
    pub a: NodePtr,
    pub b: NodePtr,
    pub cc: OpCc,
    pub tf: TyFb,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_opt_aset_struct {
    pub a: NodePtr,
    pub b: NodePtr,
    pub v: NodePtr,
    pub cc: OpCc,
    pub tf: TyFb,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_method_entry_struct {
    pub nlocals: u32,
    pub nparams: u32,
    pub flags: u32,
    pub body: NodePtr,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_block_entry_struct {
    pub nlocals: u32,
    pub nparams: u32,
    pub flags: u32,
    pub body: NodePtr,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_def_struct {
    pub mid: Sym,
    pub body: IseqRef,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_sdef_struct {
    pub recv: NodePtr,
    pub mid: Sym,
    pub body: IseqRef,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_class_struct {
    pub cpath: NodePtr,
    pub name: Sym,
    pub superclass: NodePtr,
    pub body: IseqRef,
    pub has_cpath: bool,
    pub has_super: bool,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_module_struct {
    pub cpath: NodePtr,
    pub name: Sym,
    pub body: IseqRef,
    pub has_cpath: bool,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_sclass_struct {
    pub recv: NodePtr,
    pub body: IseqRef,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_alias_struct {
    pub newname: NodePtr,
    pub oldname: NodePtr,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_valias_struct {
    pub newname: Sym,
    pub oldname: Sym,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_undef_struct {
    pub names: NodeList,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_masgn_struct {
    pub rhs: NodePtr,
    pub tmp: u32,
    pub targets: NodeList,
    pub splat_rhs: bool,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_masgn_elem_struct {
    pub tmp: u32,
    pub idx: u32,
    pub kind: u32,
    pub post: u32,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_unsupported_struct {
    pub what: Str,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_special_struct {
    pub kind: u32,
    pub arg: NodePtr,
}

#[allow(non_snake_case)]
pub fn ALLOC_node_nil() -> NodePtr {
    let n = node_allocate(&KIND_node_nil, node_nil_struct {  });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_true() -> NodePtr {
    let n = node_allocate(&KIND_node_true, node_true_struct {  });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_false() -> NodePtr {
    let n = node_allocate(&KIND_node_false, node_false_struct {  });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_self() -> NodePtr {
    let n = node_allocate(&KIND_node_self, node_self_struct {  });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_lit(v: Value) -> NodePtr {
    let n = node_allocate(&KIND_node_lit, node_lit_struct { v });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_float(v: f64) -> NodePtr {
    let n = node_allocate(&KIND_node_float, node_float_struct { v });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_str(s: Str, enc: u32) -> NodePtr {
    let n = node_allocate(&KIND_node_str, node_str_struct { s, enc });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_fstr(s: Str, enc: u32) -> NodePtr {
    let n = node_allocate(&KIND_node_fstr, node_fstr_struct { s, enc, lc: Default::default() });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_dstr(parts: NodeList, enc: u32, cap: u32) -> NodePtr {
    let n = node_allocate(&KIND_node_dstr, node_dstr_struct { parts, enc, cap });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_tostr(v: NodePtr) -> NodePtr {
    let n = node_allocate(&KIND_node_tostr, node_tostr_struct { v });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_dsym(s: NodePtr) -> NodePtr {
    let n = node_allocate(&KIND_node_dsym, node_dsym_struct { s });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_regex(src: Str, opts: u32) -> NodePtr {
    let n = node_allocate(&KIND_node_regex, node_regex_struct { src, opts, lc: Default::default() });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_dregex(s: NodePtr, opts: u32, once: bool) -> NodePtr {
    let n = node_allocate(&KIND_node_dregex, node_dregex_struct { s, opts, once, lc: Default::default() });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_biglit(digits: Str) -> NodePtr {
    let n = node_allocate(&KIND_node_biglit, node_biglit_struct { digits, lc: Default::default() });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_ary(elems: NodeList) -> NodePtr {
    let n = node_allocate(&KIND_node_ary, node_ary_struct { elems });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_ary_build(elems: NodeList, kinds: Str) -> NodePtr {
    let n = node_allocate(&KIND_node_ary_build, node_ary_build_struct { elems, kinds });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_hash(items: NodeList, kinds: Str) -> NodePtr {
    let n = node_allocate(&KIND_node_hash, node_hash_struct { items, kinds });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_range(b: NodePtr, e: NodePtr, excl: bool) -> NodePtr {
    let n = node_allocate(&KIND_node_range, node_range_struct { b, e, excl });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_range_lit(b: Value, e: Value, excl: bool) -> NodePtr {
    let n = node_allocate(&KIND_node_range_lit, node_range_lit_struct { b, e, excl, lc: Default::default() });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_lvar(idx: u32) -> NodePtr {
    let n = node_allocate(&KIND_node_lvar, node_lvar_struct { idx });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_lasgn(idx: u32, v: NodePtr) -> NodePtr {
    let n = node_allocate(&KIND_node_lasgn, node_lasgn_struct { idx, v });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_dvar(hops: u32, idx: u32) -> NodePtr {
    let n = node_allocate(&KIND_node_dvar, node_dvar_struct { hops, idx });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_dasgn(hops: u32, idx: u32, v: NodePtr) -> NodePtr {
    let n = node_allocate(&KIND_node_dasgn, node_dasgn_struct { hops, idx, v });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_ivar(name: Sym) -> NodePtr {
    let n = node_allocate(&KIND_node_ivar, node_ivar_struct { name, ic: Default::default() });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_iasgn(name: Sym, v: NodePtr) -> NodePtr {
    let n = node_allocate(&KIND_node_iasgn, node_iasgn_struct { name, v, ic: Default::default() });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_ivar_arith(name: Sym, sub: bool, v: NodePtr, line: u32) -> NodePtr {
    let n = node_allocate(&KIND_node_ivar_arith, node_ivar_arith_struct { name, sub, v, ic: Default::default(), cc: Default::default() });
    unsafe { (*n).head.line = line; } n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_gvar(name: Sym) -> NodePtr {
    let n = node_allocate(&KIND_node_gvar, node_gvar_struct { name });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_gasgn(name: Sym, v: NodePtr) -> NodePtr {
    let n = node_allocate(&KIND_node_gasgn, node_gasgn_struct { name, v });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_cvar(name: Sym) -> NodePtr {
    let n = node_allocate(&KIND_node_cvar, node_cvar_struct { name });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_cvasgn(name: Sym, v: NodePtr) -> NodePtr {
    let n = node_allocate(&KIND_node_cvasgn, node_cvasgn_struct { name, v });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_const(name: Sym, line: u32) -> NodePtr {
    let n = node_allocate(&KIND_node_const, node_const_struct { name, cc: Default::default() });
    unsafe { (*n).head.line = line; } n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_const_path(parent: NodePtr, name: Sym, line: u32) -> NodePtr {
    let n = node_allocate(&KIND_node_const_path, node_const_path_struct { parent, name });
    unsafe { (*n).head.line = line; } n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_top_const(name: Sym) -> NodePtr {
    let n = node_allocate(&KIND_node_top_const, node_top_const_struct { name });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_cdecl(name: Sym, v: NodePtr) -> NodePtr {
    let n = node_allocate(&KIND_node_cdecl, node_cdecl_struct { name, v });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_cdecl_path(parent: NodePtr, name: Sym, v: NodePtr) -> NodePtr {
    let n = node_allocate(&KIND_node_cdecl_path, node_cdecl_path_struct { parent, name, v });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_errinfo() -> NodePtr {
    let n = node_allocate(&KIND_node_errinfo, node_errinfo_struct {  });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_nth_ref(nth: u32) -> NodePtr {
    let n = node_allocate(&KIND_node_nth_ref, node_nth_ref_struct { nth });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_back_ref(kind: u32) -> NodePtr {
    let n = node_allocate(&KIND_node_back_ref, node_back_ref_struct { kind });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_block(stmts: NodeList) -> NodePtr {
    let n = node_allocate(&KIND_node_block, node_block_struct { stmts });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_line(line: u32, body: NodePtr) -> NodePtr {
    let n = node_allocate(&KIND_node_line, node_line_struct { body });
    unsafe { (*n).head.line = line; } n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_trace(ev: u32, line: u32, body: NodePtr) -> NodePtr {
    let n = node_allocate(&KIND_node_trace, node_trace_struct { ev, body });
    unsafe { (*n).head.line = line; } n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_if(cond: NodePtr, then_b: NodePtr, else_b: NodePtr) -> NodePtr {
    let n = node_allocate(&KIND_node_if, node_if_struct { cond, then_b, else_b });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_and(a: NodePtr, b: NodePtr) -> NodePtr {
    let n = node_allocate(&KIND_node_and, node_and_struct { a, b });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_or(a: NodePtr, b: NodePtr) -> NodePtr {
    let n = node_allocate(&KIND_node_or, node_or_struct { a, b });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_not(v: NodePtr) -> NodePtr {
    let n = node_allocate(&KIND_node_not, node_not_struct { v, cc: Default::default() });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_while(cond: NodePtr, body: NodePtr, until: bool) -> NodePtr {
    let n = node_allocate(&KIND_node_while, node_while_struct { cond, body, until });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_dowhile(cond: NodePtr, body: NodePtr, until: bool) -> NodePtr {
    let n = node_allocate(&KIND_node_dowhile, node_dowhile_struct { cond, body, until });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_loop_break(v: NodePtr) -> NodePtr {
    let n = node_allocate(&KIND_node_loop_break, node_loop_break_struct { v });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_loop_next(v: NodePtr) -> NodePtr {
    let n = node_allocate(&KIND_node_loop_next, node_loop_next_struct { v });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_break(v: NodePtr) -> NodePtr {
    let n = node_allocate(&KIND_node_break, node_break_struct { v });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_next(v: NodePtr) -> NodePtr {
    let n = node_allocate(&KIND_node_next, node_next_struct { v });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_return(v: NodePtr) -> NodePtr {
    let n = node_allocate(&KIND_node_return, node_return_struct { v });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_retry() -> NodePtr {
    let n = node_allocate(&KIND_node_retry, node_retry_struct {  });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_redo() -> NodePtr {
    let n = node_allocate(&KIND_node_redo, node_redo_struct {  });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_rescue(body: NodePtr, clauses: NodeList, else_b: NodePtr, has_else: bool) -> NodePtr {
    let n = node_allocate(&KIND_node_rescue, node_rescue_struct { body, clauses, else_b, has_else });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_resbody(classes: NodeList, kinds: Str, assign: NodePtr, body: NodePtr) -> NodePtr {
    let n = node_allocate(&KIND_node_resbody, node_resbody_struct { classes, kinds, assign, body });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_ensure(body: NodePtr, ens: NodePtr) -> NodePtr {
    let n = node_allocate(&KIND_node_ensure, node_ensure_struct { body, ens });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_when(subj_slot: u32, pats: NodeList, kinds: Str) -> NodePtr {
    let n = node_allocate(&KIND_node_when, node_when_struct { subj_slot, pats, kinds, cc: Default::default(), km: Default::default() });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_when_lit(subj_slot: u32, v: Value) -> NodePtr {
    let n = node_allocate(&KIND_node_when_lit, node_when_lit_struct { subj_slot, v, cc: Default::default() });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_defined(kind: u32, name: Sym, recv: NodePtr) -> NodePtr {
    let n = node_allocate(&KIND_node_defined, node_defined_struct { kind, name, recv });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_call(recv: NodePtr, args: NodeList, mid: Sym, flags: u32, line: u32) -> NodePtr {
    let n = node_allocate(&KIND_node_call, node_call_struct { recv, args, mid, flags, cc: Default::default() });
    unsafe { (*n).head.line = line; } n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_call_blk(recv: NodePtr, args: NodeList, mid: Sym, flags: u32, line: u32, blk: IseqRef) -> NodePtr {
    let n = node_allocate(&KIND_node_call_blk, node_call_blk_struct { recv, args, mid, flags, blk, cc: Default::default() });
    unsafe { (*n).head.line = line; } n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_call_kw(recv: NodePtr, args: NodeList, mid: Sym, flags: u32, line: u32, ci: CiRef) -> NodePtr {
    let n = node_allocate(&KIND_node_call_kw, node_call_kw_struct { recv, args, mid, flags, ci, cc: Default::default() });
    unsafe { (*n).head.line = line; } n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_call_x(recv: NodePtr, args: NodeList, mid: Sym, flags: u32, line: u32, ci: CiRef) -> NodePtr {
    let n = node_allocate(&KIND_node_call_x, node_call_x_struct { recv, args, mid, flags, ci, cc: Default::default() });
    unsafe { (*n).head.line = line; } n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_send_sym(recv: NodePtr, smid: Sym, call: NodePtr, general: NodePtr) -> NodePtr {
    let n = node_allocate(&KIND_node_send_sym, node_send_sym_struct { recv, smid, call, general, g: Default::default() });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_attrasgn(recv: NodePtr, args: NodeList, mid: Sym, flags: u32, line: u32) -> NodePtr {
    let n = node_allocate(&KIND_node_attrasgn, node_attrasgn_struct { recv, args, mid, flags, cc: Default::default() });
    unsafe { (*n).head.line = line; } n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_super(args: NodeList, line: u32, ci: CiRef, zsuper: bool) -> NodePtr {
    let n = node_allocate(&KIND_node_super, node_super_struct { args, ci, zsuper });
    unsafe { (*n).head.line = line; } n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_yield(args: NodeList) -> NodePtr {
    let n = node_allocate(&KIND_node_yield, node_yield_struct { args, yc: Default::default() });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_yield_x(args: NodeList, ci: CiRef) -> NodePtr {
    let n = node_allocate(&KIND_node_yield_x, node_yield_x_struct { args, ci });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_block_given() -> NodePtr {
    let n = node_allocate(&KIND_node_block_given, node_block_given_struct {  });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_lambda(blk: IseqRef) -> NodePtr {
    let n = node_allocate(&KIND_node_lambda, node_lambda_struct { blk });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_opt_plus(a: NodePtr, b: NodePtr, line: u32) -> NodePtr {
    let n = node_allocate(&KIND_node_opt_plus, node_opt_plus_struct { a, b, cc: Default::default(), tf: Default::default() });
    unsafe { (*n).head.line = line; } n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_opt_minus(a: NodePtr, b: NodePtr, line: u32) -> NodePtr {
    let n = node_allocate(&KIND_node_opt_minus, node_opt_minus_struct { a, b, cc: Default::default(), tf: Default::default() });
    unsafe { (*n).head.line = line; } n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_opt_mul(a: NodePtr, b: NodePtr, line: u32) -> NodePtr {
    let n = node_allocate(&KIND_node_opt_mul, node_opt_mul_struct { a, b, cc: Default::default(), tf: Default::default() });
    unsafe { (*n).head.line = line; } n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_opt_div(a: NodePtr, b: NodePtr, line: u32) -> NodePtr {
    let n = node_allocate(&KIND_node_opt_div, node_opt_div_struct { a, b, cc: Default::default(), tf: Default::default() });
    unsafe { (*n).head.line = line; } n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_opt_pow(a: NodePtr, b: NodePtr, line: u32) -> NodePtr {
    let n = node_allocate(&KIND_node_opt_pow, node_opt_pow_struct { a, b, cc: Default::default() });
    unsafe { (*n).head.line = line; } n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_opt_mod(a: NodePtr, b: NodePtr, line: u32) -> NodePtr {
    let n = node_allocate(&KIND_node_opt_mod, node_opt_mod_struct { a, b, cc: Default::default() });
    unsafe { (*n).head.line = line; } n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_opt_lt(a: NodePtr, b: NodePtr, line: u32) -> NodePtr {
    let n = node_allocate(&KIND_node_opt_lt, node_opt_lt_struct { a, b, cc: Default::default(), tf: Default::default() });
    unsafe { (*n).head.line = line; } n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_opt_le(a: NodePtr, b: NodePtr, line: u32) -> NodePtr {
    let n = node_allocate(&KIND_node_opt_le, node_opt_le_struct { a, b, cc: Default::default(), tf: Default::default() });
    unsafe { (*n).head.line = line; } n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_opt_gt(a: NodePtr, b: NodePtr, line: u32) -> NodePtr {
    let n = node_allocate(&KIND_node_opt_gt, node_opt_gt_struct { a, b, cc: Default::default(), tf: Default::default() });
    unsafe { (*n).head.line = line; } n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_opt_ge(a: NodePtr, b: NodePtr, line: u32) -> NodePtr {
    let n = node_allocate(&KIND_node_opt_ge, node_opt_ge_struct { a, b, cc: Default::default(), tf: Default::default() });
    unsafe { (*n).head.line = line; } n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_opt_eq(a: NodePtr, b: NodePtr, line: u32) -> NodePtr {
    let n = node_allocate(&KIND_node_opt_eq, node_opt_eq_struct { a, b, cc: Default::default(), tf: Default::default() });
    unsafe { (*n).head.line = line; } n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_opt_neq(a: NodePtr, b: NodePtr, line: u32) -> NodePtr {
    let n = node_allocate(&KIND_node_opt_neq, node_opt_neq_struct { a, b, cc: Default::default(), tf: Default::default() });
    unsafe { (*n).head.line = line; } n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_opt_ltlt(a: NodePtr, b: NodePtr, line: u32) -> NodePtr {
    let n = node_allocate(&KIND_node_opt_ltlt, node_opt_ltlt_struct { a, b, cc: Default::default() });
    unsafe { (*n).head.line = line; } n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_opt_gtgt(a: NodePtr, b: NodePtr, line: u32) -> NodePtr {
    let n = node_allocate(&KIND_node_opt_gtgt, node_opt_gtgt_struct { a, b, cc: Default::default() });
    unsafe { (*n).head.line = line; } n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_opt_xor(a: NodePtr, b: NodePtr, line: u32) -> NodePtr {
    let n = node_allocate(&KIND_node_opt_xor, node_opt_xor_struct { a, b, cc: Default::default() });
    unsafe { (*n).head.line = line; } n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_opt_and(a: NodePtr, b: NodePtr, line: u32) -> NodePtr {
    let n = node_allocate(&KIND_node_opt_and, node_opt_and_struct { a, b, cc: Default::default() });
    unsafe { (*n).head.line = line; } n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_opt_or(a: NodePtr, b: NodePtr, line: u32) -> NodePtr {
    let n = node_allocate(&KIND_node_opt_or, node_opt_or_struct { a, b, cc: Default::default() });
    unsafe { (*n).head.line = line; } n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_opt_aref(a: NodePtr, b: NodePtr, line: u32) -> NodePtr {
    let n = node_allocate(&KIND_node_opt_aref, node_opt_aref_struct { a, b, cc: Default::default(), tf: Default::default() });
    unsafe { (*n).head.line = line; } n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_opt_aset(a: NodePtr, b: NodePtr, v: NodePtr, line: u32) -> NodePtr {
    let n = node_allocate(&KIND_node_opt_aset, node_opt_aset_struct { a, b, v, cc: Default::default(), tf: Default::default() });
    unsafe { (*n).head.line = line; } n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_method_entry(nlocals: u32, nparams: u32, flags: u32, line: u32, body: NodePtr) -> NodePtr {
    let n = node_allocate(&KIND_node_method_entry, node_method_entry_struct { nlocals, nparams, flags, body });
    unsafe { (*n).head.line = line; } n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_block_entry(nlocals: u32, nparams: u32, flags: u32, line: u32, body: NodePtr) -> NodePtr {
    let n = node_allocate(&KIND_node_block_entry, node_block_entry_struct { nlocals, nparams, flags, body });
    unsafe { (*n).head.line = line; } n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_def(mid: Sym, body: IseqRef) -> NodePtr {
    let n = node_allocate(&KIND_node_def, node_def_struct { mid, body });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_sdef(recv: NodePtr, mid: Sym, body: IseqRef) -> NodePtr {
    let n = node_allocate(&KIND_node_sdef, node_sdef_struct { recv, mid, body });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_class(cpath: NodePtr, name: Sym, superclass: NodePtr, body: IseqRef, has_cpath: bool, has_super: bool) -> NodePtr {
    let n = node_allocate(&KIND_node_class, node_class_struct { cpath, name, superclass, body, has_cpath, has_super });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_module(cpath: NodePtr, name: Sym, body: IseqRef, has_cpath: bool) -> NodePtr {
    let n = node_allocate(&KIND_node_module, node_module_struct { cpath, name, body, has_cpath });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_sclass(recv: NodePtr, body: IseqRef) -> NodePtr {
    let n = node_allocate(&KIND_node_sclass, node_sclass_struct { recv, body });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_alias(newname: NodePtr, oldname: NodePtr) -> NodePtr {
    let n = node_allocate(&KIND_node_alias, node_alias_struct { newname, oldname });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_valias(newname: Sym, oldname: Sym) -> NodePtr {
    let n = node_allocate(&KIND_node_valias, node_valias_struct { newname, oldname });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_undef(names: NodeList) -> NodePtr {
    let n = node_allocate(&KIND_node_undef, node_undef_struct { names });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_masgn(rhs: NodePtr, tmp: u32, targets: NodeList, splat_rhs: bool) -> NodePtr {
    let n = node_allocate(&KIND_node_masgn, node_masgn_struct { rhs, tmp, targets, splat_rhs });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_masgn_elem(tmp: u32, idx: u32, kind: u32, post: u32) -> NodePtr {
    let n = node_allocate(&KIND_node_masgn_elem, node_masgn_elem_struct { tmp, idx, kind, post });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_unsupported(what: Str) -> NodePtr {
    let n = node_allocate(&KIND_node_unsupported, node_unsupported_struct { what });
    n
}

#[allow(non_snake_case)]
pub fn ALLOC_node_special(kind: u32, arg: NodePtr) -> NodePtr {
    let n = node_allocate(&KIND_node_special, node_special_struct { kind, arg });
    n
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_nil(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_nil_struct>)).u };

}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_true(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_true_struct>)).u };

}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_false(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_false_struct>)).u };

}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_self(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_self_struct>)).u };

}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_lit(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_lit_struct>)).u };

}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_float(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_float_struct>)).u };

}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_str(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_str_struct>)).u };

}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_fstr(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_fstr_struct>)).u };

}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_dstr(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_dstr_struct>)).u };
    { if own { u.parts = Box::leak(u.parts.to_vec().into_boxed_slice()); } let l = u.parts; let p = l.as_ptr() as *mut NodePtr; for i in 0..l.len() { unsafe { f(&mut *p.add(i)); } } }
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_tostr(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_tostr_struct>)).u };
    f(&mut u.v);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_dsym(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_dsym_struct>)).u };
    f(&mut u.s);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_regex(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_regex_struct>)).u };

}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_dregex(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_dregex_struct>)).u };
    f(&mut u.s);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_biglit(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_biglit_struct>)).u };

}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_ary(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_ary_struct>)).u };
    { if own { u.elems = Box::leak(u.elems.to_vec().into_boxed_slice()); } let l = u.elems; let p = l.as_ptr() as *mut NodePtr; for i in 0..l.len() { unsafe { f(&mut *p.add(i)); } } }
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_ary_build(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_ary_build_struct>)).u };
    { if own { u.elems = Box::leak(u.elems.to_vec().into_boxed_slice()); } let l = u.elems; let p = l.as_ptr() as *mut NodePtr; for i in 0..l.len() { unsafe { f(&mut *p.add(i)); } } }
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_hash(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_hash_struct>)).u };
    { if own { u.items = Box::leak(u.items.to_vec().into_boxed_slice()); } let l = u.items; let p = l.as_ptr() as *mut NodePtr; for i in 0..l.len() { unsafe { f(&mut *p.add(i)); } } }
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_range(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_range_struct>)).u };
    f(&mut u.b);
    f(&mut u.e);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_range_lit(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_range_lit_struct>)).u };

}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_lvar(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_lvar_struct>)).u };

}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_lasgn(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_lasgn_struct>)).u };
    f(&mut u.v);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_dvar(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_dvar_struct>)).u };

}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_dasgn(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_dasgn_struct>)).u };
    f(&mut u.v);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_ivar(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_ivar_struct>)).u };

}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_iasgn(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_iasgn_struct>)).u };
    f(&mut u.v);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_ivar_arith(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_ivar_arith_struct>)).u };
    f(&mut u.v);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_gvar(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_gvar_struct>)).u };

}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_gasgn(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_gasgn_struct>)).u };
    f(&mut u.v);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_cvar(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_cvar_struct>)).u };

}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_cvasgn(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_cvasgn_struct>)).u };
    f(&mut u.v);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_const(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_const_struct>)).u };

}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_const_path(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_const_path_struct>)).u };
    f(&mut u.parent);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_top_const(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_top_const_struct>)).u };

}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_cdecl(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_cdecl_struct>)).u };
    f(&mut u.v);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_cdecl_path(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_cdecl_path_struct>)).u };
    f(&mut u.parent);
    f(&mut u.v);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_errinfo(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_errinfo_struct>)).u };

}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_nth_ref(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_nth_ref_struct>)).u };

}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_back_ref(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_back_ref_struct>)).u };

}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_block(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_block_struct>)).u };
    { if own { u.stmts = Box::leak(u.stmts.to_vec().into_boxed_slice()); } let l = u.stmts; let p = l.as_ptr() as *mut NodePtr; for i in 0..l.len() { unsafe { f(&mut *p.add(i)); } } }
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_line(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_line_struct>)).u };
    f(&mut u.body);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_trace(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_trace_struct>)).u };
    f(&mut u.body);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_if(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_if_struct>)).u };
    f(&mut u.cond);
    f(&mut u.then_b);
    f(&mut u.else_b);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_and(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_and_struct>)).u };
    f(&mut u.a);
    f(&mut u.b);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_or(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_or_struct>)).u };
    f(&mut u.a);
    f(&mut u.b);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_not(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_not_struct>)).u };
    f(&mut u.v);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_while(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_while_struct>)).u };
    f(&mut u.cond);
    f(&mut u.body);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_dowhile(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_dowhile_struct>)).u };
    f(&mut u.cond);
    f(&mut u.body);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_loop_break(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_loop_break_struct>)).u };
    f(&mut u.v);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_loop_next(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_loop_next_struct>)).u };
    f(&mut u.v);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_break(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_break_struct>)).u };
    f(&mut u.v);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_next(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_next_struct>)).u };
    f(&mut u.v);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_return(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_return_struct>)).u };
    f(&mut u.v);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_retry(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_retry_struct>)).u };

}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_redo(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_redo_struct>)).u };

}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_rescue(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_rescue_struct>)).u };
    f(&mut u.body);
    { if own { u.clauses = Box::leak(u.clauses.to_vec().into_boxed_slice()); } let l = u.clauses; let p = l.as_ptr() as *mut NodePtr; for i in 0..l.len() { unsafe { f(&mut *p.add(i)); } } }
    f(&mut u.else_b);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_resbody(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_resbody_struct>)).u };
    { if own { u.classes = Box::leak(u.classes.to_vec().into_boxed_slice()); } let l = u.classes; let p = l.as_ptr() as *mut NodePtr; for i in 0..l.len() { unsafe { f(&mut *p.add(i)); } } }
    f(&mut u.assign);
    f(&mut u.body);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_ensure(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_ensure_struct>)).u };
    f(&mut u.body);
    f(&mut u.ens);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_when(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_when_struct>)).u };
    { if own { u.pats = Box::leak(u.pats.to_vec().into_boxed_slice()); } let l = u.pats; let p = l.as_ptr() as *mut NodePtr; for i in 0..l.len() { unsafe { f(&mut *p.add(i)); } } }
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_when_lit(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_when_lit_struct>)).u };

}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_defined(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_defined_struct>)).u };
    f(&mut u.recv);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_call(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_call_struct>)).u };
    f(&mut u.recv);
    { if own { u.args = Box::leak(u.args.to_vec().into_boxed_slice()); } let l = u.args; let p = l.as_ptr() as *mut NodePtr; for i in 0..l.len() { unsafe { f(&mut *p.add(i)); } } }
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_call_blk(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_call_blk_struct>)).u };
    f(&mut u.recv);
    { if own { u.args = Box::leak(u.args.to_vec().into_boxed_slice()); } let l = u.args; let p = l.as_ptr() as *mut NodePtr; for i in 0..l.len() { unsafe { f(&mut *p.add(i)); } } }
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_call_kw(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_call_kw_struct>)).u };
    f(&mut u.recv);
    { if own { u.args = Box::leak(u.args.to_vec().into_boxed_slice()); } let l = u.args; let p = l.as_ptr() as *mut NodePtr; for i in 0..l.len() { unsafe { f(&mut *p.add(i)); } } }
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_call_x(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_call_x_struct>)).u };
    f(&mut u.recv);
    { if own { u.args = Box::leak(u.args.to_vec().into_boxed_slice()); } let l = u.args; let p = l.as_ptr() as *mut NodePtr; for i in 0..l.len() { unsafe { f(&mut *p.add(i)); } } }
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_send_sym(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_send_sym_struct>)).u };
    f(&mut u.recv);
    f(&mut u.call);
    f(&mut u.general);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_attrasgn(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_attrasgn_struct>)).u };
    f(&mut u.recv);
    { if own { u.args = Box::leak(u.args.to_vec().into_boxed_slice()); } let l = u.args; let p = l.as_ptr() as *mut NodePtr; for i in 0..l.len() { unsafe { f(&mut *p.add(i)); } } }
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_super(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_super_struct>)).u };
    { if own { u.args = Box::leak(u.args.to_vec().into_boxed_slice()); } let l = u.args; let p = l.as_ptr() as *mut NodePtr; for i in 0..l.len() { unsafe { f(&mut *p.add(i)); } } }
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_yield(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_yield_struct>)).u };
    { if own { u.args = Box::leak(u.args.to_vec().into_boxed_slice()); } let l = u.args; let p = l.as_ptr() as *mut NodePtr; for i in 0..l.len() { unsafe { f(&mut *p.add(i)); } } }
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_yield_x(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_yield_x_struct>)).u };
    { if own { u.args = Box::leak(u.args.to_vec().into_boxed_slice()); } let l = u.args; let p = l.as_ptr() as *mut NodePtr; for i in 0..l.len() { unsafe { f(&mut *p.add(i)); } } }
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_block_given(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_block_given_struct>)).u };

}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_lambda(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_lambda_struct>)).u };

}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_opt_plus(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_opt_plus_struct>)).u };
    f(&mut u.a);
    f(&mut u.b);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_opt_minus(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_opt_minus_struct>)).u };
    f(&mut u.a);
    f(&mut u.b);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_opt_mul(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_opt_mul_struct>)).u };
    f(&mut u.a);
    f(&mut u.b);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_opt_div(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_opt_div_struct>)).u };
    f(&mut u.a);
    f(&mut u.b);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_opt_pow(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_opt_pow_struct>)).u };
    f(&mut u.a);
    f(&mut u.b);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_opt_mod(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_opt_mod_struct>)).u };
    f(&mut u.a);
    f(&mut u.b);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_opt_lt(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_opt_lt_struct>)).u };
    f(&mut u.a);
    f(&mut u.b);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_opt_le(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_opt_le_struct>)).u };
    f(&mut u.a);
    f(&mut u.b);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_opt_gt(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_opt_gt_struct>)).u };
    f(&mut u.a);
    f(&mut u.b);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_opt_ge(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_opt_ge_struct>)).u };
    f(&mut u.a);
    f(&mut u.b);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_opt_eq(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_opt_eq_struct>)).u };
    f(&mut u.a);
    f(&mut u.b);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_opt_neq(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_opt_neq_struct>)).u };
    f(&mut u.a);
    f(&mut u.b);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_opt_ltlt(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_opt_ltlt_struct>)).u };
    f(&mut u.a);
    f(&mut u.b);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_opt_gtgt(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_opt_gtgt_struct>)).u };
    f(&mut u.a);
    f(&mut u.b);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_opt_xor(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_opt_xor_struct>)).u };
    f(&mut u.a);
    f(&mut u.b);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_opt_and(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_opt_and_struct>)).u };
    f(&mut u.a);
    f(&mut u.b);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_opt_or(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_opt_or_struct>)).u };
    f(&mut u.a);
    f(&mut u.b);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_opt_aref(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_opt_aref_struct>)).u };
    f(&mut u.a);
    f(&mut u.b);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_opt_aset(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_opt_aset_struct>)).u };
    f(&mut u.a);
    f(&mut u.b);
    f(&mut u.v);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_method_entry(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_method_entry_struct>)).u };
    f(&mut u.body);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_block_entry(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_block_entry_struct>)).u };
    f(&mut u.body);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_def(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_def_struct>)).u };

}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_sdef(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_sdef_struct>)).u };
    f(&mut u.recv);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_class(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_class_struct>)).u };
    f(&mut u.cpath);
    f(&mut u.superclass);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_module(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_module_struct>)).u };
    f(&mut u.cpath);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_sclass(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_sclass_struct>)).u };
    f(&mut u.recv);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_alias(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_alias_struct>)).u };
    f(&mut u.newname);
    f(&mut u.oldname);
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_valias(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_valias_struct>)).u };

}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_undef(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_undef_struct>)).u };
    { if own { u.names = Box::leak(u.names.to_vec().into_boxed_slice()); } let l = u.names; let p = l.as_ptr() as *mut NodePtr; for i in 0..l.len() { unsafe { f(&mut *p.add(i)); } } }
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_masgn(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_masgn_struct>)).u };
    f(&mut u.rhs);
    { if own { u.targets = Box::leak(u.targets.to_vec().into_boxed_slice()); } let l = u.targets; let p = l.as_ptr() as *mut NodePtr; for i in 0..l.len() { unsafe { f(&mut *p.add(i)); } } }
}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_masgn_elem(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_masgn_elem_struct>)).u };

}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_unsupported(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_unsupported_struct>)).u };

}

#[allow(non_snake_case, unused_variables)]
fn CHILDREN_node_special(n: NodePtr, own: bool, f: &mut dyn FnMut(&mut NodePtr)) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_special_struct>)).u };
    f(&mut u.arg);
}

#[allow(non_upper_case_globals)]
pub static KIND_node_nil: NodeKind = NodeKind {
    name: "node_nil",
    default_dispatcher: DISPATCH_node_nil,
    hash_func: HASH_node_nil,
    profile: PROF_node_nil,
    specializer: SPECIALIZE_node_nil,
    dumper: DUMP_node_nil,
    replacer: REPLACE_node_nil,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_nil_struct>>(),
    children: CHILDREN_node_nil,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_nil_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_nil_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_true: NodeKind = NodeKind {
    name: "node_true",
    default_dispatcher: DISPATCH_node_true,
    hash_func: HASH_node_true,
    profile: PROF_node_true,
    specializer: SPECIALIZE_node_true,
    dumper: DUMP_node_true,
    replacer: REPLACE_node_true,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_true_struct>>(),
    children: CHILDREN_node_true,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_true_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_true_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_false: NodeKind = NodeKind {
    name: "node_false",
    default_dispatcher: DISPATCH_node_false,
    hash_func: HASH_node_false,
    profile: PROF_node_false,
    specializer: SPECIALIZE_node_false,
    dumper: DUMP_node_false,
    replacer: REPLACE_node_false,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_false_struct>>(),
    children: CHILDREN_node_false,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_false_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_false_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_self: NodeKind = NodeKind {
    name: "node_self",
    default_dispatcher: DISPATCH_node_self,
    hash_func: HASH_node_self,
    profile: PROF_node_self,
    specializer: SPECIALIZE_node_self,
    dumper: DUMP_node_self,
    replacer: REPLACE_node_self,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_self_struct>>(),
    children: CHILDREN_node_self,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_self_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_self_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_lit: NodeKind = NodeKind {
    name: "node_lit",
    default_dispatcher: DISPATCH_node_lit,
    hash_func: HASH_node_lit,
    profile: PROF_node_lit,
    specializer: SPECIALIZE_node_lit,
    dumper: DUMP_node_lit,
    replacer: REPLACE_node_lit,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_lit_struct>>(),
    children: CHILDREN_node_lit,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_lit_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_lit_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_float: NodeKind = NodeKind {
    name: "node_float",
    default_dispatcher: DISPATCH_node_float,
    hash_func: HASH_node_float,
    profile: PROF_node_float,
    specializer: SPECIALIZE_node_float,
    dumper: DUMP_node_float,
    replacer: REPLACE_node_float,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_float_struct>>(),
    children: CHILDREN_node_float,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_float_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_float_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_str: NodeKind = NodeKind {
    name: "node_str",
    default_dispatcher: DISPATCH_node_str,
    hash_func: HASH_node_str,
    profile: PROF_node_str,
    specializer: SPECIALIZE_node_str,
    dumper: DUMP_node_str,
    replacer: REPLACE_node_str,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_str_struct>>(),
    children: CHILDREN_node_str,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_str_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_str_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_fstr: NodeKind = NodeKind {
    name: "node_fstr",
    default_dispatcher: DISPATCH_node_fstr,
    hash_func: HASH_node_fstr,
    profile: PROF_node_fstr,
    specializer: SPECIALIZE_node_fstr,
    dumper: DUMP_node_fstr,
    replacer: REPLACE_node_fstr,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_fstr_struct>>(),
    children: CHILDREN_node_fstr,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_fstr_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_fstr_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_dstr: NodeKind = NodeKind {
    name: "node_dstr",
    default_dispatcher: DISPATCH_node_dstr,
    hash_func: HASH_node_dstr,
    profile: PROF_node_dstr,
    specializer: SPECIALIZE_node_dstr,
    dumper: DUMP_node_dstr,
    replacer: REPLACE_node_dstr,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_dstr_struct>>(),
    children: CHILDREN_node_dstr,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_dstr_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_dstr_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_tostr: NodeKind = NodeKind {
    name: "node_tostr",
    default_dispatcher: DISPATCH_node_tostr,
    hash_func: HASH_node_tostr,
    profile: PROF_node_tostr,
    specializer: SPECIALIZE_node_tostr,
    dumper: DUMP_node_tostr,
    replacer: REPLACE_node_tostr,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_tostr_struct>>(),
    children: CHILDREN_node_tostr,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_tostr_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_tostr_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_dsym: NodeKind = NodeKind {
    name: "node_dsym",
    default_dispatcher: DISPATCH_node_dsym,
    hash_func: HASH_node_dsym,
    profile: PROF_node_dsym,
    specializer: SPECIALIZE_node_dsym,
    dumper: DUMP_node_dsym,
    replacer: REPLACE_node_dsym,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_dsym_struct>>(),
    children: CHILDREN_node_dsym,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_dsym_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_dsym_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_regex: NodeKind = NodeKind {
    name: "node_regex",
    default_dispatcher: DISPATCH_node_regex,
    hash_func: HASH_node_regex,
    profile: PROF_node_regex,
    specializer: SPECIALIZE_node_regex,
    dumper: DUMP_node_regex,
    replacer: REPLACE_node_regex,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_regex_struct>>(),
    children: CHILDREN_node_regex,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_regex_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_regex_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_dregex: NodeKind = NodeKind {
    name: "node_dregex",
    default_dispatcher: DISPATCH_node_dregex,
    hash_func: HASH_node_dregex,
    profile: PROF_node_dregex,
    specializer: SPECIALIZE_node_dregex,
    dumper: DUMP_node_dregex,
    replacer: REPLACE_node_dregex,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_dregex_struct>>(),
    children: CHILDREN_node_dregex,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_dregex_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_dregex_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_biglit: NodeKind = NodeKind {
    name: "node_biglit",
    default_dispatcher: DISPATCH_node_biglit,
    hash_func: HASH_node_biglit,
    profile: PROF_node_biglit,
    specializer: SPECIALIZE_node_biglit,
    dumper: DUMP_node_biglit,
    replacer: REPLACE_node_biglit,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_biglit_struct>>(),
    children: CHILDREN_node_biglit,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_biglit_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_biglit_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_ary: NodeKind = NodeKind {
    name: "node_ary",
    default_dispatcher: DISPATCH_node_ary,
    hash_func: HASH_node_ary,
    profile: PROF_node_ary,
    specializer: SPECIALIZE_node_ary,
    dumper: DUMP_node_ary,
    replacer: REPLACE_node_ary,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_ary_struct>>(),
    children: CHILDREN_node_ary,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_ary_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_ary_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_ary_build: NodeKind = NodeKind {
    name: "node_ary_build",
    default_dispatcher: DISPATCH_node_ary_build,
    hash_func: HASH_node_ary_build,
    profile: PROF_node_ary_build,
    specializer: SPECIALIZE_node_ary_build,
    dumper: DUMP_node_ary_build,
    replacer: REPLACE_node_ary_build,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_ary_build_struct>>(),
    children: CHILDREN_node_ary_build,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_ary_build_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_ary_build_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_hash: NodeKind = NodeKind {
    name: "node_hash",
    default_dispatcher: DISPATCH_node_hash,
    hash_func: HASH_node_hash,
    profile: PROF_node_hash,
    specializer: SPECIALIZE_node_hash,
    dumper: DUMP_node_hash,
    replacer: REPLACE_node_hash,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_hash_struct>>(),
    children: CHILDREN_node_hash,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_hash_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_hash_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_range: NodeKind = NodeKind {
    name: "node_range",
    default_dispatcher: DISPATCH_node_range,
    hash_func: HASH_node_range,
    profile: PROF_node_range,
    specializer: SPECIALIZE_node_range,
    dumper: DUMP_node_range,
    replacer: REPLACE_node_range,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_range_struct>>(),
    children: CHILDREN_node_range,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_range_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_range_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_range_lit: NodeKind = NodeKind {
    name: "node_range_lit",
    default_dispatcher: DISPATCH_node_range_lit,
    hash_func: HASH_node_range_lit,
    profile: PROF_node_range_lit,
    specializer: SPECIALIZE_node_range_lit,
    dumper: DUMP_node_range_lit,
    replacer: REPLACE_node_range_lit,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_range_lit_struct>>(),
    children: CHILDREN_node_range_lit,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_range_lit_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_range_lit_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_lvar: NodeKind = NodeKind {
    name: "node_lvar",
    default_dispatcher: DISPATCH_node_lvar,
    hash_func: HASH_node_lvar,
    profile: PROF_node_lvar,
    specializer: SPECIALIZE_node_lvar,
    dumper: DUMP_node_lvar,
    replacer: REPLACE_node_lvar,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_lvar_struct>>(),
    children: CHILDREN_node_lvar,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_lvar_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_lvar_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_lasgn: NodeKind = NodeKind {
    name: "node_lasgn",
    default_dispatcher: DISPATCH_node_lasgn,
    hash_func: HASH_node_lasgn,
    profile: PROF_node_lasgn,
    specializer: SPECIALIZE_node_lasgn,
    dumper: DUMP_node_lasgn,
    replacer: REPLACE_node_lasgn,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_lasgn_struct>>(),
    children: CHILDREN_node_lasgn,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_lasgn_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_lasgn_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_dvar: NodeKind = NodeKind {
    name: "node_dvar",
    default_dispatcher: DISPATCH_node_dvar,
    hash_func: HASH_node_dvar,
    profile: PROF_node_dvar,
    specializer: SPECIALIZE_node_dvar,
    dumper: DUMP_node_dvar,
    replacer: REPLACE_node_dvar,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_dvar_struct>>(),
    children: CHILDREN_node_dvar,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_dvar_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_dvar_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_dasgn: NodeKind = NodeKind {
    name: "node_dasgn",
    default_dispatcher: DISPATCH_node_dasgn,
    hash_func: HASH_node_dasgn,
    profile: PROF_node_dasgn,
    specializer: SPECIALIZE_node_dasgn,
    dumper: DUMP_node_dasgn,
    replacer: REPLACE_node_dasgn,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_dasgn_struct>>(),
    children: CHILDREN_node_dasgn,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_dasgn_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_dasgn_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_ivar: NodeKind = NodeKind {
    name: "node_ivar",
    default_dispatcher: DISPATCH_node_ivar,
    hash_func: HASH_node_ivar,
    profile: PROF_node_ivar,
    specializer: SPECIALIZE_node_ivar,
    dumper: DUMP_node_ivar,
    replacer: REPLACE_node_ivar,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_ivar_struct>>(),
    children: CHILDREN_node_ivar,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_ivar_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_ivar_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_iasgn: NodeKind = NodeKind {
    name: "node_iasgn",
    default_dispatcher: DISPATCH_node_iasgn,
    hash_func: HASH_node_iasgn,
    profile: PROF_node_iasgn,
    specializer: SPECIALIZE_node_iasgn,
    dumper: DUMP_node_iasgn,
    replacer: REPLACE_node_iasgn,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_iasgn_struct>>(),
    children: CHILDREN_node_iasgn,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_iasgn_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_iasgn_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_ivar_arith: NodeKind = NodeKind {
    name: "node_ivar_arith",
    default_dispatcher: DISPATCH_node_ivar_arith,
    hash_func: HASH_node_ivar_arith,
    profile: PROF_node_ivar_arith,
    specializer: SPECIALIZE_node_ivar_arith,
    dumper: DUMP_node_ivar_arith,
    replacer: REPLACE_node_ivar_arith,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_ivar_arith_struct>>(),
    children: CHILDREN_node_ivar_arith,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_ivar_arith_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_ivar_arith_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_gvar: NodeKind = NodeKind {
    name: "node_gvar",
    default_dispatcher: DISPATCH_node_gvar,
    hash_func: HASH_node_gvar,
    profile: PROF_node_gvar,
    specializer: SPECIALIZE_node_gvar,
    dumper: DUMP_node_gvar,
    replacer: REPLACE_node_gvar,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_gvar_struct>>(),
    children: CHILDREN_node_gvar,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_gvar_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_gvar_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_gasgn: NodeKind = NodeKind {
    name: "node_gasgn",
    default_dispatcher: DISPATCH_node_gasgn,
    hash_func: HASH_node_gasgn,
    profile: PROF_node_gasgn,
    specializer: SPECIALIZE_node_gasgn,
    dumper: DUMP_node_gasgn,
    replacer: REPLACE_node_gasgn,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_gasgn_struct>>(),
    children: CHILDREN_node_gasgn,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_gasgn_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_gasgn_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_cvar: NodeKind = NodeKind {
    name: "node_cvar",
    default_dispatcher: DISPATCH_node_cvar,
    hash_func: HASH_node_cvar,
    profile: PROF_node_cvar,
    specializer: SPECIALIZE_node_cvar,
    dumper: DUMP_node_cvar,
    replacer: REPLACE_node_cvar,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_cvar_struct>>(),
    children: CHILDREN_node_cvar,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_cvar_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_cvar_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_cvasgn: NodeKind = NodeKind {
    name: "node_cvasgn",
    default_dispatcher: DISPATCH_node_cvasgn,
    hash_func: HASH_node_cvasgn,
    profile: PROF_node_cvasgn,
    specializer: SPECIALIZE_node_cvasgn,
    dumper: DUMP_node_cvasgn,
    replacer: REPLACE_node_cvasgn,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_cvasgn_struct>>(),
    children: CHILDREN_node_cvasgn,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_cvasgn_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_cvasgn_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_const: NodeKind = NodeKind {
    name: "node_const",
    default_dispatcher: DISPATCH_node_const,
    hash_func: HASH_node_const,
    profile: PROF_node_const,
    specializer: SPECIALIZE_node_const,
    dumper: DUMP_node_const,
    replacer: REPLACE_node_const,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_const_struct>>(),
    children: CHILDREN_node_const,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_const_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_const_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_const_path: NodeKind = NodeKind {
    name: "node_const_path",
    default_dispatcher: DISPATCH_node_const_path,
    hash_func: HASH_node_const_path,
    profile: PROF_node_const_path,
    specializer: SPECIALIZE_node_const_path,
    dumper: DUMP_node_const_path,
    replacer: REPLACE_node_const_path,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_const_path_struct>>(),
    children: CHILDREN_node_const_path,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_const_path_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_const_path_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_top_const: NodeKind = NodeKind {
    name: "node_top_const",
    default_dispatcher: DISPATCH_node_top_const,
    hash_func: HASH_node_top_const,
    profile: PROF_node_top_const,
    specializer: SPECIALIZE_node_top_const,
    dumper: DUMP_node_top_const,
    replacer: REPLACE_node_top_const,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_top_const_struct>>(),
    children: CHILDREN_node_top_const,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_top_const_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_top_const_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_cdecl: NodeKind = NodeKind {
    name: "node_cdecl",
    default_dispatcher: DISPATCH_node_cdecl,
    hash_func: HASH_node_cdecl,
    profile: PROF_node_cdecl,
    specializer: SPECIALIZE_node_cdecl,
    dumper: DUMP_node_cdecl,
    replacer: REPLACE_node_cdecl,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_cdecl_struct>>(),
    children: CHILDREN_node_cdecl,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_cdecl_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_cdecl_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_cdecl_path: NodeKind = NodeKind {
    name: "node_cdecl_path",
    default_dispatcher: DISPATCH_node_cdecl_path,
    hash_func: HASH_node_cdecl_path,
    profile: PROF_node_cdecl_path,
    specializer: SPECIALIZE_node_cdecl_path,
    dumper: DUMP_node_cdecl_path,
    replacer: REPLACE_node_cdecl_path,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_cdecl_path_struct>>(),
    children: CHILDREN_node_cdecl_path,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_cdecl_path_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_cdecl_path_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_errinfo: NodeKind = NodeKind {
    name: "node_errinfo",
    default_dispatcher: DISPATCH_node_errinfo,
    hash_func: HASH_node_errinfo,
    profile: PROF_node_errinfo,
    specializer: SPECIALIZE_node_errinfo,
    dumper: DUMP_node_errinfo,
    replacer: REPLACE_node_errinfo,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_errinfo_struct>>(),
    children: CHILDREN_node_errinfo,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_errinfo_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_errinfo_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_nth_ref: NodeKind = NodeKind {
    name: "node_nth_ref",
    default_dispatcher: DISPATCH_node_nth_ref,
    hash_func: HASH_node_nth_ref,
    profile: PROF_node_nth_ref,
    specializer: SPECIALIZE_node_nth_ref,
    dumper: DUMP_node_nth_ref,
    replacer: REPLACE_node_nth_ref,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_nth_ref_struct>>(),
    children: CHILDREN_node_nth_ref,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_nth_ref_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_nth_ref_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_back_ref: NodeKind = NodeKind {
    name: "node_back_ref",
    default_dispatcher: DISPATCH_node_back_ref,
    hash_func: HASH_node_back_ref,
    profile: PROF_node_back_ref,
    specializer: SPECIALIZE_node_back_ref,
    dumper: DUMP_node_back_ref,
    replacer: REPLACE_node_back_ref,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_back_ref_struct>>(),
    children: CHILDREN_node_back_ref,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_back_ref_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_back_ref_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_block: NodeKind = NodeKind {
    name: "node_block",
    default_dispatcher: DISPATCH_node_block,
    hash_func: HASH_node_block,
    profile: PROF_node_block,
    specializer: SPECIALIZE_node_block,
    dumper: DUMP_node_block,
    replacer: REPLACE_node_block,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_block_struct>>(),
    children: CHILDREN_node_block,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_block_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_block_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_line: NodeKind = NodeKind {
    name: "node_line",
    default_dispatcher: DISPATCH_node_line,
    hash_func: HASH_node_line,
    profile: PROF_node_line,
    specializer: SPECIALIZE_node_line,
    dumper: DUMP_node_line,
    replacer: REPLACE_node_line,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_line_struct>>(),
    children: CHILDREN_node_line,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_line_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_line_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_trace: NodeKind = NodeKind {
    name: "node_trace",
    default_dispatcher: DISPATCH_node_trace,
    hash_func: HASH_node_trace,
    profile: PROF_node_trace,
    specializer: SPECIALIZE_node_trace,
    dumper: DUMP_node_trace,
    replacer: REPLACE_node_trace,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_trace_struct>>(),
    children: CHILDREN_node_trace,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_trace_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_trace_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_if: NodeKind = NodeKind {
    name: "node_if",
    default_dispatcher: DISPATCH_node_if,
    hash_func: HASH_node_if,
    profile: PROF_node_if,
    specializer: SPECIALIZE_node_if,
    dumper: DUMP_node_if,
    replacer: REPLACE_node_if,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_if_struct>>(),
    children: CHILDREN_node_if,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_if_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_if_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_and: NodeKind = NodeKind {
    name: "node_and",
    default_dispatcher: DISPATCH_node_and,
    hash_func: HASH_node_and,
    profile: PROF_node_and,
    specializer: SPECIALIZE_node_and,
    dumper: DUMP_node_and,
    replacer: REPLACE_node_and,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_and_struct>>(),
    children: CHILDREN_node_and,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_and_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_and_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_or: NodeKind = NodeKind {
    name: "node_or",
    default_dispatcher: DISPATCH_node_or,
    hash_func: HASH_node_or,
    profile: PROF_node_or,
    specializer: SPECIALIZE_node_or,
    dumper: DUMP_node_or,
    replacer: REPLACE_node_or,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_or_struct>>(),
    children: CHILDREN_node_or,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_or_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_or_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_not: NodeKind = NodeKind {
    name: "node_not",
    default_dispatcher: DISPATCH_node_not,
    hash_func: HASH_node_not,
    profile: PROF_node_not,
    specializer: SPECIALIZE_node_not,
    dumper: DUMP_node_not,
    replacer: REPLACE_node_not,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_not_struct>>(),
    children: CHILDREN_node_not,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_not_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_not_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_while: NodeKind = NodeKind {
    name: "node_while",
    default_dispatcher: DISPATCH_node_while,
    hash_func: HASH_node_while,
    profile: PROF_node_while,
    specializer: SPECIALIZE_node_while,
    dumper: DUMP_node_while,
    replacer: REPLACE_node_while,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_while_struct>>(),
    children: CHILDREN_node_while,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_while_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_while_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_dowhile: NodeKind = NodeKind {
    name: "node_dowhile",
    default_dispatcher: DISPATCH_node_dowhile,
    hash_func: HASH_node_dowhile,
    profile: PROF_node_dowhile,
    specializer: SPECIALIZE_node_dowhile,
    dumper: DUMP_node_dowhile,
    replacer: REPLACE_node_dowhile,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_dowhile_struct>>(),
    children: CHILDREN_node_dowhile,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_dowhile_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_dowhile_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_loop_break: NodeKind = NodeKind {
    name: "node_loop_break",
    default_dispatcher: DISPATCH_node_loop_break,
    hash_func: HASH_node_loop_break,
    profile: PROF_node_loop_break,
    specializer: SPECIALIZE_node_loop_break,
    dumper: DUMP_node_loop_break,
    replacer: REPLACE_node_loop_break,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_loop_break_struct>>(),
    children: CHILDREN_node_loop_break,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_loop_break_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_loop_break_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_loop_next: NodeKind = NodeKind {
    name: "node_loop_next",
    default_dispatcher: DISPATCH_node_loop_next,
    hash_func: HASH_node_loop_next,
    profile: PROF_node_loop_next,
    specializer: SPECIALIZE_node_loop_next,
    dumper: DUMP_node_loop_next,
    replacer: REPLACE_node_loop_next,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_loop_next_struct>>(),
    children: CHILDREN_node_loop_next,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_loop_next_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_loop_next_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_break: NodeKind = NodeKind {
    name: "node_break",
    default_dispatcher: DISPATCH_node_break,
    hash_func: HASH_node_break,
    profile: PROF_node_break,
    specializer: SPECIALIZE_node_break,
    dumper: DUMP_node_break,
    replacer: REPLACE_node_break,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_break_struct>>(),
    children: CHILDREN_node_break,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_break_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_break_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_next: NodeKind = NodeKind {
    name: "node_next",
    default_dispatcher: DISPATCH_node_next,
    hash_func: HASH_node_next,
    profile: PROF_node_next,
    specializer: SPECIALIZE_node_next,
    dumper: DUMP_node_next,
    replacer: REPLACE_node_next,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_next_struct>>(),
    children: CHILDREN_node_next,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_next_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_next_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_return: NodeKind = NodeKind {
    name: "node_return",
    default_dispatcher: DISPATCH_node_return,
    hash_func: HASH_node_return,
    profile: PROF_node_return,
    specializer: SPECIALIZE_node_return,
    dumper: DUMP_node_return,
    replacer: REPLACE_node_return,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_return_struct>>(),
    children: CHILDREN_node_return,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_return_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_return_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_retry: NodeKind = NodeKind {
    name: "node_retry",
    default_dispatcher: DISPATCH_node_retry,
    hash_func: HASH_node_retry,
    profile: PROF_node_retry,
    specializer: SPECIALIZE_node_retry,
    dumper: DUMP_node_retry,
    replacer: REPLACE_node_retry,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_retry_struct>>(),
    children: CHILDREN_node_retry,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_retry_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_retry_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_redo: NodeKind = NodeKind {
    name: "node_redo",
    default_dispatcher: DISPATCH_node_redo,
    hash_func: HASH_node_redo,
    profile: PROF_node_redo,
    specializer: SPECIALIZE_node_redo,
    dumper: DUMP_node_redo,
    replacer: REPLACE_node_redo,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_redo_struct>>(),
    children: CHILDREN_node_redo,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_redo_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_redo_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_rescue: NodeKind = NodeKind {
    name: "node_rescue",
    default_dispatcher: DISPATCH_node_rescue,
    hash_func: HASH_node_rescue,
    profile: PROF_node_rescue,
    specializer: SPECIALIZE_node_rescue,
    dumper: DUMP_node_rescue,
    replacer: REPLACE_node_rescue,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_rescue_struct>>(),
    children: CHILDREN_node_rescue,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_rescue_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_rescue_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_resbody: NodeKind = NodeKind {
    name: "node_resbody",
    default_dispatcher: DISPATCH_node_resbody,
    hash_func: HASH_node_resbody,
    profile: PROF_node_resbody,
    specializer: SPECIALIZE_node_resbody,
    dumper: DUMP_node_resbody,
    replacer: REPLACE_node_resbody,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_resbody_struct>>(),
    children: CHILDREN_node_resbody,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_resbody_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_resbody_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_ensure: NodeKind = NodeKind {
    name: "node_ensure",
    default_dispatcher: DISPATCH_node_ensure,
    hash_func: HASH_node_ensure,
    profile: PROF_node_ensure,
    specializer: SPECIALIZE_node_ensure,
    dumper: DUMP_node_ensure,
    replacer: REPLACE_node_ensure,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_ensure_struct>>(),
    children: CHILDREN_node_ensure,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_ensure_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_ensure_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_when: NodeKind = NodeKind {
    name: "node_when",
    default_dispatcher: DISPATCH_node_when,
    hash_func: HASH_node_when,
    profile: PROF_node_when,
    specializer: SPECIALIZE_node_when,
    dumper: DUMP_node_when,
    replacer: REPLACE_node_when,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_when_struct>>(),
    children: CHILDREN_node_when,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_when_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_when_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_when_lit: NodeKind = NodeKind {
    name: "node_when_lit",
    default_dispatcher: DISPATCH_node_when_lit,
    hash_func: HASH_node_when_lit,
    profile: PROF_node_when_lit,
    specializer: SPECIALIZE_node_when_lit,
    dumper: DUMP_node_when_lit,
    replacer: REPLACE_node_when_lit,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_when_lit_struct>>(),
    children: CHILDREN_node_when_lit,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_when_lit_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_when_lit_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_defined: NodeKind = NodeKind {
    name: "node_defined",
    default_dispatcher: DISPATCH_node_defined,
    hash_func: HASH_node_defined,
    profile: PROF_node_defined,
    specializer: SPECIALIZE_node_defined,
    dumper: DUMP_node_defined,
    replacer: REPLACE_node_defined,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_defined_struct>>(),
    children: CHILDREN_node_defined,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_defined_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_defined_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_call: NodeKind = NodeKind {
    name: "node_call",
    default_dispatcher: DISPATCH_node_call,
    hash_func: HASH_node_call,
    profile: PROF_node_call,
    specializer: SPECIALIZE_node_call,
    dumper: DUMP_node_call,
    replacer: REPLACE_node_call,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_call_struct>>(),
    children: CHILDREN_node_call,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_call_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_call_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_call_blk: NodeKind = NodeKind {
    name: "node_call_blk",
    default_dispatcher: DISPATCH_node_call_blk,
    hash_func: HASH_node_call_blk,
    profile: PROF_node_call_blk,
    specializer: SPECIALIZE_node_call_blk,
    dumper: DUMP_node_call_blk,
    replacer: REPLACE_node_call_blk,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_call_blk_struct>>(),
    children: CHILDREN_node_call_blk,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_call_blk_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_call_blk_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_call_kw: NodeKind = NodeKind {
    name: "node_call_kw",
    default_dispatcher: DISPATCH_node_call_kw,
    hash_func: HASH_node_call_kw,
    profile: PROF_node_call_kw,
    specializer: SPECIALIZE_node_call_kw,
    dumper: DUMP_node_call_kw,
    replacer: REPLACE_node_call_kw,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_call_kw_struct>>(),
    children: CHILDREN_node_call_kw,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_call_kw_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_call_kw_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_call_x: NodeKind = NodeKind {
    name: "node_call_x",
    default_dispatcher: DISPATCH_node_call_x,
    hash_func: HASH_node_call_x,
    profile: PROF_node_call_x,
    specializer: SPECIALIZE_node_call_x,
    dumper: DUMP_node_call_x,
    replacer: REPLACE_node_call_x,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_call_x_struct>>(),
    children: CHILDREN_node_call_x,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_call_x_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_call_x_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_send_sym: NodeKind = NodeKind {
    name: "node_send_sym",
    default_dispatcher: DISPATCH_node_send_sym,
    hash_func: HASH_node_send_sym,
    profile: PROF_node_send_sym,
    specializer: SPECIALIZE_node_send_sym,
    dumper: DUMP_node_send_sym,
    replacer: REPLACE_node_send_sym,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_send_sym_struct>>(),
    children: CHILDREN_node_send_sym,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_send_sym_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_send_sym_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_attrasgn: NodeKind = NodeKind {
    name: "node_attrasgn",
    default_dispatcher: DISPATCH_node_attrasgn,
    hash_func: HASH_node_attrasgn,
    profile: PROF_node_attrasgn,
    specializer: SPECIALIZE_node_attrasgn,
    dumper: DUMP_node_attrasgn,
    replacer: REPLACE_node_attrasgn,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_attrasgn_struct>>(),
    children: CHILDREN_node_attrasgn,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_attrasgn_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_attrasgn_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_super: NodeKind = NodeKind {
    name: "node_super",
    default_dispatcher: DISPATCH_node_super,
    hash_func: HASH_node_super,
    profile: PROF_node_super,
    specializer: SPECIALIZE_node_super,
    dumper: DUMP_node_super,
    replacer: REPLACE_node_super,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_super_struct>>(),
    children: CHILDREN_node_super,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_super_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_super_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_yield: NodeKind = NodeKind {
    name: "node_yield",
    default_dispatcher: DISPATCH_node_yield,
    hash_func: HASH_node_yield,
    profile: PROF_node_yield,
    specializer: SPECIALIZE_node_yield,
    dumper: DUMP_node_yield,
    replacer: REPLACE_node_yield,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_yield_struct>>(),
    children: CHILDREN_node_yield,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_yield_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_yield_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_yield_x: NodeKind = NodeKind {
    name: "node_yield_x",
    default_dispatcher: DISPATCH_node_yield_x,
    hash_func: HASH_node_yield_x,
    profile: PROF_node_yield_x,
    specializer: SPECIALIZE_node_yield_x,
    dumper: DUMP_node_yield_x,
    replacer: REPLACE_node_yield_x,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_yield_x_struct>>(),
    children: CHILDREN_node_yield_x,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_yield_x_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_yield_x_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_block_given: NodeKind = NodeKind {
    name: "node_block_given",
    default_dispatcher: DISPATCH_node_block_given,
    hash_func: HASH_node_block_given,
    profile: PROF_node_block_given,
    specializer: SPECIALIZE_node_block_given,
    dumper: DUMP_node_block_given,
    replacer: REPLACE_node_block_given,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_block_given_struct>>(),
    children: CHILDREN_node_block_given,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_block_given_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_block_given_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_lambda: NodeKind = NodeKind {
    name: "node_lambda",
    default_dispatcher: DISPATCH_node_lambda,
    hash_func: HASH_node_lambda,
    profile: PROF_node_lambda,
    specializer: SPECIALIZE_node_lambda,
    dumper: DUMP_node_lambda,
    replacer: REPLACE_node_lambda,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_lambda_struct>>(),
    children: CHILDREN_node_lambda,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_lambda_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_lambda_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_opt_plus: NodeKind = NodeKind {
    name: "node_opt_plus",
    default_dispatcher: DISPATCH_node_opt_plus,
    hash_func: HASH_node_opt_plus,
    profile: PROF_node_opt_plus,
    specializer: SPECIALIZE_node_opt_plus,
    dumper: DUMP_node_opt_plus,
    replacer: REPLACE_node_opt_plus,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_opt_plus_struct>>(),
    children: CHILDREN_node_opt_plus,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_opt_plus_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_opt_plus_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_opt_minus: NodeKind = NodeKind {
    name: "node_opt_minus",
    default_dispatcher: DISPATCH_node_opt_minus,
    hash_func: HASH_node_opt_minus,
    profile: PROF_node_opt_minus,
    specializer: SPECIALIZE_node_opt_minus,
    dumper: DUMP_node_opt_minus,
    replacer: REPLACE_node_opt_minus,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_opt_minus_struct>>(),
    children: CHILDREN_node_opt_minus,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_opt_minus_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_opt_minus_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_opt_mul: NodeKind = NodeKind {
    name: "node_opt_mul",
    default_dispatcher: DISPATCH_node_opt_mul,
    hash_func: HASH_node_opt_mul,
    profile: PROF_node_opt_mul,
    specializer: SPECIALIZE_node_opt_mul,
    dumper: DUMP_node_opt_mul,
    replacer: REPLACE_node_opt_mul,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_opt_mul_struct>>(),
    children: CHILDREN_node_opt_mul,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_opt_mul_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_opt_mul_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_opt_div: NodeKind = NodeKind {
    name: "node_opt_div",
    default_dispatcher: DISPATCH_node_opt_div,
    hash_func: HASH_node_opt_div,
    profile: PROF_node_opt_div,
    specializer: SPECIALIZE_node_opt_div,
    dumper: DUMP_node_opt_div,
    replacer: REPLACE_node_opt_div,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_opt_div_struct>>(),
    children: CHILDREN_node_opt_div,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_opt_div_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_opt_div_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_opt_pow: NodeKind = NodeKind {
    name: "node_opt_pow",
    default_dispatcher: DISPATCH_node_opt_pow,
    hash_func: HASH_node_opt_pow,
    profile: PROF_node_opt_pow,
    specializer: SPECIALIZE_node_opt_pow,
    dumper: DUMP_node_opt_pow,
    replacer: REPLACE_node_opt_pow,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_opt_pow_struct>>(),
    children: CHILDREN_node_opt_pow,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_opt_pow_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_opt_pow_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_opt_mod: NodeKind = NodeKind {
    name: "node_opt_mod",
    default_dispatcher: DISPATCH_node_opt_mod,
    hash_func: HASH_node_opt_mod,
    profile: PROF_node_opt_mod,
    specializer: SPECIALIZE_node_opt_mod,
    dumper: DUMP_node_opt_mod,
    replacer: REPLACE_node_opt_mod,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_opt_mod_struct>>(),
    children: CHILDREN_node_opt_mod,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_opt_mod_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_opt_mod_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_opt_lt: NodeKind = NodeKind {
    name: "node_opt_lt",
    default_dispatcher: DISPATCH_node_opt_lt,
    hash_func: HASH_node_opt_lt,
    profile: PROF_node_opt_lt,
    specializer: SPECIALIZE_node_opt_lt,
    dumper: DUMP_node_opt_lt,
    replacer: REPLACE_node_opt_lt,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_opt_lt_struct>>(),
    children: CHILDREN_node_opt_lt,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_opt_lt_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_opt_lt_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_opt_le: NodeKind = NodeKind {
    name: "node_opt_le",
    default_dispatcher: DISPATCH_node_opt_le,
    hash_func: HASH_node_opt_le,
    profile: PROF_node_opt_le,
    specializer: SPECIALIZE_node_opt_le,
    dumper: DUMP_node_opt_le,
    replacer: REPLACE_node_opt_le,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_opt_le_struct>>(),
    children: CHILDREN_node_opt_le,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_opt_le_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_opt_le_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_opt_gt: NodeKind = NodeKind {
    name: "node_opt_gt",
    default_dispatcher: DISPATCH_node_opt_gt,
    hash_func: HASH_node_opt_gt,
    profile: PROF_node_opt_gt,
    specializer: SPECIALIZE_node_opt_gt,
    dumper: DUMP_node_opt_gt,
    replacer: REPLACE_node_opt_gt,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_opt_gt_struct>>(),
    children: CHILDREN_node_opt_gt,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_opt_gt_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_opt_gt_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_opt_ge: NodeKind = NodeKind {
    name: "node_opt_ge",
    default_dispatcher: DISPATCH_node_opt_ge,
    hash_func: HASH_node_opt_ge,
    profile: PROF_node_opt_ge,
    specializer: SPECIALIZE_node_opt_ge,
    dumper: DUMP_node_opt_ge,
    replacer: REPLACE_node_opt_ge,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_opt_ge_struct>>(),
    children: CHILDREN_node_opt_ge,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_opt_ge_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_opt_ge_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_opt_eq: NodeKind = NodeKind {
    name: "node_opt_eq",
    default_dispatcher: DISPATCH_node_opt_eq,
    hash_func: HASH_node_opt_eq,
    profile: PROF_node_opt_eq,
    specializer: SPECIALIZE_node_opt_eq,
    dumper: DUMP_node_opt_eq,
    replacer: REPLACE_node_opt_eq,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_opt_eq_struct>>(),
    children: CHILDREN_node_opt_eq,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_opt_eq_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_opt_eq_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_opt_neq: NodeKind = NodeKind {
    name: "node_opt_neq",
    default_dispatcher: DISPATCH_node_opt_neq,
    hash_func: HASH_node_opt_neq,
    profile: PROF_node_opt_neq,
    specializer: SPECIALIZE_node_opt_neq,
    dumper: DUMP_node_opt_neq,
    replacer: REPLACE_node_opt_neq,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_opt_neq_struct>>(),
    children: CHILDREN_node_opt_neq,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_opt_neq_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_opt_neq_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_opt_ltlt: NodeKind = NodeKind {
    name: "node_opt_ltlt",
    default_dispatcher: DISPATCH_node_opt_ltlt,
    hash_func: HASH_node_opt_ltlt,
    profile: PROF_node_opt_ltlt,
    specializer: SPECIALIZE_node_opt_ltlt,
    dumper: DUMP_node_opt_ltlt,
    replacer: REPLACE_node_opt_ltlt,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_opt_ltlt_struct>>(),
    children: CHILDREN_node_opt_ltlt,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_opt_ltlt_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_opt_ltlt_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_opt_gtgt: NodeKind = NodeKind {
    name: "node_opt_gtgt",
    default_dispatcher: DISPATCH_node_opt_gtgt,
    hash_func: HASH_node_opt_gtgt,
    profile: PROF_node_opt_gtgt,
    specializer: SPECIALIZE_node_opt_gtgt,
    dumper: DUMP_node_opt_gtgt,
    replacer: REPLACE_node_opt_gtgt,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_opt_gtgt_struct>>(),
    children: CHILDREN_node_opt_gtgt,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_opt_gtgt_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_opt_gtgt_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_opt_xor: NodeKind = NodeKind {
    name: "node_opt_xor",
    default_dispatcher: DISPATCH_node_opt_xor,
    hash_func: HASH_node_opt_xor,
    profile: PROF_node_opt_xor,
    specializer: SPECIALIZE_node_opt_xor,
    dumper: DUMP_node_opt_xor,
    replacer: REPLACE_node_opt_xor,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_opt_xor_struct>>(),
    children: CHILDREN_node_opt_xor,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_opt_xor_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_opt_xor_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_opt_and: NodeKind = NodeKind {
    name: "node_opt_and",
    default_dispatcher: DISPATCH_node_opt_and,
    hash_func: HASH_node_opt_and,
    profile: PROF_node_opt_and,
    specializer: SPECIALIZE_node_opt_and,
    dumper: DUMP_node_opt_and,
    replacer: REPLACE_node_opt_and,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_opt_and_struct>>(),
    children: CHILDREN_node_opt_and,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_opt_and_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_opt_and_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_opt_or: NodeKind = NodeKind {
    name: "node_opt_or",
    default_dispatcher: DISPATCH_node_opt_or,
    hash_func: HASH_node_opt_or,
    profile: PROF_node_opt_or,
    specializer: SPECIALIZE_node_opt_or,
    dumper: DUMP_node_opt_or,
    replacer: REPLACE_node_opt_or,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_opt_or_struct>>(),
    children: CHILDREN_node_opt_or,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_opt_or_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_opt_or_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_opt_aref: NodeKind = NodeKind {
    name: "node_opt_aref",
    default_dispatcher: DISPATCH_node_opt_aref,
    hash_func: HASH_node_opt_aref,
    profile: PROF_node_opt_aref,
    specializer: SPECIALIZE_node_opt_aref,
    dumper: DUMP_node_opt_aref,
    replacer: REPLACE_node_opt_aref,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_opt_aref_struct>>(),
    children: CHILDREN_node_opt_aref,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_opt_aref_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_opt_aref_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_opt_aset: NodeKind = NodeKind {
    name: "node_opt_aset",
    default_dispatcher: DISPATCH_node_opt_aset,
    hash_func: HASH_node_opt_aset,
    profile: PROF_node_opt_aset,
    specializer: SPECIALIZE_node_opt_aset,
    dumper: DUMP_node_opt_aset,
    replacer: REPLACE_node_opt_aset,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_opt_aset_struct>>(),
    children: CHILDREN_node_opt_aset,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_opt_aset_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_opt_aset_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_method_entry: NodeKind = NodeKind {
    name: "node_method_entry",
    default_dispatcher: DISPATCH_node_method_entry,
    hash_func: HASH_node_method_entry,
    profile: PROF_node_method_entry,
    specializer: SPECIALIZE_node_method_entry,
    dumper: DUMP_node_method_entry,
    replacer: REPLACE_node_method_entry,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_method_entry_struct>>(),
    children: CHILDREN_node_method_entry,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_method_entry_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_method_entry_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_block_entry: NodeKind = NodeKind {
    name: "node_block_entry",
    default_dispatcher: DISPATCH_node_block_entry,
    hash_func: HASH_node_block_entry,
    profile: PROF_node_block_entry,
    specializer: SPECIALIZE_node_block_entry,
    dumper: DUMP_node_block_entry,
    replacer: REPLACE_node_block_entry,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_block_entry_struct>>(),
    children: CHILDREN_node_block_entry,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_block_entry_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_block_entry_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_def: NodeKind = NodeKind {
    name: "node_def",
    default_dispatcher: DISPATCH_node_def,
    hash_func: HASH_node_def,
    profile: PROF_node_def,
    specializer: SPECIALIZE_node_def,
    dumper: DUMP_node_def,
    replacer: REPLACE_node_def,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_def_struct>>(),
    children: CHILDREN_node_def,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_def_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_def_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_sdef: NodeKind = NodeKind {
    name: "node_sdef",
    default_dispatcher: DISPATCH_node_sdef,
    hash_func: HASH_node_sdef,
    profile: PROF_node_sdef,
    specializer: SPECIALIZE_node_sdef,
    dumper: DUMP_node_sdef,
    replacer: REPLACE_node_sdef,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_sdef_struct>>(),
    children: CHILDREN_node_sdef,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_sdef_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_sdef_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_class: NodeKind = NodeKind {
    name: "node_class",
    default_dispatcher: DISPATCH_node_class,
    hash_func: HASH_node_class,
    profile: PROF_node_class,
    specializer: SPECIALIZE_node_class,
    dumper: DUMP_node_class,
    replacer: REPLACE_node_class,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_class_struct>>(),
    children: CHILDREN_node_class,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_class_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_class_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_module: NodeKind = NodeKind {
    name: "node_module",
    default_dispatcher: DISPATCH_node_module,
    hash_func: HASH_node_module,
    profile: PROF_node_module,
    specializer: SPECIALIZE_node_module,
    dumper: DUMP_node_module,
    replacer: REPLACE_node_module,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_module_struct>>(),
    children: CHILDREN_node_module,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_module_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_module_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_sclass: NodeKind = NodeKind {
    name: "node_sclass",
    default_dispatcher: DISPATCH_node_sclass,
    hash_func: HASH_node_sclass,
    profile: PROF_node_sclass,
    specializer: SPECIALIZE_node_sclass,
    dumper: DUMP_node_sclass,
    replacer: REPLACE_node_sclass,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_sclass_struct>>(),
    children: CHILDREN_node_sclass,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_sclass_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_sclass_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_alias: NodeKind = NodeKind {
    name: "node_alias",
    default_dispatcher: DISPATCH_node_alias,
    hash_func: HASH_node_alias,
    profile: PROF_node_alias,
    specializer: SPECIALIZE_node_alias,
    dumper: DUMP_node_alias,
    replacer: REPLACE_node_alias,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_alias_struct>>(),
    children: CHILDREN_node_alias,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_alias_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_alias_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_valias: NodeKind = NodeKind {
    name: "node_valias",
    default_dispatcher: DISPATCH_node_valias,
    hash_func: HASH_node_valias,
    profile: PROF_node_valias,
    specializer: SPECIALIZE_node_valias,
    dumper: DUMP_node_valias,
    replacer: REPLACE_node_valias,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_valias_struct>>(),
    children: CHILDREN_node_valias,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_valias_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_valias_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_undef: NodeKind = NodeKind {
    name: "node_undef",
    default_dispatcher: DISPATCH_node_undef,
    hash_func: HASH_node_undef,
    profile: PROF_node_undef,
    specializer: SPECIALIZE_node_undef,
    dumper: DUMP_node_undef,
    replacer: REPLACE_node_undef,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_undef_struct>>(),
    children: CHILDREN_node_undef,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_undef_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_undef_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_masgn: NodeKind = NodeKind {
    name: "node_masgn",
    default_dispatcher: DISPATCH_node_masgn,
    hash_func: HASH_node_masgn,
    profile: PROF_node_masgn,
    specializer: SPECIALIZE_node_masgn,
    dumper: DUMP_node_masgn,
    replacer: REPLACE_node_masgn,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_masgn_struct>>(),
    children: CHILDREN_node_masgn,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_masgn_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_masgn_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_masgn_elem: NodeKind = NodeKind {
    name: "node_masgn_elem",
    default_dispatcher: DISPATCH_node_masgn_elem,
    hash_func: HASH_node_masgn_elem,
    profile: PROF_node_masgn_elem,
    specializer: SPECIALIZE_node_masgn_elem,
    dumper: DUMP_node_masgn_elem,
    replacer: REPLACE_node_masgn_elem,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_masgn_elem_struct>>(),
    children: CHILDREN_node_masgn_elem,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_masgn_elem_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_masgn_elem_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_unsupported: NodeKind = NodeKind {
    name: "node_unsupported",
    default_dispatcher: DISPATCH_node_unsupported,
    hash_func: HASH_node_unsupported,
    profile: PROF_node_unsupported,
    specializer: SPECIALIZE_node_unsupported,
    dumper: DUMP_node_unsupported,
    replacer: REPLACE_node_unsupported,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_unsupported_struct>>(),
    children: CHILDREN_node_unsupported,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_unsupported_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_unsupported_struct>>() <= 8);

#[allow(non_upper_case_globals)]
pub static KIND_node_special: NodeKind = NodeKind {
    name: "node_special",
    default_dispatcher: DISPATCH_node_special,
    hash_func: HASH_node_special,
    profile: PROF_node_special,
    specializer: SPECIALIZE_node_special,
    dumper: DUMP_node_special,
    replacer: REPLACE_node_special,
    no_inline: false,
    size: std::mem::size_of::<NodeOf<node_special_struct>>(),
    children: CHILDREN_node_special,
};
const _: () = assert!(std::mem::size_of::<NodeOf<node_special_struct>>() % 8 == 0 && std::mem::align_of::<NodeOf<node_special_struct>>() <= 8);


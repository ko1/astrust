// This file is auto-generated from node.def by astrustgen. Do not edit.

pub type DispatchFn = fn(&mut Ctx, NodePtr, *mut Value) -> RV;
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
    pub line: u32,
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
pub fn {}(c: &mut Ctx, n: NodePtr, fp: *mut Value) -> RV {{ <{} as Disp>::call(c, n, fp) }}
", fname, t));
    sc.emitted.insert(fname.clone());
    fname
}

/// A dispatcher as a type: `Dyn` calls through the node head, an SD type statically.
pub trait Disp { fn call(c: &mut Ctx, n: NodePtr, fp: *mut Value) -> RV; }
/// A list of children: `DynList` loops over the heads, an SD list type is unrolled.
pub trait DispList {
    fn len(ns: NodeList) -> usize;
    fn nth(c: &mut Ctx, ns: NodeList, i: usize, fp: *mut Value) -> RV;
}
pub struct Dyn;
impl Disp for Dyn {
    #[inline(always)]
    fn call(c: &mut Ctx, n: NodePtr, fp: *mut Value) -> RV { unsafe { ((*n).head.dispatcher)(c, n, fp) } }
}
pub struct DynList;
impl DispList for DynList {
    #[inline(always)] fn len(ns: NodeList) -> usize { ns.len() }
    #[inline(always)]
    fn nth(c: &mut Ctx, ns: NodeList, i: usize, fp: *mut Value) -> RV {
        let x = ns[i];
        unsafe { ((*x).head.dispatcher)(c, x, fp) }
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
    pub line: u32,
    pub cc: ConstCache,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_const_path_struct {
    pub parent: NodePtr,
    pub name: Sym,
    pub line: u32,
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
    pub line: u32,
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
    pub cc: CallCache,
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
    pub line: u32,
    pub cc: CallCache,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_call_blk_struct {
    pub recv: NodePtr,
    pub args: NodeList,
    pub mid: Sym,
    pub flags: u32,
    pub line: u32,
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
    pub line: u32,
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
    pub line: u32,
    pub ci: CiRef,
    pub cc: CallCache,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_attrasgn_struct {
    pub recv: NodePtr,
    pub args: NodeList,
    pub mid: Sym,
    pub flags: u32,
    pub line: u32,
    pub cc: CallCache,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_super_struct {
    pub args: NodeList,
    pub line: u32,
    pub ci: CiRef,
    pub zsuper: bool,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_yield_struct {
    pub args: NodeList,
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
    pub line: u32,
    pub cc: CallCache,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_opt_minus_struct {
    pub a: NodePtr,
    pub b: NodePtr,
    pub line: u32,
    pub cc: CallCache,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_opt_mul_struct {
    pub a: NodePtr,
    pub b: NodePtr,
    pub line: u32,
    pub cc: CallCache,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_opt_div_struct {
    pub a: NodePtr,
    pub b: NodePtr,
    pub line: u32,
    pub cc: CallCache,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_opt_mod_struct {
    pub a: NodePtr,
    pub b: NodePtr,
    pub line: u32,
    pub cc: CallCache,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_opt_lt_struct {
    pub a: NodePtr,
    pub b: NodePtr,
    pub line: u32,
    pub cc: CallCache,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_opt_le_struct {
    pub a: NodePtr,
    pub b: NodePtr,
    pub line: u32,
    pub cc: CallCache,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_opt_gt_struct {
    pub a: NodePtr,
    pub b: NodePtr,
    pub line: u32,
    pub cc: CallCache,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_opt_ge_struct {
    pub a: NodePtr,
    pub b: NodePtr,
    pub line: u32,
    pub cc: CallCache,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_opt_eq_struct {
    pub a: NodePtr,
    pub b: NodePtr,
    pub line: u32,
    pub cc: CallCache,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_opt_neq_struct {
    pub a: NodePtr,
    pub b: NodePtr,
    pub line: u32,
    pub cc: CallCache,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_opt_ltlt_struct {
    pub a: NodePtr,
    pub b: NodePtr,
    pub line: u32,
    pub cc: CallCache,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_opt_gtgt_struct {
    pub a: NodePtr,
    pub b: NodePtr,
    pub line: u32,
    pub cc: CallCache,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_opt_xor_struct {
    pub a: NodePtr,
    pub b: NodePtr,
    pub line: u32,
    pub cc: CallCache,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_opt_and_struct {
    pub a: NodePtr,
    pub b: NodePtr,
    pub line: u32,
    pub cc: CallCache,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_opt_or_struct {
    pub a: NodePtr,
    pub b: NodePtr,
    pub line: u32,
    pub cc: CallCache,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_opt_aref_struct {
    pub a: NodePtr,
    pub b: NodePtr,
    pub line: u32,
    pub cc: CallCache,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_opt_aset_struct {
    pub a: NodePtr,
    pub b: NodePtr,
    pub v: NodePtr,
    pub line: u32,
    pub cc: CallCache,
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

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct node_flipflop_struct {
    pub a: NodePtr,
    pub b: NodePtr,
    pub excl: bool,
    pub slot: u32,
}

#[allow(non_snake_case)]
pub fn ALLOC_node_nil() -> NodePtr {
    node_allocate(&KIND_node_nil, node_nil_struct {  })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_true() -> NodePtr {
    node_allocate(&KIND_node_true, node_true_struct {  })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_false() -> NodePtr {
    node_allocate(&KIND_node_false, node_false_struct {  })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_self() -> NodePtr {
    node_allocate(&KIND_node_self, node_self_struct {  })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_lit(v: Value) -> NodePtr {
    node_allocate(&KIND_node_lit, node_lit_struct { v })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_float(v: f64) -> NodePtr {
    node_allocate(&KIND_node_float, node_float_struct { v })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_str(s: Str, enc: u32) -> NodePtr {
    node_allocate(&KIND_node_str, node_str_struct { s, enc })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_fstr(s: Str, enc: u32) -> NodePtr {
    node_allocate(&KIND_node_fstr, node_fstr_struct { s, enc, lc: Default::default() })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_dstr(parts: NodeList, enc: u32) -> NodePtr {
    node_allocate(&KIND_node_dstr, node_dstr_struct { parts, enc })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_tostr(v: NodePtr) -> NodePtr {
    node_allocate(&KIND_node_tostr, node_tostr_struct { v })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_dsym(s: NodePtr) -> NodePtr {
    node_allocate(&KIND_node_dsym, node_dsym_struct { s })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_regex(src: Str, opts: u32) -> NodePtr {
    node_allocate(&KIND_node_regex, node_regex_struct { src, opts, lc: Default::default() })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_dregex(s: NodePtr, opts: u32, once: bool) -> NodePtr {
    node_allocate(&KIND_node_dregex, node_dregex_struct { s, opts, once, lc: Default::default() })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_biglit(digits: Str) -> NodePtr {
    node_allocate(&KIND_node_biglit, node_biglit_struct { digits, lc: Default::default() })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_ary(elems: NodeList) -> NodePtr {
    node_allocate(&KIND_node_ary, node_ary_struct { elems })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_ary_build(elems: NodeList, kinds: Str) -> NodePtr {
    node_allocate(&KIND_node_ary_build, node_ary_build_struct { elems, kinds })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_hash(items: NodeList, kinds: Str) -> NodePtr {
    node_allocate(&KIND_node_hash, node_hash_struct { items, kinds })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_range(b: NodePtr, e: NodePtr, excl: bool) -> NodePtr {
    node_allocate(&KIND_node_range, node_range_struct { b, e, excl })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_range_lit(b: Value, e: Value, excl: bool) -> NodePtr {
    node_allocate(&KIND_node_range_lit, node_range_lit_struct { b, e, excl, lc: Default::default() })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_lvar(idx: u32) -> NodePtr {
    node_allocate(&KIND_node_lvar, node_lvar_struct { idx })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_lasgn(idx: u32, v: NodePtr) -> NodePtr {
    node_allocate(&KIND_node_lasgn, node_lasgn_struct { idx, v })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_dvar(hops: u32, idx: u32) -> NodePtr {
    node_allocate(&KIND_node_dvar, node_dvar_struct { hops, idx })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_dasgn(hops: u32, idx: u32, v: NodePtr) -> NodePtr {
    node_allocate(&KIND_node_dasgn, node_dasgn_struct { hops, idx, v })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_ivar(name: Sym) -> NodePtr {
    node_allocate(&KIND_node_ivar, node_ivar_struct { name, ic: Default::default() })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_iasgn(name: Sym, v: NodePtr) -> NodePtr {
    node_allocate(&KIND_node_iasgn, node_iasgn_struct { name, v, ic: Default::default() })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_gvar(name: Sym) -> NodePtr {
    node_allocate(&KIND_node_gvar, node_gvar_struct { name })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_gasgn(name: Sym, v: NodePtr) -> NodePtr {
    node_allocate(&KIND_node_gasgn, node_gasgn_struct { name, v })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_cvar(name: Sym) -> NodePtr {
    node_allocate(&KIND_node_cvar, node_cvar_struct { name })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_cvasgn(name: Sym, v: NodePtr) -> NodePtr {
    node_allocate(&KIND_node_cvasgn, node_cvasgn_struct { name, v })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_const(name: Sym, line: u32) -> NodePtr {
    node_allocate(&KIND_node_const, node_const_struct { name, line, cc: Default::default() })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_const_path(parent: NodePtr, name: Sym, line: u32) -> NodePtr {
    node_allocate(&KIND_node_const_path, node_const_path_struct { parent, name, line })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_top_const(name: Sym) -> NodePtr {
    node_allocate(&KIND_node_top_const, node_top_const_struct { name })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_cdecl(name: Sym, v: NodePtr) -> NodePtr {
    node_allocate(&KIND_node_cdecl, node_cdecl_struct { name, v })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_cdecl_path(parent: NodePtr, name: Sym, v: NodePtr) -> NodePtr {
    node_allocate(&KIND_node_cdecl_path, node_cdecl_path_struct { parent, name, v })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_errinfo() -> NodePtr {
    node_allocate(&KIND_node_errinfo, node_errinfo_struct {  })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_nth_ref(nth: u32) -> NodePtr {
    node_allocate(&KIND_node_nth_ref, node_nth_ref_struct { nth })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_back_ref(kind: u32) -> NodePtr {
    node_allocate(&KIND_node_back_ref, node_back_ref_struct { kind })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_block(stmts: NodeList) -> NodePtr {
    node_allocate(&KIND_node_block, node_block_struct { stmts })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_line(line: u32, body: NodePtr) -> NodePtr {
    node_allocate(&KIND_node_line, node_line_struct { line, body })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_if(cond: NodePtr, then_b: NodePtr, else_b: NodePtr) -> NodePtr {
    node_allocate(&KIND_node_if, node_if_struct { cond, then_b, else_b })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_and(a: NodePtr, b: NodePtr) -> NodePtr {
    node_allocate(&KIND_node_and, node_and_struct { a, b })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_or(a: NodePtr, b: NodePtr) -> NodePtr {
    node_allocate(&KIND_node_or, node_or_struct { a, b })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_not(v: NodePtr) -> NodePtr {
    node_allocate(&KIND_node_not, node_not_struct { v, cc: Default::default() })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_while(cond: NodePtr, body: NodePtr, until: bool) -> NodePtr {
    node_allocate(&KIND_node_while, node_while_struct { cond, body, until })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_dowhile(cond: NodePtr, body: NodePtr, until: bool) -> NodePtr {
    node_allocate(&KIND_node_dowhile, node_dowhile_struct { cond, body, until })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_loop_break(v: NodePtr) -> NodePtr {
    node_allocate(&KIND_node_loop_break, node_loop_break_struct { v })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_loop_next(v: NodePtr) -> NodePtr {
    node_allocate(&KIND_node_loop_next, node_loop_next_struct { v })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_break(v: NodePtr) -> NodePtr {
    node_allocate(&KIND_node_break, node_break_struct { v })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_next(v: NodePtr) -> NodePtr {
    node_allocate(&KIND_node_next, node_next_struct { v })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_return(v: NodePtr) -> NodePtr {
    node_allocate(&KIND_node_return, node_return_struct { v })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_retry() -> NodePtr {
    node_allocate(&KIND_node_retry, node_retry_struct {  })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_redo() -> NodePtr {
    node_allocate(&KIND_node_redo, node_redo_struct {  })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_rescue(body: NodePtr, clauses: NodeList, else_b: NodePtr, has_else: bool) -> NodePtr {
    node_allocate(&KIND_node_rescue, node_rescue_struct { body, clauses, else_b, has_else })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_resbody(classes: NodeList, kinds: Str, assign: NodePtr, body: NodePtr) -> NodePtr {
    node_allocate(&KIND_node_resbody, node_resbody_struct { classes, kinds, assign, body })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_ensure(body: NodePtr, ens: NodePtr) -> NodePtr {
    node_allocate(&KIND_node_ensure, node_ensure_struct { body, ens })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_when(subj_slot: u32, pats: NodeList, kinds: Str) -> NodePtr {
    node_allocate(&KIND_node_when, node_when_struct { subj_slot, pats, kinds, cc: Default::default() })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_when_lit(subj_slot: u32, v: Value) -> NodePtr {
    node_allocate(&KIND_node_when_lit, node_when_lit_struct { subj_slot, v, cc: Default::default() })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_defined(kind: u32, name: Sym, recv: NodePtr) -> NodePtr {
    node_allocate(&KIND_node_defined, node_defined_struct { kind, name, recv })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_call(recv: NodePtr, args: NodeList, mid: Sym, flags: u32, line: u32) -> NodePtr {
    node_allocate(&KIND_node_call, node_call_struct { recv, args, mid, flags, line, cc: Default::default() })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_call_blk(recv: NodePtr, args: NodeList, mid: Sym, flags: u32, line: u32, blk: IseqRef) -> NodePtr {
    node_allocate(&KIND_node_call_blk, node_call_blk_struct { recv, args, mid, flags, line, blk, cc: Default::default() })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_call_kw(recv: NodePtr, args: NodeList, mid: Sym, flags: u32, line: u32, ci: CiRef) -> NodePtr {
    node_allocate(&KIND_node_call_kw, node_call_kw_struct { recv, args, mid, flags, line, ci, cc: Default::default() })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_call_x(recv: NodePtr, args: NodeList, mid: Sym, flags: u32, line: u32, ci: CiRef) -> NodePtr {
    node_allocate(&KIND_node_call_x, node_call_x_struct { recv, args, mid, flags, line, ci, cc: Default::default() })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_attrasgn(recv: NodePtr, args: NodeList, mid: Sym, flags: u32, line: u32) -> NodePtr {
    node_allocate(&KIND_node_attrasgn, node_attrasgn_struct { recv, args, mid, flags, line, cc: Default::default() })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_super(args: NodeList, line: u32, ci: CiRef, zsuper: bool) -> NodePtr {
    node_allocate(&KIND_node_super, node_super_struct { args, line, ci, zsuper })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_yield(args: NodeList) -> NodePtr {
    node_allocate(&KIND_node_yield, node_yield_struct { args })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_yield_x(args: NodeList, ci: CiRef) -> NodePtr {
    node_allocate(&KIND_node_yield_x, node_yield_x_struct { args, ci })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_block_given() -> NodePtr {
    node_allocate(&KIND_node_block_given, node_block_given_struct {  })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_lambda(blk: IseqRef) -> NodePtr {
    node_allocate(&KIND_node_lambda, node_lambda_struct { blk })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_opt_plus(a: NodePtr, b: NodePtr, line: u32) -> NodePtr {
    node_allocate(&KIND_node_opt_plus, node_opt_plus_struct { a, b, line, cc: Default::default() })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_opt_minus(a: NodePtr, b: NodePtr, line: u32) -> NodePtr {
    node_allocate(&KIND_node_opt_minus, node_opt_minus_struct { a, b, line, cc: Default::default() })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_opt_mul(a: NodePtr, b: NodePtr, line: u32) -> NodePtr {
    node_allocate(&KIND_node_opt_mul, node_opt_mul_struct { a, b, line, cc: Default::default() })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_opt_div(a: NodePtr, b: NodePtr, line: u32) -> NodePtr {
    node_allocate(&KIND_node_opt_div, node_opt_div_struct { a, b, line, cc: Default::default() })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_opt_mod(a: NodePtr, b: NodePtr, line: u32) -> NodePtr {
    node_allocate(&KIND_node_opt_mod, node_opt_mod_struct { a, b, line, cc: Default::default() })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_opt_lt(a: NodePtr, b: NodePtr, line: u32) -> NodePtr {
    node_allocate(&KIND_node_opt_lt, node_opt_lt_struct { a, b, line, cc: Default::default() })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_opt_le(a: NodePtr, b: NodePtr, line: u32) -> NodePtr {
    node_allocate(&KIND_node_opt_le, node_opt_le_struct { a, b, line, cc: Default::default() })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_opt_gt(a: NodePtr, b: NodePtr, line: u32) -> NodePtr {
    node_allocate(&KIND_node_opt_gt, node_opt_gt_struct { a, b, line, cc: Default::default() })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_opt_ge(a: NodePtr, b: NodePtr, line: u32) -> NodePtr {
    node_allocate(&KIND_node_opt_ge, node_opt_ge_struct { a, b, line, cc: Default::default() })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_opt_eq(a: NodePtr, b: NodePtr, line: u32) -> NodePtr {
    node_allocate(&KIND_node_opt_eq, node_opt_eq_struct { a, b, line, cc: Default::default() })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_opt_neq(a: NodePtr, b: NodePtr, line: u32) -> NodePtr {
    node_allocate(&KIND_node_opt_neq, node_opt_neq_struct { a, b, line, cc: Default::default() })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_opt_ltlt(a: NodePtr, b: NodePtr, line: u32) -> NodePtr {
    node_allocate(&KIND_node_opt_ltlt, node_opt_ltlt_struct { a, b, line, cc: Default::default() })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_opt_gtgt(a: NodePtr, b: NodePtr, line: u32) -> NodePtr {
    node_allocate(&KIND_node_opt_gtgt, node_opt_gtgt_struct { a, b, line, cc: Default::default() })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_opt_xor(a: NodePtr, b: NodePtr, line: u32) -> NodePtr {
    node_allocate(&KIND_node_opt_xor, node_opt_xor_struct { a, b, line, cc: Default::default() })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_opt_and(a: NodePtr, b: NodePtr, line: u32) -> NodePtr {
    node_allocate(&KIND_node_opt_and, node_opt_and_struct { a, b, line, cc: Default::default() })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_opt_or(a: NodePtr, b: NodePtr, line: u32) -> NodePtr {
    node_allocate(&KIND_node_opt_or, node_opt_or_struct { a, b, line, cc: Default::default() })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_opt_aref(a: NodePtr, b: NodePtr, line: u32) -> NodePtr {
    node_allocate(&KIND_node_opt_aref, node_opt_aref_struct { a, b, line, cc: Default::default() })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_opt_aset(a: NodePtr, b: NodePtr, v: NodePtr, line: u32) -> NodePtr {
    node_allocate(&KIND_node_opt_aset, node_opt_aset_struct { a, b, v, line, cc: Default::default() })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_def(mid: Sym, body: IseqRef) -> NodePtr {
    node_allocate(&KIND_node_def, node_def_struct { mid, body })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_sdef(recv: NodePtr, mid: Sym, body: IseqRef) -> NodePtr {
    node_allocate(&KIND_node_sdef, node_sdef_struct { recv, mid, body })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_class(cpath: NodePtr, name: Sym, superclass: NodePtr, body: IseqRef, has_cpath: bool, has_super: bool) -> NodePtr {
    node_allocate(&KIND_node_class, node_class_struct { cpath, name, superclass, body, has_cpath, has_super })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_module(cpath: NodePtr, name: Sym, body: IseqRef, has_cpath: bool) -> NodePtr {
    node_allocate(&KIND_node_module, node_module_struct { cpath, name, body, has_cpath })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_sclass(recv: NodePtr, body: IseqRef) -> NodePtr {
    node_allocate(&KIND_node_sclass, node_sclass_struct { recv, body })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_alias(newname: NodePtr, oldname: NodePtr) -> NodePtr {
    node_allocate(&KIND_node_alias, node_alias_struct { newname, oldname })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_valias(newname: Sym, oldname: Sym) -> NodePtr {
    node_allocate(&KIND_node_valias, node_valias_struct { newname, oldname })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_undef(names: NodeList) -> NodePtr {
    node_allocate(&KIND_node_undef, node_undef_struct { names })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_masgn(rhs: NodePtr, tmp: u32, targets: NodeList, splat_rhs: bool) -> NodePtr {
    node_allocate(&KIND_node_masgn, node_masgn_struct { rhs, tmp, targets, splat_rhs })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_masgn_elem(tmp: u32, idx: u32, kind: u32, post: u32) -> NodePtr {
    node_allocate(&KIND_node_masgn_elem, node_masgn_elem_struct { tmp, idx, kind, post })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_unsupported(what: Str) -> NodePtr {
    node_allocate(&KIND_node_unsupported, node_unsupported_struct { what })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_special(kind: u32, arg: NodePtr) -> NodePtr {
    node_allocate(&KIND_node_special, node_special_struct { kind, arg })
}

#[allow(non_snake_case)]
pub fn ALLOC_node_flipflop(a: NodePtr, b: NodePtr, excl: bool, slot: u32) -> NodePtr {
    node_allocate(&KIND_node_flipflop, node_flipflop_struct { a, b, excl, slot })
}

#[allow(non_upper_case_globals)]
pub static KIND_node_nil: NodeKind = NodeKind {
    name: "node_nil",
    default_dispatcher: DISPATCH_node_nil,
    hash_func: HASH_node_nil,
    specializer: SPECIALIZE_node_nil,
    dumper: DUMP_node_nil,
    replacer: REPLACE_node_nil,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_true: NodeKind = NodeKind {
    name: "node_true",
    default_dispatcher: DISPATCH_node_true,
    hash_func: HASH_node_true,
    specializer: SPECIALIZE_node_true,
    dumper: DUMP_node_true,
    replacer: REPLACE_node_true,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_false: NodeKind = NodeKind {
    name: "node_false",
    default_dispatcher: DISPATCH_node_false,
    hash_func: HASH_node_false,
    specializer: SPECIALIZE_node_false,
    dumper: DUMP_node_false,
    replacer: REPLACE_node_false,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_self: NodeKind = NodeKind {
    name: "node_self",
    default_dispatcher: DISPATCH_node_self,
    hash_func: HASH_node_self,
    specializer: SPECIALIZE_node_self,
    dumper: DUMP_node_self,
    replacer: REPLACE_node_self,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_lit: NodeKind = NodeKind {
    name: "node_lit",
    default_dispatcher: DISPATCH_node_lit,
    hash_func: HASH_node_lit,
    specializer: SPECIALIZE_node_lit,
    dumper: DUMP_node_lit,
    replacer: REPLACE_node_lit,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_float: NodeKind = NodeKind {
    name: "node_float",
    default_dispatcher: DISPATCH_node_float,
    hash_func: HASH_node_float,
    specializer: SPECIALIZE_node_float,
    dumper: DUMP_node_float,
    replacer: REPLACE_node_float,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_str: NodeKind = NodeKind {
    name: "node_str",
    default_dispatcher: DISPATCH_node_str,
    hash_func: HASH_node_str,
    specializer: SPECIALIZE_node_str,
    dumper: DUMP_node_str,
    replacer: REPLACE_node_str,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_fstr: NodeKind = NodeKind {
    name: "node_fstr",
    default_dispatcher: DISPATCH_node_fstr,
    hash_func: HASH_node_fstr,
    specializer: SPECIALIZE_node_fstr,
    dumper: DUMP_node_fstr,
    replacer: REPLACE_node_fstr,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_dstr: NodeKind = NodeKind {
    name: "node_dstr",
    default_dispatcher: DISPATCH_node_dstr,
    hash_func: HASH_node_dstr,
    specializer: SPECIALIZE_node_dstr,
    dumper: DUMP_node_dstr,
    replacer: REPLACE_node_dstr,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_tostr: NodeKind = NodeKind {
    name: "node_tostr",
    default_dispatcher: DISPATCH_node_tostr,
    hash_func: HASH_node_tostr,
    specializer: SPECIALIZE_node_tostr,
    dumper: DUMP_node_tostr,
    replacer: REPLACE_node_tostr,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_dsym: NodeKind = NodeKind {
    name: "node_dsym",
    default_dispatcher: DISPATCH_node_dsym,
    hash_func: HASH_node_dsym,
    specializer: SPECIALIZE_node_dsym,
    dumper: DUMP_node_dsym,
    replacer: REPLACE_node_dsym,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_regex: NodeKind = NodeKind {
    name: "node_regex",
    default_dispatcher: DISPATCH_node_regex,
    hash_func: HASH_node_regex,
    specializer: SPECIALIZE_node_regex,
    dumper: DUMP_node_regex,
    replacer: REPLACE_node_regex,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_dregex: NodeKind = NodeKind {
    name: "node_dregex",
    default_dispatcher: DISPATCH_node_dregex,
    hash_func: HASH_node_dregex,
    specializer: SPECIALIZE_node_dregex,
    dumper: DUMP_node_dregex,
    replacer: REPLACE_node_dregex,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_biglit: NodeKind = NodeKind {
    name: "node_biglit",
    default_dispatcher: DISPATCH_node_biglit,
    hash_func: HASH_node_biglit,
    specializer: SPECIALIZE_node_biglit,
    dumper: DUMP_node_biglit,
    replacer: REPLACE_node_biglit,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_ary: NodeKind = NodeKind {
    name: "node_ary",
    default_dispatcher: DISPATCH_node_ary,
    hash_func: HASH_node_ary,
    specializer: SPECIALIZE_node_ary,
    dumper: DUMP_node_ary,
    replacer: REPLACE_node_ary,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_ary_build: NodeKind = NodeKind {
    name: "node_ary_build",
    default_dispatcher: DISPATCH_node_ary_build,
    hash_func: HASH_node_ary_build,
    specializer: SPECIALIZE_node_ary_build,
    dumper: DUMP_node_ary_build,
    replacer: REPLACE_node_ary_build,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_hash: NodeKind = NodeKind {
    name: "node_hash",
    default_dispatcher: DISPATCH_node_hash,
    hash_func: HASH_node_hash,
    specializer: SPECIALIZE_node_hash,
    dumper: DUMP_node_hash,
    replacer: REPLACE_node_hash,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_range: NodeKind = NodeKind {
    name: "node_range",
    default_dispatcher: DISPATCH_node_range,
    hash_func: HASH_node_range,
    specializer: SPECIALIZE_node_range,
    dumper: DUMP_node_range,
    replacer: REPLACE_node_range,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_range_lit: NodeKind = NodeKind {
    name: "node_range_lit",
    default_dispatcher: DISPATCH_node_range_lit,
    hash_func: HASH_node_range_lit,
    specializer: SPECIALIZE_node_range_lit,
    dumper: DUMP_node_range_lit,
    replacer: REPLACE_node_range_lit,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_lvar: NodeKind = NodeKind {
    name: "node_lvar",
    default_dispatcher: DISPATCH_node_lvar,
    hash_func: HASH_node_lvar,
    specializer: SPECIALIZE_node_lvar,
    dumper: DUMP_node_lvar,
    replacer: REPLACE_node_lvar,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_lasgn: NodeKind = NodeKind {
    name: "node_lasgn",
    default_dispatcher: DISPATCH_node_lasgn,
    hash_func: HASH_node_lasgn,
    specializer: SPECIALIZE_node_lasgn,
    dumper: DUMP_node_lasgn,
    replacer: REPLACE_node_lasgn,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_dvar: NodeKind = NodeKind {
    name: "node_dvar",
    default_dispatcher: DISPATCH_node_dvar,
    hash_func: HASH_node_dvar,
    specializer: SPECIALIZE_node_dvar,
    dumper: DUMP_node_dvar,
    replacer: REPLACE_node_dvar,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_dasgn: NodeKind = NodeKind {
    name: "node_dasgn",
    default_dispatcher: DISPATCH_node_dasgn,
    hash_func: HASH_node_dasgn,
    specializer: SPECIALIZE_node_dasgn,
    dumper: DUMP_node_dasgn,
    replacer: REPLACE_node_dasgn,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_ivar: NodeKind = NodeKind {
    name: "node_ivar",
    default_dispatcher: DISPATCH_node_ivar,
    hash_func: HASH_node_ivar,
    specializer: SPECIALIZE_node_ivar,
    dumper: DUMP_node_ivar,
    replacer: REPLACE_node_ivar,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_iasgn: NodeKind = NodeKind {
    name: "node_iasgn",
    default_dispatcher: DISPATCH_node_iasgn,
    hash_func: HASH_node_iasgn,
    specializer: SPECIALIZE_node_iasgn,
    dumper: DUMP_node_iasgn,
    replacer: REPLACE_node_iasgn,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_gvar: NodeKind = NodeKind {
    name: "node_gvar",
    default_dispatcher: DISPATCH_node_gvar,
    hash_func: HASH_node_gvar,
    specializer: SPECIALIZE_node_gvar,
    dumper: DUMP_node_gvar,
    replacer: REPLACE_node_gvar,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_gasgn: NodeKind = NodeKind {
    name: "node_gasgn",
    default_dispatcher: DISPATCH_node_gasgn,
    hash_func: HASH_node_gasgn,
    specializer: SPECIALIZE_node_gasgn,
    dumper: DUMP_node_gasgn,
    replacer: REPLACE_node_gasgn,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_cvar: NodeKind = NodeKind {
    name: "node_cvar",
    default_dispatcher: DISPATCH_node_cvar,
    hash_func: HASH_node_cvar,
    specializer: SPECIALIZE_node_cvar,
    dumper: DUMP_node_cvar,
    replacer: REPLACE_node_cvar,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_cvasgn: NodeKind = NodeKind {
    name: "node_cvasgn",
    default_dispatcher: DISPATCH_node_cvasgn,
    hash_func: HASH_node_cvasgn,
    specializer: SPECIALIZE_node_cvasgn,
    dumper: DUMP_node_cvasgn,
    replacer: REPLACE_node_cvasgn,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_const: NodeKind = NodeKind {
    name: "node_const",
    default_dispatcher: DISPATCH_node_const,
    hash_func: HASH_node_const,
    specializer: SPECIALIZE_node_const,
    dumper: DUMP_node_const,
    replacer: REPLACE_node_const,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_const_path: NodeKind = NodeKind {
    name: "node_const_path",
    default_dispatcher: DISPATCH_node_const_path,
    hash_func: HASH_node_const_path,
    specializer: SPECIALIZE_node_const_path,
    dumper: DUMP_node_const_path,
    replacer: REPLACE_node_const_path,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_top_const: NodeKind = NodeKind {
    name: "node_top_const",
    default_dispatcher: DISPATCH_node_top_const,
    hash_func: HASH_node_top_const,
    specializer: SPECIALIZE_node_top_const,
    dumper: DUMP_node_top_const,
    replacer: REPLACE_node_top_const,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_cdecl: NodeKind = NodeKind {
    name: "node_cdecl",
    default_dispatcher: DISPATCH_node_cdecl,
    hash_func: HASH_node_cdecl,
    specializer: SPECIALIZE_node_cdecl,
    dumper: DUMP_node_cdecl,
    replacer: REPLACE_node_cdecl,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_cdecl_path: NodeKind = NodeKind {
    name: "node_cdecl_path",
    default_dispatcher: DISPATCH_node_cdecl_path,
    hash_func: HASH_node_cdecl_path,
    specializer: SPECIALIZE_node_cdecl_path,
    dumper: DUMP_node_cdecl_path,
    replacer: REPLACE_node_cdecl_path,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_errinfo: NodeKind = NodeKind {
    name: "node_errinfo",
    default_dispatcher: DISPATCH_node_errinfo,
    hash_func: HASH_node_errinfo,
    specializer: SPECIALIZE_node_errinfo,
    dumper: DUMP_node_errinfo,
    replacer: REPLACE_node_errinfo,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_nth_ref: NodeKind = NodeKind {
    name: "node_nth_ref",
    default_dispatcher: DISPATCH_node_nth_ref,
    hash_func: HASH_node_nth_ref,
    specializer: SPECIALIZE_node_nth_ref,
    dumper: DUMP_node_nth_ref,
    replacer: REPLACE_node_nth_ref,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_back_ref: NodeKind = NodeKind {
    name: "node_back_ref",
    default_dispatcher: DISPATCH_node_back_ref,
    hash_func: HASH_node_back_ref,
    specializer: SPECIALIZE_node_back_ref,
    dumper: DUMP_node_back_ref,
    replacer: REPLACE_node_back_ref,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_block: NodeKind = NodeKind {
    name: "node_block",
    default_dispatcher: DISPATCH_node_block,
    hash_func: HASH_node_block,
    specializer: SPECIALIZE_node_block,
    dumper: DUMP_node_block,
    replacer: REPLACE_node_block,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_line: NodeKind = NodeKind {
    name: "node_line",
    default_dispatcher: DISPATCH_node_line,
    hash_func: HASH_node_line,
    specializer: SPECIALIZE_node_line,
    dumper: DUMP_node_line,
    replacer: REPLACE_node_line,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_if: NodeKind = NodeKind {
    name: "node_if",
    default_dispatcher: DISPATCH_node_if,
    hash_func: HASH_node_if,
    specializer: SPECIALIZE_node_if,
    dumper: DUMP_node_if,
    replacer: REPLACE_node_if,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_and: NodeKind = NodeKind {
    name: "node_and",
    default_dispatcher: DISPATCH_node_and,
    hash_func: HASH_node_and,
    specializer: SPECIALIZE_node_and,
    dumper: DUMP_node_and,
    replacer: REPLACE_node_and,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_or: NodeKind = NodeKind {
    name: "node_or",
    default_dispatcher: DISPATCH_node_or,
    hash_func: HASH_node_or,
    specializer: SPECIALIZE_node_or,
    dumper: DUMP_node_or,
    replacer: REPLACE_node_or,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_not: NodeKind = NodeKind {
    name: "node_not",
    default_dispatcher: DISPATCH_node_not,
    hash_func: HASH_node_not,
    specializer: SPECIALIZE_node_not,
    dumper: DUMP_node_not,
    replacer: REPLACE_node_not,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_while: NodeKind = NodeKind {
    name: "node_while",
    default_dispatcher: DISPATCH_node_while,
    hash_func: HASH_node_while,
    specializer: SPECIALIZE_node_while,
    dumper: DUMP_node_while,
    replacer: REPLACE_node_while,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_dowhile: NodeKind = NodeKind {
    name: "node_dowhile",
    default_dispatcher: DISPATCH_node_dowhile,
    hash_func: HASH_node_dowhile,
    specializer: SPECIALIZE_node_dowhile,
    dumper: DUMP_node_dowhile,
    replacer: REPLACE_node_dowhile,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_loop_break: NodeKind = NodeKind {
    name: "node_loop_break",
    default_dispatcher: DISPATCH_node_loop_break,
    hash_func: HASH_node_loop_break,
    specializer: SPECIALIZE_node_loop_break,
    dumper: DUMP_node_loop_break,
    replacer: REPLACE_node_loop_break,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_loop_next: NodeKind = NodeKind {
    name: "node_loop_next",
    default_dispatcher: DISPATCH_node_loop_next,
    hash_func: HASH_node_loop_next,
    specializer: SPECIALIZE_node_loop_next,
    dumper: DUMP_node_loop_next,
    replacer: REPLACE_node_loop_next,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_break: NodeKind = NodeKind {
    name: "node_break",
    default_dispatcher: DISPATCH_node_break,
    hash_func: HASH_node_break,
    specializer: SPECIALIZE_node_break,
    dumper: DUMP_node_break,
    replacer: REPLACE_node_break,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_next: NodeKind = NodeKind {
    name: "node_next",
    default_dispatcher: DISPATCH_node_next,
    hash_func: HASH_node_next,
    specializer: SPECIALIZE_node_next,
    dumper: DUMP_node_next,
    replacer: REPLACE_node_next,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_return: NodeKind = NodeKind {
    name: "node_return",
    default_dispatcher: DISPATCH_node_return,
    hash_func: HASH_node_return,
    specializer: SPECIALIZE_node_return,
    dumper: DUMP_node_return,
    replacer: REPLACE_node_return,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_retry: NodeKind = NodeKind {
    name: "node_retry",
    default_dispatcher: DISPATCH_node_retry,
    hash_func: HASH_node_retry,
    specializer: SPECIALIZE_node_retry,
    dumper: DUMP_node_retry,
    replacer: REPLACE_node_retry,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_redo: NodeKind = NodeKind {
    name: "node_redo",
    default_dispatcher: DISPATCH_node_redo,
    hash_func: HASH_node_redo,
    specializer: SPECIALIZE_node_redo,
    dumper: DUMP_node_redo,
    replacer: REPLACE_node_redo,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_rescue: NodeKind = NodeKind {
    name: "node_rescue",
    default_dispatcher: DISPATCH_node_rescue,
    hash_func: HASH_node_rescue,
    specializer: SPECIALIZE_node_rescue,
    dumper: DUMP_node_rescue,
    replacer: REPLACE_node_rescue,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_resbody: NodeKind = NodeKind {
    name: "node_resbody",
    default_dispatcher: DISPATCH_node_resbody,
    hash_func: HASH_node_resbody,
    specializer: SPECIALIZE_node_resbody,
    dumper: DUMP_node_resbody,
    replacer: REPLACE_node_resbody,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_ensure: NodeKind = NodeKind {
    name: "node_ensure",
    default_dispatcher: DISPATCH_node_ensure,
    hash_func: HASH_node_ensure,
    specializer: SPECIALIZE_node_ensure,
    dumper: DUMP_node_ensure,
    replacer: REPLACE_node_ensure,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_when: NodeKind = NodeKind {
    name: "node_when",
    default_dispatcher: DISPATCH_node_when,
    hash_func: HASH_node_when,
    specializer: SPECIALIZE_node_when,
    dumper: DUMP_node_when,
    replacer: REPLACE_node_when,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_when_lit: NodeKind = NodeKind {
    name: "node_when_lit",
    default_dispatcher: DISPATCH_node_when_lit,
    hash_func: HASH_node_when_lit,
    specializer: SPECIALIZE_node_when_lit,
    dumper: DUMP_node_when_lit,
    replacer: REPLACE_node_when_lit,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_defined: NodeKind = NodeKind {
    name: "node_defined",
    default_dispatcher: DISPATCH_node_defined,
    hash_func: HASH_node_defined,
    specializer: SPECIALIZE_node_defined,
    dumper: DUMP_node_defined,
    replacer: REPLACE_node_defined,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_call: NodeKind = NodeKind {
    name: "node_call",
    default_dispatcher: DISPATCH_node_call,
    hash_func: HASH_node_call,
    specializer: SPECIALIZE_node_call,
    dumper: DUMP_node_call,
    replacer: REPLACE_node_call,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_call_blk: NodeKind = NodeKind {
    name: "node_call_blk",
    default_dispatcher: DISPATCH_node_call_blk,
    hash_func: HASH_node_call_blk,
    specializer: SPECIALIZE_node_call_blk,
    dumper: DUMP_node_call_blk,
    replacer: REPLACE_node_call_blk,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_call_kw: NodeKind = NodeKind {
    name: "node_call_kw",
    default_dispatcher: DISPATCH_node_call_kw,
    hash_func: HASH_node_call_kw,
    specializer: SPECIALIZE_node_call_kw,
    dumper: DUMP_node_call_kw,
    replacer: REPLACE_node_call_kw,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_call_x: NodeKind = NodeKind {
    name: "node_call_x",
    default_dispatcher: DISPATCH_node_call_x,
    hash_func: HASH_node_call_x,
    specializer: SPECIALIZE_node_call_x,
    dumper: DUMP_node_call_x,
    replacer: REPLACE_node_call_x,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_attrasgn: NodeKind = NodeKind {
    name: "node_attrasgn",
    default_dispatcher: DISPATCH_node_attrasgn,
    hash_func: HASH_node_attrasgn,
    specializer: SPECIALIZE_node_attrasgn,
    dumper: DUMP_node_attrasgn,
    replacer: REPLACE_node_attrasgn,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_super: NodeKind = NodeKind {
    name: "node_super",
    default_dispatcher: DISPATCH_node_super,
    hash_func: HASH_node_super,
    specializer: SPECIALIZE_node_super,
    dumper: DUMP_node_super,
    replacer: REPLACE_node_super,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_yield: NodeKind = NodeKind {
    name: "node_yield",
    default_dispatcher: DISPATCH_node_yield,
    hash_func: HASH_node_yield,
    specializer: SPECIALIZE_node_yield,
    dumper: DUMP_node_yield,
    replacer: REPLACE_node_yield,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_yield_x: NodeKind = NodeKind {
    name: "node_yield_x",
    default_dispatcher: DISPATCH_node_yield_x,
    hash_func: HASH_node_yield_x,
    specializer: SPECIALIZE_node_yield_x,
    dumper: DUMP_node_yield_x,
    replacer: REPLACE_node_yield_x,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_block_given: NodeKind = NodeKind {
    name: "node_block_given",
    default_dispatcher: DISPATCH_node_block_given,
    hash_func: HASH_node_block_given,
    specializer: SPECIALIZE_node_block_given,
    dumper: DUMP_node_block_given,
    replacer: REPLACE_node_block_given,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_lambda: NodeKind = NodeKind {
    name: "node_lambda",
    default_dispatcher: DISPATCH_node_lambda,
    hash_func: HASH_node_lambda,
    specializer: SPECIALIZE_node_lambda,
    dumper: DUMP_node_lambda,
    replacer: REPLACE_node_lambda,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_opt_plus: NodeKind = NodeKind {
    name: "node_opt_plus",
    default_dispatcher: DISPATCH_node_opt_plus,
    hash_func: HASH_node_opt_plus,
    specializer: SPECIALIZE_node_opt_plus,
    dumper: DUMP_node_opt_plus,
    replacer: REPLACE_node_opt_plus,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_opt_minus: NodeKind = NodeKind {
    name: "node_opt_minus",
    default_dispatcher: DISPATCH_node_opt_minus,
    hash_func: HASH_node_opt_minus,
    specializer: SPECIALIZE_node_opt_minus,
    dumper: DUMP_node_opt_minus,
    replacer: REPLACE_node_opt_minus,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_opt_mul: NodeKind = NodeKind {
    name: "node_opt_mul",
    default_dispatcher: DISPATCH_node_opt_mul,
    hash_func: HASH_node_opt_mul,
    specializer: SPECIALIZE_node_opt_mul,
    dumper: DUMP_node_opt_mul,
    replacer: REPLACE_node_opt_mul,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_opt_div: NodeKind = NodeKind {
    name: "node_opt_div",
    default_dispatcher: DISPATCH_node_opt_div,
    hash_func: HASH_node_opt_div,
    specializer: SPECIALIZE_node_opt_div,
    dumper: DUMP_node_opt_div,
    replacer: REPLACE_node_opt_div,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_opt_mod: NodeKind = NodeKind {
    name: "node_opt_mod",
    default_dispatcher: DISPATCH_node_opt_mod,
    hash_func: HASH_node_opt_mod,
    specializer: SPECIALIZE_node_opt_mod,
    dumper: DUMP_node_opt_mod,
    replacer: REPLACE_node_opt_mod,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_opt_lt: NodeKind = NodeKind {
    name: "node_opt_lt",
    default_dispatcher: DISPATCH_node_opt_lt,
    hash_func: HASH_node_opt_lt,
    specializer: SPECIALIZE_node_opt_lt,
    dumper: DUMP_node_opt_lt,
    replacer: REPLACE_node_opt_lt,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_opt_le: NodeKind = NodeKind {
    name: "node_opt_le",
    default_dispatcher: DISPATCH_node_opt_le,
    hash_func: HASH_node_opt_le,
    specializer: SPECIALIZE_node_opt_le,
    dumper: DUMP_node_opt_le,
    replacer: REPLACE_node_opt_le,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_opt_gt: NodeKind = NodeKind {
    name: "node_opt_gt",
    default_dispatcher: DISPATCH_node_opt_gt,
    hash_func: HASH_node_opt_gt,
    specializer: SPECIALIZE_node_opt_gt,
    dumper: DUMP_node_opt_gt,
    replacer: REPLACE_node_opt_gt,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_opt_ge: NodeKind = NodeKind {
    name: "node_opt_ge",
    default_dispatcher: DISPATCH_node_opt_ge,
    hash_func: HASH_node_opt_ge,
    specializer: SPECIALIZE_node_opt_ge,
    dumper: DUMP_node_opt_ge,
    replacer: REPLACE_node_opt_ge,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_opt_eq: NodeKind = NodeKind {
    name: "node_opt_eq",
    default_dispatcher: DISPATCH_node_opt_eq,
    hash_func: HASH_node_opt_eq,
    specializer: SPECIALIZE_node_opt_eq,
    dumper: DUMP_node_opt_eq,
    replacer: REPLACE_node_opt_eq,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_opt_neq: NodeKind = NodeKind {
    name: "node_opt_neq",
    default_dispatcher: DISPATCH_node_opt_neq,
    hash_func: HASH_node_opt_neq,
    specializer: SPECIALIZE_node_opt_neq,
    dumper: DUMP_node_opt_neq,
    replacer: REPLACE_node_opt_neq,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_opt_ltlt: NodeKind = NodeKind {
    name: "node_opt_ltlt",
    default_dispatcher: DISPATCH_node_opt_ltlt,
    hash_func: HASH_node_opt_ltlt,
    specializer: SPECIALIZE_node_opt_ltlt,
    dumper: DUMP_node_opt_ltlt,
    replacer: REPLACE_node_opt_ltlt,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_opt_gtgt: NodeKind = NodeKind {
    name: "node_opt_gtgt",
    default_dispatcher: DISPATCH_node_opt_gtgt,
    hash_func: HASH_node_opt_gtgt,
    specializer: SPECIALIZE_node_opt_gtgt,
    dumper: DUMP_node_opt_gtgt,
    replacer: REPLACE_node_opt_gtgt,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_opt_xor: NodeKind = NodeKind {
    name: "node_opt_xor",
    default_dispatcher: DISPATCH_node_opt_xor,
    hash_func: HASH_node_opt_xor,
    specializer: SPECIALIZE_node_opt_xor,
    dumper: DUMP_node_opt_xor,
    replacer: REPLACE_node_opt_xor,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_opt_and: NodeKind = NodeKind {
    name: "node_opt_and",
    default_dispatcher: DISPATCH_node_opt_and,
    hash_func: HASH_node_opt_and,
    specializer: SPECIALIZE_node_opt_and,
    dumper: DUMP_node_opt_and,
    replacer: REPLACE_node_opt_and,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_opt_or: NodeKind = NodeKind {
    name: "node_opt_or",
    default_dispatcher: DISPATCH_node_opt_or,
    hash_func: HASH_node_opt_or,
    specializer: SPECIALIZE_node_opt_or,
    dumper: DUMP_node_opt_or,
    replacer: REPLACE_node_opt_or,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_opt_aref: NodeKind = NodeKind {
    name: "node_opt_aref",
    default_dispatcher: DISPATCH_node_opt_aref,
    hash_func: HASH_node_opt_aref,
    specializer: SPECIALIZE_node_opt_aref,
    dumper: DUMP_node_opt_aref,
    replacer: REPLACE_node_opt_aref,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_opt_aset: NodeKind = NodeKind {
    name: "node_opt_aset",
    default_dispatcher: DISPATCH_node_opt_aset,
    hash_func: HASH_node_opt_aset,
    specializer: SPECIALIZE_node_opt_aset,
    dumper: DUMP_node_opt_aset,
    replacer: REPLACE_node_opt_aset,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_def: NodeKind = NodeKind {
    name: "node_def",
    default_dispatcher: DISPATCH_node_def,
    hash_func: HASH_node_def,
    specializer: SPECIALIZE_node_def,
    dumper: DUMP_node_def,
    replacer: REPLACE_node_def,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_sdef: NodeKind = NodeKind {
    name: "node_sdef",
    default_dispatcher: DISPATCH_node_sdef,
    hash_func: HASH_node_sdef,
    specializer: SPECIALIZE_node_sdef,
    dumper: DUMP_node_sdef,
    replacer: REPLACE_node_sdef,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_class: NodeKind = NodeKind {
    name: "node_class",
    default_dispatcher: DISPATCH_node_class,
    hash_func: HASH_node_class,
    specializer: SPECIALIZE_node_class,
    dumper: DUMP_node_class,
    replacer: REPLACE_node_class,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_module: NodeKind = NodeKind {
    name: "node_module",
    default_dispatcher: DISPATCH_node_module,
    hash_func: HASH_node_module,
    specializer: SPECIALIZE_node_module,
    dumper: DUMP_node_module,
    replacer: REPLACE_node_module,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_sclass: NodeKind = NodeKind {
    name: "node_sclass",
    default_dispatcher: DISPATCH_node_sclass,
    hash_func: HASH_node_sclass,
    specializer: SPECIALIZE_node_sclass,
    dumper: DUMP_node_sclass,
    replacer: REPLACE_node_sclass,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_alias: NodeKind = NodeKind {
    name: "node_alias",
    default_dispatcher: DISPATCH_node_alias,
    hash_func: HASH_node_alias,
    specializer: SPECIALIZE_node_alias,
    dumper: DUMP_node_alias,
    replacer: REPLACE_node_alias,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_valias: NodeKind = NodeKind {
    name: "node_valias",
    default_dispatcher: DISPATCH_node_valias,
    hash_func: HASH_node_valias,
    specializer: SPECIALIZE_node_valias,
    dumper: DUMP_node_valias,
    replacer: REPLACE_node_valias,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_undef: NodeKind = NodeKind {
    name: "node_undef",
    default_dispatcher: DISPATCH_node_undef,
    hash_func: HASH_node_undef,
    specializer: SPECIALIZE_node_undef,
    dumper: DUMP_node_undef,
    replacer: REPLACE_node_undef,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_masgn: NodeKind = NodeKind {
    name: "node_masgn",
    default_dispatcher: DISPATCH_node_masgn,
    hash_func: HASH_node_masgn,
    specializer: SPECIALIZE_node_masgn,
    dumper: DUMP_node_masgn,
    replacer: REPLACE_node_masgn,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_masgn_elem: NodeKind = NodeKind {
    name: "node_masgn_elem",
    default_dispatcher: DISPATCH_node_masgn_elem,
    hash_func: HASH_node_masgn_elem,
    specializer: SPECIALIZE_node_masgn_elem,
    dumper: DUMP_node_masgn_elem,
    replacer: REPLACE_node_masgn_elem,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_unsupported: NodeKind = NodeKind {
    name: "node_unsupported",
    default_dispatcher: DISPATCH_node_unsupported,
    hash_func: HASH_node_unsupported,
    specializer: SPECIALIZE_node_unsupported,
    dumper: DUMP_node_unsupported,
    replacer: REPLACE_node_unsupported,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_special: NodeKind = NodeKind {
    name: "node_special",
    default_dispatcher: DISPATCH_node_special,
    hash_func: HASH_node_special,
    specializer: SPECIALIZE_node_special,
    dumper: DUMP_node_special,
    replacer: REPLACE_node_special,
    no_inline: false,
};

#[allow(non_upper_case_globals)]
pub static KIND_node_flipflop: NodeKind = NodeKind {
    name: "node_flipflop",
    default_dispatcher: DISPATCH_node_flipflop,
    hash_func: HASH_node_flipflop,
    specializer: SPECIALIZE_node_flipflop,
    dumper: DUMP_node_flipflop,
    replacer: REPLACE_node_flipflop,
    no_inline: false,
};


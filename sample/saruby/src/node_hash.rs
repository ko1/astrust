// This file is auto-generated from node.def by astrustgen. Do not edit.
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_nil(n: NodePtr) -> u64 {
    let u = node_u::<node_nil_struct>(n);
    let mut h = hash_str("node_nil");

    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_nil(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_nil_struct>(n);
    let mut h = hash_str("node_nil");

    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_true(n: NodePtr) -> u64 {
    let u = node_u::<node_true_struct>(n);
    let mut h = hash_str("node_true");

    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_true(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_true_struct>(n);
    let mut h = hash_str("node_true");

    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_false(n: NodePtr) -> u64 {
    let u = node_u::<node_false_struct>(n);
    let mut h = hash_str("node_false");

    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_false(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_false_struct>(n);
    let mut h = hash_str("node_false");

    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_self(n: NodePtr) -> u64 {
    let u = node_u::<node_self_struct>(n);
    let mut h = hash_str("node_self");

    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_self(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_self_struct>(n);
    let mut h = hash_str("node_self");

    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_lit(n: NodePtr) -> u64 {
    let u = node_u::<node_lit_struct>(n);
    let mut h = hash_str("node_lit");
    h = hash_merge(h, crate::compile::value_hash(u.v));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_lit(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_lit_struct>(n);
    let mut h = hash_str("node_lit");

    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_float(n: NodePtr) -> u64 {
    let u = node_u::<node_float_struct>(n);
    let mut h = hash_str("node_float");
    h = hash_merge(h, hash_u64(u.v.to_bits()));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_float(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_float_struct>(n);
    let mut h = hash_str("node_float");

    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_str(n: NodePtr) -> u64 {
    let u = node_u::<node_str_struct>(n);
    let mut h = hash_str("node_str");
    h = hash_merge(h, hash_bytes(u.s));
    h = hash_merge(h, hash_u64(u.enc as u64));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_str(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_str_struct>(n);
    let mut h = hash_str("node_str");

    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_fstr(n: NodePtr) -> u64 {
    let u = node_u::<node_fstr_struct>(n);
    let mut h = hash_str("node_fstr");
    h = hash_merge(h, hash_bytes(u.s));
    h = hash_merge(h, hash_u64(u.enc as u64));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_fstr(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_fstr_struct>(n);
    let mut h = hash_str("node_fstr");

    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_dstr(n: NodePtr) -> u64 {
    let u = node_u::<node_dstr_struct>(n);
    let mut h = hash_str("node_dstr");
    h = hash_merge(h, hash_node_list(u.parts));
    h = hash_merge(h, hash_u64(u.enc as u64));
    h = hash_merge(h, hash_u64(u.cap as u64));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_dstr(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_dstr_struct>(n);
    let mut h = hash_str("node_dstr");
    for x in u.parts.iter() { h = hash_merge(h, profile_node(pc, *x)); }
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_tostr(n: NodePtr) -> u64 {
    let u = node_u::<node_tostr_struct>(n);
    let mut h = hash_str("node_tostr");
    h = hash_merge(h, hash_node(u.v));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_tostr(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_tostr_struct>(n);
    let mut h = hash_str("node_tostr");
    h = hash_merge(h, profile_node(pc, u.v));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_dsym(n: NodePtr) -> u64 {
    let u = node_u::<node_dsym_struct>(n);
    let mut h = hash_str("node_dsym");
    h = hash_merge(h, hash_node(u.s));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_dsym(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_dsym_struct>(n);
    let mut h = hash_str("node_dsym");
    h = hash_merge(h, profile_node(pc, u.s));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_regex(n: NodePtr) -> u64 {
    let u = node_u::<node_regex_struct>(n);
    let mut h = hash_str("node_regex");
    h = hash_merge(h, hash_bytes(u.src));
    h = hash_merge(h, hash_u64(u.opts as u64));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_regex(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_regex_struct>(n);
    let mut h = hash_str("node_regex");

    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_dregex(n: NodePtr) -> u64 {
    let u = node_u::<node_dregex_struct>(n);
    let mut h = hash_str("node_dregex");
    h = hash_merge(h, hash_node(u.s));
    h = hash_merge(h, hash_u64(u.opts as u64));
    h = hash_merge(h, hash_u64(u.once as u64));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_dregex(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_dregex_struct>(n);
    let mut h = hash_str("node_dregex");
    h = hash_merge(h, profile_node(pc, u.s));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_biglit(n: NodePtr) -> u64 {
    let u = node_u::<node_biglit_struct>(n);
    let mut h = hash_str("node_biglit");
    h = hash_merge(h, hash_bytes(u.digits));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_biglit(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_biglit_struct>(n);
    let mut h = hash_str("node_biglit");

    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_ary(n: NodePtr) -> u64 {
    let u = node_u::<node_ary_struct>(n);
    let mut h = hash_str("node_ary");
    h = hash_merge(h, hash_node_list(u.elems));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_ary(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_ary_struct>(n);
    let mut h = hash_str("node_ary");
    for x in u.elems.iter() { h = hash_merge(h, profile_node(pc, *x)); }
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_ary_build(n: NodePtr) -> u64 {
    let u = node_u::<node_ary_build_struct>(n);
    let mut h = hash_str("node_ary_build");
    h = hash_merge(h, hash_node_list(u.elems));
    h = hash_merge(h, hash_bytes(u.kinds));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_ary_build(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_ary_build_struct>(n);
    let mut h = hash_str("node_ary_build");
    for x in u.elems.iter() { h = hash_merge(h, profile_node(pc, *x)); }
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_hash(n: NodePtr) -> u64 {
    let u = node_u::<node_hash_struct>(n);
    let mut h = hash_str("node_hash");
    h = hash_merge(h, hash_node_list(u.items));
    h = hash_merge(h, hash_bytes(u.kinds));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_hash(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_hash_struct>(n);
    let mut h = hash_str("node_hash");
    for x in u.items.iter() { h = hash_merge(h, profile_node(pc, *x)); }
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_range(n: NodePtr) -> u64 {
    let u = node_u::<node_range_struct>(n);
    let mut h = hash_str("node_range");
    h = hash_merge(h, hash_node(u.b));
    h = hash_merge(h, hash_node(u.e));
    h = hash_merge(h, hash_u64(u.excl as u64));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_range(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_range_struct>(n);
    let mut h = hash_str("node_range");
    h = hash_merge(h, profile_node(pc, u.b));
    h = hash_merge(h, profile_node(pc, u.e));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_range_lit(n: NodePtr) -> u64 {
    let u = node_u::<node_range_lit_struct>(n);
    let mut h = hash_str("node_range_lit");
    h = hash_merge(h, crate::compile::value_hash(u.b));
    h = hash_merge(h, crate::compile::value_hash(u.e));
    h = hash_merge(h, hash_u64(u.excl as u64));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_range_lit(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_range_lit_struct>(n);
    let mut h = hash_str("node_range_lit");

    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_lvar(n: NodePtr) -> u64 {
    let u = node_u::<node_lvar_struct>(n);
    let mut h = hash_str("node_lvar");
    h = hash_merge(h, hash_u64(u.idx as u64));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_lvar(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_lvar_struct>(n);
    let mut h = hash_str("node_lvar");

    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_lasgn(n: NodePtr) -> u64 {
    let u = node_u::<node_lasgn_struct>(n);
    let mut h = hash_str("node_lasgn");
    h = hash_merge(h, hash_u64(u.idx as u64));
    h = hash_merge(h, hash_node(u.v));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_lasgn(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_lasgn_struct>(n);
    let mut h = hash_str("node_lasgn");
    h = hash_merge(h, profile_node(pc, u.v));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_dvar(n: NodePtr) -> u64 {
    let u = node_u::<node_dvar_struct>(n);
    let mut h = hash_str("node_dvar");
    h = hash_merge(h, hash_u64(u.hops as u64));
    h = hash_merge(h, hash_u64(u.idx as u64));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_dvar(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_dvar_struct>(n);
    let mut h = hash_str("node_dvar");

    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_dasgn(n: NodePtr) -> u64 {
    let u = node_u::<node_dasgn_struct>(n);
    let mut h = hash_str("node_dasgn");
    h = hash_merge(h, hash_u64(u.hops as u64));
    h = hash_merge(h, hash_u64(u.idx as u64));
    h = hash_merge(h, hash_node(u.v));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_dasgn(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_dasgn_struct>(n);
    let mut h = hash_str("node_dasgn");
    h = hash_merge(h, profile_node(pc, u.v));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_ivar(n: NodePtr) -> u64 {
    let u = node_u::<node_ivar_struct>(n);
    let mut h = hash_str("node_ivar");
    h = hash_merge(h, hash_bytes(crate::symbol::sym_bytes(u.name)));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_ivar(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_ivar_struct>(n);
    let mut h = hash_str("node_ivar");
    h = hash_merge(h, hash_u64(({ if u.ic.klass.get() != 0 { u.ic.idx.get() + 1 } else { 0 } }) as u64));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_iasgn(n: NodePtr) -> u64 {
    let u = node_u::<node_iasgn_struct>(n);
    let mut h = hash_str("node_iasgn");
    h = hash_merge(h, hash_bytes(crate::symbol::sym_bytes(u.name)));
    h = hash_merge(h, hash_node(u.v));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_iasgn(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_iasgn_struct>(n);
    let mut h = hash_str("node_iasgn");
    h = hash_merge(h, profile_node(pc, u.v));
    h = hash_merge(h, hash_u64(({ if u.ic.klass.get() != 0 { u.ic.idx.get() + 1 } else { 0 } }) as u64));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_ivar_arith(n: NodePtr) -> u64 {
    let u = node_u::<node_ivar_arith_struct>(n);
    let mut h = hash_str("node_ivar_arith");
    h = hash_merge(h, hash_bytes(crate::symbol::sym_bytes(u.name)));
    h = hash_merge(h, hash_u64(u.sub as u64));
    h = hash_merge(h, hash_node(u.v));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_ivar_arith(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_ivar_arith_struct>(n);
    let mut h = hash_str("node_ivar_arith");
    h = hash_merge(h, profile_node(pc, u.v));
    h = hash_merge(h, hash_u64(({ if u.ic.klass.get() != 0 { u.ic.idx.get() + 1 } else { 0 } }) as u64));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_gvar(n: NodePtr) -> u64 {
    let u = node_u::<node_gvar_struct>(n);
    let mut h = hash_str("node_gvar");
    h = hash_merge(h, hash_bytes(crate::symbol::sym_bytes(u.name)));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_gvar(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_gvar_struct>(n);
    let mut h = hash_str("node_gvar");

    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_gasgn(n: NodePtr) -> u64 {
    let u = node_u::<node_gasgn_struct>(n);
    let mut h = hash_str("node_gasgn");
    h = hash_merge(h, hash_bytes(crate::symbol::sym_bytes(u.name)));
    h = hash_merge(h, hash_node(u.v));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_gasgn(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_gasgn_struct>(n);
    let mut h = hash_str("node_gasgn");
    h = hash_merge(h, profile_node(pc, u.v));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_cvar(n: NodePtr) -> u64 {
    let u = node_u::<node_cvar_struct>(n);
    let mut h = hash_str("node_cvar");
    h = hash_merge(h, hash_bytes(crate::symbol::sym_bytes(u.name)));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_cvar(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_cvar_struct>(n);
    let mut h = hash_str("node_cvar");

    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_cvasgn(n: NodePtr) -> u64 {
    let u = node_u::<node_cvasgn_struct>(n);
    let mut h = hash_str("node_cvasgn");
    h = hash_merge(h, hash_bytes(crate::symbol::sym_bytes(u.name)));
    h = hash_merge(h, hash_node(u.v));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_cvasgn(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_cvasgn_struct>(n);
    let mut h = hash_str("node_cvasgn");
    h = hash_merge(h, profile_node(pc, u.v));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_const(n: NodePtr) -> u64 {
    let u = node_u::<node_const_struct>(n);
    let mut h = hash_str("node_const");
    h = hash_merge(h, hash_bytes(crate::symbol::sym_bytes(u.name)));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_const(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_const_struct>(n);
    let mut h = hash_str("node_const");

    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_const_path(n: NodePtr) -> u64 {
    let u = node_u::<node_const_path_struct>(n);
    let mut h = hash_str("node_const_path");
    h = hash_merge(h, hash_node(u.parent));
    h = hash_merge(h, hash_bytes(crate::symbol::sym_bytes(u.name)));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_const_path(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_const_path_struct>(n);
    let mut h = hash_str("node_const_path");
    h = hash_merge(h, profile_node(pc, u.parent));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_top_const(n: NodePtr) -> u64 {
    let u = node_u::<node_top_const_struct>(n);
    let mut h = hash_str("node_top_const");
    h = hash_merge(h, hash_bytes(crate::symbol::sym_bytes(u.name)));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_top_const(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_top_const_struct>(n);
    let mut h = hash_str("node_top_const");

    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_cdecl(n: NodePtr) -> u64 {
    let u = node_u::<node_cdecl_struct>(n);
    let mut h = hash_str("node_cdecl");
    h = hash_merge(h, hash_bytes(crate::symbol::sym_bytes(u.name)));
    h = hash_merge(h, hash_node(u.v));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_cdecl(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_cdecl_struct>(n);
    let mut h = hash_str("node_cdecl");
    h = hash_merge(h, profile_node(pc, u.v));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_cdecl_path(n: NodePtr) -> u64 {
    let u = node_u::<node_cdecl_path_struct>(n);
    let mut h = hash_str("node_cdecl_path");
    h = hash_merge(h, hash_node(u.parent));
    h = hash_merge(h, hash_bytes(crate::symbol::sym_bytes(u.name)));
    h = hash_merge(h, hash_node(u.v));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_cdecl_path(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_cdecl_path_struct>(n);
    let mut h = hash_str("node_cdecl_path");
    h = hash_merge(h, profile_node(pc, u.parent));
    h = hash_merge(h, profile_node(pc, u.v));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_errinfo(n: NodePtr) -> u64 {
    let u = node_u::<node_errinfo_struct>(n);
    let mut h = hash_str("node_errinfo");

    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_errinfo(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_errinfo_struct>(n);
    let mut h = hash_str("node_errinfo");

    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_nth_ref(n: NodePtr) -> u64 {
    let u = node_u::<node_nth_ref_struct>(n);
    let mut h = hash_str("node_nth_ref");
    h = hash_merge(h, hash_u64(u.nth as u64));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_nth_ref(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_nth_ref_struct>(n);
    let mut h = hash_str("node_nth_ref");

    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_back_ref(n: NodePtr) -> u64 {
    let u = node_u::<node_back_ref_struct>(n);
    let mut h = hash_str("node_back_ref");
    h = hash_merge(h, hash_u64(u.kind as u64));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_back_ref(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_back_ref_struct>(n);
    let mut h = hash_str("node_back_ref");

    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_block(n: NodePtr) -> u64 {
    let u = node_u::<node_block_struct>(n);
    let mut h = hash_str("node_block");
    h = hash_merge(h, hash_node_list(u.stmts));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_block(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_block_struct>(n);
    let mut h = hash_str("node_block");
    for x in u.stmts.iter() { h = hash_merge(h, profile_node(pc, *x)); }
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_line(n: NodePtr) -> u64 {
    let u = node_u::<node_line_struct>(n);
    let mut h = hash_str("node_line");
    h = hash_merge(h, hash_node(u.body));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_line(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_line_struct>(n);
    let mut h = hash_str("node_line");
    h = hash_merge(h, profile_node(pc, u.body));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_trace(n: NodePtr) -> u64 {
    let u = node_u::<node_trace_struct>(n);
    let mut h = hash_str("node_trace");
    h = hash_merge(h, hash_u64(u.ev as u64));
    h = hash_merge(h, hash_node(u.body));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_trace(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_trace_struct>(n);
    let mut h = hash_str("node_trace");
    h = hash_merge(h, profile_node(pc, u.body));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_poll(n: NodePtr) -> u64 {
    let u = node_u::<node_poll_struct>(n);
    let mut h = hash_str("node_poll");
    h = hash_merge(h, hash_node(u.body));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_poll(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_poll_struct>(n);
    let mut h = hash_str("node_poll");
    h = hash_merge(h, profile_node(pc, u.body));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_cov(n: NodePtr) -> u64 {
    let u = node_u::<node_cov_struct>(n);
    let mut h = hash_str("node_cov");
    h = hash_merge(h, hash_u64(u.id as u64));
    h = hash_merge(h, hash_node(u.body));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_cov(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_cov_struct>(n);
    let mut h = hash_str("node_cov");
    h = hash_merge(h, profile_node(pc, u.body));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_if(n: NodePtr) -> u64 {
    let u = node_u::<node_if_struct>(n);
    let mut h = hash_str("node_if");
    h = hash_merge(h, hash_node(u.cond));
    h = hash_merge(h, hash_node(u.then_b));
    h = hash_merge(h, hash_node(u.else_b));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_if(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_if_struct>(n);
    let mut h = hash_str("node_if");
    h = hash_merge(h, profile_node(pc, u.cond));
    h = hash_merge(h, profile_node(pc, u.then_b));
    h = hash_merge(h, profile_node(pc, u.else_b));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_and(n: NodePtr) -> u64 {
    let u = node_u::<node_and_struct>(n);
    let mut h = hash_str("node_and");
    h = hash_merge(h, hash_node(u.a));
    h = hash_merge(h, hash_node(u.b));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_and(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_and_struct>(n);
    let mut h = hash_str("node_and");
    h = hash_merge(h, profile_node(pc, u.a));
    h = hash_merge(h, profile_node(pc, u.b));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_or(n: NodePtr) -> u64 {
    let u = node_u::<node_or_struct>(n);
    let mut h = hash_str("node_or");
    h = hash_merge(h, hash_node(u.a));
    h = hash_merge(h, hash_node(u.b));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_or(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_or_struct>(n);
    let mut h = hash_str("node_or");
    h = hash_merge(h, profile_node(pc, u.a));
    h = hash_merge(h, profile_node(pc, u.b));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_not(n: NodePtr) -> u64 {
    let u = node_u::<node_not_struct>(n);
    let mut h = hash_str("node_not");
    h = hash_merge(h, hash_node(u.v));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_not(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_not_struct>(n);
    let mut h = hash_str("node_not");
    h = hash_merge(h, profile_node(pc, u.v));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_while(n: NodePtr) -> u64 {
    let u = node_u::<node_while_struct>(n);
    let mut h = hash_str("node_while");
    h = hash_merge(h, hash_node(u.cond));
    h = hash_merge(h, hash_node(u.body));
    h = hash_merge(h, hash_u64(u.until as u64));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_while(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_while_struct>(n);
    let mut h = hash_str("node_while");
    h = hash_merge(h, profile_node(pc, u.cond));
    h = hash_merge(h, profile_node(pc, u.body));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_dowhile(n: NodePtr) -> u64 {
    let u = node_u::<node_dowhile_struct>(n);
    let mut h = hash_str("node_dowhile");
    h = hash_merge(h, hash_node(u.cond));
    h = hash_merge(h, hash_node(u.body));
    h = hash_merge(h, hash_u64(u.until as u64));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_dowhile(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_dowhile_struct>(n);
    let mut h = hash_str("node_dowhile");
    h = hash_merge(h, profile_node(pc, u.cond));
    h = hash_merge(h, profile_node(pc, u.body));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_loop_break(n: NodePtr) -> u64 {
    let u = node_u::<node_loop_break_struct>(n);
    let mut h = hash_str("node_loop_break");
    h = hash_merge(h, hash_node(u.v));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_loop_break(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_loop_break_struct>(n);
    let mut h = hash_str("node_loop_break");
    h = hash_merge(h, profile_node(pc, u.v));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_loop_next(n: NodePtr) -> u64 {
    let u = node_u::<node_loop_next_struct>(n);
    let mut h = hash_str("node_loop_next");
    h = hash_merge(h, hash_node(u.v));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_loop_next(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_loop_next_struct>(n);
    let mut h = hash_str("node_loop_next");
    h = hash_merge(h, profile_node(pc, u.v));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_break(n: NodePtr) -> u64 {
    let u = node_u::<node_break_struct>(n);
    let mut h = hash_str("node_break");
    h = hash_merge(h, hash_node(u.v));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_break(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_break_struct>(n);
    let mut h = hash_str("node_break");
    h = hash_merge(h, profile_node(pc, u.v));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_next(n: NodePtr) -> u64 {
    let u = node_u::<node_next_struct>(n);
    let mut h = hash_str("node_next");
    h = hash_merge(h, hash_node(u.v));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_next(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_next_struct>(n);
    let mut h = hash_str("node_next");
    h = hash_merge(h, profile_node(pc, u.v));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_return(n: NodePtr) -> u64 {
    let u = node_u::<node_return_struct>(n);
    let mut h = hash_str("node_return");
    h = hash_merge(h, hash_node(u.v));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_return(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_return_struct>(n);
    let mut h = hash_str("node_return");
    h = hash_merge(h, profile_node(pc, u.v));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_retry(n: NodePtr) -> u64 {
    let u = node_u::<node_retry_struct>(n);
    let mut h = hash_str("node_retry");

    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_retry(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_retry_struct>(n);
    let mut h = hash_str("node_retry");

    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_redo(n: NodePtr) -> u64 {
    let u = node_u::<node_redo_struct>(n);
    let mut h = hash_str("node_redo");

    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_redo(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_redo_struct>(n);
    let mut h = hash_str("node_redo");

    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_rescue(n: NodePtr) -> u64 {
    let u = node_u::<node_rescue_struct>(n);
    let mut h = hash_str("node_rescue");
    h = hash_merge(h, hash_node(u.body));
    h = hash_merge(h, hash_node_list(u.clauses));
    h = hash_merge(h, hash_node(u.else_b));
    h = hash_merge(h, hash_u64(u.has_else as u64));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_rescue(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_rescue_struct>(n);
    let mut h = hash_str("node_rescue");
    h = hash_merge(h, profile_node(pc, u.body));
    for x in u.clauses.iter() { h = hash_merge(h, profile_node(pc, *x)); }
    h = hash_merge(h, profile_node(pc, u.else_b));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_resbody(n: NodePtr) -> u64 {
    let u = node_u::<node_resbody_struct>(n);
    let mut h = hash_str("node_resbody");
    h = hash_merge(h, hash_node_list(u.classes));
    h = hash_merge(h, hash_bytes(u.kinds));
    h = hash_merge(h, hash_node(u.assign));
    h = hash_merge(h, hash_node(u.body));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_resbody(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_resbody_struct>(n);
    let mut h = hash_str("node_resbody");
    for x in u.classes.iter() { h = hash_merge(h, profile_node(pc, *x)); }
    h = hash_merge(h, profile_node(pc, u.assign));
    h = hash_merge(h, profile_node(pc, u.body));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_ensure(n: NodePtr) -> u64 {
    let u = node_u::<node_ensure_struct>(n);
    let mut h = hash_str("node_ensure");
    h = hash_merge(h, hash_node(u.body));
    h = hash_merge(h, hash_node(u.ens));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_ensure(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_ensure_struct>(n);
    let mut h = hash_str("node_ensure");
    h = hash_merge(h, profile_node(pc, u.body));
    h = hash_merge(h, profile_node(pc, u.ens));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_when(n: NodePtr) -> u64 {
    let u = node_u::<node_when_struct>(n);
    let mut h = hash_str("node_when");
    h = hash_merge(h, hash_u64(u.subj_slot as u64));
    h = hash_merge(h, hash_node_list(u.pats));
    h = hash_merge(h, hash_bytes(u.kinds));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_when(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_when_struct>(n);
    let mut h = hash_str("node_when");
    for x in u.pats.iter() { h = hash_merge(h, profile_node(pc, *x)); }
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_when_lit(n: NodePtr) -> u64 {
    let u = node_u::<node_when_lit_struct>(n);
    let mut h = hash_str("node_when_lit");
    h = hash_merge(h, hash_u64(u.subj_slot as u64));
    h = hash_merge(h, crate::compile::value_hash(u.v));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_when_lit(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_when_lit_struct>(n);
    let mut h = hash_str("node_when_lit");

    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_defined(n: NodePtr) -> u64 {
    let u = node_u::<node_defined_struct>(n);
    let mut h = hash_str("node_defined");
    h = hash_merge(h, hash_u64(u.kind as u64));
    h = hash_merge(h, hash_bytes(crate::symbol::sym_bytes(u.name)));
    h = hash_merge(h, hash_node(u.recv));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_defined(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_defined_struct>(n);
    let mut h = hash_str("node_defined");
    h = hash_merge(h, profile_node(pc, u.recv));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_call(n: NodePtr) -> u64 {
    let u = node_u::<node_call_struct>(n);
    let mut h = hash_str("node_call");
    h = hash_merge(h, hash_node(u.recv));
    h = hash_merge(h, hash_node_list(u.args));
    h = hash_merge(h, hash_bytes(crate::symbol::sym_bytes(u.mid)));
    h = hash_merge(h, hash_u64(u.flags as u64));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_call(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_call_struct>(n);
    let mut h = hash_str("node_call");
    h = hash_merge(h, profile_node(pc, u.recv));
    for x in u.args.iter() { h = hash_merge(h, profile_node(pc, *x)); }
    { let p: NodePtr = { crate::vm::spec_callee(&u.cc, u.args.len(), u.flags) }; h = hash_merge(h, if p.is_null() { 0 } else { hash_merge(hash_node(p), profile_node(pc, p)) }); }
    { let p: NodePtr = { crate::vm::spec_callee_rec(&u.cc, u.args.len(), u.flags) }; h = hash_merge(h, if p.is_null() { 0 } else { hash_merge(hash_node(p), profile_node(pc, p)) }); }
    h = hash_merge(h, hash_u64(({ crate::vm::site_kind(&u.cc, u.args.len(), u.flags) }) as u64));
    h = hash_merge(h, hash_u64(({ crate::vm::spec_shared(u.recv, u.args.len(), crate::vm::spec_callee(&u.cc, u.args.len(), u.flags)) }) as u64));
    h = hash_merge(h, hash_u64(({ crate::inline_builtin::site_builtin(u.args.len(), crate::vm::site_kind(&u.cc, u.args.len(), u.flags), u.mid) }) as u64));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_call_blk(n: NodePtr) -> u64 {
    let u = node_u::<node_call_blk_struct>(n);
    let mut h = hash_str("node_call_blk");
    h = hash_merge(h, hash_node(u.recv));
    h = hash_merge(h, hash_node_list(u.args));
    h = hash_merge(h, hash_bytes(crate::symbol::sym_bytes(u.mid)));
    h = hash_merge(h, hash_u64(u.flags as u64));
    h = hash_merge(h, crate::compile::iseq_hash(u.blk));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_call_blk(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_call_blk_struct>(n);
    let mut h = hash_str("node_call_blk");
    h = hash_merge(h, profile_node(pc, u.recv));
    for x in u.args.iter() { h = hash_merge(h, profile_node(pc, *x)); }
    { let p: NodePtr = { crate::vm::spec_callee(&u.cc, u.args.len(), u.flags) }; h = hash_merge(h, if p.is_null() { 0 } else { hash_merge(hash_node(p), profile_node(pc, p)) }); }
    { let p: NodePtr = { crate::vm::spec_callee_rec(&u.cc, u.args.len(), u.flags) }; h = hash_merge(h, if p.is_null() { 0 } else { hash_merge(hash_node(p), profile_node(pc, p)) }); }
    h = hash_merge(h, hash_u64(({ crate::vm::site_kind(&u.cc, u.args.len(), u.flags) }) as u64));
    { let p: NodePtr = { crate::inline_iter::spec_iter_block(&u.cc, u.args.len(), crate::vm::site_kind(&u.cc, u.args.len(), u.flags), u.blk) }; h = hash_merge(h, if p.is_null() { 0 } else { hash_merge(hash_node(p), profile_node(pc, p)) }); }
    h = hash_merge(h, hash_u64(({ crate::inline_iter::site_iter(&u.cc, u.args.len(), crate::vm::site_kind(&u.cc, u.args.len(), u.flags), u.blk) }) as u64));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_call_kw(n: NodePtr) -> u64 {
    let u = node_u::<node_call_kw_struct>(n);
    let mut h = hash_str("node_call_kw");
    h = hash_merge(h, hash_node(u.recv));
    h = hash_merge(h, hash_node_list(u.args));
    h = hash_merge(h, hash_bytes(crate::symbol::sym_bytes(u.mid)));
    h = hash_merge(h, hash_u64(u.flags as u64));
    h = hash_merge(h, crate::compile::ci_hash(u.ci));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_call_kw(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_call_kw_struct>(n);
    let mut h = hash_str("node_call_kw");
    h = hash_merge(h, profile_node(pc, u.recv));
    for x in u.args.iter() { h = hash_merge(h, profile_node(pc, *x)); }
    { let p: NodePtr = { crate::vm::spec_callee_kw(&u.cc, u.args.len(), u.ci) }; h = hash_merge(h, if p.is_null() { 0 } else { hash_merge(hash_node(p), profile_node(pc, p)) }); }
    { let p: NodePtr = { crate::vm::spec_callee_kw_rec(&u.cc, u.args.len(), u.ci) }; h = hash_merge(h, if p.is_null() { 0 } else { hash_merge(hash_node(p), profile_node(pc, p)) }); }
    h = hash_merge(h, hash_u64(({ crate::vm::site_kind_kw(&u.cc, u.args.len(), u.ci) }) as u64));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_call_x(n: NodePtr) -> u64 {
    let u = node_u::<node_call_x_struct>(n);
    let mut h = hash_str("node_call_x");
    h = hash_merge(h, hash_node(u.recv));
    h = hash_merge(h, hash_node_list(u.args));
    h = hash_merge(h, hash_bytes(crate::symbol::sym_bytes(u.mid)));
    h = hash_merge(h, hash_u64(u.flags as u64));
    h = hash_merge(h, crate::compile::ci_hash(u.ci));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_call_x(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_call_x_struct>(n);
    let mut h = hash_str("node_call_x");
    h = hash_merge(h, profile_node(pc, u.recv));
    for x in u.args.iter() { h = hash_merge(h, profile_node(pc, *x)); }
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_send_sym(n: NodePtr) -> u64 {
    let u = node_u::<node_send_sym_struct>(n);
    let mut h = hash_str("node_send_sym");
    h = hash_merge(h, hash_node(u.recv));
    h = hash_merge(h, hash_bytes(crate::symbol::sym_bytes(u.smid)));
    h = hash_merge(h, hash_node(u.call));
    h = hash_merge(h, hash_node(u.general));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_send_sym(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_send_sym_struct>(n);
    let mut h = hash_str("node_send_sym");
    h = hash_merge(h, profile_node(pc, u.recv));
    h = hash_merge(h, profile_node(pc, u.call));
    h = hash_merge(h, profile_node(pc, u.general));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_attrasgn(n: NodePtr) -> u64 {
    let u = node_u::<node_attrasgn_struct>(n);
    let mut h = hash_str("node_attrasgn");
    h = hash_merge(h, hash_node(u.recv));
    h = hash_merge(h, hash_node_list(u.args));
    h = hash_merge(h, hash_bytes(crate::symbol::sym_bytes(u.mid)));
    h = hash_merge(h, hash_u64(u.flags as u64));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_attrasgn(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_attrasgn_struct>(n);
    let mut h = hash_str("node_attrasgn");
    h = hash_merge(h, profile_node(pc, u.recv));
    for x in u.args.iter() { h = hash_merge(h, profile_node(pc, *x)); }
    h = hash_merge(h, hash_u64(({ crate::vm::site_kind(&u.cc, u.args.len(), u.flags) }) as u64));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_super(n: NodePtr) -> u64 {
    let u = node_u::<node_super_struct>(n);
    let mut h = hash_str("node_super");
    h = hash_merge(h, hash_node_list(u.args));
    h = hash_merge(h, crate::compile::ci_hash(u.ci));
    h = hash_merge(h, hash_u64(u.zsuper as u64));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_super(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_super_struct>(n);
    let mut h = hash_str("node_super");
    for x in u.args.iter() { h = hash_merge(h, profile_node(pc, *x)); }
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_yield(n: NodePtr) -> u64 {
    let u = node_u::<node_yield_struct>(n);
    let mut h = hash_str("node_yield");
    h = hash_merge(h, hash_node_list(u.args));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_yield(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_yield_struct>(n);
    let mut h = hash_str("node_yield");
    for x in u.args.iter() { h = hash_merge(h, profile_node(pc, *x)); }
    { let p: NodePtr = { crate::vm::spec_block(&u.yc, u.args.len()) }; h = hash_merge(h, if p.is_null() { 0 } else { hash_merge(hash_node(p), profile_node(pc, p)) }); }
    h = hash_merge(h, hash_u64(({ crate::vm::spec_block_np(&u.yc, u.args.len()) }) as u64));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_yield_x(n: NodePtr) -> u64 {
    let u = node_u::<node_yield_x_struct>(n);
    let mut h = hash_str("node_yield_x");
    h = hash_merge(h, hash_node_list(u.args));
    h = hash_merge(h, crate::compile::ci_hash(u.ci));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_yield_x(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_yield_x_struct>(n);
    let mut h = hash_str("node_yield_x");
    for x in u.args.iter() { h = hash_merge(h, profile_node(pc, *x)); }
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_block_given(n: NodePtr) -> u64 {
    let u = node_u::<node_block_given_struct>(n);
    let mut h = hash_str("node_block_given");

    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_block_given(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_block_given_struct>(n);
    let mut h = hash_str("node_block_given");

    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_lambda(n: NodePtr) -> u64 {
    let u = node_u::<node_lambda_struct>(n);
    let mut h = hash_str("node_lambda");
    h = hash_merge(h, crate::compile::iseq_hash(u.blk));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_lambda(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_lambda_struct>(n);
    let mut h = hash_str("node_lambda");

    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_opt_plus(n: NodePtr) -> u64 {
    let u = node_u::<node_opt_plus_struct>(n);
    let mut h = hash_str("node_opt_plus");
    h = hash_merge(h, hash_node(u.a));
    h = hash_merge(h, hash_node(u.b));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_opt_plus(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_opt_plus_struct>(n);
    let mut h = hash_str("node_opt_plus");
    h = hash_merge(h, profile_node(pc, u.a));
    h = hash_merge(h, profile_node(pc, u.b));
    h = hash_merge(h, hash_u64(({ 1 + u.tf.0.get() }) as u64));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_opt_minus(n: NodePtr) -> u64 {
    let u = node_u::<node_opt_minus_struct>(n);
    let mut h = hash_str("node_opt_minus");
    h = hash_merge(h, hash_node(u.a));
    h = hash_merge(h, hash_node(u.b));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_opt_minus(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_opt_minus_struct>(n);
    let mut h = hash_str("node_opt_minus");
    h = hash_merge(h, profile_node(pc, u.a));
    h = hash_merge(h, profile_node(pc, u.b));
    h = hash_merge(h, hash_u64(({ 1 + u.tf.0.get() }) as u64));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_opt_mul(n: NodePtr) -> u64 {
    let u = node_u::<node_opt_mul_struct>(n);
    let mut h = hash_str("node_opt_mul");
    h = hash_merge(h, hash_node(u.a));
    h = hash_merge(h, hash_node(u.b));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_opt_mul(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_opt_mul_struct>(n);
    let mut h = hash_str("node_opt_mul");
    h = hash_merge(h, profile_node(pc, u.a));
    h = hash_merge(h, profile_node(pc, u.b));
    h = hash_merge(h, hash_u64(({ 1 + u.tf.0.get() }) as u64));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_opt_div(n: NodePtr) -> u64 {
    let u = node_u::<node_opt_div_struct>(n);
    let mut h = hash_str("node_opt_div");
    h = hash_merge(h, hash_node(u.a));
    h = hash_merge(h, hash_node(u.b));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_opt_div(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_opt_div_struct>(n);
    let mut h = hash_str("node_opt_div");
    h = hash_merge(h, profile_node(pc, u.a));
    h = hash_merge(h, profile_node(pc, u.b));
    h = hash_merge(h, hash_u64(({ 1 + u.tf.0.get() }) as u64));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_opt_pow(n: NodePtr) -> u64 {
    let u = node_u::<node_opt_pow_struct>(n);
    let mut h = hash_str("node_opt_pow");
    h = hash_merge(h, hash_node(u.a));
    h = hash_merge(h, hash_node(u.b));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_opt_pow(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_opt_pow_struct>(n);
    let mut h = hash_str("node_opt_pow");
    h = hash_merge(h, profile_node(pc, u.a));
    h = hash_merge(h, profile_node(pc, u.b));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_opt_mod(n: NodePtr) -> u64 {
    let u = node_u::<node_opt_mod_struct>(n);
    let mut h = hash_str("node_opt_mod");
    h = hash_merge(h, hash_node(u.a));
    h = hash_merge(h, hash_node(u.b));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_opt_mod(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_opt_mod_struct>(n);
    let mut h = hash_str("node_opt_mod");
    h = hash_merge(h, profile_node(pc, u.a));
    h = hash_merge(h, profile_node(pc, u.b));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_opt_lt(n: NodePtr) -> u64 {
    let u = node_u::<node_opt_lt_struct>(n);
    let mut h = hash_str("node_opt_lt");
    h = hash_merge(h, hash_node(u.a));
    h = hash_merge(h, hash_node(u.b));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_opt_lt(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_opt_lt_struct>(n);
    let mut h = hash_str("node_opt_lt");
    h = hash_merge(h, profile_node(pc, u.a));
    h = hash_merge(h, profile_node(pc, u.b));
    h = hash_merge(h, hash_u64(({ 1 + u.tf.0.get() }) as u64));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_opt_le(n: NodePtr) -> u64 {
    let u = node_u::<node_opt_le_struct>(n);
    let mut h = hash_str("node_opt_le");
    h = hash_merge(h, hash_node(u.a));
    h = hash_merge(h, hash_node(u.b));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_opt_le(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_opt_le_struct>(n);
    let mut h = hash_str("node_opt_le");
    h = hash_merge(h, profile_node(pc, u.a));
    h = hash_merge(h, profile_node(pc, u.b));
    h = hash_merge(h, hash_u64(({ 1 + u.tf.0.get() }) as u64));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_opt_gt(n: NodePtr) -> u64 {
    let u = node_u::<node_opt_gt_struct>(n);
    let mut h = hash_str("node_opt_gt");
    h = hash_merge(h, hash_node(u.a));
    h = hash_merge(h, hash_node(u.b));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_opt_gt(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_opt_gt_struct>(n);
    let mut h = hash_str("node_opt_gt");
    h = hash_merge(h, profile_node(pc, u.a));
    h = hash_merge(h, profile_node(pc, u.b));
    h = hash_merge(h, hash_u64(({ 1 + u.tf.0.get() }) as u64));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_opt_ge(n: NodePtr) -> u64 {
    let u = node_u::<node_opt_ge_struct>(n);
    let mut h = hash_str("node_opt_ge");
    h = hash_merge(h, hash_node(u.a));
    h = hash_merge(h, hash_node(u.b));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_opt_ge(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_opt_ge_struct>(n);
    let mut h = hash_str("node_opt_ge");
    h = hash_merge(h, profile_node(pc, u.a));
    h = hash_merge(h, profile_node(pc, u.b));
    h = hash_merge(h, hash_u64(({ 1 + u.tf.0.get() }) as u64));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_opt_eq(n: NodePtr) -> u64 {
    let u = node_u::<node_opt_eq_struct>(n);
    let mut h = hash_str("node_opt_eq");
    h = hash_merge(h, hash_node(u.a));
    h = hash_merge(h, hash_node(u.b));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_opt_eq(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_opt_eq_struct>(n);
    let mut h = hash_str("node_opt_eq");
    h = hash_merge(h, profile_node(pc, u.a));
    h = hash_merge(h, profile_node(pc, u.b));
    h = hash_merge(h, hash_u64(({ 1 + u.tf.0.get() }) as u64));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_opt_neq(n: NodePtr) -> u64 {
    let u = node_u::<node_opt_neq_struct>(n);
    let mut h = hash_str("node_opt_neq");
    h = hash_merge(h, hash_node(u.a));
    h = hash_merge(h, hash_node(u.b));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_opt_neq(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_opt_neq_struct>(n);
    let mut h = hash_str("node_opt_neq");
    h = hash_merge(h, profile_node(pc, u.a));
    h = hash_merge(h, profile_node(pc, u.b));
    h = hash_merge(h, hash_u64(({ 1 + u.tf.0.get() }) as u64));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_opt_ltlt(n: NodePtr) -> u64 {
    let u = node_u::<node_opt_ltlt_struct>(n);
    let mut h = hash_str("node_opt_ltlt");
    h = hash_merge(h, hash_node(u.a));
    h = hash_merge(h, hash_node(u.b));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_opt_ltlt(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_opt_ltlt_struct>(n);
    let mut h = hash_str("node_opt_ltlt");
    h = hash_merge(h, profile_node(pc, u.a));
    h = hash_merge(h, profile_node(pc, u.b));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_opt_gtgt(n: NodePtr) -> u64 {
    let u = node_u::<node_opt_gtgt_struct>(n);
    let mut h = hash_str("node_opt_gtgt");
    h = hash_merge(h, hash_node(u.a));
    h = hash_merge(h, hash_node(u.b));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_opt_gtgt(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_opt_gtgt_struct>(n);
    let mut h = hash_str("node_opt_gtgt");
    h = hash_merge(h, profile_node(pc, u.a));
    h = hash_merge(h, profile_node(pc, u.b));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_opt_xor(n: NodePtr) -> u64 {
    let u = node_u::<node_opt_xor_struct>(n);
    let mut h = hash_str("node_opt_xor");
    h = hash_merge(h, hash_node(u.a));
    h = hash_merge(h, hash_node(u.b));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_opt_xor(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_opt_xor_struct>(n);
    let mut h = hash_str("node_opt_xor");
    h = hash_merge(h, profile_node(pc, u.a));
    h = hash_merge(h, profile_node(pc, u.b));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_opt_and(n: NodePtr) -> u64 {
    let u = node_u::<node_opt_and_struct>(n);
    let mut h = hash_str("node_opt_and");
    h = hash_merge(h, hash_node(u.a));
    h = hash_merge(h, hash_node(u.b));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_opt_and(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_opt_and_struct>(n);
    let mut h = hash_str("node_opt_and");
    h = hash_merge(h, profile_node(pc, u.a));
    h = hash_merge(h, profile_node(pc, u.b));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_opt_or(n: NodePtr) -> u64 {
    let u = node_u::<node_opt_or_struct>(n);
    let mut h = hash_str("node_opt_or");
    h = hash_merge(h, hash_node(u.a));
    h = hash_merge(h, hash_node(u.b));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_opt_or(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_opt_or_struct>(n);
    let mut h = hash_str("node_opt_or");
    h = hash_merge(h, profile_node(pc, u.a));
    h = hash_merge(h, profile_node(pc, u.b));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_opt_aref(n: NodePtr) -> u64 {
    let u = node_u::<node_opt_aref_struct>(n);
    let mut h = hash_str("node_opt_aref");
    h = hash_merge(h, hash_node(u.a));
    h = hash_merge(h, hash_node(u.b));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_opt_aref(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_opt_aref_struct>(n);
    let mut h = hash_str("node_opt_aref");
    h = hash_merge(h, profile_node(pc, u.a));
    h = hash_merge(h, profile_node(pc, u.b));
    h = hash_merge(h, hash_u64(({ 1 + u.tf.0.get() }) as u64));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_opt_aset(n: NodePtr) -> u64 {
    let u = node_u::<node_opt_aset_struct>(n);
    let mut h = hash_str("node_opt_aset");
    h = hash_merge(h, hash_node(u.a));
    h = hash_merge(h, hash_node(u.b));
    h = hash_merge(h, hash_node(u.v));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_opt_aset(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_opt_aset_struct>(n);
    let mut h = hash_str("node_opt_aset");
    h = hash_merge(h, profile_node(pc, u.a));
    h = hash_merge(h, profile_node(pc, u.b));
    h = hash_merge(h, profile_node(pc, u.v));
    h = hash_merge(h, hash_u64(({ 1 + u.tf.0.get() }) as u64));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_method_entry(n: NodePtr) -> u64 {
    let u = node_u::<node_method_entry_struct>(n);
    let mut h = hash_str("node_method_entry");
    h = hash_merge(h, hash_u64(u.nlocals as u64));
    h = hash_merge(h, hash_u64(u.nparams as u64));
    h = hash_merge(h, hash_u64(u.flags as u64));
    h = hash_merge(h, hash_node(u.body));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_method_entry(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_method_entry_struct>(n);
    let mut h = hash_str("node_method_entry");
    h = hash_merge(h, profile_node(pc, u.body));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_block_entry(n: NodePtr) -> u64 {
    let u = node_u::<node_block_entry_struct>(n);
    let mut h = hash_str("node_block_entry");
    h = hash_merge(h, hash_u64(u.nlocals as u64));
    h = hash_merge(h, hash_u64(u.nparams as u64));
    h = hash_merge(h, hash_u64(u.flags as u64));
    h = hash_merge(h, hash_node(u.body));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_block_entry(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_block_entry_struct>(n);
    let mut h = hash_str("node_block_entry");
    h = hash_merge(h, profile_node(pc, u.body));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_def(n: NodePtr) -> u64 {
    let u = node_u::<node_def_struct>(n);
    let mut h = hash_str("node_def");
    h = hash_merge(h, hash_bytes(crate::symbol::sym_bytes(u.mid)));
    h = hash_merge(h, crate::compile::iseq_hash(u.body));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_def(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_def_struct>(n);
    let mut h = hash_str("node_def");

    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_sdef(n: NodePtr) -> u64 {
    let u = node_u::<node_sdef_struct>(n);
    let mut h = hash_str("node_sdef");
    h = hash_merge(h, hash_node(u.recv));
    h = hash_merge(h, hash_bytes(crate::symbol::sym_bytes(u.mid)));
    h = hash_merge(h, crate::compile::iseq_hash(u.body));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_sdef(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_sdef_struct>(n);
    let mut h = hash_str("node_sdef");
    h = hash_merge(h, profile_node(pc, u.recv));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_class(n: NodePtr) -> u64 {
    let u = node_u::<node_class_struct>(n);
    let mut h = hash_str("node_class");
    h = hash_merge(h, hash_node(u.cpath));
    h = hash_merge(h, hash_bytes(crate::symbol::sym_bytes(u.name)));
    h = hash_merge(h, hash_node(u.superclass));
    h = hash_merge(h, crate::compile::iseq_hash(u.body));
    h = hash_merge(h, hash_u64(u.has_cpath as u64));
    h = hash_merge(h, hash_u64(u.has_super as u64));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_class(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_class_struct>(n);
    let mut h = hash_str("node_class");
    h = hash_merge(h, profile_node(pc, u.cpath));
    h = hash_merge(h, profile_node(pc, u.superclass));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_module(n: NodePtr) -> u64 {
    let u = node_u::<node_module_struct>(n);
    let mut h = hash_str("node_module");
    h = hash_merge(h, hash_node(u.cpath));
    h = hash_merge(h, hash_bytes(crate::symbol::sym_bytes(u.name)));
    h = hash_merge(h, crate::compile::iseq_hash(u.body));
    h = hash_merge(h, hash_u64(u.has_cpath as u64));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_module(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_module_struct>(n);
    let mut h = hash_str("node_module");
    h = hash_merge(h, profile_node(pc, u.cpath));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_sclass(n: NodePtr) -> u64 {
    let u = node_u::<node_sclass_struct>(n);
    let mut h = hash_str("node_sclass");
    h = hash_merge(h, hash_node(u.recv));
    h = hash_merge(h, crate::compile::iseq_hash(u.body));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_sclass(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_sclass_struct>(n);
    let mut h = hash_str("node_sclass");
    h = hash_merge(h, profile_node(pc, u.recv));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_alias(n: NodePtr) -> u64 {
    let u = node_u::<node_alias_struct>(n);
    let mut h = hash_str("node_alias");
    h = hash_merge(h, hash_node(u.newname));
    h = hash_merge(h, hash_node(u.oldname));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_alias(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_alias_struct>(n);
    let mut h = hash_str("node_alias");
    h = hash_merge(h, profile_node(pc, u.newname));
    h = hash_merge(h, profile_node(pc, u.oldname));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_valias(n: NodePtr) -> u64 {
    let u = node_u::<node_valias_struct>(n);
    let mut h = hash_str("node_valias");
    h = hash_merge(h, hash_bytes(crate::symbol::sym_bytes(u.newname)));
    h = hash_merge(h, hash_bytes(crate::symbol::sym_bytes(u.oldname)));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_valias(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_valias_struct>(n);
    let mut h = hash_str("node_valias");

    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_undef(n: NodePtr) -> u64 {
    let u = node_u::<node_undef_struct>(n);
    let mut h = hash_str("node_undef");
    h = hash_merge(h, hash_node_list(u.names));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_undef(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_undef_struct>(n);
    let mut h = hash_str("node_undef");
    for x in u.names.iter() { h = hash_merge(h, profile_node(pc, *x)); }
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_masgn(n: NodePtr) -> u64 {
    let u = node_u::<node_masgn_struct>(n);
    let mut h = hash_str("node_masgn");
    h = hash_merge(h, hash_node(u.rhs));
    h = hash_merge(h, hash_u64(u.tmp as u64));
    h = hash_merge(h, hash_node_list(u.targets));
    h = hash_merge(h, hash_u64(u.splat_rhs as u64));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_masgn(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_masgn_struct>(n);
    let mut h = hash_str("node_masgn");
    h = hash_merge(h, profile_node(pc, u.rhs));
    for x in u.targets.iter() { h = hash_merge(h, profile_node(pc, *x)); }
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_masgn_elem(n: NodePtr) -> u64 {
    let u = node_u::<node_masgn_elem_struct>(n);
    let mut h = hash_str("node_masgn_elem");
    h = hash_merge(h, hash_u64(u.tmp as u64));
    h = hash_merge(h, hash_u64(u.idx as u64));
    h = hash_merge(h, hash_u64(u.kind as u64));
    h = hash_merge(h, hash_u64(u.post as u64));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_masgn_elem(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_masgn_elem_struct>(n);
    let mut h = hash_str("node_masgn_elem");

    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_unsupported(n: NodePtr) -> u64 {
    let u = node_u::<node_unsupported_struct>(n);
    let mut h = hash_str("node_unsupported");
    h = hash_merge(h, hash_bytes(u.what));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_unsupported(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_unsupported_struct>(n);
    let mut h = hash_str("node_unsupported");

    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_special(n: NodePtr) -> u64 {
    let u = node_u::<node_special_struct>(n);
    let mut h = hash_str("node_special");
    h = hash_merge(h, hash_u64(u.kind as u64));
    h = hash_merge(h, hash_node(u.arg));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_special(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_special_struct>(n);
    let mut h = hash_str("node_special");
    h = hash_merge(h, profile_node(pc, u.arg));
    h
}


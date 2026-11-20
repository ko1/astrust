// This file is auto-generated from node.def by astrustgen. Do not edit.
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_nil(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_nil_struct>)).u };
    let mut h = hash_str("node_nil");

    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_true(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_true_struct>)).u };
    let mut h = hash_str("node_true");

    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_false(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_false_struct>)).u };
    let mut h = hash_str("node_false");

    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_self(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_self_struct>)).u };
    let mut h = hash_str("node_self");

    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_lit(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_lit_struct>)).u };
    let mut h = hash_str("node_lit");
    h = hash_merge(h, crate::compile::value_hash(u.v));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_float(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_float_struct>)).u };
    let mut h = hash_str("node_float");
    h = hash_merge(h, hash_u64(u.v.to_bits()));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_str(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_str_struct>)).u };
    let mut h = hash_str("node_str");
    h = hash_merge(h, hash_bytes(u.s));
    h = hash_merge(h, hash_u64(u.enc as u64));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_fstr(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_fstr_struct>)).u };
    let mut h = hash_str("node_fstr");
    h = hash_merge(h, hash_bytes(u.s));
    h = hash_merge(h, hash_u64(u.enc as u64));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_dstr(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_dstr_struct>)).u };
    let mut h = hash_str("node_dstr");
    h = hash_merge(h, hash_node_list(u.parts));
    h = hash_merge(h, hash_u64(u.enc as u64));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_tostr(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_tostr_struct>)).u };
    let mut h = hash_str("node_tostr");
    h = hash_merge(h, hash_node(u.v));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_dsym(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_dsym_struct>)).u };
    let mut h = hash_str("node_dsym");
    h = hash_merge(h, hash_node(u.s));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_regex(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_regex_struct>)).u };
    let mut h = hash_str("node_regex");
    h = hash_merge(h, hash_bytes(u.src));
    h = hash_merge(h, hash_u64(u.opts as u64));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_dregex(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_dregex_struct>)).u };
    let mut h = hash_str("node_dregex");
    h = hash_merge(h, hash_node(u.s));
    h = hash_merge(h, hash_u64(u.opts as u64));
    h = hash_merge(h, hash_u64(u.once as u64));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_biglit(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_biglit_struct>)).u };
    let mut h = hash_str("node_biglit");
    h = hash_merge(h, hash_bytes(u.digits));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_ary(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_ary_struct>)).u };
    let mut h = hash_str("node_ary");
    h = hash_merge(h, hash_node_list(u.elems));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_ary_build(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_ary_build_struct>)).u };
    let mut h = hash_str("node_ary_build");
    h = hash_merge(h, hash_node_list(u.elems));
    h = hash_merge(h, hash_bytes(u.kinds));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_hash(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_hash_struct>)).u };
    let mut h = hash_str("node_hash");
    h = hash_merge(h, hash_node_list(u.items));
    h = hash_merge(h, hash_bytes(u.kinds));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_range(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_range_struct>)).u };
    let mut h = hash_str("node_range");
    h = hash_merge(h, hash_node(u.b));
    h = hash_merge(h, hash_node(u.e));
    h = hash_merge(h, hash_u64(u.excl as u64));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_range_lit(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_range_lit_struct>)).u };
    let mut h = hash_str("node_range_lit");
    h = hash_merge(h, crate::compile::value_hash(u.b));
    h = hash_merge(h, crate::compile::value_hash(u.e));
    h = hash_merge(h, hash_u64(u.excl as u64));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_lvar(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_lvar_struct>)).u };
    let mut h = hash_str("node_lvar");
    h = hash_merge(h, hash_u64(u.idx as u64));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_lasgn(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_lasgn_struct>)).u };
    let mut h = hash_str("node_lasgn");
    h = hash_merge(h, hash_u64(u.idx as u64));
    h = hash_merge(h, hash_node(u.v));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_dvar(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_dvar_struct>)).u };
    let mut h = hash_str("node_dvar");
    h = hash_merge(h, hash_u64(u.hops as u64));
    h = hash_merge(h, hash_u64(u.idx as u64));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_dasgn(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_dasgn_struct>)).u };
    let mut h = hash_str("node_dasgn");
    h = hash_merge(h, hash_u64(u.hops as u64));
    h = hash_merge(h, hash_u64(u.idx as u64));
    h = hash_merge(h, hash_node(u.v));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_ivar(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_ivar_struct>)).u };
    let mut h = hash_str("node_ivar");
    h = hash_merge(h, hash_bytes(crate::symbol::sym_bytes(u.name)));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_iasgn(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_iasgn_struct>)).u };
    let mut h = hash_str("node_iasgn");
    h = hash_merge(h, hash_bytes(crate::symbol::sym_bytes(u.name)));
    h = hash_merge(h, hash_node(u.v));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_gvar(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_gvar_struct>)).u };
    let mut h = hash_str("node_gvar");
    h = hash_merge(h, hash_bytes(crate::symbol::sym_bytes(u.name)));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_gasgn(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_gasgn_struct>)).u };
    let mut h = hash_str("node_gasgn");
    h = hash_merge(h, hash_bytes(crate::symbol::sym_bytes(u.name)));
    h = hash_merge(h, hash_node(u.v));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_cvar(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_cvar_struct>)).u };
    let mut h = hash_str("node_cvar");
    h = hash_merge(h, hash_bytes(crate::symbol::sym_bytes(u.name)));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_cvasgn(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_cvasgn_struct>)).u };
    let mut h = hash_str("node_cvasgn");
    h = hash_merge(h, hash_bytes(crate::symbol::sym_bytes(u.name)));
    h = hash_merge(h, hash_node(u.v));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_const(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_const_struct>)).u };
    let mut h = hash_str("node_const");
    h = hash_merge(h, hash_bytes(crate::symbol::sym_bytes(u.name)));
    h = hash_merge(h, hash_u64(u.line as u64));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_const_path(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_const_path_struct>)).u };
    let mut h = hash_str("node_const_path");
    h = hash_merge(h, hash_node(u.parent));
    h = hash_merge(h, hash_bytes(crate::symbol::sym_bytes(u.name)));
    h = hash_merge(h, hash_u64(u.line as u64));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_top_const(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_top_const_struct>)).u };
    let mut h = hash_str("node_top_const");
    h = hash_merge(h, hash_bytes(crate::symbol::sym_bytes(u.name)));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_cdecl(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_cdecl_struct>)).u };
    let mut h = hash_str("node_cdecl");
    h = hash_merge(h, hash_bytes(crate::symbol::sym_bytes(u.name)));
    h = hash_merge(h, hash_node(u.v));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_cdecl_path(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_cdecl_path_struct>)).u };
    let mut h = hash_str("node_cdecl_path");
    h = hash_merge(h, hash_node(u.parent));
    h = hash_merge(h, hash_bytes(crate::symbol::sym_bytes(u.name)));
    h = hash_merge(h, hash_node(u.v));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_errinfo(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_errinfo_struct>)).u };
    let mut h = hash_str("node_errinfo");

    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_nth_ref(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_nth_ref_struct>)).u };
    let mut h = hash_str("node_nth_ref");
    h = hash_merge(h, hash_u64(u.nth as u64));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_back_ref(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_back_ref_struct>)).u };
    let mut h = hash_str("node_back_ref");
    h = hash_merge(h, hash_u64(u.kind as u64));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_block(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_block_struct>)).u };
    let mut h = hash_str("node_block");
    h = hash_merge(h, hash_node_list(u.stmts));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_line(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_line_struct>)).u };
    let mut h = hash_str("node_line");
    h = hash_merge(h, hash_u64(u.line as u64));
    h = hash_merge(h, hash_node(u.body));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_if(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_if_struct>)).u };
    let mut h = hash_str("node_if");
    h = hash_merge(h, hash_node(u.cond));
    h = hash_merge(h, hash_node(u.then_b));
    h = hash_merge(h, hash_node(u.else_b));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_and(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_and_struct>)).u };
    let mut h = hash_str("node_and");
    h = hash_merge(h, hash_node(u.a));
    h = hash_merge(h, hash_node(u.b));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_or(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_or_struct>)).u };
    let mut h = hash_str("node_or");
    h = hash_merge(h, hash_node(u.a));
    h = hash_merge(h, hash_node(u.b));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_not(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_not_struct>)).u };
    let mut h = hash_str("node_not");
    h = hash_merge(h, hash_node(u.v));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_while(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_while_struct>)).u };
    let mut h = hash_str("node_while");
    h = hash_merge(h, hash_node(u.cond));
    h = hash_merge(h, hash_node(u.body));
    h = hash_merge(h, hash_u64(u.until as u64));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_dowhile(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_dowhile_struct>)).u };
    let mut h = hash_str("node_dowhile");
    h = hash_merge(h, hash_node(u.cond));
    h = hash_merge(h, hash_node(u.body));
    h = hash_merge(h, hash_u64(u.until as u64));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_loop_break(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_loop_break_struct>)).u };
    let mut h = hash_str("node_loop_break");
    h = hash_merge(h, hash_node(u.v));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_loop_next(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_loop_next_struct>)).u };
    let mut h = hash_str("node_loop_next");
    h = hash_merge(h, hash_node(u.v));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_break(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_break_struct>)).u };
    let mut h = hash_str("node_break");
    h = hash_merge(h, hash_node(u.v));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_next(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_next_struct>)).u };
    let mut h = hash_str("node_next");
    h = hash_merge(h, hash_node(u.v));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_return(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_return_struct>)).u };
    let mut h = hash_str("node_return");
    h = hash_merge(h, hash_node(u.v));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_retry(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_retry_struct>)).u };
    let mut h = hash_str("node_retry");

    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_redo(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_redo_struct>)).u };
    let mut h = hash_str("node_redo");

    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_rescue(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_rescue_struct>)).u };
    let mut h = hash_str("node_rescue");
    h = hash_merge(h, hash_node(u.body));
    h = hash_merge(h, hash_node_list(u.clauses));
    h = hash_merge(h, hash_node(u.else_b));
    h = hash_merge(h, hash_u64(u.has_else as u64));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_resbody(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_resbody_struct>)).u };
    let mut h = hash_str("node_resbody");
    h = hash_merge(h, hash_node_list(u.classes));
    h = hash_merge(h, hash_bytes(u.kinds));
    h = hash_merge(h, hash_node(u.assign));
    h = hash_merge(h, hash_node(u.body));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_ensure(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_ensure_struct>)).u };
    let mut h = hash_str("node_ensure");
    h = hash_merge(h, hash_node(u.body));
    h = hash_merge(h, hash_node(u.ens));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_when(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_when_struct>)).u };
    let mut h = hash_str("node_when");
    h = hash_merge(h, hash_u64(u.subj_slot as u64));
    h = hash_merge(h, hash_node_list(u.pats));
    h = hash_merge(h, hash_bytes(u.kinds));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_defined(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_defined_struct>)).u };
    let mut h = hash_str("node_defined");
    h = hash_merge(h, hash_u64(u.kind as u64));
    h = hash_merge(h, hash_bytes(crate::symbol::sym_bytes(u.name)));
    h = hash_merge(h, hash_node(u.recv));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_call(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_call_struct>)).u };
    let mut h = hash_str("node_call");
    h = hash_merge(h, hash_node(u.recv));
    h = hash_merge(h, hash_node_list(u.args));
    h = hash_merge(h, hash_bytes(crate::symbol::sym_bytes(u.mid)));
    h = hash_merge(h, hash_u64(u.flags as u64));
    h = hash_merge(h, hash_u64(u.line as u64));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_call_blk(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_call_blk_struct>)).u };
    let mut h = hash_str("node_call_blk");
    h = hash_merge(h, hash_node(u.recv));
    h = hash_merge(h, hash_node_list(u.args));
    h = hash_merge(h, hash_bytes(crate::symbol::sym_bytes(u.mid)));
    h = hash_merge(h, hash_u64(u.flags as u64));
    h = hash_merge(h, hash_u64(u.line as u64));
    h = hash_merge(h, crate::compile::iseq_hash(u.blk));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_call_x(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_call_x_struct>)).u };
    let mut h = hash_str("node_call_x");
    h = hash_merge(h, hash_node(u.recv));
    h = hash_merge(h, hash_node_list(u.args));
    h = hash_merge(h, hash_bytes(crate::symbol::sym_bytes(u.mid)));
    h = hash_merge(h, hash_u64(u.flags as u64));
    h = hash_merge(h, hash_u64(u.line as u64));
    h = hash_merge(h, crate::compile::ci_hash(u.ci));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_attrasgn(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_attrasgn_struct>)).u };
    let mut h = hash_str("node_attrasgn");
    h = hash_merge(h, hash_node(u.recv));
    h = hash_merge(h, hash_node_list(u.args));
    h = hash_merge(h, hash_bytes(crate::symbol::sym_bytes(u.mid)));
    h = hash_merge(h, hash_u64(u.flags as u64));
    h = hash_merge(h, hash_u64(u.line as u64));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_super(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_super_struct>)).u };
    let mut h = hash_str("node_super");
    h = hash_merge(h, hash_node_list(u.args));
    h = hash_merge(h, hash_u64(u.line as u64));
    h = hash_merge(h, crate::compile::ci_hash(u.ci));
    h = hash_merge(h, hash_u64(u.zsuper as u64));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_yield(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_yield_struct>)).u };
    let mut h = hash_str("node_yield");
    h = hash_merge(h, hash_node_list(u.args));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_yield_x(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_yield_x_struct>)).u };
    let mut h = hash_str("node_yield_x");
    h = hash_merge(h, hash_node_list(u.args));
    h = hash_merge(h, crate::compile::ci_hash(u.ci));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_block_given(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_block_given_struct>)).u };
    let mut h = hash_str("node_block_given");

    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_lambda(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_lambda_struct>)).u };
    let mut h = hash_str("node_lambda");
    h = hash_merge(h, crate::compile::iseq_hash(u.blk));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_opt_plus(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_opt_plus_struct>)).u };
    let mut h = hash_str("node_opt_plus");
    h = hash_merge(h, hash_node(u.a));
    h = hash_merge(h, hash_node(u.b));
    h = hash_merge(h, hash_u64(u.line as u64));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_opt_minus(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_opt_minus_struct>)).u };
    let mut h = hash_str("node_opt_minus");
    h = hash_merge(h, hash_node(u.a));
    h = hash_merge(h, hash_node(u.b));
    h = hash_merge(h, hash_u64(u.line as u64));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_opt_mul(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_opt_mul_struct>)).u };
    let mut h = hash_str("node_opt_mul");
    h = hash_merge(h, hash_node(u.a));
    h = hash_merge(h, hash_node(u.b));
    h = hash_merge(h, hash_u64(u.line as u64));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_opt_div(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_opt_div_struct>)).u };
    let mut h = hash_str("node_opt_div");
    h = hash_merge(h, hash_node(u.a));
    h = hash_merge(h, hash_node(u.b));
    h = hash_merge(h, hash_u64(u.line as u64));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_opt_mod(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_opt_mod_struct>)).u };
    let mut h = hash_str("node_opt_mod");
    h = hash_merge(h, hash_node(u.a));
    h = hash_merge(h, hash_node(u.b));
    h = hash_merge(h, hash_u64(u.line as u64));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_opt_lt(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_opt_lt_struct>)).u };
    let mut h = hash_str("node_opt_lt");
    h = hash_merge(h, hash_node(u.a));
    h = hash_merge(h, hash_node(u.b));
    h = hash_merge(h, hash_u64(u.line as u64));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_opt_le(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_opt_le_struct>)).u };
    let mut h = hash_str("node_opt_le");
    h = hash_merge(h, hash_node(u.a));
    h = hash_merge(h, hash_node(u.b));
    h = hash_merge(h, hash_u64(u.line as u64));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_opt_gt(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_opt_gt_struct>)).u };
    let mut h = hash_str("node_opt_gt");
    h = hash_merge(h, hash_node(u.a));
    h = hash_merge(h, hash_node(u.b));
    h = hash_merge(h, hash_u64(u.line as u64));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_opt_ge(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_opt_ge_struct>)).u };
    let mut h = hash_str("node_opt_ge");
    h = hash_merge(h, hash_node(u.a));
    h = hash_merge(h, hash_node(u.b));
    h = hash_merge(h, hash_u64(u.line as u64));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_opt_eq(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_opt_eq_struct>)).u };
    let mut h = hash_str("node_opt_eq");
    h = hash_merge(h, hash_node(u.a));
    h = hash_merge(h, hash_node(u.b));
    h = hash_merge(h, hash_u64(u.line as u64));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_opt_neq(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_opt_neq_struct>)).u };
    let mut h = hash_str("node_opt_neq");
    h = hash_merge(h, hash_node(u.a));
    h = hash_merge(h, hash_node(u.b));
    h = hash_merge(h, hash_u64(u.line as u64));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_opt_ltlt(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_opt_ltlt_struct>)).u };
    let mut h = hash_str("node_opt_ltlt");
    h = hash_merge(h, hash_node(u.a));
    h = hash_merge(h, hash_node(u.b));
    h = hash_merge(h, hash_u64(u.line as u64));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_opt_and(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_opt_and_struct>)).u };
    let mut h = hash_str("node_opt_and");
    h = hash_merge(h, hash_node(u.a));
    h = hash_merge(h, hash_node(u.b));
    h = hash_merge(h, hash_u64(u.line as u64));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_opt_or(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_opt_or_struct>)).u };
    let mut h = hash_str("node_opt_or");
    h = hash_merge(h, hash_node(u.a));
    h = hash_merge(h, hash_node(u.b));
    h = hash_merge(h, hash_u64(u.line as u64));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_opt_aref(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_opt_aref_struct>)).u };
    let mut h = hash_str("node_opt_aref");
    h = hash_merge(h, hash_node(u.a));
    h = hash_merge(h, hash_node(u.b));
    h = hash_merge(h, hash_u64(u.line as u64));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_opt_aset(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_opt_aset_struct>)).u };
    let mut h = hash_str("node_opt_aset");
    h = hash_merge(h, hash_node(u.a));
    h = hash_merge(h, hash_node(u.b));
    h = hash_merge(h, hash_node(u.v));
    h = hash_merge(h, hash_u64(u.line as u64));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_def(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_def_struct>)).u };
    let mut h = hash_str("node_def");
    h = hash_merge(h, hash_bytes(crate::symbol::sym_bytes(u.mid)));
    h = hash_merge(h, crate::compile::iseq_hash(u.body));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_sdef(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_sdef_struct>)).u };
    let mut h = hash_str("node_sdef");
    h = hash_merge(h, hash_node(u.recv));
    h = hash_merge(h, hash_bytes(crate::symbol::sym_bytes(u.mid)));
    h = hash_merge(h, crate::compile::iseq_hash(u.body));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_class(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_class_struct>)).u };
    let mut h = hash_str("node_class");
    h = hash_merge(h, hash_node(u.cpath));
    h = hash_merge(h, hash_bytes(crate::symbol::sym_bytes(u.name)));
    h = hash_merge(h, hash_node(u.superclass));
    h = hash_merge(h, crate::compile::iseq_hash(u.body));
    h = hash_merge(h, hash_u64(u.has_cpath as u64));
    h = hash_merge(h, hash_u64(u.has_super as u64));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_module(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_module_struct>)).u };
    let mut h = hash_str("node_module");
    h = hash_merge(h, hash_node(u.cpath));
    h = hash_merge(h, hash_bytes(crate::symbol::sym_bytes(u.name)));
    h = hash_merge(h, crate::compile::iseq_hash(u.body));
    h = hash_merge(h, hash_u64(u.has_cpath as u64));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_sclass(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_sclass_struct>)).u };
    let mut h = hash_str("node_sclass");
    h = hash_merge(h, hash_node(u.recv));
    h = hash_merge(h, crate::compile::iseq_hash(u.body));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_alias(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_alias_struct>)).u };
    let mut h = hash_str("node_alias");
    h = hash_merge(h, hash_node(u.newname));
    h = hash_merge(h, hash_node(u.oldname));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_valias(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_valias_struct>)).u };
    let mut h = hash_str("node_valias");
    h = hash_merge(h, hash_bytes(crate::symbol::sym_bytes(u.newname)));
    h = hash_merge(h, hash_bytes(crate::symbol::sym_bytes(u.oldname)));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_undef(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_undef_struct>)).u };
    let mut h = hash_str("node_undef");
    h = hash_merge(h, hash_node_list(u.names));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_masgn(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_masgn_struct>)).u };
    let mut h = hash_str("node_masgn");
    h = hash_merge(h, hash_node(u.rhs));
    h = hash_merge(h, hash_u64(u.tmp as u64));
    h = hash_merge(h, hash_node_list(u.targets));
    h = hash_merge(h, hash_u64(u.splat_rhs as u64));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_masgn_elem(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_masgn_elem_struct>)).u };
    let mut h = hash_str("node_masgn_elem");
    h = hash_merge(h, hash_u64(u.tmp as u64));
    h = hash_merge(h, hash_u64(u.idx as u64));
    h = hash_merge(h, hash_u64(u.kind as u64));
    h = hash_merge(h, hash_u64(u.post as u64));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_unsupported(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_unsupported_struct>)).u };
    let mut h = hash_str("node_unsupported");
    h = hash_merge(h, hash_bytes(u.what));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_special(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_special_struct>)).u };
    let mut h = hash_str("node_special");
    h = hash_merge(h, hash_u64(u.kind as u64));
    h = hash_merge(h, hash_node(u.arg));
    h
}

#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_flipflop(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_flipflop_struct>)).u };
    let mut h = hash_str("node_flipflop");
    h = hash_merge(h, hash_node(u.a));
    h = hash_merge(h, hash_node(u.b));
    h = hash_merge(h, hash_u64(u.excl as u64));
    h = hash_merge(h, hash_u64(u.slot as u64));
    h
}

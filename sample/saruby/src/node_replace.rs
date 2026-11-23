// This file is auto-generated from node.def by astrustgen. Do not edit.
#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_nil(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_nil_struct>(n);

}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_true(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_true_struct>(n);

}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_false(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_false_struct>(n);

}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_self(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_self_struct>(n);

}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_lit(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_lit_struct>(n);

}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_float(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_float_struct>(n);

}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_str(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_str_struct>(n);

}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_cstr(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_cstr_struct>(n);

}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_fstr(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_fstr_struct>(n);

}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_fstr_recv(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_fstr_recv_struct>(n);

}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_dstr(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_dstr_struct>(n);
    if u.parts.contains(&old) { u.parts = Box::leak(u.parts.iter().map(|&x| if x == old { new } else { x }).collect::<Vec<_>>().into_boxed_slice()); }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_tostr(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_tostr_struct>(n);
    if u.v == old { u.v = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_dsym(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_dsym_struct>(n);
    if u.s == old { u.s = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_regex(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_regex_struct>(n);

}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_dregex(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_dregex_struct>(n);
    if u.s == old { u.s = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_biglit(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_biglit_struct>(n);

}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_ary(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_ary_struct>(n);
    if u.elems.contains(&old) { u.elems = Box::leak(u.elems.iter().map(|&x| if x == old { new } else { x }).collect::<Vec<_>>().into_boxed_slice()); }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_ary_build(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_ary_build_struct>(n);
    if u.elems.contains(&old) { u.elems = Box::leak(u.elems.iter().map(|&x| if x == old { new } else { x }).collect::<Vec<_>>().into_boxed_slice()); }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_hash(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_hash_struct>(n);
    if u.items.contains(&old) { u.items = Box::leak(u.items.iter().map(|&x| if x == old { new } else { x }).collect::<Vec<_>>().into_boxed_slice()); }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_range(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_range_struct>(n);
    if u.b == old { u.b = new; }
    if u.e == old { u.e = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_range_lit(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_range_lit_struct>(n);

}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_lvar(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_lvar_struct>(n);

}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_lasgn(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_lasgn_struct>(n);
    if u.v == old { u.v = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_dvar(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_dvar_struct>(n);

}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_dasgn(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_dasgn_struct>(n);
    if u.v == old { u.v = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_ivar(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_ivar_struct>(n);

}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_iasgn(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_iasgn_struct>(n);
    if u.v == old { u.v = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_ivar_arith(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_ivar_arith_struct>(n);
    if u.v == old { u.v = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_gvar(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_gvar_struct>(n);

}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_gasgn(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_gasgn_struct>(n);
    if u.v == old { u.v = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_cvar(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_cvar_struct>(n);

}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_cvasgn(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_cvasgn_struct>(n);
    if u.v == old { u.v = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_const(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_const_struct>(n);

}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_const_path(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_const_path_struct>(n);
    if u.parent == old { u.parent = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_top_const(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_top_const_struct>(n);

}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_cdecl(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_cdecl_struct>(n);
    if u.v == old { u.v = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_cdecl_path(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_cdecl_path_struct>(n);
    if u.parent == old { u.parent = new; }
    if u.v == old { u.v = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_errinfo(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_errinfo_struct>(n);

}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_nth_ref(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_nth_ref_struct>(n);

}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_back_ref(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_back_ref_struct>(n);

}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_block(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_block_struct>(n);
    if u.stmts.contains(&old) { u.stmts = Box::leak(u.stmts.iter().map(|&x| if x == old { new } else { x }).collect::<Vec<_>>().into_boxed_slice()); }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_line(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_line_struct>(n);
    if u.body == old { u.body = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_trace(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_trace_struct>(n);
    if u.body == old { u.body = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_poll(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_poll_struct>(n);
    if u.body == old { u.body = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_cov(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_cov_struct>(n);
    if u.body == old { u.body = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_if(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_if_struct>(n);
    if u.cond == old { u.cond = new; }
    if u.then_b == old { u.then_b = new; }
    if u.else_b == old { u.else_b = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_and(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_and_struct>(n);
    if u.a == old { u.a = new; }
    if u.b == old { u.b = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_or(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_or_struct>(n);
    if u.a == old { u.a = new; }
    if u.b == old { u.b = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_newarray_minmax(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_newarray_minmax_struct>(n);
    if u.a == old { u.a = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_not(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_not_struct>(n);
    if u.v == old { u.v = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_while(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_while_struct>(n);
    if u.cond == old { u.cond = new; }
    if u.body == old { u.body = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_dowhile(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_dowhile_struct>(n);
    if u.cond == old { u.cond = new; }
    if u.body == old { u.body = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_loop_break(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_loop_break_struct>(n);
    if u.v == old { u.v = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_loop_next(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_loop_next_struct>(n);
    if u.v == old { u.v = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_break(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_break_struct>(n);
    if u.v == old { u.v = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_next(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_next_struct>(n);
    if u.v == old { u.v = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_return(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_return_struct>(n);
    if u.v == old { u.v = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_retry(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_retry_struct>(n);

}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_redo(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_redo_struct>(n);

}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_rescue(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_rescue_struct>(n);
    if u.body == old { u.body = new; }
    if u.clauses.contains(&old) { u.clauses = Box::leak(u.clauses.iter().map(|&x| if x == old { new } else { x }).collect::<Vec<_>>().into_boxed_slice()); }
    if u.else_b == old { u.else_b = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_resbody(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_resbody_struct>(n);
    if u.classes.contains(&old) { u.classes = Box::leak(u.classes.iter().map(|&x| if x == old { new } else { x }).collect::<Vec<_>>().into_boxed_slice()); }
    if u.assign == old { u.assign = new; }
    if u.body == old { u.body = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_ensure(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_ensure_struct>(n);
    if u.body == old { u.body = new; }
    if u.ens == old { u.ens = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_when(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_when_struct>(n);
    if u.pats.contains(&old) { u.pats = Box::leak(u.pats.iter().map(|&x| if x == old { new } else { x }).collect::<Vec<_>>().into_boxed_slice()); }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_when_lit(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_when_lit_struct>(n);

}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_defined(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_defined_struct>(n);
    if u.recv == old { u.recv = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_call(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_call_struct>(n);
    if u.recv == old { u.recv = new; }
    if u.args.contains(&old) { u.args = Box::leak(u.args.iter().map(|&x| if x == old { new } else { x }).collect::<Vec<_>>().into_boxed_slice()); }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_call_blk(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_call_blk_struct>(n);
    if u.recv == old { u.recv = new; }
    if u.args.contains(&old) { u.args = Box::leak(u.args.iter().map(|&x| if x == old { new } else { x }).collect::<Vec<_>>().into_boxed_slice()); }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_call_kw(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_call_kw_struct>(n);
    if u.recv == old { u.recv = new; }
    if u.args.contains(&old) { u.args = Box::leak(u.args.iter().map(|&x| if x == old { new } else { x }).collect::<Vec<_>>().into_boxed_slice()); }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_call_x(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_call_x_struct>(n);
    if u.recv == old { u.recv = new; }
    if u.args.contains(&old) { u.args = Box::leak(u.args.iter().map(|&x| if x == old { new } else { x }).collect::<Vec<_>>().into_boxed_slice()); }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_send_sym(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_send_sym_struct>(n);
    if u.recv == old { u.recv = new; }
    if u.call == old { u.call = new; }
    if u.general == old { u.general = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_attrasgn(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_attrasgn_struct>(n);
    if u.recv == old { u.recv = new; }
    if u.args.contains(&old) { u.args = Box::leak(u.args.iter().map(|&x| if x == old { new } else { x }).collect::<Vec<_>>().into_boxed_slice()); }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_super(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_super_struct>(n);
    if u.args.contains(&old) { u.args = Box::leak(u.args.iter().map(|&x| if x == old { new } else { x }).collect::<Vec<_>>().into_boxed_slice()); }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_yield(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_yield_struct>(n);
    if u.args.contains(&old) { u.args = Box::leak(u.args.iter().map(|&x| if x == old { new } else { x }).collect::<Vec<_>>().into_boxed_slice()); }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_yield_x(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_yield_x_struct>(n);
    if u.args.contains(&old) { u.args = Box::leak(u.args.iter().map(|&x| if x == old { new } else { x }).collect::<Vec<_>>().into_boxed_slice()); }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_block_given(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_block_given_struct>(n);

}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_lambda(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_lambda_struct>(n);

}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_opt_plus(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_opt_plus_struct>(n);
    if u.a == old { u.a = new; }
    if u.b == old { u.b = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_opt_minus(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_opt_minus_struct>(n);
    if u.a == old { u.a = new; }
    if u.b == old { u.b = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_opt_mul(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_opt_mul_struct>(n);
    if u.a == old { u.a = new; }
    if u.b == old { u.b = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_opt_div(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_opt_div_struct>(n);
    if u.a == old { u.a = new; }
    if u.b == old { u.b = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_opt_pow(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_opt_pow_struct>(n);
    if u.a == old { u.a = new; }
    if u.b == old { u.b = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_opt_mod(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_opt_mod_struct>(n);
    if u.a == old { u.a = new; }
    if u.b == old { u.b = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_opt_lt(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_opt_lt_struct>(n);
    if u.a == old { u.a = new; }
    if u.b == old { u.b = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_opt_le(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_opt_le_struct>(n);
    if u.a == old { u.a = new; }
    if u.b == old { u.b = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_opt_gt(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_opt_gt_struct>(n);
    if u.a == old { u.a = new; }
    if u.b == old { u.b = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_opt_ge(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_opt_ge_struct>(n);
    if u.a == old { u.a = new; }
    if u.b == old { u.b = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_opt_eq(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_opt_eq_struct>(n);
    if u.a == old { u.a = new; }
    if u.b == old { u.b = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_opt_neq(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_opt_neq_struct>(n);
    if u.a == old { u.a = new; }
    if u.b == old { u.b = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_opt_ltlt(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_opt_ltlt_struct>(n);
    if u.a == old { u.a = new; }
    if u.b == old { u.b = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_opt_gtgt(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_opt_gtgt_struct>(n);
    if u.a == old { u.a = new; }
    if u.b == old { u.b = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_opt_xor(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_opt_xor_struct>(n);
    if u.a == old { u.a = new; }
    if u.b == old { u.b = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_opt_and(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_opt_and_struct>(n);
    if u.a == old { u.a = new; }
    if u.b == old { u.b = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_opt_or(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_opt_or_struct>(n);
    if u.a == old { u.a = new; }
    if u.b == old { u.b = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_opt_aref(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_opt_aref_struct>(n);
    if u.a == old { u.a = new; }
    if u.b == old { u.b = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_opt_aset(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_opt_aset_struct>(n);
    if u.a == old { u.a = new; }
    if u.b == old { u.b = new; }
    if u.v == old { u.v = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_method_entry(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_method_entry_struct>(n);
    if u.body == old { u.body = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_block_entry(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_block_entry_struct>(n);
    if u.body == old { u.body = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_def(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_def_struct>(n);

}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_sdef(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_sdef_struct>(n);
    if u.recv == old { u.recv = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_class(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_class_struct>(n);
    if u.cpath == old { u.cpath = new; }
    if u.superclass == old { u.superclass = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_module(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_module_struct>(n);
    if u.cpath == old { u.cpath = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_sclass(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_sclass_struct>(n);
    if u.recv == old { u.recv = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_alias(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_alias_struct>(n);
    if u.newname == old { u.newname = new; }
    if u.oldname == old { u.oldname = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_valias(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_valias_struct>(n);

}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_undef(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_undef_struct>(n);
    if u.names.contains(&old) { u.names = Box::leak(u.names.iter().map(|&x| if x == old { new } else { x }).collect::<Vec<_>>().into_boxed_slice()); }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_masgn(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_masgn_struct>(n);
    if u.rhs == old { u.rhs = new; }
    if u.targets.contains(&old) { u.targets = Box::leak(u.targets.iter().map(|&x| if x == old { new } else { x }).collect::<Vec<_>>().into_boxed_slice()); }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_masgn_elem(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_masgn_elem_struct>(n);

}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_unsupported(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_unsupported_struct>(n);

}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_special(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_special_struct>(n);
    if u.arg == old { u.arg = new; }
}

// This file is auto-generated from node.def by astrustgen. Do not edit.
#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_nil(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_nil_struct>)).u };

}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_true(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_true_struct>)).u };

}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_false(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_false_struct>)).u };

}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_self(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_self_struct>)).u };

}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_lit(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_lit_struct>)).u };

}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_float(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_float_struct>)).u };

}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_str(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_str_struct>)).u };

}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_fstr(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_fstr_struct>)).u };

}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_dstr(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_dstr_struct>)).u };
    { let l = u.parts; let p = l.as_ptr() as *mut NodePtr; for i in 0..l.len() { unsafe { if *p.add(i) == old { *p.add(i) = new; } } } }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_tostr(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_tostr_struct>)).u };
    if u.v == old { u.v = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_dsym(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_dsym_struct>)).u };
    if u.s == old { u.s = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_regex(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_regex_struct>)).u };

}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_dregex(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_dregex_struct>)).u };
    if u.s == old { u.s = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_biglit(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_biglit_struct>)).u };

}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_ary(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_ary_struct>)).u };
    { let l = u.elems; let p = l.as_ptr() as *mut NodePtr; for i in 0..l.len() { unsafe { if *p.add(i) == old { *p.add(i) = new; } } } }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_ary_build(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_ary_build_struct>)).u };
    { let l = u.elems; let p = l.as_ptr() as *mut NodePtr; for i in 0..l.len() { unsafe { if *p.add(i) == old { *p.add(i) = new; } } } }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_hash(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_hash_struct>)).u };
    { let l = u.items; let p = l.as_ptr() as *mut NodePtr; for i in 0..l.len() { unsafe { if *p.add(i) == old { *p.add(i) = new; } } } }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_range(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_range_struct>)).u };
    if u.b == old { u.b = new; }
    if u.e == old { u.e = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_range_lit(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_range_lit_struct>)).u };

}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_lvar(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_lvar_struct>)).u };

}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_lasgn(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_lasgn_struct>)).u };
    if u.v == old { u.v = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_dvar(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_dvar_struct>)).u };

}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_dasgn(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_dasgn_struct>)).u };
    if u.v == old { u.v = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_ivar(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_ivar_struct>)).u };

}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_iasgn(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_iasgn_struct>)).u };
    if u.v == old { u.v = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_gvar(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_gvar_struct>)).u };

}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_gasgn(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_gasgn_struct>)).u };
    if u.v == old { u.v = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_cvar(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_cvar_struct>)).u };

}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_cvasgn(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_cvasgn_struct>)).u };
    if u.v == old { u.v = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_const(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_const_struct>)).u };

}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_const_path(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_const_path_struct>)).u };
    if u.parent == old { u.parent = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_top_const(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_top_const_struct>)).u };

}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_cdecl(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_cdecl_struct>)).u };
    if u.v == old { u.v = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_cdecl_path(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_cdecl_path_struct>)).u };
    if u.parent == old { u.parent = new; }
    if u.v == old { u.v = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_errinfo(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_errinfo_struct>)).u };

}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_nth_ref(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_nth_ref_struct>)).u };

}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_back_ref(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_back_ref_struct>)).u };

}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_block(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_block_struct>)).u };
    { let l = u.stmts; let p = l.as_ptr() as *mut NodePtr; for i in 0..l.len() { unsafe { if *p.add(i) == old { *p.add(i) = new; } } } }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_line(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_line_struct>)).u };
    if u.body == old { u.body = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_if(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_if_struct>)).u };
    if u.cond == old { u.cond = new; }
    if u.then_b == old { u.then_b = new; }
    if u.else_b == old { u.else_b = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_and(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_and_struct>)).u };
    if u.a == old { u.a = new; }
    if u.b == old { u.b = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_or(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_or_struct>)).u };
    if u.a == old { u.a = new; }
    if u.b == old { u.b = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_not(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_not_struct>)).u };
    if u.v == old { u.v = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_while(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_while_struct>)).u };
    if u.cond == old { u.cond = new; }
    if u.body == old { u.body = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_dowhile(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_dowhile_struct>)).u };
    if u.cond == old { u.cond = new; }
    if u.body == old { u.body = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_loop_break(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_loop_break_struct>)).u };
    if u.v == old { u.v = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_loop_next(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_loop_next_struct>)).u };
    if u.v == old { u.v = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_break(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_break_struct>)).u };
    if u.v == old { u.v = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_next(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_next_struct>)).u };
    if u.v == old { u.v = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_return(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_return_struct>)).u };
    if u.v == old { u.v = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_retry(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_retry_struct>)).u };

}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_redo(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_redo_struct>)).u };

}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_rescue(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_rescue_struct>)).u };
    if u.body == old { u.body = new; }
    { let l = u.clauses; let p = l.as_ptr() as *mut NodePtr; for i in 0..l.len() { unsafe { if *p.add(i) == old { *p.add(i) = new; } } } }
    if u.else_b == old { u.else_b = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_resbody(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_resbody_struct>)).u };
    { let l = u.classes; let p = l.as_ptr() as *mut NodePtr; for i in 0..l.len() { unsafe { if *p.add(i) == old { *p.add(i) = new; } } } }
    if u.assign == old { u.assign = new; }
    if u.body == old { u.body = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_ensure(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_ensure_struct>)).u };
    if u.body == old { u.body = new; }
    if u.ens == old { u.ens = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_when(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_when_struct>)).u };
    { let l = u.pats; let p = l.as_ptr() as *mut NodePtr; for i in 0..l.len() { unsafe { if *p.add(i) == old { *p.add(i) = new; } } } }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_when_lit(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_when_lit_struct>)).u };

}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_defined(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_defined_struct>)).u };
    if u.recv == old { u.recv = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_call(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_call_struct>)).u };
    if u.recv == old { u.recv = new; }
    { let l = u.args; let p = l.as_ptr() as *mut NodePtr; for i in 0..l.len() { unsafe { if *p.add(i) == old { *p.add(i) = new; } } } }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_call_blk(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_call_blk_struct>)).u };
    if u.recv == old { u.recv = new; }
    { let l = u.args; let p = l.as_ptr() as *mut NodePtr; for i in 0..l.len() { unsafe { if *p.add(i) == old { *p.add(i) = new; } } } }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_call_kw(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_call_kw_struct>)).u };
    if u.recv == old { u.recv = new; }
    { let l = u.args; let p = l.as_ptr() as *mut NodePtr; for i in 0..l.len() { unsafe { if *p.add(i) == old { *p.add(i) = new; } } } }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_call_x(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_call_x_struct>)).u };
    if u.recv == old { u.recv = new; }
    { let l = u.args; let p = l.as_ptr() as *mut NodePtr; for i in 0..l.len() { unsafe { if *p.add(i) == old { *p.add(i) = new; } } } }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_attrasgn(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_attrasgn_struct>)).u };
    if u.recv == old { u.recv = new; }
    { let l = u.args; let p = l.as_ptr() as *mut NodePtr; for i in 0..l.len() { unsafe { if *p.add(i) == old { *p.add(i) = new; } } } }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_super(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_super_struct>)).u };
    { let l = u.args; let p = l.as_ptr() as *mut NodePtr; for i in 0..l.len() { unsafe { if *p.add(i) == old { *p.add(i) = new; } } } }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_yield(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_yield_struct>)).u };
    { let l = u.args; let p = l.as_ptr() as *mut NodePtr; for i in 0..l.len() { unsafe { if *p.add(i) == old { *p.add(i) = new; } } } }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_yield_x(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_yield_x_struct>)).u };
    { let l = u.args; let p = l.as_ptr() as *mut NodePtr; for i in 0..l.len() { unsafe { if *p.add(i) == old { *p.add(i) = new; } } } }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_block_given(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_block_given_struct>)).u };

}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_lambda(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_lambda_struct>)).u };

}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_opt_plus(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_opt_plus_struct>)).u };
    if u.a == old { u.a = new; }
    if u.b == old { u.b = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_opt_minus(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_opt_minus_struct>)).u };
    if u.a == old { u.a = new; }
    if u.b == old { u.b = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_opt_mul(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_opt_mul_struct>)).u };
    if u.a == old { u.a = new; }
    if u.b == old { u.b = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_opt_div(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_opt_div_struct>)).u };
    if u.a == old { u.a = new; }
    if u.b == old { u.b = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_opt_pow(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_opt_pow_struct>)).u };
    if u.a == old { u.a = new; }
    if u.b == old { u.b = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_opt_mod(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_opt_mod_struct>)).u };
    if u.a == old { u.a = new; }
    if u.b == old { u.b = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_opt_lt(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_opt_lt_struct>)).u };
    if u.a == old { u.a = new; }
    if u.b == old { u.b = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_opt_le(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_opt_le_struct>)).u };
    if u.a == old { u.a = new; }
    if u.b == old { u.b = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_opt_gt(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_opt_gt_struct>)).u };
    if u.a == old { u.a = new; }
    if u.b == old { u.b = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_opt_ge(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_opt_ge_struct>)).u };
    if u.a == old { u.a = new; }
    if u.b == old { u.b = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_opt_eq(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_opt_eq_struct>)).u };
    if u.a == old { u.a = new; }
    if u.b == old { u.b = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_opt_neq(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_opt_neq_struct>)).u };
    if u.a == old { u.a = new; }
    if u.b == old { u.b = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_opt_ltlt(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_opt_ltlt_struct>)).u };
    if u.a == old { u.a = new; }
    if u.b == old { u.b = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_opt_gtgt(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_opt_gtgt_struct>)).u };
    if u.a == old { u.a = new; }
    if u.b == old { u.b = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_opt_xor(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_opt_xor_struct>)).u };
    if u.a == old { u.a = new; }
    if u.b == old { u.b = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_opt_and(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_opt_and_struct>)).u };
    if u.a == old { u.a = new; }
    if u.b == old { u.b = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_opt_or(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_opt_or_struct>)).u };
    if u.a == old { u.a = new; }
    if u.b == old { u.b = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_opt_aref(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_opt_aref_struct>)).u };
    if u.a == old { u.a = new; }
    if u.b == old { u.b = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_opt_aset(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_opt_aset_struct>)).u };
    if u.a == old { u.a = new; }
    if u.b == old { u.b = new; }
    if u.v == old { u.v = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_method_entry(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_method_entry_struct>)).u };
    if u.body == old { u.body = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_block_entry(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_block_entry_struct>)).u };
    if u.body == old { u.body = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_def(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_def_struct>)).u };

}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_sdef(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_sdef_struct>)).u };
    if u.recv == old { u.recv = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_class(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_class_struct>)).u };
    if u.cpath == old { u.cpath = new; }
    if u.superclass == old { u.superclass = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_module(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_module_struct>)).u };
    if u.cpath == old { u.cpath = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_sclass(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_sclass_struct>)).u };
    if u.recv == old { u.recv = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_alias(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_alias_struct>)).u };
    if u.newname == old { u.newname = new; }
    if u.oldname == old { u.oldname = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_valias(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_valias_struct>)).u };

}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_undef(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_undef_struct>)).u };
    { let l = u.names; let p = l.as_ptr() as *mut NodePtr; for i in 0..l.len() { unsafe { if *p.add(i) == old { *p.add(i) = new; } } } }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_masgn(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_masgn_struct>)).u };
    if u.rhs == old { u.rhs = new; }
    { let l = u.targets; let p = l.as_ptr() as *mut NodePtr; for i in 0..l.len() { unsafe { if *p.add(i) == old { *p.add(i) = new; } } } }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_masgn_elem(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_masgn_elem_struct>)).u };

}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_unsupported(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_unsupported_struct>)).u };

}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_special(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_special_struct>)).u };
    if u.arg == old { u.arg = new; }
}

// This file is auto-generated from node.def by astrustgen. Do not edit.
#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_nil(out: &mut String, n: NodePtr) {
    let u = node_u::<node_nil_struct>(n);
    out.push_str("(node_nil");

    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_true(out: &mut String, n: NodePtr) {
    let u = node_u::<node_true_struct>(n);
    out.push_str("(node_true");

    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_false(out: &mut String, n: NodePtr) {
    let u = node_u::<node_false_struct>(n);
    out.push_str("(node_false");

    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_self(out: &mut String, n: NodePtr) {
    let u = node_u::<node_self_struct>(n);
    out.push_str("(node_self");

    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_lit(out: &mut String, n: NodePtr) {
    let u = node_u::<node_lit_struct>(n);
    out.push_str("(node_lit");
    out.push(' '); out.push_str(&format!("{:?}", u.v));
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_float(out: &mut String, n: NodePtr) {
    let u = node_u::<node_float_struct>(n);
    out.push_str("(node_float");
    out.push(' '); out.push_str(&format!("{:?}", u.v));
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_str(out: &mut String, n: NodePtr) {
    let u = node_u::<node_str_struct>(n);
    out.push_str("(node_str");
    out.push(' '); out.push_str(&format!("{:?}", String::from_utf8_lossy(u.s)));
    out.push(' '); out.push_str(&format!("{:?}", u.enc));
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_fstr(out: &mut String, n: NodePtr) {
    let u = node_u::<node_fstr_struct>(n);
    out.push_str("(node_fstr");
    out.push(' '); out.push_str(&format!("{:?}", String::from_utf8_lossy(u.s)));
    out.push(' '); out.push_str(&format!("{:?}", u.enc));
    out.push(' '); out.push_str("<lc>");
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_fstr_recv(out: &mut String, n: NodePtr) {
    let u = node_u::<node_fstr_recv_struct>(n);
    out.push_str("(node_fstr_recv");
    out.push(' '); out.push_str(&format!("{:?}", String::from_utf8_lossy(u.s)));
    out.push(' '); out.push_str(&format!("{:?}", u.enc));
    out.push(' '); out.push_str(&format!("{:?}", u.um));
    out.push(' '); out.push_str("<lc>");
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_dstr(out: &mut String, n: NodePtr) {
    let u = node_u::<node_dstr_struct>(n);
    out.push_str("(node_dstr");
    out.push(' '); out.push('['); for (i, x) in u.parts.iter().enumerate() { if i > 0 { out.push(' '); } dump_node(out, *x); } out.push(']');
    out.push(' '); out.push_str(&format!("{:?}", u.enc));
    out.push(' '); out.push_str(&format!("{:?}", u.cap));
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_tostr(out: &mut String, n: NodePtr) {
    let u = node_u::<node_tostr_struct>(n);
    out.push_str("(node_tostr");
    out.push(' '); dump_node(out, u.v);
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_dsym(out: &mut String, n: NodePtr) {
    let u = node_u::<node_dsym_struct>(n);
    out.push_str("(node_dsym");
    out.push(' '); dump_node(out, u.s);
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_regex(out: &mut String, n: NodePtr) {
    let u = node_u::<node_regex_struct>(n);
    out.push_str("(node_regex");
    out.push(' '); out.push_str(&format!("{:?}", String::from_utf8_lossy(u.src)));
    out.push(' '); out.push_str(&format!("{:?}", u.opts));
    out.push(' '); out.push_str("<lc>");
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_dregex(out: &mut String, n: NodePtr) {
    let u = node_u::<node_dregex_struct>(n);
    out.push_str("(node_dregex");
    out.push(' '); dump_node(out, u.s);
    out.push(' '); out.push_str(&format!("{:?}", u.opts));
    out.push(' '); out.push_str(&format!("{:?}", u.once));
    out.push(' '); out.push_str("<lc>");
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_biglit(out: &mut String, n: NodePtr) {
    let u = node_u::<node_biglit_struct>(n);
    out.push_str("(node_biglit");
    out.push(' '); out.push_str(&format!("{:?}", String::from_utf8_lossy(u.digits)));
    out.push(' '); out.push_str("<lc>");
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_ary(out: &mut String, n: NodePtr) {
    let u = node_u::<node_ary_struct>(n);
    out.push_str("(node_ary");
    out.push(' '); out.push('['); for (i, x) in u.elems.iter().enumerate() { if i > 0 { out.push(' '); } dump_node(out, *x); } out.push(']');
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_ary_build(out: &mut String, n: NodePtr) {
    let u = node_u::<node_ary_build_struct>(n);
    out.push_str("(node_ary_build");
    out.push(' '); out.push('['); for (i, x) in u.elems.iter().enumerate() { if i > 0 { out.push(' '); } dump_node(out, *x); } out.push(']');
    out.push(' '); out.push_str(&format!("{:?}", String::from_utf8_lossy(u.kinds)));
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_hash(out: &mut String, n: NodePtr) {
    let u = node_u::<node_hash_struct>(n);
    out.push_str("(node_hash");
    out.push(' '); out.push('['); for (i, x) in u.items.iter().enumerate() { if i > 0 { out.push(' '); } dump_node(out, *x); } out.push(']');
    out.push(' '); out.push_str(&format!("{:?}", String::from_utf8_lossy(u.kinds)));
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_range(out: &mut String, n: NodePtr) {
    let u = node_u::<node_range_struct>(n);
    out.push_str("(node_range");
    out.push(' '); dump_node(out, u.b);
    out.push(' '); dump_node(out, u.e);
    out.push(' '); out.push_str(&format!("{:?}", u.excl));
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_range_lit(out: &mut String, n: NodePtr) {
    let u = node_u::<node_range_lit_struct>(n);
    out.push_str("(node_range_lit");
    out.push(' '); out.push_str(&format!("{:?}", u.b));
    out.push(' '); out.push_str(&format!("{:?}", u.e));
    out.push(' '); out.push_str(&format!("{:?}", u.excl));
    out.push(' '); out.push_str("<lc>");
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_lvar(out: &mut String, n: NodePtr) {
    let u = node_u::<node_lvar_struct>(n);
    out.push_str("(node_lvar");
    out.push(' '); out.push_str(&format!("{:?}", u.idx));
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_lasgn(out: &mut String, n: NodePtr) {
    let u = node_u::<node_lasgn_struct>(n);
    out.push_str("(node_lasgn");
    out.push(' '); out.push_str(&format!("{:?}", u.idx));
    out.push(' '); dump_node(out, u.v);
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_dvar(out: &mut String, n: NodePtr) {
    let u = node_u::<node_dvar_struct>(n);
    out.push_str("(node_dvar");
    out.push(' '); out.push_str(&format!("{:?}", u.hops));
    out.push(' '); out.push_str(&format!("{:?}", u.idx));
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_dasgn(out: &mut String, n: NodePtr) {
    let u = node_u::<node_dasgn_struct>(n);
    out.push_str("(node_dasgn");
    out.push(' '); out.push_str(&format!("{:?}", u.hops));
    out.push(' '); out.push_str(&format!("{:?}", u.idx));
    out.push(' '); dump_node(out, u.v);
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_ivar(out: &mut String, n: NodePtr) {
    let u = node_u::<node_ivar_struct>(n);
    out.push_str("(node_ivar");
    out.push(' '); out.push(':'); out.push_str(&crate::symbol::sym_str(u.name));
    out.push(' '); out.push_str("<ic>");
    out.push(' '); 
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_iasgn(out: &mut String, n: NodePtr) {
    let u = node_u::<node_iasgn_struct>(n);
    out.push_str("(node_iasgn");
    out.push(' '); out.push(':'); out.push_str(&crate::symbol::sym_str(u.name));
    out.push(' '); dump_node(out, u.v);
    out.push(' '); out.push_str("<ic>");
    out.push(' '); 
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_ivar_arith(out: &mut String, n: NodePtr) {
    let u = node_u::<node_ivar_arith_struct>(n);
    out.push_str("(node_ivar_arith");
    out.push(' '); out.push(':'); out.push_str(&crate::symbol::sym_str(u.name));
    out.push(' '); out.push_str(&format!("{:?}", u.sub));
    out.push(' '); dump_node(out, u.v);
    out.push(' '); out.push_str(&format!("{:?}", (*node_head_line(n))));
    out.push(' '); out.push_str("<ic>");
    out.push(' '); out.push_str("<cc>");
    out.push(' '); 
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_gvar(out: &mut String, n: NodePtr) {
    let u = node_u::<node_gvar_struct>(n);
    out.push_str("(node_gvar");
    out.push(' '); out.push(':'); out.push_str(&crate::symbol::sym_str(u.name));
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_gasgn(out: &mut String, n: NodePtr) {
    let u = node_u::<node_gasgn_struct>(n);
    out.push_str("(node_gasgn");
    out.push(' '); out.push(':'); out.push_str(&crate::symbol::sym_str(u.name));
    out.push(' '); dump_node(out, u.v);
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_cvar(out: &mut String, n: NodePtr) {
    let u = node_u::<node_cvar_struct>(n);
    out.push_str("(node_cvar");
    out.push(' '); out.push(':'); out.push_str(&crate::symbol::sym_str(u.name));
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_cvasgn(out: &mut String, n: NodePtr) {
    let u = node_u::<node_cvasgn_struct>(n);
    out.push_str("(node_cvasgn");
    out.push(' '); out.push(':'); out.push_str(&crate::symbol::sym_str(u.name));
    out.push(' '); dump_node(out, u.v);
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_const(out: &mut String, n: NodePtr) {
    let u = node_u::<node_const_struct>(n);
    out.push_str("(node_const");
    out.push(' '); out.push(':'); out.push_str(&crate::symbol::sym_str(u.name));
    out.push(' '); out.push_str(&format!("{:?}", (*node_head_line(n))));
    out.push(' '); out.push_str("<cc>");
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_const_path(out: &mut String, n: NodePtr) {
    let u = node_u::<node_const_path_struct>(n);
    out.push_str("(node_const_path");
    out.push(' '); dump_node(out, u.parent);
    out.push(' '); out.push(':'); out.push_str(&crate::symbol::sym_str(u.name));
    out.push(' '); out.push_str(&format!("{:?}", (*node_head_line(n))));
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_top_const(out: &mut String, n: NodePtr) {
    let u = node_u::<node_top_const_struct>(n);
    out.push_str("(node_top_const");
    out.push(' '); out.push(':'); out.push_str(&crate::symbol::sym_str(u.name));
    out.push(' '); out.push_str(&format!("{:?}", (*node_head_line(n))));
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_cdecl(out: &mut String, n: NodePtr) {
    let u = node_u::<node_cdecl_struct>(n);
    out.push_str("(node_cdecl");
    out.push(' '); out.push(':'); out.push_str(&crate::symbol::sym_str(u.name));
    out.push(' '); dump_node(out, u.v);
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_cdecl_path(out: &mut String, n: NodePtr) {
    let u = node_u::<node_cdecl_path_struct>(n);
    out.push_str("(node_cdecl_path");
    out.push(' '); dump_node(out, u.parent);
    out.push(' '); out.push(':'); out.push_str(&crate::symbol::sym_str(u.name));
    out.push(' '); dump_node(out, u.v);
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_errinfo(out: &mut String, n: NodePtr) {
    let u = node_u::<node_errinfo_struct>(n);
    out.push_str("(node_errinfo");

    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_nth_ref(out: &mut String, n: NodePtr) {
    let u = node_u::<node_nth_ref_struct>(n);
    out.push_str("(node_nth_ref");
    out.push(' '); out.push_str(&format!("{:?}", u.nth));
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_back_ref(out: &mut String, n: NodePtr) {
    let u = node_u::<node_back_ref_struct>(n);
    out.push_str("(node_back_ref");
    out.push(' '); out.push_str(&format!("{:?}", u.kind));
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_block(out: &mut String, n: NodePtr) {
    let u = node_u::<node_block_struct>(n);
    out.push_str("(node_block");
    out.push(' '); out.push('['); for (i, x) in u.stmts.iter().enumerate() { if i > 0 { out.push(' '); } dump_node(out, *x); } out.push(']');
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_line(out: &mut String, n: NodePtr) {
    let u = node_u::<node_line_struct>(n);
    out.push_str("(node_line");
    out.push(' '); out.push_str(&format!("{:?}", (*node_head_line(n))));
    out.push(' '); dump_node(out, u.body);
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_trace(out: &mut String, n: NodePtr) {
    let u = node_u::<node_trace_struct>(n);
    out.push_str("(node_trace");
    out.push(' '); out.push_str(&format!("{:?}", u.ev));
    out.push(' '); out.push_str(&format!("{:?}", (*node_head_line(n))));
    out.push(' '); dump_node(out, u.body);
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_poll(out: &mut String, n: NodePtr) {
    let u = node_u::<node_poll_struct>(n);
    out.push_str("(node_poll");
    out.push(' '); out.push_str(&format!("{:?}", (*node_head_line(n))));
    out.push(' '); dump_node(out, u.body);
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_cov(out: &mut String, n: NodePtr) {
    let u = node_u::<node_cov_struct>(n);
    out.push_str("(node_cov");
    out.push(' '); out.push_str(&format!("{:?}", u.id));
    out.push(' '); dump_node(out, u.body);
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_if(out: &mut String, n: NodePtr) {
    let u = node_u::<node_if_struct>(n);
    out.push_str("(node_if");
    out.push(' '); dump_node(out, u.cond);
    out.push(' '); dump_node(out, u.then_b);
    out.push(' '); dump_node(out, u.else_b);
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_and(out: &mut String, n: NodePtr) {
    let u = node_u::<node_and_struct>(n);
    out.push_str("(node_and");
    out.push(' '); dump_node(out, u.a);
    out.push(' '); dump_node(out, u.b);
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_or(out: &mut String, n: NodePtr) {
    let u = node_u::<node_or_struct>(n);
    out.push_str("(node_or");
    out.push(' '); dump_node(out, u.a);
    out.push(' '); dump_node(out, u.b);
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_newarray_minmax(out: &mut String, n: NodePtr) {
    let u = node_u::<node_newarray_minmax_struct>(n);
    out.push_str("(node_newarray_minmax");
    out.push(' '); dump_node(out, u.a);
    out.push(' '); out.push_str(&format!("{:?}", u.mx));
    out.push(' '); out.push_str(&format!("{:?}", (*node_head_line(n))));
    out.push(' '); out.push_str("<cc>");
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_not(out: &mut String, n: NodePtr) {
    let u = node_u::<node_not_struct>(n);
    out.push_str("(node_not");
    out.push(' '); dump_node(out, u.v);
    out.push(' '); out.push_str("<cc>");
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_while(out: &mut String, n: NodePtr) {
    let u = node_u::<node_while_struct>(n);
    out.push_str("(node_while");
    out.push(' '); dump_node(out, u.cond);
    out.push(' '); dump_node(out, u.body);
    out.push(' '); out.push_str(&format!("{:?}", u.until));
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_dowhile(out: &mut String, n: NodePtr) {
    let u = node_u::<node_dowhile_struct>(n);
    out.push_str("(node_dowhile");
    out.push(' '); dump_node(out, u.cond);
    out.push(' '); dump_node(out, u.body);
    out.push(' '); out.push_str(&format!("{:?}", u.until));
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_loop_break(out: &mut String, n: NodePtr) {
    let u = node_u::<node_loop_break_struct>(n);
    out.push_str("(node_loop_break");
    out.push(' '); dump_node(out, u.v);
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_loop_next(out: &mut String, n: NodePtr) {
    let u = node_u::<node_loop_next_struct>(n);
    out.push_str("(node_loop_next");
    out.push(' '); dump_node(out, u.v);
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_break(out: &mut String, n: NodePtr) {
    let u = node_u::<node_break_struct>(n);
    out.push_str("(node_break");
    out.push(' '); dump_node(out, u.v);
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_next(out: &mut String, n: NodePtr) {
    let u = node_u::<node_next_struct>(n);
    out.push_str("(node_next");
    out.push(' '); dump_node(out, u.v);
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_return(out: &mut String, n: NodePtr) {
    let u = node_u::<node_return_struct>(n);
    out.push_str("(node_return");
    out.push(' '); dump_node(out, u.v);
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_retry(out: &mut String, n: NodePtr) {
    let u = node_u::<node_retry_struct>(n);
    out.push_str("(node_retry");

    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_redo(out: &mut String, n: NodePtr) {
    let u = node_u::<node_redo_struct>(n);
    out.push_str("(node_redo");

    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_rescue(out: &mut String, n: NodePtr) {
    let u = node_u::<node_rescue_struct>(n);
    out.push_str("(node_rescue");
    out.push(' '); dump_node(out, u.body);
    out.push(' '); out.push('['); for (i, x) in u.clauses.iter().enumerate() { if i > 0 { out.push(' '); } dump_node(out, *x); } out.push(']');
    out.push(' '); dump_node(out, u.else_b);
    out.push(' '); out.push_str(&format!("{:?}", u.has_else));
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_resbody(out: &mut String, n: NodePtr) {
    let u = node_u::<node_resbody_struct>(n);
    out.push_str("(node_resbody");
    out.push(' '); out.push('['); for (i, x) in u.classes.iter().enumerate() { if i > 0 { out.push(' '); } dump_node(out, *x); } out.push(']');
    out.push(' '); out.push_str(&format!("{:?}", String::from_utf8_lossy(u.kinds)));
    out.push(' '); dump_node(out, u.assign);
    out.push(' '); dump_node(out, u.body);
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_ensure(out: &mut String, n: NodePtr) {
    let u = node_u::<node_ensure_struct>(n);
    out.push_str("(node_ensure");
    out.push(' '); dump_node(out, u.body);
    out.push(' '); dump_node(out, u.ens);
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_when(out: &mut String, n: NodePtr) {
    let u = node_u::<node_when_struct>(n);
    out.push_str("(node_when");
    out.push(' '); out.push_str(&format!("{:?}", u.subj_slot));
    out.push(' '); out.push('['); for (i, x) in u.pats.iter().enumerate() { if i > 0 { out.push(' '); } dump_node(out, *x); } out.push(']');
    out.push(' '); out.push_str(&format!("{:?}", String::from_utf8_lossy(u.kinds)));
    out.push(' '); out.push_str(&format!("{:?}", (*node_head_line(n))));
    out.push(' '); out.push_str("<cc>");
    out.push(' '); out.push_str("<km>");
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_when_lit(out: &mut String, n: NodePtr) {
    let u = node_u::<node_when_lit_struct>(n);
    out.push_str("(node_when_lit");
    out.push(' '); out.push_str(&format!("{:?}", u.subj_slot));
    out.push(' '); out.push_str(&format!("{:?}", u.v));
    out.push(' '); out.push_str(&format!("{:?}", (*node_head_line(n))));
    out.push(' '); out.push_str("<cc>");
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_defined(out: &mut String, n: NodePtr) {
    let u = node_u::<node_defined_struct>(n);
    out.push_str("(node_defined");
    out.push(' '); out.push_str(&format!("{:?}", u.kind));
    out.push(' '); out.push(':'); out.push_str(&crate::symbol::sym_str(u.name));
    out.push(' '); dump_node(out, u.recv);
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_call(out: &mut String, n: NodePtr) {
    let u = node_u::<node_call_struct>(n);
    out.push_str("(node_call");
    out.push(' '); dump_node(out, u.recv);
    out.push(' '); out.push('['); for (i, x) in u.args.iter().enumerate() { if i > 0 { out.push(' '); } dump_node(out, *x); } out.push(']');
    out.push(' '); out.push(':'); out.push_str(&crate::symbol::sym_str(u.mid));
    out.push(' '); out.push_str(&format!("{:?}", u.flags));
    out.push(' '); out.push_str(&format!("{:?}", (*node_head_line(n))));
    out.push(' '); out.push_str("<cc>");
    out.push(' '); 
    out.push(' '); 
    out.push(' '); 
    out.push(' '); 
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_call_blk(out: &mut String, n: NodePtr) {
    let u = node_u::<node_call_blk_struct>(n);
    out.push_str("(node_call_blk");
    out.push(' '); dump_node(out, u.recv);
    out.push(' '); out.push('['); for (i, x) in u.args.iter().enumerate() { if i > 0 { out.push(' '); } dump_node(out, *x); } out.push(']');
    out.push(' '); out.push(':'); out.push_str(&crate::symbol::sym_str(u.mid));
    out.push(' '); out.push_str(&format!("{:?}", u.flags));
    out.push(' '); out.push_str(&format!("{:?}", (*node_head_line(n))));
    out.push(' '); out.push_str("<blk>");
    out.push(' '); out.push_str("<cc>");
    out.push(' '); 
    out.push(' '); 
    out.push(' '); 
    out.push(' '); 
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_call_kw(out: &mut String, n: NodePtr) {
    let u = node_u::<node_call_kw_struct>(n);
    out.push_str("(node_call_kw");
    out.push(' '); dump_node(out, u.recv);
    out.push(' '); out.push('['); for (i, x) in u.args.iter().enumerate() { if i > 0 { out.push(' '); } dump_node(out, *x); } out.push(']');
    out.push(' '); out.push(':'); out.push_str(&crate::symbol::sym_str(u.mid));
    out.push(' '); out.push_str(&format!("{:?}", u.flags));
    out.push(' '); out.push_str(&format!("{:?}", (*node_head_line(n))));
    out.push(' '); out.push_str("<ci>");
    out.push(' '); out.push_str("<cc>");
    out.push(' '); 
    out.push(' '); 
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_call_x(out: &mut String, n: NodePtr) {
    let u = node_u::<node_call_x_struct>(n);
    out.push_str("(node_call_x");
    out.push(' '); dump_node(out, u.recv);
    out.push(' '); out.push('['); for (i, x) in u.args.iter().enumerate() { if i > 0 { out.push(' '); } dump_node(out, *x); } out.push(']');
    out.push(' '); out.push(':'); out.push_str(&crate::symbol::sym_str(u.mid));
    out.push(' '); out.push_str(&format!("{:?}", u.flags));
    out.push(' '); out.push_str(&format!("{:?}", (*node_head_line(n))));
    out.push(' '); out.push_str("<ci>");
    out.push(' '); out.push_str("<cc>");
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_send_sym(out: &mut String, n: NodePtr) {
    let u = node_u::<node_send_sym_struct>(n);
    out.push_str("(node_send_sym");
    out.push(' '); dump_node(out, u.recv);
    out.push(' '); out.push(':'); out.push_str(&crate::symbol::sym_str(u.smid));
    out.push(' '); dump_node(out, u.call);
    out.push(' '); dump_node(out, u.general);
    out.push(' '); out.push_str("<g>");
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_attrasgn(out: &mut String, n: NodePtr) {
    let u = node_u::<node_attrasgn_struct>(n);
    out.push_str("(node_attrasgn");
    out.push(' '); dump_node(out, u.recv);
    out.push(' '); out.push('['); for (i, x) in u.args.iter().enumerate() { if i > 0 { out.push(' '); } dump_node(out, *x); } out.push(']');
    out.push(' '); out.push(':'); out.push_str(&crate::symbol::sym_str(u.mid));
    out.push(' '); out.push_str(&format!("{:?}", u.flags));
    out.push(' '); out.push_str(&format!("{:?}", (*node_head_line(n))));
    out.push(' '); out.push_str("<cc>");
    out.push(' '); 
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_super(out: &mut String, n: NodePtr) {
    let u = node_u::<node_super_struct>(n);
    out.push_str("(node_super");
    out.push(' '); out.push('['); for (i, x) in u.args.iter().enumerate() { if i > 0 { out.push(' '); } dump_node(out, *x); } out.push(']');
    out.push(' '); out.push_str(&format!("{:?}", (*node_head_line(n))));
    out.push(' '); out.push_str("<ci>");
    out.push(' '); out.push_str(&format!("{:?}", u.zsuper));
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_yield(out: &mut String, n: NodePtr) {
    let u = node_u::<node_yield_struct>(n);
    out.push_str("(node_yield");
    out.push(' '); out.push('['); for (i, x) in u.args.iter().enumerate() { if i > 0 { out.push(' '); } dump_node(out, *x); } out.push(']');
    out.push(' '); out.push_str("<yc>");
    out.push(' '); 
    out.push(' '); 
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_yield_x(out: &mut String, n: NodePtr) {
    let u = node_u::<node_yield_x_struct>(n);
    out.push_str("(node_yield_x");
    out.push(' '); out.push('['); for (i, x) in u.args.iter().enumerate() { if i > 0 { out.push(' '); } dump_node(out, *x); } out.push(']');
    out.push(' '); out.push_str("<ci>");
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_block_given(out: &mut String, n: NodePtr) {
    let u = node_u::<node_block_given_struct>(n);
    out.push_str("(node_block_given");

    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_lambda(out: &mut String, n: NodePtr) {
    let u = node_u::<node_lambda_struct>(n);
    out.push_str("(node_lambda");
    out.push(' '); out.push_str("<blk>");
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_opt_plus(out: &mut String, n: NodePtr) {
    let u = node_u::<node_opt_plus_struct>(n);
    out.push_str("(node_opt_plus");
    out.push(' '); dump_node(out, u.a);
    out.push(' '); dump_node(out, u.b);
    out.push(' '); out.push_str(&format!("{:?}", (*node_head_line(n))));
    out.push(' '); out.push_str("<cc>");
    out.push(' '); out.push_str("<tf>");
    out.push(' '); 
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_opt_minus(out: &mut String, n: NodePtr) {
    let u = node_u::<node_opt_minus_struct>(n);
    out.push_str("(node_opt_minus");
    out.push(' '); dump_node(out, u.a);
    out.push(' '); dump_node(out, u.b);
    out.push(' '); out.push_str(&format!("{:?}", (*node_head_line(n))));
    out.push(' '); out.push_str("<cc>");
    out.push(' '); out.push_str("<tf>");
    out.push(' '); 
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_opt_mul(out: &mut String, n: NodePtr) {
    let u = node_u::<node_opt_mul_struct>(n);
    out.push_str("(node_opt_mul");
    out.push(' '); dump_node(out, u.a);
    out.push(' '); dump_node(out, u.b);
    out.push(' '); out.push_str(&format!("{:?}", (*node_head_line(n))));
    out.push(' '); out.push_str("<cc>");
    out.push(' '); out.push_str("<tf>");
    out.push(' '); 
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_opt_div(out: &mut String, n: NodePtr) {
    let u = node_u::<node_opt_div_struct>(n);
    out.push_str("(node_opt_div");
    out.push(' '); dump_node(out, u.a);
    out.push(' '); dump_node(out, u.b);
    out.push(' '); out.push_str(&format!("{:?}", (*node_head_line(n))));
    out.push(' '); out.push_str("<cc>");
    out.push(' '); out.push_str("<tf>");
    out.push(' '); 
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_opt_pow(out: &mut String, n: NodePtr) {
    let u = node_u::<node_opt_pow_struct>(n);
    out.push_str("(node_opt_pow");
    out.push(' '); dump_node(out, u.a);
    out.push(' '); dump_node(out, u.b);
    out.push(' '); out.push_str(&format!("{:?}", (*node_head_line(n))));
    out.push(' '); out.push_str("<cc>");
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_opt_mod(out: &mut String, n: NodePtr) {
    let u = node_u::<node_opt_mod_struct>(n);
    out.push_str("(node_opt_mod");
    out.push(' '); dump_node(out, u.a);
    out.push(' '); dump_node(out, u.b);
    out.push(' '); out.push_str(&format!("{:?}", (*node_head_line(n))));
    out.push(' '); out.push_str("<cc>");
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_opt_lt(out: &mut String, n: NodePtr) {
    let u = node_u::<node_opt_lt_struct>(n);
    out.push_str("(node_opt_lt");
    out.push(' '); dump_node(out, u.a);
    out.push(' '); dump_node(out, u.b);
    out.push(' '); out.push_str(&format!("{:?}", (*node_head_line(n))));
    out.push(' '); out.push_str("<cc>");
    out.push(' '); out.push_str("<tf>");
    out.push(' '); 
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_opt_le(out: &mut String, n: NodePtr) {
    let u = node_u::<node_opt_le_struct>(n);
    out.push_str("(node_opt_le");
    out.push(' '); dump_node(out, u.a);
    out.push(' '); dump_node(out, u.b);
    out.push(' '); out.push_str(&format!("{:?}", (*node_head_line(n))));
    out.push(' '); out.push_str("<cc>");
    out.push(' '); out.push_str("<tf>");
    out.push(' '); 
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_opt_gt(out: &mut String, n: NodePtr) {
    let u = node_u::<node_opt_gt_struct>(n);
    out.push_str("(node_opt_gt");
    out.push(' '); dump_node(out, u.a);
    out.push(' '); dump_node(out, u.b);
    out.push(' '); out.push_str(&format!("{:?}", (*node_head_line(n))));
    out.push(' '); out.push_str("<cc>");
    out.push(' '); out.push_str("<tf>");
    out.push(' '); 
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_opt_ge(out: &mut String, n: NodePtr) {
    let u = node_u::<node_opt_ge_struct>(n);
    out.push_str("(node_opt_ge");
    out.push(' '); dump_node(out, u.a);
    out.push(' '); dump_node(out, u.b);
    out.push(' '); out.push_str(&format!("{:?}", (*node_head_line(n))));
    out.push(' '); out.push_str("<cc>");
    out.push(' '); out.push_str("<tf>");
    out.push(' '); 
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_opt_eq(out: &mut String, n: NodePtr) {
    let u = node_u::<node_opt_eq_struct>(n);
    out.push_str("(node_opt_eq");
    out.push(' '); dump_node(out, u.a);
    out.push(' '); dump_node(out, u.b);
    out.push(' '); out.push_str(&format!("{:?}", (*node_head_line(n))));
    out.push(' '); out.push_str("<cc>");
    out.push(' '); out.push_str("<tf>");
    out.push(' '); 
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_opt_neq(out: &mut String, n: NodePtr) {
    let u = node_u::<node_opt_neq_struct>(n);
    out.push_str("(node_opt_neq");
    out.push(' '); dump_node(out, u.a);
    out.push(' '); dump_node(out, u.b);
    out.push(' '); out.push_str(&format!("{:?}", (*node_head_line(n))));
    out.push(' '); out.push_str("<cc>");
    out.push(' '); out.push_str("<tf>");
    out.push(' '); 
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_opt_ltlt(out: &mut String, n: NodePtr) {
    let u = node_u::<node_opt_ltlt_struct>(n);
    out.push_str("(node_opt_ltlt");
    out.push(' '); dump_node(out, u.a);
    out.push(' '); dump_node(out, u.b);
    out.push(' '); out.push_str(&format!("{:?}", (*node_head_line(n))));
    out.push(' '); out.push_str("<cc>");
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_opt_gtgt(out: &mut String, n: NodePtr) {
    let u = node_u::<node_opt_gtgt_struct>(n);
    out.push_str("(node_opt_gtgt");
    out.push(' '); dump_node(out, u.a);
    out.push(' '); dump_node(out, u.b);
    out.push(' '); out.push_str(&format!("{:?}", (*node_head_line(n))));
    out.push(' '); out.push_str("<cc>");
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_opt_xor(out: &mut String, n: NodePtr) {
    let u = node_u::<node_opt_xor_struct>(n);
    out.push_str("(node_opt_xor");
    out.push(' '); dump_node(out, u.a);
    out.push(' '); dump_node(out, u.b);
    out.push(' '); out.push_str(&format!("{:?}", (*node_head_line(n))));
    out.push(' '); out.push_str("<cc>");
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_opt_and(out: &mut String, n: NodePtr) {
    let u = node_u::<node_opt_and_struct>(n);
    out.push_str("(node_opt_and");
    out.push(' '); dump_node(out, u.a);
    out.push(' '); dump_node(out, u.b);
    out.push(' '); out.push_str(&format!("{:?}", (*node_head_line(n))));
    out.push(' '); out.push_str("<cc>");
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_opt_or(out: &mut String, n: NodePtr) {
    let u = node_u::<node_opt_or_struct>(n);
    out.push_str("(node_opt_or");
    out.push(' '); dump_node(out, u.a);
    out.push(' '); dump_node(out, u.b);
    out.push(' '); out.push_str(&format!("{:?}", (*node_head_line(n))));
    out.push(' '); out.push_str("<cc>");
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_opt_aref(out: &mut String, n: NodePtr) {
    let u = node_u::<node_opt_aref_struct>(n);
    out.push_str("(node_opt_aref");
    out.push(' '); dump_node(out, u.a);
    out.push(' '); dump_node(out, u.b);
    out.push(' '); out.push_str(&format!("{:?}", (*node_head_line(n))));
    out.push(' '); out.push_str("<cc>");
    out.push(' '); out.push_str("<tf>");
    out.push(' '); 
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_opt_aset(out: &mut String, n: NodePtr) {
    let u = node_u::<node_opt_aset_struct>(n);
    out.push_str("(node_opt_aset");
    out.push(' '); dump_node(out, u.a);
    out.push(' '); dump_node(out, u.b);
    out.push(' '); dump_node(out, u.v);
    out.push(' '); out.push_str(&format!("{:?}", (*node_head_line(n))));
    out.push(' '); out.push_str("<cc>");
    out.push(' '); out.push_str("<tf>");
    out.push(' '); 
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_method_entry(out: &mut String, n: NodePtr) {
    let u = node_u::<node_method_entry_struct>(n);
    out.push_str("(node_method_entry");
    out.push(' '); out.push_str(&format!("{:?}", u.nlocals));
    out.push(' '); out.push_str(&format!("{:?}", u.nparams));
    out.push(' '); out.push_str(&format!("{:?}", u.flags));
    out.push(' '); out.push_str(&format!("{:?}", (*node_head_line(n))));
    out.push(' '); dump_node(out, u.body);
    out.push(' '); 
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_block_entry(out: &mut String, n: NodePtr) {
    let u = node_u::<node_block_entry_struct>(n);
    out.push_str("(node_block_entry");
    out.push(' '); out.push_str(&format!("{:?}", u.nlocals));
    out.push(' '); out.push_str(&format!("{:?}", u.nparams));
    out.push(' '); out.push_str(&format!("{:?}", u.flags));
    out.push(' '); out.push_str(&format!("{:?}", (*node_head_line(n))));
    out.push(' '); dump_node(out, u.body);
    out.push(' '); 
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_def(out: &mut String, n: NodePtr) {
    let u = node_u::<node_def_struct>(n);
    out.push_str("(node_def");
    out.push(' '); out.push(':'); out.push_str(&crate::symbol::sym_str(u.mid));
    out.push(' '); out.push_str("<body>");
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_sdef(out: &mut String, n: NodePtr) {
    let u = node_u::<node_sdef_struct>(n);
    out.push_str("(node_sdef");
    out.push(' '); dump_node(out, u.recv);
    out.push(' '); out.push(':'); out.push_str(&crate::symbol::sym_str(u.mid));
    out.push(' '); out.push_str("<body>");
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_class(out: &mut String, n: NodePtr) {
    let u = node_u::<node_class_struct>(n);
    out.push_str("(node_class");
    out.push(' '); dump_node(out, u.cpath);
    out.push(' '); out.push(':'); out.push_str(&crate::symbol::sym_str(u.name));
    out.push(' '); dump_node(out, u.superclass);
    out.push(' '); out.push_str("<body>");
    out.push(' '); out.push_str(&format!("{:?}", u.has_cpath));
    out.push(' '); out.push_str(&format!("{:?}", u.has_super));
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_module(out: &mut String, n: NodePtr) {
    let u = node_u::<node_module_struct>(n);
    out.push_str("(node_module");
    out.push(' '); dump_node(out, u.cpath);
    out.push(' '); out.push(':'); out.push_str(&crate::symbol::sym_str(u.name));
    out.push(' '); out.push_str("<body>");
    out.push(' '); out.push_str(&format!("{:?}", u.has_cpath));
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_sclass(out: &mut String, n: NodePtr) {
    let u = node_u::<node_sclass_struct>(n);
    out.push_str("(node_sclass");
    out.push(' '); dump_node(out, u.recv);
    out.push(' '); out.push_str("<body>");
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_alias(out: &mut String, n: NodePtr) {
    let u = node_u::<node_alias_struct>(n);
    out.push_str("(node_alias");
    out.push(' '); dump_node(out, u.newname);
    out.push(' '); dump_node(out, u.oldname);
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_valias(out: &mut String, n: NodePtr) {
    let u = node_u::<node_valias_struct>(n);
    out.push_str("(node_valias");
    out.push(' '); out.push(':'); out.push_str(&crate::symbol::sym_str(u.newname));
    out.push(' '); out.push(':'); out.push_str(&crate::symbol::sym_str(u.oldname));
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_undef(out: &mut String, n: NodePtr) {
    let u = node_u::<node_undef_struct>(n);
    out.push_str("(node_undef");
    out.push(' '); out.push('['); for (i, x) in u.names.iter().enumerate() { if i > 0 { out.push(' '); } dump_node(out, *x); } out.push(']');
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_masgn(out: &mut String, n: NodePtr) {
    let u = node_u::<node_masgn_struct>(n);
    out.push_str("(node_masgn");
    out.push(' '); dump_node(out, u.rhs);
    out.push(' '); out.push_str(&format!("{:?}", u.tmp));
    out.push(' '); out.push('['); for (i, x) in u.targets.iter().enumerate() { if i > 0 { out.push(' '); } dump_node(out, *x); } out.push(']');
    out.push(' '); out.push_str(&format!("{:?}", u.splat_rhs));
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_masgn_elem(out: &mut String, n: NodePtr) {
    let u = node_u::<node_masgn_elem_struct>(n);
    out.push_str("(node_masgn_elem");
    out.push(' '); out.push_str(&format!("{:?}", u.tmp));
    out.push(' '); out.push_str(&format!("{:?}", u.idx));
    out.push(' '); out.push_str(&format!("{:?}", u.kind));
    out.push(' '); out.push_str(&format!("{:?}", u.post));
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_unsupported(out: &mut String, n: NodePtr) {
    let u = node_u::<node_unsupported_struct>(n);
    out.push_str("(node_unsupported");
    out.push(' '); out.push_str(&format!("{:?}", String::from_utf8_lossy(u.what)));
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_special(out: &mut String, n: NodePtr) {
    let u = node_u::<node_special_struct>(n);
    out.push_str("(node_special");
    out.push(' '); out.push_str(&format!("{:?}", u.kind));
    out.push(' '); dump_node(out, u.arg);
    out.push(')');
}

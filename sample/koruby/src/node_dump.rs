// This file is auto-generated from node.def by astrustgen. Do not edit.
#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_nil(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_nil_struct>;
    unsafe {
    out.push_str("(node_nil");

    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_true(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_true_struct>;
    unsafe {
    out.push_str("(node_true");

    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_false(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_false_struct>;
    unsafe {
    out.push_str("(node_false");

    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_self(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_self_struct>;
    unsafe {
    out.push_str("(node_self");

    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_lit(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_lit_struct>;
    unsafe {
    out.push_str("(node_lit");
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.v));
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_float(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_float_struct>;
    unsafe {
    out.push_str("(node_float");
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.v));
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_str(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_str_struct>;
    unsafe {
    out.push_str("(node_str");
    out.push(' '); out.push_str(&format!("{:?}", String::from_utf8_lossy((*np).u.s)));
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.enc));
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_fstr(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_fstr_struct>;
    unsafe {
    out.push_str("(node_fstr");
    out.push(' '); out.push_str(&format!("{:?}", String::from_utf8_lossy((*np).u.s)));
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.enc));
    out.push(' '); out.push_str("<lc>");
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_dstr(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_dstr_struct>;
    unsafe {
    out.push_str("(node_dstr");
    out.push(' '); out.push('['); for (i, x) in (*np).u.parts.iter().enumerate() { if i > 0 { out.push(' '); } dump_node(out, *x); } out.push(']');
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.enc));
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.cap));
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_tostr(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_tostr_struct>;
    unsafe {
    out.push_str("(node_tostr");
    out.push(' '); dump_node(out, (*np).u.v);
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_dsym(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_dsym_struct>;
    unsafe {
    out.push_str("(node_dsym");
    out.push(' '); dump_node(out, (*np).u.s);
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_regex(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_regex_struct>;
    unsafe {
    out.push_str("(node_regex");
    out.push(' '); out.push_str(&format!("{:?}", String::from_utf8_lossy((*np).u.src)));
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.opts));
    out.push(' '); out.push_str("<lc>");
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_dregex(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_dregex_struct>;
    unsafe {
    out.push_str("(node_dregex");
    out.push(' '); dump_node(out, (*np).u.s);
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.opts));
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.once));
    out.push(' '); out.push_str("<lc>");
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_biglit(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_biglit_struct>;
    unsafe {
    out.push_str("(node_biglit");
    out.push(' '); out.push_str(&format!("{:?}", String::from_utf8_lossy((*np).u.digits)));
    out.push(' '); out.push_str("<lc>");
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_ary(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_ary_struct>;
    unsafe {
    out.push_str("(node_ary");
    out.push(' '); out.push('['); for (i, x) in (*np).u.elems.iter().enumerate() { if i > 0 { out.push(' '); } dump_node(out, *x); } out.push(']');
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_ary_build(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_ary_build_struct>;
    unsafe {
    out.push_str("(node_ary_build");
    out.push(' '); out.push('['); for (i, x) in (*np).u.elems.iter().enumerate() { if i > 0 { out.push(' '); } dump_node(out, *x); } out.push(']');
    out.push(' '); out.push_str(&format!("{:?}", String::from_utf8_lossy((*np).u.kinds)));
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_hash(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_hash_struct>;
    unsafe {
    out.push_str("(node_hash");
    out.push(' '); out.push('['); for (i, x) in (*np).u.items.iter().enumerate() { if i > 0 { out.push(' '); } dump_node(out, *x); } out.push(']');
    out.push(' '); out.push_str(&format!("{:?}", String::from_utf8_lossy((*np).u.kinds)));
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_range(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_range_struct>;
    unsafe {
    out.push_str("(node_range");
    out.push(' '); dump_node(out, (*np).u.b);
    out.push(' '); dump_node(out, (*np).u.e);
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.excl));
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_range_lit(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_range_lit_struct>;
    unsafe {
    out.push_str("(node_range_lit");
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.b));
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.e));
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.excl));
    out.push(' '); out.push_str("<lc>");
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_lvar(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_lvar_struct>;
    unsafe {
    out.push_str("(node_lvar");
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.idx));
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_lasgn(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_lasgn_struct>;
    unsafe {
    out.push_str("(node_lasgn");
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.idx));
    out.push(' '); dump_node(out, (*np).u.v);
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_dvar(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_dvar_struct>;
    unsafe {
    out.push_str("(node_dvar");
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.hops));
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.idx));
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_dasgn(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_dasgn_struct>;
    unsafe {
    out.push_str("(node_dasgn");
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.hops));
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.idx));
    out.push(' '); dump_node(out, (*np).u.v);
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_ivar(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_ivar_struct>;
    unsafe {
    out.push_str("(node_ivar");
    out.push(' '); out.push(':'); out.push_str(&crate::symbol::sym_str((*np).u.name));
    out.push(' '); out.push_str("<ic>");
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_iasgn(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_iasgn_struct>;
    unsafe {
    out.push_str("(node_iasgn");
    out.push(' '); out.push(':'); out.push_str(&crate::symbol::sym_str((*np).u.name));
    out.push(' '); dump_node(out, (*np).u.v);
    out.push(' '); out.push_str("<ic>");
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_gvar(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_gvar_struct>;
    unsafe {
    out.push_str("(node_gvar");
    out.push(' '); out.push(':'); out.push_str(&crate::symbol::sym_str((*np).u.name));
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_gasgn(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_gasgn_struct>;
    unsafe {
    out.push_str("(node_gasgn");
    out.push(' '); out.push(':'); out.push_str(&crate::symbol::sym_str((*np).u.name));
    out.push(' '); dump_node(out, (*np).u.v);
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_cvar(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_cvar_struct>;
    unsafe {
    out.push_str("(node_cvar");
    out.push(' '); out.push(':'); out.push_str(&crate::symbol::sym_str((*np).u.name));
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_cvasgn(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_cvasgn_struct>;
    unsafe {
    out.push_str("(node_cvasgn");
    out.push(' '); out.push(':'); out.push_str(&crate::symbol::sym_str((*np).u.name));
    out.push(' '); dump_node(out, (*np).u.v);
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_const(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_const_struct>;
    unsafe {
    out.push_str("(node_const");
    out.push(' '); out.push(':'); out.push_str(&crate::symbol::sym_str((*np).u.name));
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.line));
    out.push(' '); out.push_str("<cc>");
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_const_path(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_const_path_struct>;
    unsafe {
    out.push_str("(node_const_path");
    out.push(' '); dump_node(out, (*np).u.parent);
    out.push(' '); out.push(':'); out.push_str(&crate::symbol::sym_str((*np).u.name));
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.line));
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_top_const(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_top_const_struct>;
    unsafe {
    out.push_str("(node_top_const");
    out.push(' '); out.push(':'); out.push_str(&crate::symbol::sym_str((*np).u.name));
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_cdecl(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_cdecl_struct>;
    unsafe {
    out.push_str("(node_cdecl");
    out.push(' '); out.push(':'); out.push_str(&crate::symbol::sym_str((*np).u.name));
    out.push(' '); dump_node(out, (*np).u.v);
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_cdecl_path(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_cdecl_path_struct>;
    unsafe {
    out.push_str("(node_cdecl_path");
    out.push(' '); dump_node(out, (*np).u.parent);
    out.push(' '); out.push(':'); out.push_str(&crate::symbol::sym_str((*np).u.name));
    out.push(' '); dump_node(out, (*np).u.v);
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_errinfo(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_errinfo_struct>;
    unsafe {
    out.push_str("(node_errinfo");

    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_nth_ref(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_nth_ref_struct>;
    unsafe {
    out.push_str("(node_nth_ref");
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.nth));
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_back_ref(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_back_ref_struct>;
    unsafe {
    out.push_str("(node_back_ref");
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.kind));
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_block(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_block_struct>;
    unsafe {
    out.push_str("(node_block");
    out.push(' '); out.push('['); for (i, x) in (*np).u.stmts.iter().enumerate() { if i > 0 { out.push(' '); } dump_node(out, *x); } out.push(']');
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_line(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_line_struct>;
    unsafe {
    out.push_str("(node_line");
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.line));
    out.push(' '); dump_node(out, (*np).u.body);
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_if(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_if_struct>;
    unsafe {
    out.push_str("(node_if");
    out.push(' '); dump_node(out, (*np).u.cond);
    out.push(' '); dump_node(out, (*np).u.then_b);
    out.push(' '); dump_node(out, (*np).u.else_b);
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_and(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_and_struct>;
    unsafe {
    out.push_str("(node_and");
    out.push(' '); dump_node(out, (*np).u.a);
    out.push(' '); dump_node(out, (*np).u.b);
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_or(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_or_struct>;
    unsafe {
    out.push_str("(node_or");
    out.push(' '); dump_node(out, (*np).u.a);
    out.push(' '); dump_node(out, (*np).u.b);
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_not(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_not_struct>;
    unsafe {
    out.push_str("(node_not");
    out.push(' '); dump_node(out, (*np).u.v);
    out.push(' '); out.push_str("<cc>");
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_while(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_while_struct>;
    unsafe {
    out.push_str("(node_while");
    out.push(' '); dump_node(out, (*np).u.cond);
    out.push(' '); dump_node(out, (*np).u.body);
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.until));
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_dowhile(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_dowhile_struct>;
    unsafe {
    out.push_str("(node_dowhile");
    out.push(' '); dump_node(out, (*np).u.cond);
    out.push(' '); dump_node(out, (*np).u.body);
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.until));
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_loop_break(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_loop_break_struct>;
    unsafe {
    out.push_str("(node_loop_break");
    out.push(' '); dump_node(out, (*np).u.v);
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_loop_next(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_loop_next_struct>;
    unsafe {
    out.push_str("(node_loop_next");
    out.push(' '); dump_node(out, (*np).u.v);
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_break(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_break_struct>;
    unsafe {
    out.push_str("(node_break");
    out.push(' '); dump_node(out, (*np).u.v);
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_next(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_next_struct>;
    unsafe {
    out.push_str("(node_next");
    out.push(' '); dump_node(out, (*np).u.v);
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_return(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_return_struct>;
    unsafe {
    out.push_str("(node_return");
    out.push(' '); dump_node(out, (*np).u.v);
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_retry(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_retry_struct>;
    unsafe {
    out.push_str("(node_retry");

    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_redo(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_redo_struct>;
    unsafe {
    out.push_str("(node_redo");

    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_rescue(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_rescue_struct>;
    unsafe {
    out.push_str("(node_rescue");
    out.push(' '); dump_node(out, (*np).u.body);
    out.push(' '); out.push('['); for (i, x) in (*np).u.clauses.iter().enumerate() { if i > 0 { out.push(' '); } dump_node(out, *x); } out.push(']');
    out.push(' '); dump_node(out, (*np).u.else_b);
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.has_else));
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_resbody(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_resbody_struct>;
    unsafe {
    out.push_str("(node_resbody");
    out.push(' '); out.push('['); for (i, x) in (*np).u.classes.iter().enumerate() { if i > 0 { out.push(' '); } dump_node(out, *x); } out.push(']');
    out.push(' '); out.push_str(&format!("{:?}", String::from_utf8_lossy((*np).u.kinds)));
    out.push(' '); dump_node(out, (*np).u.assign);
    out.push(' '); dump_node(out, (*np).u.body);
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_ensure(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_ensure_struct>;
    unsafe {
    out.push_str("(node_ensure");
    out.push(' '); dump_node(out, (*np).u.body);
    out.push(' '); dump_node(out, (*np).u.ens);
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_when(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_when_struct>;
    unsafe {
    out.push_str("(node_when");
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.subj_slot));
    out.push(' '); out.push('['); for (i, x) in (*np).u.pats.iter().enumerate() { if i > 0 { out.push(' '); } dump_node(out, *x); } out.push(']');
    out.push(' '); out.push_str(&format!("{:?}", String::from_utf8_lossy((*np).u.kinds)));
    out.push(' '); out.push_str("<cc>");
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_when_lit(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_when_lit_struct>;
    unsafe {
    out.push_str("(node_when_lit");
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.subj_slot));
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.v));
    out.push(' '); out.push_str("<cc>");
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_defined(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_defined_struct>;
    unsafe {
    out.push_str("(node_defined");
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.kind));
    out.push(' '); out.push(':'); out.push_str(&crate::symbol::sym_str((*np).u.name));
    out.push(' '); dump_node(out, (*np).u.recv);
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_call(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_call_struct>;
    unsafe {
    out.push_str("(node_call");
    out.push(' '); dump_node(out, (*np).u.recv);
    out.push(' '); out.push('['); for (i, x) in (*np).u.args.iter().enumerate() { if i > 0 { out.push(' '); } dump_node(out, *x); } out.push(']');
    out.push(' '); out.push(':'); out.push_str(&crate::symbol::sym_str((*np).u.mid));
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.flags));
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.line));
    out.push(' '); out.push_str("<cc>");
    out.push(' '); 
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_call_blk(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_call_blk_struct>;
    unsafe {
    out.push_str("(node_call_blk");
    out.push(' '); dump_node(out, (*np).u.recv);
    out.push(' '); out.push('['); for (i, x) in (*np).u.args.iter().enumerate() { if i > 0 { out.push(' '); } dump_node(out, *x); } out.push(']');
    out.push(' '); out.push(':'); out.push_str(&crate::symbol::sym_str((*np).u.mid));
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.flags));
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.line));
    out.push(' '); out.push_str("<blk>");
    out.push(' '); out.push_str("<cc>");
    out.push(' '); 
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_call_kw(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_call_kw_struct>;
    unsafe {
    out.push_str("(node_call_kw");
    out.push(' '); dump_node(out, (*np).u.recv);
    out.push(' '); out.push('['); for (i, x) in (*np).u.args.iter().enumerate() { if i > 0 { out.push(' '); } dump_node(out, *x); } out.push(']');
    out.push(' '); out.push(':'); out.push_str(&crate::symbol::sym_str((*np).u.mid));
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.flags));
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.line));
    out.push(' '); out.push_str("<ci>");
    out.push(' '); out.push_str("<cc>");
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_call_x(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_call_x_struct>;
    unsafe {
    out.push_str("(node_call_x");
    out.push(' '); dump_node(out, (*np).u.recv);
    out.push(' '); out.push('['); for (i, x) in (*np).u.args.iter().enumerate() { if i > 0 { out.push(' '); } dump_node(out, *x); } out.push(']');
    out.push(' '); out.push(':'); out.push_str(&crate::symbol::sym_str((*np).u.mid));
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.flags));
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.line));
    out.push(' '); out.push_str("<ci>");
    out.push(' '); out.push_str("<cc>");
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_attrasgn(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_attrasgn_struct>;
    unsafe {
    out.push_str("(node_attrasgn");
    out.push(' '); dump_node(out, (*np).u.recv);
    out.push(' '); out.push('['); for (i, x) in (*np).u.args.iter().enumerate() { if i > 0 { out.push(' '); } dump_node(out, *x); } out.push(']');
    out.push(' '); out.push(':'); out.push_str(&crate::symbol::sym_str((*np).u.mid));
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.flags));
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.line));
    out.push(' '); out.push_str("<cc>");
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_super(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_super_struct>;
    unsafe {
    out.push_str("(node_super");
    out.push(' '); out.push('['); for (i, x) in (*np).u.args.iter().enumerate() { if i > 0 { out.push(' '); } dump_node(out, *x); } out.push(']');
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.line));
    out.push(' '); out.push_str("<ci>");
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.zsuper));
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_yield(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_yield_struct>;
    unsafe {
    out.push_str("(node_yield");
    out.push(' '); out.push('['); for (i, x) in (*np).u.args.iter().enumerate() { if i > 0 { out.push(' '); } dump_node(out, *x); } out.push(']');
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_yield_x(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_yield_x_struct>;
    unsafe {
    out.push_str("(node_yield_x");
    out.push(' '); out.push('['); for (i, x) in (*np).u.args.iter().enumerate() { if i > 0 { out.push(' '); } dump_node(out, *x); } out.push(']');
    out.push(' '); out.push_str("<ci>");
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_block_given(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_block_given_struct>;
    unsafe {
    out.push_str("(node_block_given");

    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_lambda(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_lambda_struct>;
    unsafe {
    out.push_str("(node_lambda");
    out.push(' '); out.push_str("<blk>");
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_opt_plus(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_opt_plus_struct>;
    unsafe {
    out.push_str("(node_opt_plus");
    out.push(' '); dump_node(out, (*np).u.a);
    out.push(' '); dump_node(out, (*np).u.b);
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.line));
    out.push(' '); out.push_str("<cc>");
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_opt_minus(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_opt_minus_struct>;
    unsafe {
    out.push_str("(node_opt_minus");
    out.push(' '); dump_node(out, (*np).u.a);
    out.push(' '); dump_node(out, (*np).u.b);
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.line));
    out.push(' '); out.push_str("<cc>");
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_opt_mul(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_opt_mul_struct>;
    unsafe {
    out.push_str("(node_opt_mul");
    out.push(' '); dump_node(out, (*np).u.a);
    out.push(' '); dump_node(out, (*np).u.b);
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.line));
    out.push(' '); out.push_str("<cc>");
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_opt_div(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_opt_div_struct>;
    unsafe {
    out.push_str("(node_opt_div");
    out.push(' '); dump_node(out, (*np).u.a);
    out.push(' '); dump_node(out, (*np).u.b);
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.line));
    out.push(' '); out.push_str("<cc>");
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_opt_pow(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_opt_pow_struct>;
    unsafe {
    out.push_str("(node_opt_pow");
    out.push(' '); dump_node(out, (*np).u.a);
    out.push(' '); dump_node(out, (*np).u.b);
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.line));
    out.push(' '); out.push_str("<cc>");
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_opt_mod(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_opt_mod_struct>;
    unsafe {
    out.push_str("(node_opt_mod");
    out.push(' '); dump_node(out, (*np).u.a);
    out.push(' '); dump_node(out, (*np).u.b);
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.line));
    out.push(' '); out.push_str("<cc>");
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_opt_lt(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_opt_lt_struct>;
    unsafe {
    out.push_str("(node_opt_lt");
    out.push(' '); dump_node(out, (*np).u.a);
    out.push(' '); dump_node(out, (*np).u.b);
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.line));
    out.push(' '); out.push_str("<cc>");
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_opt_le(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_opt_le_struct>;
    unsafe {
    out.push_str("(node_opt_le");
    out.push(' '); dump_node(out, (*np).u.a);
    out.push(' '); dump_node(out, (*np).u.b);
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.line));
    out.push(' '); out.push_str("<cc>");
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_opt_gt(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_opt_gt_struct>;
    unsafe {
    out.push_str("(node_opt_gt");
    out.push(' '); dump_node(out, (*np).u.a);
    out.push(' '); dump_node(out, (*np).u.b);
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.line));
    out.push(' '); out.push_str("<cc>");
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_opt_ge(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_opt_ge_struct>;
    unsafe {
    out.push_str("(node_opt_ge");
    out.push(' '); dump_node(out, (*np).u.a);
    out.push(' '); dump_node(out, (*np).u.b);
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.line));
    out.push(' '); out.push_str("<cc>");
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_opt_eq(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_opt_eq_struct>;
    unsafe {
    out.push_str("(node_opt_eq");
    out.push(' '); dump_node(out, (*np).u.a);
    out.push(' '); dump_node(out, (*np).u.b);
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.line));
    out.push(' '); out.push_str("<cc>");
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_opt_neq(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_opt_neq_struct>;
    unsafe {
    out.push_str("(node_opt_neq");
    out.push(' '); dump_node(out, (*np).u.a);
    out.push(' '); dump_node(out, (*np).u.b);
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.line));
    out.push(' '); out.push_str("<cc>");
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_opt_ltlt(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_opt_ltlt_struct>;
    unsafe {
    out.push_str("(node_opt_ltlt");
    out.push(' '); dump_node(out, (*np).u.a);
    out.push(' '); dump_node(out, (*np).u.b);
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.line));
    out.push(' '); out.push_str("<cc>");
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_opt_gtgt(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_opt_gtgt_struct>;
    unsafe {
    out.push_str("(node_opt_gtgt");
    out.push(' '); dump_node(out, (*np).u.a);
    out.push(' '); dump_node(out, (*np).u.b);
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.line));
    out.push(' '); out.push_str("<cc>");
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_opt_xor(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_opt_xor_struct>;
    unsafe {
    out.push_str("(node_opt_xor");
    out.push(' '); dump_node(out, (*np).u.a);
    out.push(' '); dump_node(out, (*np).u.b);
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.line));
    out.push(' '); out.push_str("<cc>");
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_opt_and(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_opt_and_struct>;
    unsafe {
    out.push_str("(node_opt_and");
    out.push(' '); dump_node(out, (*np).u.a);
    out.push(' '); dump_node(out, (*np).u.b);
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.line));
    out.push(' '); out.push_str("<cc>");
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_opt_or(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_opt_or_struct>;
    unsafe {
    out.push_str("(node_opt_or");
    out.push(' '); dump_node(out, (*np).u.a);
    out.push(' '); dump_node(out, (*np).u.b);
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.line));
    out.push(' '); out.push_str("<cc>");
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_opt_aref(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_opt_aref_struct>;
    unsafe {
    out.push_str("(node_opt_aref");
    out.push(' '); dump_node(out, (*np).u.a);
    out.push(' '); dump_node(out, (*np).u.b);
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.line));
    out.push(' '); out.push_str("<cc>");
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_opt_aset(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_opt_aset_struct>;
    unsafe {
    out.push_str("(node_opt_aset");
    out.push(' '); dump_node(out, (*np).u.a);
    out.push(' '); dump_node(out, (*np).u.b);
    out.push(' '); dump_node(out, (*np).u.v);
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.line));
    out.push(' '); out.push_str("<cc>");
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_method_entry(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_method_entry_struct>;
    unsafe {
    out.push_str("(node_method_entry");
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.nlocals));
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.nparams));
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.has_env));
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.line));
    out.push(' '); dump_node(out, (*np).u.body);
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_def(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_def_struct>;
    unsafe {
    out.push_str("(node_def");
    out.push(' '); out.push(':'); out.push_str(&crate::symbol::sym_str((*np).u.mid));
    out.push(' '); out.push_str("<body>");
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_sdef(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_sdef_struct>;
    unsafe {
    out.push_str("(node_sdef");
    out.push(' '); dump_node(out, (*np).u.recv);
    out.push(' '); out.push(':'); out.push_str(&crate::symbol::sym_str((*np).u.mid));
    out.push(' '); out.push_str("<body>");
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_class(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_class_struct>;
    unsafe {
    out.push_str("(node_class");
    out.push(' '); dump_node(out, (*np).u.cpath);
    out.push(' '); out.push(':'); out.push_str(&crate::symbol::sym_str((*np).u.name));
    out.push(' '); dump_node(out, (*np).u.superclass);
    out.push(' '); out.push_str("<body>");
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.has_cpath));
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.has_super));
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_module(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_module_struct>;
    unsafe {
    out.push_str("(node_module");
    out.push(' '); dump_node(out, (*np).u.cpath);
    out.push(' '); out.push(':'); out.push_str(&crate::symbol::sym_str((*np).u.name));
    out.push(' '); out.push_str("<body>");
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.has_cpath));
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_sclass(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_sclass_struct>;
    unsafe {
    out.push_str("(node_sclass");
    out.push(' '); dump_node(out, (*np).u.recv);
    out.push(' '); out.push_str("<body>");
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_alias(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_alias_struct>;
    unsafe {
    out.push_str("(node_alias");
    out.push(' '); dump_node(out, (*np).u.newname);
    out.push(' '); dump_node(out, (*np).u.oldname);
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_valias(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_valias_struct>;
    unsafe {
    out.push_str("(node_valias");
    out.push(' '); out.push(':'); out.push_str(&crate::symbol::sym_str((*np).u.newname));
    out.push(' '); out.push(':'); out.push_str(&crate::symbol::sym_str((*np).u.oldname));
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_undef(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_undef_struct>;
    unsafe {
    out.push_str("(node_undef");
    out.push(' '); out.push('['); for (i, x) in (*np).u.names.iter().enumerate() { if i > 0 { out.push(' '); } dump_node(out, *x); } out.push(']');
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_masgn(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_masgn_struct>;
    unsafe {
    out.push_str("(node_masgn");
    out.push(' '); dump_node(out, (*np).u.rhs);
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.tmp));
    out.push(' '); out.push('['); for (i, x) in (*np).u.targets.iter().enumerate() { if i > 0 { out.push(' '); } dump_node(out, *x); } out.push(']');
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.splat_rhs));
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_masgn_elem(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_masgn_elem_struct>;
    unsafe {
    out.push_str("(node_masgn_elem");
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.tmp));
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.idx));
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.kind));
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.post));
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_unsupported(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_unsupported_struct>;
    unsafe {
    out.push_str("(node_unsupported");
    out.push(' '); out.push_str(&format!("{:?}", String::from_utf8_lossy((*np).u.what)));
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_special(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_special_struct>;
    unsafe {
    out.push_str("(node_special");
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.kind));
    out.push(' '); dump_node(out, (*np).u.arg);
    out.push(')');
    }
}

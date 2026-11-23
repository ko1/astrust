// This file is auto-generated from node.def by astrustgen. Do not edit.
#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_nil(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_nil");
    let u = node_u::<node_nil_struct>(n);

    EVAL_node_nil(c, n, fp)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_true(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_true");
    let u = node_u::<node_true_struct>(n);

    EVAL_node_true(c, n, fp)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_false(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_false");
    let u = node_u::<node_false_struct>(n);

    EVAL_node_false(c, n, fp)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_self(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_self");
    let u = node_u::<node_self_struct>(n);

    EVAL_node_self(c, n, fp)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_lit(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_lit");
    let u = node_u::<node_lit_struct>(n);

    EVAL_node_lit(c, n, fp, u.v)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_float(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_float");
    let u = node_u::<node_float_struct>(n);

    EVAL_node_float(c, n, fp, u.v)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_str(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_str");
    let u = node_u::<node_str_struct>(n);

    EVAL_node_str(c, n, fp, u.s, u.enc)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_fstr(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_fstr");
    let u = node_u::<node_fstr_struct>(n);

    EVAL_node_fstr(c, n, fp, u.s, u.enc, &u.lc)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_fstr_recv(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_fstr_recv");
    let u = node_u::<node_fstr_recv_struct>(n);

    EVAL_node_fstr_recv(c, n, fp, u.s, u.enc, u.um, &u.lc)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_dstr(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_dstr");
    let u = node_u::<node_dstr_struct>(n);

    EVAL_node_dstr::<DynList>(c, n, fp, u.parts, u.enc, u.cap)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_tostr(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_tostr");
    let u = node_u::<node_tostr_struct>(n);
    let v_v = <Dyn as Disp>::call(c, u.v, fp)?;
    EVAL_node_tostr(c, n, fp, v_v)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_dsym(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_dsym");
    let u = node_u::<node_dsym_struct>(n);
    let v_s = <Dyn as Disp>::call(c, u.s, fp)?;
    EVAL_node_dsym(c, n, fp, v_s)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_regex(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_regex");
    let u = node_u::<node_regex_struct>(n);

    EVAL_node_regex(c, n, fp, u.src, u.opts, &u.lc)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_dregex(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_dregex");
    let u = node_u::<node_dregex_struct>(n);
    let v_s = <Dyn as Disp>::call(c, u.s, fp)?;
    EVAL_node_dregex(c, n, fp, v_s, u.opts, u.once, &u.lc)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_biglit(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_biglit");
    let u = node_u::<node_biglit_struct>(n);

    EVAL_node_biglit(c, n, fp, u.digits, &u.lc)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_ary(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_ary");
    let u = node_u::<node_ary_struct>(n);

    EVAL_node_ary::<DynList>(c, n, fp, u.elems)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_ary_build(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_ary_build");
    let u = node_u::<node_ary_build_struct>(n);

    EVAL_node_ary_build::<DynList>(c, n, fp, u.elems, u.kinds)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_hash(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_hash");
    let u = node_u::<node_hash_struct>(n);

    EVAL_node_hash::<DynList>(c, n, fp, u.items, u.kinds)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_range(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_range");
    let u = node_u::<node_range_struct>(n);
    let v_b = <Dyn as Disp>::call(c, u.b, fp)?;
    let v_e = <Dyn as Disp>::call(c, u.e, fp)?;
    EVAL_node_range(c, n, fp, v_b, v_e, u.excl)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_range_lit(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_range_lit");
    let u = node_u::<node_range_lit_struct>(n);

    EVAL_node_range_lit(c, n, fp, u.b, u.e, u.excl, &u.lc)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_lvar(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_lvar");
    let u = node_u::<node_lvar_struct>(n);

    EVAL_node_lvar(c, n, fp, u.idx)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_lasgn(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_lasgn");
    let u = node_u::<node_lasgn_struct>(n);
    let v_v = <Dyn as Disp>::call(c, u.v, fp)?;
    EVAL_node_lasgn(c, n, fp, u.idx, v_v)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_dvar(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_dvar");
    let u = node_u::<node_dvar_struct>(n);

    EVAL_node_dvar(c, n, fp, u.hops, u.idx)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_dasgn(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_dasgn");
    let u = node_u::<node_dasgn_struct>(n);
    let v_v = <Dyn as Disp>::call(c, u.v, fp)?;
    EVAL_node_dasgn(c, n, fp, u.hops, u.idx, v_v)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_ivar(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_ivar");
    let u = node_u::<node_ivar_struct>(n);

    EVAL_node_ivar(c, n, fp, u.name, &u.ic, 0)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_iasgn(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_iasgn");
    let u = node_u::<node_iasgn_struct>(n);
    let v_v = <Dyn as Disp>::call(c, u.v, fp)?;
    EVAL_node_iasgn(c, n, fp, u.name, v_v, &u.ic, 0)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_ivar_arith(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_ivar_arith");
    let u = node_u::<node_ivar_arith_struct>(n);
    let v_v = <Dyn as Disp>::call(c, u.v, fp)?;
    EVAL_node_ivar_arith::<crate::vm::LineAt>(c, n, fp, u.name, u.sub, v_v, crate::vm::LineAt(&(*node_head_line(n))), &u.ic, &u.cc, 0)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_gvar(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_gvar");
    let u = node_u::<node_gvar_struct>(n);

    EVAL_node_gvar(c, n, fp, u.name)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_gasgn(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_gasgn");
    let u = node_u::<node_gasgn_struct>(n);
    let v_v = <Dyn as Disp>::call(c, u.v, fp)?;
    EVAL_node_gasgn(c, n, fp, u.name, v_v)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_cvar(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_cvar");
    let u = node_u::<node_cvar_struct>(n);

    EVAL_node_cvar(c, n, fp, u.name)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_cvasgn(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_cvasgn");
    let u = node_u::<node_cvasgn_struct>(n);
    let v_v = <Dyn as Disp>::call(c, u.v, fp)?;
    EVAL_node_cvasgn(c, n, fp, u.name, v_v)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_const(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_const");
    let u = node_u::<node_const_struct>(n);

    EVAL_node_const::<crate::vm::LineAt>(c, n, fp, u.name, crate::vm::LineAt(&(*node_head_line(n))), &u.cc)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_const_path(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_const_path");
    let u = node_u::<node_const_path_struct>(n);
    let v_parent = <Dyn as Disp>::call(c, u.parent, fp)?;
    EVAL_node_const_path::<crate::vm::LineAt>(c, n, fp, v_parent, u.name, crate::vm::LineAt(&(*node_head_line(n))))
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_top_const(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_top_const");
    let u = node_u::<node_top_const_struct>(n);

    EVAL_node_top_const(c, n, fp, u.name)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_cdecl(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_cdecl");
    let u = node_u::<node_cdecl_struct>(n);
    let v_v = <Dyn as Disp>::call(c, u.v, fp)?;
    EVAL_node_cdecl(c, n, fp, u.name, v_v)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_cdecl_path(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_cdecl_path");
    let u = node_u::<node_cdecl_path_struct>(n);
    let v_parent = <Dyn as Disp>::call(c, u.parent, fp)?;
    let v_v = <Dyn as Disp>::call(c, u.v, fp)?;
    EVAL_node_cdecl_path(c, n, fp, v_parent, u.name, v_v)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_errinfo(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_errinfo");
    let u = node_u::<node_errinfo_struct>(n);

    EVAL_node_errinfo(c, n, fp)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_nth_ref(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_nth_ref");
    let u = node_u::<node_nth_ref_struct>(n);

    EVAL_node_nth_ref(c, n, fp, u.nth)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_back_ref(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_back_ref");
    let u = node_u::<node_back_ref_struct>(n);

    EVAL_node_back_ref(c, n, fp, u.kind)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_block(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_block");
    let u = node_u::<node_block_struct>(n);

    EVAL_node_block::<DynList>(c, n, fp, u.stmts)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_line(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_line");
    let u = node_u::<node_line_struct>(n);

    EVAL_node_line::<crate::vm::LineAt, Dyn>(c, n, fp, crate::vm::LineAt(&(*node_head_line(n))), u.body)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_trace(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_trace");
    let u = node_u::<node_trace_struct>(n);

    EVAL_node_trace::<crate::vm::LineAt, Dyn>(c, n, fp, u.ev, crate::vm::LineAt(&(*node_head_line(n))), u.body)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_poll(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_poll");
    let u = node_u::<node_poll_struct>(n);

    EVAL_node_poll::<crate::vm::LineAt, Dyn>(c, n, fp, crate::vm::LineAt(&(*node_head_line(n))), u.body)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_cov(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_cov");
    let u = node_u::<node_cov_struct>(n);

    EVAL_node_cov::<Dyn>(c, n, fp, u.id, u.body)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_if(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_if");
    let u = node_u::<node_if_struct>(n);

    EVAL_node_if::<Dyn, Dyn, Dyn>(c, n, fp, u.cond, u.then_b, u.else_b)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_and(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_and");
    let u = node_u::<node_and_struct>(n);

    EVAL_node_and::<Dyn, Dyn>(c, n, fp, u.a, u.b)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_or(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_or");
    let u = node_u::<node_or_struct>(n);

    EVAL_node_or::<Dyn, Dyn>(c, n, fp, u.a, u.b)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_newarray_minmax(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_newarray_minmax");
    let u = node_u::<node_newarray_minmax_struct>(n);
    let v_a = <Dyn as Disp>::call(c, u.a, fp)?;
    EVAL_node_newarray_minmax::<crate::vm::LineAt>(c, n, fp, v_a, u.mx, crate::vm::LineAt(&(*node_head_line(n))), &u.cc)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_not(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_not");
    let u = node_u::<node_not_struct>(n);
    let v_v = <Dyn as Disp>::call(c, u.v, fp)?;
    EVAL_node_not(c, n, fp, v_v, &u.cc)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_while(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_while");
    let u = node_u::<node_while_struct>(n);

    EVAL_node_while::<Dyn, Dyn>(c, n, fp, u.cond, u.body, u.until)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_dowhile(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_dowhile");
    let u = node_u::<node_dowhile_struct>(n);

    EVAL_node_dowhile::<Dyn, Dyn>(c, n, fp, u.cond, u.body, u.until)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_loop_break(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_loop_break");
    let u = node_u::<node_loop_break_struct>(n);
    let v_v = <Dyn as Disp>::call(c, u.v, fp)?;
    EVAL_node_loop_break(c, n, fp, v_v)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_loop_next(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_loop_next");
    let u = node_u::<node_loop_next_struct>(n);
    let v_v = <Dyn as Disp>::call(c, u.v, fp)?;
    EVAL_node_loop_next(c, n, fp, v_v)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_break(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_break");
    let u = node_u::<node_break_struct>(n);
    let v_v = <Dyn as Disp>::call(c, u.v, fp)?;
    EVAL_node_break(c, n, fp, v_v)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_next(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_next");
    let u = node_u::<node_next_struct>(n);
    let v_v = <Dyn as Disp>::call(c, u.v, fp)?;
    EVAL_node_next(c, n, fp, v_v)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_return(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_return");
    let u = node_u::<node_return_struct>(n);
    let v_v = <Dyn as Disp>::call(c, u.v, fp)?;
    EVAL_node_return(c, n, fp, v_v)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_retry(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_retry");
    let u = node_u::<node_retry_struct>(n);

    EVAL_node_retry(c, n, fp)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_redo(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_redo");
    let u = node_u::<node_redo_struct>(n);

    EVAL_node_redo(c, n, fp)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_rescue(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_rescue");
    let u = node_u::<node_rescue_struct>(n);

    EVAL_node_rescue::<Dyn, DynList, Dyn>(c, n, fp, u.body, u.clauses, u.else_b, u.has_else)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_resbody(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_resbody");
    let u = node_u::<node_resbody_struct>(n);

    EVAL_node_resbody::<DynList, Dyn, Dyn>(c, n, fp, u.classes, u.kinds, u.assign, u.body)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_ensure(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_ensure");
    let u = node_u::<node_ensure_struct>(n);

    EVAL_node_ensure::<Dyn, Dyn>(c, n, fp, u.body, u.ens)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_when(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_when");
    let u = node_u::<node_when_struct>(n);

    EVAL_node_when::<DynList, crate::vm::LineAt>(c, n, fp, u.subj_slot, u.pats, u.kinds, crate::vm::LineAt(&(*node_head_line(n))), &u.cc, &u.km)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_when_lit(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_when_lit");
    let u = node_u::<node_when_lit_struct>(n);

    EVAL_node_when_lit::<crate::vm::LineAt>(c, n, fp, u.subj_slot, u.v, crate::vm::LineAt(&(*node_head_line(n))), &u.cc)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_defined(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_defined");
    let u = node_u::<node_defined_struct>(n);

    EVAL_node_defined::<Dyn>(c, n, fp, u.kind, u.name, u.recv)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_call");
    let u = node_u::<node_call_struct>(n);

    EVAL_node_call::<Dyn, DynList, crate::vm::LineAt, Dyn>(c, n, fp, u.recv, u.args, u.mid, u.flags, crate::vm::LineAt(&(*node_head_line(n))), &u.cc, 0u64, 0, 0, 0)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_call_blk(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_call_blk");
    let u = node_u::<node_call_blk_struct>(n);

    EVAL_node_call_blk::<Dyn, DynList, crate::vm::LineAt, Dyn, Dyn>(c, n, fp, u.recv, u.args, u.mid, u.flags, crate::vm::LineAt(&(*node_head_line(n))), u.blk, &u.cc, 0u64, 0, 0u64, 0)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_call_kw(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_call_kw");
    let u = node_u::<node_call_kw_struct>(n);

    EVAL_node_call_kw::<Dyn, DynList, crate::vm::LineAt, Dyn>(c, n, fp, u.recv, u.args, u.mid, u.flags, crate::vm::LineAt(&(*node_head_line(n))), u.ci, &u.cc, 0u64, 0)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_call_x(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_call_x");
    let u = node_u::<node_call_x_struct>(n);

    EVAL_node_call_x::<Dyn, DynList, crate::vm::LineAt>(c, n, fp, u.recv, u.args, u.mid, u.flags, crate::vm::LineAt(&(*node_head_line(n))), u.ci, &u.cc)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_send_sym(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_send_sym");
    let u = node_u::<node_send_sym_struct>(n);
    let v_recv = <Dyn as Disp>::call(c, u.recv, fp)?;
    EVAL_node_send_sym::<Dyn, Dyn>(c, n, fp, v_recv, u.smid, u.call, u.general, &u.g)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_attrasgn(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_attrasgn");
    let u = node_u::<node_attrasgn_struct>(n);

    EVAL_node_attrasgn::<Dyn, DynList, crate::vm::LineAt>(c, n, fp, u.recv, u.args, u.mid, u.flags, crate::vm::LineAt(&(*node_head_line(n))), &u.cc, 0)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_super(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_super");
    let u = node_u::<node_super_struct>(n);

    EVAL_node_super::<DynList, crate::vm::LineAt>(c, n, fp, u.args, crate::vm::LineAt(&(*node_head_line(n))), u.ci, u.zsuper)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_yield(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_yield");
    let u = node_u::<node_yield_struct>(n);

    EVAL_node_yield::<DynList, Dyn>(c, n, fp, u.args, &u.yc, 0u64, 0)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_yield_x(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_yield_x");
    let u = node_u::<node_yield_x_struct>(n);

    EVAL_node_yield_x::<DynList>(c, n, fp, u.args, u.ci)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_block_given(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_block_given");
    let u = node_u::<node_block_given_struct>(n);

    EVAL_node_block_given(c, n, fp)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_lambda(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_lambda");
    let u = node_u::<node_lambda_struct>(n);

    EVAL_node_lambda(c, n, fp, u.blk)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_opt_plus(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_opt_plus");
    let u = node_u::<node_opt_plus_struct>(n);
    let v_a = <Dyn as Disp>::call(c, u.a, fp)?;
    let v_b = <Dyn as Disp>::call(c, u.b, fp)?;
    EVAL_node_opt_plus::<crate::vm::LineAt>(c, n, fp, v_a, v_b, crate::vm::LineAt(&(*node_head_line(n))), &u.cc, &u.tf, 0)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_opt_minus(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_opt_minus");
    let u = node_u::<node_opt_minus_struct>(n);
    let v_a = <Dyn as Disp>::call(c, u.a, fp)?;
    let v_b = <Dyn as Disp>::call(c, u.b, fp)?;
    EVAL_node_opt_minus::<crate::vm::LineAt>(c, n, fp, v_a, v_b, crate::vm::LineAt(&(*node_head_line(n))), &u.cc, &u.tf, 0)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_opt_mul(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_opt_mul");
    let u = node_u::<node_opt_mul_struct>(n);
    let v_a = <Dyn as Disp>::call(c, u.a, fp)?;
    let v_b = <Dyn as Disp>::call(c, u.b, fp)?;
    EVAL_node_opt_mul::<crate::vm::LineAt>(c, n, fp, v_a, v_b, crate::vm::LineAt(&(*node_head_line(n))), &u.cc, &u.tf, 0)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_opt_div(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_opt_div");
    let u = node_u::<node_opt_div_struct>(n);
    let v_a = <Dyn as Disp>::call(c, u.a, fp)?;
    let v_b = <Dyn as Disp>::call(c, u.b, fp)?;
    EVAL_node_opt_div::<crate::vm::LineAt>(c, n, fp, v_a, v_b, crate::vm::LineAt(&(*node_head_line(n))), &u.cc, &u.tf, 0)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_opt_pow(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_opt_pow");
    let u = node_u::<node_opt_pow_struct>(n);
    let v_a = <Dyn as Disp>::call(c, u.a, fp)?;
    let v_b = <Dyn as Disp>::call(c, u.b, fp)?;
    EVAL_node_opt_pow::<crate::vm::LineAt>(c, n, fp, v_a, v_b, crate::vm::LineAt(&(*node_head_line(n))), &u.cc)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_opt_mod(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_opt_mod");
    let u = node_u::<node_opt_mod_struct>(n);
    let v_a = <Dyn as Disp>::call(c, u.a, fp)?;
    let v_b = <Dyn as Disp>::call(c, u.b, fp)?;
    EVAL_node_opt_mod::<crate::vm::LineAt>(c, n, fp, v_a, v_b, crate::vm::LineAt(&(*node_head_line(n))), &u.cc)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_opt_lt(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_opt_lt");
    let u = node_u::<node_opt_lt_struct>(n);
    let v_a = <Dyn as Disp>::call(c, u.a, fp)?;
    let v_b = <Dyn as Disp>::call(c, u.b, fp)?;
    EVAL_node_opt_lt::<crate::vm::LineAt>(c, n, fp, v_a, v_b, crate::vm::LineAt(&(*node_head_line(n))), &u.cc, &u.tf, 0)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_opt_le(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_opt_le");
    let u = node_u::<node_opt_le_struct>(n);
    let v_a = <Dyn as Disp>::call(c, u.a, fp)?;
    let v_b = <Dyn as Disp>::call(c, u.b, fp)?;
    EVAL_node_opt_le::<crate::vm::LineAt>(c, n, fp, v_a, v_b, crate::vm::LineAt(&(*node_head_line(n))), &u.cc, &u.tf, 0)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_opt_gt(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_opt_gt");
    let u = node_u::<node_opt_gt_struct>(n);
    let v_a = <Dyn as Disp>::call(c, u.a, fp)?;
    let v_b = <Dyn as Disp>::call(c, u.b, fp)?;
    EVAL_node_opt_gt::<crate::vm::LineAt>(c, n, fp, v_a, v_b, crate::vm::LineAt(&(*node_head_line(n))), &u.cc, &u.tf, 0)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_opt_ge(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_opt_ge");
    let u = node_u::<node_opt_ge_struct>(n);
    let v_a = <Dyn as Disp>::call(c, u.a, fp)?;
    let v_b = <Dyn as Disp>::call(c, u.b, fp)?;
    EVAL_node_opt_ge::<crate::vm::LineAt>(c, n, fp, v_a, v_b, crate::vm::LineAt(&(*node_head_line(n))), &u.cc, &u.tf, 0)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_opt_eq(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_opt_eq");
    let u = node_u::<node_opt_eq_struct>(n);
    let v_a = <Dyn as Disp>::call(c, u.a, fp)?;
    let v_b = <Dyn as Disp>::call(c, u.b, fp)?;
    EVAL_node_opt_eq::<crate::vm::LineAt>(c, n, fp, v_a, v_b, crate::vm::LineAt(&(*node_head_line(n))), &u.cc, &u.tf, 0)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_opt_neq(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_opt_neq");
    let u = node_u::<node_opt_neq_struct>(n);
    let v_a = <Dyn as Disp>::call(c, u.a, fp)?;
    let v_b = <Dyn as Disp>::call(c, u.b, fp)?;
    EVAL_node_opt_neq::<crate::vm::LineAt>(c, n, fp, v_a, v_b, crate::vm::LineAt(&(*node_head_line(n))), &u.cc, &u.tf, 0)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_opt_ltlt(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_opt_ltlt");
    let u = node_u::<node_opt_ltlt_struct>(n);
    let v_a = <Dyn as Disp>::call(c, u.a, fp)?;
    let v_b = <Dyn as Disp>::call(c, u.b, fp)?;
    EVAL_node_opt_ltlt::<crate::vm::LineAt>(c, n, fp, v_a, v_b, crate::vm::LineAt(&(*node_head_line(n))), &u.cc)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_opt_gtgt(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_opt_gtgt");
    let u = node_u::<node_opt_gtgt_struct>(n);
    let v_a = <Dyn as Disp>::call(c, u.a, fp)?;
    let v_b = <Dyn as Disp>::call(c, u.b, fp)?;
    EVAL_node_opt_gtgt::<crate::vm::LineAt>(c, n, fp, v_a, v_b, crate::vm::LineAt(&(*node_head_line(n))), &u.cc)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_opt_xor(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_opt_xor");
    let u = node_u::<node_opt_xor_struct>(n);
    let v_a = <Dyn as Disp>::call(c, u.a, fp)?;
    let v_b = <Dyn as Disp>::call(c, u.b, fp)?;
    EVAL_node_opt_xor::<crate::vm::LineAt>(c, n, fp, v_a, v_b, crate::vm::LineAt(&(*node_head_line(n))), &u.cc)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_opt_and(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_opt_and");
    let u = node_u::<node_opt_and_struct>(n);
    let v_a = <Dyn as Disp>::call(c, u.a, fp)?;
    let v_b = <Dyn as Disp>::call(c, u.b, fp)?;
    EVAL_node_opt_and::<crate::vm::LineAt>(c, n, fp, v_a, v_b, crate::vm::LineAt(&(*node_head_line(n))), &u.cc)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_opt_or(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_opt_or");
    let u = node_u::<node_opt_or_struct>(n);
    let v_a = <Dyn as Disp>::call(c, u.a, fp)?;
    let v_b = <Dyn as Disp>::call(c, u.b, fp)?;
    EVAL_node_opt_or::<crate::vm::LineAt>(c, n, fp, v_a, v_b, crate::vm::LineAt(&(*node_head_line(n))), &u.cc)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_opt_aref(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_opt_aref");
    let u = node_u::<node_opt_aref_struct>(n);
    let v_a = <Dyn as Disp>::call(c, u.a, fp)?;
    let v_b = <Dyn as Disp>::call(c, u.b, fp)?;
    EVAL_node_opt_aref::<crate::vm::LineAt>(c, n, fp, v_a, v_b, crate::vm::LineAt(&(*node_head_line(n))), &u.cc, &u.tf, 0)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_opt_aset(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_opt_aset");
    let u = node_u::<node_opt_aset_struct>(n);
    let v_a = <Dyn as Disp>::call(c, u.a, fp)?;
    let v_b = <Dyn as Disp>::call(c, u.b, fp)?;
    let v_v = <Dyn as Disp>::call(c, u.v, fp)?;
    EVAL_node_opt_aset::<crate::vm::LineAt>(c, n, fp, v_a, v_b, v_v, crate::vm::LineAt(&(*node_head_line(n))), &u.cc, &u.tf, 0)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_method_entry(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_method_entry");
    let u = node_u::<node_method_entry_struct>(n);

    EVAL_node_method_entry::<crate::vm::LineAt, Dyn>(c, n, fp, u.nlocals, u.nparams, u.flags, crate::vm::LineAt(&(*node_head_line(n))), u.body, crate::vm::CallArgs::load(c))
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_block_entry(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_block_entry");
    let u = node_u::<node_block_entry_struct>(n);

    EVAL_node_block_entry::<crate::vm::LineAt, Dyn>(c, n, fp, u.nlocals, u.nparams, u.flags, crate::vm::LineAt(&(*node_head_line(n))), u.body, crate::vm::CallArgs::load(c))
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_def(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_def");
    let u = node_u::<node_def_struct>(n);

    EVAL_node_def(c, n, fp, u.mid, u.body)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_sdef(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_sdef");
    let u = node_u::<node_sdef_struct>(n);
    let v_recv = <Dyn as Disp>::call(c, u.recv, fp)?;
    EVAL_node_sdef(c, n, fp, v_recv, u.mid, u.body)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_class(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_class");
    let u = node_u::<node_class_struct>(n);

    EVAL_node_class::<Dyn, Dyn>(c, n, fp, u.cpath, u.name, u.superclass, u.body, u.has_cpath, u.has_super)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_module(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_module");
    let u = node_u::<node_module_struct>(n);

    EVAL_node_module::<Dyn>(c, n, fp, u.cpath, u.name, u.body, u.has_cpath)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_sclass(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_sclass");
    let u = node_u::<node_sclass_struct>(n);
    let v_recv = <Dyn as Disp>::call(c, u.recv, fp)?;
    EVAL_node_sclass(c, n, fp, v_recv, u.body)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_alias(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_alias");
    let u = node_u::<node_alias_struct>(n);
    let v_newname = <Dyn as Disp>::call(c, u.newname, fp)?;
    let v_oldname = <Dyn as Disp>::call(c, u.oldname, fp)?;
    EVAL_node_alias(c, n, fp, v_newname, v_oldname)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_valias(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_valias");
    let u = node_u::<node_valias_struct>(n);

    EVAL_node_valias(c, n, fp, u.newname, u.oldname)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_undef(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_undef");
    let u = node_u::<node_undef_struct>(n);

    EVAL_node_undef::<DynList>(c, n, fp, u.names)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_masgn(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_masgn");
    let u = node_u::<node_masgn_struct>(n);
    let v_rhs = <Dyn as Disp>::call(c, u.rhs, fp)?;
    EVAL_node_masgn::<DynList>(c, n, fp, v_rhs, u.tmp, u.targets, u.splat_rhs)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_masgn_elem(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_masgn_elem");
    let u = node_u::<node_masgn_elem_struct>(n);

    EVAL_node_masgn_elem(c, n, fp, u.tmp, u.idx, u.kind, u.post)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_unsupported(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_unsupported");
    let u = node_u::<node_unsupported_struct>(n);

    EVAL_node_unsupported(c, n, fp, u.what)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_special(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    #[cfg(feature = "nodeprof")] crate::nodeprof::hit("node_special");
    let u = node_u::<node_special_struct>(n);

    EVAL_node_special::<Dyn>(c, n, fp, u.kind, u.arg)
}

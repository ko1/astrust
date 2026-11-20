// This file is auto-generated from node.def by astrustgen. Do not edit.
#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_nil(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_nil_struct>;
    unsafe {

    EVAL_node_nil(c, n, fp)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_true(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_true_struct>;
    unsafe {

    EVAL_node_true(c, n, fp)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_false(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_false_struct>;
    unsafe {

    EVAL_node_false(c, n, fp)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_self(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_self_struct>;
    unsafe {

    EVAL_node_self(c, n, fp)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_lit(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_lit_struct>;
    unsafe {

    EVAL_node_lit(c, n, fp, (*np).u.v)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_float(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_float_struct>;
    unsafe {

    EVAL_node_float(c, n, fp, (*np).u.v)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_str(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_str_struct>;
    unsafe {

    EVAL_node_str(c, n, fp, (*np).u.s, (*np).u.enc)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_fstr(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_fstr_struct>;
    unsafe {

    EVAL_node_fstr(c, n, fp, (*np).u.s, (*np).u.enc, &mut (*np).u.lc)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_dstr(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_dstr_struct>;
    unsafe {

    EVAL_node_dstr::<DynList>(c, n, fp, (*np).u.parts, (*np).u.enc, (*np).u.cap)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_tostr(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_tostr_struct>;
    unsafe {
    let v_v = <Dyn as Disp>::call(c, (*np).u.v, fp)?;
    EVAL_node_tostr(c, n, fp, v_v)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_dsym(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_dsym_struct>;
    unsafe {
    let v_s = <Dyn as Disp>::call(c, (*np).u.s, fp)?;
    EVAL_node_dsym(c, n, fp, v_s)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_regex(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_regex_struct>;
    unsafe {

    EVAL_node_regex(c, n, fp, (*np).u.src, (*np).u.opts, &mut (*np).u.lc)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_dregex(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_dregex_struct>;
    unsafe {
    let v_s = <Dyn as Disp>::call(c, (*np).u.s, fp)?;
    EVAL_node_dregex(c, n, fp, v_s, (*np).u.opts, (*np).u.once, &mut (*np).u.lc)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_biglit(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_biglit_struct>;
    unsafe {

    EVAL_node_biglit(c, n, fp, (*np).u.digits, &mut (*np).u.lc)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_ary(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_ary_struct>;
    unsafe {

    EVAL_node_ary::<DynList>(c, n, fp, (*np).u.elems)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_ary_build(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_ary_build_struct>;
    unsafe {

    EVAL_node_ary_build::<DynList>(c, n, fp, (*np).u.elems, (*np).u.kinds)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_hash(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_hash_struct>;
    unsafe {

    EVAL_node_hash::<DynList>(c, n, fp, (*np).u.items, (*np).u.kinds)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_range(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_range_struct>;
    unsafe {
    let v_b = <Dyn as Disp>::call(c, (*np).u.b, fp)?;
    let v_e = <Dyn as Disp>::call(c, (*np).u.e, fp)?;
    EVAL_node_range(c, n, fp, v_b, v_e, (*np).u.excl)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_range_lit(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_range_lit_struct>;
    unsafe {

    EVAL_node_range_lit(c, n, fp, (*np).u.b, (*np).u.e, (*np).u.excl, &mut (*np).u.lc)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_lvar(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_lvar_struct>;
    unsafe {

    EVAL_node_lvar(c, n, fp, (*np).u.idx)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_lasgn(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_lasgn_struct>;
    unsafe {
    let v_v = <Dyn as Disp>::call(c, (*np).u.v, fp)?;
    EVAL_node_lasgn(c, n, fp, (*np).u.idx, v_v)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_dvar(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_dvar_struct>;
    unsafe {

    EVAL_node_dvar(c, n, fp, (*np).u.hops, (*np).u.idx)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_dasgn(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_dasgn_struct>;
    unsafe {
    let v_v = <Dyn as Disp>::call(c, (*np).u.v, fp)?;
    EVAL_node_dasgn(c, n, fp, (*np).u.hops, (*np).u.idx, v_v)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_ivar(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_ivar_struct>;
    unsafe {

    EVAL_node_ivar(c, n, fp, (*np).u.name, &mut (*np).u.ic)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_iasgn(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_iasgn_struct>;
    unsafe {
    let v_v = <Dyn as Disp>::call(c, (*np).u.v, fp)?;
    EVAL_node_iasgn(c, n, fp, (*np).u.name, v_v, &mut (*np).u.ic)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_gvar(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_gvar_struct>;
    unsafe {

    EVAL_node_gvar(c, n, fp, (*np).u.name)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_gasgn(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_gasgn_struct>;
    unsafe {
    let v_v = <Dyn as Disp>::call(c, (*np).u.v, fp)?;
    EVAL_node_gasgn(c, n, fp, (*np).u.name, v_v)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_cvar(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_cvar_struct>;
    unsafe {

    EVAL_node_cvar(c, n, fp, (*np).u.name)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_cvasgn(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_cvasgn_struct>;
    unsafe {
    let v_v = <Dyn as Disp>::call(c, (*np).u.v, fp)?;
    EVAL_node_cvasgn(c, n, fp, (*np).u.name, v_v)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_const(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_const_struct>;
    unsafe {

    EVAL_node_const(c, n, fp, (*np).u.name, (*np).u.line, &mut (*np).u.cc)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_const_path(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_const_path_struct>;
    unsafe {
    let v_parent = <Dyn as Disp>::call(c, (*np).u.parent, fp)?;
    EVAL_node_const_path(c, n, fp, v_parent, (*np).u.name, (*np).u.line)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_top_const(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_top_const_struct>;
    unsafe {

    EVAL_node_top_const(c, n, fp, (*np).u.name)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_cdecl(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_cdecl_struct>;
    unsafe {
    let v_v = <Dyn as Disp>::call(c, (*np).u.v, fp)?;
    EVAL_node_cdecl(c, n, fp, (*np).u.name, v_v)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_cdecl_path(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_cdecl_path_struct>;
    unsafe {
    let v_parent = <Dyn as Disp>::call(c, (*np).u.parent, fp)?;
    let v_v = <Dyn as Disp>::call(c, (*np).u.v, fp)?;
    EVAL_node_cdecl_path(c, n, fp, v_parent, (*np).u.name, v_v)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_errinfo(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_errinfo_struct>;
    unsafe {

    EVAL_node_errinfo(c, n, fp)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_nth_ref(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_nth_ref_struct>;
    unsafe {

    EVAL_node_nth_ref(c, n, fp, (*np).u.nth)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_back_ref(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_back_ref_struct>;
    unsafe {

    EVAL_node_back_ref(c, n, fp, (*np).u.kind)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_block(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_block_struct>;
    unsafe {

    EVAL_node_block::<DynList>(c, n, fp, (*np).u.stmts)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_line(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_line_struct>;
    unsafe {

    EVAL_node_line::<Dyn>(c, n, fp, (*np).u.line, (*np).u.body)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_if(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_if_struct>;
    unsafe {

    EVAL_node_if::<Dyn, Dyn, Dyn>(c, n, fp, (*np).u.cond, (*np).u.then_b, (*np).u.else_b)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_and(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_and_struct>;
    unsafe {

    EVAL_node_and::<Dyn, Dyn>(c, n, fp, (*np).u.a, (*np).u.b)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_or(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_or_struct>;
    unsafe {

    EVAL_node_or::<Dyn, Dyn>(c, n, fp, (*np).u.a, (*np).u.b)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_not(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_not_struct>;
    unsafe {
    let v_v = <Dyn as Disp>::call(c, (*np).u.v, fp)?;
    EVAL_node_not(c, n, fp, v_v, &mut (*np).u.cc)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_while(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_while_struct>;
    unsafe {

    EVAL_node_while::<Dyn, Dyn>(c, n, fp, (*np).u.cond, (*np).u.body, (*np).u.until)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_dowhile(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_dowhile_struct>;
    unsafe {

    EVAL_node_dowhile::<Dyn, Dyn>(c, n, fp, (*np).u.cond, (*np).u.body, (*np).u.until)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_loop_break(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_loop_break_struct>;
    unsafe {
    let v_v = <Dyn as Disp>::call(c, (*np).u.v, fp)?;
    EVAL_node_loop_break(c, n, fp, v_v)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_loop_next(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_loop_next_struct>;
    unsafe {
    let v_v = <Dyn as Disp>::call(c, (*np).u.v, fp)?;
    EVAL_node_loop_next(c, n, fp, v_v)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_break(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_break_struct>;
    unsafe {
    let v_v = <Dyn as Disp>::call(c, (*np).u.v, fp)?;
    EVAL_node_break(c, n, fp, v_v)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_next(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_next_struct>;
    unsafe {
    let v_v = <Dyn as Disp>::call(c, (*np).u.v, fp)?;
    EVAL_node_next(c, n, fp, v_v)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_return(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_return_struct>;
    unsafe {
    let v_v = <Dyn as Disp>::call(c, (*np).u.v, fp)?;
    EVAL_node_return(c, n, fp, v_v)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_retry(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_retry_struct>;
    unsafe {

    EVAL_node_retry(c, n, fp)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_redo(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_redo_struct>;
    unsafe {

    EVAL_node_redo(c, n, fp)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_rescue(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_rescue_struct>;
    unsafe {

    EVAL_node_rescue::<Dyn, DynList, Dyn>(c, n, fp, (*np).u.body, (*np).u.clauses, (*np).u.else_b, (*np).u.has_else)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_resbody(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_resbody_struct>;
    unsafe {

    EVAL_node_resbody::<DynList, Dyn, Dyn>(c, n, fp, (*np).u.classes, (*np).u.kinds, (*np).u.assign, (*np).u.body)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_ensure(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_ensure_struct>;
    unsafe {

    EVAL_node_ensure::<Dyn, Dyn>(c, n, fp, (*np).u.body, (*np).u.ens)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_when(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_when_struct>;
    unsafe {

    EVAL_node_when::<DynList>(c, n, fp, (*np).u.subj_slot, (*np).u.pats, (*np).u.kinds, &mut (*np).u.cc)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_when_lit(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_when_lit_struct>;
    unsafe {

    EVAL_node_when_lit(c, n, fp, (*np).u.subj_slot, (*np).u.v, &mut (*np).u.cc)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_defined(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_defined_struct>;
    unsafe {

    EVAL_node_defined::<Dyn>(c, n, fp, (*np).u.kind, (*np).u.name, (*np).u.recv)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_call_struct>;
    unsafe {

    EVAL_node_call::<Dyn, DynList, Dyn>(c, n, fp, (*np).u.recv, (*np).u.args, (*np).u.mid, (*np).u.flags, (*np).u.line, &mut (*np).u.cc, 0u64)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_call_blk(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_call_blk_struct>;
    unsafe {

    EVAL_node_call_blk::<Dyn, DynList, Dyn>(c, n, fp, (*np).u.recv, (*np).u.args, (*np).u.mid, (*np).u.flags, (*np).u.line, (*np).u.blk, &mut (*np).u.cc, 0u64)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_call_kw(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_call_kw_struct>;
    unsafe {

    EVAL_node_call_kw::<Dyn, DynList>(c, n, fp, (*np).u.recv, (*np).u.args, (*np).u.mid, (*np).u.flags, (*np).u.line, (*np).u.ci, &mut (*np).u.cc)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_call_x(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_call_x_struct>;
    unsafe {

    EVAL_node_call_x::<Dyn, DynList>(c, n, fp, (*np).u.recv, (*np).u.args, (*np).u.mid, (*np).u.flags, (*np).u.line, (*np).u.ci, &mut (*np).u.cc)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_attrasgn(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_attrasgn_struct>;
    unsafe {

    EVAL_node_attrasgn::<Dyn, DynList>(c, n, fp, (*np).u.recv, (*np).u.args, (*np).u.mid, (*np).u.flags, (*np).u.line, &mut (*np).u.cc)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_super(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_super_struct>;
    unsafe {

    EVAL_node_super::<DynList>(c, n, fp, (*np).u.args, (*np).u.line, (*np).u.ci, (*np).u.zsuper)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_yield(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_yield_struct>;
    unsafe {

    EVAL_node_yield::<DynList>(c, n, fp, (*np).u.args)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_yield_x(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_yield_x_struct>;
    unsafe {

    EVAL_node_yield_x::<DynList>(c, n, fp, (*np).u.args, (*np).u.ci)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_block_given(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_block_given_struct>;
    unsafe {

    EVAL_node_block_given(c, n, fp)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_lambda(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_lambda_struct>;
    unsafe {

    EVAL_node_lambda(c, n, fp, (*np).u.blk)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_opt_plus(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_opt_plus_struct>;
    unsafe {
    let v_a = <Dyn as Disp>::call(c, (*np).u.a, fp)?;
    let v_b = <Dyn as Disp>::call(c, (*np).u.b, fp)?;
    EVAL_node_opt_plus(c, n, fp, v_a, v_b, (*np).u.line, &mut (*np).u.cc)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_opt_minus(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_opt_minus_struct>;
    unsafe {
    let v_a = <Dyn as Disp>::call(c, (*np).u.a, fp)?;
    let v_b = <Dyn as Disp>::call(c, (*np).u.b, fp)?;
    EVAL_node_opt_minus(c, n, fp, v_a, v_b, (*np).u.line, &mut (*np).u.cc)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_opt_mul(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_opt_mul_struct>;
    unsafe {
    let v_a = <Dyn as Disp>::call(c, (*np).u.a, fp)?;
    let v_b = <Dyn as Disp>::call(c, (*np).u.b, fp)?;
    EVAL_node_opt_mul(c, n, fp, v_a, v_b, (*np).u.line, &mut (*np).u.cc)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_opt_div(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_opt_div_struct>;
    unsafe {
    let v_a = <Dyn as Disp>::call(c, (*np).u.a, fp)?;
    let v_b = <Dyn as Disp>::call(c, (*np).u.b, fp)?;
    EVAL_node_opt_div(c, n, fp, v_a, v_b, (*np).u.line, &mut (*np).u.cc)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_opt_pow(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_opt_pow_struct>;
    unsafe {
    let v_a = <Dyn as Disp>::call(c, (*np).u.a, fp)?;
    let v_b = <Dyn as Disp>::call(c, (*np).u.b, fp)?;
    EVAL_node_opt_pow(c, n, fp, v_a, v_b, (*np).u.line, &mut (*np).u.cc)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_opt_mod(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_opt_mod_struct>;
    unsafe {
    let v_a = <Dyn as Disp>::call(c, (*np).u.a, fp)?;
    let v_b = <Dyn as Disp>::call(c, (*np).u.b, fp)?;
    EVAL_node_opt_mod(c, n, fp, v_a, v_b, (*np).u.line, &mut (*np).u.cc)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_opt_lt(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_opt_lt_struct>;
    unsafe {
    let v_a = <Dyn as Disp>::call(c, (*np).u.a, fp)?;
    let v_b = <Dyn as Disp>::call(c, (*np).u.b, fp)?;
    EVAL_node_opt_lt(c, n, fp, v_a, v_b, (*np).u.line, &mut (*np).u.cc)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_opt_le(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_opt_le_struct>;
    unsafe {
    let v_a = <Dyn as Disp>::call(c, (*np).u.a, fp)?;
    let v_b = <Dyn as Disp>::call(c, (*np).u.b, fp)?;
    EVAL_node_opt_le(c, n, fp, v_a, v_b, (*np).u.line, &mut (*np).u.cc)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_opt_gt(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_opt_gt_struct>;
    unsafe {
    let v_a = <Dyn as Disp>::call(c, (*np).u.a, fp)?;
    let v_b = <Dyn as Disp>::call(c, (*np).u.b, fp)?;
    EVAL_node_opt_gt(c, n, fp, v_a, v_b, (*np).u.line, &mut (*np).u.cc)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_opt_ge(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_opt_ge_struct>;
    unsafe {
    let v_a = <Dyn as Disp>::call(c, (*np).u.a, fp)?;
    let v_b = <Dyn as Disp>::call(c, (*np).u.b, fp)?;
    EVAL_node_opt_ge(c, n, fp, v_a, v_b, (*np).u.line, &mut (*np).u.cc)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_opt_eq(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_opt_eq_struct>;
    unsafe {
    let v_a = <Dyn as Disp>::call(c, (*np).u.a, fp)?;
    let v_b = <Dyn as Disp>::call(c, (*np).u.b, fp)?;
    EVAL_node_opt_eq(c, n, fp, v_a, v_b, (*np).u.line, &mut (*np).u.cc)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_opt_neq(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_opt_neq_struct>;
    unsafe {
    let v_a = <Dyn as Disp>::call(c, (*np).u.a, fp)?;
    let v_b = <Dyn as Disp>::call(c, (*np).u.b, fp)?;
    EVAL_node_opt_neq(c, n, fp, v_a, v_b, (*np).u.line, &mut (*np).u.cc)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_opt_ltlt(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_opt_ltlt_struct>;
    unsafe {
    let v_a = <Dyn as Disp>::call(c, (*np).u.a, fp)?;
    let v_b = <Dyn as Disp>::call(c, (*np).u.b, fp)?;
    EVAL_node_opt_ltlt(c, n, fp, v_a, v_b, (*np).u.line, &mut (*np).u.cc)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_opt_gtgt(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_opt_gtgt_struct>;
    unsafe {
    let v_a = <Dyn as Disp>::call(c, (*np).u.a, fp)?;
    let v_b = <Dyn as Disp>::call(c, (*np).u.b, fp)?;
    EVAL_node_opt_gtgt(c, n, fp, v_a, v_b, (*np).u.line, &mut (*np).u.cc)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_opt_xor(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_opt_xor_struct>;
    unsafe {
    let v_a = <Dyn as Disp>::call(c, (*np).u.a, fp)?;
    let v_b = <Dyn as Disp>::call(c, (*np).u.b, fp)?;
    EVAL_node_opt_xor(c, n, fp, v_a, v_b, (*np).u.line, &mut (*np).u.cc)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_opt_and(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_opt_and_struct>;
    unsafe {
    let v_a = <Dyn as Disp>::call(c, (*np).u.a, fp)?;
    let v_b = <Dyn as Disp>::call(c, (*np).u.b, fp)?;
    EVAL_node_opt_and(c, n, fp, v_a, v_b, (*np).u.line, &mut (*np).u.cc)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_opt_or(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_opt_or_struct>;
    unsafe {
    let v_a = <Dyn as Disp>::call(c, (*np).u.a, fp)?;
    let v_b = <Dyn as Disp>::call(c, (*np).u.b, fp)?;
    EVAL_node_opt_or(c, n, fp, v_a, v_b, (*np).u.line, &mut (*np).u.cc)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_opt_aref(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_opt_aref_struct>;
    unsafe {
    let v_a = <Dyn as Disp>::call(c, (*np).u.a, fp)?;
    let v_b = <Dyn as Disp>::call(c, (*np).u.b, fp)?;
    EVAL_node_opt_aref(c, n, fp, v_a, v_b, (*np).u.line, &mut (*np).u.cc)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_opt_aset(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_opt_aset_struct>;
    unsafe {
    let v_a = <Dyn as Disp>::call(c, (*np).u.a, fp)?;
    let v_b = <Dyn as Disp>::call(c, (*np).u.b, fp)?;
    let v_v = <Dyn as Disp>::call(c, (*np).u.v, fp)?;
    EVAL_node_opt_aset(c, n, fp, v_a, v_b, v_v, (*np).u.line, &mut (*np).u.cc)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_method_entry(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_method_entry_struct>;
    unsafe {

    EVAL_node_method_entry::<Dyn>(c, n, fp, (*np).u.nlocals, (*np).u.nparams, (*np).u.has_env, (*np).u.line, (*np).u.body)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_def(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_def_struct>;
    unsafe {

    EVAL_node_def(c, n, fp, (*np).u.mid, (*np).u.body)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_sdef(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_sdef_struct>;
    unsafe {
    let v_recv = <Dyn as Disp>::call(c, (*np).u.recv, fp)?;
    EVAL_node_sdef(c, n, fp, v_recv, (*np).u.mid, (*np).u.body)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_class(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_class_struct>;
    unsafe {

    EVAL_node_class::<Dyn, Dyn>(c, n, fp, (*np).u.cpath, (*np).u.name, (*np).u.superclass, (*np).u.body, (*np).u.has_cpath, (*np).u.has_super)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_module(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_module_struct>;
    unsafe {

    EVAL_node_module::<Dyn>(c, n, fp, (*np).u.cpath, (*np).u.name, (*np).u.body, (*np).u.has_cpath)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_sclass(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_sclass_struct>;
    unsafe {
    let v_recv = <Dyn as Disp>::call(c, (*np).u.recv, fp)?;
    EVAL_node_sclass(c, n, fp, v_recv, (*np).u.body)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_alias(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_alias_struct>;
    unsafe {
    let v_newname = <Dyn as Disp>::call(c, (*np).u.newname, fp)?;
    let v_oldname = <Dyn as Disp>::call(c, (*np).u.oldname, fp)?;
    EVAL_node_alias(c, n, fp, v_newname, v_oldname)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_valias(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_valias_struct>;
    unsafe {

    EVAL_node_valias(c, n, fp, (*np).u.newname, (*np).u.oldname)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_undef(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_undef_struct>;
    unsafe {

    EVAL_node_undef::<DynList>(c, n, fp, (*np).u.names)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_masgn(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_masgn_struct>;
    unsafe {
    let v_rhs = <Dyn as Disp>::call(c, (*np).u.rhs, fp)?;
    EVAL_node_masgn::<DynList>(c, n, fp, v_rhs, (*np).u.tmp, (*np).u.targets, (*np).u.splat_rhs)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_masgn_elem(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_masgn_elem_struct>;
    unsafe {

    EVAL_node_masgn_elem(c, n, fp, (*np).u.tmp, (*np).u.idx, (*np).u.kind, (*np).u.post)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_unsupported(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_unsupported_struct>;
    unsafe {

    EVAL_node_unsupported(c, n, fp, (*np).u.what)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_special(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let np = n as *mut NodeOf<node_special_struct>;
    unsafe {

    EVAL_node_special::<Dyn>(c, n, fp, (*np).u.kind, (*np).u.arg)
    }
}

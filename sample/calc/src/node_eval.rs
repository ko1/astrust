// This file is auto-generated from node.def by astrustgen. Do not edit.
// calc — toy ASTrust sample: integer arithmetic with one accumulator variable.
use crate::*;










#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_num(c: &mut Ctx, n: NodePtr, v: i64) -> i64 {
    v
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_add<D_l: Disp, D_r: Disp>(c: &mut Ctx, n: NodePtr, l: NodePtr, r: NodePtr) -> i64 {
    <D_l as Disp>::call(c, l).wrapping_add(<D_r as Disp>::call(c, r))
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_sub<D_l: Disp, D_r: Disp>(c: &mut Ctx, n: NodePtr, l: NodePtr, r: NodePtr) -> i64 {
    <D_l as Disp>::call(c, l).wrapping_sub(<D_r as Disp>::call(c, r))
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_mul<D_l: Disp, D_r: Disp>(c: &mut Ctx, n: NodePtr, l: NodePtr, r: NodePtr) -> i64 {
    <D_l as Disp>::call(c, l).wrapping_mul(<D_r as Disp>::call(c, r))
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_mod(c: &mut Ctx, n: NodePtr, l: i64, r: i64) -> i64 {
    if r == 0 { 0 } else { l % r }
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_var(c: &mut Ctx, n: NodePtr, idx: u32) -> i64 {
    c.vars[idx as usize]
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_set(c: &mut Ctx, n: NodePtr, idx: u32, v: i64) -> i64 {
    c.vars[idx as usize] = v;
    v
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_seq<D_xs: DispList>(c: &mut Ctx, n: NodePtr, xs: NodeList) -> i64 {
    let mut v = 0;
    <D_xs as DispList>::each(c, xs,  |c, i, r| v = r);
    v
}


#[inline(always)]
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
pub fn EVAL_node_times<D_cnt: Disp, D_body: Disp>(c: &mut Ctx, n: NodePtr, cnt: NodePtr, body: NodePtr) -> i64 {
    let k = <D_cnt as Disp>::call(c, cnt);
    let mut v = 0;
    for _ in 0..k { v = <D_body as Disp>::call(c, body); }
    v
}


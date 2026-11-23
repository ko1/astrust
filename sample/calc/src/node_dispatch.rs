// This file is auto-generated from node.def by astrustgen. Do not edit.
#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_num(c: &mut Ctx, n: NodePtr) -> i64 {
    let u = node_u::<node_num_struct>(n);

    EVAL_node_num(c, n, u.v)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_add(c: &mut Ctx, n: NodePtr) -> i64 {
    let u = node_u::<node_add_struct>(n);

    EVAL_node_add::<Dyn, Dyn>(c, n, u.l, u.r)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_sub(c: &mut Ctx, n: NodePtr) -> i64 {
    let u = node_u::<node_sub_struct>(n);

    EVAL_node_sub::<Dyn, Dyn>(c, n, u.l, u.r)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_mul(c: &mut Ctx, n: NodePtr) -> i64 {
    let u = node_u::<node_mul_struct>(n);

    EVAL_node_mul::<Dyn, Dyn>(c, n, u.l, u.r)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_mod(c: &mut Ctx, n: NodePtr) -> i64 {
    let u = node_u::<node_mod_struct>(n);
    let v_l = <Dyn as Disp>::call(c, u.l);
    let v_r = <Dyn as Disp>::call(c, u.r);
    EVAL_node_mod(c, n, v_l, v_r)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_var(c: &mut Ctx, n: NodePtr) -> i64 {
    let u = node_u::<node_var_struct>(n);

    EVAL_node_var(c, n, u.idx)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_set(c: &mut Ctx, n: NodePtr) -> i64 {
    let u = node_u::<node_set_struct>(n);
    let v_v = <Dyn as Disp>::call(c, u.v);
    EVAL_node_set(c, n, u.idx, v_v)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_seq(c: &mut Ctx, n: NodePtr) -> i64 {
    let u = node_u::<node_seq_struct>(n);

    EVAL_node_seq::<DynList>(c, n, u.xs)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_times(c: &mut Ctx, n: NodePtr) -> i64 {
    let u = node_u::<node_times_struct>(n);

    EVAL_node_times::<Dyn, Dyn>(c, n, u.cnt, u.body)
}

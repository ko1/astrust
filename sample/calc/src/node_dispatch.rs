// This file is auto-generated from node.def by astrustgen. Do not edit.
#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_num(c: &mut Ctx, n: NodePtr) -> i64 {
    let np = n as *mut NodeOf<node_num_struct>;
    unsafe {

    EVAL_node_num(c, n, (*np).u.v)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_add(c: &mut Ctx, n: NodePtr) -> i64 {
    let np = n as *mut NodeOf<node_add_struct>;
    unsafe {

    EVAL_node_add::<Dyn, Dyn>(c, n, (*np).u.l, (*np).u.r)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_sub(c: &mut Ctx, n: NodePtr) -> i64 {
    let np = n as *mut NodeOf<node_sub_struct>;
    unsafe {

    EVAL_node_sub::<Dyn, Dyn>(c, n, (*np).u.l, (*np).u.r)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_mul(c: &mut Ctx, n: NodePtr) -> i64 {
    let np = n as *mut NodeOf<node_mul_struct>;
    unsafe {

    EVAL_node_mul::<Dyn, Dyn>(c, n, (*np).u.l, (*np).u.r)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_mod(c: &mut Ctx, n: NodePtr) -> i64 {
    let np = n as *mut NodeOf<node_mod_struct>;
    unsafe {
    let v_l = <Dyn as Disp>::call(c, (*np).u.l);
    let v_r = <Dyn as Disp>::call(c, (*np).u.r);
    EVAL_node_mod(c, n, v_l, v_r)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_var(c: &mut Ctx, n: NodePtr) -> i64 {
    let np = n as *mut NodeOf<node_var_struct>;
    unsafe {

    EVAL_node_var(c, n, (*np).u.idx)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_set(c: &mut Ctx, n: NodePtr) -> i64 {
    let np = n as *mut NodeOf<node_set_struct>;
    unsafe {
    let v_v = <Dyn as Disp>::call(c, (*np).u.v);
    EVAL_node_set(c, n, (*np).u.idx, v_v)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_seq(c: &mut Ctx, n: NodePtr) -> i64 {
    let np = n as *mut NodeOf<node_seq_struct>;
    unsafe {

    EVAL_node_seq::<DynList>(c, n, (*np).u.xs)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_times(c: &mut Ctx, n: NodePtr) -> i64 {
    let np = n as *mut NodeOf<node_times_struct>;
    unsafe {

    EVAL_node_times::<Dyn, Dyn>(c, n, (*np).u.cnt, (*np).u.body)
    }
}

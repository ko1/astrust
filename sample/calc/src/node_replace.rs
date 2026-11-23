// This file is auto-generated from node.def by astrustgen. Do not edit.
#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_num(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_num_struct>(n);

}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_add(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_add_struct>(n);
    if u.l == old { u.l = new; }
    if u.r == old { u.r = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_sub(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_sub_struct>(n);
    if u.l == old { u.l = new; }
    if u.r == old { u.r = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_mul(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_mul_struct>(n);
    if u.l == old { u.l = new; }
    if u.r == old { u.r = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_mod(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_mod_struct>(n);
    if u.l == old { u.l = new; }
    if u.r == old { u.r = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_var(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_var_struct>(n);

}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_set(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_set_struct>(n);
    if u.v == old { u.v = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_seq(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_seq_struct>(n);
    if u.xs.contains(&old) { u.xs = Box::leak(u.xs.iter().map(|&x| if x == old { new } else { x }).collect::<Vec<_>>().into_boxed_slice()); }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_times(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = node_u_mut::<node_times_struct>(n);
    if u.cnt == old { u.cnt = new; }
    if u.body == old { u.body = new; }
}

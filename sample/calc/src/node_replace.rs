// This file is auto-generated from node.def by astrustgen. Do not edit.
#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_num(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_num_struct>)).u };

}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_add(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_add_struct>)).u };
    if u.l == old { u.l = new; }
    if u.r == old { u.r = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_sub(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_sub_struct>)).u };
    if u.l == old { u.l = new; }
    if u.r == old { u.r = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_mul(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_mul_struct>)).u };
    if u.l == old { u.l = new; }
    if u.r == old { u.r = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_mod(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_mod_struct>)).u };
    if u.l == old { u.l = new; }
    if u.r == old { u.r = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_var(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_var_struct>)).u };

}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_set(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_set_struct>)).u };
    if u.v == old { u.v = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_seq(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_seq_struct>)).u };
    { let l = u.xs; let p = l.as_ptr() as *mut NodePtr; for i in 0..l.len() { unsafe { if *p.add(i) == old { *p.add(i) = new; } } } }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_times(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_times_struct>)).u };
    if u.cnt == old { u.cnt = new; }
    if u.body == old { u.body = new; }
}

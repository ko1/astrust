// This file is auto-generated from node.def by astrustgen. Do not edit.
#[allow(non_snake_case, unused_unsafe)]
fn HASH_node_num(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_num_struct>)).u };
    let mut h = hash_str("node_num");
    h = hash_merge(h, hash_u64(u.v as u64));
    h
}

#[allow(non_snake_case, unused_unsafe)]
fn HASH_node_add(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_add_struct>)).u };
    let mut h = hash_str("node_add");
    h = hash_merge(h, hash_node(u.l));
    h = hash_merge(h, hash_node(u.r));
    h
}

#[allow(non_snake_case, unused_unsafe)]
fn HASH_node_sub(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_sub_struct>)).u };
    let mut h = hash_str("node_sub");
    h = hash_merge(h, hash_node(u.l));
    h = hash_merge(h, hash_node(u.r));
    h
}

#[allow(non_snake_case, unused_unsafe)]
fn HASH_node_mul(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_mul_struct>)).u };
    let mut h = hash_str("node_mul");
    h = hash_merge(h, hash_node(u.l));
    h = hash_merge(h, hash_node(u.r));
    h
}

#[allow(non_snake_case, unused_unsafe)]
fn HASH_node_mod(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_mod_struct>)).u };
    let mut h = hash_str("node_mod");
    h = hash_merge(h, hash_node(u.l));
    h = hash_merge(h, hash_node(u.r));
    h
}

#[allow(non_snake_case, unused_unsafe)]
fn HASH_node_var(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_var_struct>)).u };
    let mut h = hash_str("node_var");
    h = hash_merge(h, hash_u64(u.idx as u64));
    h
}

#[allow(non_snake_case, unused_unsafe)]
fn HASH_node_set(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_set_struct>)).u };
    let mut h = hash_str("node_set");
    h = hash_merge(h, hash_u64(u.idx as u64));
    h = hash_merge(h, hash_node(u.v));
    h
}

#[allow(non_snake_case, unused_unsafe)]
fn HASH_node_seq(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_seq_struct>)).u };
    let mut h = hash_str("node_seq");
    h = hash_merge(h, hash_node_list(u.xs));
    h
}

#[allow(non_snake_case, unused_unsafe)]
fn HASH_node_times(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_times_struct>)).u };
    let mut h = hash_str("node_times");
    h = hash_merge(h, hash_node(u.cnt));
    h = hash_merge(h, hash_node(u.body));
    h
}

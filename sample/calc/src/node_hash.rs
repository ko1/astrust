// This file is auto-generated from node.def by astrustgen. Do not edit.
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_num(n: NodePtr) -> u64 {
    let u = node_u::<node_num_struct>(n);
    let mut h = hash_str("node_num");
    h = hash_merge(h, hash_u64(u.v as u64));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_num(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_num_struct>(n);
    let mut h = hash_str("node_num");

    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_add(n: NodePtr) -> u64 {
    let u = node_u::<node_add_struct>(n);
    let mut h = hash_str("node_add");
    h = hash_merge(h, hash_node(u.l));
    h = hash_merge(h, hash_node(u.r));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_add(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_add_struct>(n);
    let mut h = hash_str("node_add");
    h = hash_merge(h, profile_node(pc, u.l));
    h = hash_merge(h, profile_node(pc, u.r));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_sub(n: NodePtr) -> u64 {
    let u = node_u::<node_sub_struct>(n);
    let mut h = hash_str("node_sub");
    h = hash_merge(h, hash_node(u.l));
    h = hash_merge(h, hash_node(u.r));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_sub(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_sub_struct>(n);
    let mut h = hash_str("node_sub");
    h = hash_merge(h, profile_node(pc, u.l));
    h = hash_merge(h, profile_node(pc, u.r));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_mul(n: NodePtr) -> u64 {
    let u = node_u::<node_mul_struct>(n);
    let mut h = hash_str("node_mul");
    h = hash_merge(h, hash_node(u.l));
    h = hash_merge(h, hash_node(u.r));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_mul(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_mul_struct>(n);
    let mut h = hash_str("node_mul");
    h = hash_merge(h, profile_node(pc, u.l));
    h = hash_merge(h, profile_node(pc, u.r));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_mod(n: NodePtr) -> u64 {
    let u = node_u::<node_mod_struct>(n);
    let mut h = hash_str("node_mod");
    h = hash_merge(h, hash_node(u.l));
    h = hash_merge(h, hash_node(u.r));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_mod(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_mod_struct>(n);
    let mut h = hash_str("node_mod");
    h = hash_merge(h, profile_node(pc, u.l));
    h = hash_merge(h, profile_node(pc, u.r));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_var(n: NodePtr) -> u64 {
    let u = node_u::<node_var_struct>(n);
    let mut h = hash_str("node_var");
    h = hash_merge(h, hash_u64(u.idx as u64));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_var(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_var_struct>(n);
    let mut h = hash_str("node_var");

    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_set(n: NodePtr) -> u64 {
    let u = node_u::<node_set_struct>(n);
    let mut h = hash_str("node_set");
    h = hash_merge(h, hash_u64(u.idx as u64));
    h = hash_merge(h, hash_node(u.v));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_set(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_set_struct>(n);
    let mut h = hash_str("node_set");
    h = hash_merge(h, profile_node(pc, u.v));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_seq(n: NodePtr) -> u64 {
    let u = node_u::<node_seq_struct>(n);
    let mut h = hash_str("node_seq");
    h = hash_merge(h, hash_node_list(u.xs));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_seq(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_seq_struct>(n);
    let mut h = hash_str("node_seq");
    for x in u.xs.iter() { h = hash_merge(h, profile_node(pc, *x)); }
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_times(n: NodePtr) -> u64 {
    let u = node_u::<node_times_struct>(n);
    let mut h = hash_str("node_times");
    h = hash_merge(h, hash_node(u.cnt));
    h = hash_merge(h, hash_node(u.body));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces, unused_parens)]
fn PROF_node_times(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let u = node_u::<node_times_struct>(n);
    let mut h = hash_str("node_times");
    h = hash_merge(h, profile_node(pc, u.cnt));
    h = hash_merge(h, profile_node(pc, u.body));
    h
}


// This file is auto-generated from node.def by astrustgen. Do not edit.
#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_num(out: &mut String, n: NodePtr) {
    let u = node_u::<node_num_struct>(n);
    out.push_str("(node_num");
    out.push(' '); out.push_str(&format!("{:?}", u.v));
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_add(out: &mut String, n: NodePtr) {
    let u = node_u::<node_add_struct>(n);
    out.push_str("(node_add");
    out.push(' '); dump_node(out, u.l);
    out.push(' '); dump_node(out, u.r);
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_sub(out: &mut String, n: NodePtr) {
    let u = node_u::<node_sub_struct>(n);
    out.push_str("(node_sub");
    out.push(' '); dump_node(out, u.l);
    out.push(' '); dump_node(out, u.r);
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_mul(out: &mut String, n: NodePtr) {
    let u = node_u::<node_mul_struct>(n);
    out.push_str("(node_mul");
    out.push(' '); dump_node(out, u.l);
    out.push(' '); dump_node(out, u.r);
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_mod(out: &mut String, n: NodePtr) {
    let u = node_u::<node_mod_struct>(n);
    out.push_str("(node_mod");
    out.push(' '); dump_node(out, u.l);
    out.push(' '); dump_node(out, u.r);
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_var(out: &mut String, n: NodePtr) {
    let u = node_u::<node_var_struct>(n);
    out.push_str("(node_var");
    out.push(' '); out.push_str(&format!("{:?}", u.idx));
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_set(out: &mut String, n: NodePtr) {
    let u = node_u::<node_set_struct>(n);
    out.push_str("(node_set");
    out.push(' '); out.push_str(&format!("{:?}", u.idx));
    out.push(' '); dump_node(out, u.v);
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_seq(out: &mut String, n: NodePtr) {
    let u = node_u::<node_seq_struct>(n);
    out.push_str("(node_seq");
    out.push(' '); out.push('['); for (i, x) in u.xs.iter().enumerate() { if i > 0 { out.push(' '); } dump_node(out, *x); } out.push(']');
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_times(out: &mut String, n: NodePtr) {
    let u = node_u::<node_times_struct>(n);
    out.push_str("(node_times");
    out.push(' '); dump_node(out, u.cnt);
    out.push(' '); dump_node(out, u.body);
    out.push(')');
}

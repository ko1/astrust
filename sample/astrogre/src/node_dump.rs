// This file is auto-generated from node.def by astrustgen. Do not edit.
#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_re_succ(out: &mut String, n: NodePtr) {
    let u = node_u::<node_re_succ_struct>(n);
    out.push_str("(node_re_succ");

    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_re_sub_succ(out: &mut String, n: NodePtr) {
    let u = node_u::<node_re_sub_succ_struct>(n);
    out.push_str("(node_re_sub_succ");

    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_re_sub_succ_at(out: &mut String, n: NodePtr) {
    let u = node_u::<node_re_sub_succ_at_struct>(n);
    out.push_str("(node_re_sub_succ_at");

    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_re_byte(out: &mut String, n: NodePtr) {
    let u = node_u::<node_re_byte_struct>(n);
    out.push_str("(node_re_byte");
    out.push(' '); out.push_str(&format!("{:?}", u.b));
    out.push(' '); dump_node(out, u.next);
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_re_lit(out: &mut String, n: NodePtr) {
    let u = node_u::<node_re_lit_struct>(n);
    out.push_str("(node_re_lit");
    out.push(' '); out.push_str(&format!("{:?}", String::from_utf8_lossy(u.bytes)));
    out.push(' '); dump_node(out, u.next);
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_re_one(out: &mut String, n: NodePtr) {
    let u = node_u::<node_re_one_struct>(n);
    out.push_str("(node_re_one");
    out.push(' '); out.push_str(&format!("{:?}", u.kind));
    out.push(' '); out.push_str(&format!("{:?}", u.ch));
    out.push(' '); out.push_str("<cls>");
    out.push(' '); dump_node(out, u.next);
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_re_grapheme(out: &mut String, n: NodePtr) {
    let u = node_u::<node_re_grapheme_struct>(n);
    out.push_str("(node_re_grapheme");
    out.push(' '); dump_node(out, u.next);
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_re_rep1(out: &mut String, n: NodePtr) {
    let u = node_u::<node_re_rep1_struct>(n);
    out.push_str("(node_re_rep1");
    out.push(' '); out.push_str(&format!("{:?}", u.kind));
    out.push(' '); out.push_str(&format!("{:?}", u.ch));
    out.push(' '); out.push_str("<cls>");
    out.push(' '); out.push_str(&format!("{:?}", u.min));
    out.push(' '); out.push_str(&format!("{:?}", u.max));
    out.push(' '); out.push_str(&format!("{:?}", u.greedy));
    out.push(' '); out.push_str(&format!("{:?}", u.nb));
    out.push(' '); dump_node(out, u.next);
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_re_assert(out: &mut String, n: NodePtr) {
    let u = node_u::<node_re_assert_struct>(n);
    out.push_str("(node_re_assert");
    out.push(' '); out.push_str(&format!("{:?}", u.kind));
    out.push(' '); dump_node(out, u.next);
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_re_keep(out: &mut String, n: NodePtr) {
    let u = node_u::<node_re_keep_struct>(n);
    out.push_str("(node_re_keep");
    out.push(' '); dump_node(out, u.next);
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_re_open(out: &mut String, n: NodePtr) {
    let u = node_u::<node_re_open_struct>(n);
    out.push_str("(node_re_open");
    out.push(' '); out.push_str(&format!("{:?}", u.g));
    out.push(' '); dump_node(out, u.next);
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_re_open_nr(out: &mut String, n: NodePtr) {
    let u = node_u::<node_re_open_nr_struct>(n);
    out.push_str("(node_re_open_nr");
    out.push(' '); out.push_str(&format!("{:?}", u.g));
    out.push(' '); dump_node(out, u.next);
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_re_close(out: &mut String, n: NodePtr) {
    let u = node_u::<node_re_close_struct>(n);
    out.push_str("(node_re_close");
    out.push(' '); out.push_str(&format!("{:?}", u.g));
    out.push(' '); dump_node(out, u.next);
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_re_backref(out: &mut String, n: NodePtr) {
    let u = node_u::<node_re_backref_struct>(n);
    out.push_str("(node_re_backref");
    out.push(' '); out.push_str("<groups>");
    out.push(' '); out.push_str(&format!("{:?}", u.icase));
    out.push(' '); dump_node(out, u.next);
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_re_open_rec(out: &mut String, n: NodePtr) {
    let u = node_u::<node_re_open_rec_struct>(n);
    out.push_str("(node_re_open_rec");
    out.push(' '); out.push_str(&format!("{:?}", u.g));
    out.push(' '); dump_node(out, u.next);
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_re_close_rec(out: &mut String, n: NodePtr) {
    let u = node_u::<node_re_close_rec_struct>(n);
    out.push_str("(node_re_close_rec");
    out.push(' '); out.push_str(&format!("{:?}", u.g));
    out.push(' '); dump_node(out, u.next);
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_re_call(out: &mut String, n: NodePtr) {
    let u = node_u::<node_re_call_struct>(n);
    out.push_str("(node_re_call");
    out.push(' '); out.push_str(&format!("{:?}", u.g));
    out.push(' '); dump_node(out, u.next);
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_re_ret(out: &mut String, n: NodePtr) {
    let u = node_u::<node_re_ret_struct>(n);
    out.push_str("(node_re_ret");

    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_re_backref_lv(out: &mut String, n: NodePtr) {
    let u = node_u::<node_re_backref_lv_struct>(n);
    out.push_str("(node_re_backref_lv");
    out.push(' '); out.push_str("<groups>");
    out.push(' '); out.push_str(&format!("{:?}", u.icase));
    out.push(' '); out.push_str(&format!("{:?}", u.nest));
    out.push(' '); dump_node(out, u.next);
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_re_cond(out: &mut String, n: NodePtr) {
    let u = node_u::<node_re_cond_struct>(n);
    out.push_str("(node_re_cond");
    out.push(' '); out.push_str("<groups>");
    out.push(' '); dump_node(out, u.yes);
    out.push(' '); dump_node(out, u.no);
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_re_alt(out: &mut String, n: NodePtr) {
    let u = node_u::<node_re_alt_struct>(n);
    out.push_str("(node_re_alt");
    out.push(' '); dump_node(out, u.a);
    out.push(' '); dump_node(out, u.b);
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_re_opt(out: &mut String, n: NodePtr) {
    let u = node_u::<node_re_opt_struct>(n);
    out.push_str("(node_re_opt");
    out.push(' '); dump_node(out, u.body);
    out.push(' '); dump_node(out, u.skip);
    out.push(' '); out.push_str(&format!("{:?}", u.greedy));
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_re_rep(out: &mut String, n: NodePtr) {
    let u = node_u::<node_re_rep_struct>(n);
    out.push_str("(node_re_rep");
    out.push(' '); dump_node(out, u.body);
    out.push(' '); dump_node(out, u.outer);
    out.push(' '); out.push_str(&format!("{:?}", u.min));
    out.push(' '); out.push_str(&format!("{:?}", u.max));
    out.push(' '); out.push_str(&format!("{:?}", u.greedy));
    out.push(' '); out.push_str(&format!("{:?}", u.mem));
    out.push(' '); out.push_str(&format!("{:?}", u.shape));
    out.push(' '); out.push_str(&format!("{:?}", u.cost));
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_re_rep_cont(out: &mut String, n: NodePtr) {
    let u = node_u::<node_re_rep_cont_struct>(n);
    out.push_str("(node_re_rep_cont");

    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_re_look(out: &mut String, n: NodePtr) {
    let u = node_u::<node_re_look_struct>(n);
    out.push_str("(node_re_look");
    out.push(' '); out.push_str(&format!("{:?}", u.kind));
    out.push(' '); dump_node(out, u.body);
    out.push(' '); dump_node(out, u.next);
    out.push(' '); out.push_str(&format!("{:?}", u.minlen));
    out.push(' '); out.push_str(&format!("{:?}", u.maxlen));
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_re_absent(out: &mut String, n: NodePtr) {
    let u = node_u::<node_re_absent_struct>(n);
    out.push_str("(node_re_absent");
    out.push(' '); dump_node(out, u.body);
    out.push(' '); dump_node(out, u.next);
    out.push(' '); out.push_str(&format!("{:?}", u.minlen));
    out.push(' '); out.push_str(&format!("{:?}", u.maxlen));
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_search(out: &mut String, n: NodePtr) {
    let u = node_u::<node_search_struct>(n);
    out.push_str("(node_search");
    out.push(' '); dump_node(out, u.body);
    out.push(' '); out.push_str(&format!("{:?}", u.anchor));
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_search_memchr(out: &mut String, n: NodePtr) {
    let u = node_u::<node_search_memchr_struct>(n);
    out.push_str("(node_search_memchr");
    out.push(' '); dump_node(out, u.body);
    out.push(' '); out.push_str(&format!("{:?}", u.b));
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_search_memmem(out: &mut String, n: NodePtr) {
    let u = node_u::<node_search_memmem_struct>(n);
    out.push_str("(node_search_memmem");
    out.push(' '); dump_node(out, u.body);
    out.push(' '); out.push_str(&format!("{:?}", String::from_utf8_lossy(u.needle)));
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_search_byteset(out: &mut String, n: NodePtr) {
    let u = node_u::<node_search_byteset_struct>(n);
    out.push_str("(node_search_byteset");
    out.push(' '); dump_node(out, u.body);
    out.push(' '); out.push_str("<set>");
    out.push(')');
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_match_at(out: &mut String, n: NodePtr) {
    let u = node_u::<node_match_at_struct>(n);
    out.push_str("(node_match_at");
    out.push(' '); dump_node(out, u.body);
    out.push(')');
}

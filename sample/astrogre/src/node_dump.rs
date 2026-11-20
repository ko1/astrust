// This file is auto-generated from node.def by astrustgen. Do not edit.
#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_re_succ(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_re_succ_struct>;
    unsafe {
    out.push_str("(node_re_succ");

    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_re_sub_succ(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_re_sub_succ_struct>;
    unsafe {
    out.push_str("(node_re_sub_succ");

    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_re_sub_succ_at(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_re_sub_succ_at_struct>;
    unsafe {
    out.push_str("(node_re_sub_succ_at");

    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_re_byte(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_re_byte_struct>;
    unsafe {
    out.push_str("(node_re_byte");
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.b));
    out.push(' '); dump_node(out, (*np).u.next);
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_re_lit(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_re_lit_struct>;
    unsafe {
    out.push_str("(node_re_lit");
    out.push(' '); out.push_str(&format!("{:?}", String::from_utf8_lossy((*np).u.bytes)));
    out.push(' '); dump_node(out, (*np).u.next);
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_re_one(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_re_one_struct>;
    unsafe {
    out.push_str("(node_re_one");
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.kind));
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.ch));
    out.push(' '); out.push_str("<cls>");
    out.push(' '); dump_node(out, (*np).u.next);
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_re_grapheme(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_re_grapheme_struct>;
    unsafe {
    out.push_str("(node_re_grapheme");
    out.push(' '); dump_node(out, (*np).u.next);
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_re_rep1(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_re_rep1_struct>;
    unsafe {
    out.push_str("(node_re_rep1");
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.kind));
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.ch));
    out.push(' '); out.push_str("<cls>");
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.min));
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.max));
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.greedy));
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.nb));
    out.push(' '); dump_node(out, (*np).u.next);
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_re_assert(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_re_assert_struct>;
    unsafe {
    out.push_str("(node_re_assert");
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.kind));
    out.push(' '); dump_node(out, (*np).u.next);
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_re_keep(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_re_keep_struct>;
    unsafe {
    out.push_str("(node_re_keep");
    out.push(' '); dump_node(out, (*np).u.next);
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_re_open(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_re_open_struct>;
    unsafe {
    out.push_str("(node_re_open");
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.g));
    out.push(' '); dump_node(out, (*np).u.next);
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_re_open_nr(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_re_open_nr_struct>;
    unsafe {
    out.push_str("(node_re_open_nr");
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.g));
    out.push(' '); dump_node(out, (*np).u.next);
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_re_close(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_re_close_struct>;
    unsafe {
    out.push_str("(node_re_close");
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.g));
    out.push(' '); dump_node(out, (*np).u.next);
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_re_backref(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_re_backref_struct>;
    unsafe {
    out.push_str("(node_re_backref");
    out.push(' '); out.push_str("<groups>");
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.icase));
    out.push(' '); dump_node(out, (*np).u.next);
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_re_cond(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_re_cond_struct>;
    unsafe {
    out.push_str("(node_re_cond");
    out.push(' '); out.push_str("<groups>");
    out.push(' '); dump_node(out, (*np).u.yes);
    out.push(' '); dump_node(out, (*np).u.no);
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_re_alt(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_re_alt_struct>;
    unsafe {
    out.push_str("(node_re_alt");
    out.push(' '); dump_node(out, (*np).u.a);
    out.push(' '); dump_node(out, (*np).u.b);
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_re_opt(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_re_opt_struct>;
    unsafe {
    out.push_str("(node_re_opt");
    out.push(' '); dump_node(out, (*np).u.body);
    out.push(' '); dump_node(out, (*np).u.skip);
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.greedy));
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_re_rep(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_re_rep_struct>;
    unsafe {
    out.push_str("(node_re_rep");
    out.push(' '); dump_node(out, (*np).u.body);
    out.push(' '); dump_node(out, (*np).u.outer);
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.min));
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.max));
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.greedy));
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.mem));
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.shape));
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.cost));
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_re_rep_cont(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_re_rep_cont_struct>;
    unsafe {
    out.push_str("(node_re_rep_cont");

    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_re_look(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_re_look_struct>;
    unsafe {
    out.push_str("(node_re_look");
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.kind));
    out.push(' '); dump_node(out, (*np).u.body);
    out.push(' '); dump_node(out, (*np).u.next);
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.minlen));
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.maxlen));
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_search(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_search_struct>;
    unsafe {
    out.push_str("(node_search");
    out.push(' '); dump_node(out, (*np).u.body);
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.anchor));
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_search_memchr(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_search_memchr_struct>;
    unsafe {
    out.push_str("(node_search_memchr");
    out.push(' '); dump_node(out, (*np).u.body);
    out.push(' '); out.push_str(&format!("{:?}", (*np).u.b));
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_search_memmem(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_search_memmem_struct>;
    unsafe {
    out.push_str("(node_search_memmem");
    out.push(' '); dump_node(out, (*np).u.body);
    out.push(' '); out.push_str(&format!("{:?}", String::from_utf8_lossy((*np).u.needle)));
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_search_byteset(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_search_byteset_struct>;
    unsafe {
    out.push_str("(node_search_byteset");
    out.push(' '); dump_node(out, (*np).u.body);
    out.push(' '); out.push_str("<set>");
    out.push(')');
    }
}

#[allow(non_snake_case, unused_unsafe, unused_variables)]
fn DUMP_node_match_at(out: &mut String, n: NodePtr) {
    let np = n as *mut NodeOf<node_match_at_struct>;
    unsafe {
    out.push_str("(node_match_at");
    out.push(' '); dump_node(out, (*np).u.body);
    out.push(')');
    }
}

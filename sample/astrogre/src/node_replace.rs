// This file is auto-generated from node.def by astrustgen. Do not edit.
#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_re_succ(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_re_succ_struct>)).u };

}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_re_sub_succ(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_re_sub_succ_struct>)).u };

}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_re_sub_succ_at(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_re_sub_succ_at_struct>)).u };

}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_re_byte(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_re_byte_struct>)).u };
    if u.next == old { u.next = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_re_lit(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_re_lit_struct>)).u };
    if u.next == old { u.next = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_re_one(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_re_one_struct>)).u };
    if u.next == old { u.next = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_re_grapheme(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_re_grapheme_struct>)).u };
    if u.next == old { u.next = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_re_rep1(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_re_rep1_struct>)).u };
    if u.next == old { u.next = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_re_assert(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_re_assert_struct>)).u };
    if u.next == old { u.next = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_re_keep(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_re_keep_struct>)).u };
    if u.next == old { u.next = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_re_open(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_re_open_struct>)).u };
    if u.next == old { u.next = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_re_open_nr(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_re_open_nr_struct>)).u };
    if u.next == old { u.next = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_re_close(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_re_close_struct>)).u };
    if u.next == old { u.next = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_re_backref(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_re_backref_struct>)).u };
    if u.next == old { u.next = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_re_cond(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_re_cond_struct>)).u };
    if u.yes == old { u.yes = new; }
    if u.no == old { u.no = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_re_alt(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_re_alt_struct>)).u };
    if u.a == old { u.a = new; }
    if u.b == old { u.b = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_re_opt(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_re_opt_struct>)).u };
    if u.body == old { u.body = new; }
    if u.skip == old { u.skip = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_re_rep(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_re_rep_struct>)).u };
    if u.body == old { u.body = new; }
    if u.outer == old { u.outer = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_re_rep_cont(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_re_rep_cont_struct>)).u };

}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_re_look(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_re_look_struct>)).u };
    if u.body == old { u.body = new; }
    if u.next == old { u.next = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_search(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_search_struct>)).u };
    if u.body == old { u.body = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_search_memchr(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_search_memchr_struct>)).u };
    if u.body == old { u.body = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_search_memmem(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_search_memmem_struct>)).u };
    if u.body == old { u.body = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_search_byteset(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_search_byteset_struct>)).u };
    if u.body == old { u.body = new; }
}

#[allow(non_snake_case, unused_variables)]
fn REPLACE_node_match_at(n: NodePtr, old: NodePtr, new: NodePtr) {
    let u = unsafe { &mut (*(n as *mut NodeOf<node_match_at_struct>)).u };
    if u.body == old { u.body = new; }
}

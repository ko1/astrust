// This file is auto-generated from node.def by astrustgen. Do not edit.
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_re_succ(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_re_succ_struct>)).u };
    let mut h = hash_str("node_re_succ");

    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces)]
fn PROF_node_re_succ(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let np = n as *mut NodeOf<node_re_succ_struct>;
    let u = unsafe { &(*np).u };
    let mut h = hash_str("node_re_succ");

    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_re_sub_succ(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_re_sub_succ_struct>)).u };
    let mut h = hash_str("node_re_sub_succ");

    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces)]
fn PROF_node_re_sub_succ(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let np = n as *mut NodeOf<node_re_sub_succ_struct>;
    let u = unsafe { &(*np).u };
    let mut h = hash_str("node_re_sub_succ");

    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_re_sub_succ_at(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_re_sub_succ_at_struct>)).u };
    let mut h = hash_str("node_re_sub_succ_at");

    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces)]
fn PROF_node_re_sub_succ_at(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let np = n as *mut NodeOf<node_re_sub_succ_at_struct>;
    let u = unsafe { &(*np).u };
    let mut h = hash_str("node_re_sub_succ_at");

    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_re_byte(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_re_byte_struct>)).u };
    let mut h = hash_str("node_re_byte");
    h = hash_merge(h, hash_u64(u.b as u64));
    h = hash_merge(h, hash_node(u.next));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces)]
fn PROF_node_re_byte(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let np = n as *mut NodeOf<node_re_byte_struct>;
    let u = unsafe { &(*np).u };
    let mut h = hash_str("node_re_byte");
    h = hash_merge(h, profile_node(pc, u.next));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_re_lit(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_re_lit_struct>)).u };
    let mut h = hash_str("node_re_lit");
    h = hash_merge(h, hash_bytes(u.bytes));
    h = hash_merge(h, hash_node(u.next));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces)]
fn PROF_node_re_lit(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let np = n as *mut NodeOf<node_re_lit_struct>;
    let u = unsafe { &(*np).u };
    let mut h = hash_str("node_re_lit");
    h = hash_merge(h, profile_node(pc, u.next));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_re_one(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_re_one_struct>)).u };
    let mut h = hash_str("node_re_one");
    h = hash_merge(h, hash_u64(u.kind as u64));
    h = hash_merge(h, hash_u64(u.ch as u64));
    h = hash_merge(h, unsafe { (*u.cls).hash });
    h = hash_merge(h, hash_node(u.next));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces)]
fn PROF_node_re_one(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let np = n as *mut NodeOf<node_re_one_struct>;
    let u = unsafe { &(*np).u };
    let mut h = hash_str("node_re_one");
    h = hash_merge(h, profile_node(pc, u.next));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_re_grapheme(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_re_grapheme_struct>)).u };
    let mut h = hash_str("node_re_grapheme");
    h = hash_merge(h, hash_node(u.next));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces)]
fn PROF_node_re_grapheme(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let np = n as *mut NodeOf<node_re_grapheme_struct>;
    let u = unsafe { &(*np).u };
    let mut h = hash_str("node_re_grapheme");
    h = hash_merge(h, profile_node(pc, u.next));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_re_rep1(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_re_rep1_struct>)).u };
    let mut h = hash_str("node_re_rep1");
    h = hash_merge(h, hash_u64(u.kind as u64));
    h = hash_merge(h, hash_u64(u.ch as u64));
    h = hash_merge(h, unsafe { (*u.cls).hash });
    h = hash_merge(h, hash_u64(u.min as u64));
    h = hash_merge(h, hash_u64(u.max as u64));
    h = hash_merge(h, hash_u64(u.greedy as u64));
    h = hash_merge(h, hash_u64(u.nb as u64));
    h = hash_merge(h, hash_node(u.next));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces)]
fn PROF_node_re_rep1(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let np = n as *mut NodeOf<node_re_rep1_struct>;
    let u = unsafe { &(*np).u };
    let mut h = hash_str("node_re_rep1");
    h = hash_merge(h, profile_node(pc, u.next));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_re_assert(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_re_assert_struct>)).u };
    let mut h = hash_str("node_re_assert");
    h = hash_merge(h, hash_u64(u.kind as u64));
    h = hash_merge(h, hash_node(u.next));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces)]
fn PROF_node_re_assert(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let np = n as *mut NodeOf<node_re_assert_struct>;
    let u = unsafe { &(*np).u };
    let mut h = hash_str("node_re_assert");
    h = hash_merge(h, profile_node(pc, u.next));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_re_keep(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_re_keep_struct>)).u };
    let mut h = hash_str("node_re_keep");
    h = hash_merge(h, hash_node(u.next));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces)]
fn PROF_node_re_keep(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let np = n as *mut NodeOf<node_re_keep_struct>;
    let u = unsafe { &(*np).u };
    let mut h = hash_str("node_re_keep");
    h = hash_merge(h, profile_node(pc, u.next));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_re_open(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_re_open_struct>)).u };
    let mut h = hash_str("node_re_open");
    h = hash_merge(h, hash_u64(u.g as u64));
    h = hash_merge(h, hash_node(u.next));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces)]
fn PROF_node_re_open(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let np = n as *mut NodeOf<node_re_open_struct>;
    let u = unsafe { &(*np).u };
    let mut h = hash_str("node_re_open");
    h = hash_merge(h, profile_node(pc, u.next));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_re_open_nr(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_re_open_nr_struct>)).u };
    let mut h = hash_str("node_re_open_nr");
    h = hash_merge(h, hash_u64(u.g as u64));
    h = hash_merge(h, hash_node(u.next));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces)]
fn PROF_node_re_open_nr(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let np = n as *mut NodeOf<node_re_open_nr_struct>;
    let u = unsafe { &(*np).u };
    let mut h = hash_str("node_re_open_nr");
    h = hash_merge(h, profile_node(pc, u.next));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_re_close(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_re_close_struct>)).u };
    let mut h = hash_str("node_re_close");
    h = hash_merge(h, hash_u64(u.g as u64));
    h = hash_merge(h, hash_node(u.next));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces)]
fn PROF_node_re_close(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let np = n as *mut NodeOf<node_re_close_struct>;
    let u = unsafe { &(*np).u };
    let mut h = hash_str("node_re_close");
    h = hash_merge(h, profile_node(pc, u.next));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_re_backref(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_re_backref_struct>)).u };
    let mut h = hash_str("node_re_backref");
    h = hash_merge(h, { let mut h = hash_u64(u.groups.len() as u64); for &g in u.groups.iter() { h = hash_merge(h, hash_u64(g as u64)); } h });
    h = hash_merge(h, hash_u64(u.icase as u64));
    h = hash_merge(h, hash_node(u.next));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces)]
fn PROF_node_re_backref(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let np = n as *mut NodeOf<node_re_backref_struct>;
    let u = unsafe { &(*np).u };
    let mut h = hash_str("node_re_backref");
    h = hash_merge(h, profile_node(pc, u.next));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_re_open_rec(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_re_open_rec_struct>)).u };
    let mut h = hash_str("node_re_open_rec");
    h = hash_merge(h, hash_u64(u.g as u64));
    h = hash_merge(h, hash_node(u.next));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces)]
fn PROF_node_re_open_rec(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let np = n as *mut NodeOf<node_re_open_rec_struct>;
    let u = unsafe { &(*np).u };
    let mut h = hash_str("node_re_open_rec");
    h = hash_merge(h, profile_node(pc, u.next));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_re_close_rec(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_re_close_rec_struct>)).u };
    let mut h = hash_str("node_re_close_rec");
    h = hash_merge(h, hash_u64(u.g as u64));
    h = hash_merge(h, hash_node(u.next));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces)]
fn PROF_node_re_close_rec(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let np = n as *mut NodeOf<node_re_close_rec_struct>;
    let u = unsafe { &(*np).u };
    let mut h = hash_str("node_re_close_rec");
    h = hash_merge(h, profile_node(pc, u.next));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_re_call(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_re_call_struct>)).u };
    let mut h = hash_str("node_re_call");
    h = hash_merge(h, hash_u64(u.g as u64));
    h = hash_merge(h, hash_node(u.next));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces)]
fn PROF_node_re_call(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let np = n as *mut NodeOf<node_re_call_struct>;
    let u = unsafe { &(*np).u };
    let mut h = hash_str("node_re_call");
    h = hash_merge(h, profile_node(pc, u.next));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_re_ret(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_re_ret_struct>)).u };
    let mut h = hash_str("node_re_ret");

    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces)]
fn PROF_node_re_ret(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let np = n as *mut NodeOf<node_re_ret_struct>;
    let u = unsafe { &(*np).u };
    let mut h = hash_str("node_re_ret");

    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_re_backref_lv(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_re_backref_lv_struct>)).u };
    let mut h = hash_str("node_re_backref_lv");
    h = hash_merge(h, { let mut h = hash_u64(u.groups.len() as u64); for &g in u.groups.iter() { h = hash_merge(h, hash_u64(g as u64)); } h });
    h = hash_merge(h, hash_u64(u.icase as u64));
    h = hash_merge(h, hash_u64(u.nest as u64));
    h = hash_merge(h, hash_node(u.next));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces)]
fn PROF_node_re_backref_lv(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let np = n as *mut NodeOf<node_re_backref_lv_struct>;
    let u = unsafe { &(*np).u };
    let mut h = hash_str("node_re_backref_lv");
    h = hash_merge(h, profile_node(pc, u.next));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_re_cond(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_re_cond_struct>)).u };
    let mut h = hash_str("node_re_cond");
    h = hash_merge(h, { let mut h = hash_u64(u.groups.len() as u64); for &g in u.groups.iter() { h = hash_merge(h, hash_u64(g as u64)); } h });
    h = hash_merge(h, hash_node(u.yes));
    h = hash_merge(h, hash_node(u.no));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces)]
fn PROF_node_re_cond(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let np = n as *mut NodeOf<node_re_cond_struct>;
    let u = unsafe { &(*np).u };
    let mut h = hash_str("node_re_cond");
    h = hash_merge(h, profile_node(pc, u.yes));
    h = hash_merge(h, profile_node(pc, u.no));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_re_alt(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_re_alt_struct>)).u };
    let mut h = hash_str("node_re_alt");
    h = hash_merge(h, hash_node(u.a));
    h = hash_merge(h, hash_node(u.b));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces)]
fn PROF_node_re_alt(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let np = n as *mut NodeOf<node_re_alt_struct>;
    let u = unsafe { &(*np).u };
    let mut h = hash_str("node_re_alt");
    h = hash_merge(h, profile_node(pc, u.a));
    h = hash_merge(h, profile_node(pc, u.b));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_re_opt(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_re_opt_struct>)).u };
    let mut h = hash_str("node_re_opt");
    h = hash_merge(h, hash_node(u.body));
    h = hash_merge(h, hash_node(u.skip));
    h = hash_merge(h, hash_u64(u.greedy as u64));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces)]
fn PROF_node_re_opt(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let np = n as *mut NodeOf<node_re_opt_struct>;
    let u = unsafe { &(*np).u };
    let mut h = hash_str("node_re_opt");
    h = hash_merge(h, profile_node(pc, u.body));
    h = hash_merge(h, profile_node(pc, u.skip));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_re_rep(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_re_rep_struct>)).u };
    let mut h = hash_str("node_re_rep");
    h = hash_merge(h, hash_node(u.body));
    h = hash_merge(h, hash_node(u.outer));
    h = hash_merge(h, hash_u64(u.min as u64));
    h = hash_merge(h, hash_u64(u.max as u64));
    h = hash_merge(h, hash_u64(u.greedy as u64));
    h = hash_merge(h, hash_u64(u.mem as u64));
    h = hash_merge(h, hash_u64(u.shape as u64));
    h = hash_merge(h, hash_u64(u.cost as u64));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces)]
fn PROF_node_re_rep(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let np = n as *mut NodeOf<node_re_rep_struct>;
    let u = unsafe { &(*np).u };
    let mut h = hash_str("node_re_rep");
    h = hash_merge(h, profile_node(pc, u.body));
    h = hash_merge(h, profile_node(pc, u.outer));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_re_rep_cont(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_re_rep_cont_struct>)).u };
    let mut h = hash_str("node_re_rep_cont");

    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces)]
fn PROF_node_re_rep_cont(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let np = n as *mut NodeOf<node_re_rep_cont_struct>;
    let u = unsafe { &(*np).u };
    let mut h = hash_str("node_re_rep_cont");

    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_re_look(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_re_look_struct>)).u };
    let mut h = hash_str("node_re_look");
    h = hash_merge(h, hash_u64(u.kind as u64));
    h = hash_merge(h, hash_node(u.body));
    h = hash_merge(h, hash_node(u.next));
    h = hash_merge(h, hash_u64(u.minlen as u64));
    h = hash_merge(h, hash_u64(u.maxlen as u64));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces)]
fn PROF_node_re_look(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let np = n as *mut NodeOf<node_re_look_struct>;
    let u = unsafe { &(*np).u };
    let mut h = hash_str("node_re_look");
    h = hash_merge(h, profile_node(pc, u.body));
    h = hash_merge(h, profile_node(pc, u.next));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_re_absent(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_re_absent_struct>)).u };
    let mut h = hash_str("node_re_absent");
    h = hash_merge(h, hash_node(u.body));
    h = hash_merge(h, hash_node(u.next));
    h = hash_merge(h, hash_u64(u.minlen as u64));
    h = hash_merge(h, hash_u64(u.maxlen as u64));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces)]
fn PROF_node_re_absent(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let np = n as *mut NodeOf<node_re_absent_struct>;
    let u = unsafe { &(*np).u };
    let mut h = hash_str("node_re_absent");
    h = hash_merge(h, profile_node(pc, u.body));
    h = hash_merge(h, profile_node(pc, u.next));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_search(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_search_struct>)).u };
    let mut h = hash_str("node_search");
    h = hash_merge(h, hash_node(u.body));
    h = hash_merge(h, hash_u64(u.anchor as u64));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces)]
fn PROF_node_search(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let np = n as *mut NodeOf<node_search_struct>;
    let u = unsafe { &(*np).u };
    let mut h = hash_str("node_search");
    h = hash_merge(h, profile_node(pc, u.body));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_search_memchr(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_search_memchr_struct>)).u };
    let mut h = hash_str("node_search_memchr");
    h = hash_merge(h, hash_node(u.body));
    h = hash_merge(h, hash_u64(u.b as u64));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces)]
fn PROF_node_search_memchr(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let np = n as *mut NodeOf<node_search_memchr_struct>;
    let u = unsafe { &(*np).u };
    let mut h = hash_str("node_search_memchr");
    h = hash_merge(h, profile_node(pc, u.body));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_search_memmem(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_search_memmem_struct>)).u };
    let mut h = hash_str("node_search_memmem");
    h = hash_merge(h, hash_node(u.body));
    h = hash_merge(h, hash_bytes(u.needle));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces)]
fn PROF_node_search_memmem(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let np = n as *mut NodeOf<node_search_memmem_struct>;
    let u = unsafe { &(*np).u };
    let mut h = hash_str("node_search_memmem");
    h = hash_merge(h, profile_node(pc, u.body));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_search_byteset(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_search_byteset_struct>)).u };
    let mut h = hash_str("node_search_byteset");
    h = hash_merge(h, hash_node(u.body));
    h = hash_merge(h, hash_bytes(unsafe { &*(u.set as *const [u8; 256]) }));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces)]
fn PROF_node_search_byteset(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let np = n as *mut NodeOf<node_search_byteset_struct>;
    let u = unsafe { &(*np).u };
    let mut h = hash_str("node_search_byteset");
    h = hash_merge(h, profile_node(pc, u.body));
    h
}


#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
fn HASH_node_match_at(n: NodePtr) -> u64 {
    let u = unsafe { &(*(n as *mut NodeOf<node_match_at_struct>)).u };
    let mut h = hash_str("node_match_at");
    h = hash_merge(h, hash_node(u.body));
    h
}
#[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut, unused_braces)]
fn PROF_node_match_at(pc: &mut ProfCtx, n: NodePtr) -> u64 {
    let np = n as *mut NodeOf<node_match_at_struct>;
    let u = unsafe { &(*np).u };
    let mut h = hash_str("node_match_at");
    h = hash_merge(h, profile_node(pc, u.body));
    h
}


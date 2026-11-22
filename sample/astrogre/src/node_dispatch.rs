// This file is auto-generated from node.def by astrustgen. Do not edit.
#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_re_succ(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {
    let np = n as *mut NodeOf<node_re_succ_struct>;
    unsafe {

    EVAL_node_re_succ(c, n, pos)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_re_sub_succ(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {
    let np = n as *mut NodeOf<node_re_sub_succ_struct>;
    unsafe {

    EVAL_node_re_sub_succ(c, n, pos)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_re_sub_succ_at(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {
    let np = n as *mut NodeOf<node_re_sub_succ_at_struct>;
    unsafe {

    EVAL_node_re_sub_succ_at(c, n, pos)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_re_byte(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {
    let np = n as *mut NodeOf<node_re_byte_struct>;
    unsafe {

    EVAL_node_re_byte::<Dyn>(c, n, pos, (*np).u.b, (*np).u.next)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_re_lit(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {
    let np = n as *mut NodeOf<node_re_lit_struct>;
    unsafe {

    EVAL_node_re_lit::<Dyn>(c, n, pos, (*np).u.bytes, (*np).u.next)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_re_one(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {
    let np = n as *mut NodeOf<node_re_one_struct>;
    unsafe {

    EVAL_node_re_one::<Dyn>(c, n, pos, (*np).u.kind, (*np).u.ch, (*np).u.cls, (*np).u.next)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_re_grapheme(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {
    let np = n as *mut NodeOf<node_re_grapheme_struct>;
    unsafe {

    EVAL_node_re_grapheme::<Dyn>(c, n, pos, (*np).u.next)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_re_rep1(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {
    let np = n as *mut NodeOf<node_re_rep1_struct>;
    unsafe {

    EVAL_node_re_rep1::<Dyn>(c, n, pos, (*np).u.kind, (*np).u.ch, (*np).u.cls, (*np).u.min, (*np).u.max, (*np).u.greedy, (*np).u.nb, (*np).u.next)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_re_assert(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {
    let np = n as *mut NodeOf<node_re_assert_struct>;
    unsafe {

    EVAL_node_re_assert::<Dyn>(c, n, pos, (*np).u.kind, (*np).u.next)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_re_keep(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {
    let np = n as *mut NodeOf<node_re_keep_struct>;
    unsafe {

    EVAL_node_re_keep::<Dyn>(c, n, pos, (*np).u.next)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_re_open(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {
    let np = n as *mut NodeOf<node_re_open_struct>;
    unsafe {

    EVAL_node_re_open::<Dyn>(c, n, pos, (*np).u.g, (*np).u.next)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_re_open_nr(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {
    let np = n as *mut NodeOf<node_re_open_nr_struct>;
    unsafe {

    EVAL_node_re_open_nr::<Dyn>(c, n, pos, (*np).u.g, (*np).u.next)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_re_close(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {
    let np = n as *mut NodeOf<node_re_close_struct>;
    unsafe {

    EVAL_node_re_close::<Dyn>(c, n, pos, (*np).u.g, (*np).u.next)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_re_backref(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {
    let np = n as *mut NodeOf<node_re_backref_struct>;
    unsafe {

    EVAL_node_re_backref::<Dyn>(c, n, pos, (*np).u.groups, (*np).u.icase, (*np).u.next)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_re_open_rec(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {
    let np = n as *mut NodeOf<node_re_open_rec_struct>;
    unsafe {

    EVAL_node_re_open_rec::<Dyn>(c, n, pos, (*np).u.g, (*np).u.next)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_re_close_rec(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {
    let np = n as *mut NodeOf<node_re_close_rec_struct>;
    unsafe {

    EVAL_node_re_close_rec::<Dyn>(c, n, pos, (*np).u.g, (*np).u.next)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_re_call(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {
    let np = n as *mut NodeOf<node_re_call_struct>;
    unsafe {

    EVAL_node_re_call::<Dyn>(c, n, pos, (*np).u.g, (*np).u.next)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_re_ret(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {
    let np = n as *mut NodeOf<node_re_ret_struct>;
    unsafe {

    EVAL_node_re_ret(c, n, pos)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_re_backref_lv(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {
    let np = n as *mut NodeOf<node_re_backref_lv_struct>;
    unsafe {

    EVAL_node_re_backref_lv::<Dyn>(c, n, pos, (*np).u.groups, (*np).u.icase, (*np).u.nest, (*np).u.next)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_re_cond(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {
    let np = n as *mut NodeOf<node_re_cond_struct>;
    unsafe {

    EVAL_node_re_cond::<Dyn, Dyn>(c, n, pos, (*np).u.groups, (*np).u.yes, (*np).u.no)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_re_alt(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {
    let np = n as *mut NodeOf<node_re_alt_struct>;
    unsafe {

    EVAL_node_re_alt::<Dyn, Dyn>(c, n, pos, (*np).u.a, (*np).u.b)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_re_opt(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {
    let np = n as *mut NodeOf<node_re_opt_struct>;
    unsafe {

    EVAL_node_re_opt::<Dyn, Dyn>(c, n, pos, (*np).u.body, (*np).u.skip, (*np).u.greedy)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_re_rep(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {
    let np = n as *mut NodeOf<node_re_rep_struct>;
    unsafe {

    EVAL_node_re_rep::<Dyn, Dyn>(c, n, pos, (*np).u.body, (*np).u.outer, (*np).u.min, (*np).u.max, (*np).u.greedy, (*np).u.mem, (*np).u.shape, (*np).u.cost)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_re_rep_cont(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {
    let np = n as *mut NodeOf<node_re_rep_cont_struct>;
    unsafe {

    EVAL_node_re_rep_cont(c, n, pos)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_re_look(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {
    let np = n as *mut NodeOf<node_re_look_struct>;
    unsafe {

    EVAL_node_re_look::<Dyn, Dyn>(c, n, pos, (*np).u.kind, (*np).u.body, (*np).u.next, (*np).u.minlen, (*np).u.maxlen)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_search(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {
    let np = n as *mut NodeOf<node_search_struct>;
    unsafe {

    EVAL_node_search::<Dyn>(c, n, pos, (*np).u.body, (*np).u.anchor)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_search_memchr(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {
    let np = n as *mut NodeOf<node_search_memchr_struct>;
    unsafe {

    EVAL_node_search_memchr::<Dyn>(c, n, pos, (*np).u.body, (*np).u.b)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_search_memmem(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {
    let np = n as *mut NodeOf<node_search_memmem_struct>;
    unsafe {

    EVAL_node_search_memmem::<Dyn>(c, n, pos, (*np).u.body, (*np).u.needle)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_search_byteset(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {
    let np = n as *mut NodeOf<node_search_byteset_struct>;
    unsafe {

    EVAL_node_search_byteset::<Dyn>(c, n, pos, (*np).u.body, (*np).u.set)
    }
}

#[allow(non_snake_case, unused_variables, unused_unsafe)]
pub fn DISPATCH_node_match_at(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {
    let np = n as *mut NodeOf<node_match_at_struct>;
    unsafe {

    EVAL_node_match_at::<Dyn>(c, n, pos, (*np).u.body)
    }
}

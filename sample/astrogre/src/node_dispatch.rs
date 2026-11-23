// This file is auto-generated from node.def by astrustgen. Do not edit.
#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_re_succ(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {
    let u = node_u::<node_re_succ_struct>(n);

    EVAL_node_re_succ(c, n, pos)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_re_sub_succ(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {
    let u = node_u::<node_re_sub_succ_struct>(n);

    EVAL_node_re_sub_succ(c, n, pos)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_re_sub_succ_at(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {
    let u = node_u::<node_re_sub_succ_at_struct>(n);

    EVAL_node_re_sub_succ_at(c, n, pos)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_re_byte(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {
    let u = node_u::<node_re_byte_struct>(n);

    EVAL_node_re_byte::<Dyn>(c, n, pos, u.b, u.next)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_re_lit(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {
    let u = node_u::<node_re_lit_struct>(n);

    EVAL_node_re_lit::<Dyn>(c, n, pos, u.bytes, u.next)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_re_one(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {
    let u = node_u::<node_re_one_struct>(n);

    EVAL_node_re_one::<Dyn>(c, n, pos, u.kind, u.ch, u.cls, u.next)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_re_grapheme(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {
    let u = node_u::<node_re_grapheme_struct>(n);

    EVAL_node_re_grapheme::<Dyn>(c, n, pos, u.next)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_re_rep1(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {
    let u = node_u::<node_re_rep1_struct>(n);

    EVAL_node_re_rep1::<Dyn>(c, n, pos, u.kind, u.ch, u.cls, u.min, u.max, u.greedy, u.nb, u.next)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_re_assert(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {
    let u = node_u::<node_re_assert_struct>(n);

    EVAL_node_re_assert::<Dyn>(c, n, pos, u.kind, u.next)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_re_keep(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {
    let u = node_u::<node_re_keep_struct>(n);

    EVAL_node_re_keep::<Dyn>(c, n, pos, u.next)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_re_open(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {
    let u = node_u::<node_re_open_struct>(n);

    EVAL_node_re_open::<Dyn>(c, n, pos, u.g, u.next)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_re_open_nr(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {
    let u = node_u::<node_re_open_nr_struct>(n);

    EVAL_node_re_open_nr::<Dyn>(c, n, pos, u.g, u.next)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_re_close(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {
    let u = node_u::<node_re_close_struct>(n);

    EVAL_node_re_close::<Dyn>(c, n, pos, u.g, u.next)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_re_backref(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {
    let u = node_u::<node_re_backref_struct>(n);

    EVAL_node_re_backref::<Dyn>(c, n, pos, u.groups, u.icase, u.next)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_re_open_rec(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {
    let u = node_u::<node_re_open_rec_struct>(n);

    EVAL_node_re_open_rec::<Dyn>(c, n, pos, u.g, u.next)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_re_close_rec(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {
    let u = node_u::<node_re_close_rec_struct>(n);

    EVAL_node_re_close_rec::<Dyn>(c, n, pos, u.g, u.next)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_re_call(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {
    let u = node_u::<node_re_call_struct>(n);

    EVAL_node_re_call::<Dyn>(c, n, pos, u.g, u.next)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_re_ret(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {
    let u = node_u::<node_re_ret_struct>(n);

    EVAL_node_re_ret(c, n, pos)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_re_backref_lv(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {
    let u = node_u::<node_re_backref_lv_struct>(n);

    EVAL_node_re_backref_lv::<Dyn>(c, n, pos, u.groups, u.icase, u.nest, u.next)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_re_cond(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {
    let u = node_u::<node_re_cond_struct>(n);

    EVAL_node_re_cond::<Dyn, Dyn>(c, n, pos, u.groups, u.yes, u.no)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_re_alt(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {
    let u = node_u::<node_re_alt_struct>(n);

    EVAL_node_re_alt::<Dyn, Dyn>(c, n, pos, u.a, u.b)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_re_opt(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {
    let u = node_u::<node_re_opt_struct>(n);

    EVAL_node_re_opt::<Dyn, Dyn>(c, n, pos, u.body, u.skip, u.greedy)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_re_rep(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {
    let u = node_u::<node_re_rep_struct>(n);

    EVAL_node_re_rep::<Dyn, Dyn>(c, n, pos, u.body, u.outer, u.min, u.max, u.greedy, u.mem, u.shape, u.cost)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_re_rep_cont(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {
    let u = node_u::<node_re_rep_cont_struct>(n);

    EVAL_node_re_rep_cont(c, n, pos)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_re_look(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {
    let u = node_u::<node_re_look_struct>(n);

    EVAL_node_re_look::<Dyn, Dyn>(c, n, pos, u.kind, u.body, u.next, u.minlen, u.maxlen)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_re_absent(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {
    let u = node_u::<node_re_absent_struct>(n);

    EVAL_node_re_absent::<Dyn, Dyn>(c, n, pos, u.body, u.next, u.minlen, u.maxlen)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_search(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {
    let u = node_u::<node_search_struct>(n);

    EVAL_node_search::<Dyn>(c, n, pos, u.body, u.anchor)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_search_memchr(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {
    let u = node_u::<node_search_memchr_struct>(n);

    EVAL_node_search_memchr::<Dyn>(c, n, pos, u.body, u.b)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_search_memmem(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {
    let u = node_u::<node_search_memmem_struct>(n);

    EVAL_node_search_memmem::<Dyn>(c, n, pos, u.body, u.needle)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_search_byteset(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {
    let u = node_u::<node_search_byteset_struct>(n);

    EVAL_node_search_byteset::<Dyn>(c, n, pos, u.body, u.set)
}

#[allow(non_snake_case, unused_variables)]
pub fn DISPATCH_node_match_at(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {
    let u = node_u::<node_match_at_struct>(n);

    EVAL_node_match_at::<Dyn>(c, n, pos, u.body)
}

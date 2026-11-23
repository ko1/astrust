// This file is auto-generated from node.def by astrustgen. Do not edit.
#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_re_succ(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_re_succ_struct>(n);

    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "pos".to_string()];

    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {{\n"));
    body.push_str(&format!("        let u = node_u::<node_re_succ_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_re_succ{}({})\n    }}\n", "", a.join(", ")));


    body.push_str("}
");
    let name = format!("T_{:016x}", hash_str(&body));
    if sc.emitted.insert(name.clone()) {
        sc.out.push_str(&format!("pub struct {};\n", name));
        if sc.comments { sc.out.push_str("// "); dump_node(&mut sc.out, n); sc.out.push('\n'); }
        sc.out.push_str(&body.replace("@SELF@", &name));
    }
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_re_sub_succ(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_re_sub_succ_struct>(n);

    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "pos".to_string()];

    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {{\n"));
    body.push_str(&format!("        let u = node_u::<node_re_sub_succ_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_re_sub_succ{}({})\n    }}\n", "", a.join(", ")));


    body.push_str("}
");
    let name = format!("T_{:016x}", hash_str(&body));
    if sc.emitted.insert(name.clone()) {
        sc.out.push_str(&format!("pub struct {};\n", name));
        if sc.comments { sc.out.push_str("// "); dump_node(&mut sc.out, n); sc.out.push('\n'); }
        sc.out.push_str(&body.replace("@SELF@", &name));
    }
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_re_sub_succ_at(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_re_sub_succ_at_struct>(n);

    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "pos".to_string()];

    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {{\n"));
    body.push_str(&format!("        let u = node_u::<node_re_sub_succ_at_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_re_sub_succ_at{}({})\n    }}\n", "", a.join(", ")));


    body.push_str("}
");
    let name = format!("T_{:016x}", hash_str(&body));
    if sc.emitted.insert(name.clone()) {
        sc.out.push_str(&format!("pub struct {};\n", name));
        if sc.comments { sc.out.push_str("// "); dump_node(&mut sc.out, n); sc.out.push('\n'); }
        sc.out.push_str(&body.replace("@SELF@", &name));
    }
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_re_byte(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_re_byte_struct>(n);
    let t_next = specialize_node(sc, u.next);
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "pos".to_string()];
    a.push(format!("{}u8", u.b));
    a.push("u.next".to_string());
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {{\n"));
    body.push_str(&format!("        let u = node_u::<node_re_byte_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_re_byte{}({})\n    }}\n", format!("::<{}>", t_next), a.join(", ")));


    body.push_str("}
");
    let name = format!("T_{:016x}", hash_str(&body));
    if sc.emitted.insert(name.clone()) {
        sc.out.push_str(&format!("pub struct {};\n", name));
        if sc.comments { sc.out.push_str("// "); dump_node(&mut sc.out, n); sc.out.push('\n'); }
        sc.out.push_str(&body.replace("@SELF@", &name));
    }
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_re_lit(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_re_lit_struct>(n);
    let t_next = specialize_node(sc, u.next);
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "pos".to_string()];
    a.push(bytes_lit(u.bytes));
    a.push("u.next".to_string());
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {{\n"));
    body.push_str(&format!("        let u = node_u::<node_re_lit_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_re_lit{}({})\n    }}\n", format!("::<{}>", t_next), a.join(", ")));


    body.push_str("}
");
    let name = format!("T_{:016x}", hash_str(&body));
    if sc.emitted.insert(name.clone()) {
        sc.out.push_str(&format!("pub struct {};\n", name));
        if sc.comments { sc.out.push_str("// "); dump_node(&mut sc.out, n); sc.out.push('\n'); }
        sc.out.push_str(&body.replace("@SELF@", &name));
    }
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_re_one(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_re_one_struct>(n);
    let t_next = specialize_node(sc, u.next);
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "pos".to_string()];
    a.push(format!("{}u32", u.kind));
    a.push(format!("{}u32", u.ch));
    a.push("u.cls".to_string());
    a.push("u.next".to_string());
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {{\n"));
    body.push_str(&format!("        let u = node_u::<node_re_one_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_re_one{}({})\n    }}\n", format!("::<{}>", t_next), a.join(", ")));


    body.push_str("}
");
    let name = format!("T_{:016x}", hash_str(&body));
    if sc.emitted.insert(name.clone()) {
        sc.out.push_str(&format!("pub struct {};\n", name));
        if sc.comments { sc.out.push_str("// "); dump_node(&mut sc.out, n); sc.out.push('\n'); }
        sc.out.push_str(&body.replace("@SELF@", &name));
    }
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_re_grapheme(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_re_grapheme_struct>(n);
    let t_next = specialize_node(sc, u.next);
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "pos".to_string()];
    a.push("u.next".to_string());
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {{\n"));
    body.push_str(&format!("        let u = node_u::<node_re_grapheme_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_re_grapheme{}({})\n    }}\n", format!("::<{}>", t_next), a.join(", ")));


    body.push_str("}
");
    let name = format!("T_{:016x}", hash_str(&body));
    if sc.emitted.insert(name.clone()) {
        sc.out.push_str(&format!("pub struct {};\n", name));
        if sc.comments { sc.out.push_str("// "); dump_node(&mut sc.out, n); sc.out.push('\n'); }
        sc.out.push_str(&body.replace("@SELF@", &name));
    }
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_re_rep1(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_re_rep1_struct>(n);
    let t_next = specialize_node(sc, u.next);
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "pos".to_string()];
    a.push(format!("{}u32", u.kind));
    a.push(format!("{}u32", u.ch));
    a.push("u.cls".to_string());
    a.push(format!("{}u32", u.min));
    a.push(format!("{}u32", u.max));
    a.push(format!("{}", u.greedy));
    a.push(format!("{}i32", u.nb));
    a.push("u.next".to_string());
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {{\n"));
    body.push_str(&format!("        let u = node_u::<node_re_rep1_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_re_rep1{}({})\n    }}\n", format!("::<{}>", t_next), a.join(", ")));


    body.push_str("}
");
    let name = format!("T_{:016x}", hash_str(&body));
    if sc.emitted.insert(name.clone()) {
        sc.out.push_str(&format!("pub struct {};\n", name));
        if sc.comments { sc.out.push_str("// "); dump_node(&mut sc.out, n); sc.out.push('\n'); }
        sc.out.push_str(&body.replace("@SELF@", &name));
    }
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_re_assert(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_re_assert_struct>(n);
    let t_next = specialize_node(sc, u.next);
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "pos".to_string()];
    a.push(format!("{}u32", u.kind));
    a.push("u.next".to_string());
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {{\n"));
    body.push_str(&format!("        let u = node_u::<node_re_assert_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_re_assert{}({})\n    }}\n", format!("::<{}>", t_next), a.join(", ")));


    body.push_str("}
");
    let name = format!("T_{:016x}", hash_str(&body));
    if sc.emitted.insert(name.clone()) {
        sc.out.push_str(&format!("pub struct {};\n", name));
        if sc.comments { sc.out.push_str("// "); dump_node(&mut sc.out, n); sc.out.push('\n'); }
        sc.out.push_str(&body.replace("@SELF@", &name));
    }
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_re_keep(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_re_keep_struct>(n);
    let t_next = specialize_node(sc, u.next);
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "pos".to_string()];
    a.push("u.next".to_string());
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {{\n"));
    body.push_str(&format!("        let u = node_u::<node_re_keep_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_re_keep{}({})\n    }}\n", format!("::<{}>", t_next), a.join(", ")));


    body.push_str("}
");
    let name = format!("T_{:016x}", hash_str(&body));
    if sc.emitted.insert(name.clone()) {
        sc.out.push_str(&format!("pub struct {};\n", name));
        if sc.comments { sc.out.push_str("// "); dump_node(&mut sc.out, n); sc.out.push('\n'); }
        sc.out.push_str(&body.replace("@SELF@", &name));
    }
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_re_open(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_re_open_struct>(n);
    let t_next = specialize_node(sc, u.next);
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "pos".to_string()];
    a.push(format!("{}u32", u.g));
    a.push("u.next".to_string());
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {{\n"));
    body.push_str(&format!("        let u = node_u::<node_re_open_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_re_open{}({})\n    }}\n", format!("::<{}>", t_next), a.join(", ")));


    body.push_str("}
");
    let name = format!("T_{:016x}", hash_str(&body));
    if sc.emitted.insert(name.clone()) {
        sc.out.push_str(&format!("pub struct {};\n", name));
        if sc.comments { sc.out.push_str("// "); dump_node(&mut sc.out, n); sc.out.push('\n'); }
        sc.out.push_str(&body.replace("@SELF@", &name));
    }
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_re_open_nr(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_re_open_nr_struct>(n);
    let t_next = specialize_node(sc, u.next);
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "pos".to_string()];
    a.push(format!("{}u32", u.g));
    a.push("u.next".to_string());
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {{\n"));
    body.push_str(&format!("        let u = node_u::<node_re_open_nr_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_re_open_nr{}({})\n    }}\n", format!("::<{}>", t_next), a.join(", ")));


    body.push_str("}
");
    let name = format!("T_{:016x}", hash_str(&body));
    if sc.emitted.insert(name.clone()) {
        sc.out.push_str(&format!("pub struct {};\n", name));
        if sc.comments { sc.out.push_str("// "); dump_node(&mut sc.out, n); sc.out.push('\n'); }
        sc.out.push_str(&body.replace("@SELF@", &name));
    }
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_re_close(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_re_close_struct>(n);
    let t_next = specialize_node(sc, u.next);
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "pos".to_string()];
    a.push(format!("{}u32", u.g));
    a.push("u.next".to_string());
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {{\n"));
    body.push_str(&format!("        let u = node_u::<node_re_close_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_re_close{}({})\n    }}\n", format!("::<{}>", t_next), a.join(", ")));


    body.push_str("}
");
    let name = format!("T_{:016x}", hash_str(&body));
    if sc.emitted.insert(name.clone()) {
        sc.out.push_str(&format!("pub struct {};\n", name));
        if sc.comments { sc.out.push_str("// "); dump_node(&mut sc.out, n); sc.out.push('\n'); }
        sc.out.push_str(&body.replace("@SELF@", &name));
    }
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_re_backref(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_re_backref_struct>(n);
    let t_next = specialize_node(sc, u.next);
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "pos".to_string()];
    a.push("u.groups".to_string());
    a.push(format!("{}", u.icase));
    a.push("u.next".to_string());
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {{\n"));
    body.push_str(&format!("        let u = node_u::<node_re_backref_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_re_backref{}({})\n    }}\n", format!("::<{}>", t_next), a.join(", ")));


    body.push_str("}
");
    let name = format!("T_{:016x}", hash_str(&body));
    if sc.emitted.insert(name.clone()) {
        sc.out.push_str(&format!("pub struct {};\n", name));
        if sc.comments { sc.out.push_str("// "); dump_node(&mut sc.out, n); sc.out.push('\n'); }
        sc.out.push_str(&body.replace("@SELF@", &name));
    }
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_re_open_rec(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_re_open_rec_struct>(n);
    let t_next = specialize_node(sc, u.next);
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "pos".to_string()];
    a.push(format!("{}u32", u.g));
    a.push("u.next".to_string());
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {{\n"));
    body.push_str(&format!("        let u = node_u::<node_re_open_rec_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_re_open_rec{}({})\n    }}\n", format!("::<{}>", t_next), a.join(", ")));


    body.push_str("}
");
    let name = format!("T_{:016x}", hash_str(&body));
    if sc.emitted.insert(name.clone()) {
        sc.out.push_str(&format!("pub struct {};\n", name));
        if sc.comments { sc.out.push_str("// "); dump_node(&mut sc.out, n); sc.out.push('\n'); }
        sc.out.push_str(&body.replace("@SELF@", &name));
    }
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_re_close_rec(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_re_close_rec_struct>(n);
    let t_next = specialize_node(sc, u.next);
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "pos".to_string()];
    a.push(format!("{}u32", u.g));
    a.push("u.next".to_string());
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {{\n"));
    body.push_str(&format!("        let u = node_u::<node_re_close_rec_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_re_close_rec{}({})\n    }}\n", format!("::<{}>", t_next), a.join(", ")));


    body.push_str("}
");
    let name = format!("T_{:016x}", hash_str(&body));
    if sc.emitted.insert(name.clone()) {
        sc.out.push_str(&format!("pub struct {};\n", name));
        if sc.comments { sc.out.push_str("// "); dump_node(&mut sc.out, n); sc.out.push('\n'); }
        sc.out.push_str(&body.replace("@SELF@", &name));
    }
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_re_call(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_re_call_struct>(n);
    let t_next = specialize_node(sc, u.next);
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "pos".to_string()];
    a.push(format!("{}u32", u.g));
    a.push("u.next".to_string());
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {{\n"));
    body.push_str(&format!("        let u = node_u::<node_re_call_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_re_call{}({})\n    }}\n", format!("::<{}>", t_next), a.join(", ")));


    body.push_str("}
");
    let name = format!("T_{:016x}", hash_str(&body));
    if sc.emitted.insert(name.clone()) {
        sc.out.push_str(&format!("pub struct {};\n", name));
        if sc.comments { sc.out.push_str("// "); dump_node(&mut sc.out, n); sc.out.push('\n'); }
        sc.out.push_str(&body.replace("@SELF@", &name));
    }
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_re_ret(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_re_ret_struct>(n);

    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "pos".to_string()];

    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {{\n"));
    body.push_str(&format!("        let u = node_u::<node_re_ret_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_re_ret{}({})\n    }}\n", "", a.join(", ")));


    body.push_str("}
");
    let name = format!("T_{:016x}", hash_str(&body));
    if sc.emitted.insert(name.clone()) {
        sc.out.push_str(&format!("pub struct {};\n", name));
        if sc.comments { sc.out.push_str("// "); dump_node(&mut sc.out, n); sc.out.push('\n'); }
        sc.out.push_str(&body.replace("@SELF@", &name));
    }
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_re_backref_lv(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_re_backref_lv_struct>(n);
    let t_next = specialize_node(sc, u.next);
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "pos".to_string()];
    a.push("u.groups".to_string());
    a.push(format!("{}", u.icase));
    a.push(format!("{}i32", u.nest));
    a.push("u.next".to_string());
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {{\n"));
    body.push_str(&format!("        let u = node_u::<node_re_backref_lv_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_re_backref_lv{}({})\n    }}\n", format!("::<{}>", t_next), a.join(", ")));


    body.push_str("}
");
    let name = format!("T_{:016x}", hash_str(&body));
    if sc.emitted.insert(name.clone()) {
        sc.out.push_str(&format!("pub struct {};\n", name));
        if sc.comments { sc.out.push_str("// "); dump_node(&mut sc.out, n); sc.out.push('\n'); }
        sc.out.push_str(&body.replace("@SELF@", &name));
    }
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_re_cond(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_re_cond_struct>(n);
    let t_yes = specialize_node(sc, u.yes);
    let t_no = specialize_node(sc, u.no);
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "pos".to_string()];
    a.push("u.groups".to_string());
    a.push("u.yes".to_string());
    a.push("u.no".to_string());
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {{\n"));
    body.push_str(&format!("        let u = node_u::<node_re_cond_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_re_cond{}({})\n    }}\n", format!("::<{}, {}>", t_yes, t_no), a.join(", ")));


    body.push_str("}
");
    let name = format!("T_{:016x}", hash_str(&body));
    if sc.emitted.insert(name.clone()) {
        sc.out.push_str(&format!("pub struct {};\n", name));
        if sc.comments { sc.out.push_str("// "); dump_node(&mut sc.out, n); sc.out.push('\n'); }
        sc.out.push_str(&body.replace("@SELF@", &name));
    }
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_re_alt(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_re_alt_struct>(n);
    let t_a = specialize_node(sc, u.a);
    let t_b = specialize_node(sc, u.b);
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "pos".to_string()];
    a.push("u.a".to_string());
    a.push("u.b".to_string());
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {{\n"));
    body.push_str(&format!("        let u = node_u::<node_re_alt_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_re_alt{}({})\n    }}\n", format!("::<{}, {}>", t_a, t_b), a.join(", ")));


    body.push_str("}
");
    let name = format!("T_{:016x}", hash_str(&body));
    if sc.emitted.insert(name.clone()) {
        sc.out.push_str(&format!("pub struct {};\n", name));
        if sc.comments { sc.out.push_str("// "); dump_node(&mut sc.out, n); sc.out.push('\n'); }
        sc.out.push_str(&body.replace("@SELF@", &name));
    }
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_re_opt(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_re_opt_struct>(n);
    let t_body = specialize_node(sc, u.body);
    let t_skip = specialize_node(sc, u.skip);
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "pos".to_string()];
    a.push("u.body".to_string());
    a.push("u.skip".to_string());
    a.push(format!("{}", u.greedy));
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {{\n"));
    body.push_str(&format!("        let u = node_u::<node_re_opt_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_re_opt{}({})\n    }}\n", format!("::<{}, {}>", t_body, t_skip), a.join(", ")));


    body.push_str("}
");
    let name = format!("T_{:016x}", hash_str(&body));
    if sc.emitted.insert(name.clone()) {
        sc.out.push_str(&format!("pub struct {};\n", name));
        if sc.comments { sc.out.push_str("// "); dump_node(&mut sc.out, n); sc.out.push('\n'); }
        sc.out.push_str(&body.replace("@SELF@", &name));
    }
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_re_rep(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_re_rep_struct>(n);
    let t_body = specialize_node(sc, u.body);
    let t_outer = specialize_node(sc, u.outer);
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "pos".to_string()];
    a.push("u.body".to_string());
    a.push("u.outer".to_string());
    a.push(format!("{}u32", u.min));
    a.push(format!("{}u32", u.max));
    a.push(format!("{}", u.greedy));
    a.push(format!("{}", u.mem));
    a.push(format!("{}u32", u.shape));
    a.push(format!("{}u32", u.cost));
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {{\n"));
    body.push_str(&format!("        let u = node_u::<node_re_rep_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_re_rep{}({})\n    }}\n", format!("::<{}, {}>", t_body, t_outer), a.join(", ")));


    body.push_str("}
");
    let name = format!("T_{:016x}", hash_str(&body));
    if sc.emitted.insert(name.clone()) {
        sc.out.push_str(&format!("pub struct {};\n", name));
        if sc.comments { sc.out.push_str("// "); dump_node(&mut sc.out, n); sc.out.push('\n'); }
        sc.out.push_str(&body.replace("@SELF@", &name));
    }
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_re_rep_cont(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_re_rep_cont_struct>(n);

    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "pos".to_string()];

    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {{\n"));
    body.push_str(&format!("        let u = node_u::<node_re_rep_cont_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_re_rep_cont{}({})\n    }}\n", "", a.join(", ")));


    body.push_str("}
");
    let name = format!("T_{:016x}", hash_str(&body));
    if sc.emitted.insert(name.clone()) {
        sc.out.push_str(&format!("pub struct {};\n", name));
        if sc.comments { sc.out.push_str("// "); dump_node(&mut sc.out, n); sc.out.push('\n'); }
        sc.out.push_str(&body.replace("@SELF@", &name));
    }
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_re_look(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_re_look_struct>(n);
    let t_body = specialize_node(sc, u.body);
    let t_next = specialize_node(sc, u.next);
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "pos".to_string()];
    a.push(format!("{}u32", u.kind));
    a.push("u.body".to_string());
    a.push("u.next".to_string());
    a.push(format!("{}u32", u.minlen));
    a.push(format!("{}u32", u.maxlen));
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {{\n"));
    body.push_str(&format!("        let u = node_u::<node_re_look_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_re_look{}({})\n    }}\n", format!("::<{}, {}>", t_body, t_next), a.join(", ")));


    body.push_str("}
");
    let name = format!("T_{:016x}", hash_str(&body));
    if sc.emitted.insert(name.clone()) {
        sc.out.push_str(&format!("pub struct {};\n", name));
        if sc.comments { sc.out.push_str("// "); dump_node(&mut sc.out, n); sc.out.push('\n'); }
        sc.out.push_str(&body.replace("@SELF@", &name));
    }
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_re_absent(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_re_absent_struct>(n);
    let t_body = specialize_node(sc, u.body);
    let t_next = specialize_node(sc, u.next);
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "pos".to_string()];
    a.push("u.body".to_string());
    a.push("u.next".to_string());
    a.push(format!("{}u32", u.minlen));
    a.push(format!("{}u32", u.maxlen));
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {{\n"));
    body.push_str(&format!("        let u = node_u::<node_re_absent_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_re_absent{}({})\n    }}\n", format!("::<{}, {}>", t_body, t_next), a.join(", ")));


    body.push_str("}
");
    let name = format!("T_{:016x}", hash_str(&body));
    if sc.emitted.insert(name.clone()) {
        sc.out.push_str(&format!("pub struct {};\n", name));
        if sc.comments { sc.out.push_str("// "); dump_node(&mut sc.out, n); sc.out.push('\n'); }
        sc.out.push_str(&body.replace("@SELF@", &name));
    }
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_search(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_search_struct>(n);
    let t_body = specialize_node(sc, u.body);
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "pos".to_string()];
    a.push("u.body".to_string());
    a.push(format!("{}u32", u.anchor));
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {{\n"));
    body.push_str(&format!("        let u = node_u::<node_search_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_search{}({})\n    }}\n", format!("::<{}>", t_body), a.join(", ")));


    body.push_str("}
");
    let name = format!("T_{:016x}", hash_str(&body));
    if sc.emitted.insert(name.clone()) {
        sc.out.push_str(&format!("pub struct {};\n", name));
        if sc.comments { sc.out.push_str("// "); dump_node(&mut sc.out, n); sc.out.push('\n'); }
        sc.out.push_str(&body.replace("@SELF@", &name));
    }
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_search_memchr(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_search_memchr_struct>(n);
    let t_body = specialize_node(sc, u.body);
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "pos".to_string()];
    a.push("u.body".to_string());
    a.push(format!("{}u8", u.b));
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {{\n"));
    body.push_str(&format!("        let u = node_u::<node_search_memchr_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_search_memchr{}({})\n    }}\n", format!("::<{}>", t_body), a.join(", ")));


    body.push_str("}
");
    let name = format!("T_{:016x}", hash_str(&body));
    if sc.emitted.insert(name.clone()) {
        sc.out.push_str(&format!("pub struct {};\n", name));
        if sc.comments { sc.out.push_str("// "); dump_node(&mut sc.out, n); sc.out.push('\n'); }
        sc.out.push_str(&body.replace("@SELF@", &name));
    }
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_search_memmem(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_search_memmem_struct>(n);
    let t_body = specialize_node(sc, u.body);
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "pos".to_string()];
    a.push("u.body".to_string());
    a.push(bytes_lit(u.needle));
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {{\n"));
    body.push_str(&format!("        let u = node_u::<node_search_memmem_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_search_memmem{}({})\n    }}\n", format!("::<{}>", t_body), a.join(", ")));


    body.push_str("}
");
    let name = format!("T_{:016x}", hash_str(&body));
    if sc.emitted.insert(name.clone()) {
        sc.out.push_str(&format!("pub struct {};\n", name));
        if sc.comments { sc.out.push_str("// "); dump_node(&mut sc.out, n); sc.out.push('\n'); }
        sc.out.push_str(&body.replace("@SELF@", &name));
    }
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_search_byteset(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_search_byteset_struct>(n);
    let t_body = specialize_node(sc, u.body);
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "pos".to_string()];
    a.push("u.body".to_string());
    a.push("u.set".to_string());
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {{\n"));
    body.push_str(&format!("        let u = node_u::<node_search_byteset_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_search_byteset{}({})\n    }}\n", format!("::<{}>", t_body), a.join(", ")));


    body.push_str("}
");
    let name = format!("T_{:016x}", hash_str(&body));
    if sc.emitted.insert(name.clone()) {
        sc.out.push_str(&format!("pub struct {};\n", name));
        if sc.comments { sc.out.push_str("// "); dump_node(&mut sc.out, n); sc.out.push('\n'); }
        sc.out.push_str(&body.replace("@SELF@", &name));
    }
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_match_at(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_match_at_struct>(n);
    let t_body = specialize_node(sc, u.body);
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "pos".to_string()];
    a.push("u.body".to_string());
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {{\n"));
    body.push_str(&format!("        let u = node_u::<node_match_at_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_match_at{}({})\n    }}\n", format!("::<{}>", t_body), a.join(", ")));


    body.push_str("}
");
    let name = format!("T_{:016x}", hash_str(&body));
    if sc.emitted.insert(name.clone()) {
        sc.out.push_str(&format!("pub struct {};\n", name));
        if sc.comments { sc.out.push_str("// "); dump_node(&mut sc.out, n); sc.out.push('\n'); }
        sc.out.push_str(&body.replace("@SELF@", &name));
    }
    name
}

/// SD type for a NODES operand: a `DispList` whose nth() dispatches statically.
pub fn specialize_list(sc: &mut SpecCtx, ns: &NodeList) -> String {
    let ts: Vec<String> = ns.iter().map(|x| specialize_node(sc, *x)).collect();
    // named by its code, as a T_
    let mut s = format!("impl DispList for @SELF@ {{\n    #[inline(always)] fn len(_ns: NodeList) -> usize {{ {} }}\n", ts.len());
    s.push_str(&format!("    #[inline(always)]\n    fn nth(c: &mut Ctx, ns: NodeList, i: usize, pos: usize) -> u32 {{\n        match i {{\n"));
    for (i, t) in ts.iter().enumerate() {
        let x = format!("unsafe {{ *ns.get_unchecked({}) }}", i);
        s.push_str(&format!("            {} => <{} as Disp>::call(c, {}, pos),\n", i, t, x));
    }
    // i < len (EVAL_NTH!): a safe default arm adds a range check where i is not a constant (unsafe_catalog.md)
    s.push_str("            _ => unsafe { std::hint::unreachable_unchecked() },\n        }\n    }\n");
    // EVAL_EACH!: the calls in a row
    s.push_str("    #[inline(always)]\n    fn each<F: FnMut(&mut Ctx, usize, u32) -> ()>(c: &mut Ctx, ns: NodeList, pos: usize, mut f: F) -> () {\n");
    for (i, t) in ts.iter().enumerate() {
        let x = format!("unsafe {{ *ns.get_unchecked({}) }}", i);
        let call = format!("<{} as Disp>::call(c, {}, pos)", t, x);
        let step = format!("{{ let r = {}; f(c, {}, r) }}", "@CALL@".replace("@CALL@", &call), i);
        s.push_str(&format!("        {}\n", "@STEP@;".replace("@STEP@", &step)));
    }
    s.push_str("        ()\n    }\n}\n");
    let name = format!("L_{:016x}", hash_str(&s));
    if sc.emitted.insert(name.clone()) { sc.out.push_str(&format!("pub struct {};\n{}", name, s.replace("@SELF@", &name))); }
    name
}

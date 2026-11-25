// This file is auto-generated from node.def by astrustgen. Do not edit.
#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_nil(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_nil_struct>(n);

    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];

    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_nil_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_nil{}({})\n    }}\n", "", a.join(", ")));


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
fn SPECIALIZE_node_true(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_true_struct>(n);

    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];

    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_true_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_true{}({})\n    }}\n", "", a.join(", ")));


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
fn SPECIALIZE_node_false(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_false_struct>(n);

    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];

    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_false_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_false{}({})\n    }}\n", "", a.join(", ")));


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
fn SPECIALIZE_node_internal(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_internal_struct>(n);

    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];

    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_internal_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_internal{}({})\n    }}\n", "", a.join(", ")));


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
fn SPECIALIZE_node_self(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_self_struct>(n);

    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];

    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_self_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_self{}({})\n    }}\n", "", a.join(", ")));


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
fn SPECIALIZE_node_lit(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_lit_struct>(n);

    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push(crate::compile::value_bake(u.v).unwrap_or_else(|| "u.v".to_string()));
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_lit_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_lit{}({})\n    }}\n", "", a.join(", ")));


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
fn SPECIALIZE_node_float(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_float_struct>(n);

    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push(format!("f64::from_bits({}u64)", u.v.to_bits()));
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_float_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_float{}({})\n    }}\n", "", a.join(", ")));


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
fn SPECIALIZE_node_str(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_str_struct>(n);

    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push("u.s".to_string());
    a.push(format!("{}u32", u.enc));
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_str_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_str{}({})\n    }}\n", "", a.join(", ")));


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
fn SPECIALIZE_node_cstr(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_cstr_struct>(n);

    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push("u.s".to_string());
    a.push(format!("{}u32", u.enc));
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_cstr_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_cstr{}({})\n    }}\n", "", a.join(", ")));


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
fn SPECIALIZE_node_fstr(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_fstr_struct>(n);

    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push("u.s".to_string());
    a.push(format!("{}u32", u.enc));
    a.push("&u.lc".to_string());
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_fstr_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_fstr{}({})\n    }}\n", "", a.join(", ")));


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
fn SPECIALIZE_node_fstr_recv(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_fstr_recv_struct>(n);

    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push("u.s".to_string());
    a.push(format!("{}u32", u.enc));
    a.push(format!("{}u32", u.um));
    a.push("&u.lc".to_string());
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_fstr_recv_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_fstr_recv{}({})\n    }}\n", "", a.join(", ")));


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
fn SPECIALIZE_node_dstr(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_dstr_struct>(n);
    let t_parts = specialize_list(sc, &u.parts, n);
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push("list_base(n)".to_string());
    a.push(format!("{}u32", u.enc));
    a.push(format!("{}u32", u.cap));
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_dstr_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_dstr{}({})\n    }}\n", format!("::<{}>", t_parts), a.join(", ")));


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
fn SPECIALIZE_node_tostr(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_tostr_struct>(n);
    let t_v = specialize_node(sc, u.v);
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push("v_v".to_string());
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_tostr_struct>({});\n", "n"));
    body.push_str(&format!("        let v_v = <{} as Disp>::call(c, {}, fp)?;\n", t_v, child_ptr_expr(n, u.v)));
    body.push_str(&format!("        EVAL_node_tostr{}({})\n    }}\n", "", a.join(", ")));


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
fn SPECIALIZE_node_dsym(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_dsym_struct>(n);
    let t_s = specialize_node(sc, u.s);
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push("v_s".to_string());
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_dsym_struct>({});\n", "n"));
    body.push_str(&format!("        let v_s = <{} as Disp>::call(c, {}, fp)?;\n", t_s, child_ptr_expr(n, u.s)));
    body.push_str(&format!("        EVAL_node_dsym{}({})\n    }}\n", "", a.join(", ")));


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
fn SPECIALIZE_node_regex(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_regex_struct>(n);

    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push("u.src".to_string());
    a.push(format!("{}u32", u.opts));
    a.push("&u.lc".to_string());
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_regex_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_regex{}({})\n    }}\n", "", a.join(", ")));


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
fn SPECIALIZE_node_dregex(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_dregex_struct>(n);
    let t_s = specialize_node(sc, u.s);
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push("v_s".to_string());
    a.push(format!("{}u32", u.opts));
    a.push(format!("{}", u.once));
    a.push("&u.lc".to_string());
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_dregex_struct>({});\n", "n"));
    body.push_str(&format!("        let v_s = <{} as Disp>::call(c, {}, fp)?;\n", t_s, child_ptr_expr(n, u.s)));
    body.push_str(&format!("        EVAL_node_dregex{}({})\n    }}\n", "", a.join(", ")));


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
fn SPECIALIZE_node_biglit(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_biglit_struct>(n);

    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push("u.digits".to_string());
    a.push("&u.lc".to_string());
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_biglit_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_biglit{}({})\n    }}\n", "", a.join(", ")));


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
fn SPECIALIZE_node_ary(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_ary_struct>(n);
    let t_elems = specialize_list(sc, &u.elems, n);
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push("list_base(n)".to_string());
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_ary_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_ary{}({})\n    }}\n", format!("::<{}>", t_elems), a.join(", ")));


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
fn SPECIALIZE_node_ary_build(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_ary_build_struct>(n);
    let t_elems = specialize_list(sc, &u.elems, n);
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push("list_base(n)".to_string());
    a.push("u.kinds".to_string());
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_ary_build_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_ary_build{}({})\n    }}\n", format!("::<{}>", t_elems), a.join(", ")));


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
fn SPECIALIZE_node_hash(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_hash_struct>(n);
    let t_items = specialize_list(sc, &u.items, n);
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push("list_base(n)".to_string());
    a.push("u.kinds".to_string());
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_hash_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_hash{}({})\n    }}\n", format!("::<{}>", t_items), a.join(", ")));


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
fn SPECIALIZE_node_range(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_range_struct>(n);
    let t_b = specialize_node(sc, u.b);
    let t_e = specialize_node(sc, u.e);
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push("v_b".to_string());
    a.push("v_e".to_string());
    a.push(format!("{}", u.excl));
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_range_struct>({});\n", "n"));
    body.push_str(&format!("        let v_b = <{} as Disp>::call(c, {}, fp)?;\n", t_b, child_ptr_expr(n, u.b)));
    body.push_str(&format!("        let v_e = <{} as Disp>::call(c, {}, fp)?;\n", t_e, child_ptr_expr(n, u.e)));
    body.push_str(&format!("        EVAL_node_range{}({})\n    }}\n", "", a.join(", ")));


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
fn SPECIALIZE_node_range_lit(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_range_lit_struct>(n);

    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push(crate::compile::value_bake(u.b).unwrap_or_else(|| "u.b".to_string()));
    a.push(crate::compile::value_bake(u.e).unwrap_or_else(|| "u.e".to_string()));
    a.push(format!("{}", u.excl));
    a.push("&u.lc".to_string());
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_range_lit_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_range_lit{}({})\n    }}\n", "", a.join(", ")));


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
fn SPECIALIZE_node_lvar(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_lvar_struct>(n);

    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push(format!("{}u32", u.idx));
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_lvar_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_lvar{}({})\n    }}\n", "", a.join(", ")));


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
fn SPECIALIZE_node_lasgn(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_lasgn_struct>(n);
    let t_v = specialize_node(sc, u.v);
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push(format!("{}u32", u.idx));
    a.push("v_v".to_string());
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_lasgn_struct>({});\n", "n"));
    body.push_str(&format!("        let v_v = <{} as Disp>::call(c, {}, fp)?;\n", t_v, child_ptr_expr(n, u.v)));
    body.push_str(&format!("        EVAL_node_lasgn{}({})\n    }}\n", "", a.join(", ")));


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
fn SPECIALIZE_node_dvar(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_dvar_struct>(n);

    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push(format!("{}u32", u.hops));
    a.push(format!("{}u32", u.idx));
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_dvar_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_dvar{}({})\n    }}\n", "", a.join(", ")));


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
fn SPECIALIZE_node_dasgn(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_dasgn_struct>(n);
    let t_v = specialize_node(sc, u.v);
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push(format!("{}u32", u.hops));
    a.push(format!("{}u32", u.idx));
    a.push("v_v".to_string());
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_dasgn_struct>({});\n", "n"));
    body.push_str(&format!("        let v_v = <{} as Disp>::call(c, {}, fp)?;\n", t_v, child_ptr_expr(n, u.v)));
    body.push_str(&format!("        EVAL_node_dasgn{}({})\n    }}\n", "", a.join(", ")));


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
fn SPECIALIZE_node_ivar(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_ivar_struct>(n);

    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push("u.name".to_string());
    a.push("&u.ic".to_string());
    a.push(format!("{}u32", { if u.ic.klass.get() != 0 { u.ic.idx.get() + 1 } else { 0 } }));
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_ivar_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_ivar{}({})\n    }}\n", "", a.join(", ")));


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
fn SPECIALIZE_node_iasgn(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_iasgn_struct>(n);
    let t_v = specialize_node(sc, u.v);
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push("u.name".to_string());
    a.push("v_v".to_string());
    a.push("&u.ic".to_string());
    a.push(format!("{}u32", { if u.ic.klass.get() != 0 { u.ic.idx.get() + 1 } else { 0 } }));
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_iasgn_struct>({});\n", "n"));
    body.push_str(&format!("        let v_v = <{} as Disp>::call(c, {}, fp)?;\n", t_v, child_ptr_expr(n, u.v)));
    body.push_str(&format!("        EVAL_node_iasgn{}({})\n    }}\n", "", a.join(", ")));


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
fn SPECIALIZE_node_ivar_arith(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_ivar_arith_struct>(n);
    let t_v = specialize_node(sc, u.v);
    let mut t_line = "crate::vm::LineAt".to_string();
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push("u.name".to_string());
    a.push(format!("{}", u.sub));
    a.push("v_v".to_string());
    if sc.spec_depth == 0 { sc.holes.push((n as usize + std::mem::offset_of!(NodeOf<node_ivar_arith_struct>, head.line), std::mem::size_of::<u32>())); } if let Some(s) = sc.hole_sym("line", n as usize + std::mem::offset_of!(NodeOf<node_ivar_arith_struct>, head.line), "impl Clone for H_%SYM% { #[inline(always)] fn clone(&self) -> Self { *self } } impl Copy for H_%SYM% {} impl crate::vm::LineHole for H_%SYM% {   #[inline(always)] fn pos(self) -> crate::vm::Pos { unsafe { crate::vm::Pos::bound(&raw const %SYM%) } }   #[inline(always)] fn store(self, f: &mut crate::vm::Frame) { unsafe {     std::arch::asm!(\"mov qword ptr [{0} + {1}], offset {2}\", in(reg) f, const std::mem::offset_of!(crate::vm::Frame, pos), sym %SYM%, options(nostack, preserves_flags)) } } } ") { t_line = "H_%SYM%".replace("%SYM%", &s); a.push("H_%SYM%".replace("%SYM%", &s)); } else { a.push("crate::vm::LineAt(&(*node_head_line(n)))".to_string()); }
    a.push("&u.ic".to_string());
    a.push("&u.cc".to_string());
    a.push(format!("{}u32", { if u.ic.klass.get() != 0 { u.ic.idx.get() + 1 } else { 0 } }));
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_ivar_arith_struct>({});\n", "n"));
    body.push_str(&format!("        let v_v = <{} as Disp>::call(c, {}, fp)?;\n", t_v, child_ptr_expr(n, u.v)));
    body.push_str(&format!("        EVAL_node_ivar_arith{}({})\n    }}\n", format!("::<{}>", t_line), a.join(", ")));


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
fn SPECIALIZE_node_gvar(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_gvar_struct>(n);

    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push("u.name".to_string());
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_gvar_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_gvar{}({})\n    }}\n", "", a.join(", ")));


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
fn SPECIALIZE_node_gasgn(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_gasgn_struct>(n);
    let t_v = specialize_node(sc, u.v);
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push("u.name".to_string());
    a.push("v_v".to_string());
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_gasgn_struct>({});\n", "n"));
    body.push_str(&format!("        let v_v = <{} as Disp>::call(c, {}, fp)?;\n", t_v, child_ptr_expr(n, u.v)));
    body.push_str(&format!("        EVAL_node_gasgn{}({})\n    }}\n", "", a.join(", ")));


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
fn SPECIALIZE_node_cvar(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_cvar_struct>(n);

    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push("u.name".to_string());
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_cvar_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_cvar{}({})\n    }}\n", "", a.join(", ")));


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
fn SPECIALIZE_node_cvasgn(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_cvasgn_struct>(n);
    let t_v = specialize_node(sc, u.v);
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push("u.name".to_string());
    a.push("v_v".to_string());
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_cvasgn_struct>({});\n", "n"));
    body.push_str(&format!("        let v_v = <{} as Disp>::call(c, {}, fp)?;\n", t_v, child_ptr_expr(n, u.v)));
    body.push_str(&format!("        EVAL_node_cvasgn{}({})\n    }}\n", "", a.join(", ")));


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
fn SPECIALIZE_node_const(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_const_struct>(n);
    let mut t_line = "crate::vm::LineAt".to_string();
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push("u.name".to_string());
    if sc.spec_depth == 0 { sc.holes.push((n as usize + std::mem::offset_of!(NodeOf<node_const_struct>, head.line), std::mem::size_of::<u32>())); } if let Some(s) = sc.hole_sym("line", n as usize + std::mem::offset_of!(NodeOf<node_const_struct>, head.line), "impl Clone for H_%SYM% { #[inline(always)] fn clone(&self) -> Self { *self } } impl Copy for H_%SYM% {} impl crate::vm::LineHole for H_%SYM% {   #[inline(always)] fn pos(self) -> crate::vm::Pos { unsafe { crate::vm::Pos::bound(&raw const %SYM%) } }   #[inline(always)] fn store(self, f: &mut crate::vm::Frame) { unsafe {     std::arch::asm!(\"mov qword ptr [{0} + {1}], offset {2}\", in(reg) f, const std::mem::offset_of!(crate::vm::Frame, pos), sym %SYM%, options(nostack, preserves_flags)) } } } ") { t_line = "H_%SYM%".replace("%SYM%", &s); a.push("H_%SYM%".replace("%SYM%", &s)); } else { a.push("crate::vm::LineAt(&(*node_head_line(n)))".to_string()); }
    a.push("&u.cc".to_string());
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_const_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_const{}({})\n    }}\n", format!("::<{}>", t_line), a.join(", ")));


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
fn SPECIALIZE_node_const_path(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_const_path_struct>(n);
    let t_parent = specialize_node(sc, u.parent);
    let mut t_line = "crate::vm::LineAt".to_string();
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push("v_parent".to_string());
    a.push("u.name".to_string());
    if sc.spec_depth == 0 { sc.holes.push((n as usize + std::mem::offset_of!(NodeOf<node_const_path_struct>, head.line), std::mem::size_of::<u32>())); } if let Some(s) = sc.hole_sym("line", n as usize + std::mem::offset_of!(NodeOf<node_const_path_struct>, head.line), "impl Clone for H_%SYM% { #[inline(always)] fn clone(&self) -> Self { *self } } impl Copy for H_%SYM% {} impl crate::vm::LineHole for H_%SYM% {   #[inline(always)] fn pos(self) -> crate::vm::Pos { unsafe { crate::vm::Pos::bound(&raw const %SYM%) } }   #[inline(always)] fn store(self, f: &mut crate::vm::Frame) { unsafe {     std::arch::asm!(\"mov qword ptr [{0} + {1}], offset {2}\", in(reg) f, const std::mem::offset_of!(crate::vm::Frame, pos), sym %SYM%, options(nostack, preserves_flags)) } } } ") { t_line = "H_%SYM%".replace("%SYM%", &s); a.push("H_%SYM%".replace("%SYM%", &s)); } else { a.push("crate::vm::LineAt(&(*node_head_line(n)))".to_string()); }
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_const_path_struct>({});\n", "n"));
    body.push_str(&format!("        let v_parent = <{} as Disp>::call(c, {}, fp)?;\n", t_parent, child_ptr_expr(n, u.parent)));
    body.push_str(&format!("        EVAL_node_const_path{}({})\n    }}\n", format!("::<{}>", t_line), a.join(", ")));


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
fn SPECIALIZE_node_top_const(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_top_const_struct>(n);
    let mut t_line = "crate::vm::LineAt".to_string();
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push("u.name".to_string());
    if sc.spec_depth == 0 { sc.holes.push((n as usize + std::mem::offset_of!(NodeOf<node_top_const_struct>, head.line), std::mem::size_of::<u32>())); } if let Some(s) = sc.hole_sym("line", n as usize + std::mem::offset_of!(NodeOf<node_top_const_struct>, head.line), "impl Clone for H_%SYM% { #[inline(always)] fn clone(&self) -> Self { *self } } impl Copy for H_%SYM% {} impl crate::vm::LineHole for H_%SYM% {   #[inline(always)] fn pos(self) -> crate::vm::Pos { unsafe { crate::vm::Pos::bound(&raw const %SYM%) } }   #[inline(always)] fn store(self, f: &mut crate::vm::Frame) { unsafe {     std::arch::asm!(\"mov qword ptr [{0} + {1}], offset {2}\", in(reg) f, const std::mem::offset_of!(crate::vm::Frame, pos), sym %SYM%, options(nostack, preserves_flags)) } } } ") { t_line = "H_%SYM%".replace("%SYM%", &s); a.push("H_%SYM%".replace("%SYM%", &s)); } else { a.push("crate::vm::LineAt(&(*node_head_line(n)))".to_string()); }
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_top_const_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_top_const{}({})\n    }}\n", format!("::<{}>", t_line), a.join(", ")));


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
fn SPECIALIZE_node_cdecl(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_cdecl_struct>(n);
    let t_v = specialize_node(sc, u.v);
    let mut t_line = "crate::vm::LineAt".to_string();
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push("u.name".to_string());
    a.push("v_v".to_string());
    if sc.spec_depth == 0 { sc.holes.push((n as usize + std::mem::offset_of!(NodeOf<node_cdecl_struct>, head.line), std::mem::size_of::<u32>())); } if let Some(s) = sc.hole_sym("line", n as usize + std::mem::offset_of!(NodeOf<node_cdecl_struct>, head.line), "impl Clone for H_%SYM% { #[inline(always)] fn clone(&self) -> Self { *self } } impl Copy for H_%SYM% {} impl crate::vm::LineHole for H_%SYM% {   #[inline(always)] fn pos(self) -> crate::vm::Pos { unsafe { crate::vm::Pos::bound(&raw const %SYM%) } }   #[inline(always)] fn store(self, f: &mut crate::vm::Frame) { unsafe {     std::arch::asm!(\"mov qword ptr [{0} + {1}], offset {2}\", in(reg) f, const std::mem::offset_of!(crate::vm::Frame, pos), sym %SYM%, options(nostack, preserves_flags)) } } } ") { t_line = "H_%SYM%".replace("%SYM%", &s); a.push("H_%SYM%".replace("%SYM%", &s)); } else { a.push("crate::vm::LineAt(&(*node_head_line(n)))".to_string()); }
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_cdecl_struct>({});\n", "n"));
    body.push_str(&format!("        let v_v = <{} as Disp>::call(c, {}, fp)?;\n", t_v, child_ptr_expr(n, u.v)));
    body.push_str(&format!("        EVAL_node_cdecl{}({})\n    }}\n", format!("::<{}>", t_line), a.join(", ")));


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
fn SPECIALIZE_node_cdecl_path(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_cdecl_path_struct>(n);
    let t_parent = specialize_node(sc, u.parent);
    let t_v = specialize_node(sc, u.v);
    let mut t_line = "crate::vm::LineAt".to_string();
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push("v_parent".to_string());
    a.push("u.name".to_string());
    a.push("v_v".to_string());
    if sc.spec_depth == 0 { sc.holes.push((n as usize + std::mem::offset_of!(NodeOf<node_cdecl_path_struct>, head.line), std::mem::size_of::<u32>())); } if let Some(s) = sc.hole_sym("line", n as usize + std::mem::offset_of!(NodeOf<node_cdecl_path_struct>, head.line), "impl Clone for H_%SYM% { #[inline(always)] fn clone(&self) -> Self { *self } } impl Copy for H_%SYM% {} impl crate::vm::LineHole for H_%SYM% {   #[inline(always)] fn pos(self) -> crate::vm::Pos { unsafe { crate::vm::Pos::bound(&raw const %SYM%) } }   #[inline(always)] fn store(self, f: &mut crate::vm::Frame) { unsafe {     std::arch::asm!(\"mov qword ptr [{0} + {1}], offset {2}\", in(reg) f, const std::mem::offset_of!(crate::vm::Frame, pos), sym %SYM%, options(nostack, preserves_flags)) } } } ") { t_line = "H_%SYM%".replace("%SYM%", &s); a.push("H_%SYM%".replace("%SYM%", &s)); } else { a.push("crate::vm::LineAt(&(*node_head_line(n)))".to_string()); }
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_cdecl_path_struct>({});\n", "n"));
    body.push_str(&format!("        let v_parent = <{} as Disp>::call(c, {}, fp)?;\n", t_parent, child_ptr_expr(n, u.parent)));
    body.push_str(&format!("        let v_v = <{} as Disp>::call(c, {}, fp)?;\n", t_v, child_ptr_expr(n, u.v)));
    body.push_str(&format!("        EVAL_node_cdecl_path{}({})\n    }}\n", format!("::<{}>", t_line), a.join(", ")));


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
fn SPECIALIZE_node_errinfo(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_errinfo_struct>(n);

    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];

    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_errinfo_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_errinfo{}({})\n    }}\n", "", a.join(", ")));


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
fn SPECIALIZE_node_nth_ref(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_nth_ref_struct>(n);

    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push(format!("{}u32", u.nth));
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_nth_ref_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_nth_ref{}({})\n    }}\n", "", a.join(", ")));


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
fn SPECIALIZE_node_back_ref(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_back_ref_struct>(n);

    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push(format!("{}u32", u.kind));
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_back_ref_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_back_ref{}({})\n    }}\n", "", a.join(", ")));


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
fn SPECIALIZE_node_block(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_block_struct>(n);
    let t_stmts = specialize_list(sc, &u.stmts, n);
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push("list_base(n)".to_string());
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_block_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_block{}({})\n    }}\n", format!("::<{}>", t_stmts), a.join(", ")));


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
fn SPECIALIZE_node_line(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_line_struct>(n);
    let mut t_line = "crate::vm::LineAt".to_string();
    let t_body = specialize_node(sc, u.body);
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    if sc.spec_depth == 0 { sc.holes.push((n as usize + std::mem::offset_of!(NodeOf<node_line_struct>, head.line), std::mem::size_of::<u32>())); } if let Some(s) = sc.hole_sym("line", n as usize + std::mem::offset_of!(NodeOf<node_line_struct>, head.line), "impl Clone for H_%SYM% { #[inline(always)] fn clone(&self) -> Self { *self } } impl Copy for H_%SYM% {} impl crate::vm::LineHole for H_%SYM% {   #[inline(always)] fn pos(self) -> crate::vm::Pos { unsafe { crate::vm::Pos::bound(&raw const %SYM%) } }   #[inline(always)] fn store(self, f: &mut crate::vm::Frame) { unsafe {     std::arch::asm!(\"mov qword ptr [{0} + {1}], offset {2}\", in(reg) f, const std::mem::offset_of!(crate::vm::Frame, pos), sym %SYM%, options(nostack, preserves_flags)) } } } ") { t_line = "H_%SYM%".replace("%SYM%", &s); a.push("H_%SYM%".replace("%SYM%", &s)); } else { a.push("crate::vm::LineAt(&(*node_head_line(n)))".to_string()); }
    a.push(child_ptr_expr(n, u.body));
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_line_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_line{}({})\n    }}\n", format!("::<{}, {}>", t_line, t_body), a.join(", ")));


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
fn SPECIALIZE_node_trace(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_trace_struct>(n);
    let mut t_line = "crate::vm::LineAt".to_string();
    let t_body = specialize_node(sc, u.body);
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push(format!("{}u32", u.ev));
    if sc.spec_depth == 0 { sc.holes.push((n as usize + std::mem::offset_of!(NodeOf<node_trace_struct>, head.line), std::mem::size_of::<u32>())); } if let Some(s) = sc.hole_sym("line", n as usize + std::mem::offset_of!(NodeOf<node_trace_struct>, head.line), "impl Clone for H_%SYM% { #[inline(always)] fn clone(&self) -> Self { *self } } impl Copy for H_%SYM% {} impl crate::vm::LineHole for H_%SYM% {   #[inline(always)] fn pos(self) -> crate::vm::Pos { unsafe { crate::vm::Pos::bound(&raw const %SYM%) } }   #[inline(always)] fn store(self, f: &mut crate::vm::Frame) { unsafe {     std::arch::asm!(\"mov qword ptr [{0} + {1}], offset {2}\", in(reg) f, const std::mem::offset_of!(crate::vm::Frame, pos), sym %SYM%, options(nostack, preserves_flags)) } } } ") { t_line = "H_%SYM%".replace("%SYM%", &s); a.push("H_%SYM%".replace("%SYM%", &s)); } else { a.push("crate::vm::LineAt(&(*node_head_line(n)))".to_string()); }
    a.push(child_ptr_expr(n, u.body));
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_trace_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_trace{}({})\n    }}\n", format!("::<{}, {}>", t_line, t_body), a.join(", ")));


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
fn SPECIALIZE_node_poll(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_poll_struct>(n);
    let mut t_line = "crate::vm::LineAt".to_string();
    let t_body = specialize_node(sc, u.body);
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    if sc.spec_depth == 0 { sc.holes.push((n as usize + std::mem::offset_of!(NodeOf<node_poll_struct>, head.line), std::mem::size_of::<u32>())); } if let Some(s) = sc.hole_sym("line", n as usize + std::mem::offset_of!(NodeOf<node_poll_struct>, head.line), "impl Clone for H_%SYM% { #[inline(always)] fn clone(&self) -> Self { *self } } impl Copy for H_%SYM% {} impl crate::vm::LineHole for H_%SYM% {   #[inline(always)] fn pos(self) -> crate::vm::Pos { unsafe { crate::vm::Pos::bound(&raw const %SYM%) } }   #[inline(always)] fn store(self, f: &mut crate::vm::Frame) { unsafe {     std::arch::asm!(\"mov qword ptr [{0} + {1}], offset {2}\", in(reg) f, const std::mem::offset_of!(crate::vm::Frame, pos), sym %SYM%, options(nostack, preserves_flags)) } } } ") { t_line = "H_%SYM%".replace("%SYM%", &s); a.push("H_%SYM%".replace("%SYM%", &s)); } else { a.push("crate::vm::LineAt(&(*node_head_line(n)))".to_string()); }
    a.push(child_ptr_expr(n, u.body));
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_poll_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_poll{}({})\n    }}\n", format!("::<{}, {}>", t_line, t_body), a.join(", ")));


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
fn SPECIALIZE_node_cov(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_cov_struct>(n);
    let t_body = specialize_node(sc, u.body);
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push(format!("{}u32", u.id));
    a.push(child_ptr_expr(n, u.body));
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_cov_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_cov{}({})\n    }}\n", format!("::<{}>", t_body), a.join(", ")));


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
fn SPECIALIZE_node_if(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_if_struct>(n);
    let t_cond = specialize_node(sc, u.cond);
    let t_then_b = specialize_node(sc, u.then_b);
    let t_else_b = specialize_node(sc, u.else_b);
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push(child_ptr_expr(n, u.cond));
    a.push(child_ptr_expr(n, u.then_b));
    a.push(child_ptr_expr(n, u.else_b));
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_if_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_if{}({})\n    }}\n", format!("::<{}, {}, {}>", t_cond, t_then_b, t_else_b), a.join(", ")));


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
fn SPECIALIZE_node_and(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_and_struct>(n);
    let t_a = specialize_node(sc, u.a);
    let t_b = specialize_node(sc, u.b);
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push(child_ptr_expr(n, u.a));
    a.push(child_ptr_expr(n, u.b));
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_and_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_and{}({})\n    }}\n", format!("::<{}, {}>", t_a, t_b), a.join(", ")));
    body.push_str(&format!("    #[inline(always)]\n    fn test(c: &mut Ctx, n: NodePtr, fp: Fp) -> Result<bool, Esc> {{\n"));
    body.push_str(&format!("        let u = node_u::<node_and_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_TEST_node_and{}({})\n    }}\n", format!("::<{}, {}>", t_a, t_b), a.join(", ")));

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
fn SPECIALIZE_node_or(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_or_struct>(n);
    let t_a = specialize_node(sc, u.a);
    let t_b = specialize_node(sc, u.b);
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push(child_ptr_expr(n, u.a));
    a.push(child_ptr_expr(n, u.b));
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_or_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_or{}({})\n    }}\n", format!("::<{}, {}>", t_a, t_b), a.join(", ")));
    body.push_str(&format!("    #[inline(always)]\n    fn test(c: &mut Ctx, n: NodePtr, fp: Fp) -> Result<bool, Esc> {{\n"));
    body.push_str(&format!("        let u = node_u::<node_or_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_TEST_node_or{}({})\n    }}\n", format!("::<{}, {}>", t_a, t_b), a.join(", ")));

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
fn SPECIALIZE_node_newarray_minmax(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_newarray_minmax_struct>(n);
    let t_a = specialize_node(sc, u.a);
    let mut t_line = "crate::vm::LineAt".to_string();
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push("v_a".to_string());
    a.push(format!("{}u32", u.mx));
    if sc.spec_depth == 0 { sc.holes.push((n as usize + std::mem::offset_of!(NodeOf<node_newarray_minmax_struct>, head.line), std::mem::size_of::<u32>())); } if let Some(s) = sc.hole_sym("line", n as usize + std::mem::offset_of!(NodeOf<node_newarray_minmax_struct>, head.line), "impl Clone for H_%SYM% { #[inline(always)] fn clone(&self) -> Self { *self } } impl Copy for H_%SYM% {} impl crate::vm::LineHole for H_%SYM% {   #[inline(always)] fn pos(self) -> crate::vm::Pos { unsafe { crate::vm::Pos::bound(&raw const %SYM%) } }   #[inline(always)] fn store(self, f: &mut crate::vm::Frame) { unsafe {     std::arch::asm!(\"mov qword ptr [{0} + {1}], offset {2}\", in(reg) f, const std::mem::offset_of!(crate::vm::Frame, pos), sym %SYM%, options(nostack, preserves_flags)) } } } ") { t_line = "H_%SYM%".replace("%SYM%", &s); a.push("H_%SYM%".replace("%SYM%", &s)); } else { a.push("crate::vm::LineAt(&(*node_head_line(n)))".to_string()); }
    a.push("&u.cc".to_string());
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_newarray_minmax_struct>({});\n", "n"));
    body.push_str(&format!("        let v_a = <{} as Disp>::call(c, {}, fp)?;\n", t_a, child_ptr_expr(n, u.a)));
    body.push_str(&format!("        EVAL_node_newarray_minmax{}({})\n    }}\n", format!("::<{}>", t_line), a.join(", ")));


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
fn SPECIALIZE_node_not(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_not_struct>(n);
    let t_v = specialize_node(sc, u.v);
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push("v_v".to_string());
    a.push("&u.cc".to_string());
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_not_struct>({});\n", "n"));
    body.push_str(&format!("        let v_v = <{} as Disp>::call(c, {}, fp)?;\n", t_v, child_ptr_expr(n, u.v)));
    body.push_str(&format!("        EVAL_node_not{}({})\n    }}\n", "", a.join(", ")));
    body.push_str(&format!("    #[inline(always)]\n    fn test(c: &mut Ctx, n: NodePtr, fp: Fp) -> Result<bool, Esc> {{\n"));
    body.push_str(&format!("        let u = node_u::<node_not_struct>({});\n", "n"));
    body.push_str(&format!("        let v_v = <{} as Disp>::call(c, {}, fp)?;\n", t_v, child_ptr_expr(n, u.v)));
    body.push_str(&format!("        EVAL_TEST_node_not{}({})\n    }}\n", "", a.join(", ")));

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
fn SPECIALIZE_node_while(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_while_struct>(n);
    let t_cond = specialize_node(sc, u.cond);
    let t_body = specialize_node(sc, u.body);
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push(child_ptr_expr(n, u.cond));
    a.push(child_ptr_expr(n, u.body));
    a.push(format!("{}", u.until));
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_while_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_while{}({})\n    }}\n", format!("::<{}, {}>", t_cond, t_body), a.join(", ")));


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
fn SPECIALIZE_node_dowhile(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_dowhile_struct>(n);
    let t_cond = specialize_node(sc, u.cond);
    let t_body = specialize_node(sc, u.body);
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push(child_ptr_expr(n, u.cond));
    a.push(child_ptr_expr(n, u.body));
    a.push(format!("{}", u.until));
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_dowhile_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_dowhile{}({})\n    }}\n", format!("::<{}, {}>", t_cond, t_body), a.join(", ")));


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
fn SPECIALIZE_node_loop_break(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_loop_break_struct>(n);
    let t_v = specialize_node(sc, u.v);
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push("v_v".to_string());
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_loop_break_struct>({});\n", "n"));
    body.push_str(&format!("        let v_v = <{} as Disp>::call(c, {}, fp)?;\n", t_v, child_ptr_expr(n, u.v)));
    body.push_str(&format!("        EVAL_node_loop_break{}({})\n    }}\n", "", a.join(", ")));


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
fn SPECIALIZE_node_loop_next(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_loop_next_struct>(n);
    let t_v = specialize_node(sc, u.v);
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push("v_v".to_string());
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_loop_next_struct>({});\n", "n"));
    body.push_str(&format!("        let v_v = <{} as Disp>::call(c, {}, fp)?;\n", t_v, child_ptr_expr(n, u.v)));
    body.push_str(&format!("        EVAL_node_loop_next{}({})\n    }}\n", "", a.join(", ")));


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
fn SPECIALIZE_node_break(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_break_struct>(n);
    let t_v = specialize_node(sc, u.v);
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push("v_v".to_string());
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_break_struct>({});\n", "n"));
    body.push_str(&format!("        let v_v = <{} as Disp>::call(c, {}, fp)?;\n", t_v, child_ptr_expr(n, u.v)));
    body.push_str(&format!("        EVAL_node_break{}({})\n    }}\n", "", a.join(", ")));


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
fn SPECIALIZE_node_next(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_next_struct>(n);
    let t_v = specialize_node(sc, u.v);
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push("v_v".to_string());
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_next_struct>({});\n", "n"));
    body.push_str(&format!("        let v_v = <{} as Disp>::call(c, {}, fp)?;\n", t_v, child_ptr_expr(n, u.v)));
    body.push_str(&format!("        EVAL_node_next{}({})\n    }}\n", "", a.join(", ")));


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
fn SPECIALIZE_node_return(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_return_struct>(n);
    let t_v = specialize_node(sc, u.v);
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push("v_v".to_string());
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_return_struct>({});\n", "n"));
    body.push_str(&format!("        let v_v = <{} as Disp>::call(c, {}, fp)?;\n", t_v, child_ptr_expr(n, u.v)));
    body.push_str(&format!("        EVAL_node_return{}({})\n    }}\n", "", a.join(", ")));


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
fn SPECIALIZE_node_retry(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_retry_struct>(n);

    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];

    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_retry_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_retry{}({})\n    }}\n", "", a.join(", ")));


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
fn SPECIALIZE_node_redo(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_redo_struct>(n);

    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];

    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_redo_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_redo{}({})\n    }}\n", "", a.join(", ")));


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
fn SPECIALIZE_node_rescue(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_rescue_struct>(n);
    let t_body = specialize_node(sc, u.body);
    let t_clauses = specialize_list(sc, &u.clauses, n);
    let t_else_b = specialize_node(sc, u.else_b);
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push(child_ptr_expr(n, u.body));
    a.push("list_base(n)".to_string());
    a.push(child_ptr_expr(n, u.else_b));
    a.push(format!("{}", u.has_else));
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_rescue_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_rescue{}({})\n    }}\n", format!("::<{}, {}, {}>", t_body, t_clauses, t_else_b), a.join(", ")));


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
fn SPECIALIZE_node_resbody(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_resbody_struct>(n);
    let t_classes = specialize_list(sc, &u.classes, n);
    let t_assign = specialize_node(sc, u.assign);
    let t_body = specialize_node(sc, u.body);
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push("list_base(n)".to_string());
    a.push("u.kinds".to_string());
    a.push(child_ptr_expr(n, u.assign));
    a.push(child_ptr_expr(n, u.body));
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_resbody_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_resbody{}({})\n    }}\n", format!("::<{}, {}, {}>", t_classes, t_assign, t_body), a.join(", ")));


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
fn SPECIALIZE_node_ensure(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_ensure_struct>(n);
    let t_body = specialize_node(sc, u.body);
    let t_ens = specialize_node(sc, u.ens);
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push(child_ptr_expr(n, u.body));
    a.push(child_ptr_expr(n, u.ens));
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_ensure_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_ensure{}({})\n    }}\n", format!("::<{}, {}>", t_body, t_ens), a.join(", ")));


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
fn SPECIALIZE_node_when(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_when_struct>(n);
    let t_pats = specialize_list(sc, &u.pats, n);
    let mut t_line = "crate::vm::LineAt".to_string();
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push(format!("{}u32", u.subj_slot));
    a.push("list_base(n)".to_string());
    a.push("u.kinds".to_string());
    if sc.spec_depth == 0 { sc.holes.push((n as usize + std::mem::offset_of!(NodeOf<node_when_struct>, head.line), std::mem::size_of::<u32>())); } if let Some(s) = sc.hole_sym("line", n as usize + std::mem::offset_of!(NodeOf<node_when_struct>, head.line), "impl Clone for H_%SYM% { #[inline(always)] fn clone(&self) -> Self { *self } } impl Copy for H_%SYM% {} impl crate::vm::LineHole for H_%SYM% {   #[inline(always)] fn pos(self) -> crate::vm::Pos { unsafe { crate::vm::Pos::bound(&raw const %SYM%) } }   #[inline(always)] fn store(self, f: &mut crate::vm::Frame) { unsafe {     std::arch::asm!(\"mov qword ptr [{0} + {1}], offset {2}\", in(reg) f, const std::mem::offset_of!(crate::vm::Frame, pos), sym %SYM%, options(nostack, preserves_flags)) } } } ") { t_line = "H_%SYM%".replace("%SYM%", &s); a.push("H_%SYM%".replace("%SYM%", &s)); } else { a.push("crate::vm::LineAt(&(*node_head_line(n)))".to_string()); }
    a.push("&u.cc".to_string());
    a.push("&u.km".to_string());
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_when_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_when{}({})\n    }}\n", format!("::<{}, {}>", t_pats, t_line), a.join(", ")));


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
fn SPECIALIZE_node_when_lit(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_when_lit_struct>(n);
    let mut t_line = "crate::vm::LineAt".to_string();
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push(format!("{}u32", u.subj_slot));
    a.push(crate::compile::value_bake(u.v).unwrap_or_else(|| "u.v".to_string()));
    if sc.spec_depth == 0 { sc.holes.push((n as usize + std::mem::offset_of!(NodeOf<node_when_lit_struct>, head.line), std::mem::size_of::<u32>())); } if let Some(s) = sc.hole_sym("line", n as usize + std::mem::offset_of!(NodeOf<node_when_lit_struct>, head.line), "impl Clone for H_%SYM% { #[inline(always)] fn clone(&self) -> Self { *self } } impl Copy for H_%SYM% {} impl crate::vm::LineHole for H_%SYM% {   #[inline(always)] fn pos(self) -> crate::vm::Pos { unsafe { crate::vm::Pos::bound(&raw const %SYM%) } }   #[inline(always)] fn store(self, f: &mut crate::vm::Frame) { unsafe {     std::arch::asm!(\"mov qword ptr [{0} + {1}], offset {2}\", in(reg) f, const std::mem::offset_of!(crate::vm::Frame, pos), sym %SYM%, options(nostack, preserves_flags)) } } } ") { t_line = "H_%SYM%".replace("%SYM%", &s); a.push("H_%SYM%".replace("%SYM%", &s)); } else { a.push("crate::vm::LineAt(&(*node_head_line(n)))".to_string()); }
    a.push("&u.cc".to_string());
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_when_lit_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_when_lit{}({})\n    }}\n", format!("::<{}>", t_line), a.join(", ")));
    body.push_str(&format!("    #[inline(always)]\n    fn test(c: &mut Ctx, n: NodePtr, fp: Fp) -> Result<bool, Esc> {{\n"));
    body.push_str(&format!("        let u = node_u::<node_when_lit_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_TEST_node_when_lit{}({})\n    }}\n", format!("::<{}>", t_line), a.join(", ")));

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
fn SPECIALIZE_node_defined(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_defined_struct>(n);
    let t_recv = specialize_node(sc, u.recv);
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push(format!("{}u32", u.kind));
    a.push("u.name".to_string());
    a.push(child_ptr_expr(n, u.recv));
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_defined_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_defined{}({})\n    }}\n", format!("::<{}>", t_recv), a.join(", ")));


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
fn SPECIALIZE_node_call(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_call_struct>(n);
    let t_recv = specialize_node(sc, u.recv);
    let t_args = specialize_list(sc, &u.args, n);
    let mut t_line = "crate::vm::LineAt".to_string();
    let (t_callee, h_callee) = { let p: NodePtr = { crate::vm::spec_callee(&u.cc, u.args.len(), u.flags) }; if p.is_null() || sc.spec_depth >= sc.spec_max_depth { { let q: NodePtr = { crate::vm::spec_callee_rec(&u.cc, u.args.len(), u.flags) }; if sc.spec_max_depth > 0 && !q.is_null() && sc.open.contains(&hash_node(q)) { (specialize_rec(sc, q), hash_node(q)) } else { ("Dyn".to_string(), 0u64) } } } else if sc.open.contains(&hash_node(p)) { if sc.rec_unroll && sc.unrolled.insert(hash_node(p)) { sc.spec_depth += 1; let t = specialize_open(sc, p); sc.spec_depth -= 1; sc.unrolled.remove(&hash_node(p)); (t, hash_node(p)) } else { let q: NodePtr = { crate::vm::spec_callee_rec(&u.cc, u.args.len(), u.flags) }; if sc.spec_max_depth > 0 && !q.is_null() && sc.open.contains(&hash_node(q)) { (specialize_rec(sc, q), hash_node(q)) } else { ("Dyn".to_string(), 0u64) } } } else { sc.spec_depth += 1; let t = specialize_node(sc, p); sc.spec_depth -= 1; (t, hash_node(p)) } };
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push(child_ptr_expr(n, u.recv));
    a.push("list_base(n)".to_string());
    a.push("u.mid".to_string());
    a.push(format!("{}u32", u.flags));
    if sc.spec_depth == 0 { sc.holes.push((n as usize + std::mem::offset_of!(NodeOf<node_call_struct>, head.line), std::mem::size_of::<u32>())); } if let Some(s) = sc.hole_sym("line", n as usize + std::mem::offset_of!(NodeOf<node_call_struct>, head.line), "impl Clone for H_%SYM% { #[inline(always)] fn clone(&self) -> Self { *self } } impl Copy for H_%SYM% {} impl crate::vm::LineHole for H_%SYM% {   #[inline(always)] fn pos(self) -> crate::vm::Pos { unsafe { crate::vm::Pos::bound(&raw const %SYM%) } }   #[inline(always)] fn store(self, f: &mut crate::vm::Frame) { unsafe {     std::arch::asm!(\"mov qword ptr [{0} + {1}], offset {2}\", in(reg) f, const std::mem::offset_of!(crate::vm::Frame, pos), sym %SYM%, options(nostack, preserves_flags)) } } } ") { t_line = "H_%SYM%".replace("%SYM%", &s); a.push("H_%SYM%".replace("%SYM%", &s)); } else { a.push("crate::vm::LineAt(&(*node_head_line(n)))".to_string()); }
    a.push("&u.cc".to_string());
    a.push(format!("{}u64", h_callee));
    a.push(format!("{}u32", { crate::vm::site_kind(&u.cc, u.args.len(), u.flags) }));
    a.push(format!("{}u32", { crate::vm::spec_shared(u.recv, u.args.len(), crate::vm::spec_callee(&u.cc, u.args.len(), u.flags)) }));
    a.push(format!("{}u32", { crate::inline_builtin::site_builtin(u.args.len(), crate::vm::site_kind(&u.cc, u.args.len(), u.flags), u.mid) }));
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_call_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_call{}({})\n    }}\n", format!("::<{}, {}, {}, {}>", t_recv, t_args, t_line, t_callee), a.join(", ")));


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
fn SPECIALIZE_node_call_blk(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_call_blk_struct>(n);
    let t_recv = specialize_node(sc, u.recv);
    let t_args = specialize_list(sc, &u.args, n);
    let mut t_line = "crate::vm::LineAt".to_string();
    let (t_callee, h_callee) = { let p: NodePtr = { crate::vm::spec_callee(&u.cc, u.args.len(), u.flags) }; if p.is_null() || sc.spec_depth >= sc.spec_max_depth { { let q: NodePtr = { crate::vm::spec_callee_rec(&u.cc, u.args.len(), u.flags) }; if sc.spec_max_depth > 0 && !q.is_null() && sc.open.contains(&hash_node(q)) { (specialize_rec(sc, q), hash_node(q)) } else { ("Dyn".to_string(), 0u64) } } } else if sc.open.contains(&hash_node(p)) { if sc.rec_unroll && sc.unrolled.insert(hash_node(p)) { sc.spec_depth += 1; let t = specialize_open(sc, p); sc.spec_depth -= 1; sc.unrolled.remove(&hash_node(p)); (t, hash_node(p)) } else { let q: NodePtr = { crate::vm::spec_callee_rec(&u.cc, u.args.len(), u.flags) }; if sc.spec_max_depth > 0 && !q.is_null() && sc.open.contains(&hash_node(q)) { (specialize_rec(sc, q), hash_node(q)) } else { ("Dyn".to_string(), 0u64) } } } else { sc.spec_depth += 1; let t = specialize_node(sc, p); sc.spec_depth -= 1; (t, hash_node(p)) } };
    let (t_body, h_body) = { let p: NodePtr = { crate::inline_iter::spec_iter_block(&u.cc, u.args.len(), crate::vm::site_kind(&u.cc, u.args.len(), u.flags), u.blk) }; if p.is_null() || sc.spec_depth >= sc.spec_max_depth { ("Dyn".to_string(), 0u64) } else if sc.open.contains(&hash_node(p)) { ("Dyn".to_string(), 0u64) } else { sc.spec_depth += 1; let t = specialize_node(sc, p); sc.spec_depth -= 1; (t, hash_node(p)) } };
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push(child_ptr_expr(n, u.recv));
    a.push("list_base(n)".to_string());
    a.push("u.mid".to_string());
    a.push(format!("{}u32", u.flags));
    if sc.spec_depth == 0 { sc.holes.push((n as usize + std::mem::offset_of!(NodeOf<node_call_blk_struct>, head.line), std::mem::size_of::<u32>())); } if let Some(s) = sc.hole_sym("line", n as usize + std::mem::offset_of!(NodeOf<node_call_blk_struct>, head.line), "impl Clone for H_%SYM% { #[inline(always)] fn clone(&self) -> Self { *self } } impl Copy for H_%SYM% {} impl crate::vm::LineHole for H_%SYM% {   #[inline(always)] fn pos(self) -> crate::vm::Pos { unsafe { crate::vm::Pos::bound(&raw const %SYM%) } }   #[inline(always)] fn store(self, f: &mut crate::vm::Frame) { unsafe {     std::arch::asm!(\"mov qword ptr [{0} + {1}], offset {2}\", in(reg) f, const std::mem::offset_of!(crate::vm::Frame, pos), sym %SYM%, options(nostack, preserves_flags)) } } } ") { t_line = "H_%SYM%".replace("%SYM%", &s); a.push("H_%SYM%".replace("%SYM%", &s)); } else { a.push("crate::vm::LineAt(&(*node_head_line(n)))".to_string()); }
    a.push("u.blk".to_string());
    a.push("&u.cc".to_string());
    a.push(format!("{}u64", h_callee));
    a.push(format!("{}u32", { crate::vm::site_kind(&u.cc, u.args.len(), u.flags) }));
    a.push(format!("{}u64", h_body));
    a.push(format!("{}u32", { crate::inline_iter::site_iter(&u.cc, u.args.len(), crate::vm::site_kind(&u.cc, u.args.len(), u.flags), u.blk) }));
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_call_blk_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_call_blk{}({})\n    }}\n", format!("::<{}, {}, {}, {}, {}>", t_recv, t_args, t_line, t_callee, t_body), a.join(", ")));


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
fn SPECIALIZE_node_call_kw(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_call_kw_struct>(n);
    let t_recv = specialize_node(sc, u.recv);
    let t_args = specialize_list(sc, &u.args, n);
    let mut t_line = "crate::vm::LineAt".to_string();
    let (t_callee, h_callee) = { let p: NodePtr = { crate::vm::spec_callee_kw(&u.cc, u.args.len(), u.ci) }; if p.is_null() || sc.spec_depth >= sc.spec_max_depth { { let q: NodePtr = { crate::vm::spec_callee_kw_rec(&u.cc, u.args.len(), u.ci) }; if sc.spec_max_depth > 0 && !q.is_null() && sc.open.contains(&hash_node(q)) { (specialize_rec(sc, q), hash_node(q)) } else { ("Dyn".to_string(), 0u64) } } } else if sc.open.contains(&hash_node(p)) { if sc.rec_unroll && sc.unrolled.insert(hash_node(p)) { sc.spec_depth += 1; let t = specialize_open(sc, p); sc.spec_depth -= 1; sc.unrolled.remove(&hash_node(p)); (t, hash_node(p)) } else { let q: NodePtr = { crate::vm::spec_callee_kw_rec(&u.cc, u.args.len(), u.ci) }; if sc.spec_max_depth > 0 && !q.is_null() && sc.open.contains(&hash_node(q)) { (specialize_rec(sc, q), hash_node(q)) } else { ("Dyn".to_string(), 0u64) } } } else { sc.spec_depth += 1; let t = specialize_node(sc, p); sc.spec_depth -= 1; (t, hash_node(p)) } };
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push(child_ptr_expr(n, u.recv));
    a.push("list_base(n)".to_string());
    a.push("u.mid".to_string());
    a.push(format!("{}u32", u.flags));
    if sc.spec_depth == 0 { sc.holes.push((n as usize + std::mem::offset_of!(NodeOf<node_call_kw_struct>, head.line), std::mem::size_of::<u32>())); } if let Some(s) = sc.hole_sym("line", n as usize + std::mem::offset_of!(NodeOf<node_call_kw_struct>, head.line), "impl Clone for H_%SYM% { #[inline(always)] fn clone(&self) -> Self { *self } } impl Copy for H_%SYM% {} impl crate::vm::LineHole for H_%SYM% {   #[inline(always)] fn pos(self) -> crate::vm::Pos { unsafe { crate::vm::Pos::bound(&raw const %SYM%) } }   #[inline(always)] fn store(self, f: &mut crate::vm::Frame) { unsafe {     std::arch::asm!(\"mov qword ptr [{0} + {1}], offset {2}\", in(reg) f, const std::mem::offset_of!(crate::vm::Frame, pos), sym %SYM%, options(nostack, preserves_flags)) } } } ") { t_line = "H_%SYM%".replace("%SYM%", &s); a.push("H_%SYM%".replace("%SYM%", &s)); } else { a.push("crate::vm::LineAt(&(*node_head_line(n)))".to_string()); }
    a.push("u.ci".to_string());
    a.push("&u.cc".to_string());
    a.push(format!("{}u64", h_callee));
    a.push(format!("{}u32", { crate::vm::site_kind_kw(&u.cc, u.args.len(), u.ci) }));
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_call_kw_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_call_kw{}({})\n    }}\n", format!("::<{}, {}, {}, {}>", t_recv, t_args, t_line, t_callee), a.join(", ")));


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
fn SPECIALIZE_node_call_x(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_call_x_struct>(n);
    let t_recv = specialize_node(sc, u.recv);
    let t_args = specialize_list(sc, &u.args, n);
    let mut t_line = "crate::vm::LineAt".to_string();
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push(child_ptr_expr(n, u.recv));
    a.push("list_base(n)".to_string());
    a.push("u.mid".to_string());
    a.push(format!("{}u32", u.flags));
    if sc.spec_depth == 0 { sc.holes.push((n as usize + std::mem::offset_of!(NodeOf<node_call_x_struct>, head.line), std::mem::size_of::<u32>())); } if let Some(s) = sc.hole_sym("line", n as usize + std::mem::offset_of!(NodeOf<node_call_x_struct>, head.line), "impl Clone for H_%SYM% { #[inline(always)] fn clone(&self) -> Self { *self } } impl Copy for H_%SYM% {} impl crate::vm::LineHole for H_%SYM% {   #[inline(always)] fn pos(self) -> crate::vm::Pos { unsafe { crate::vm::Pos::bound(&raw const %SYM%) } }   #[inline(always)] fn store(self, f: &mut crate::vm::Frame) { unsafe {     std::arch::asm!(\"mov qword ptr [{0} + {1}], offset {2}\", in(reg) f, const std::mem::offset_of!(crate::vm::Frame, pos), sym %SYM%, options(nostack, preserves_flags)) } } } ") { t_line = "H_%SYM%".replace("%SYM%", &s); a.push("H_%SYM%".replace("%SYM%", &s)); } else { a.push("crate::vm::LineAt(&(*node_head_line(n)))".to_string()); }
    a.push("u.ci".to_string());
    a.push("&u.cc".to_string());
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_call_x_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_call_x{}({})\n    }}\n", format!("::<{}, {}, {}>", t_recv, t_args, t_line), a.join(", ")));


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
fn SPECIALIZE_node_send_sym(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_send_sym_struct>(n);
    let t_recv = specialize_node(sc, u.recv);
    let t_call = specialize_node(sc, u.call);
    let t_general = specialize_node(sc, u.general);
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push("v_recv".to_string());
    a.push("u.smid".to_string());
    a.push(child_ptr_expr(n, u.call));
    a.push(child_ptr_expr(n, u.general));
    a.push("&u.g".to_string());
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_send_sym_struct>({});\n", "n"));
    body.push_str(&format!("        let v_recv = <{} as Disp>::call(c, {}, fp)?;\n", t_recv, child_ptr_expr(n, u.recv)));
    body.push_str(&format!("        EVAL_node_send_sym{}({})\n    }}\n", format!("::<{}, {}>", t_call, t_general), a.join(", ")));


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
fn SPECIALIZE_node_attrasgn(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_attrasgn_struct>(n);
    let t_recv = specialize_node(sc, u.recv);
    let t_args = specialize_list(sc, &u.args, n);
    let mut t_line = "crate::vm::LineAt".to_string();
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push(child_ptr_expr(n, u.recv));
    a.push("list_base(n)".to_string());
    a.push("u.mid".to_string());
    a.push(format!("{}u32", u.flags));
    if sc.spec_depth == 0 { sc.holes.push((n as usize + std::mem::offset_of!(NodeOf<node_attrasgn_struct>, head.line), std::mem::size_of::<u32>())); } if let Some(s) = sc.hole_sym("line", n as usize + std::mem::offset_of!(NodeOf<node_attrasgn_struct>, head.line), "impl Clone for H_%SYM% { #[inline(always)] fn clone(&self) -> Self { *self } } impl Copy for H_%SYM% {} impl crate::vm::LineHole for H_%SYM% {   #[inline(always)] fn pos(self) -> crate::vm::Pos { unsafe { crate::vm::Pos::bound(&raw const %SYM%) } }   #[inline(always)] fn store(self, f: &mut crate::vm::Frame) { unsafe {     std::arch::asm!(\"mov qword ptr [{0} + {1}], offset {2}\", in(reg) f, const std::mem::offset_of!(crate::vm::Frame, pos), sym %SYM%, options(nostack, preserves_flags)) } } } ") { t_line = "H_%SYM%".replace("%SYM%", &s); a.push("H_%SYM%".replace("%SYM%", &s)); } else { a.push("crate::vm::LineAt(&(*node_head_line(n)))".to_string()); }
    a.push("&u.cc".to_string());
    a.push(format!("{}u32", { crate::vm::site_kind(&u.cc, u.args.len(), u.flags) }));
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_attrasgn_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_attrasgn{}({})\n    }}\n", format!("::<{}, {}, {}>", t_recv, t_args, t_line), a.join(", ")));


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
fn SPECIALIZE_node_super(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_super_struct>(n);
    let t_args = specialize_list(sc, &u.args, n);
    let mut t_line = "crate::vm::LineAt".to_string();
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push("list_base(n)".to_string());
    if sc.spec_depth == 0 { sc.holes.push((n as usize + std::mem::offset_of!(NodeOf<node_super_struct>, head.line), std::mem::size_of::<u32>())); } if let Some(s) = sc.hole_sym("line", n as usize + std::mem::offset_of!(NodeOf<node_super_struct>, head.line), "impl Clone for H_%SYM% { #[inline(always)] fn clone(&self) -> Self { *self } } impl Copy for H_%SYM% {} impl crate::vm::LineHole for H_%SYM% {   #[inline(always)] fn pos(self) -> crate::vm::Pos { unsafe { crate::vm::Pos::bound(&raw const %SYM%) } }   #[inline(always)] fn store(self, f: &mut crate::vm::Frame) { unsafe {     std::arch::asm!(\"mov qword ptr [{0} + {1}], offset {2}\", in(reg) f, const std::mem::offset_of!(crate::vm::Frame, pos), sym %SYM%, options(nostack, preserves_flags)) } } } ") { t_line = "H_%SYM%".replace("%SYM%", &s); a.push("H_%SYM%".replace("%SYM%", &s)); } else { a.push("crate::vm::LineAt(&(*node_head_line(n)))".to_string()); }
    a.push("u.ci".to_string());
    a.push(format!("{}", u.zsuper));
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_super_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_super{}({})\n    }}\n", format!("::<{}, {}>", t_args, t_line), a.join(", ")));


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
fn SPECIALIZE_node_yield(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_yield_struct>(n);
    let t_args = specialize_list(sc, &u.args, n);
    let (t_callee, h_callee) = { let p: NodePtr = { crate::vm::spec_block(&u.yc, u.args.len()) }; if p.is_null() || sc.spec_depth >= sc.spec_max_depth { ("Dyn".to_string(), 0u64) } else if sc.open.contains(&hash_node(p)) { ("Dyn".to_string(), 0u64) } else { sc.spec_depth += 1; let t = specialize_node(sc, p); sc.spec_depth -= 1; (t, hash_node(p)) } };
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push("list_base(n)".to_string());
    a.push("&u.yc".to_string());
    a.push(format!("{}u64", h_callee));
    a.push(format!("{}u32", { crate::vm::spec_block_np(&u.yc, u.args.len()) }));
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_yield_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_yield{}({})\n    }}\n", format!("::<{}, {}>", t_args, t_callee), a.join(", ")));


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
fn SPECIALIZE_node_yield_x(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_yield_x_struct>(n);
    let t_args = specialize_list(sc, &u.args, n);
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push("list_base(n)".to_string());
    a.push("u.ci".to_string());
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_yield_x_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_yield_x{}({})\n    }}\n", format!("::<{}>", t_args), a.join(", ")));


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
fn SPECIALIZE_node_block_given(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_block_given_struct>(n);

    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];

    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_block_given_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_block_given{}({})\n    }}\n", "", a.join(", ")));


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
fn SPECIALIZE_node_lambda(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_lambda_struct>(n);

    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push("u.blk".to_string());
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_lambda_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_lambda{}({})\n    }}\n", "", a.join(", ")));


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
fn SPECIALIZE_node_opt_plus(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_opt_plus_struct>(n);
    let t_a = specialize_node(sc, u.a);
    let t_b = specialize_node(sc, u.b);
    let mut t_line = "crate::vm::LineAt".to_string();
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push("v_a".to_string());
    a.push("v_b".to_string());
    if sc.spec_depth == 0 { sc.holes.push((n as usize + std::mem::offset_of!(NodeOf<node_opt_plus_struct>, head.line), std::mem::size_of::<u32>())); } if let Some(s) = sc.hole_sym("line", n as usize + std::mem::offset_of!(NodeOf<node_opt_plus_struct>, head.line), "impl Clone for H_%SYM% { #[inline(always)] fn clone(&self) -> Self { *self } } impl Copy for H_%SYM% {} impl crate::vm::LineHole for H_%SYM% {   #[inline(always)] fn pos(self) -> crate::vm::Pos { unsafe { crate::vm::Pos::bound(&raw const %SYM%) } }   #[inline(always)] fn store(self, f: &mut crate::vm::Frame) { unsafe {     std::arch::asm!(\"mov qword ptr [{0} + {1}], offset {2}\", in(reg) f, const std::mem::offset_of!(crate::vm::Frame, pos), sym %SYM%, options(nostack, preserves_flags)) } } } ") { t_line = "H_%SYM%".replace("%SYM%", &s); a.push("H_%SYM%".replace("%SYM%", &s)); } else { a.push("crate::vm::LineAt(&(*node_head_line(n)))".to_string()); }
    a.push("&u.cc".to_string());
    a.push("&u.tf".to_string());
    a.push(format!("{}u32", { 1 + u.tf.0.get() }));
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_opt_plus_struct>({});\n", "n"));
    body.push_str(&format!("        let v_a = <{} as Disp>::call(c, {}, fp)?;\n", t_a, child_ptr_expr(n, u.a)));
    body.push_str(&format!("        let v_b = <{} as Disp>::call(c, {}, fp)?;\n", t_b, child_ptr_expr(n, u.b)));
    body.push_str(&format!("        EVAL_node_opt_plus{}({})\n    }}\n", format!("::<{}>", t_line), a.join(", ")));


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
fn SPECIALIZE_node_opt_minus(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_opt_minus_struct>(n);
    let t_a = specialize_node(sc, u.a);
    let t_b = specialize_node(sc, u.b);
    let mut t_line = "crate::vm::LineAt".to_string();
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push("v_a".to_string());
    a.push("v_b".to_string());
    if sc.spec_depth == 0 { sc.holes.push((n as usize + std::mem::offset_of!(NodeOf<node_opt_minus_struct>, head.line), std::mem::size_of::<u32>())); } if let Some(s) = sc.hole_sym("line", n as usize + std::mem::offset_of!(NodeOf<node_opt_minus_struct>, head.line), "impl Clone for H_%SYM% { #[inline(always)] fn clone(&self) -> Self { *self } } impl Copy for H_%SYM% {} impl crate::vm::LineHole for H_%SYM% {   #[inline(always)] fn pos(self) -> crate::vm::Pos { unsafe { crate::vm::Pos::bound(&raw const %SYM%) } }   #[inline(always)] fn store(self, f: &mut crate::vm::Frame) { unsafe {     std::arch::asm!(\"mov qword ptr [{0} + {1}], offset {2}\", in(reg) f, const std::mem::offset_of!(crate::vm::Frame, pos), sym %SYM%, options(nostack, preserves_flags)) } } } ") { t_line = "H_%SYM%".replace("%SYM%", &s); a.push("H_%SYM%".replace("%SYM%", &s)); } else { a.push("crate::vm::LineAt(&(*node_head_line(n)))".to_string()); }
    a.push("&u.cc".to_string());
    a.push("&u.tf".to_string());
    a.push(format!("{}u32", { 1 + u.tf.0.get() }));
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_opt_minus_struct>({});\n", "n"));
    body.push_str(&format!("        let v_a = <{} as Disp>::call(c, {}, fp)?;\n", t_a, child_ptr_expr(n, u.a)));
    body.push_str(&format!("        let v_b = <{} as Disp>::call(c, {}, fp)?;\n", t_b, child_ptr_expr(n, u.b)));
    body.push_str(&format!("        EVAL_node_opt_minus{}({})\n    }}\n", format!("::<{}>", t_line), a.join(", ")));


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
fn SPECIALIZE_node_opt_mul(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_opt_mul_struct>(n);
    let t_a = specialize_node(sc, u.a);
    let t_b = specialize_node(sc, u.b);
    let mut t_line = "crate::vm::LineAt".to_string();
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push("v_a".to_string());
    a.push("v_b".to_string());
    if sc.spec_depth == 0 { sc.holes.push((n as usize + std::mem::offset_of!(NodeOf<node_opt_mul_struct>, head.line), std::mem::size_of::<u32>())); } if let Some(s) = sc.hole_sym("line", n as usize + std::mem::offset_of!(NodeOf<node_opt_mul_struct>, head.line), "impl Clone for H_%SYM% { #[inline(always)] fn clone(&self) -> Self { *self } } impl Copy for H_%SYM% {} impl crate::vm::LineHole for H_%SYM% {   #[inline(always)] fn pos(self) -> crate::vm::Pos { unsafe { crate::vm::Pos::bound(&raw const %SYM%) } }   #[inline(always)] fn store(self, f: &mut crate::vm::Frame) { unsafe {     std::arch::asm!(\"mov qword ptr [{0} + {1}], offset {2}\", in(reg) f, const std::mem::offset_of!(crate::vm::Frame, pos), sym %SYM%, options(nostack, preserves_flags)) } } } ") { t_line = "H_%SYM%".replace("%SYM%", &s); a.push("H_%SYM%".replace("%SYM%", &s)); } else { a.push("crate::vm::LineAt(&(*node_head_line(n)))".to_string()); }
    a.push("&u.cc".to_string());
    a.push("&u.tf".to_string());
    a.push(format!("{}u32", { 1 + u.tf.0.get() }));
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_opt_mul_struct>({});\n", "n"));
    body.push_str(&format!("        let v_a = <{} as Disp>::call(c, {}, fp)?;\n", t_a, child_ptr_expr(n, u.a)));
    body.push_str(&format!("        let v_b = <{} as Disp>::call(c, {}, fp)?;\n", t_b, child_ptr_expr(n, u.b)));
    body.push_str(&format!("        EVAL_node_opt_mul{}({})\n    }}\n", format!("::<{}>", t_line), a.join(", ")));


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
fn SPECIALIZE_node_opt_div(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_opt_div_struct>(n);
    let t_a = specialize_node(sc, u.a);
    let t_b = specialize_node(sc, u.b);
    let mut t_line = "crate::vm::LineAt".to_string();
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push("v_a".to_string());
    a.push("v_b".to_string());
    if sc.spec_depth == 0 { sc.holes.push((n as usize + std::mem::offset_of!(NodeOf<node_opt_div_struct>, head.line), std::mem::size_of::<u32>())); } if let Some(s) = sc.hole_sym("line", n as usize + std::mem::offset_of!(NodeOf<node_opt_div_struct>, head.line), "impl Clone for H_%SYM% { #[inline(always)] fn clone(&self) -> Self { *self } } impl Copy for H_%SYM% {} impl crate::vm::LineHole for H_%SYM% {   #[inline(always)] fn pos(self) -> crate::vm::Pos { unsafe { crate::vm::Pos::bound(&raw const %SYM%) } }   #[inline(always)] fn store(self, f: &mut crate::vm::Frame) { unsafe {     std::arch::asm!(\"mov qword ptr [{0} + {1}], offset {2}\", in(reg) f, const std::mem::offset_of!(crate::vm::Frame, pos), sym %SYM%, options(nostack, preserves_flags)) } } } ") { t_line = "H_%SYM%".replace("%SYM%", &s); a.push("H_%SYM%".replace("%SYM%", &s)); } else { a.push("crate::vm::LineAt(&(*node_head_line(n)))".to_string()); }
    a.push("&u.cc".to_string());
    a.push("&u.tf".to_string());
    a.push(format!("{}u32", { 1 + u.tf.0.get() }));
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_opt_div_struct>({});\n", "n"));
    body.push_str(&format!("        let v_a = <{} as Disp>::call(c, {}, fp)?;\n", t_a, child_ptr_expr(n, u.a)));
    body.push_str(&format!("        let v_b = <{} as Disp>::call(c, {}, fp)?;\n", t_b, child_ptr_expr(n, u.b)));
    body.push_str(&format!("        EVAL_node_opt_div{}({})\n    }}\n", format!("::<{}>", t_line), a.join(", ")));


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
fn SPECIALIZE_node_opt_pow(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_opt_pow_struct>(n);
    let t_a = specialize_node(sc, u.a);
    let t_b = specialize_node(sc, u.b);
    let mut t_line = "crate::vm::LineAt".to_string();
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push("v_a".to_string());
    a.push("v_b".to_string());
    if sc.spec_depth == 0 { sc.holes.push((n as usize + std::mem::offset_of!(NodeOf<node_opt_pow_struct>, head.line), std::mem::size_of::<u32>())); } if let Some(s) = sc.hole_sym("line", n as usize + std::mem::offset_of!(NodeOf<node_opt_pow_struct>, head.line), "impl Clone for H_%SYM% { #[inline(always)] fn clone(&self) -> Self { *self } } impl Copy for H_%SYM% {} impl crate::vm::LineHole for H_%SYM% {   #[inline(always)] fn pos(self) -> crate::vm::Pos { unsafe { crate::vm::Pos::bound(&raw const %SYM%) } }   #[inline(always)] fn store(self, f: &mut crate::vm::Frame) { unsafe {     std::arch::asm!(\"mov qword ptr [{0} + {1}], offset {2}\", in(reg) f, const std::mem::offset_of!(crate::vm::Frame, pos), sym %SYM%, options(nostack, preserves_flags)) } } } ") { t_line = "H_%SYM%".replace("%SYM%", &s); a.push("H_%SYM%".replace("%SYM%", &s)); } else { a.push("crate::vm::LineAt(&(*node_head_line(n)))".to_string()); }
    a.push("&u.cc".to_string());
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_opt_pow_struct>({});\n", "n"));
    body.push_str(&format!("        let v_a = <{} as Disp>::call(c, {}, fp)?;\n", t_a, child_ptr_expr(n, u.a)));
    body.push_str(&format!("        let v_b = <{} as Disp>::call(c, {}, fp)?;\n", t_b, child_ptr_expr(n, u.b)));
    body.push_str(&format!("        EVAL_node_opt_pow{}({})\n    }}\n", format!("::<{}>", t_line), a.join(", ")));


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
fn SPECIALIZE_node_opt_mod(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_opt_mod_struct>(n);
    let t_a = specialize_node(sc, u.a);
    let t_b = specialize_node(sc, u.b);
    let mut t_line = "crate::vm::LineAt".to_string();
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push("v_a".to_string());
    a.push("v_b".to_string());
    if sc.spec_depth == 0 { sc.holes.push((n as usize + std::mem::offset_of!(NodeOf<node_opt_mod_struct>, head.line), std::mem::size_of::<u32>())); } if let Some(s) = sc.hole_sym("line", n as usize + std::mem::offset_of!(NodeOf<node_opt_mod_struct>, head.line), "impl Clone for H_%SYM% { #[inline(always)] fn clone(&self) -> Self { *self } } impl Copy for H_%SYM% {} impl crate::vm::LineHole for H_%SYM% {   #[inline(always)] fn pos(self) -> crate::vm::Pos { unsafe { crate::vm::Pos::bound(&raw const %SYM%) } }   #[inline(always)] fn store(self, f: &mut crate::vm::Frame) { unsafe {     std::arch::asm!(\"mov qword ptr [{0} + {1}], offset {2}\", in(reg) f, const std::mem::offset_of!(crate::vm::Frame, pos), sym %SYM%, options(nostack, preserves_flags)) } } } ") { t_line = "H_%SYM%".replace("%SYM%", &s); a.push("H_%SYM%".replace("%SYM%", &s)); } else { a.push("crate::vm::LineAt(&(*node_head_line(n)))".to_string()); }
    a.push("&u.cc".to_string());
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_opt_mod_struct>({});\n", "n"));
    body.push_str(&format!("        let v_a = <{} as Disp>::call(c, {}, fp)?;\n", t_a, child_ptr_expr(n, u.a)));
    body.push_str(&format!("        let v_b = <{} as Disp>::call(c, {}, fp)?;\n", t_b, child_ptr_expr(n, u.b)));
    body.push_str(&format!("        EVAL_node_opt_mod{}({})\n    }}\n", format!("::<{}>", t_line), a.join(", ")));


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
fn SPECIALIZE_node_opt_lt(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_opt_lt_struct>(n);
    let t_a = specialize_node(sc, u.a);
    let t_b = specialize_node(sc, u.b);
    let mut t_line = "crate::vm::LineAt".to_string();
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push("v_a".to_string());
    a.push("v_b".to_string());
    if sc.spec_depth == 0 { sc.holes.push((n as usize + std::mem::offset_of!(NodeOf<node_opt_lt_struct>, head.line), std::mem::size_of::<u32>())); } if let Some(s) = sc.hole_sym("line", n as usize + std::mem::offset_of!(NodeOf<node_opt_lt_struct>, head.line), "impl Clone for H_%SYM% { #[inline(always)] fn clone(&self) -> Self { *self } } impl Copy for H_%SYM% {} impl crate::vm::LineHole for H_%SYM% {   #[inline(always)] fn pos(self) -> crate::vm::Pos { unsafe { crate::vm::Pos::bound(&raw const %SYM%) } }   #[inline(always)] fn store(self, f: &mut crate::vm::Frame) { unsafe {     std::arch::asm!(\"mov qword ptr [{0} + {1}], offset {2}\", in(reg) f, const std::mem::offset_of!(crate::vm::Frame, pos), sym %SYM%, options(nostack, preserves_flags)) } } } ") { t_line = "H_%SYM%".replace("%SYM%", &s); a.push("H_%SYM%".replace("%SYM%", &s)); } else { a.push("crate::vm::LineAt(&(*node_head_line(n)))".to_string()); }
    a.push("&u.cc".to_string());
    a.push("&u.tf".to_string());
    a.push(format!("{}u32", { 1 + u.tf.0.get() }));
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_opt_lt_struct>({});\n", "n"));
    body.push_str(&format!("        let v_a = <{} as Disp>::call(c, {}, fp)?;\n", t_a, child_ptr_expr(n, u.a)));
    body.push_str(&format!("        let v_b = <{} as Disp>::call(c, {}, fp)?;\n", t_b, child_ptr_expr(n, u.b)));
    body.push_str(&format!("        EVAL_node_opt_lt{}({})\n    }}\n", format!("::<{}>", t_line), a.join(", ")));
    body.push_str(&format!("    #[inline(always)]\n    fn test(c: &mut Ctx, n: NodePtr, fp: Fp) -> Result<bool, Esc> {{\n"));
    body.push_str(&format!("        let u = node_u::<node_opt_lt_struct>({});\n", "n"));
    body.push_str(&format!("        let v_a = <{} as Disp>::call(c, {}, fp)?;\n", t_a, child_ptr_expr(n, u.a)));
    body.push_str(&format!("        let v_b = <{} as Disp>::call(c, {}, fp)?;\n", t_b, child_ptr_expr(n, u.b)));
    body.push_str(&format!("        EVAL_TEST_node_opt_lt{}({})\n    }}\n", format!("::<{}>", t_line), a.join(", ")));

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
fn SPECIALIZE_node_opt_le(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_opt_le_struct>(n);
    let t_a = specialize_node(sc, u.a);
    let t_b = specialize_node(sc, u.b);
    let mut t_line = "crate::vm::LineAt".to_string();
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push("v_a".to_string());
    a.push("v_b".to_string());
    if sc.spec_depth == 0 { sc.holes.push((n as usize + std::mem::offset_of!(NodeOf<node_opt_le_struct>, head.line), std::mem::size_of::<u32>())); } if let Some(s) = sc.hole_sym("line", n as usize + std::mem::offset_of!(NodeOf<node_opt_le_struct>, head.line), "impl Clone for H_%SYM% { #[inline(always)] fn clone(&self) -> Self { *self } } impl Copy for H_%SYM% {} impl crate::vm::LineHole for H_%SYM% {   #[inline(always)] fn pos(self) -> crate::vm::Pos { unsafe { crate::vm::Pos::bound(&raw const %SYM%) } }   #[inline(always)] fn store(self, f: &mut crate::vm::Frame) { unsafe {     std::arch::asm!(\"mov qword ptr [{0} + {1}], offset {2}\", in(reg) f, const std::mem::offset_of!(crate::vm::Frame, pos), sym %SYM%, options(nostack, preserves_flags)) } } } ") { t_line = "H_%SYM%".replace("%SYM%", &s); a.push("H_%SYM%".replace("%SYM%", &s)); } else { a.push("crate::vm::LineAt(&(*node_head_line(n)))".to_string()); }
    a.push("&u.cc".to_string());
    a.push("&u.tf".to_string());
    a.push(format!("{}u32", { 1 + u.tf.0.get() }));
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_opt_le_struct>({});\n", "n"));
    body.push_str(&format!("        let v_a = <{} as Disp>::call(c, {}, fp)?;\n", t_a, child_ptr_expr(n, u.a)));
    body.push_str(&format!("        let v_b = <{} as Disp>::call(c, {}, fp)?;\n", t_b, child_ptr_expr(n, u.b)));
    body.push_str(&format!("        EVAL_node_opt_le{}({})\n    }}\n", format!("::<{}>", t_line), a.join(", ")));
    body.push_str(&format!("    #[inline(always)]\n    fn test(c: &mut Ctx, n: NodePtr, fp: Fp) -> Result<bool, Esc> {{\n"));
    body.push_str(&format!("        let u = node_u::<node_opt_le_struct>({});\n", "n"));
    body.push_str(&format!("        let v_a = <{} as Disp>::call(c, {}, fp)?;\n", t_a, child_ptr_expr(n, u.a)));
    body.push_str(&format!("        let v_b = <{} as Disp>::call(c, {}, fp)?;\n", t_b, child_ptr_expr(n, u.b)));
    body.push_str(&format!("        EVAL_TEST_node_opt_le{}({})\n    }}\n", format!("::<{}>", t_line), a.join(", ")));

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
fn SPECIALIZE_node_opt_gt(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_opt_gt_struct>(n);
    let t_a = specialize_node(sc, u.a);
    let t_b = specialize_node(sc, u.b);
    let mut t_line = "crate::vm::LineAt".to_string();
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push("v_a".to_string());
    a.push("v_b".to_string());
    if sc.spec_depth == 0 { sc.holes.push((n as usize + std::mem::offset_of!(NodeOf<node_opt_gt_struct>, head.line), std::mem::size_of::<u32>())); } if let Some(s) = sc.hole_sym("line", n as usize + std::mem::offset_of!(NodeOf<node_opt_gt_struct>, head.line), "impl Clone for H_%SYM% { #[inline(always)] fn clone(&self) -> Self { *self } } impl Copy for H_%SYM% {} impl crate::vm::LineHole for H_%SYM% {   #[inline(always)] fn pos(self) -> crate::vm::Pos { unsafe { crate::vm::Pos::bound(&raw const %SYM%) } }   #[inline(always)] fn store(self, f: &mut crate::vm::Frame) { unsafe {     std::arch::asm!(\"mov qword ptr [{0} + {1}], offset {2}\", in(reg) f, const std::mem::offset_of!(crate::vm::Frame, pos), sym %SYM%, options(nostack, preserves_flags)) } } } ") { t_line = "H_%SYM%".replace("%SYM%", &s); a.push("H_%SYM%".replace("%SYM%", &s)); } else { a.push("crate::vm::LineAt(&(*node_head_line(n)))".to_string()); }
    a.push("&u.cc".to_string());
    a.push("&u.tf".to_string());
    a.push(format!("{}u32", { 1 + u.tf.0.get() }));
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_opt_gt_struct>({});\n", "n"));
    body.push_str(&format!("        let v_a = <{} as Disp>::call(c, {}, fp)?;\n", t_a, child_ptr_expr(n, u.a)));
    body.push_str(&format!("        let v_b = <{} as Disp>::call(c, {}, fp)?;\n", t_b, child_ptr_expr(n, u.b)));
    body.push_str(&format!("        EVAL_node_opt_gt{}({})\n    }}\n", format!("::<{}>", t_line), a.join(", ")));
    body.push_str(&format!("    #[inline(always)]\n    fn test(c: &mut Ctx, n: NodePtr, fp: Fp) -> Result<bool, Esc> {{\n"));
    body.push_str(&format!("        let u = node_u::<node_opt_gt_struct>({});\n", "n"));
    body.push_str(&format!("        let v_a = <{} as Disp>::call(c, {}, fp)?;\n", t_a, child_ptr_expr(n, u.a)));
    body.push_str(&format!("        let v_b = <{} as Disp>::call(c, {}, fp)?;\n", t_b, child_ptr_expr(n, u.b)));
    body.push_str(&format!("        EVAL_TEST_node_opt_gt{}({})\n    }}\n", format!("::<{}>", t_line), a.join(", ")));

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
fn SPECIALIZE_node_opt_ge(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_opt_ge_struct>(n);
    let t_a = specialize_node(sc, u.a);
    let t_b = specialize_node(sc, u.b);
    let mut t_line = "crate::vm::LineAt".to_string();
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push("v_a".to_string());
    a.push("v_b".to_string());
    if sc.spec_depth == 0 { sc.holes.push((n as usize + std::mem::offset_of!(NodeOf<node_opt_ge_struct>, head.line), std::mem::size_of::<u32>())); } if let Some(s) = sc.hole_sym("line", n as usize + std::mem::offset_of!(NodeOf<node_opt_ge_struct>, head.line), "impl Clone for H_%SYM% { #[inline(always)] fn clone(&self) -> Self { *self } } impl Copy for H_%SYM% {} impl crate::vm::LineHole for H_%SYM% {   #[inline(always)] fn pos(self) -> crate::vm::Pos { unsafe { crate::vm::Pos::bound(&raw const %SYM%) } }   #[inline(always)] fn store(self, f: &mut crate::vm::Frame) { unsafe {     std::arch::asm!(\"mov qword ptr [{0} + {1}], offset {2}\", in(reg) f, const std::mem::offset_of!(crate::vm::Frame, pos), sym %SYM%, options(nostack, preserves_flags)) } } } ") { t_line = "H_%SYM%".replace("%SYM%", &s); a.push("H_%SYM%".replace("%SYM%", &s)); } else { a.push("crate::vm::LineAt(&(*node_head_line(n)))".to_string()); }
    a.push("&u.cc".to_string());
    a.push("&u.tf".to_string());
    a.push(format!("{}u32", { 1 + u.tf.0.get() }));
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_opt_ge_struct>({});\n", "n"));
    body.push_str(&format!("        let v_a = <{} as Disp>::call(c, {}, fp)?;\n", t_a, child_ptr_expr(n, u.a)));
    body.push_str(&format!("        let v_b = <{} as Disp>::call(c, {}, fp)?;\n", t_b, child_ptr_expr(n, u.b)));
    body.push_str(&format!("        EVAL_node_opt_ge{}({})\n    }}\n", format!("::<{}>", t_line), a.join(", ")));
    body.push_str(&format!("    #[inline(always)]\n    fn test(c: &mut Ctx, n: NodePtr, fp: Fp) -> Result<bool, Esc> {{\n"));
    body.push_str(&format!("        let u = node_u::<node_opt_ge_struct>({});\n", "n"));
    body.push_str(&format!("        let v_a = <{} as Disp>::call(c, {}, fp)?;\n", t_a, child_ptr_expr(n, u.a)));
    body.push_str(&format!("        let v_b = <{} as Disp>::call(c, {}, fp)?;\n", t_b, child_ptr_expr(n, u.b)));
    body.push_str(&format!("        EVAL_TEST_node_opt_ge{}({})\n    }}\n", format!("::<{}>", t_line), a.join(", ")));

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
fn SPECIALIZE_node_opt_eq(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_opt_eq_struct>(n);
    let t_a = specialize_node(sc, u.a);
    let t_b = specialize_node(sc, u.b);
    let mut t_line = "crate::vm::LineAt".to_string();
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push("v_a".to_string());
    a.push("v_b".to_string());
    if sc.spec_depth == 0 { sc.holes.push((n as usize + std::mem::offset_of!(NodeOf<node_opt_eq_struct>, head.line), std::mem::size_of::<u32>())); } if let Some(s) = sc.hole_sym("line", n as usize + std::mem::offset_of!(NodeOf<node_opt_eq_struct>, head.line), "impl Clone for H_%SYM% { #[inline(always)] fn clone(&self) -> Self { *self } } impl Copy for H_%SYM% {} impl crate::vm::LineHole for H_%SYM% {   #[inline(always)] fn pos(self) -> crate::vm::Pos { unsafe { crate::vm::Pos::bound(&raw const %SYM%) } }   #[inline(always)] fn store(self, f: &mut crate::vm::Frame) { unsafe {     std::arch::asm!(\"mov qword ptr [{0} + {1}], offset {2}\", in(reg) f, const std::mem::offset_of!(crate::vm::Frame, pos), sym %SYM%, options(nostack, preserves_flags)) } } } ") { t_line = "H_%SYM%".replace("%SYM%", &s); a.push("H_%SYM%".replace("%SYM%", &s)); } else { a.push("crate::vm::LineAt(&(*node_head_line(n)))".to_string()); }
    a.push("&u.cc".to_string());
    a.push("&u.tf".to_string());
    a.push(format!("{}u32", { 1 + u.tf.0.get() }));
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_opt_eq_struct>({});\n", "n"));
    body.push_str(&format!("        let v_a = <{} as Disp>::call(c, {}, fp)?;\n", t_a, child_ptr_expr(n, u.a)));
    body.push_str(&format!("        let v_b = <{} as Disp>::call(c, {}, fp)?;\n", t_b, child_ptr_expr(n, u.b)));
    body.push_str(&format!("        EVAL_node_opt_eq{}({})\n    }}\n", format!("::<{}>", t_line), a.join(", ")));
    body.push_str(&format!("    #[inline(always)]\n    fn test(c: &mut Ctx, n: NodePtr, fp: Fp) -> Result<bool, Esc> {{\n"));
    body.push_str(&format!("        let u = node_u::<node_opt_eq_struct>({});\n", "n"));
    body.push_str(&format!("        let v_a = <{} as Disp>::call(c, {}, fp)?;\n", t_a, child_ptr_expr(n, u.a)));
    body.push_str(&format!("        let v_b = <{} as Disp>::call(c, {}, fp)?;\n", t_b, child_ptr_expr(n, u.b)));
    body.push_str(&format!("        EVAL_TEST_node_opt_eq{}({})\n    }}\n", format!("::<{}>", t_line), a.join(", ")));

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
fn SPECIALIZE_node_opt_neq(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_opt_neq_struct>(n);
    let t_a = specialize_node(sc, u.a);
    let t_b = specialize_node(sc, u.b);
    let mut t_line = "crate::vm::LineAt".to_string();
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push("v_a".to_string());
    a.push("v_b".to_string());
    if sc.spec_depth == 0 { sc.holes.push((n as usize + std::mem::offset_of!(NodeOf<node_opt_neq_struct>, head.line), std::mem::size_of::<u32>())); } if let Some(s) = sc.hole_sym("line", n as usize + std::mem::offset_of!(NodeOf<node_opt_neq_struct>, head.line), "impl Clone for H_%SYM% { #[inline(always)] fn clone(&self) -> Self { *self } } impl Copy for H_%SYM% {} impl crate::vm::LineHole for H_%SYM% {   #[inline(always)] fn pos(self) -> crate::vm::Pos { unsafe { crate::vm::Pos::bound(&raw const %SYM%) } }   #[inline(always)] fn store(self, f: &mut crate::vm::Frame) { unsafe {     std::arch::asm!(\"mov qword ptr [{0} + {1}], offset {2}\", in(reg) f, const std::mem::offset_of!(crate::vm::Frame, pos), sym %SYM%, options(nostack, preserves_flags)) } } } ") { t_line = "H_%SYM%".replace("%SYM%", &s); a.push("H_%SYM%".replace("%SYM%", &s)); } else { a.push("crate::vm::LineAt(&(*node_head_line(n)))".to_string()); }
    a.push("&u.cc".to_string());
    a.push("&u.tf".to_string());
    a.push(format!("{}u32", { 1 + u.tf.0.get() }));
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_opt_neq_struct>({});\n", "n"));
    body.push_str(&format!("        let v_a = <{} as Disp>::call(c, {}, fp)?;\n", t_a, child_ptr_expr(n, u.a)));
    body.push_str(&format!("        let v_b = <{} as Disp>::call(c, {}, fp)?;\n", t_b, child_ptr_expr(n, u.b)));
    body.push_str(&format!("        EVAL_node_opt_neq{}({})\n    }}\n", format!("::<{}>", t_line), a.join(", ")));
    body.push_str(&format!("    #[inline(always)]\n    fn test(c: &mut Ctx, n: NodePtr, fp: Fp) -> Result<bool, Esc> {{\n"));
    body.push_str(&format!("        let u = node_u::<node_opt_neq_struct>({});\n", "n"));
    body.push_str(&format!("        let v_a = <{} as Disp>::call(c, {}, fp)?;\n", t_a, child_ptr_expr(n, u.a)));
    body.push_str(&format!("        let v_b = <{} as Disp>::call(c, {}, fp)?;\n", t_b, child_ptr_expr(n, u.b)));
    body.push_str(&format!("        EVAL_TEST_node_opt_neq{}({})\n    }}\n", format!("::<{}>", t_line), a.join(", ")));

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
fn SPECIALIZE_node_opt_ltlt(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_opt_ltlt_struct>(n);
    let t_a = specialize_node(sc, u.a);
    let t_b = specialize_node(sc, u.b);
    let mut t_line = "crate::vm::LineAt".to_string();
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push("v_a".to_string());
    a.push("v_b".to_string());
    if sc.spec_depth == 0 { sc.holes.push((n as usize + std::mem::offset_of!(NodeOf<node_opt_ltlt_struct>, head.line), std::mem::size_of::<u32>())); } if let Some(s) = sc.hole_sym("line", n as usize + std::mem::offset_of!(NodeOf<node_opt_ltlt_struct>, head.line), "impl Clone for H_%SYM% { #[inline(always)] fn clone(&self) -> Self { *self } } impl Copy for H_%SYM% {} impl crate::vm::LineHole for H_%SYM% {   #[inline(always)] fn pos(self) -> crate::vm::Pos { unsafe { crate::vm::Pos::bound(&raw const %SYM%) } }   #[inline(always)] fn store(self, f: &mut crate::vm::Frame) { unsafe {     std::arch::asm!(\"mov qword ptr [{0} + {1}], offset {2}\", in(reg) f, const std::mem::offset_of!(crate::vm::Frame, pos), sym %SYM%, options(nostack, preserves_flags)) } } } ") { t_line = "H_%SYM%".replace("%SYM%", &s); a.push("H_%SYM%".replace("%SYM%", &s)); } else { a.push("crate::vm::LineAt(&(*node_head_line(n)))".to_string()); }
    a.push("&u.cc".to_string());
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_opt_ltlt_struct>({});\n", "n"));
    body.push_str(&format!("        let v_a = <{} as Disp>::call(c, {}, fp)?;\n", t_a, child_ptr_expr(n, u.a)));
    body.push_str(&format!("        let v_b = <{} as Disp>::call(c, {}, fp)?;\n", t_b, child_ptr_expr(n, u.b)));
    body.push_str(&format!("        EVAL_node_opt_ltlt{}({})\n    }}\n", format!("::<{}>", t_line), a.join(", ")));


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
fn SPECIALIZE_node_opt_gtgt(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_opt_gtgt_struct>(n);
    let t_a = specialize_node(sc, u.a);
    let t_b = specialize_node(sc, u.b);
    let mut t_line = "crate::vm::LineAt".to_string();
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push("v_a".to_string());
    a.push("v_b".to_string());
    if sc.spec_depth == 0 { sc.holes.push((n as usize + std::mem::offset_of!(NodeOf<node_opt_gtgt_struct>, head.line), std::mem::size_of::<u32>())); } if let Some(s) = sc.hole_sym("line", n as usize + std::mem::offset_of!(NodeOf<node_opt_gtgt_struct>, head.line), "impl Clone for H_%SYM% { #[inline(always)] fn clone(&self) -> Self { *self } } impl Copy for H_%SYM% {} impl crate::vm::LineHole for H_%SYM% {   #[inline(always)] fn pos(self) -> crate::vm::Pos { unsafe { crate::vm::Pos::bound(&raw const %SYM%) } }   #[inline(always)] fn store(self, f: &mut crate::vm::Frame) { unsafe {     std::arch::asm!(\"mov qword ptr [{0} + {1}], offset {2}\", in(reg) f, const std::mem::offset_of!(crate::vm::Frame, pos), sym %SYM%, options(nostack, preserves_flags)) } } } ") { t_line = "H_%SYM%".replace("%SYM%", &s); a.push("H_%SYM%".replace("%SYM%", &s)); } else { a.push("crate::vm::LineAt(&(*node_head_line(n)))".to_string()); }
    a.push("&u.cc".to_string());
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_opt_gtgt_struct>({});\n", "n"));
    body.push_str(&format!("        let v_a = <{} as Disp>::call(c, {}, fp)?;\n", t_a, child_ptr_expr(n, u.a)));
    body.push_str(&format!("        let v_b = <{} as Disp>::call(c, {}, fp)?;\n", t_b, child_ptr_expr(n, u.b)));
    body.push_str(&format!("        EVAL_node_opt_gtgt{}({})\n    }}\n", format!("::<{}>", t_line), a.join(", ")));


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
fn SPECIALIZE_node_opt_xor(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_opt_xor_struct>(n);
    let t_a = specialize_node(sc, u.a);
    let t_b = specialize_node(sc, u.b);
    let mut t_line = "crate::vm::LineAt".to_string();
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push("v_a".to_string());
    a.push("v_b".to_string());
    if sc.spec_depth == 0 { sc.holes.push((n as usize + std::mem::offset_of!(NodeOf<node_opt_xor_struct>, head.line), std::mem::size_of::<u32>())); } if let Some(s) = sc.hole_sym("line", n as usize + std::mem::offset_of!(NodeOf<node_opt_xor_struct>, head.line), "impl Clone for H_%SYM% { #[inline(always)] fn clone(&self) -> Self { *self } } impl Copy for H_%SYM% {} impl crate::vm::LineHole for H_%SYM% {   #[inline(always)] fn pos(self) -> crate::vm::Pos { unsafe { crate::vm::Pos::bound(&raw const %SYM%) } }   #[inline(always)] fn store(self, f: &mut crate::vm::Frame) { unsafe {     std::arch::asm!(\"mov qword ptr [{0} + {1}], offset {2}\", in(reg) f, const std::mem::offset_of!(crate::vm::Frame, pos), sym %SYM%, options(nostack, preserves_flags)) } } } ") { t_line = "H_%SYM%".replace("%SYM%", &s); a.push("H_%SYM%".replace("%SYM%", &s)); } else { a.push("crate::vm::LineAt(&(*node_head_line(n)))".to_string()); }
    a.push("&u.cc".to_string());
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_opt_xor_struct>({});\n", "n"));
    body.push_str(&format!("        let v_a = <{} as Disp>::call(c, {}, fp)?;\n", t_a, child_ptr_expr(n, u.a)));
    body.push_str(&format!("        let v_b = <{} as Disp>::call(c, {}, fp)?;\n", t_b, child_ptr_expr(n, u.b)));
    body.push_str(&format!("        EVAL_node_opt_xor{}({})\n    }}\n", format!("::<{}>", t_line), a.join(", ")));


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
fn SPECIALIZE_node_opt_and(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_opt_and_struct>(n);
    let t_a = specialize_node(sc, u.a);
    let t_b = specialize_node(sc, u.b);
    let mut t_line = "crate::vm::LineAt".to_string();
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push("v_a".to_string());
    a.push("v_b".to_string());
    if sc.spec_depth == 0 { sc.holes.push((n as usize + std::mem::offset_of!(NodeOf<node_opt_and_struct>, head.line), std::mem::size_of::<u32>())); } if let Some(s) = sc.hole_sym("line", n as usize + std::mem::offset_of!(NodeOf<node_opt_and_struct>, head.line), "impl Clone for H_%SYM% { #[inline(always)] fn clone(&self) -> Self { *self } } impl Copy for H_%SYM% {} impl crate::vm::LineHole for H_%SYM% {   #[inline(always)] fn pos(self) -> crate::vm::Pos { unsafe { crate::vm::Pos::bound(&raw const %SYM%) } }   #[inline(always)] fn store(self, f: &mut crate::vm::Frame) { unsafe {     std::arch::asm!(\"mov qword ptr [{0} + {1}], offset {2}\", in(reg) f, const std::mem::offset_of!(crate::vm::Frame, pos), sym %SYM%, options(nostack, preserves_flags)) } } } ") { t_line = "H_%SYM%".replace("%SYM%", &s); a.push("H_%SYM%".replace("%SYM%", &s)); } else { a.push("crate::vm::LineAt(&(*node_head_line(n)))".to_string()); }
    a.push("&u.cc".to_string());
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_opt_and_struct>({});\n", "n"));
    body.push_str(&format!("        let v_a = <{} as Disp>::call(c, {}, fp)?;\n", t_a, child_ptr_expr(n, u.a)));
    body.push_str(&format!("        let v_b = <{} as Disp>::call(c, {}, fp)?;\n", t_b, child_ptr_expr(n, u.b)));
    body.push_str(&format!("        EVAL_node_opt_and{}({})\n    }}\n", format!("::<{}>", t_line), a.join(", ")));


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
fn SPECIALIZE_node_opt_or(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_opt_or_struct>(n);
    let t_a = specialize_node(sc, u.a);
    let t_b = specialize_node(sc, u.b);
    let mut t_line = "crate::vm::LineAt".to_string();
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push("v_a".to_string());
    a.push("v_b".to_string());
    if sc.spec_depth == 0 { sc.holes.push((n as usize + std::mem::offset_of!(NodeOf<node_opt_or_struct>, head.line), std::mem::size_of::<u32>())); } if let Some(s) = sc.hole_sym("line", n as usize + std::mem::offset_of!(NodeOf<node_opt_or_struct>, head.line), "impl Clone for H_%SYM% { #[inline(always)] fn clone(&self) -> Self { *self } } impl Copy for H_%SYM% {} impl crate::vm::LineHole for H_%SYM% {   #[inline(always)] fn pos(self) -> crate::vm::Pos { unsafe { crate::vm::Pos::bound(&raw const %SYM%) } }   #[inline(always)] fn store(self, f: &mut crate::vm::Frame) { unsafe {     std::arch::asm!(\"mov qword ptr [{0} + {1}], offset {2}\", in(reg) f, const std::mem::offset_of!(crate::vm::Frame, pos), sym %SYM%, options(nostack, preserves_flags)) } } } ") { t_line = "H_%SYM%".replace("%SYM%", &s); a.push("H_%SYM%".replace("%SYM%", &s)); } else { a.push("crate::vm::LineAt(&(*node_head_line(n)))".to_string()); }
    a.push("&u.cc".to_string());
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_opt_or_struct>({});\n", "n"));
    body.push_str(&format!("        let v_a = <{} as Disp>::call(c, {}, fp)?;\n", t_a, child_ptr_expr(n, u.a)));
    body.push_str(&format!("        let v_b = <{} as Disp>::call(c, {}, fp)?;\n", t_b, child_ptr_expr(n, u.b)));
    body.push_str(&format!("        EVAL_node_opt_or{}({})\n    }}\n", format!("::<{}>", t_line), a.join(", ")));


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
fn SPECIALIZE_node_opt_aref(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_opt_aref_struct>(n);
    let t_a = specialize_node(sc, u.a);
    let t_b = specialize_node(sc, u.b);
    let mut t_line = "crate::vm::LineAt".to_string();
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push("v_a".to_string());
    a.push("v_b".to_string());
    if sc.spec_depth == 0 { sc.holes.push((n as usize + std::mem::offset_of!(NodeOf<node_opt_aref_struct>, head.line), std::mem::size_of::<u32>())); } if let Some(s) = sc.hole_sym("line", n as usize + std::mem::offset_of!(NodeOf<node_opt_aref_struct>, head.line), "impl Clone for H_%SYM% { #[inline(always)] fn clone(&self) -> Self { *self } } impl Copy for H_%SYM% {} impl crate::vm::LineHole for H_%SYM% {   #[inline(always)] fn pos(self) -> crate::vm::Pos { unsafe { crate::vm::Pos::bound(&raw const %SYM%) } }   #[inline(always)] fn store(self, f: &mut crate::vm::Frame) { unsafe {     std::arch::asm!(\"mov qword ptr [{0} + {1}], offset {2}\", in(reg) f, const std::mem::offset_of!(crate::vm::Frame, pos), sym %SYM%, options(nostack, preserves_flags)) } } } ") { t_line = "H_%SYM%".replace("%SYM%", &s); a.push("H_%SYM%".replace("%SYM%", &s)); } else { a.push("crate::vm::LineAt(&(*node_head_line(n)))".to_string()); }
    a.push("&u.cc".to_string());
    a.push("&u.tf".to_string());
    a.push(format!("{}u32", { 1 + u.tf.0.get() }));
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_opt_aref_struct>({});\n", "n"));
    body.push_str(&format!("        let v_a = <{} as Disp>::call(c, {}, fp)?;\n", t_a, child_ptr_expr(n, u.a)));
    body.push_str(&format!("        let v_b = <{} as Disp>::call(c, {}, fp)?;\n", t_b, child_ptr_expr(n, u.b)));
    body.push_str(&format!("        EVAL_node_opt_aref{}({})\n    }}\n", format!("::<{}>", t_line), a.join(", ")));


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
fn SPECIALIZE_node_opt_aset(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_opt_aset_struct>(n);
    let t_a = specialize_node(sc, u.a);
    let t_b = specialize_node(sc, u.b);
    let t_v = specialize_node(sc, u.v);
    let mut t_line = "crate::vm::LineAt".to_string();
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push("v_a".to_string());
    a.push("v_b".to_string());
    a.push("v_v".to_string());
    if sc.spec_depth == 0 { sc.holes.push((n as usize + std::mem::offset_of!(NodeOf<node_opt_aset_struct>, head.line), std::mem::size_of::<u32>())); } if let Some(s) = sc.hole_sym("line", n as usize + std::mem::offset_of!(NodeOf<node_opt_aset_struct>, head.line), "impl Clone for H_%SYM% { #[inline(always)] fn clone(&self) -> Self { *self } } impl Copy for H_%SYM% {} impl crate::vm::LineHole for H_%SYM% {   #[inline(always)] fn pos(self) -> crate::vm::Pos { unsafe { crate::vm::Pos::bound(&raw const %SYM%) } }   #[inline(always)] fn store(self, f: &mut crate::vm::Frame) { unsafe {     std::arch::asm!(\"mov qword ptr [{0} + {1}], offset {2}\", in(reg) f, const std::mem::offset_of!(crate::vm::Frame, pos), sym %SYM%, options(nostack, preserves_flags)) } } } ") { t_line = "H_%SYM%".replace("%SYM%", &s); a.push("H_%SYM%".replace("%SYM%", &s)); } else { a.push("crate::vm::LineAt(&(*node_head_line(n)))".to_string()); }
    a.push("&u.cc".to_string());
    a.push("&u.tf".to_string());
    a.push(format!("{}u32", { 1 + u.tf.0.get() }));
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_opt_aset_struct>({});\n", "n"));
    body.push_str(&format!("        let v_a = <{} as Disp>::call(c, {}, fp)?;\n", t_a, child_ptr_expr(n, u.a)));
    body.push_str(&format!("        let v_b = <{} as Disp>::call(c, {}, fp)?;\n", t_b, child_ptr_expr(n, u.b)));
    body.push_str(&format!("        let v_v = <{} as Disp>::call(c, {}, fp)?;\n", t_v, child_ptr_expr(n, u.v)));
    body.push_str(&format!("        EVAL_node_opt_aset{}({})\n    }}\n", format!("::<{}>", t_line), a.join(", ")));


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
fn SPECIALIZE_node_method_entry(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_method_entry_struct>(n);
    let mut t_line = "crate::vm::LineAt".to_string();
    let t_body = specialize_node(sc, u.body);
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push(format!("{}u32", u.nlocals));
    a.push(format!("{}u32", u.nparams));
    a.push(format!("{}u32", u.flags));
    if sc.spec_depth == 0 { sc.holes.push((n as usize + std::mem::offset_of!(NodeOf<node_method_entry_struct>, head.line), std::mem::size_of::<u32>())); } if let Some(s) = sc.hole_sym("line", n as usize + std::mem::offset_of!(NodeOf<node_method_entry_struct>, head.line), "impl Clone for H_%SYM% { #[inline(always)] fn clone(&self) -> Self { *self } } impl Copy for H_%SYM% {} impl crate::vm::LineHole for H_%SYM% {   #[inline(always)] fn pos(self) -> crate::vm::Pos { unsafe { crate::vm::Pos::bound(&raw const %SYM%) } }   #[inline(always)] fn store(self, f: &mut crate::vm::Frame) { unsafe {     std::arch::asm!(\"mov qword ptr [{0} + {1}], offset {2}\", in(reg) f, const std::mem::offset_of!(crate::vm::Frame, pos), sym %SYM%, options(nostack, preserves_flags)) } } } ") { t_line = "H_%SYM%".replace("%SYM%", &s); a.push("H_%SYM%".replace("%SYM%", &s)); } else { a.push("crate::vm::LineAt(&(*node_head_line(n)))".to_string()); }
    a.push(child_ptr_expr(n, u.body));
    a.push("crate::vm::CallArgs::load(c)".to_string());
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_method_entry_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_method_entry{}({})\n    }}\n", format!("::<{}, {}>", t_line, t_body), a.join(", ")));

    let a_spec: Vec<String> = a.iter().enumerate().map(|(i, x)| if i == 8 { "ca".to_string() } else { x.clone() }).collect();
    body.push_str(&format!("    #[inline(always)]\n    fn spec(c: &mut Ctx, n: NodePtr, fp: Fp, ca: CallArgs) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_method_entry_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_method_entry{}({})\n    }}\n", format!("::<{}, {}>", t_line, t_body), a_spec.join(", ")));
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
fn SPECIALIZE_node_block_entry(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_block_entry_struct>(n);
    let mut t_line = "crate::vm::LineAt".to_string();
    let t_body = specialize_node(sc, u.body);
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push(format!("{}u32", u.nlocals));
    a.push(format!("{}u32", u.nparams));
    a.push(format!("{}u32", u.flags));
    if sc.spec_depth == 0 { sc.holes.push((n as usize + std::mem::offset_of!(NodeOf<node_block_entry_struct>, head.line), std::mem::size_of::<u32>())); } if let Some(s) = sc.hole_sym("line", n as usize + std::mem::offset_of!(NodeOf<node_block_entry_struct>, head.line), "impl Clone for H_%SYM% { #[inline(always)] fn clone(&self) -> Self { *self } } impl Copy for H_%SYM% {} impl crate::vm::LineHole for H_%SYM% {   #[inline(always)] fn pos(self) -> crate::vm::Pos { unsafe { crate::vm::Pos::bound(&raw const %SYM%) } }   #[inline(always)] fn store(self, f: &mut crate::vm::Frame) { unsafe {     std::arch::asm!(\"mov qword ptr [{0} + {1}], offset {2}\", in(reg) f, const std::mem::offset_of!(crate::vm::Frame, pos), sym %SYM%, options(nostack, preserves_flags)) } } } ") { t_line = "H_%SYM%".replace("%SYM%", &s); a.push("H_%SYM%".replace("%SYM%", &s)); } else { a.push("crate::vm::LineAt(&(*node_head_line(n)))".to_string()); }
    a.push(child_ptr_expr(n, u.body));
    a.push("crate::vm::CallArgs::load(c)".to_string());
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_block_entry_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_block_entry{}({})\n    }}\n", format!("::<{}, {}>", t_line, t_body), a.join(", ")));

    let a_spec: Vec<String> = a.iter().enumerate().map(|(i, x)| if i == 8 { "ca".to_string() } else { x.clone() }).collect();
    body.push_str(&format!("    #[inline(always)]\n    fn spec(c: &mut Ctx, n: NodePtr, fp: Fp, ca: CallArgs) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_block_entry_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_block_entry{}({})\n    }}\n", format!("::<{}, {}>", t_line, t_body), a_spec.join(", ")));
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
fn SPECIALIZE_node_def(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_def_struct>(n);

    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push("u.mid".to_string());
    a.push("u.body".to_string());
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_def_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_def{}({})\n    }}\n", "", a.join(", ")));


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
fn SPECIALIZE_node_sdef(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_sdef_struct>(n);
    let t_recv = specialize_node(sc, u.recv);
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push("v_recv".to_string());
    a.push("u.mid".to_string());
    a.push("u.body".to_string());
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_sdef_struct>({});\n", "n"));
    body.push_str(&format!("        let v_recv = <{} as Disp>::call(c, {}, fp)?;\n", t_recv, child_ptr_expr(n, u.recv)));
    body.push_str(&format!("        EVAL_node_sdef{}({})\n    }}\n", "", a.join(", ")));


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
fn SPECIALIZE_node_class(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_class_struct>(n);
    let t_cpath = specialize_node(sc, u.cpath);
    let t_superclass = specialize_node(sc, u.superclass);
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push(child_ptr_expr(n, u.cpath));
    a.push("u.name".to_string());
    a.push(child_ptr_expr(n, u.superclass));
    a.push("u.body".to_string());
    a.push(format!("{}", u.has_cpath));
    a.push(format!("{}", u.has_super));
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_class_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_class{}({})\n    }}\n", format!("::<{}, {}>", t_cpath, t_superclass), a.join(", ")));


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
fn SPECIALIZE_node_module(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_module_struct>(n);
    let t_cpath = specialize_node(sc, u.cpath);
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push(child_ptr_expr(n, u.cpath));
    a.push("u.name".to_string());
    a.push("u.body".to_string());
    a.push(format!("{}", u.has_cpath));
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_module_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_module{}({})\n    }}\n", format!("::<{}>", t_cpath), a.join(", ")));


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
fn SPECIALIZE_node_sclass(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_sclass_struct>(n);
    let t_recv = specialize_node(sc, u.recv);
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push("v_recv".to_string());
    a.push("u.body".to_string());
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_sclass_struct>({});\n", "n"));
    body.push_str(&format!("        let v_recv = <{} as Disp>::call(c, {}, fp)?;\n", t_recv, child_ptr_expr(n, u.recv)));
    body.push_str(&format!("        EVAL_node_sclass{}({})\n    }}\n", "", a.join(", ")));


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
fn SPECIALIZE_node_alias(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_alias_struct>(n);
    let t_newname = specialize_node(sc, u.newname);
    let t_oldname = specialize_node(sc, u.oldname);
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push("v_newname".to_string());
    a.push("v_oldname".to_string());
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_alias_struct>({});\n", "n"));
    body.push_str(&format!("        let v_newname = <{} as Disp>::call(c, {}, fp)?;\n", t_newname, child_ptr_expr(n, u.newname)));
    body.push_str(&format!("        let v_oldname = <{} as Disp>::call(c, {}, fp)?;\n", t_oldname, child_ptr_expr(n, u.oldname)));
    body.push_str(&format!("        EVAL_node_alias{}({})\n    }}\n", "", a.join(", ")));


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
fn SPECIALIZE_node_valias(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_valias_struct>(n);

    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push("u.newname".to_string());
    a.push("u.oldname".to_string());
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_valias_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_valias{}({})\n    }}\n", "", a.join(", ")));


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
fn SPECIALIZE_node_undef(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_undef_struct>(n);
    let t_names = specialize_list(sc, &u.names, n);
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push("list_base(n)".to_string());
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_undef_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_undef{}({})\n    }}\n", format!("::<{}>", t_names), a.join(", ")));


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
fn SPECIALIZE_node_masgn(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_masgn_struct>(n);
    let t_rhs = specialize_node(sc, u.rhs);
    let t_targets = specialize_list(sc, &u.targets, n);
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push("v_rhs".to_string());
    a.push(format!("{}u32", u.tmp));
    a.push("list_base(n)".to_string());
    a.push(format!("{}", u.splat_rhs));
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_masgn_struct>({});\n", "n"));
    body.push_str(&format!("        let v_rhs = <{} as Disp>::call(c, {}, fp)?;\n", t_rhs, child_ptr_expr(n, u.rhs)));
    body.push_str(&format!("        EVAL_node_masgn{}({})\n    }}\n", format!("::<{}>", t_targets), a.join(", ")));


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
fn SPECIALIZE_node_masgn_elem(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_masgn_elem_struct>(n);

    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push(format!("{}u32", u.tmp));
    a.push(format!("{}u32", u.idx));
    a.push(format!("{}u32", u.kind));
    a.push(format!("{}u32", u.post));
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_masgn_elem_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_masgn_elem{}({})\n    }}\n", "", a.join(", ")));


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
fn SPECIALIZE_node_unsupported(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_unsupported_struct>(n);

    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push("u.what".to_string());
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_unsupported_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_unsupported{}({})\n    }}\n", "", a.join(", ")));


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
fn SPECIALIZE_node_special(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_special_struct>(n);
    let t_arg = specialize_node(sc, u.arg);
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    a.push(format!("{}u32", u.kind));
    a.push(child_ptr_expr(n, u.arg));
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{\n"));
    body.push_str(&format!("        let u = node_u::<node_special_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_special{}({})\n    }}\n", format!("::<{}>", t_arg), a.join(", ")));


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
pub fn specialize_list(sc: &mut SpecCtx, ns: &NodeList, parent: NodePtr) -> String {
    let ts: Vec<String> = ns.iter().map(|x| specialize_node(sc, *x)).collect();
    // named by its code, as a T_
    let mut s = format!("impl DispList for @SELF@ {{\n    #[inline(always)] fn len(_ns: NodeList) -> usize {{ {} }}\n", ts.len());
    s.push_str(&format!("    #[inline(always)]\n    fn nth(c: &mut Ctx, ns: NodeList, i: usize, fp: Fp) -> RV {{\n        match i {{\n"));
    for (i, t) in ts.iter().enumerate() {
        let x = child_ptr_expr(parent, ns[i]).replacen("n.", "(ns.as_ptr() as NodePtr).", 1);
        s.push_str(&format!("            {} => <{} as Disp>::call(c, {}, fp),\n", i, t, x));
    }
    // i < len (EVAL_NTH!): a safe default arm adds a range check where i is not a constant (unsafe_catalog.md)
    s.push_str("            _ => unsafe { std::hint::unreachable_unchecked() },\n        }\n    }\n");
    // EVAL_EACH!: the calls in a row
    s.push_str("    #[inline(always)]\n    fn each<F: FnMut(&mut Ctx, usize, Value) -> Result<(), Esc>>(c: &mut Ctx, ns: NodeList, fp: Fp, mut f: F) -> Result<(), Esc> {\n");
    for (i, t) in ts.iter().enumerate() {
        let x = child_ptr_expr(parent, ns[i]).replacen("n.", "(ns.as_ptr() as NodePtr).", 1);
        let call = format!("<{} as Disp>::call(c, {}, fp)", t, x);
        let step = format!("{{ let r = {}; f(c, {}, r) }}", "@CALL@?".replace("@CALL@", &call), i);
        s.push_str(&format!("        {}\n", "@STEP@?;".replace("@STEP@", &step)));
    }
    s.push_str("        Ok(())\n    }\n}\n");
    let name = format!("L_{:016x}", hash_str(&s));
    if sc.emitted.insert(name.clone()) { sc.out.push_str(&format!("pub struct {};\n{}", name, s.replace("@SELF@", &name))); }
    name
}

// This file is auto-generated from node.def by astrustgen. Do not edit.
#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_num(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_num_struct>(n);

    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string()];
    a.push(format!("{}i64", u.v));
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr) -> i64 {{\n"));
    body.push_str(&format!("        let u = node_u::<node_num_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_num{}({})\n    }}\n", "", a.join(", ")));


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
fn SPECIALIZE_node_add(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_add_struct>(n);
    let t_l = specialize_node(sc, u.l);
    let t_r = specialize_node(sc, u.r);
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string()];
    a.push("u.l".to_string());
    a.push("u.r".to_string());
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr) -> i64 {{\n"));
    body.push_str(&format!("        let u = node_u::<node_add_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_add{}({})\n    }}\n", format!("::<{}, {}>", t_l, t_r), a.join(", ")));


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
fn SPECIALIZE_node_sub(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_sub_struct>(n);
    let t_l = specialize_node(sc, u.l);
    let t_r = specialize_node(sc, u.r);
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string()];
    a.push("u.l".to_string());
    a.push("u.r".to_string());
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr) -> i64 {{\n"));
    body.push_str(&format!("        let u = node_u::<node_sub_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_sub{}({})\n    }}\n", format!("::<{}, {}>", t_l, t_r), a.join(", ")));


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
fn SPECIALIZE_node_mul(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_mul_struct>(n);
    let t_l = specialize_node(sc, u.l);
    let t_r = specialize_node(sc, u.r);
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string()];
    a.push("u.l".to_string());
    a.push("u.r".to_string());
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr) -> i64 {{\n"));
    body.push_str(&format!("        let u = node_u::<node_mul_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_mul{}({})\n    }}\n", format!("::<{}, {}>", t_l, t_r), a.join(", ")));


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
fn SPECIALIZE_node_mod(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_mod_struct>(n);
    let t_l = specialize_node(sc, u.l);
    let t_r = specialize_node(sc, u.r);
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string()];
    a.push("v_l".to_string());
    a.push("v_r".to_string());
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr) -> i64 {{\n"));
    body.push_str(&format!("        let u = node_u::<node_mod_struct>({});\n", "n"));
    body.push_str(&format!("        let v_l = <{} as Disp>::call(c, u.l);\n", t_l));
    body.push_str(&format!("        let v_r = <{} as Disp>::call(c, u.r);\n", t_r));
    body.push_str(&format!("        EVAL_node_mod{}({})\n    }}\n", "", a.join(", ")));


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
fn SPECIALIZE_node_var(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_var_struct>(n);

    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string()];
    a.push(format!("{}u32", u.idx));
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr) -> i64 {{\n"));
    body.push_str(&format!("        let u = node_u::<node_var_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_var{}({})\n    }}\n", "", a.join(", ")));


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
fn SPECIALIZE_node_set(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_set_struct>(n);
    let t_v = specialize_node(sc, u.v);
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string()];
    a.push(format!("{}u32", u.idx));
    a.push("v_v".to_string());
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr) -> i64 {{\n"));
    body.push_str(&format!("        let u = node_u::<node_set_struct>({});\n", "n"));
    body.push_str(&format!("        let v_v = <{} as Disp>::call(c, u.v);\n", t_v));
    body.push_str(&format!("        EVAL_node_set{}({})\n    }}\n", "", a.join(", ")));


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
fn SPECIALIZE_node_seq(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_seq_struct>(n);
    let t_xs = specialize_list(sc, &u.xs);
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string()];
    a.push("u.xs".to_string());
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr) -> i64 {{\n"));
    body.push_str(&format!("        let u = node_u::<node_seq_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_seq{}({})\n    }}\n", format!("::<{}>", t_xs), a.join(", ")));


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
fn SPECIALIZE_node_times(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = node_u::<node_times_struct>(n);
    let t_cnt = specialize_node(sc, u.cnt);
    let t_body = specialize_node(sc, u.body);
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string()];
    a.push("u.cnt".to_string());
    a.push("u.body".to_string());
    if !was_open { sc.open.remove(&own_hash); }
    // named by its code (children's names, baked operands, bound holes are in it): equal code is
    // one item, whichever entry or position it was reached from
    let mut body = String::new();
    body.push_str(&format!("impl Disp for @SELF@ {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr) -> i64 {{\n"));
    body.push_str(&format!("        let u = node_u::<node_times_struct>({});\n", "n"));

    body.push_str(&format!("        EVAL_node_times{}({})\n    }}\n", format!("::<{}, {}>", t_cnt, t_body), a.join(", ")));


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
    s.push_str(&format!("    #[inline(always)]\n    fn nth(c: &mut Ctx, ns: NodeList, i: usize) -> i64 {{\n        match i {{\n"));
    for (i, t) in ts.iter().enumerate() {
        let x = format!("unsafe {{ *ns.get_unchecked({}) }}", i);
        s.push_str(&format!("            {} => <{} as Disp>::call(c, {}),\n", i, t, x));
    }
    // i < len (EVAL_NTH!): a safe default arm adds a range check where i is not a constant (unsafe_catalog.md)
    s.push_str("            _ => unsafe { std::hint::unreachable_unchecked() },\n        }\n    }\n");
    // EVAL_EACH!: the calls in a row
    s.push_str("    #[inline(always)]\n    fn each<F: FnMut(&mut Ctx, usize, i64) -> ()>(c: &mut Ctx, ns: NodeList, mut f: F) -> () {\n");
    for (i, t) in ts.iter().enumerate() {
        let x = format!("unsafe {{ *ns.get_unchecked({}) }}", i);
        let call = format!("<{} as Disp>::call(c, {})", t, x);
        let step = format!("{{ let r = {}; f(c, {}, r) }}", "@CALL@".replace("@CALL@", &call), i);
        s.push_str(&format!("        {}\n", "@STEP@;".replace("@STEP@", &step)));
    }
    s.push_str("        ()\n    }\n}\n");
    let name = format!("L_{:016x}", hash_str(&s));
    if sc.emitted.insert(name.clone()) { sc.out.push_str(&format!("pub struct {};\n{}", name, s.replace("@SELF@", &name))); }
    name
}

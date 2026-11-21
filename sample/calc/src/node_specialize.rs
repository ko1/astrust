// This file is auto-generated from node.def by astrustgen. Do not edit.
#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_num(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_num_struct>)).u };

    let np = n as *mut NodeOf<node_num_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string()];
    unsafe {
    a.push(format!("{}i64", (*np).u.v));
    }
    if !was_open { sc.open.remove(&own_hash); }
    // named by what is baked too (children's types, @bake values), not by the hash alone:
    // two sites with one hash may have seen different things
    let name = format!("T_{:016x}", hash_merge(own_hash, hash_str(&format!("{}|{}", <[&str]>::join(&[], ","), a.join(",")))));
    let name = sc.positional(name, n as usize);
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr) -> i64 {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_num_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_num{}({})\n    }} }}\n", "", a.join(", ")));


    body.push_str("}
");
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_add(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_add_struct>)).u };
    let t_l = specialize_node(sc, u.l);
    let t_r = specialize_node(sc, u.r);
    let np = n as *mut NodeOf<node_add_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string()];
    unsafe {
    a.push("(*np).u.l".to_string());
    a.push("(*np).u.r".to_string());
    }
    if !was_open { sc.open.remove(&own_hash); }
    // named by what is baked too (children's types, @bake values), not by the hash alone:
    // two sites with one hash may have seen different things
    let name = format!("T_{:016x}", hash_merge(own_hash, hash_str(&format!("{}|{}", <[&str]>::join(&[t_l.as_str(), t_r.as_str()], ","), a.join(",")))));
    let name = sc.positional(name, n as usize);
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr) -> i64 {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_add_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_add{}({})\n    }} }}\n", format!("::<{}, {}>", t_l, t_r), a.join(", ")));


    body.push_str("}
");
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_sub(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_sub_struct>)).u };
    let t_l = specialize_node(sc, u.l);
    let t_r = specialize_node(sc, u.r);
    let np = n as *mut NodeOf<node_sub_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string()];
    unsafe {
    a.push("(*np).u.l".to_string());
    a.push("(*np).u.r".to_string());
    }
    if !was_open { sc.open.remove(&own_hash); }
    // named by what is baked too (children's types, @bake values), not by the hash alone:
    // two sites with one hash may have seen different things
    let name = format!("T_{:016x}", hash_merge(own_hash, hash_str(&format!("{}|{}", <[&str]>::join(&[t_l.as_str(), t_r.as_str()], ","), a.join(",")))));
    let name = sc.positional(name, n as usize);
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr) -> i64 {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_sub_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_sub{}({})\n    }} }}\n", format!("::<{}, {}>", t_l, t_r), a.join(", ")));


    body.push_str("}
");
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_mul(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_mul_struct>)).u };
    let t_l = specialize_node(sc, u.l);
    let t_r = specialize_node(sc, u.r);
    let np = n as *mut NodeOf<node_mul_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string()];
    unsafe {
    a.push("(*np).u.l".to_string());
    a.push("(*np).u.r".to_string());
    }
    if !was_open { sc.open.remove(&own_hash); }
    // named by what is baked too (children's types, @bake values), not by the hash alone:
    // two sites with one hash may have seen different things
    let name = format!("T_{:016x}", hash_merge(own_hash, hash_str(&format!("{}|{}", <[&str]>::join(&[t_l.as_str(), t_r.as_str()], ","), a.join(",")))));
    let name = sc.positional(name, n as usize);
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr) -> i64 {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_mul_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_mul{}({})\n    }} }}\n", format!("::<{}, {}>", t_l, t_r), a.join(", ")));


    body.push_str("}
");
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_mod(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_mod_struct>)).u };
    let t_l = specialize_node(sc, u.l);
    let t_r = specialize_node(sc, u.r);
    let np = n as *mut NodeOf<node_mod_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string()];
    unsafe {
    a.push("v_l".to_string());
    a.push("v_r".to_string());
    }
    if !was_open { sc.open.remove(&own_hash); }
    // named by what is baked too (children's types, @bake values), not by the hash alone:
    // two sites with one hash may have seen different things
    let name = format!("T_{:016x}", hash_merge(own_hash, hash_str(&format!("{}|{}", <[&str]>::join(&[t_l.as_str(), t_r.as_str()], ","), a.join(",")))));
    let name = sc.positional(name, n as usize);
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr) -> i64 {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_mod_struct>;\n", "n"));
    body.push_str(&format!("        let v_l = <{} as Disp>::call(c, (*np).u.l);\n", t_l));
    body.push_str(&format!("        let v_r = <{} as Disp>::call(c, (*np).u.r);\n", t_r));
    body.push_str(&format!("        EVAL_node_mod{}({})\n    }} }}\n", "", a.join(", ")));


    body.push_str("}
");
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_var(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_var_struct>)).u };

    let np = n as *mut NodeOf<node_var_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string()];
    unsafe {
    a.push(format!("{}u32", (*np).u.idx));
    }
    if !was_open { sc.open.remove(&own_hash); }
    // named by what is baked too (children's types, @bake values), not by the hash alone:
    // two sites with one hash may have seen different things
    let name = format!("T_{:016x}", hash_merge(own_hash, hash_str(&format!("{}|{}", <[&str]>::join(&[], ","), a.join(",")))));
    let name = sc.positional(name, n as usize);
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr) -> i64 {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_var_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_var{}({})\n    }} }}\n", "", a.join(", ")));


    body.push_str("}
");
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_set(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_set_struct>)).u };
    let t_v = specialize_node(sc, u.v);
    let np = n as *mut NodeOf<node_set_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string()];
    unsafe {
    a.push(format!("{}u32", (*np).u.idx));
    a.push("v_v".to_string());
    }
    if !was_open { sc.open.remove(&own_hash); }
    // named by what is baked too (children's types, @bake values), not by the hash alone:
    // two sites with one hash may have seen different things
    let name = format!("T_{:016x}", hash_merge(own_hash, hash_str(&format!("{}|{}", <[&str]>::join(&[t_v.as_str()], ","), a.join(",")))));
    let name = sc.positional(name, n as usize);
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr) -> i64 {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_set_struct>;\n", "n"));
    body.push_str(&format!("        let v_v = <{} as Disp>::call(c, (*np).u.v);\n", t_v));
    body.push_str(&format!("        EVAL_node_set{}({})\n    }} }}\n", "", a.join(", ")));


    body.push_str("}
");
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_seq(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_seq_struct>)).u };
    let t_xs = specialize_list(sc, &u.xs);
    let np = n as *mut NodeOf<node_seq_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string()];
    unsafe {
    a.push("(*np).u.xs".to_string());
    }
    if !was_open { sc.open.remove(&own_hash); }
    // named by what is baked too (children's types, @bake values), not by the hash alone:
    // two sites with one hash may have seen different things
    let name = format!("T_{:016x}", hash_merge(own_hash, hash_str(&format!("{}|{}", <[&str]>::join(&[t_xs.as_str()], ","), a.join(",")))));
    let name = sc.positional(name, n as usize);
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr) -> i64 {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_seq_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_seq{}({})\n    }} }}\n", format!("::<{}>", t_xs), a.join(", ")));


    body.push_str("}
");
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_times(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_times_struct>)).u };
    let t_cnt = specialize_node(sc, u.cnt);
    let t_body = specialize_node(sc, u.body);
    let np = n as *mut NodeOf<node_times_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string()];
    unsafe {
    a.push("(*np).u.cnt".to_string());
    a.push("(*np).u.body".to_string());
    }
    if !was_open { sc.open.remove(&own_hash); }
    // named by what is baked too (children's types, @bake values), not by the hash alone:
    // two sites with one hash may have seen different things
    let name = format!("T_{:016x}", hash_merge(own_hash, hash_str(&format!("{}|{}", <[&str]>::join(&[t_cnt.as_str(), t_body.as_str()], ","), a.join(",")))));
    let name = sc.positional(name, n as usize);
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr) -> i64 {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_times_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_times{}({})\n    }} }}\n", format!("::<{}, {}>", t_cnt, t_body), a.join(", ")));


    body.push_str("}
");
    sc.out.push_str(&body);
    name
}

/// SD type for a NODES operand: a `DispList` whose nth() dispatches statically.
pub fn specialize_list(sc: &mut SpecCtx, ns: &NodeList) -> String {
    let ts: Vec<String> = ns.iter().map(|x| specialize_node(sc, *x)).collect();
    let mut h = hash_str("list");
    for x in ns.iter() { h = hash_merge(h, hash_node(*x)); }
    
    // patch crate: the elements' names too (one in a bound entry's copy may call its R_<hash>P)
    if sc.patch { h = hash_merge(h, hash_str(&ts.join(","))); }
    let name = sc.positional(format!("L_{:016x}", h), 0);
    if sc.emitted.contains(&name) { return name; }
    let mut s = format!("pub struct {};\nimpl DispList for {} {{\n    #[inline(always)] fn len(_ns: NodeList) -> usize {{ {} }}\n", name, name, ts.len());
    s.push_str(&format!("    #[inline(always)]\n    fn nth(c: &mut Ctx, ns: NodeList, i: usize) -> i64 {{\n        match i {{\n"));
    for (i, t) in ts.iter().enumerate() {
        let x = format!("*ns.get_unchecked({})", i);
        s.push_str(&format!("            {} => unsafe {{ <{} as Disp>::call(c, {}) }},\n", i, t, x));
    }
    s.push_str("            _ => unsafe { std::hint::unreachable_unchecked() },\n        }\n    }\n}\n");
    sc.emitted.insert(name.clone());
    sc.out.push_str(&s);
    name
}

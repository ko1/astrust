// This file is auto-generated from node.def by astrustgen. Do not edit.
#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_nil(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_nil_struct>)).u };

    let np = n as *mut NodeOf<node_nil_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {

    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_nil_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_nil{}({})\n    }} }}\n", "", a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_true(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_true_struct>)).u };

    let np = n as *mut NodeOf<node_true_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {

    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_true_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_true{}({})\n    }} }}\n", "", a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_false(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_false_struct>)).u };

    let np = n as *mut NodeOf<node_false_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {

    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_false_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_false{}({})\n    }} }}\n", "", a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_self(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_self_struct>)).u };

    let np = n as *mut NodeOf<node_self_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {

    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_self_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_self{}({})\n    }} }}\n", "", a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_lit(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_lit_struct>)).u };

    let np = n as *mut NodeOf<node_lit_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push(crate::compile::value_bake((*np).u.v).unwrap_or_else(|| "(*np).u.v".to_string()));
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_lit_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_lit{}({})\n    }} }}\n", "", a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_float(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_float_struct>)).u };

    let np = n as *mut NodeOf<node_float_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push(format!("f64::from_bits({}u64)", (*np).u.v.to_bits()));
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_float_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_float{}({})\n    }} }}\n", "", a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_str(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_str_struct>)).u };

    let np = n as *mut NodeOf<node_str_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push("(*np).u.s".to_string());
    a.push(format!("{}u32", (*np).u.enc));
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_str_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_str{}({})\n    }} }}\n", "", a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_fstr(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_fstr_struct>)).u };

    let np = n as *mut NodeOf<node_fstr_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push("(*np).u.s".to_string());
    a.push(format!("{}u32", (*np).u.enc));
    a.push("&mut (*np).u.lc".to_string());
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_fstr_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_fstr{}({})\n    }} }}\n", "", a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_dstr(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_dstr_struct>)).u };
    let t_parts = specialize_list(sc, &u.parts, n);
    let np = n as *mut NodeOf<node_dstr_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push("list_base(n)".to_string());
    a.push(format!("{}u32", (*np).u.enc));
    a.push(format!("{}u32", (*np).u.cap));
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_dstr_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_dstr{}({})\n    }} }}\n", format!("::<{}>", t_parts), a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_tostr(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_tostr_struct>)).u };
    let t_v = specialize_node(sc, u.v);
    let np = n as *mut NodeOf<node_tostr_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push("v_v".to_string());
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_tostr_struct>;\n", "n"));
    body.push_str(&format!("        let v_v = <{} as Disp>::call(c, {}, fp)?;\n", t_v, child_ptr_expr(n, u.v)));
    body.push_str(&format!("        EVAL_node_tostr{}({})\n    }} }}\n", "", a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_dsym(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_dsym_struct>)).u };
    let t_s = specialize_node(sc, u.s);
    let np = n as *mut NodeOf<node_dsym_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push("v_s".to_string());
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_dsym_struct>;\n", "n"));
    body.push_str(&format!("        let v_s = <{} as Disp>::call(c, {}, fp)?;\n", t_s, child_ptr_expr(n, u.s)));
    body.push_str(&format!("        EVAL_node_dsym{}({})\n    }} }}\n", "", a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_regex(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_regex_struct>)).u };

    let np = n as *mut NodeOf<node_regex_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push("(*np).u.src".to_string());
    a.push(format!("{}u32", (*np).u.opts));
    a.push("&mut (*np).u.lc".to_string());
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_regex_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_regex{}({})\n    }} }}\n", "", a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_dregex(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_dregex_struct>)).u };
    let t_s = specialize_node(sc, u.s);
    let np = n as *mut NodeOf<node_dregex_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push("v_s".to_string());
    a.push(format!("{}u32", (*np).u.opts));
    a.push(format!("{}", (*np).u.once));
    a.push("&mut (*np).u.lc".to_string());
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_dregex_struct>;\n", "n"));
    body.push_str(&format!("        let v_s = <{} as Disp>::call(c, {}, fp)?;\n", t_s, child_ptr_expr(n, u.s)));
    body.push_str(&format!("        EVAL_node_dregex{}({})\n    }} }}\n", "", a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_biglit(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_biglit_struct>)).u };

    let np = n as *mut NodeOf<node_biglit_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push("(*np).u.digits".to_string());
    a.push("&mut (*np).u.lc".to_string());
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_biglit_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_biglit{}({})\n    }} }}\n", "", a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_ary(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_ary_struct>)).u };
    let t_elems = specialize_list(sc, &u.elems, n);
    let np = n as *mut NodeOf<node_ary_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push("list_base(n)".to_string());
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_ary_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_ary{}({})\n    }} }}\n", format!("::<{}>", t_elems), a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_ary_build(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_ary_build_struct>)).u };
    let t_elems = specialize_list(sc, &u.elems, n);
    let np = n as *mut NodeOf<node_ary_build_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push("list_base(n)".to_string());
    a.push("(*np).u.kinds".to_string());
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_ary_build_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_ary_build{}({})\n    }} }}\n", format!("::<{}>", t_elems), a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_hash(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_hash_struct>)).u };
    let t_items = specialize_list(sc, &u.items, n);
    let np = n as *mut NodeOf<node_hash_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push("list_base(n)".to_string());
    a.push("(*np).u.kinds".to_string());
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_hash_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_hash{}({})\n    }} }}\n", format!("::<{}>", t_items), a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_range(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_range_struct>)).u };
    let t_b = specialize_node(sc, u.b);
    let t_e = specialize_node(sc, u.e);
    let np = n as *mut NodeOf<node_range_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push("v_b".to_string());
    a.push("v_e".to_string());
    a.push(format!("{}", (*np).u.excl));
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_range_struct>;\n", "n"));
    body.push_str(&format!("        let v_b = <{} as Disp>::call(c, {}, fp)?;\n", t_b, child_ptr_expr(n, u.b)));
    body.push_str(&format!("        let v_e = <{} as Disp>::call(c, {}, fp)?;\n", t_e, child_ptr_expr(n, u.e)));
    body.push_str(&format!("        EVAL_node_range{}({})\n    }} }}\n", "", a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_range_lit(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_range_lit_struct>)).u };

    let np = n as *mut NodeOf<node_range_lit_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push(crate::compile::value_bake((*np).u.b).unwrap_or_else(|| "(*np).u.b".to_string()));
    a.push(crate::compile::value_bake((*np).u.e).unwrap_or_else(|| "(*np).u.e".to_string()));
    a.push(format!("{}", (*np).u.excl));
    a.push("&mut (*np).u.lc".to_string());
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_range_lit_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_range_lit{}({})\n    }} }}\n", "", a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_lvar(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_lvar_struct>)).u };

    let np = n as *mut NodeOf<node_lvar_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push(format!("{}u32", (*np).u.idx));
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_lvar_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_lvar{}({})\n    }} }}\n", "", a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_lasgn(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_lasgn_struct>)).u };
    let t_v = specialize_node(sc, u.v);
    let np = n as *mut NodeOf<node_lasgn_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push(format!("{}u32", (*np).u.idx));
    a.push("v_v".to_string());
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_lasgn_struct>;\n", "n"));
    body.push_str(&format!("        let v_v = <{} as Disp>::call(c, {}, fp)?;\n", t_v, child_ptr_expr(n, u.v)));
    body.push_str(&format!("        EVAL_node_lasgn{}({})\n    }} }}\n", "", a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_dvar(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_dvar_struct>)).u };

    let np = n as *mut NodeOf<node_dvar_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push(format!("{}u32", (*np).u.hops));
    a.push(format!("{}u32", (*np).u.idx));
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_dvar_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_dvar{}({})\n    }} }}\n", "", a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_dasgn(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_dasgn_struct>)).u };
    let t_v = specialize_node(sc, u.v);
    let np = n as *mut NodeOf<node_dasgn_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push(format!("{}u32", (*np).u.hops));
    a.push(format!("{}u32", (*np).u.idx));
    a.push("v_v".to_string());
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_dasgn_struct>;\n", "n"));
    body.push_str(&format!("        let v_v = <{} as Disp>::call(c, {}, fp)?;\n", t_v, child_ptr_expr(n, u.v)));
    body.push_str(&format!("        EVAL_node_dasgn{}({})\n    }} }}\n", "", a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_ivar(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_ivar_struct>)).u };

    let np = n as *mut NodeOf<node_ivar_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push("(*np).u.name".to_string());
    a.push("&mut (*np).u.ic".to_string());
    a.push(format!("{}u32", { let u = &(*np).u; if u.ic.klass != 0 { u.ic.idx + 1 } else { 0 } }));
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_ivar_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_ivar{}({})\n    }} }}\n", "", a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_iasgn(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_iasgn_struct>)).u };
    let t_v = specialize_node(sc, u.v);
    let np = n as *mut NodeOf<node_iasgn_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push("(*np).u.name".to_string());
    a.push("v_v".to_string());
    a.push("&mut (*np).u.ic".to_string());
    a.push(format!("{}u32", { let u = &(*np).u; if u.ic.klass != 0 { u.ic.idx + 1 } else { 0 } }));
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_iasgn_struct>;\n", "n"));
    body.push_str(&format!("        let v_v = <{} as Disp>::call(c, {}, fp)?;\n", t_v, child_ptr_expr(n, u.v)));
    body.push_str(&format!("        EVAL_node_iasgn{}({})\n    }} }}\n", "", a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_ivar_arith(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_ivar_arith_struct>)).u };
    let t_v = specialize_node(sc, u.v);
    let np = n as *mut NodeOf<node_ivar_arith_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push("(*np).u.name".to_string());
    a.push(format!("{}", (*np).u.sub));
    a.push("v_v".to_string());
    a.push(format!("{}u32", (*np).u.line));
    a.push("&mut (*np).u.ic".to_string());
    a.push("&mut (*np).u.cc".to_string());
    a.push(format!("{}u32", { let u = &(*np).u; if u.ic.klass != 0 { u.ic.idx + 1 } else { 0 } }));
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_ivar_arith_struct>;\n", "n"));
    body.push_str(&format!("        let v_v = <{} as Disp>::call(c, {}, fp)?;\n", t_v, child_ptr_expr(n, u.v)));
    body.push_str(&format!("        EVAL_node_ivar_arith{}({})\n    }} }}\n", "", a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_gvar(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_gvar_struct>)).u };

    let np = n as *mut NodeOf<node_gvar_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push("(*np).u.name".to_string());
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_gvar_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_gvar{}({})\n    }} }}\n", "", a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_gasgn(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_gasgn_struct>)).u };
    let t_v = specialize_node(sc, u.v);
    let np = n as *mut NodeOf<node_gasgn_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push("(*np).u.name".to_string());
    a.push("v_v".to_string());
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_gasgn_struct>;\n", "n"));
    body.push_str(&format!("        let v_v = <{} as Disp>::call(c, {}, fp)?;\n", t_v, child_ptr_expr(n, u.v)));
    body.push_str(&format!("        EVAL_node_gasgn{}({})\n    }} }}\n", "", a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_cvar(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_cvar_struct>)).u };

    let np = n as *mut NodeOf<node_cvar_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push("(*np).u.name".to_string());
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_cvar_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_cvar{}({})\n    }} }}\n", "", a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_cvasgn(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_cvasgn_struct>)).u };
    let t_v = specialize_node(sc, u.v);
    let np = n as *mut NodeOf<node_cvasgn_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push("(*np).u.name".to_string());
    a.push("v_v".to_string());
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_cvasgn_struct>;\n", "n"));
    body.push_str(&format!("        let v_v = <{} as Disp>::call(c, {}, fp)?;\n", t_v, child_ptr_expr(n, u.v)));
    body.push_str(&format!("        EVAL_node_cvasgn{}({})\n    }} }}\n", "", a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_const(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_const_struct>)).u };

    let np = n as *mut NodeOf<node_const_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push("(*np).u.name".to_string());
    a.push(format!("{}u32", (*np).u.line));
    a.push("&mut (*np).u.cc".to_string());
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_const_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_const{}({})\n    }} }}\n", "", a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_const_path(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_const_path_struct>)).u };
    let t_parent = specialize_node(sc, u.parent);
    let np = n as *mut NodeOf<node_const_path_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push("v_parent".to_string());
    a.push("(*np).u.name".to_string());
    a.push(format!("{}u32", (*np).u.line));
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_const_path_struct>;\n", "n"));
    body.push_str(&format!("        let v_parent = <{} as Disp>::call(c, {}, fp)?;\n", t_parent, child_ptr_expr(n, u.parent)));
    body.push_str(&format!("        EVAL_node_const_path{}({})\n    }} }}\n", "", a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_top_const(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_top_const_struct>)).u };

    let np = n as *mut NodeOf<node_top_const_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push("(*np).u.name".to_string());
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_top_const_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_top_const{}({})\n    }} }}\n", "", a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_cdecl(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_cdecl_struct>)).u };
    let t_v = specialize_node(sc, u.v);
    let np = n as *mut NodeOf<node_cdecl_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push("(*np).u.name".to_string());
    a.push("v_v".to_string());
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_cdecl_struct>;\n", "n"));
    body.push_str(&format!("        let v_v = <{} as Disp>::call(c, {}, fp)?;\n", t_v, child_ptr_expr(n, u.v)));
    body.push_str(&format!("        EVAL_node_cdecl{}({})\n    }} }}\n", "", a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_cdecl_path(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_cdecl_path_struct>)).u };
    let t_parent = specialize_node(sc, u.parent);
    let t_v = specialize_node(sc, u.v);
    let np = n as *mut NodeOf<node_cdecl_path_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push("v_parent".to_string());
    a.push("(*np).u.name".to_string());
    a.push("v_v".to_string());
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_cdecl_path_struct>;\n", "n"));
    body.push_str(&format!("        let v_parent = <{} as Disp>::call(c, {}, fp)?;\n", t_parent, child_ptr_expr(n, u.parent)));
    body.push_str(&format!("        let v_v = <{} as Disp>::call(c, {}, fp)?;\n", t_v, child_ptr_expr(n, u.v)));
    body.push_str(&format!("        EVAL_node_cdecl_path{}({})\n    }} }}\n", "", a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_errinfo(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_errinfo_struct>)).u };

    let np = n as *mut NodeOf<node_errinfo_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {

    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_errinfo_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_errinfo{}({})\n    }} }}\n", "", a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_nth_ref(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_nth_ref_struct>)).u };

    let np = n as *mut NodeOf<node_nth_ref_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push(format!("{}u32", (*np).u.nth));
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_nth_ref_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_nth_ref{}({})\n    }} }}\n", "", a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_back_ref(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_back_ref_struct>)).u };

    let np = n as *mut NodeOf<node_back_ref_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push(format!("{}u32", (*np).u.kind));
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_back_ref_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_back_ref{}({})\n    }} }}\n", "", a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_block(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_block_struct>)).u };
    let t_stmts = specialize_list(sc, &u.stmts, n);
    let np = n as *mut NodeOf<node_block_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push("list_base(n)".to_string());
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_block_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_block{}({})\n    }} }}\n", format!("::<{}>", t_stmts), a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_line(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_line_struct>)).u };
    let t_body = specialize_node(sc, u.body);
    let np = n as *mut NodeOf<node_line_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push(format!("{}u32", (*np).u.line));
    a.push(child_ptr_expr(n, u.body));
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_line_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_line{}({})\n    }} }}\n", format!("::<{}>", t_body), a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_if(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_if_struct>)).u };
    let t_cond = specialize_node(sc, u.cond);
    let t_then_b = specialize_node(sc, u.then_b);
    let t_else_b = specialize_node(sc, u.else_b);
    let np = n as *mut NodeOf<node_if_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push(child_ptr_expr(n, u.cond));
    a.push(child_ptr_expr(n, u.then_b));
    a.push(child_ptr_expr(n, u.else_b));
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_if_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_if{}({})\n    }} }}\n", format!("::<{}, {}, {}>", t_cond, t_then_b, t_else_b), a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_and(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_and_struct>)).u };
    let t_a = specialize_node(sc, u.a);
    let t_b = specialize_node(sc, u.b);
    let np = n as *mut NodeOf<node_and_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push(child_ptr_expr(n, u.a));
    a.push(child_ptr_expr(n, u.b));
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_and_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_and{}({})\n    }} }}\n", format!("::<{}, {}>", t_a, t_b), a.join(", ")));
    body.push_str(&format!("    #[inline(always)]\n    fn test(c: &mut Ctx, n: NodePtr, fp: Fp) -> Result<bool, Esc> {{ unsafe {{\n"));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_and_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_TEST_node_and{}({})\n    }} }}\n", format!("::<{}, {}>", t_a, t_b), a.join(", ")));

    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_or(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_or_struct>)).u };
    let t_a = specialize_node(sc, u.a);
    let t_b = specialize_node(sc, u.b);
    let np = n as *mut NodeOf<node_or_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push(child_ptr_expr(n, u.a));
    a.push(child_ptr_expr(n, u.b));
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_or_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_or{}({})\n    }} }}\n", format!("::<{}, {}>", t_a, t_b), a.join(", ")));
    body.push_str(&format!("    #[inline(always)]\n    fn test(c: &mut Ctx, n: NodePtr, fp: Fp) -> Result<bool, Esc> {{ unsafe {{\n"));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_or_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_TEST_node_or{}({})\n    }} }}\n", format!("::<{}, {}>", t_a, t_b), a.join(", ")));

    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_not(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_not_struct>)).u };
    let t_v = specialize_node(sc, u.v);
    let np = n as *mut NodeOf<node_not_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push("v_v".to_string());
    a.push("&mut (*np).u.cc".to_string());
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_not_struct>;\n", "n"));
    body.push_str(&format!("        let v_v = <{} as Disp>::call(c, {}, fp)?;\n", t_v, child_ptr_expr(n, u.v)));
    body.push_str(&format!("        EVAL_node_not{}({})\n    }} }}\n", "", a.join(", ")));
    body.push_str(&format!("    #[inline(always)]\n    fn test(c: &mut Ctx, n: NodePtr, fp: Fp) -> Result<bool, Esc> {{ unsafe {{\n"));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_not_struct>;\n", "n"));
    body.push_str(&format!("        let v_v = <{} as Disp>::call(c, {}, fp)?;\n", t_v, child_ptr_expr(n, u.v)));
    body.push_str(&format!("        EVAL_TEST_node_not{}({})\n    }} }}\n", "", a.join(", ")));

    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_while(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_while_struct>)).u };
    let t_cond = specialize_node(sc, u.cond);
    let t_body = specialize_node(sc, u.body);
    let np = n as *mut NodeOf<node_while_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push(child_ptr_expr(n, u.cond));
    a.push(child_ptr_expr(n, u.body));
    a.push(format!("{}", (*np).u.until));
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_while_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_while{}({})\n    }} }}\n", format!("::<{}, {}>", t_cond, t_body), a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_dowhile(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_dowhile_struct>)).u };
    let t_cond = specialize_node(sc, u.cond);
    let t_body = specialize_node(sc, u.body);
    let np = n as *mut NodeOf<node_dowhile_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push(child_ptr_expr(n, u.cond));
    a.push(child_ptr_expr(n, u.body));
    a.push(format!("{}", (*np).u.until));
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_dowhile_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_dowhile{}({})\n    }} }}\n", format!("::<{}, {}>", t_cond, t_body), a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_loop_break(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_loop_break_struct>)).u };
    let t_v = specialize_node(sc, u.v);
    let np = n as *mut NodeOf<node_loop_break_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push("v_v".to_string());
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_loop_break_struct>;\n", "n"));
    body.push_str(&format!("        let v_v = <{} as Disp>::call(c, {}, fp)?;\n", t_v, child_ptr_expr(n, u.v)));
    body.push_str(&format!("        EVAL_node_loop_break{}({})\n    }} }}\n", "", a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_loop_next(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_loop_next_struct>)).u };
    let t_v = specialize_node(sc, u.v);
    let np = n as *mut NodeOf<node_loop_next_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push("v_v".to_string());
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_loop_next_struct>;\n", "n"));
    body.push_str(&format!("        let v_v = <{} as Disp>::call(c, {}, fp)?;\n", t_v, child_ptr_expr(n, u.v)));
    body.push_str(&format!("        EVAL_node_loop_next{}({})\n    }} }}\n", "", a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_break(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_break_struct>)).u };
    let t_v = specialize_node(sc, u.v);
    let np = n as *mut NodeOf<node_break_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push("v_v".to_string());
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_break_struct>;\n", "n"));
    body.push_str(&format!("        let v_v = <{} as Disp>::call(c, {}, fp)?;\n", t_v, child_ptr_expr(n, u.v)));
    body.push_str(&format!("        EVAL_node_break{}({})\n    }} }}\n", "", a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_next(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_next_struct>)).u };
    let t_v = specialize_node(sc, u.v);
    let np = n as *mut NodeOf<node_next_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push("v_v".to_string());
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_next_struct>;\n", "n"));
    body.push_str(&format!("        let v_v = <{} as Disp>::call(c, {}, fp)?;\n", t_v, child_ptr_expr(n, u.v)));
    body.push_str(&format!("        EVAL_node_next{}({})\n    }} }}\n", "", a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_return(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_return_struct>)).u };
    let t_v = specialize_node(sc, u.v);
    let np = n as *mut NodeOf<node_return_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push("v_v".to_string());
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_return_struct>;\n", "n"));
    body.push_str(&format!("        let v_v = <{} as Disp>::call(c, {}, fp)?;\n", t_v, child_ptr_expr(n, u.v)));
    body.push_str(&format!("        EVAL_node_return{}({})\n    }} }}\n", "", a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_retry(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_retry_struct>)).u };

    let np = n as *mut NodeOf<node_retry_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {

    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_retry_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_retry{}({})\n    }} }}\n", "", a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_redo(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_redo_struct>)).u };

    let np = n as *mut NodeOf<node_redo_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {

    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_redo_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_redo{}({})\n    }} }}\n", "", a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_rescue(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_rescue_struct>)).u };
    let t_body = specialize_node(sc, u.body);
    let t_clauses = specialize_list(sc, &u.clauses, n);
    let t_else_b = specialize_node(sc, u.else_b);
    let np = n as *mut NodeOf<node_rescue_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push(child_ptr_expr(n, u.body));
    a.push("list_base(n)".to_string());
    a.push(child_ptr_expr(n, u.else_b));
    a.push(format!("{}", (*np).u.has_else));
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_rescue_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_rescue{}({})\n    }} }}\n", format!("::<{}, {}, {}>", t_body, t_clauses, t_else_b), a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_resbody(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_resbody_struct>)).u };
    let t_classes = specialize_list(sc, &u.classes, n);
    let t_assign = specialize_node(sc, u.assign);
    let t_body = specialize_node(sc, u.body);
    let np = n as *mut NodeOf<node_resbody_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push("list_base(n)".to_string());
    a.push("(*np).u.kinds".to_string());
    a.push(child_ptr_expr(n, u.assign));
    a.push(child_ptr_expr(n, u.body));
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_resbody_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_resbody{}({})\n    }} }}\n", format!("::<{}, {}, {}>", t_classes, t_assign, t_body), a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_ensure(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_ensure_struct>)).u };
    let t_body = specialize_node(sc, u.body);
    let t_ens = specialize_node(sc, u.ens);
    let np = n as *mut NodeOf<node_ensure_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push(child_ptr_expr(n, u.body));
    a.push(child_ptr_expr(n, u.ens));
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_ensure_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_ensure{}({})\n    }} }}\n", format!("::<{}, {}>", t_body, t_ens), a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_when(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_when_struct>)).u };
    let t_pats = specialize_list(sc, &u.pats, n);
    let np = n as *mut NodeOf<node_when_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push(format!("{}u32", (*np).u.subj_slot));
    a.push("list_base(n)".to_string());
    a.push("(*np).u.kinds".to_string());
    a.push("&mut (*np).u.cc".to_string());
    a.push("&mut (*np).u.km".to_string());
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_when_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_when{}({})\n    }} }}\n", format!("::<{}>", t_pats), a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_when_lit(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_when_lit_struct>)).u };

    let np = n as *mut NodeOf<node_when_lit_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push(format!("{}u32", (*np).u.subj_slot));
    a.push(crate::compile::value_bake((*np).u.v).unwrap_or_else(|| "(*np).u.v".to_string()));
    a.push("&mut (*np).u.cc".to_string());
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_when_lit_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_when_lit{}({})\n    }} }}\n", "", a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_defined(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_defined_struct>)).u };
    let t_recv = specialize_node(sc, u.recv);
    let np = n as *mut NodeOf<node_defined_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push(format!("{}u32", (*np).u.kind));
    a.push("(*np).u.name".to_string());
    a.push(child_ptr_expr(n, u.recv));
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_defined_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_defined{}({})\n    }} }}\n", format!("::<{}>", t_recv), a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_call(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_call_struct>)).u };
    let t_recv = specialize_node(sc, u.recv);
    let t_args = specialize_list(sc, &u.args, n);
    let (t_callee, h_callee) = { let np = n as *mut NodeOf<node_call_struct>; let p: NodePtr = unsafe { { let u = &(*np).u; crate::vm::spec_callee(&u.cc, u.args.len(), u.flags) } }; if p.is_null() || sc.spec_depth >= sc.spec_max_depth || sc.open.contains(&hash_node(p)) { ("Dyn".to_string(), 0u64) } else { sc.spec_depth += 1; let t = specialize_node(sc, p); sc.spec_depth -= 1; (t, hash_node(p)) } };
    let np = n as *mut NodeOf<node_call_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push(child_ptr_expr(n, u.recv));
    a.push("list_base(n)".to_string());
    a.push("(*np).u.mid".to_string());
    a.push(format!("{}u32", (*np).u.flags));
    a.push(format!("{}u32", (*np).u.line));
    a.push("&mut (*np).u.cc".to_string());
    a.push(format!("{}u64", h_callee));
    a.push(format!("{}u32", { let u = &(*np).u; crate::vm::site_kind(&u.cc, u.args.len(), u.flags) }));
    a.push(format!("{}u32", { let u = &(*np).u; crate::vm::spec_shared(u.recv, u.args.len(), crate::vm::spec_callee(&u.cc, u.args.len(), u.flags)) }));
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_call_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_call{}({})\n    }} }}\n", format!("::<{}, {}, {}>", t_recv, t_args, t_callee), a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_call_blk(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_call_blk_struct>)).u };
    let t_recv = specialize_node(sc, u.recv);
    let t_args = specialize_list(sc, &u.args, n);
    let (t_callee, h_callee) = { let np = n as *mut NodeOf<node_call_blk_struct>; let p: NodePtr = unsafe { { let u = &(*np).u; crate::vm::spec_callee(&u.cc, u.args.len(), u.flags) } }; if p.is_null() || sc.spec_depth >= sc.spec_max_depth || sc.open.contains(&hash_node(p)) { ("Dyn".to_string(), 0u64) } else { sc.spec_depth += 1; let t = specialize_node(sc, p); sc.spec_depth -= 1; (t, hash_node(p)) } };
    let np = n as *mut NodeOf<node_call_blk_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push(child_ptr_expr(n, u.recv));
    a.push("list_base(n)".to_string());
    a.push("(*np).u.mid".to_string());
    a.push(format!("{}u32", (*np).u.flags));
    a.push(format!("{}u32", (*np).u.line));
    a.push("(*np).u.blk".to_string());
    a.push("&mut (*np).u.cc".to_string());
    a.push(format!("{}u64", h_callee));
    a.push(format!("{}u32", { let u = &(*np).u; crate::vm::site_kind(&u.cc, u.args.len(), u.flags) }));
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_call_blk_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_call_blk{}({})\n    }} }}\n", format!("::<{}, {}, {}>", t_recv, t_args, t_callee), a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_call_kw(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_call_kw_struct>)).u };
    let t_recv = specialize_node(sc, u.recv);
    let t_args = specialize_list(sc, &u.args, n);
    let (t_callee, h_callee) = { let np = n as *mut NodeOf<node_call_kw_struct>; let p: NodePtr = unsafe { { let u = &(*np).u; crate::vm::spec_callee_kw(&u.cc, u.args.len(), u.ci) } }; if p.is_null() || sc.spec_depth >= sc.spec_max_depth || sc.open.contains(&hash_node(p)) { ("Dyn".to_string(), 0u64) } else { sc.spec_depth += 1; let t = specialize_node(sc, p); sc.spec_depth -= 1; (t, hash_node(p)) } };
    let np = n as *mut NodeOf<node_call_kw_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push(child_ptr_expr(n, u.recv));
    a.push("list_base(n)".to_string());
    a.push("(*np).u.mid".to_string());
    a.push(format!("{}u32", (*np).u.flags));
    a.push(format!("{}u32", (*np).u.line));
    a.push("(*np).u.ci".to_string());
    a.push("&mut (*np).u.cc".to_string());
    a.push(format!("{}u64", h_callee));
    a.push(format!("{}u32", { let u = &(*np).u; crate::vm::site_kind_kw(&u.cc, u.args.len(), u.ci) }));
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_call_kw_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_call_kw{}({})\n    }} }}\n", format!("::<{}, {}, {}>", t_recv, t_args, t_callee), a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_call_x(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_call_x_struct>)).u };
    let t_recv = specialize_node(sc, u.recv);
    let t_args = specialize_list(sc, &u.args, n);
    let np = n as *mut NodeOf<node_call_x_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push(child_ptr_expr(n, u.recv));
    a.push("list_base(n)".to_string());
    a.push("(*np).u.mid".to_string());
    a.push(format!("{}u32", (*np).u.flags));
    a.push(format!("{}u32", (*np).u.line));
    a.push("(*np).u.ci".to_string());
    a.push("&mut (*np).u.cc".to_string());
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_call_x_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_call_x{}({})\n    }} }}\n", format!("::<{}, {}>", t_recv, t_args), a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_attrasgn(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_attrasgn_struct>)).u };
    let t_recv = specialize_node(sc, u.recv);
    let t_args = specialize_list(sc, &u.args, n);
    let np = n as *mut NodeOf<node_attrasgn_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push(child_ptr_expr(n, u.recv));
    a.push("list_base(n)".to_string());
    a.push("(*np).u.mid".to_string());
    a.push(format!("{}u32", (*np).u.flags));
    a.push(format!("{}u32", (*np).u.line));
    a.push("&mut (*np).u.cc".to_string());
    a.push(format!("{}u32", { let u = &(*np).u; crate::vm::site_kind(&u.cc, u.args.len(), u.flags) }));
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_attrasgn_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_attrasgn{}({})\n    }} }}\n", format!("::<{}, {}>", t_recv, t_args), a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_super(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_super_struct>)).u };
    let t_args = specialize_list(sc, &u.args, n);
    let np = n as *mut NodeOf<node_super_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push("list_base(n)".to_string());
    a.push(format!("{}u32", (*np).u.line));
    a.push("(*np).u.ci".to_string());
    a.push(format!("{}", (*np).u.zsuper));
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_super_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_super{}({})\n    }} }}\n", format!("::<{}>", t_args), a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_yield(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_yield_struct>)).u };
    let t_args = specialize_list(sc, &u.args, n);
    let (t_callee, h_callee) = { let np = n as *mut NodeOf<node_yield_struct>; let p: NodePtr = unsafe { { let u = &(*np).u; crate::vm::spec_block(&u.yc, u.args.len()) } }; if p.is_null() || sc.spec_depth >= sc.spec_max_depth || sc.open.contains(&hash_node(p)) { ("Dyn".to_string(), 0u64) } else { sc.spec_depth += 1; let t = specialize_node(sc, p); sc.spec_depth -= 1; (t, hash_node(p)) } };
    let np = n as *mut NodeOf<node_yield_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push("list_base(n)".to_string());
    a.push("&mut (*np).u.yc".to_string());
    a.push(format!("{}u64", h_callee));
    a.push(format!("{}u32", { let u = &(*np).u; crate::vm::spec_block_np(&u.yc, u.args.len()) }));
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_yield_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_yield{}({})\n    }} }}\n", format!("::<{}, {}>", t_args, t_callee), a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_yield_x(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_yield_x_struct>)).u };
    let t_args = specialize_list(sc, &u.args, n);
    let np = n as *mut NodeOf<node_yield_x_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push("list_base(n)".to_string());
    a.push("(*np).u.ci".to_string());
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_yield_x_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_yield_x{}({})\n    }} }}\n", format!("::<{}>", t_args), a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_block_given(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_block_given_struct>)).u };

    let np = n as *mut NodeOf<node_block_given_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {

    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_block_given_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_block_given{}({})\n    }} }}\n", "", a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_lambda(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_lambda_struct>)).u };

    let np = n as *mut NodeOf<node_lambda_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push("(*np).u.blk".to_string());
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_lambda_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_lambda{}({})\n    }} }}\n", "", a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_opt_plus(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_opt_plus_struct>)).u };
    let t_a = specialize_node(sc, u.a);
    let t_b = specialize_node(sc, u.b);
    let np = n as *mut NodeOf<node_opt_plus_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push("v_a".to_string());
    a.push("v_b".to_string());
    a.push(format!("{}u32", (*np).u.line));
    a.push("&mut (*np).u.cc".to_string());
    a.push("&mut (*np).u.tf".to_string());
    a.push(format!("{}u32", { 1 + (*np).u.tf.0 }));
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_opt_plus_struct>;\n", "n"));
    body.push_str(&format!("        let v_a = <{} as Disp>::call(c, {}, fp)?;\n", t_a, child_ptr_expr(n, u.a)));
    body.push_str(&format!("        let v_b = <{} as Disp>::call(c, {}, fp)?;\n", t_b, child_ptr_expr(n, u.b)));
    body.push_str(&format!("        EVAL_node_opt_plus{}({})\n    }} }}\n", "", a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_opt_minus(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_opt_minus_struct>)).u };
    let t_a = specialize_node(sc, u.a);
    let t_b = specialize_node(sc, u.b);
    let np = n as *mut NodeOf<node_opt_minus_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push("v_a".to_string());
    a.push("v_b".to_string());
    a.push(format!("{}u32", (*np).u.line));
    a.push("&mut (*np).u.cc".to_string());
    a.push("&mut (*np).u.tf".to_string());
    a.push(format!("{}u32", { 1 + (*np).u.tf.0 }));
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_opt_minus_struct>;\n", "n"));
    body.push_str(&format!("        let v_a = <{} as Disp>::call(c, {}, fp)?;\n", t_a, child_ptr_expr(n, u.a)));
    body.push_str(&format!("        let v_b = <{} as Disp>::call(c, {}, fp)?;\n", t_b, child_ptr_expr(n, u.b)));
    body.push_str(&format!("        EVAL_node_opt_minus{}({})\n    }} }}\n", "", a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_opt_mul(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_opt_mul_struct>)).u };
    let t_a = specialize_node(sc, u.a);
    let t_b = specialize_node(sc, u.b);
    let np = n as *mut NodeOf<node_opt_mul_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push("v_a".to_string());
    a.push("v_b".to_string());
    a.push(format!("{}u32", (*np).u.line));
    a.push("&mut (*np).u.cc".to_string());
    a.push("&mut (*np).u.tf".to_string());
    a.push(format!("{}u32", { 1 + (*np).u.tf.0 }));
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_opt_mul_struct>;\n", "n"));
    body.push_str(&format!("        let v_a = <{} as Disp>::call(c, {}, fp)?;\n", t_a, child_ptr_expr(n, u.a)));
    body.push_str(&format!("        let v_b = <{} as Disp>::call(c, {}, fp)?;\n", t_b, child_ptr_expr(n, u.b)));
    body.push_str(&format!("        EVAL_node_opt_mul{}({})\n    }} }}\n", "", a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_opt_div(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_opt_div_struct>)).u };
    let t_a = specialize_node(sc, u.a);
    let t_b = specialize_node(sc, u.b);
    let np = n as *mut NodeOf<node_opt_div_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push("v_a".to_string());
    a.push("v_b".to_string());
    a.push(format!("{}u32", (*np).u.line));
    a.push("&mut (*np).u.cc".to_string());
    a.push("&mut (*np).u.tf".to_string());
    a.push(format!("{}u32", { 1 + (*np).u.tf.0 }));
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_opt_div_struct>;\n", "n"));
    body.push_str(&format!("        let v_a = <{} as Disp>::call(c, {}, fp)?;\n", t_a, child_ptr_expr(n, u.a)));
    body.push_str(&format!("        let v_b = <{} as Disp>::call(c, {}, fp)?;\n", t_b, child_ptr_expr(n, u.b)));
    body.push_str(&format!("        EVAL_node_opt_div{}({})\n    }} }}\n", "", a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_opt_pow(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_opt_pow_struct>)).u };
    let t_a = specialize_node(sc, u.a);
    let t_b = specialize_node(sc, u.b);
    let np = n as *mut NodeOf<node_opt_pow_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push("v_a".to_string());
    a.push("v_b".to_string());
    a.push(format!("{}u32", (*np).u.line));
    a.push("&mut (*np).u.cc".to_string());
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_opt_pow_struct>;\n", "n"));
    body.push_str(&format!("        let v_a = <{} as Disp>::call(c, {}, fp)?;\n", t_a, child_ptr_expr(n, u.a)));
    body.push_str(&format!("        let v_b = <{} as Disp>::call(c, {}, fp)?;\n", t_b, child_ptr_expr(n, u.b)));
    body.push_str(&format!("        EVAL_node_opt_pow{}({})\n    }} }}\n", "", a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_opt_mod(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_opt_mod_struct>)).u };
    let t_a = specialize_node(sc, u.a);
    let t_b = specialize_node(sc, u.b);
    let np = n as *mut NodeOf<node_opt_mod_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push("v_a".to_string());
    a.push("v_b".to_string());
    a.push(format!("{}u32", (*np).u.line));
    a.push("&mut (*np).u.cc".to_string());
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_opt_mod_struct>;\n", "n"));
    body.push_str(&format!("        let v_a = <{} as Disp>::call(c, {}, fp)?;\n", t_a, child_ptr_expr(n, u.a)));
    body.push_str(&format!("        let v_b = <{} as Disp>::call(c, {}, fp)?;\n", t_b, child_ptr_expr(n, u.b)));
    body.push_str(&format!("        EVAL_node_opt_mod{}({})\n    }} }}\n", "", a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_opt_lt(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_opt_lt_struct>)).u };
    let t_a = specialize_node(sc, u.a);
    let t_b = specialize_node(sc, u.b);
    let np = n as *mut NodeOf<node_opt_lt_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push("v_a".to_string());
    a.push("v_b".to_string());
    a.push(format!("{}u32", (*np).u.line));
    a.push("&mut (*np).u.cc".to_string());
    a.push("&mut (*np).u.tf".to_string());
    a.push(format!("{}u32", { 1 + (*np).u.tf.0 }));
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_opt_lt_struct>;\n", "n"));
    body.push_str(&format!("        let v_a = <{} as Disp>::call(c, {}, fp)?;\n", t_a, child_ptr_expr(n, u.a)));
    body.push_str(&format!("        let v_b = <{} as Disp>::call(c, {}, fp)?;\n", t_b, child_ptr_expr(n, u.b)));
    body.push_str(&format!("        EVAL_node_opt_lt{}({})\n    }} }}\n", "", a.join(", ")));
    body.push_str(&format!("    #[inline(always)]\n    fn test(c: &mut Ctx, n: NodePtr, fp: Fp) -> Result<bool, Esc> {{ unsafe {{\n"));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_opt_lt_struct>;\n", "n"));
    body.push_str(&format!("        let v_a = <{} as Disp>::call(c, {}, fp)?;\n", t_a, child_ptr_expr(n, u.a)));
    body.push_str(&format!("        let v_b = <{} as Disp>::call(c, {}, fp)?;\n", t_b, child_ptr_expr(n, u.b)));
    body.push_str(&format!("        EVAL_TEST_node_opt_lt{}({})\n    }} }}\n", "", a.join(", ")));

    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_opt_le(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_opt_le_struct>)).u };
    let t_a = specialize_node(sc, u.a);
    let t_b = specialize_node(sc, u.b);
    let np = n as *mut NodeOf<node_opt_le_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push("v_a".to_string());
    a.push("v_b".to_string());
    a.push(format!("{}u32", (*np).u.line));
    a.push("&mut (*np).u.cc".to_string());
    a.push("&mut (*np).u.tf".to_string());
    a.push(format!("{}u32", { 1 + (*np).u.tf.0 }));
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_opt_le_struct>;\n", "n"));
    body.push_str(&format!("        let v_a = <{} as Disp>::call(c, {}, fp)?;\n", t_a, child_ptr_expr(n, u.a)));
    body.push_str(&format!("        let v_b = <{} as Disp>::call(c, {}, fp)?;\n", t_b, child_ptr_expr(n, u.b)));
    body.push_str(&format!("        EVAL_node_opt_le{}({})\n    }} }}\n", "", a.join(", ")));
    body.push_str(&format!("    #[inline(always)]\n    fn test(c: &mut Ctx, n: NodePtr, fp: Fp) -> Result<bool, Esc> {{ unsafe {{\n"));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_opt_le_struct>;\n", "n"));
    body.push_str(&format!("        let v_a = <{} as Disp>::call(c, {}, fp)?;\n", t_a, child_ptr_expr(n, u.a)));
    body.push_str(&format!("        let v_b = <{} as Disp>::call(c, {}, fp)?;\n", t_b, child_ptr_expr(n, u.b)));
    body.push_str(&format!("        EVAL_TEST_node_opt_le{}({})\n    }} }}\n", "", a.join(", ")));

    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_opt_gt(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_opt_gt_struct>)).u };
    let t_a = specialize_node(sc, u.a);
    let t_b = specialize_node(sc, u.b);
    let np = n as *mut NodeOf<node_opt_gt_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push("v_a".to_string());
    a.push("v_b".to_string());
    a.push(format!("{}u32", (*np).u.line));
    a.push("&mut (*np).u.cc".to_string());
    a.push("&mut (*np).u.tf".to_string());
    a.push(format!("{}u32", { 1 + (*np).u.tf.0 }));
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_opt_gt_struct>;\n", "n"));
    body.push_str(&format!("        let v_a = <{} as Disp>::call(c, {}, fp)?;\n", t_a, child_ptr_expr(n, u.a)));
    body.push_str(&format!("        let v_b = <{} as Disp>::call(c, {}, fp)?;\n", t_b, child_ptr_expr(n, u.b)));
    body.push_str(&format!("        EVAL_node_opt_gt{}({})\n    }} }}\n", "", a.join(", ")));
    body.push_str(&format!("    #[inline(always)]\n    fn test(c: &mut Ctx, n: NodePtr, fp: Fp) -> Result<bool, Esc> {{ unsafe {{\n"));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_opt_gt_struct>;\n", "n"));
    body.push_str(&format!("        let v_a = <{} as Disp>::call(c, {}, fp)?;\n", t_a, child_ptr_expr(n, u.a)));
    body.push_str(&format!("        let v_b = <{} as Disp>::call(c, {}, fp)?;\n", t_b, child_ptr_expr(n, u.b)));
    body.push_str(&format!("        EVAL_TEST_node_opt_gt{}({})\n    }} }}\n", "", a.join(", ")));

    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_opt_ge(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_opt_ge_struct>)).u };
    let t_a = specialize_node(sc, u.a);
    let t_b = specialize_node(sc, u.b);
    let np = n as *mut NodeOf<node_opt_ge_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push("v_a".to_string());
    a.push("v_b".to_string());
    a.push(format!("{}u32", (*np).u.line));
    a.push("&mut (*np).u.cc".to_string());
    a.push("&mut (*np).u.tf".to_string());
    a.push(format!("{}u32", { 1 + (*np).u.tf.0 }));
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_opt_ge_struct>;\n", "n"));
    body.push_str(&format!("        let v_a = <{} as Disp>::call(c, {}, fp)?;\n", t_a, child_ptr_expr(n, u.a)));
    body.push_str(&format!("        let v_b = <{} as Disp>::call(c, {}, fp)?;\n", t_b, child_ptr_expr(n, u.b)));
    body.push_str(&format!("        EVAL_node_opt_ge{}({})\n    }} }}\n", "", a.join(", ")));
    body.push_str(&format!("    #[inline(always)]\n    fn test(c: &mut Ctx, n: NodePtr, fp: Fp) -> Result<bool, Esc> {{ unsafe {{\n"));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_opt_ge_struct>;\n", "n"));
    body.push_str(&format!("        let v_a = <{} as Disp>::call(c, {}, fp)?;\n", t_a, child_ptr_expr(n, u.a)));
    body.push_str(&format!("        let v_b = <{} as Disp>::call(c, {}, fp)?;\n", t_b, child_ptr_expr(n, u.b)));
    body.push_str(&format!("        EVAL_TEST_node_opt_ge{}({})\n    }} }}\n", "", a.join(", ")));

    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_opt_eq(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_opt_eq_struct>)).u };
    let t_a = specialize_node(sc, u.a);
    let t_b = specialize_node(sc, u.b);
    let np = n as *mut NodeOf<node_opt_eq_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push("v_a".to_string());
    a.push("v_b".to_string());
    a.push(format!("{}u32", (*np).u.line));
    a.push("&mut (*np).u.cc".to_string());
    a.push("&mut (*np).u.tf".to_string());
    a.push(format!("{}u32", { 1 + (*np).u.tf.0 }));
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_opt_eq_struct>;\n", "n"));
    body.push_str(&format!("        let v_a = <{} as Disp>::call(c, {}, fp)?;\n", t_a, child_ptr_expr(n, u.a)));
    body.push_str(&format!("        let v_b = <{} as Disp>::call(c, {}, fp)?;\n", t_b, child_ptr_expr(n, u.b)));
    body.push_str(&format!("        EVAL_node_opt_eq{}({})\n    }} }}\n", "", a.join(", ")));
    body.push_str(&format!("    #[inline(always)]\n    fn test(c: &mut Ctx, n: NodePtr, fp: Fp) -> Result<bool, Esc> {{ unsafe {{\n"));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_opt_eq_struct>;\n", "n"));
    body.push_str(&format!("        let v_a = <{} as Disp>::call(c, {}, fp)?;\n", t_a, child_ptr_expr(n, u.a)));
    body.push_str(&format!("        let v_b = <{} as Disp>::call(c, {}, fp)?;\n", t_b, child_ptr_expr(n, u.b)));
    body.push_str(&format!("        EVAL_TEST_node_opt_eq{}({})\n    }} }}\n", "", a.join(", ")));

    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_opt_neq(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_opt_neq_struct>)).u };
    let t_a = specialize_node(sc, u.a);
    let t_b = specialize_node(sc, u.b);
    let np = n as *mut NodeOf<node_opt_neq_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push("v_a".to_string());
    a.push("v_b".to_string());
    a.push(format!("{}u32", (*np).u.line));
    a.push("&mut (*np).u.cc".to_string());
    a.push("&mut (*np).u.tf".to_string());
    a.push(format!("{}u32", { 1 + (*np).u.tf.0 }));
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_opt_neq_struct>;\n", "n"));
    body.push_str(&format!("        let v_a = <{} as Disp>::call(c, {}, fp)?;\n", t_a, child_ptr_expr(n, u.a)));
    body.push_str(&format!("        let v_b = <{} as Disp>::call(c, {}, fp)?;\n", t_b, child_ptr_expr(n, u.b)));
    body.push_str(&format!("        EVAL_node_opt_neq{}({})\n    }} }}\n", "", a.join(", ")));
    body.push_str(&format!("    #[inline(always)]\n    fn test(c: &mut Ctx, n: NodePtr, fp: Fp) -> Result<bool, Esc> {{ unsafe {{\n"));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_opt_neq_struct>;\n", "n"));
    body.push_str(&format!("        let v_a = <{} as Disp>::call(c, {}, fp)?;\n", t_a, child_ptr_expr(n, u.a)));
    body.push_str(&format!("        let v_b = <{} as Disp>::call(c, {}, fp)?;\n", t_b, child_ptr_expr(n, u.b)));
    body.push_str(&format!("        EVAL_TEST_node_opt_neq{}({})\n    }} }}\n", "", a.join(", ")));

    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_opt_ltlt(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_opt_ltlt_struct>)).u };
    let t_a = specialize_node(sc, u.a);
    let t_b = specialize_node(sc, u.b);
    let np = n as *mut NodeOf<node_opt_ltlt_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push("v_a".to_string());
    a.push("v_b".to_string());
    a.push(format!("{}u32", (*np).u.line));
    a.push("&mut (*np).u.cc".to_string());
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_opt_ltlt_struct>;\n", "n"));
    body.push_str(&format!("        let v_a = <{} as Disp>::call(c, {}, fp)?;\n", t_a, child_ptr_expr(n, u.a)));
    body.push_str(&format!("        let v_b = <{} as Disp>::call(c, {}, fp)?;\n", t_b, child_ptr_expr(n, u.b)));
    body.push_str(&format!("        EVAL_node_opt_ltlt{}({})\n    }} }}\n", "", a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_opt_gtgt(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_opt_gtgt_struct>)).u };
    let t_a = specialize_node(sc, u.a);
    let t_b = specialize_node(sc, u.b);
    let np = n as *mut NodeOf<node_opt_gtgt_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push("v_a".to_string());
    a.push("v_b".to_string());
    a.push(format!("{}u32", (*np).u.line));
    a.push("&mut (*np).u.cc".to_string());
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_opt_gtgt_struct>;\n", "n"));
    body.push_str(&format!("        let v_a = <{} as Disp>::call(c, {}, fp)?;\n", t_a, child_ptr_expr(n, u.a)));
    body.push_str(&format!("        let v_b = <{} as Disp>::call(c, {}, fp)?;\n", t_b, child_ptr_expr(n, u.b)));
    body.push_str(&format!("        EVAL_node_opt_gtgt{}({})\n    }} }}\n", "", a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_opt_xor(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_opt_xor_struct>)).u };
    let t_a = specialize_node(sc, u.a);
    let t_b = specialize_node(sc, u.b);
    let np = n as *mut NodeOf<node_opt_xor_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push("v_a".to_string());
    a.push("v_b".to_string());
    a.push(format!("{}u32", (*np).u.line));
    a.push("&mut (*np).u.cc".to_string());
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_opt_xor_struct>;\n", "n"));
    body.push_str(&format!("        let v_a = <{} as Disp>::call(c, {}, fp)?;\n", t_a, child_ptr_expr(n, u.a)));
    body.push_str(&format!("        let v_b = <{} as Disp>::call(c, {}, fp)?;\n", t_b, child_ptr_expr(n, u.b)));
    body.push_str(&format!("        EVAL_node_opt_xor{}({})\n    }} }}\n", "", a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_opt_and(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_opt_and_struct>)).u };
    let t_a = specialize_node(sc, u.a);
    let t_b = specialize_node(sc, u.b);
    let np = n as *mut NodeOf<node_opt_and_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push("v_a".to_string());
    a.push("v_b".to_string());
    a.push(format!("{}u32", (*np).u.line));
    a.push("&mut (*np).u.cc".to_string());
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_opt_and_struct>;\n", "n"));
    body.push_str(&format!("        let v_a = <{} as Disp>::call(c, {}, fp)?;\n", t_a, child_ptr_expr(n, u.a)));
    body.push_str(&format!("        let v_b = <{} as Disp>::call(c, {}, fp)?;\n", t_b, child_ptr_expr(n, u.b)));
    body.push_str(&format!("        EVAL_node_opt_and{}({})\n    }} }}\n", "", a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_opt_or(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_opt_or_struct>)).u };
    let t_a = specialize_node(sc, u.a);
    let t_b = specialize_node(sc, u.b);
    let np = n as *mut NodeOf<node_opt_or_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push("v_a".to_string());
    a.push("v_b".to_string());
    a.push(format!("{}u32", (*np).u.line));
    a.push("&mut (*np).u.cc".to_string());
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_opt_or_struct>;\n", "n"));
    body.push_str(&format!("        let v_a = <{} as Disp>::call(c, {}, fp)?;\n", t_a, child_ptr_expr(n, u.a)));
    body.push_str(&format!("        let v_b = <{} as Disp>::call(c, {}, fp)?;\n", t_b, child_ptr_expr(n, u.b)));
    body.push_str(&format!("        EVAL_node_opt_or{}({})\n    }} }}\n", "", a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_opt_aref(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_opt_aref_struct>)).u };
    let t_a = specialize_node(sc, u.a);
    let t_b = specialize_node(sc, u.b);
    let np = n as *mut NodeOf<node_opt_aref_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push("v_a".to_string());
    a.push("v_b".to_string());
    a.push(format!("{}u32", (*np).u.line));
    a.push("&mut (*np).u.cc".to_string());
    a.push("&mut (*np).u.tf".to_string());
    a.push(format!("{}u32", { 1 + (*np).u.tf.0 }));
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_opt_aref_struct>;\n", "n"));
    body.push_str(&format!("        let v_a = <{} as Disp>::call(c, {}, fp)?;\n", t_a, child_ptr_expr(n, u.a)));
    body.push_str(&format!("        let v_b = <{} as Disp>::call(c, {}, fp)?;\n", t_b, child_ptr_expr(n, u.b)));
    body.push_str(&format!("        EVAL_node_opt_aref{}({})\n    }} }}\n", "", a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_opt_aset(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_opt_aset_struct>)).u };
    let t_a = specialize_node(sc, u.a);
    let t_b = specialize_node(sc, u.b);
    let t_v = specialize_node(sc, u.v);
    let np = n as *mut NodeOf<node_opt_aset_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push("v_a".to_string());
    a.push("v_b".to_string());
    a.push("v_v".to_string());
    a.push(format!("{}u32", (*np).u.line));
    a.push("&mut (*np).u.cc".to_string());
    a.push("&mut (*np).u.tf".to_string());
    a.push(format!("{}u32", { 1 + (*np).u.tf.0 }));
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_opt_aset_struct>;\n", "n"));
    body.push_str(&format!("        let v_a = <{} as Disp>::call(c, {}, fp)?;\n", t_a, child_ptr_expr(n, u.a)));
    body.push_str(&format!("        let v_b = <{} as Disp>::call(c, {}, fp)?;\n", t_b, child_ptr_expr(n, u.b)));
    body.push_str(&format!("        let v_v = <{} as Disp>::call(c, {}, fp)?;\n", t_v, child_ptr_expr(n, u.v)));
    body.push_str(&format!("        EVAL_node_opt_aset{}({})\n    }} }}\n", "", a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_method_entry(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_method_entry_struct>)).u };
    let t_body = specialize_node(sc, u.body);
    let np = n as *mut NodeOf<node_method_entry_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push(format!("{}u32", (*np).u.nlocals));
    a.push(format!("{}u32", (*np).u.nparams));
    a.push(format!("{}u32", (*np).u.flags));
    a.push(format!("{}u32", (*np).u.line));
    a.push(child_ptr_expr(n, u.body));
    a.push("crate::vm::CallArgs::load(c)".to_string());
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_method_entry_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_method_entry{}({})\n    }} }}\n", format!("::<{}>", t_body), a.join(", ")));

    let a_spec: Vec<String> = a.iter().enumerate().map(|(i, x)| if i == 8 { "ca".to_string() } else { x.clone() }).collect();
    body.push_str(&format!("    #[inline(always)]\n    fn spec(c: &mut Ctx, n: NodePtr, fp: Fp, ca: CallArgs) -> RV {{ unsafe {{\n"));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_method_entry_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_method_entry{}({})\n    }} }}\n", format!("::<{}>", t_body), a_spec.join(", ")));
    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_block_entry(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_block_entry_struct>)).u };
    let t_body = specialize_node(sc, u.body);
    let np = n as *mut NodeOf<node_block_entry_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push(format!("{}u32", (*np).u.nlocals));
    a.push(format!("{}u32", (*np).u.nparams));
    a.push(format!("{}u32", (*np).u.flags));
    a.push(format!("{}u32", (*np).u.line));
    a.push(child_ptr_expr(n, u.body));
    a.push("crate::vm::CallArgs::load(c)".to_string());
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_block_entry_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_block_entry{}({})\n    }} }}\n", format!("::<{}>", t_body), a.join(", ")));

    let a_spec: Vec<String> = a.iter().enumerate().map(|(i, x)| if i == 8 { "ca".to_string() } else { x.clone() }).collect();
    body.push_str(&format!("    #[inline(always)]\n    fn spec(c: &mut Ctx, n: NodePtr, fp: Fp, ca: CallArgs) -> RV {{ unsafe {{\n"));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_block_entry_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_block_entry{}({})\n    }} }}\n", format!("::<{}>", t_body), a_spec.join(", ")));
    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_def(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_def_struct>)).u };

    let np = n as *mut NodeOf<node_def_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push("(*np).u.mid".to_string());
    a.push("(*np).u.body".to_string());
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_def_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_def{}({})\n    }} }}\n", "", a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_sdef(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_sdef_struct>)).u };
    let t_recv = specialize_node(sc, u.recv);
    let np = n as *mut NodeOf<node_sdef_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push("v_recv".to_string());
    a.push("(*np).u.mid".to_string());
    a.push("(*np).u.body".to_string());
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_sdef_struct>;\n", "n"));
    body.push_str(&format!("        let v_recv = <{} as Disp>::call(c, {}, fp)?;\n", t_recv, child_ptr_expr(n, u.recv)));
    body.push_str(&format!("        EVAL_node_sdef{}({})\n    }} }}\n", "", a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_class(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_class_struct>)).u };
    let t_cpath = specialize_node(sc, u.cpath);
    let t_superclass = specialize_node(sc, u.superclass);
    let np = n as *mut NodeOf<node_class_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push(child_ptr_expr(n, u.cpath));
    a.push("(*np).u.name".to_string());
    a.push(child_ptr_expr(n, u.superclass));
    a.push("(*np).u.body".to_string());
    a.push(format!("{}", (*np).u.has_cpath));
    a.push(format!("{}", (*np).u.has_super));
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_class_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_class{}({})\n    }} }}\n", format!("::<{}, {}>", t_cpath, t_superclass), a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_module(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_module_struct>)).u };
    let t_cpath = specialize_node(sc, u.cpath);
    let np = n as *mut NodeOf<node_module_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push(child_ptr_expr(n, u.cpath));
    a.push("(*np).u.name".to_string());
    a.push("(*np).u.body".to_string());
    a.push(format!("{}", (*np).u.has_cpath));
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_module_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_module{}({})\n    }} }}\n", format!("::<{}>", t_cpath), a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_sclass(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_sclass_struct>)).u };
    let t_recv = specialize_node(sc, u.recv);
    let np = n as *mut NodeOf<node_sclass_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push("v_recv".to_string());
    a.push("(*np).u.body".to_string());
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_sclass_struct>;\n", "n"));
    body.push_str(&format!("        let v_recv = <{} as Disp>::call(c, {}, fp)?;\n", t_recv, child_ptr_expr(n, u.recv)));
    body.push_str(&format!("        EVAL_node_sclass{}({})\n    }} }}\n", "", a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_alias(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_alias_struct>)).u };
    let t_newname = specialize_node(sc, u.newname);
    let t_oldname = specialize_node(sc, u.oldname);
    let np = n as *mut NodeOf<node_alias_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push("v_newname".to_string());
    a.push("v_oldname".to_string());
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_alias_struct>;\n", "n"));
    body.push_str(&format!("        let v_newname = <{} as Disp>::call(c, {}, fp)?;\n", t_newname, child_ptr_expr(n, u.newname)));
    body.push_str(&format!("        let v_oldname = <{} as Disp>::call(c, {}, fp)?;\n", t_oldname, child_ptr_expr(n, u.oldname)));
    body.push_str(&format!("        EVAL_node_alias{}({})\n    }} }}\n", "", a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_valias(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_valias_struct>)).u };

    let np = n as *mut NodeOf<node_valias_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push("(*np).u.newname".to_string());
    a.push("(*np).u.oldname".to_string());
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_valias_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_valias{}({})\n    }} }}\n", "", a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_undef(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_undef_struct>)).u };
    let t_names = specialize_list(sc, &u.names, n);
    let np = n as *mut NodeOf<node_undef_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push("list_base(n)".to_string());
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_undef_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_undef{}({})\n    }} }}\n", format!("::<{}>", t_names), a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_masgn(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_masgn_struct>)).u };
    let t_rhs = specialize_node(sc, u.rhs);
    let t_targets = specialize_list(sc, &u.targets, n);
    let np = n as *mut NodeOf<node_masgn_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push("v_rhs".to_string());
    a.push(format!("{}u32", (*np).u.tmp));
    a.push("list_base(n)".to_string());
    a.push(format!("{}", (*np).u.splat_rhs));
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_masgn_struct>;\n", "n"));
    body.push_str(&format!("        let v_rhs = <{} as Disp>::call(c, {}, fp)?;\n", t_rhs, child_ptr_expr(n, u.rhs)));
    body.push_str(&format!("        EVAL_node_masgn{}({})\n    }} }}\n", format!("::<{}>", t_targets), a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_masgn_elem(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_masgn_elem_struct>)).u };

    let np = n as *mut NodeOf<node_masgn_elem_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push(format!("{}u32", (*np).u.tmp));
    a.push(format!("{}u32", (*np).u.idx));
    a.push(format!("{}u32", (*np).u.kind));
    a.push(format!("{}u32", (*np).u.post));
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_masgn_elem_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_masgn_elem{}({})\n    }} }}\n", "", a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_unsupported(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_unsupported_struct>)).u };

    let np = n as *mut NodeOf<node_unsupported_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push("(*np).u.what".to_string());
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_unsupported_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_unsupported{}({})\n    }} }}\n", "", a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_special(sc: &mut SpecCtx, n: NodePtr) -> String {
    let name = format!("T_{:016x}", hash_node(n));
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());   // before the children: a speculated callee may be this very node
    let own_hash = hash_node(n);
    sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_special_struct>)).u };
    let t_arg = specialize_node(sc, u.arg);
    let np = n as *mut NodeOf<node_special_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "fp".to_string()];
    unsafe {
    a.push(format!("{}u32", (*np).u.kind));
    a.push(child_ptr_expr(n, u.arg));
    }
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_special_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_special{}({})\n    }} }}\n", format!("::<{}>", t_arg), a.join(", ")));


    body.push_str("}
");
    sc.open.remove(&own_hash);
    sc.out.push_str(&body);
    name
}

/// SD type for a NODES operand: a `DispList` whose nth() dispatches statically.
pub fn specialize_list(sc: &mut SpecCtx, ns: &NodeList, parent: NodePtr) -> String {
    let ts: Vec<String> = ns.iter().map(|x| specialize_node(sc, *x)).collect();
    let mut h = hash_str("list");
    for x in ns.iter() { h = hash_merge(h, hash_node(*x)); }
    if let Some(x) = ns.first() { h = hash_merge(h, (*x as usize).wrapping_sub(parent as usize) as u64); }
    let name = format!("L_{:016x}", h);
    if sc.emitted.contains(&name) { return name; }
    let mut s = format!("pub struct {};\nimpl DispList for {} {{\n    #[inline(always)] fn len(_ns: NodeList) -> usize {{ {} }}\n", name, name, ts.len());
    s.push_str(&format!("    #[inline(always)]\n    fn nth(c: &mut Ctx, ns: NodeList, i: usize, fp: Fp) -> RV {{\n        match i {{\n"));
    for (i, t) in ts.iter().enumerate() {
        let x = child_ptr_expr(parent, ns[i]).replacen("n.", "(ns.as_ptr() as NodePtr).", 1);
        s.push_str(&format!("            {} => unsafe {{ <{} as Disp>::call(c, {}, fp) }},\n", i, t, x));
    }
    s.push_str("            _ => unsafe { std::hint::unreachable_unchecked() },\n        }\n    }\n}\n");
    sc.emitted.insert(name.clone());
    sc.out.push_str(&s);
    name
}

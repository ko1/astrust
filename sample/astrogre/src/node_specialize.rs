// This file is auto-generated from node.def by astrustgen. Do not edit.
#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_re_succ(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_re_succ_struct>)).u };

    let np = n as *mut NodeOf<node_re_succ_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "pos".to_string()];
    unsafe {

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
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_re_succ_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_re_succ{}({})\n    }} }}\n", "", a.join(", ")));


    body.push_str("}
");
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_re_sub_succ(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_re_sub_succ_struct>)).u };

    let np = n as *mut NodeOf<node_re_sub_succ_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "pos".to_string()];
    unsafe {

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
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_re_sub_succ_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_re_sub_succ{}({})\n    }} }}\n", "", a.join(", ")));


    body.push_str("}
");
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_re_sub_succ_at(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_re_sub_succ_at_struct>)).u };

    let np = n as *mut NodeOf<node_re_sub_succ_at_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "pos".to_string()];
    unsafe {

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
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_re_sub_succ_at_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_re_sub_succ_at{}({})\n    }} }}\n", "", a.join(", ")));


    body.push_str("}
");
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_re_byte(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_re_byte_struct>)).u };
    let t_next = specialize_node(sc, u.next);
    let np = n as *mut NodeOf<node_re_byte_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "pos".to_string()];
    unsafe {
    a.push(format!("{}u8", (*np).u.b));
    a.push("(*np).u.next".to_string());
    }
    if !was_open { sc.open.remove(&own_hash); }
    // named by what is baked too (children's types, @bake values), not by the hash alone:
    // two sites with one hash may have seen different things
    let name = format!("T_{:016x}", hash_merge(own_hash, hash_str(&format!("{}|{}", <[&str]>::join(&[t_next.as_str()], ","), a.join(",")))));
    let name = sc.positional(name, n as usize);
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_re_byte_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_re_byte{}({})\n    }} }}\n", format!("::<{}>", t_next), a.join(", ")));


    body.push_str("}
");
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_re_lit(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_re_lit_struct>)).u };
    let t_next = specialize_node(sc, u.next);
    let np = n as *mut NodeOf<node_re_lit_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "pos".to_string()];
    unsafe {
    a.push(bytes_lit((*np).u.bytes));
    a.push("(*np).u.next".to_string());
    }
    if !was_open { sc.open.remove(&own_hash); }
    // named by what is baked too (children's types, @bake values), not by the hash alone:
    // two sites with one hash may have seen different things
    let name = format!("T_{:016x}", hash_merge(own_hash, hash_str(&format!("{}|{}", <[&str]>::join(&[t_next.as_str()], ","), a.join(",")))));
    let name = sc.positional(name, n as usize);
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_re_lit_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_re_lit{}({})\n    }} }}\n", format!("::<{}>", t_next), a.join(", ")));


    body.push_str("}
");
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_re_one(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_re_one_struct>)).u };
    let t_next = specialize_node(sc, u.next);
    let np = n as *mut NodeOf<node_re_one_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "pos".to_string()];
    unsafe {
    a.push(format!("{}u32", (*np).u.kind));
    a.push(format!("{}u32", (*np).u.ch));
    a.push("(*np).u.cls".to_string());
    a.push("(*np).u.next".to_string());
    }
    if !was_open { sc.open.remove(&own_hash); }
    // named by what is baked too (children's types, @bake values), not by the hash alone:
    // two sites with one hash may have seen different things
    let name = format!("T_{:016x}", hash_merge(own_hash, hash_str(&format!("{}|{}", <[&str]>::join(&[t_next.as_str()], ","), a.join(",")))));
    let name = sc.positional(name, n as usize);
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_re_one_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_re_one{}({})\n    }} }}\n", format!("::<{}>", t_next), a.join(", ")));


    body.push_str("}
");
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_re_grapheme(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_re_grapheme_struct>)).u };
    let t_next = specialize_node(sc, u.next);
    let np = n as *mut NodeOf<node_re_grapheme_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "pos".to_string()];
    unsafe {
    a.push("(*np).u.next".to_string());
    }
    if !was_open { sc.open.remove(&own_hash); }
    // named by what is baked too (children's types, @bake values), not by the hash alone:
    // two sites with one hash may have seen different things
    let name = format!("T_{:016x}", hash_merge(own_hash, hash_str(&format!("{}|{}", <[&str]>::join(&[t_next.as_str()], ","), a.join(",")))));
    let name = sc.positional(name, n as usize);
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_re_grapheme_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_re_grapheme{}({})\n    }} }}\n", format!("::<{}>", t_next), a.join(", ")));


    body.push_str("}
");
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_re_rep1(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_re_rep1_struct>)).u };
    let t_next = specialize_node(sc, u.next);
    let np = n as *mut NodeOf<node_re_rep1_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "pos".to_string()];
    unsafe {
    a.push(format!("{}u32", (*np).u.kind));
    a.push(format!("{}u32", (*np).u.ch));
    a.push("(*np).u.cls".to_string());
    a.push(format!("{}u32", (*np).u.min));
    a.push(format!("{}u32", (*np).u.max));
    a.push(format!("{}", (*np).u.greedy));
    a.push(format!("{}i32", (*np).u.nb));
    a.push("(*np).u.next".to_string());
    }
    if !was_open { sc.open.remove(&own_hash); }
    // named by what is baked too (children's types, @bake values), not by the hash alone:
    // two sites with one hash may have seen different things
    let name = format!("T_{:016x}", hash_merge(own_hash, hash_str(&format!("{}|{}", <[&str]>::join(&[t_next.as_str()], ","), a.join(",")))));
    let name = sc.positional(name, n as usize);
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_re_rep1_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_re_rep1{}({})\n    }} }}\n", format!("::<{}>", t_next), a.join(", ")));


    body.push_str("}
");
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_re_assert(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_re_assert_struct>)).u };
    let t_next = specialize_node(sc, u.next);
    let np = n as *mut NodeOf<node_re_assert_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "pos".to_string()];
    unsafe {
    a.push(format!("{}u32", (*np).u.kind));
    a.push("(*np).u.next".to_string());
    }
    if !was_open { sc.open.remove(&own_hash); }
    // named by what is baked too (children's types, @bake values), not by the hash alone:
    // two sites with one hash may have seen different things
    let name = format!("T_{:016x}", hash_merge(own_hash, hash_str(&format!("{}|{}", <[&str]>::join(&[t_next.as_str()], ","), a.join(",")))));
    let name = sc.positional(name, n as usize);
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_re_assert_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_re_assert{}({})\n    }} }}\n", format!("::<{}>", t_next), a.join(", ")));


    body.push_str("}
");
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_re_keep(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_re_keep_struct>)).u };
    let t_next = specialize_node(sc, u.next);
    let np = n as *mut NodeOf<node_re_keep_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "pos".to_string()];
    unsafe {
    a.push("(*np).u.next".to_string());
    }
    if !was_open { sc.open.remove(&own_hash); }
    // named by what is baked too (children's types, @bake values), not by the hash alone:
    // two sites with one hash may have seen different things
    let name = format!("T_{:016x}", hash_merge(own_hash, hash_str(&format!("{}|{}", <[&str]>::join(&[t_next.as_str()], ","), a.join(",")))));
    let name = sc.positional(name, n as usize);
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_re_keep_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_re_keep{}({})\n    }} }}\n", format!("::<{}>", t_next), a.join(", ")));


    body.push_str("}
");
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_re_open(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_re_open_struct>)).u };
    let t_next = specialize_node(sc, u.next);
    let np = n as *mut NodeOf<node_re_open_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "pos".to_string()];
    unsafe {
    a.push(format!("{}u32", (*np).u.g));
    a.push("(*np).u.next".to_string());
    }
    if !was_open { sc.open.remove(&own_hash); }
    // named by what is baked too (children's types, @bake values), not by the hash alone:
    // two sites with one hash may have seen different things
    let name = format!("T_{:016x}", hash_merge(own_hash, hash_str(&format!("{}|{}", <[&str]>::join(&[t_next.as_str()], ","), a.join(",")))));
    let name = sc.positional(name, n as usize);
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_re_open_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_re_open{}({})\n    }} }}\n", format!("::<{}>", t_next), a.join(", ")));


    body.push_str("}
");
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_re_open_nr(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_re_open_nr_struct>)).u };
    let t_next = specialize_node(sc, u.next);
    let np = n as *mut NodeOf<node_re_open_nr_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "pos".to_string()];
    unsafe {
    a.push(format!("{}u32", (*np).u.g));
    a.push("(*np).u.next".to_string());
    }
    if !was_open { sc.open.remove(&own_hash); }
    // named by what is baked too (children's types, @bake values), not by the hash alone:
    // two sites with one hash may have seen different things
    let name = format!("T_{:016x}", hash_merge(own_hash, hash_str(&format!("{}|{}", <[&str]>::join(&[t_next.as_str()], ","), a.join(",")))));
    let name = sc.positional(name, n as usize);
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_re_open_nr_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_re_open_nr{}({})\n    }} }}\n", format!("::<{}>", t_next), a.join(", ")));


    body.push_str("}
");
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_re_close(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_re_close_struct>)).u };
    let t_next = specialize_node(sc, u.next);
    let np = n as *mut NodeOf<node_re_close_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "pos".to_string()];
    unsafe {
    a.push(format!("{}u32", (*np).u.g));
    a.push("(*np).u.next".to_string());
    }
    if !was_open { sc.open.remove(&own_hash); }
    // named by what is baked too (children's types, @bake values), not by the hash alone:
    // two sites with one hash may have seen different things
    let name = format!("T_{:016x}", hash_merge(own_hash, hash_str(&format!("{}|{}", <[&str]>::join(&[t_next.as_str()], ","), a.join(",")))));
    let name = sc.positional(name, n as usize);
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_re_close_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_re_close{}({})\n    }} }}\n", format!("::<{}>", t_next), a.join(", ")));


    body.push_str("}
");
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_re_backref(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_re_backref_struct>)).u };
    let t_next = specialize_node(sc, u.next);
    let np = n as *mut NodeOf<node_re_backref_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "pos".to_string()];
    unsafe {
    a.push("(*np).u.groups".to_string());
    a.push(format!("{}", (*np).u.icase));
    a.push("(*np).u.next".to_string());
    }
    if !was_open { sc.open.remove(&own_hash); }
    // named by what is baked too (children's types, @bake values), not by the hash alone:
    // two sites with one hash may have seen different things
    let name = format!("T_{:016x}", hash_merge(own_hash, hash_str(&format!("{}|{}", <[&str]>::join(&[t_next.as_str()], ","), a.join(",")))));
    let name = sc.positional(name, n as usize);
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_re_backref_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_re_backref{}({})\n    }} }}\n", format!("::<{}>", t_next), a.join(", ")));


    body.push_str("}
");
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_re_open_rec(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_re_open_rec_struct>)).u };
    let t_next = specialize_node(sc, u.next);
    let np = n as *mut NodeOf<node_re_open_rec_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "pos".to_string()];
    unsafe {
    a.push(format!("{}u32", (*np).u.g));
    a.push("(*np).u.next".to_string());
    }
    if !was_open { sc.open.remove(&own_hash); }
    // named by what is baked too (children's types, @bake values), not by the hash alone:
    // two sites with one hash may have seen different things
    let name = format!("T_{:016x}", hash_merge(own_hash, hash_str(&format!("{}|{}", <[&str]>::join(&[t_next.as_str()], ","), a.join(",")))));
    let name = sc.positional(name, n as usize);
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_re_open_rec_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_re_open_rec{}({})\n    }} }}\n", format!("::<{}>", t_next), a.join(", ")));


    body.push_str("}
");
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_re_close_rec(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_re_close_rec_struct>)).u };
    let t_next = specialize_node(sc, u.next);
    let np = n as *mut NodeOf<node_re_close_rec_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "pos".to_string()];
    unsafe {
    a.push(format!("{}u32", (*np).u.g));
    a.push("(*np).u.next".to_string());
    }
    if !was_open { sc.open.remove(&own_hash); }
    // named by what is baked too (children's types, @bake values), not by the hash alone:
    // two sites with one hash may have seen different things
    let name = format!("T_{:016x}", hash_merge(own_hash, hash_str(&format!("{}|{}", <[&str]>::join(&[t_next.as_str()], ","), a.join(",")))));
    let name = sc.positional(name, n as usize);
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_re_close_rec_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_re_close_rec{}({})\n    }} }}\n", format!("::<{}>", t_next), a.join(", ")));


    body.push_str("}
");
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_re_call(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_re_call_struct>)).u };
    let t_next = specialize_node(sc, u.next);
    let np = n as *mut NodeOf<node_re_call_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "pos".to_string()];
    unsafe {
    a.push(format!("{}u32", (*np).u.g));
    a.push("(*np).u.next".to_string());
    }
    if !was_open { sc.open.remove(&own_hash); }
    // named by what is baked too (children's types, @bake values), not by the hash alone:
    // two sites with one hash may have seen different things
    let name = format!("T_{:016x}", hash_merge(own_hash, hash_str(&format!("{}|{}", <[&str]>::join(&[t_next.as_str()], ","), a.join(",")))));
    let name = sc.positional(name, n as usize);
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_re_call_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_re_call{}({})\n    }} }}\n", format!("::<{}>", t_next), a.join(", ")));


    body.push_str("}
");
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_re_ret(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_re_ret_struct>)).u };

    let np = n as *mut NodeOf<node_re_ret_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "pos".to_string()];
    unsafe {

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
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_re_ret_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_re_ret{}({})\n    }} }}\n", "", a.join(", ")));


    body.push_str("}
");
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_re_backref_lv(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_re_backref_lv_struct>)).u };
    let t_next = specialize_node(sc, u.next);
    let np = n as *mut NodeOf<node_re_backref_lv_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "pos".to_string()];
    unsafe {
    a.push("(*np).u.groups".to_string());
    a.push(format!("{}", (*np).u.icase));
    a.push(format!("{}i32", (*np).u.nest));
    a.push("(*np).u.next".to_string());
    }
    if !was_open { sc.open.remove(&own_hash); }
    // named by what is baked too (children's types, @bake values), not by the hash alone:
    // two sites with one hash may have seen different things
    let name = format!("T_{:016x}", hash_merge(own_hash, hash_str(&format!("{}|{}", <[&str]>::join(&[t_next.as_str()], ","), a.join(",")))));
    let name = sc.positional(name, n as usize);
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_re_backref_lv_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_re_backref_lv{}({})\n    }} }}\n", format!("::<{}>", t_next), a.join(", ")));


    body.push_str("}
");
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_re_cond(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_re_cond_struct>)).u };
    let t_yes = specialize_node(sc, u.yes);
    let t_no = specialize_node(sc, u.no);
    let np = n as *mut NodeOf<node_re_cond_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "pos".to_string()];
    unsafe {
    a.push("(*np).u.groups".to_string());
    a.push("(*np).u.yes".to_string());
    a.push("(*np).u.no".to_string());
    }
    if !was_open { sc.open.remove(&own_hash); }
    // named by what is baked too (children's types, @bake values), not by the hash alone:
    // two sites with one hash may have seen different things
    let name = format!("T_{:016x}", hash_merge(own_hash, hash_str(&format!("{}|{}", <[&str]>::join(&[t_yes.as_str(), t_no.as_str()], ","), a.join(",")))));
    let name = sc.positional(name, n as usize);
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_re_cond_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_re_cond{}({})\n    }} }}\n", format!("::<{}, {}>", t_yes, t_no), a.join(", ")));


    body.push_str("}
");
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_re_alt(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_re_alt_struct>)).u };
    let t_a = specialize_node(sc, u.a);
    let t_b = specialize_node(sc, u.b);
    let np = n as *mut NodeOf<node_re_alt_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "pos".to_string()];
    unsafe {
    a.push("(*np).u.a".to_string());
    a.push("(*np).u.b".to_string());
    }
    if !was_open { sc.open.remove(&own_hash); }
    // named by what is baked too (children's types, @bake values), not by the hash alone:
    // two sites with one hash may have seen different things
    let name = format!("T_{:016x}", hash_merge(own_hash, hash_str(&format!("{}|{}", <[&str]>::join(&[t_a.as_str(), t_b.as_str()], ","), a.join(",")))));
    let name = sc.positional(name, n as usize);
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_re_alt_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_re_alt{}({})\n    }} }}\n", format!("::<{}, {}>", t_a, t_b), a.join(", ")));


    body.push_str("}
");
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_re_opt(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_re_opt_struct>)).u };
    let t_body = specialize_node(sc, u.body);
    let t_skip = specialize_node(sc, u.skip);
    let np = n as *mut NodeOf<node_re_opt_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "pos".to_string()];
    unsafe {
    a.push("(*np).u.body".to_string());
    a.push("(*np).u.skip".to_string());
    a.push(format!("{}", (*np).u.greedy));
    }
    if !was_open { sc.open.remove(&own_hash); }
    // named by what is baked too (children's types, @bake values), not by the hash alone:
    // two sites with one hash may have seen different things
    let name = format!("T_{:016x}", hash_merge(own_hash, hash_str(&format!("{}|{}", <[&str]>::join(&[t_body.as_str(), t_skip.as_str()], ","), a.join(",")))));
    let name = sc.positional(name, n as usize);
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_re_opt_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_re_opt{}({})\n    }} }}\n", format!("::<{}, {}>", t_body, t_skip), a.join(", ")));


    body.push_str("}
");
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_re_rep(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_re_rep_struct>)).u };
    let t_body = specialize_node(sc, u.body);
    let t_outer = specialize_node(sc, u.outer);
    let np = n as *mut NodeOf<node_re_rep_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "pos".to_string()];
    unsafe {
    a.push("(*np).u.body".to_string());
    a.push("(*np).u.outer".to_string());
    a.push(format!("{}u32", (*np).u.min));
    a.push(format!("{}u32", (*np).u.max));
    a.push(format!("{}", (*np).u.greedy));
    a.push(format!("{}", (*np).u.mem));
    a.push(format!("{}u32", (*np).u.shape));
    a.push(format!("{}u32", (*np).u.cost));
    }
    if !was_open { sc.open.remove(&own_hash); }
    // named by what is baked too (children's types, @bake values), not by the hash alone:
    // two sites with one hash may have seen different things
    let name = format!("T_{:016x}", hash_merge(own_hash, hash_str(&format!("{}|{}", <[&str]>::join(&[t_body.as_str(), t_outer.as_str()], ","), a.join(",")))));
    let name = sc.positional(name, n as usize);
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_re_rep_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_re_rep{}({})\n    }} }}\n", format!("::<{}, {}>", t_body, t_outer), a.join(", ")));


    body.push_str("}
");
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_re_rep_cont(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_re_rep_cont_struct>)).u };

    let np = n as *mut NodeOf<node_re_rep_cont_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "pos".to_string()];
    unsafe {

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
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_re_rep_cont_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_re_rep_cont{}({})\n    }} }}\n", "", a.join(", ")));


    body.push_str("}
");
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_re_look(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_re_look_struct>)).u };
    let t_body = specialize_node(sc, u.body);
    let t_next = specialize_node(sc, u.next);
    let np = n as *mut NodeOf<node_re_look_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "pos".to_string()];
    unsafe {
    a.push(format!("{}u32", (*np).u.kind));
    a.push("(*np).u.body".to_string());
    a.push("(*np).u.next".to_string());
    a.push(format!("{}u32", (*np).u.minlen));
    a.push(format!("{}u32", (*np).u.maxlen));
    }
    if !was_open { sc.open.remove(&own_hash); }
    // named by what is baked too (children's types, @bake values), not by the hash alone:
    // two sites with one hash may have seen different things
    let name = format!("T_{:016x}", hash_merge(own_hash, hash_str(&format!("{}|{}", <[&str]>::join(&[t_body.as_str(), t_next.as_str()], ","), a.join(",")))));
    let name = sc.positional(name, n as usize);
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_re_look_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_re_look{}({})\n    }} }}\n", format!("::<{}, {}>", t_body, t_next), a.join(", ")));


    body.push_str("}
");
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_search(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_search_struct>)).u };
    let t_body = specialize_node(sc, u.body);
    let np = n as *mut NodeOf<node_search_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "pos".to_string()];
    unsafe {
    a.push("(*np).u.body".to_string());
    a.push(format!("{}u32", (*np).u.anchor));
    }
    if !was_open { sc.open.remove(&own_hash); }
    // named by what is baked too (children's types, @bake values), not by the hash alone:
    // two sites with one hash may have seen different things
    let name = format!("T_{:016x}", hash_merge(own_hash, hash_str(&format!("{}|{}", <[&str]>::join(&[t_body.as_str()], ","), a.join(",")))));
    let name = sc.positional(name, n as usize);
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_search_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_search{}({})\n    }} }}\n", format!("::<{}>", t_body), a.join(", ")));


    body.push_str("}
");
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_search_memchr(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_search_memchr_struct>)).u };
    let t_body = specialize_node(sc, u.body);
    let np = n as *mut NodeOf<node_search_memchr_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "pos".to_string()];
    unsafe {
    a.push("(*np).u.body".to_string());
    a.push(format!("{}u8", (*np).u.b));
    }
    if !was_open { sc.open.remove(&own_hash); }
    // named by what is baked too (children's types, @bake values), not by the hash alone:
    // two sites with one hash may have seen different things
    let name = format!("T_{:016x}", hash_merge(own_hash, hash_str(&format!("{}|{}", <[&str]>::join(&[t_body.as_str()], ","), a.join(",")))));
    let name = sc.positional(name, n as usize);
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_search_memchr_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_search_memchr{}({})\n    }} }}\n", format!("::<{}>", t_body), a.join(", ")));


    body.push_str("}
");
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_search_memmem(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_search_memmem_struct>)).u };
    let t_body = specialize_node(sc, u.body);
    let np = n as *mut NodeOf<node_search_memmem_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "pos".to_string()];
    unsafe {
    a.push("(*np).u.body".to_string());
    a.push(bytes_lit((*np).u.needle));
    }
    if !was_open { sc.open.remove(&own_hash); }
    // named by what is baked too (children's types, @bake values), not by the hash alone:
    // two sites with one hash may have seen different things
    let name = format!("T_{:016x}", hash_merge(own_hash, hash_str(&format!("{}|{}", <[&str]>::join(&[t_body.as_str()], ","), a.join(",")))));
    let name = sc.positional(name, n as usize);
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_search_memmem_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_search_memmem{}({})\n    }} }}\n", format!("::<{}>", t_body), a.join(", ")));


    body.push_str("}
");
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_search_byteset(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_search_byteset_struct>)).u };
    let t_body = specialize_node(sc, u.body);
    let np = n as *mut NodeOf<node_search_byteset_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "pos".to_string()];
    unsafe {
    a.push("(*np).u.body".to_string());
    a.push("(*np).u.set".to_string());
    }
    if !was_open { sc.open.remove(&own_hash); }
    // named by what is baked too (children's types, @bake values), not by the hash alone:
    // two sites with one hash may have seen different things
    let name = format!("T_{:016x}", hash_merge(own_hash, hash_str(&format!("{}|{}", <[&str]>::join(&[t_body.as_str()], ","), a.join(",")))));
    let name = sc.positional(name, n as usize);
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_search_byteset_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_search_byteset{}({})\n    }} }}\n", format!("::<{}>", t_body), a.join(", ")));


    body.push_str("}
");
    sc.out.push_str(&body);
    name
}

#[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
fn SPECIALIZE_node_match_at(sc: &mut SpecCtx, n: NodePtr) -> String {
    let own_hash = hash_node(n);
    let was_open = !sc.open.insert(own_hash);
    let u = unsafe { &(*(n as *mut NodeOf<node_match_at_struct>)).u };
    let t_body = specialize_node(sc, u.body);
    let np = n as *mut NodeOf<node_match_at_struct>;
    let mut a: Vec<String> = vec!["c".to_string(), "n".to_string(), "pos".to_string()];
    unsafe {
    a.push("(*np).u.body".to_string());
    }
    if !was_open { sc.open.remove(&own_hash); }
    // named by what is baked too (children's types, @bake values), not by the hash alone:
    // two sites with one hash may have seen different things
    let name = format!("T_{:016x}", hash_merge(own_hash, hash_str(&format!("{}|{}", <[&str]>::join(&[t_body.as_str()], ","), a.join(",")))));
    let name = sc.positional(name, n as usize);
    if sc.emitted.contains(&name) { return name; }
    sc.emitted.insert(name.clone());
    let mut body = String::new();
    body.push_str(&format!("pub struct {};\n", name));
    if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\n'); }
    body.push_str(&format!("impl Disp for {} {{\n    #[inline(always)]\n    fn call(c: &mut Ctx, n: NodePtr, pos: usize) -> u32 {{ unsafe {{\n", name));
    body.push_str(&format!("        let np = {} as *mut NodeOf<node_match_at_struct>;\n", "n"));

    body.push_str(&format!("        EVAL_node_match_at{}({})\n    }} }}\n", format!("::<{}>", t_body), a.join(", ")));


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
    
    // the elements' names too: one list node specialized twice (a site, its unrolled copy) is two lists
    h = hash_merge(h, hash_str(&ts.join(",")));
    let name = sc.positional(format!("L_{:016x}", h), 0);
    if sc.emitted.contains(&name) { return name; }
    let mut s = format!("pub struct {};\nimpl DispList for {} {{\n    #[inline(always)] fn len(_ns: NodeList) -> usize {{ {} }}\n", name, name, ts.len());
    s.push_str(&format!("    #[inline(always)]\n    fn nth(c: &mut Ctx, ns: NodeList, i: usize, pos: usize) -> u32 {{\n        match i {{\n"));
    for (i, t) in ts.iter().enumerate() {
        let x = format!("*ns.get_unchecked({})", i);
        s.push_str(&format!("            {} => unsafe {{ <{} as Disp>::call(c, {}, pos) }},\n", i, t, x));
    }
    s.push_str("            _ => unsafe { std::hint::unreachable_unchecked() },\n        }\n    }\n");
    // EVAL_EACH!: the calls in a row
    s.push_str("    #[inline(always)]\n    fn each<F: FnMut(&mut Ctx, usize, u32) -> ()>(c: &mut Ctx, ns: NodeList, pos: usize, mut f: F) -> () {\n");
    for (i, t) in ts.iter().enumerate() {
        let x = format!("*ns.get_unchecked({})", i);
        let call = format!("<{} as Disp>::call(c, unsafe {{ {} }}, pos)", t, x);
        let step = format!("{{ let r = {}; f(c, {}, r) }}", "@CALL@".replace("@CALL@", &call), i);
        s.push_str(&format!("        {}\n", "@STEP@;".replace("@STEP@", &step)));
    }
    s.push_str("        ()\n    }\n}\n");
    sc.emitted.insert(name.clone());
    sc.out.push_str(&s);
    name
}

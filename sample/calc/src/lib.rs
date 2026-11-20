// calc: the smallest ASTrust sample.  Grammar:
//   prog := expr (';' expr)*        expr := term (('+'|'-') term)*
//   term := atom (('*'|'%') atom)*   atom := NUM | VAR | VAR '=' expr | '(' prog ')' | atom 'times' '{' prog '}'
#![allow(non_snake_case, non_camel_case_types, dead_code)]
#[path = "../../../runtime/astrust.rs"]
pub mod astrust;
pub use astrust::*;

pub struct Ctx { pub vars: [i64; 26] }

pub mod node {
    use super::*;
    include!("node_head.rs");
    include!("node_dispatch.rs");
    include!("node_hash.rs");
    include!("node_specialize.rs");
    include!("node_dump.rs");
    include!("node_replace.rs");
    pub fn node_allocated(_n: NodePtr) {}
}
pub use node::*;
pub mod eval { include!("node_eval.rs"); }
pub use eval::*;

struct P<'a> { s: &'a [u8], i: usize }
impl<'a> P<'a> {
    fn ws(&mut self) { while self.i < self.s.len() && (self.s[self.i] as char).is_whitespace() { self.i += 1; } }
    fn eat(&mut self, t: &str) -> bool { self.ws(); if self.s[self.i..].starts_with(t.as_bytes()) { self.i += t.len(); true } else { false } }
    fn prog(&mut self) -> NodePtr {
        let mut xs = vec![self.expr()];
        while self.eat(";") { xs.push(self.expr()); }
        if xs.len() == 1 { xs[0] } else { ALLOC_node_seq(Box::leak(xs.into_boxed_slice())) }
    }
    fn expr(&mut self) -> NodePtr {
        let mut l = self.term();
        loop {
            if self.eat("+") { let r = self.term(); l = ALLOC_node_add(l, r); }
            else if self.eat("-") { let r = self.term(); l = ALLOC_node_sub(l, r); }
            else { return l; }
        }
    }
    fn term(&mut self) -> NodePtr {
        let mut l = self.post();
        loop {
            if self.eat("*") { let r = self.post(); l = ALLOC_node_mul(l, r); }
            else if self.eat("%") { let r = self.post(); l = ALLOC_node_mod(l, r); }
            else { return l; }
        }
    }
    fn post(&mut self) -> NodePtr {
        let mut a = self.atom();
        while self.eat("times") { assert!(self.eat("{")); let b = self.prog(); assert!(self.eat("}")); a = ALLOC_node_times(a, b); }
        a
    }
    fn atom(&mut self) -> NodePtr {
        self.ws();
        if self.eat("(") { let e = self.prog(); assert!(self.eat(")")); return e; }
        let c = self.s[self.i];
        if c.is_ascii_digit() {
            let st = self.i; while self.i < self.s.len() && self.s[self.i].is_ascii_digit() { self.i += 1; }
            return ALLOC_node_num(std::str::from_utf8(&self.s[st..self.i]).unwrap().parse().unwrap());
        }
        if c.is_ascii_lowercase() {
            self.i += 1;
            let idx = (c - b'a') as u32;
            let save = self.i;
            if self.eat("=") && !self.eat("=") { let v = self.expr(); return ALLOC_node_set(idx, v); }
            self.i = save;
            return ALLOC_node_var(idx);
        }
        panic!("parse error at {}", self.i);
    }
}

pub fn parse(src: &str) -> NodePtr { let mut p = P { s: src.as_bytes(), i: 0 }; p.prog() }

fn bake_config() -> BakeConfig {
    let exe = std::env::current_exe().unwrap();
    let dir = exe.parent().unwrap().to_path_buf();
    let sys = PathBuf::from(env!("CALC_SYSROOT")).join("lib/rustlib").join(env!("CALC_TARGET")).join("lib");
    BakeConfig {
        crate_name: "calc".into(), crate_path: dir.join("libcalc.so"),
        lib_dirs: vec![dir.clone()], rpaths: vec![sys, dir],
        prelude: "extern crate calc;\nuse calc::*;\n".into(),
        build_id: env!("CALC_BUILD_ID").into(), verbose: std::env::var_os("CALC_VERBOSE").is_some(),
    }
}
use std::path::PathBuf;

pub fn main_entry() {
    let args: Vec<String> = std::env::args().collect();
    let mut aot = false; let mut plain = false; let mut src = None;
    for a in &args[1..] {
        match a.as_str() { "--aot-compile" => aot = true, "--plain" => plain = true, s => src = Some(s.to_string()) }
    }
    let src = src.expect("usage: calc [--aot-compile|--plain] 'expr'");
    let root = parse(&src);
    let mut cs = CodeStore::new("code_store");
    let cfg = bake_config();
    if !plain && cs.load(&cfg.build_id) {
        if let Some(p) = cs.lookup(hash_node(root)) {
            unsafe { (*root).head.dispatcher = std::mem::transmute::<*const std::os::raw::c_void, DispatchFn>(p); }
            if std::env::var_os("CALC_VERBOSE").is_some() { eprintln!("calc: using SD_{:016x}", hash_node(root)); }
        }
    }
    let mut c = Ctx { vars: [0; 26] };
    let t = std::time::Instant::now();
    let v = unsafe { ((*root).head.dispatcher)(&mut c, root) };
    println!("{}", v);
    if std::env::var_os("CALC_VERBOSE").is_some() { eprintln!("calc: {:?}", t.elapsed()); }
    if aot {
        let mut sc = SpecCtx::new();
        specialize_entry(&mut sc, root);
        std::fs::create_dir_all("code_store").unwrap();
        cs.add_items(&sc.out).unwrap();
        if let Err(e) = cs.build(&cfg) { eprintln!("calc: bake failed:\n{}", e); std::process::exit(1); }
    }
}

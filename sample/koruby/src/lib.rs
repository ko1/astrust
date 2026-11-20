pub mod prism;
pub mod prism_ast;
pub fn main_entry() {
    let a: Vec<String> = std::env::args().collect();
    let src = std::fs::read(&a[1]).unwrap();
    let r = prism::parse(&src, &prism::ParseOpts { filepath: a[1].as_bytes(), line: 1, scopes: vec![], main_script: true, frozen_string_literal: 0 });
    println!("{:#?}", r.root);
}

// astrogre: a Ruby (Onigmo) compatible regex engine on ASTrust.
// The pattern is parsed (re.rs), lowered into continuation-passing match nodes (node.def,
// cps.rs) whose search loop is itself a node, so an ASTrust bake fuses loop and pattern.
#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals, dead_code)]
#[path = "../../../runtime/astrust.rs"]
pub mod astrust;
pub mod re;
pub mod uniprops;

pub use astrust::*;
pub use re::cps::*;
pub use re::eval::*;
pub use re::node::*;
pub use re::{parse_warnings, take_warnings, Enc, OPT_I, OPT_M, OPT_X};

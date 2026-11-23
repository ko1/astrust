// A prism node read with every field (prism_ast.rs SCHEMA): the compiler's AST drops most locations, but
// error_highlight re-parses a file and needs them all, as the prism gem's nodes have them.
#![forbid(unsafe_code)]
use crate::prism::{Loc, Reader};
use crate::prism_ast::SCHEMA;

pub const F_NODE: u8 = 0;
pub const F_OPT_NODE: u8 = 1;
pub const F_NODES: u8 = 2;
pub const F_STR: u8 = 3;
pub const F_CST: u8 = 4;
pub const F_OPT_CST: u8 = 5;
pub const F_CSTS: u8 = 6;
pub const F_LOC: u8 = 7;
pub const F_OPT_LOC: u8 = 8;
pub const F_U8: u8 = 9;
pub const F_U32: u8 = 10;
pub const F_INT: u8 = 11;
pub const F_F64: u8 = 12;

pub enum Field { Node(Option<Box<GNode>>), Nodes(Vec<GNode>), Str(Vec<u8>), Cst(Option<u32>), Csts(Vec<u32>), Loc(Option<Loc>), Int(u32), Other }
pub struct GNode { pub ty: &'static str, pub id: u32, pub loc: Loc, pub fields: Vec<(&'static str, Field)> }

fn read(r: &mut Reader) -> GNode {
    let t = r.byte() as usize;
    let id = r.varu();
    let loc = r.loc();
    let (ty, len, schema) = SCHEMA[t - 1];
    if len { r.skip(4); }
    let _flags = r.varu();
    let fields = schema.iter().map(|&(name, f)| (name, match f {
        F_NODE => Field::Node(Some(Box::new(read(r)))),
        F_OPT_NODE => Field::Node(if r.buf[r.pos] == 0 { r.pos += 1; None } else { Some(Box::new(read(r))) }),
        F_NODES => { let k = r.varu(); Field::Nodes((0..k).map(|_| read(r)).collect()) }
        F_STR => Field::Str(r.string()),
        F_CST => Field::Cst(Some(r.varu() - 1)),
        F_OPT_CST => { let k = r.varu(); Field::Cst(k.checked_sub(1)) }
        F_CSTS => { let k = r.varu(); Field::Csts((0..k).map(|_| r.varu() - 1).collect()) }
        F_LOC => Field::Loc(Some(r.loc())),
        F_OPT_LOC => Field::Loc(r.opt_loc()),
        F_U8 => Field::Int(r.byte() as u32),
        F_U32 => Field::Int(r.varu()),
        F_INT => { r.integer(); Field::Other }
        _ => { r.double(); Field::Other }
    })).collect();
    GNode { ty, id, loc, fields }
}

impl GNode {
    /// The node with prism node id `id` in this tree.
    pub fn find(&self, id: u32) -> Option<&GNode> {
        if self.id == id { return Some(self); }
        self.fields.iter().find_map(|(_, f)| match f {
            Field::Node(Some(n)) => n.find(id),
            Field::Nodes(v) => v.iter().find_map(|n| n.find(id)),
            _ => None,
        })
    }
}

/// A parse of `src` as prism's default options would make it (the gem's Prism.parse_file): its root,
/// line offsets and constant pool.
pub fn parse(src: &[u8], file: &[u8]) -> (GNode, Vec<u32>, Vec<Vec<u8>>) {
    let bytes = crate::prism::serialize(src, &crate::prism::ParseOpts { filepath: file, line: 1, scopes: vec![], main_script: false, frozen_string_literal: 0, encoding: b"" });
    let mut r = Reader { buf: &bytes, pos: 0 };
    let p = crate::prism::read_prefix(&mut r);
    let root = read(&mut r);
    let constants = p.constants(&bytes);
    (root, p.line_offsets, constants)
}

// prism AST → ASTrust nodes.
//
// Locals: each scope (method, block, class body, program) gets frame slots
// [self, env, locals...].  A variable that a nested block reads or writes is
// "captured" and lives in the scope's heap Env instead (found by a pre-pass over
// the hard scope); access from a block walks `hops` env parents.
use crate::node::*;
use crate::prism::{self, Loc, ParseResult};
use crate::prism_ast::{self as pa, Kind, Node as PNode};
use crate::symbol::{intern, s};
use crate::value::*;
use crate::vm::*;
use std::collections::{HashMap, HashSet};

pub type IseqRef = &'static Iseq;
pub type CiRef = &'static CallInfo;
pub type Str = &'static [u8];

#[derive(Default)]
pub struct LitCache { pub v: u64 }

/// Argument kinds of a general call (node_call_x / node_super / node_yield_x).
pub const AK_ARG: u8 = 0;
pub const AK_SPLAT: u8 = 1;
pub const AK_KW: u8 = 2;      // a keyword Hash built by the call site
pub const AK_KWSPLAT: u8 = 3; // **h
pub const AK_BLOCK: u8 = 4;   // &blk

pub struct CallInfo { pub kinds: Vec<u8>, pub blk: Option<&'static Iseq>, pub kw_splat_only: bool, pub kw_names: Vec<Sym> }

// Merkle hashes of the opaque operands: by content, never by an address or an id of this process,
// so the same code in another program (or at another place) gets the same SD (Reusable).

/// An immediate by its bits (value_bake bakes it), a Symbol by its name, a heap literal by content
/// (it is read from the node at run time, so its content only keeps unlike sites apart).
pub fn value_hash(v: Value) -> u64 {
    use crate::astrust::{hash_bytes, hash_merge, hash_str, hash_u64};
    if v.is_sym() { hash_merge(hash_str("sym"), hash_bytes(crate::symbol::sym_bytes(v.as_sym()))) }
    else if !v.is_heap() { hash_u64(v.raw()) }
    else if crate::heap::is_heap_float(v) { hash_merge(hash_str("float"), hash_u64(crate::heap::float_val(v).to_bits())) }
    else if crate::object::is_string(v) { hash_merge(hash_str("str"), hash_bytes(crate::object::str_bytes(v))) }
    else { hash_str("heap") }
}
pub fn value_bake(v: Value) -> Option<String> {
    if v.is_sym() || v.is_heap() { None } else { Some(format!("unsafe {{ Value::from_raw({:#x}) }}", v.raw())) }
}
/// A method / block / class body by its structure: the body's tree and the parameters' shape (not the
/// names, label or line: those are read from the iseq at run time).
pub fn iseq_hash(i: IseqRef) -> u64 {
    use crate::astrust::{hash_bytes, hash_merge as m, hash_str, hash_u64 as u};
    let opt_u = |x: Option<u32>| x.map_or(0, |v| v as u64 + 1);
    let list = |h: u64, xs: &[u32]| xs.iter().fold(m(h, u(xs.len() as u64)), |h, &x| m(h, u(x as u64)));
    let p = &i.params;
    let mut h = m(hash_str("iseq"), hash_node(i.body));
    h = m(h, u(i.kind as u64));
    h = m(h, u(i.nlocals as u64 | (i.env_size as u64) << 32));
    for &(a, b) in i.env_params.iter() { h = m(h, u(a as u64 | (b as u64) << 32)); }
    h = list(h, &p.lead);
    h = list(h, &p.post);
    for &(sl, d) in p.opt.iter() { h = m(m(h, u(sl as u64)), hash_node(d)); }
    for k in p.kw.iter() {
        h = m(h, hash_bytes(crate::symbol::sym_bytes(k.name)));
        h = m(h, u(k.slot as u64 | (k.required as u64) << 32 | (k.static_default as u64) << 33));
        h = m(h, k.default.map_or(0, hash_node));
    }
    for &d in p.destructure.iter() { h = m(h, hash_node(d)); }
    h = m(h, u(opt_u(p.rest) | opt_u(p.kwrest) << 20 | opt_u(p.block) << 40));
    let fl = [p.no_kw, p.forwarding, p.has_trailing_comma, p.anon_rest, p.anon_kwrest, p.anon_block];
    m(h, u(fl.iter().enumerate().fold(0, |a, (j, &b)| a | (b as u64) << j)))
}
/// A call's argument description: kinds, keyword names (by name) and the block literal's body.
pub fn ci_hash(c: CiRef) -> u64 {
    use crate::astrust::{hash_bytes, hash_merge as m, hash_str, hash_u64 as u};
    let mut h = m(m(hash_str("ci"), hash_bytes(&c.kinds)), u(c.kw_splat_only as u64));
    for &k in c.kw_names.iter() { h = m(h, hash_bytes(crate::symbol::sym_bytes(k))); }
    m(h, c.blk.map_or(0, iseq_hash))
}

fn leak<T>(v: Vec<T>) -> &'static [T] { Box::leak(v.into_boxed_slice()) }
fn lstr(b: &[u8]) -> Str { Box::leak(b.to_vec().into_boxed_slice()) }

#[derive(Clone, Copy, Debug, PartialEq)]
enum VarLoc { Slot(u32), Env(u32) }

#[derive(Clone, Copy, PartialEq, Debug)]
enum SK { Method, Block, Lambda, Class, Top, Eval, For }

struct Scope {
    kind: SK,
    vars: HashMap<Sym, VarLoc>,
    names: Vec<(Sym, VarLoc)>,
    nslots: u32,
    env_size: u32,
    captured: HashSet<Sym>,
    loop_depth: u32,
    zsuper: Vec<(u8, Sym)>,       // method scopes: how to forward params for bare `super`
    fwd: bool,
    transparent: bool,            // `for` body: shares the enclosing scope's variables
    param_env: Vec<(Sym, u32)>,   // captured parameters: (name, setup slot)
    children: Vec<&'static Iseq>, // iseqs finished inside this scope (their parent is this scope's iseq)
    params_seen: HashSet<Sym>,    // `|_, _|`: only the first duplicate binds the variable
    param_names: Vec<(u32, Sym)>, // parameter slot -> name (for #parameters)
    needs_id: bool,               // a block literal or an eval-like call inside: its frame needs an id
}

impl Scope {
    fn new(kind: SK, captured: HashSet<Sym>) -> Scope {
        Scope { kind, vars: HashMap::new(), names: vec![], nslots: 2, env_size: 0, captured, loop_depth: 0,
                zsuper: vec![], fwd: false, transparent: kind == SK::For, param_env: vec![], children: vec![], params_seen: HashSet::new(), param_names: vec![], needs_id: false }
    }
    fn has_env(&self) -> bool { self.env_size > 0 }
}

pub struct Compiler<'a> {
    pr: &'a ParseResult,
    file: &'static str,
    scopes: Vec<Scope>,
    captures: HashMap<usize, HashSet<Sym>>,
    pub eval_outer: Vec<Vec<(Sym, u32, u32)>>,  // eval: outer scopes' (name, hops, idx), innermost first
    in_def: u32,
    pm_err: Option<u32>,          // single-pattern match: slot receiving the failure description
    tl_binding: bool,
    pub errors: Vec<String>,
}

fn key(n: &PNode) -> usize { n as *const PNode as usize }

pub const DEF_LOCALVAR: u32 = 0;
pub const DEF_IVAR: u32 = 1;
pub const DEF_GVAR: u32 = 2;
pub const DEF_CVAR: u32 = 3;
pub const DEF_CONST: u32 = 4;
pub const DEF_CPATH: u32 = 5;
pub const DEF_METHOD: u32 = 6;
pub const DEF_FCALL: u32 = 7;
pub const DEF_YIELD: u32 = 8;
pub const DEF_SUPER: u32 = 9;
pub const DEF_EXPR: u32 = 10;
pub const DEF_SELF: u32 = 11;
pub const DEF_NIL: u32 = 12;
pub const DEF_TRUE: u32 = 13;
pub const DEF_FALSE: u32 = 14;
pub const DEF_ASGN: u32 = 15;
pub const DEF_BACKREF: u32 = 16;

impl<'a> Compiler<'a> {
    pub fn new(pr: &'a ParseResult, file: &'static str) -> Self {
        Compiler { pr, file, scopes: vec![], captures: HashMap::new(), eval_outer: vec![], in_def: 0, pm_err: None, tl_binding: false, errors: vec![] }
    }

    fn cst(&self, c: u32) -> Sym { intern(&self.pr.constants[c as usize]) }
    fn line(&self, l: Loc) -> u32 { self.pr.line_of(l.start) }
    fn cur(&mut self) -> &mut Scope { self.scopes.last_mut().unwrap() }

    // ---- capture analysis -----------------------------------------------------------------

    /// Walk a hard scope's body; record variables that blocks access from outer scopes.
    fn analyze(&mut self, root: &PNode, body: &[&PNode]) {
        let mut stack: Vec<(usize, bool)> = vec![(key(root), false)];
        let mut uses_binding = false;
        for b in body { self.analyze_rec(b, &mut stack, &mut uses_binding); }
        if uses_binding {
            // every local of every scope in this tree becomes env-resident
            self.captures.entry(key(root)).or_default().insert(Sym(u32::MAX));
        }
    }

    fn analyze_rec(&mut self, n: &PNode, stack: &mut Vec<(usize, bool)>, ub: &mut bool) {
        let name_depth = match &n.kind {
            Kind::LocalVariableReadNode { name, depth } | Kind::LocalVariableTargetNode { name, depth } => Some((*name, *depth)),
            Kind::LocalVariableWriteNode { name, depth, .. } | Kind::LocalVariableOperatorWriteNode { name, depth, .. }
            | Kind::LocalVariableOrWriteNode { name, depth, .. } | Kind::LocalVariableAndWriteNode { name, depth, .. } => Some((*name, *depth)),
            _ => None,
        };
        if let Some((name, depth)) = name_depth {
            // owner = the depth-th non-transparent scope from the top
            let mut d = depth as i64;
            let mut i = stack.len() as i64 - 1;
            let mut crossed = false;
            while i >= 0 {
                if stack[i as usize].1 { crossed = true; i -= 1; continue; }
                if d == 0 { break; }
                d -= 1; i -= 1; crossed = true;
            }
            if i >= 0 && crossed {
                let k = stack[i as usize].0;
                let nm = self.cst(name);
                self.captures.entry(k).or_default().insert(nm);
            }
        }
        match &n.kind {
            // anonymous *, **, &, ... used inside a block: the method's hidden params are captured
            Kind::SplatNode { expression: None } | Kind::AssocSplatNode { value: None } | Kind::BlockArgumentNode { expression: None }
            | Kind::ForwardingArgumentsNode if stack.len() > 1 => {
                let e = self.captures.entry(stack[0].0).or_default();
                for nm in [b"__fwd_rest" as &[u8], b"__fwd_kw", b"__fwd_blk"] { e.insert(intern(nm)); }
            }
            // bare `super` in a block re-reads every parameter of the method
            Kind::ForwardingSuperNode { .. } if stack.len() > 1 => { self.captures.entry(stack[0].0).or_default().insert(Sym(u32::MAX)); }
            Kind::FlipFlopNode { .. } if stack.len() > 1 => {
                let nm = intern(format!("%ff{}", n.loc.start).as_bytes());
                self.captures.entry(stack[0].0).or_default().insert(nm);
            }
            Kind::BlockNode { .. } | Kind::LambdaNode { .. } => {
                stack.push((key(n), false));
                for ch in n.children() { self.analyze_rec(ch, stack, ub); }
                stack.pop();
                return;
            }
            Kind::ForNode { index, collection, statements } => {
                self.analyze_rec(collection, stack, ub);
                stack.push((key(n), true));
                self.analyze_rec(index, stack, ub);
                if let Some(st) = statements { self.analyze_rec(st, stack, ub); }
                stack.pop();
                return;
            }
            Kind::PostExecutionNode { statements } => {
                stack.push((key(n), true));
                if let Some(st) = statements { self.analyze_rec(st, stack, ub); }
                stack.pop();
                return;
            }
            Kind::DefNode { receiver, .. } => { if let Some(r) = receiver { self.analyze_rec(r, stack, ub); } return; }
            Kind::ClassNode { constant_path, superclass, .. } => {
                self.analyze_rec(constant_path, stack, ub);
                if let Some(s) = superclass { self.analyze_rec(s, stack, ub); }
                return;
            }
            Kind::ModuleNode { constant_path, .. } => { self.analyze_rec(constant_path, stack, ub); return; }
            Kind::SingletonClassNode { expression, .. } => { self.analyze_rec(expression, stack, ub); return; }
            // module_eval / class_eval / instance_eval with a String see the caller's locals
            Kind::CallNode { receiver: Some(_), name, arguments: Some(_), block: None, .. }
                if matches!(&self.pr.constants[*name as usize][..], b"module_eval" | b"class_eval" | b"instance_eval") => {
                *ub = true; self.mark_all_captured(stack);
            }
            Kind::CallNode { receiver: None, name, arguments, block, .. } => {
                let nm = &self.pr.constants[*name as usize];
                if matches!(&nm[..], b"module_eval" | b"class_eval" | b"instance_eval") && arguments.is_some() && block.is_none() { *ub = true; self.mark_all_captured(stack); }
                if (nm == b"binding" || nm == b"local_variables") && block.is_none() { *ub = true; self.mark_all_captured(stack); }
                if nm == b"eval" && arguments.is_some() { *ub = true; self.mark_all_captured(stack); }
            }
            _ => {}
        }
        for ch in n.children() { self.analyze_rec(ch, stack, ub); }
    }

    fn mark_all_captured(&mut self, stack: &[(usize, bool)]) {
        for (k, _) in stack { self.captures.entry(*k).or_default().insert(Sym(u32::MAX)); }
    }

    fn captured_for(&self, n: &PNode) -> HashSet<Sym> { self.captures.get(&key(n)).cloned().unwrap_or_default() }

    // ---- scopes and variables ------------------------------------------------------------

    fn push_scope(&mut self, kind: SK, n: &PNode, locals: &[u32]) {
        let cap = self.captured_for(n);
        let all = cap.contains(&Sym(u32::MAX));
        let mut sc = Scope::new(kind, cap);
        for &l in locals {
            let nm = self.cst(l);
            if sc.vars.contains_key(&nm) { continue; }
            // a frame has at most FRAME slots (stack.rs): the locals of a huge scope go to its env
            let spill = sc.nslots as usize >= crate::stack::FRAME / 2;
            let loc = if all || spill || sc.captured.contains(&nm) { sc.env_size += 1; VarLoc::Env(sc.env_size - 1) }
                      else { sc.nslots += 1; VarLoc::Slot(sc.nslots - 1) };
            sc.vars.insert(nm, loc);
            sc.names.push((nm, loc));
        }
        // flip-flop states used from blocks: env slots must exist before any block is compiled
        let ffs: Vec<Sym> = sc.captured.iter().copied().filter(|s| s.0 != u32::MAX && crate::symbol::sym_bytes(*s).starts_with(b"%ff")).collect();
        for nm in ffs { sc.env_size += 1; sc.vars.insert(nm, VarLoc::Env(sc.env_size - 1)); }
        self.scopes.push(sc);
    }

    fn hidden_slot(&mut self) -> u32 { let sc = self.cur(); sc.nslots += 1; sc.nslots - 1 }

    /// Declare a variable (e.g. an `it` param or a for-index) in the current scope if missing.
    fn declare(&mut self, nm: Sym) -> VarLoc {
        let sc = self.scopes.last_mut().unwrap();
        if let Some(&l) = sc.vars.get(&nm) { return l; }
        let all = sc.captured.contains(&Sym(u32::MAX));
        let loc = if all || sc.captured.contains(&nm) { sc.env_size += 1; VarLoc::Env(sc.env_size - 1) }
                  else { sc.nslots += 1; VarLoc::Slot(sc.nslots - 1) };
        sc.vars.insert(nm, loc);
        sc.names.push((nm, loc));
        loc
    }

    /// Resolve a variable at prism depth → (hops, loc).
    fn resolve(&mut self, nm: Sym, depth: u32) -> (u32, VarLoc) {
        let mut d = depth as i64;
        let mut i = self.scopes.len() as i64 - 1;
        let mut hops = 0;
        // count env-owning scopes strictly above the owner
        loop {
            if i < 0 { break; }
            let sc = &self.scopes[i as usize];
            if sc.transparent { if sc.has_env() { hops += 1; } i -= 1; continue; }
            if d == 0 { break; }
            if sc.has_env() { hops += 1; }
            d -= 1; i -= 1;
        }
        if i < 0 {
            // eval: an outer (caller) scope variable
            for (lvl, outer) in self.eval_outer.iter().enumerate() {
                let _ = lvl;
                if let Some(&(_, h, idx)) = outer.iter().find(|x| x.0 == nm) { return (h + hops, VarLoc::Env(idx)); }
            }
            // not found: declare in the outermost scope
            let l = self.declare(nm);
            return (0, l);
        }
        let sc = &mut self.scopes[i as usize];
        let loc = match sc.vars.get(&nm) {
            Some(&l) => l,
            None => {
                // a variable prism knows but the scope's locals list does not (e.g. eval scopes)
                if depth > 0 || sc.captured.contains(&nm) || sc.captured.contains(&Sym(u32::MAX)) {
                    sc.env_size += 1; let l = VarLoc::Env(sc.env_size - 1); sc.vars.insert(nm, l); sc.names.push((nm, l)); l
                } else {
                    // eval: look outward
                    if i == 0 && !self.eval_outer.is_empty() {
                        for outer in self.eval_outer.iter() {
                            if let Some(&(_, h, idx)) = outer.iter().find(|x| x.0 == nm) {
                                let own = if self.scopes[0].has_env() { 1 } else { 0 };
                                return (h + hops + own, VarLoc::Env(idx));
                            }
                        }
                    }
                    let sc = &mut self.scopes[i as usize];
                    sc.nslots += 1; let l = VarLoc::Slot(sc.nslots - 1); sc.vars.insert(nm, l); sc.names.push((nm, l)); l
                }
            }
        };
        (hops, loc)
    }

    fn var_read(&mut self, nm: Sym, depth: u32) -> NodePtr {
        match self.resolve(nm, depth) {
            (0, VarLoc::Slot(i)) => ALLOC_node_lvar(i),
            (h, VarLoc::Env(i)) => ALLOC_node_dvar(h, i),
            (_, VarLoc::Slot(i)) => ALLOC_node_lvar(i),
        }
    }
    fn var_write(&mut self, nm: Sym, depth: u32, v: NodePtr) -> NodePtr {
        match self.resolve(nm, depth) {
            (_, VarLoc::Slot(i)) => ALLOC_node_lasgn(i, v),
            (h, VarLoc::Env(i)) => ALLOC_node_dasgn(h, i, v),
        }
    }

    // ---- entry points ------------------------------------------------------------------------

    pub fn compile_program(&mut self, root: &PNode, kind: IseqKind) -> &'static Iseq {
        let (locals, stmts) = match &root.kind { Kind::ProgramNode { locals, statements } => (locals.clone(), statements), _ => panic!() };
        self.analyze(root, &[stmts]);
        let sk = if kind == IseqKind::Eval { SK::Eval } else { SK::Top };
        self.push_scope(sk, root, &locals);
        let mut body = self.c(stmts);
        if self.tl_binding {
            // the main script mentions TOPLEVEL_BINDING: bind it before anything runs
            let b = self.fcall("binding", vec![], 1);
            body = self.seq(vec![ALLOC_node_cdecl(intern(b"TOPLEVEL_BINDING"), b), body]);
        }
        let label = if kind == IseqKind::Eval { "<main>".to_string() } else { "<main>".to_string() };
        self.finish_iseq(body, Params::default(), kind, label, 1, Sym(0))
    }

    fn finish_iseq(&mut self, body: NodePtr, mut params: Params, kind: IseqKind, label: String, line: u32, name: Sym) -> &'static Iseq {
        let mut sc = self.scopes.pop().unwrap();
        // evaluators reach their frame through a FRAME-slot window (stack.rs)
        if sc.nslots as usize > crate::stack::FRAME {
            self.errors.push(format!("{}:{}: {} needs too many local slots ({}, max {})", self.file, line, label, sc.nslots, crate::stack::FRAME));
        }
        params.slot_names = std::mem::take(&mut sc.param_names);
        let mut env_params = vec![];
        // a captured parameter has both a slot (filled by setup) and an env index
        for &(nm, sl) in sc.param_env.iter() {
            if let Some(VarLoc::Env(ei)) = sc.vars.get(&nm) { env_params.push((sl, *ei)); }
        }
        let simple = params.opt.is_empty() && params.rest.is_none() && params.post.is_empty() && params.kw.is_empty()
            && params.kwrest.is_none() && params.block.is_none() && params.destructure.is_empty() && !params.no_kw
            && sc.env_size == 0 && params.lead.iter().enumerate().all(|(i, &s)| s == 2 + i as u32);
        let lead_only = params.opt.is_empty() && params.rest.is_none() && params.post.is_empty() && params.kw.is_empty()
            && params.kwrest.is_none() && params.block.is_none() && params.destructure.is_empty() && !params.has_trailing_comma
            && params.lead.iter().enumerate().all(|(i, &s)| s == 2 + i as u32);
        let arity = {
            let req = (params.lead.len() + params.post.len()) as i32;
            let req_kw = params.kw.iter().any(|k| k.required);
            if !params.opt.is_empty() || params.rest.is_some() || (!params.kw.is_empty() && !req_kw && false) { -(req + 1) - if req_kw { 1 } else { 0 } + if req_kw { 1 } else { 0 } }
            else if req_kw { req + 1 } else { req }
        };
        let local_names = sc.names.iter().map(|(n, l)| match l { VarLoc::Slot(i) => (*n, *i as i32, -1), VarLoc::Env(i) => (*n, -1, *i as i32) }).collect();
        let entry = if kind == IseqKind::Method && lead_only {
            let mut fl = 0;
            if sc.env_size > 0 { fl |= ME_ENV; }
            if sc.needs_id || sc.captured.contains(&Sym(u32::MAX)) { fl |= ME_ID; }
            if fl == 0 && sc.nslots as usize == 2 + params.lead.len() && leaf_body(body) { fl |= ME_LEAF; }
            ALLOC_node_method_entry(sc.nslots, params.lead.len() as u32, fl, line, body)
        } else { std::ptr::null_mut() };
        let bentry = if (kind == IseqKind::Block || kind == IseqKind::Lambda) && lead_only {
            let fl = if sc.env_size > 0 { ME_ENV } else { 0 } | if kind == IseqKind::Lambda { ME_LAMBDA } else { 0 };
            ALLOC_node_block_entry(sc.nslots, params.lead.len() as u32, fl, line, body)
        } else { std::ptr::null_mut() };
        // dense keyword layout: lead then keywords in declared order, nothing else.  kentry takes
        // them all as plain params (a site passing every keyword in order); dentry takes none.
        let dense = kind == IseqKind::Method && !params.kw.is_empty() && params.opt.is_empty() && params.rest.is_none()
            && params.post.is_empty() && params.kwrest.is_none() && params.block.is_none() && params.destructure.is_empty()
            && !params.no_kw && !params.forwarding && !params.has_trailing_comma
            && params.lead.iter().chain(params.kw.iter().map(|k| &k.slot)).enumerate().all(|(i, &s)| s == 2 + i as u32);
        let me_flags = |np: usize, b: NodePtr| {
            let mut fl = 0;
            if sc.env_size > 0 { fl |= ME_ENV; }
            if sc.needs_id || sc.captured.contains(&Sym(u32::MAX)) { fl |= ME_ID; }
            if fl == 0 && sc.nslots as usize == 2 + np && leaf_body(b) { fl |= ME_LEAF; }
            fl
        };
        let kentry = if dense {
            let np = params.lead.len() + params.kw.len();
            ALLOC_node_method_entry(sc.nslots, np as u32, me_flags(np, body), line, body)
        } else { std::ptr::null_mut() };
        // dentry runs the defaults in its own copy of the body: not where a literal's identity is cached per node
        let dentry = if dense && params.kw.iter().all(|k| !k.required) {
            // static defaults first, as in setup_params
            let mut v: Vec<NodePtr> = params.kw.iter().filter(|k| k.static_default).chain(params.kw.iter().filter(|k| !k.static_default))
                .map(|k| k.default.unwrap()).collect();
            v.push(body);
            let b = self.seq(v);
            if has_lit_cache(b) { std::ptr::null_mut() }
            else { ALLOC_node_method_entry(sc.nslots, params.lead.len() as u32, me_flags(params.lead.len(), b), line, b) }
        } else { std::ptr::null_mut() };
        let dentry = if dentry.is_null() { dentry } else { layout_tree(dentry) };
        // one block per body: baked code addresses its nodes (and their caches) as root + constant
        let (body, entry, bentry, kentry) = if !entry.is_null() {
            let e = layout_tree(entry);
            (unsafe { (*(e as *mut NodeOf<node_method_entry_struct>)).u.body }, e, bentry, kentry)
        } else if !kentry.is_null() {
            let e = layout_tree(kentry);
            (unsafe { (*(e as *mut NodeOf<node_method_entry_struct>)).u.body }, entry, bentry, e)
        } else if !bentry.is_null() {
            let e = layout_tree(bentry);
            (unsafe { (*(e as *mut NodeOf<node_block_entry_struct>)).u.body }, entry, e, kentry)
        } else { (layout_tree(body), entry, bentry, kentry) };
        let is = Box::leak(Box::new(Iseq {
            body, entry, bentry, kentry, dentry, body_used: std::cell::Cell::new(false), nlocals: sc.nslots, params, env_size: sc.env_size, env_params,
            info: CodeInfo { file: self.file, label, line }, kind, simple, lead_only, name, local_names, arity, uses_block: false,
            parent: std::cell::Cell::new(std::ptr::null()), ruby2_keywords: std::cell::Cell::new(false),
            outer_names: if kind == IseqKind::Eval { self.eval_outer.iter().flatten().cloned().collect() } else { vec![] },
        }));
        for ch in &sc.children { ch.parent.set(is); }
        if let Some(p) = self.scopes.last_mut() { p.children.push(is); }
        crate::aot::register_iseq(is);
        is
    }

    // ---- statements & expressions ----------------------------------------------------------

    fn stmts(&mut self, n: Option<&PNode>) -> NodePtr {
        match n { Some(n) => self.c(n), None => ALLOC_node_nil() }
    }

    fn seq(&mut self, v: Vec<NodePtr>) -> NodePtr {
        match v.len() { 0 => ALLOC_node_nil(), 1 => v[0], _ => ALLOC_node_block(leak(v)) }
    }

    fn unsupported(&mut self, what: &str, l: Loc) -> NodePtr {
        let msg = format!("{}:{}: {} is not supported", self.file, self.line(l), what);
        ALLOC_node_unsupported(lstr(msg.as_bytes()))
    }

    fn dconst(&self, kind: u32) -> NodePtr { ALLOC_node_defined(kind, Sym(0), ALLOC_node_nil()) }

    pub fn c(&mut self, n: &PNode) -> NodePtr {
        let line = self.line(n.loc);
        match &n.kind {
            Kind::StatementsNode { body } => {
                let v: Vec<NodePtr> = body.iter().map(|x| { let r = self.c(x); if needs_line(x) { ALLOC_node_line(self.line(x.loc), r) } else { r } }).collect();
                self.seq(v)
            }
            Kind::ParenthesesNode { body } => self.stmts(body.as_deref()),
            Kind::BeginNode { statements, rescue_clause, else_clause, ensure_clause } => {
                let mut body = self.stmts(statements.as_deref());
                if let Some(r) = rescue_clause {
                    let clauses = self.rescue_clauses(r);
                    let (eb, has_else) = match else_clause { Some(e) => (self.else_body(e), true), None => (ALLOC_node_nil(), false) };
                    body = ALLOC_node_rescue(body, leak(clauses), eb, has_else);
                } else if let Some(e) = else_clause {
                    let eb = self.else_body(e);
                    body = self.seq(vec![body, eb]);
                }
                if let Some(e) = ensure_clause {
                    let en = match &e.kind { Kind::EnsureNode { statements } => self.stmts(statements.as_deref()), _ => ALLOC_node_nil() };
                    body = ALLOC_node_ensure(body, en);
                }
                body
            }
            Kind::NilNode => ALLOC_node_nil(),
            Kind::TrueNode => ALLOC_node_true(),
            Kind::FalseNode => ALLOC_node_false(),
            Kind::SelfNode => ALLOC_node_self(),
            Kind::IntegerNode { value } => self.integer(value),
            Kind::FloatNode { value } => {
                match Value::flonum(*value) { Some(v) => ALLOC_node_lit(v), None => ALLOC_node_float(*value) }
            }
            Kind::RationalNode { numerator, denominator } => {
                let a = self.integer(numerator); let b = self.integer(denominator);
                self.fcall("Rational", vec![a, b], line)
            }
            Kind::ImaginaryNode { numeric } => {
                let z = ALLOC_node_lit(Value::fix(0));
                let v = self.c(numeric);
                self.fcall("Complex", vec![z, v], line)
            }
            Kind::StringNode { unescaped } => {
                let enc = if n.flags & pa::STRING_FLAGS_FORCED_BINARY_ENCODING != 0 { crate::object::ENC_BINARY as u32 } else { 0 };
                if n.flags & pa::STRING_FLAGS_FROZEN != 0 { ALLOC_node_fstr(lstr(unescaped), enc) } else { ALLOC_node_str(lstr(unescaped), enc) }
            }
            Kind::XStringNode { unescaped } => {
                let s = ALLOC_node_str(lstr(unescaped), 0);
                self.fcall("`", vec![s], line)
            }
            Kind::InterpolatedXStringNode { parts } => {
                let s = self.dstr(parts);
                self.fcall("`", vec![s], line)
            }
            Kind::SymbolNode { unescaped } => ALLOC_node_lit(Value::sym(intern(unescaped))),
            Kind::InterpolatedStringNode { parts } => {
                let d = self.dstr(parts);
                if n.flags & pa::INTERPOLATED_STRING_NODE_FLAGS_FROZEN != 0 { self.call_node(d, s::freeze, vec![], 0, line) } else { d }
            }
            Kind::InterpolatedSymbolNode { parts } => { let d = self.dstr(parts); ALLOC_node_dsym(d) }
            Kind::RegularExpressionNode { unescaped } => ALLOC_node_regex(lstr(unescaped), regex_opts(n.flags)),
            Kind::InterpolatedRegularExpressionNode { parts } => {
                let d = self.dstr(parts);
                ALLOC_node_dregex(d, regex_opts(n.flags), n.flags & pa::REGULAR_EXPRESSION_FLAGS_ONCE != 0)
            }
            Kind::MatchLastLineNode { unescaped } => {
                let re = ALLOC_node_regex(lstr(unescaped), regex_opts(n.flags));
                let ll = ALLOC_node_gvar(intern(b"$_"));
                self.call_node(re, s::match_op, vec![ll], 0, line)
            }
            Kind::ArrayNode { elements } => self.array(elements),
            Kind::HashNode { elements } => self.hash(elements),
            Kind::KeywordHashNode { elements } => self.hash(elements),
            Kind::RangeNode { left, right } => {
                let excl = n.flags & pa::RANGE_FLAGS_EXCLUDE_END != 0;
                let lit = |x: &Option<Box<PNode>>| -> Option<Value> {
                    match x.as_deref().map(|y| &y.kind) {
                        Some(Kind::IntegerNode { value }) if value.words.len() == 1 && (value.words[0] as i64) < (1i64 << 61) => {
                            let v = value.words[0] as i64; Some(Value::fix(if value.neg { -v } else { v }))
                        }
                        Some(Kind::NilNode) | None => Some(NIL),
                        _ => None,
                    }
                };
                if let (Some(a), Some(b)) = (lit(left), lit(right)) { if !(a == NIL && b == NIL) || true { return ALLOC_node_range_lit(a, b, excl); } }
                let a = match left { Some(x) => self.c(x), None => ALLOC_node_nil() };
                let b = match right { Some(x) => self.c(x), None => ALLOC_node_nil() };
                ALLOC_node_range(a, b, excl)
            }
            Kind::SourceFileNode { filepath } => ALLOC_node_str(lstr(filepath), 0),
            Kind::SourceLineNode => ALLOC_node_lit(Value::fix(line as i64)),
            Kind::SourceEncodingNode => {
                let e = ALLOC_node_top_const(intern(b"Encoding"));
                ALLOC_node_const_path(e, intern(b"UTF_8"), line)
            }
            // variables
            Kind::LocalVariableReadNode { name, depth } => { let nm = self.cst(*name); self.var_read(nm, *depth) }
            Kind::ItLocalVariableReadNode => { let nm = intern(b"__it"); self.var_read(nm, 0) }
            Kind::LocalVariableWriteNode { name, depth, value } => {
                let v = self.c(value); let nm = self.cst(*name); self.var_write(nm, *depth, v)
            }
            Kind::LocalVariableOperatorWriteNode { name, depth, value, binary_operator } => {
                let nm = self.cst(*name);
                let r = self.var_read(nm, *depth); let v = self.c(value);
                let op = self.cst(*binary_operator);
                let e = self.binop(r, op, v, line);
                self.var_write(nm, *depth, e)
            }
            Kind::LocalVariableOrWriteNode { name, depth, value } => {
                let nm = self.cst(*name);
                let r = self.var_read(nm, *depth); let v = self.c(value);
                let w = self.var_write(nm, *depth, v);
                ALLOC_node_or(r, w)
            }
            Kind::LocalVariableAndWriteNode { name, depth, value } => {
                let nm = self.cst(*name);
                let r = self.var_read(nm, *depth); let v = self.c(value);
                let w = self.var_write(nm, *depth, v);
                ALLOC_node_and(r, w)
            }
            Kind::InstanceVariableReadNode { name } => ALLOC_node_ivar(self.cst(*name)),
            Kind::InstanceVariableWriteNode { name, value } => { let v = self.c(value); ALLOC_node_iasgn(self.cst(*name), v) }
            Kind::InstanceVariableOperatorWriteNode { name, value, binary_operator } => {
                let nm = self.cst(*name);
                let op = self.cst(*binary_operator);
                // the fused node evaluates the right side before it reads the ivar: only for one that cannot change it
                let pure = matches!(value.kind, Kind::IntegerNode { .. } | Kind::FloatNode { .. } | Kind::LocalVariableReadNode { .. } | Kind::InstanceVariableReadNode { .. });
                if pure && (op == s::plus || op == s::minus) {
                    let v = self.c(value);
                    ALLOC_node_ivar_arith(nm, op == s::minus, v, line)
                } else {
                    let r = ALLOC_node_ivar(nm); let v = self.c(value);
                    let e = self.binop(r, op, v, line);
                    ALLOC_node_iasgn(nm, e)
                }
            }
            Kind::InstanceVariableOrWriteNode { name, value } => {
                let nm = self.cst(*name); let v = self.c(value);
                ALLOC_node_or(ALLOC_node_ivar(nm), ALLOC_node_iasgn(nm, v))
            }
            Kind::InstanceVariableAndWriteNode { name, value } => {
                let nm = self.cst(*name); let v = self.c(value);
                ALLOC_node_and(ALLOC_node_ivar(nm), ALLOC_node_iasgn(nm, v))
            }
            Kind::GlobalVariableReadNode { name } => ALLOC_node_gvar(self.cst(*name)),
            Kind::GlobalVariableWriteNode { name, value } => { let v = self.c(value); ALLOC_node_gasgn(self.cst(*name), v) }
            Kind::GlobalVariableOperatorWriteNode { name, value, binary_operator } => {
                let nm = self.cst(*name);
                let r = ALLOC_node_gvar(nm); let v = self.c(value);
                let e = self.binop(r, self.cst(*binary_operator), v, line);
                ALLOC_node_gasgn(nm, e)
            }
            Kind::GlobalVariableOrWriteNode { name, value } => {
                let nm = self.cst(*name); let v = self.c(value);
                ALLOC_node_or(ALLOC_node_gvar(nm), ALLOC_node_gasgn(nm, v))
            }
            Kind::GlobalVariableAndWriteNode { name, value } => {
                let nm = self.cst(*name); let v = self.c(value);
                ALLOC_node_and(ALLOC_node_gvar(nm), ALLOC_node_gasgn(nm, v))
            }
            Kind::ClassVariableReadNode { name } => ALLOC_node_cvar(self.cst(*name)),
            Kind::ClassVariableWriteNode { name, value } => { let v = self.c(value); ALLOC_node_cvasgn(self.cst(*name), v) }
            Kind::ClassVariableOperatorWriteNode { name, value, binary_operator } => {
                let nm = self.cst(*name);
                let r = ALLOC_node_cvar(nm); let v = self.c(value);
                let e = self.binop(r, self.cst(*binary_operator), v, line);
                ALLOC_node_cvasgn(nm, e)
            }
            Kind::ClassVariableOrWriteNode { name, value } => {
                let nm = self.cst(*name); let v = self.c(value);
                let d = ALLOC_node_defined(DEF_CVAR, nm, ALLOC_node_nil());
                let cond = ALLOC_node_and(d, ALLOC_node_cvar(nm));
                ALLOC_node_or(cond, ALLOC_node_cvasgn(nm, v))
            }
            Kind::ClassVariableAndWriteNode { name, value } => {
                let nm = self.cst(*name); let v = self.c(value);
                ALLOC_node_and(ALLOC_node_cvar(nm), ALLOC_node_cvasgn(nm, v))
            }
            Kind::ConstantReadNode { name } => ALLOC_node_const(self.cst(*name), line),
            Kind::ConstantWriteNode { name, value } => { let v = self.c(value); ALLOC_node_cdecl(self.cst(*name), v) }
            Kind::ConstantOrWriteNode { name, value } => {
                let nm = self.cst(*name); let v = self.c(value);
                let d = ALLOC_node_defined(DEF_CONST, nm, ALLOC_node_nil());
                ALLOC_node_if(d, ALLOC_node_const(nm, line), ALLOC_node_cdecl(nm, v))
            }
            Kind::ConstantAndWriteNode { name, value } => {
                let nm = self.cst(*name); let v = self.c(value);
                ALLOC_node_and(ALLOC_node_const(nm, line), ALLOC_node_cdecl(nm, v))
            }
            Kind::ConstantOperatorWriteNode { name, value, binary_operator } => {
                let nm = self.cst(*name); let v = self.c(value);
                let e = self.binop(ALLOC_node_const(nm, line), self.cst(*binary_operator), v, line);
                ALLOC_node_cdecl(nm, e)
            }
            Kind::ConstantPathNode { parent, name } => {
                let nm = self.cst(name.unwrap_or(0));
                match parent { None => ALLOC_node_top_const(nm), Some(p) => { let pv = self.c(p); ALLOC_node_const_path(pv, nm, line) } }
            }
            Kind::ConstantPathWriteNode { target, value } => {
                let v = self.c(value);
                self.cpath_write(target, v)
            }
            Kind::ConstantPathOrWriteNode { target, value } => {
                let v = self.c(value);
                let rd = self.c(target);
                let w = self.cpath_write(target, v);
                let (p, nm) = self.cpath_parts(target);
                let d = ALLOC_node_defined(DEF_CPATH, nm, p);
                ALLOC_node_if(d, rd, w)
            }
            Kind::ConstantPathAndWriteNode { target, value } => {
                let v = self.c(value); let rd = self.c(target); let w = self.cpath_write(target, v);
                ALLOC_node_and(rd, w)
            }
            Kind::ConstantPathOperatorWriteNode { target, value, binary_operator } => {
                let v = self.c(value); let rd = self.c(target);
                let e = self.binop(rd, self.cst(*binary_operator), v, line);
                self.cpath_write(target, e)
            }
            Kind::ShareableConstantNode { write } => self.c(write),
            Kind::BackReferenceReadNode { name } => {
                let nm = &self.pr.constants[*name as usize];
                let k = match nm.as_slice() { b"$&" => 0, b"$`" => 1, b"$'" => 2, b"$+" => 3, _ => 0 };
                ALLOC_node_back_ref(k)
            }
            Kind::NumberedReferenceReadNode { number } => ALLOC_node_nth_ref(*number),
            // control flow
            Kind::IfNode { predicate, statements, subsequent, .. } => {
                let cnd = self.cond(predicate);
                let t = self.stmts(statements.as_deref());
                let e = match subsequent { Some(x) => self.else_body(x), None => ALLOC_node_nil() };
                ALLOC_node_if(cnd, t, e)
            }
            Kind::UnlessNode { predicate, statements, else_clause } => {
                let cnd = self.cond(predicate);
                let t = self.stmts(statements.as_deref());
                let e = match else_clause { Some(x) => self.else_body(x), None => ALLOC_node_nil() };
                ALLOC_node_if(cnd, e, t)
            }
            Kind::ElseNode { statements } => self.stmts(statements.as_deref()),
            Kind::AndNode { left, right } => { let a = self.c(left); let b = self.c(right); ALLOC_node_and(a, b) }
            Kind::OrNode { left, right } => { let a = self.c(left); let b = self.c(right); ALLOC_node_or(a, b) }
            Kind::WhileNode { predicate, statements } | Kind::UntilNode { predicate, statements } => {
                let until = matches!(n.kind, Kind::UntilNode { .. });
                let cnd = self.cond(predicate);
                self.cur().loop_depth += 1;
                let body = self.stmts(statements.as_deref());
                self.cur().loop_depth -= 1;
                if n.flags & pa::LOOP_FLAGS_BEGIN_MODIFIER != 0 { ALLOC_node_dowhile(cnd, body, until) } else { ALLOC_node_while(cnd, body, until) }
            }
            Kind::ForNode { index, collection, statements } => self.for_loop(n, index, collection, statements.as_deref(), line),
            Kind::BreakNode { arguments } => {
                let v = self.args_value(arguments.as_deref());
                if self.cur().loop_depth > 0 { ALLOC_node_loop_break(v) } else { ALLOC_node_break(v) }
            }
            Kind::NextNode { arguments } => {
                let v = self.args_value(arguments.as_deref());
                if self.cur().loop_depth > 0 { ALLOC_node_loop_next(v) } else { ALLOC_node_next(v) }
            }
            Kind::RedoNode => ALLOC_node_redo(),
            Kind::RetryNode => ALLOC_node_retry(),
            Kind::ReturnNode { arguments } => { let v = self.args_value(arguments.as_deref()); ALLOC_node_return(v) }
            Kind::CaseNode { predicate, conditions, else_clause } => self.case_when(predicate.as_deref(), conditions, else_clause.as_deref(), line),
            Kind::CaseMatchNode { predicate, conditions, else_clause } => self.case_in(predicate.as_deref(), conditions, else_clause.as_deref(), line),
            Kind::MatchPredicateNode { value, pattern } => {
                let sl = self.hidden_slot();
                let v = self.c(value);
                let set = ALLOC_node_lasgn(sl, v);
                let p = self.pattern(pattern, sl);
                let t = self.seq(vec![set, p]);
                ALLOC_node_if(t, ALLOC_node_true(), ALLOC_node_false())
            }
            Kind::MatchRequiredNode { value, pattern } => {
                let sl = self.hidden_slot();
                let v = self.c(value);
                let set = ALLOC_node_lasgn(sl, v);
                let (p, fail) = self.single_pattern(pattern, sl);
                let t = ALLOC_node_if(p, ALLOC_node_nil(), fail);
                self.seq(vec![set, t])
            }
            Kind::RescueModifierNode { expression, rescue_expression } => {
                let body = self.c(expression);
                let r = self.c(rescue_expression);
                let rb = ALLOC_node_resbody(leak(vec![]), lstr(&[]), ALLOC_node_nil(), r);
                ALLOC_node_rescue(body, leak(vec![rb]), ALLOC_node_nil(), false)
            }
            Kind::DefinedNode { value } => self.defined(value),
            Kind::FlipFlopNode { left, right } => {
                // state lives in the enclosing hard scope so it survives block invocations
                let nm = intern(format!("%ff{}", n.loc.start).as_bytes());
                let depth = self.scopes.iter().rev().filter(|s| !s.transparent).take_while(|s| matches!(s.kind, SK::Block | SK::Lambda)).count() as u32;
                if depth == 0 && !self.cur().vars.contains_key(&nm) { let sl = self.hidden_slot(); self.cur().vars.insert(nm, VarLoc::Slot(sl)); }
                let excl = n.flags & pa::RANGE_FLAGS_EXCLUDE_END != 0;
                let a = match left { Some(x) => self.cond(x), None => ALLOC_node_nil() };
                let b1 = match right { Some(x) => self.cond(x), None => ALLOC_node_nil() };
                let off = self.var_write(nm, depth, ALLOC_node_false());
                let on_state = self.seq(vec![ALLOC_node_if(b1, off, ALLOC_node_nil()), ALLOC_node_true()]);
                let on = self.var_write(nm, depth, ALLOC_node_true());
                let start = if excl { on } else {
                    let b2 = match right { Some(x) => self.cond(x), None => ALLOC_node_nil() };
                    ALLOC_node_if(b2, ALLOC_node_nil(), on)
                };
                let off_state = ALLOC_node_if(a, self.seq(vec![start, ALLOC_node_true()]), ALLOC_node_false());
                let st = self.var_read(nm, depth);
                ALLOC_node_if(st, on_state, off_state)
            }
            // calls
            Kind::CallNode { receiver, name, arguments, block, .. } => {
                let nm = self.cst(*name);
                self.call(n, receiver.as_deref(), nm, arguments.as_deref(), block.as_deref(), line)
            }
            Kind::SuperNode { arguments, block } => self.super_call(arguments.as_deref(), block.as_deref(), false, line),
            Kind::ForwardingSuperNode { block } => self.super_call(None, block.as_deref(), true, line),
            Kind::YieldNode { arguments } => {
                let args: Vec<&PNode> = match arguments.as_deref() { Some(PNode { kind: Kind::ArgumentsNode { arguments }, .. }) => arguments.iter().collect(), _ => vec![] };
                if args.iter().all(|a| self.simple_arg(a)) {
                    let v: Vec<NodePtr> = args.iter().map(|a| self.c(a)).collect();
                    ALLOC_node_line(line, ALLOC_node_yield(leak(v)))
                } else {
                    let (v, kinds) = self.complex_args(&args);
                    let ci = Box::leak(Box::new(CallInfo { kinds, blk: None, kw_splat_only: false, kw_names: vec![] }));
                    ALLOC_node_line(line, ALLOC_node_yield_x(leak(v), ci))
                }
            }
            Kind::CallOperatorWriteNode { receiver, read_name, write_name, binary_operator, value } => {
                let sl = self.hidden_slot();
                let r = match receiver { Some(r) => self.c(r), None => ALLOC_node_self() };
                let set = ALLOC_node_lasgn(sl, r);
                let flags = if receiver.is_none() || matches!(receiver.as_deref().map(|x| &x.kind), Some(Kind::SelfNode)) { CF_FCALL } else { 0 };
                let rd = self.call_node(ALLOC_node_lvar(sl), self.cst(*read_name), vec![], flags, line);
                let v = self.c(value);
                let e = self.binop(rd, self.cst(*binary_operator), v, line);
                let w = ALLOC_node_attrasgn(ALLOC_node_lvar(sl), leak(vec![e]), self.cst(*write_name), flags, line);
                self.safe_nav(n, sl, set, w)
            }
            Kind::CallOrWriteNode { receiver, read_name, write_name, value } | Kind::CallAndWriteNode { receiver, read_name, write_name, value } => {
                let is_or = matches!(n.kind, Kind::CallOrWriteNode { .. });
                let sl = self.hidden_slot();
                let r = match receiver { Some(r) => self.c(r), None => ALLOC_node_self() };
                let set = ALLOC_node_lasgn(sl, r);
                let flags = if receiver.is_none() || matches!(receiver.as_deref().map(|x| &x.kind), Some(Kind::SelfNode)) { CF_FCALL } else { 0 };
                let rd = self.call_node(ALLOC_node_lvar(sl), self.cst(*read_name), vec![], flags, line);
                let v = self.c(value);
                let w = ALLOC_node_attrasgn(ALLOC_node_lvar(sl), leak(vec![v]), self.cst(*write_name), flags, line);
                let e = if is_or { ALLOC_node_or(rd, w) } else { ALLOC_node_and(rd, w) };
                self.safe_nav(n, sl, set, e)
            }
            Kind::IndexOperatorWriteNode { receiver, arguments, binary_operator, value, .. } => {
                let (pre, rd, mk) = self.index_parts(receiver.as_deref(), arguments.as_deref(), line);
                let v = self.c(value);
                let e = self.binop(rd, self.cst(*binary_operator), v, line);
                let w = mk(self, e);
                let mut p = pre; p.push(w);
                self.seq(p)
            }
            Kind::IndexOrWriteNode { receiver, arguments, value, .. } | Kind::IndexAndWriteNode { receiver, arguments, value, .. } => {
                let is_or = matches!(n.kind, Kind::IndexOrWriteNode { .. });
                let (pre, rd, mk) = self.index_parts(receiver.as_deref(), arguments.as_deref(), line);
                let v = self.c(value);
                let w = mk(self, v);
                let e = if is_or { ALLOC_node_or(rd, w) } else { ALLOC_node_and(rd, w) };
                let mut p = pre; p.push(e);
                self.seq(p)
            }
            Kind::MatchWriteNode { call, targets } => {
                let sl = self.hidden_slot();
                let cv = self.c(call);
                let mut v = vec![ALLOC_node_lasgn(sl, cv)];
                for t in targets {
                    if let Kind::LocalVariableTargetNode { name, depth } = &t.kind {
                        let nm = self.cst(*name);
                        let get = ALLOC_node_special(SP_NAMED_CAPTURE, ALLOC_node_lit(Value::sym(nm)));
                        v.push(self.var_write(nm, *depth, get));
                    }
                }
                v.push(ALLOC_node_lvar(sl));
                self.seq(v)
            }
            Kind::MultiWriteNode { lefts, rest, rights, value } => {
                let v = self.c(value);
                let splat_rhs = matches!(value.kind, Kind::ArrayNode { .. });
                let m = self.masgn(lefts, rest.as_deref(), rights, v, splat_rhs);
                ALLOC_node_line(line, m)
            }
            // definitions
            Kind::DefNode { name, receiver, parameters, body, locals } => {
                let nm = self.cst(*name);
                let is = self.compile_def(n, nm, parameters.as_deref(), body.as_deref(), locals, line);
                let d = match receiver {
                    Some(r) => { let rv = self.c(r); ALLOC_node_sdef(rv, nm, is) }
                    None => ALLOC_node_def(nm, is),
                };
                ALLOC_node_line(line, d)
            }
            Kind::ClassNode { locals, constant_path, superclass, body, name } => {
                let nm = self.cst(*name);
                let (cp, has_cp) = self.cpath_parent(constant_path);
                let (sup, has_sup) = match superclass { Some(s) => (self.c(s), true), None => (ALLOC_node_nil(), false) };
                let label = format!("<class:{}>", crate::symbol::sym_str(nm));
                let is = self.compile_body(n, locals, body.as_deref(), IseqKind::Class, label, line);
                ALLOC_node_line(line, ALLOC_node_class(cp, nm, sup, is, has_cp, has_sup))
            }
            Kind::ModuleNode { locals, constant_path, body, name } => {
                let nm = self.cst(*name);
                let (cp, has_cp) = self.cpath_parent(constant_path);
                let label = format!("<module:{}>", crate::symbol::sym_str(nm));
                let is = self.compile_body(n, locals, body.as_deref(), IseqKind::Class, label, line);
                ALLOC_node_line(line, ALLOC_node_module(cp, nm, is, has_cp))
            }
            Kind::SingletonClassNode { locals, expression, body } => {
                let e = self.c(expression);
                let is = self.compile_body(n, locals, body.as_deref(), IseqKind::Class, "singleton class".into(), line);
                ALLOC_node_line(line, ALLOC_node_sclass(e, is))
            }
            Kind::AliasMethodNode { new_name, old_name } => { let a = self.c(new_name); let b = self.c(old_name); ALLOC_node_line(line, ALLOC_node_alias(a, b)) }
            Kind::AliasGlobalVariableNode { new_name, old_name } => {
                let g = |s: &Self, x: &PNode| match &x.kind {
                    Kind::GlobalVariableReadNode { name } => s.cst(*name),
                    Kind::BackReferenceReadNode { name } => s.cst(*name),
                    _ => intern(b"$_"),
                };
                ALLOC_node_valias(g(self, new_name), g(self, old_name))
            }
            Kind::UndefNode { names } => { let v: Vec<NodePtr> = names.iter().map(|x| self.c(x)).collect(); ALLOC_node_line(line, ALLOC_node_undef(leak(v))) }
            Kind::LambdaNode { locals, parameters, body } => {
                let is = self.compile_block(n, locals, parameters.as_deref(), body.as_deref(), IseqKind::Lambda, line);
                ALLOC_node_lambda(is)
            }
            Kind::PreExecutionNode { statements } => self.stmts(statements.as_deref()),
            Kind::PostExecutionNode { statements } => {
                // END { } → at_exit { }
                // no scope of its own in prism: like a `for` body it shares the enclosing variables
                let mut sc = Scope::new(SK::For, HashSet::new());
                sc.transparent = true;
                self.scopes.push(sc);
                let b = self.stmts(statements.as_deref());
                let is = self.finish_iseq(b, Params::default(), IseqKind::Block, self.block_label(), line, Sym(0));
                ALLOC_node_call_blk(ALLOC_node_self(), leak(vec![]), intern(b"at_exit"), CF_FCALL, line, is)
            }
            Kind::EmbeddedStatementsNode { statements } => self.stmts(statements.as_deref()),
            Kind::EmbeddedVariableNode { variable } => self.c(variable),
            Kind::ImplicitNode { value } => self.c(value),
            Kind::SplatNode { expression } => {
                // a bare splat as a value: [*x]
                let e = match expression { Some(x) => self.c(x), None => self.anon_var(b"__fwd_rest") };
                ALLOC_node_ary_build(leak(vec![e]), lstr(&[1]))
            }
            Kind::ErrorRecoveryNode { .. } => self.unsupported("syntax error", n.loc),
            _ => self.unsupported(pa::kind_name(&n.kind), n.loc),
        }
    }

    fn else_body(&mut self, n: &PNode) -> NodePtr {
        match &n.kind { Kind::ElseNode { statements } => self.stmts(statements.as_deref()), _ => self.c(n) }
    }

    fn cond(&mut self, n: &PNode) -> NodePtr {
        match &n.kind {
            Kind::FlipFlopNode { .. } => self.c(n),
            Kind::RegularExpressionNode { .. } | Kind::InterpolatedRegularExpressionNode { .. } => {
                let re = self.c(n);
                let ll = ALLOC_node_gvar(intern(b"$_"));
                self.call_node(re, s::match_op, vec![ll], 0, self.line(n.loc))
            }
            _ => self.c(n),
        }
    }

    fn integer(&mut self, v: &prism::PInt) -> NodePtr {
        if v.words.len() <= 2 {
            let mut x: u64 = v.words[0] as u64;
            if v.words.len() == 2 { x |= (v.words[1] as u64) << 32; }
            if x <= FIXNUM_MAX as u64 {
                let i = if v.neg { -(x as i64) } else { x as i64 };
                return ALLOC_node_lit(Value::fix(i));
            }
            if v.neg && x == (1u64 << 62) { return ALLOC_node_lit(Value::fix(FIXNUM_MIN)); }
        }
        let s = crate::bignum::BigInt::from_words(v.neg, &v.words).to_string_radix(10);
        ALLOC_node_biglit(lstr(s.as_bytes()))
    }

    fn dstr(&mut self, parts: &[PNode]) -> NodePtr {
        let mut v = vec![];
        self.dstr_parts(parts, &mut v);
        // buffer size guess: the literal bytes plus a little per interpolated value
        let cap: usize = v.iter().map(|&n| unsafe {
            if std::ptr::eq((*n).head.kind, &KIND_node_fstr) { let s: Str = (*(n as *mut NodeOf<node_fstr_struct>)).u.s; s.len() } else { 12 }
        }).sum();
        // short results start inside the object slot (spilling copies at most 23 bytes)
        let cap = if cap <= 48 { 0 } else { cap.min(4096) };
        ALLOC_node_dstr(leak(v), 0, cap as u32)
    }
    fn dstr_parts(&mut self, parts: &[PNode], v: &mut Vec<NodePtr>) {
        for p in parts {
            match &p.kind {
                Kind::StringNode { unescaped } => {
                    let enc = if p.flags & pa::STRING_FLAGS_FORCED_BINARY_ENCODING != 0 { crate::object::ENC_BINARY as u32 } else { 0 };
                    v.push(ALLOC_node_fstr(lstr(unescaped), enc))
                }
                Kind::InterpolatedStringNode { parts } => self.dstr_parts(parts, v),
                // srb_dstr_append converts (no intermediate String for Integers)
                Kind::EmbeddedStatementsNode { statements } => { let e = self.stmts(statements.as_deref()); v.push(e); }
                Kind::EmbeddedVariableNode { variable } => { let e = self.c(variable); v.push(e); }
                _ => { let e = self.c(p); v.push(e); }
            }
        }
    }

    fn array(&mut self, elements: &[PNode]) -> NodePtr {
        if elements.iter().any(|e| matches!(e.kind, Kind::SplatNode { .. } | Kind::KeywordHashNode { .. })) {
            let mut v = vec![]; let mut k = vec![];
            for e in elements {
                match &e.kind {
                    Kind::SplatNode { expression } => {
                        v.push(match expression { Some(x) => self.c(x), None => self.anon_var(b"__fwd_rest") }); k.push(1u8);
                    }
                    _ => { v.push(self.c(e)); k.push(0); }
                }
            }
            return ALLOC_node_ary_build(leak(v), lstr(&k));
        }
        let v: Vec<NodePtr> = elements.iter().map(|e| self.c(e)).collect();
        ALLOC_node_ary(leak(v))
    }

    fn hash(&mut self, elements: &[PNode]) -> NodePtr {
        let mut items = vec![]; let mut kinds = vec![];
        for e in elements {
            match &e.kind {
                Kind::AssocNode { key, value } => { items.push(self.c(key)); items.push(self.c(value)); kinds.push(0u8); }
                Kind::AssocSplatNode { value } => {
                    items.push(match value { Some(x) => self.c(x), None => self.anon_var(b"__fwd_kw") }); kinds.push(1);
                }
                _ => { items.push(self.c(e)); items.push(ALLOC_node_nil()); kinds.push(0); }
            }
        }
        ALLOC_node_hash(leak(items), lstr(&kinds))
    }

    fn anon_var(&mut self, nm: &[u8]) -> NodePtr {
        let s = intern(nm);
        // walk out to the method scope that declared it
        let mut depth = 0;
        for sc in self.scopes.iter().rev() {
            if sc.vars.contains_key(&s) { break; }
            if !sc.transparent { depth += 1; }
        }
        if depth as usize >= self.scopes.len() && !self.eval_outer.iter().flatten().any(|x| x.0 == s) { return ALLOC_node_nil(); }
        self.var_read(s, depth)
    }

    fn args_value(&mut self, args: Option<&PNode>) -> NodePtr {
        match args {
            None => ALLOC_node_nil(),
            Some(PNode { kind: Kind::ArgumentsNode { arguments }, .. }) => {
                if arguments.len() == 1 && !matches!(arguments[0].kind, Kind::SplatNode { .. }) { self.c(&arguments[0]) }
                else { self.array(arguments) }
            }
            Some(n) => self.c(n),
        }
    }

    fn cpath_parent(&mut self, cp: &PNode) -> (NodePtr, bool) {
        match &cp.kind {
            Kind::ConstantPathNode { parent: Some(p), .. } => (self.c(p), true),
            Kind::ConstantPathNode { parent: None, .. } => (ALLOC_node_top_const(intern(b"Object")), true),
            _ => (ALLOC_node_nil(), false),
        }
    }
    fn cpath_parts(&mut self, t: &PNode) -> (NodePtr, Sym) {
        match &t.kind {
            Kind::ConstantPathNode { parent, name } | Kind::ConstantPathTargetNode { parent, name } => {
                let nm = self.cst(name.unwrap_or(0));
                let p = match parent { Some(p) => self.c(p), None => ALLOC_node_lit(UNDEF) };
                (p, nm)
            }
            _ => (ALLOC_node_nil(), Sym(0)),
        }
    }
    fn cpath_write(&mut self, target: &PNode, v: NodePtr) -> NodePtr {
        let (p, nm) = self.cpath_parts(target);
        ALLOC_node_cdecl_path(p, nm, v)
    }

    // ---- calls ---------------------------------------------------------------------------

    fn fcall(&mut self, name: &str, args: Vec<NodePtr>, line: u32) -> NodePtr {
        ALLOC_node_call(ALLOC_node_self(), leak(args), intern(name.as_bytes()), CF_FCALL, line)
    }
    fn call_node(&mut self, recv: NodePtr, mid: Sym, args: Vec<NodePtr>, flags: u32, line: u32) -> NodePtr {
        ALLOC_node_call(recv, leak(args), mid, flags, line)
    }

    fn binop(&mut self, a: NodePtr, op: Sym, b: NodePtr, line: u32) -> NodePtr {
        match op {
            x if x == s::plus => ALLOC_node_opt_plus(a, b, line),
            x if x == s::minus => ALLOC_node_opt_minus(a, b, line),
            x if x == s::mul => ALLOC_node_opt_mul(a, b, line),
            x if x == s::div => ALLOC_node_opt_div(a, b, line),
            x if x == s::modulo => ALLOC_node_opt_mod(a, b, line),
            x if x == s::pow => ALLOC_node_opt_pow(a, b, line),
            x if x == s::lt => ALLOC_node_opt_lt(a, b, line),
            x if x == s::le => ALLOC_node_opt_le(a, b, line),
            x if x == s::gt => ALLOC_node_opt_gt(a, b, line),
            x if x == s::ge => ALLOC_node_opt_ge(a, b, line),
            x if x == s::eq => ALLOC_node_opt_eq(a, b, line),
            x if x == s::neq => ALLOC_node_opt_neq(a, b, line),
            x if x == s::ltlt => ALLOC_node_opt_ltlt(a, b, line),
            x if x == s::gtgt => ALLOC_node_opt_gtgt(a, b, line),
            x if x == s::xor => ALLOC_node_opt_xor(a, b, line),
            x if x == s::and => ALLOC_node_opt_and(a, b, line),
            x if x == s::or => ALLOC_node_opt_or(a, b, line),
            x if x == s::aref => ALLOC_node_opt_aref(a, b, line),
            _ => ALLOC_node_call(a, leak(vec![b]), op, 0, line),
        }
    }

    fn simple_arg(&self, a: &PNode) -> bool {
        !matches!(a.kind, Kind::SplatNode { .. } | Kind::KeywordHashNode { .. } | Kind::BlockArgumentNode { .. } | Kind::ForwardingArgumentsNode)
    }

    /// Names of `k: v` literal keywords when the call is `f(simple args..., k1: v1, ...)`.
    fn literal_kw_names(&self, args: &[&PNode], has_blockarg: bool, attr: bool) -> Option<Vec<Sym>> {
        if has_blockarg || attr || args.is_empty() { return None; }
        let (last, pos) = args.split_last().unwrap();
        if !pos.iter().all(|a| self.simple_arg(a)) { return None; }
        let Kind::KeywordHashNode { elements } = &last.kind else { return None };
        let mut names = vec![];
        for e in elements {
            match &e.kind {
                Kind::AssocNode { key, .. } => match &key.kind {
                    Kind::SymbolNode { unescaped } => { let s = intern(unescaped); if names.contains(&s) { return None; } names.push(s); }
                    _ => return None,
                },
                _ => return None,
            }
        }
        if names.is_empty() || names.len() > 16 { None } else { Some(names) }
    }

    fn complex_args(&mut self, args: &[&PNode]) -> (Vec<NodePtr>, Vec<u8>) {
        let mut v = vec![]; let mut k = vec![];
        for a in args {
            match &a.kind {
                Kind::SplatNode { expression } => {
                    v.push(match expression { Some(x) => self.c(x), None => self.anon_var(b"__fwd_rest") }); k.push(AK_SPLAT);
                }
                Kind::KeywordHashNode { elements } => {
                    if elements.len() == 1 && matches!(elements[0].kind, Kind::AssocSplatNode { .. }) {
                        if let Kind::AssocSplatNode { value } = &elements[0].kind {
                            v.push(match value { Some(x) => self.c(x), None => self.anon_var(b"__fwd_kw") }); k.push(AK_KWSPLAT);
                        }
                    } else { v.push(self.hash(elements)); k.push(AK_KW); }
                }
                Kind::BlockArgumentNode { expression } => {
                    v.push(match expression { Some(x) => self.c(x), None => self.anon_var(b"__fwd_blk") }); k.push(AK_BLOCK);
                }
                Kind::ForwardingArgumentsNode => {
                    v.push(self.anon_var(b"__fwd_rest")); k.push(AK_SPLAT);
                    v.push(self.anon_var(b"__fwd_kw")); k.push(AK_KWSPLAT);
                    v.push(self.anon_var(b"__fwd_blk")); k.push(AK_BLOCK);
                }
                _ => { v.push(self.c(a)); k.push(AK_ARG); }
            }
        }
        (v, k)
    }

    /// `set` stores the receiver into `sl`; `recv&.x op= v` skips `rest` for a nil receiver.
    fn safe_nav(&mut self, n: &PNode, sl: u32, set: NodePtr, rest: NodePtr) -> NodePtr {
        if n.flags & pa::CALL_NODE_FLAGS_SAFE_NAVIGATION != 0 {
            let test = ALLOC_node_special(SP_IS_NIL, ALLOC_node_lvar(sl));
            return self.seq(vec![set, ALLOC_node_if(test, ALLOC_node_nil(), rest)]);
        }
        self.seq(vec![set, rest])
    }

    fn call(&mut self, n: &PNode, recv: Option<&PNode>, mid: Sym, args: Option<&PNode>, block: Option<&PNode>, line: u32) -> NodePtr {
        let args: Vec<&PNode> = match args { Some(PNode { kind: Kind::ArgumentsNode { arguments }, .. }) => arguments.iter().collect(), _ => vec![] };
        let safe = n.flags & pa::CALL_NODE_FLAGS_SAFE_NAVIGATION != 0;
        let mut flags = 0;
        if recv.is_none() { flags |= CF_FCALL; }
        if matches!(recv.map(|r| &r.kind), Some(Kind::SelfNode)) { flags |= CF_FCALL; }
        if n.flags & pa::CALL_NODE_FLAGS_VARIABLE_CALL != 0 { flags |= CF_VCALL; }
        if n.flags & pa::CALL_NODE_FLAGS_IGNORE_VISIBILITY != 0 { flags |= CF_FCALL; }
        let attr = n.flags & pa::CALL_NODE_FLAGS_ATTRIBUTE_WRITE != 0;
        // code from a string (or a method reached by name) may hold a block that returns to this frame
        if matches!(crate::symbol::sym_str(mid).as_str(), "eval" | "instance_eval" | "class_eval" | "module_eval" | "binding"
                    | "send" | "__send__" | "public_send" | "method" | "public_method" | "instance_method") { self.cur().needs_id = true; }
        // special forms
        if recv.is_none() && args.is_empty() {
            if mid == s::block_given_p && block.is_none() { return ALLOC_node_block_given(); }
            if mid == s::lambda {
                if let Some(b @ PNode { kind: Kind::BlockNode { locals, parameters, body }, .. }) = block {
                    let is = self.compile_block(b, locals, parameters.as_deref(), body.as_deref(), IseqKind::Lambda, line);
                    return ALLOC_node_lambda(is);
                }
            }
        }
        // receiver
        let sl = if safe { Some(self.hidden_slot()) } else { None };
        let recv_node = match recv {
            // "lit".freeze / -"lit": the receiver is the interned literal
            Some(PNode { kind: Kind::StringNode { unescaped }, flags: rf, .. }) if args.is_empty() && block.is_none() && (mid == s::freeze || mid == s::uminus) => {
                let enc = if rf & pa::STRING_FLAGS_FORCED_BINARY_ENCODING != 0 { crate::object::ENC_BINARY as u32 } else { 0 };
                ALLOC_node_fstr(lstr(unescaped), enc)
            }
            Some(r) => self.c(r),
            None => ALLOC_node_self(),
        };
        let (recv_node, pre) = match sl { Some(sl) => (ALLOC_node_lvar(sl), Some(ALLOC_node_lasgn(sl, recv_node))), None => (recv_node, None) };
        let blk_iseq = match block {
            Some(b @ PNode { kind: Kind::BlockNode { locals, parameters, body }, .. }) => {
                Some(self.compile_block(b, locals, parameters.as_deref(), body.as_deref(), IseqKind::Block, line))
            }
            _ => None,
        };
        let has_blockarg = matches!(block.map(|b| &b.kind), Some(Kind::BlockArgumentNode { .. }));
        let simple = args.iter().all(|a| self.simple_arg(a)) && !has_blockarg;
        let node = if simple && blk_iseq.is_none() && !safe && recv.is_some() && !attr && args.len() == 1 && flags & CF_FCALL == 0
            && [s::plus, s::minus, s::mul, s::div, s::modulo, s::pow, s::lt, s::le, s::gt, s::ge, s::eq, s::neq, s::ltlt, s::gtgt, s::xor, s::and, s::or, s::aref].contains(&mid) {
            let a = self.c(args[0]);
            self.binop(recv_node, mid, a, line)
        } else if simple && blk_iseq.is_none() && recv.is_some() && args.is_empty() && mid == s::not && !safe {
            ALLOC_node_not(recv_node)
        } else if simple && blk_iseq.is_none() && recv.is_some() && args.len() == 2 && mid == s::aset && flags & CF_FCALL == 0 && !safe {
            let a = self.c(args[0]); let b = self.c(args[1]);
            ALLOC_node_opt_aset(recv_node, a, b, line)
        } else if simple && attr {
            let v: Vec<NodePtr> = args.iter().map(|a| self.c(a)).collect();
            ALLOC_node_attrasgn(recv_node, leak(v), mid, flags, line)
        } else if simple {
            let v: Vec<NodePtr> = args.iter().map(|a| self.c(a)).collect();
            match blk_iseq {
                Some(is) => ALLOC_node_call_blk(recv_node, leak(v), mid, flags, line, is),
                None => ALLOC_node_call(recv_node, leak(v), mid, flags, line),
            }
        } else if let Some(names) = self.literal_kw_names(&args, has_blockarg, attr) {
            // f(a, k: v, ...): keywords staged positionally with their names (no Hash)
            let mut v: Vec<NodePtr> = args[..args.len() - 1].iter().map(|a| self.c(a)).collect();
            if let Kind::KeywordHashNode { elements } = &args[args.len() - 1].kind {
                for e in elements { if let Kind::AssocNode { value, .. } = &e.kind { v.push(self.c(value)); } }
            }
            let ci = Box::leak(Box::new(CallInfo { kinds: vec![], blk: blk_iseq, kw_splat_only: false, kw_names: names }));
            ALLOC_node_call_kw(recv_node, leak(v), mid, flags, line, ci)
        } else {
            let mut all = args.clone();
            if let Some(b) = block { if has_blockarg { all.push(b); } }
            let (v, kinds) = self.complex_args(&all);
            let ci = Box::leak(Box::new(CallInfo { kinds, blk: blk_iseq, kw_splat_only: false, kw_names: vec![] }));
            let call = ALLOC_node_call_x(recv_node, leak(v), mid, flags | if attr { CF_ATTR } else { 0 }, line, ci);
            call
        };
        match pre {
            Some(set) => {
                let test = ALLOC_node_special(SP_IS_NIL, ALLOC_node_lvar(sl.unwrap()));
                let body = ALLOC_node_if(test, ALLOC_node_nil(), node);
                self.seq(vec![set, body])
            }
            None => node,
        }
    }

    fn super_call(&mut self, args: Option<&PNode>, block: Option<&PNode>, zsuper: bool, line: u32) -> NodePtr {
        let mut list: Vec<&PNode> = match args { Some(PNode { kind: Kind::ArgumentsNode { arguments }, .. }) => arguments.iter().collect(), _ => vec![] };
        let blk_iseq = match block {
            Some(b @ PNode { kind: Kind::BlockNode { locals, parameters, body }, .. }) => {
                Some(self.compile_block(b, locals, parameters.as_deref(), body.as_deref(), IseqKind::Block, line))
            }
            _ => None,
        };
        if let Some(b) = block { if matches!(b.kind, Kind::BlockArgumentNode { .. }) { list.push(b); } }
        let (mut v, mut kinds) = self.complex_args(&list);
        if zsuper {
            // forward the method's parameters (current values)
            let mi = self.scopes.iter().rposition(|s| s.kind == SK::Method);
            if let Some(mi) = mi {
                let zs = self.scopes[mi].zsuper.clone();
                let depth = self.scopes[mi + 1..].iter().filter(|s| !s.transparent).count() as u32;
                let mut kwpairs: Vec<NodePtr> = vec![];
                let mut kwkinds: Vec<u8> = vec![];
                for (k, nm) in zs {
                    let rd = self.var_read(nm, depth);
                    match k {
                        AK_ARG => { v.push(rd); kinds.push(AK_ARG); }
                        AK_SPLAT => { v.push(rd); kinds.push(AK_SPLAT); }
                        AK_KW => { kwpairs.push(ALLOC_node_lit(Value::sym(nm))); kwpairs.push(rd); kwkinds.push(0); }
                        AK_KWSPLAT => { kwpairs.push(rd); kwkinds.push(1); }
                        AK_BLOCK => { if block.is_none() { v.push(rd); kinds.push(AK_BLOCK); } }
                        _ => {}
                    }
                }
                // only a forwarded **rest: an empty one passes nothing
                let only_splat = kwkinds.iter().all(|&k| k == 1);
                if !kwkinds.is_empty() { v.push(ALLOC_node_hash(leak(kwpairs), lstr(&kwkinds))); kinds.push(if only_splat { AK_KWSPLAT } else { AK_KW }); }
            } else {
                return self.unsupported("super outside of method", Loc::default());
            }
        }
        let ci = Box::leak(Box::new(CallInfo { kinds, blk: blk_iseq, kw_splat_only: false, kw_names: vec![] }));
        ALLOC_node_super(leak(v), line, ci, zsuper)
    }

    /// Receiver + index arguments in hidden slots, the read, and a writer builder.
    fn index_parts(&mut self, recv: Option<&PNode>, args: Option<&PNode>, line: u32)
        -> (Vec<NodePtr>, NodePtr, Box<dyn Fn(&mut Self, NodePtr) -> NodePtr>) {
        let rs = self.hidden_slot();
        let r = match recv { Some(r) => self.c(r), None => ALLOC_node_self() };
        let mut pre = vec![ALLOC_node_lasgn(rs, r)];
        let list: Vec<&PNode> = match args { Some(PNode { kind: Kind::ArgumentsNode { arguments }, .. }) => arguments.iter().collect(), _ => vec![] };
        let mut slots = vec![];
        for a in list { let sl = self.hidden_slot(); let v = self.c(a); pre.push(ALLOC_node_lasgn(sl, v)); slots.push(sl); }
        let rd = if slots.len() == 1 { ALLOC_node_opt_aref(ALLOC_node_lvar(rs), ALLOC_node_lvar(slots[0]), line) }
                 else { ALLOC_node_call(ALLOC_node_lvar(rs), leak(slots.iter().map(|&s| ALLOC_node_lvar(s)).collect()), s::aref, 0, line) };
        let slots2 = slots.clone();
        let mk = Box::new(move |_s: &mut Self, v: NodePtr| {
            let mut a: Vec<NodePtr> = slots2.iter().map(|&s| ALLOC_node_lvar(s)).collect();
            a.push(v);
            ALLOC_node_attrasgn(ALLOC_node_lvar(rs), leak(a), s::aset, 0, line)
        });
        (pre, rd, mk)
    }

    // ---- multiple assignment ----------------------------------------------------------------

    fn masgn(&mut self, lefts: &[PNode], rest: Option<&PNode>, rights: &[PNode], rhs: NodePtr, splat_rhs: bool) -> NodePtr {
        let tmp = self.hidden_slot();
        let mut targets = vec![];
        let post = rights.len() as u32;
        for (i, t) in lefts.iter().enumerate() {
            let e = ALLOC_node_masgn_elem(tmp, i as u32, 0, post);
            targets.push(self.target(t, e));
        }
        if let Some(r) = rest {
            if let Kind::SplatNode { expression: Some(x) } = &r.kind {
                let e = ALLOC_node_masgn_elem(tmp, lefts.len() as u32, 1, post);
                targets.push(self.target(x, e));
            }
        }
        for (i, t) in rights.iter().enumerate() {
            let e = ALLOC_node_masgn_elem(tmp, i as u32, 2, post);
            targets.push(self.target(t, e));
        }
        let _ = splat_rhs;
        ALLOC_node_masgn(rhs, tmp, leak(targets), false)
    }

    /// Assignment of v to a target node (masgn / for / rescue => e).
    fn target(&mut self, t: &PNode, v: NodePtr) -> NodePtr {
        let line = self.line(t.loc);
        match &t.kind {
            Kind::LocalVariableTargetNode { name, depth } => { let nm = self.cst(*name); self.var_write(nm, *depth, v) }
            Kind::LocalVariableReadNode { name, depth } => { let nm = self.cst(*name); self.var_write(nm, *depth, v) }
            Kind::RequiredParameterNode { name } => { let nm = self.cst(*name); self.var_write(nm, 0, v) }
            Kind::InstanceVariableTargetNode { name } => ALLOC_node_iasgn(self.cst(*name), v),
            Kind::GlobalVariableTargetNode { name } => ALLOC_node_gasgn(self.cst(*name), v),
            Kind::ClassVariableTargetNode { name } => ALLOC_node_cvasgn(self.cst(*name), v),
            Kind::ConstantTargetNode { name } => ALLOC_node_cdecl(self.cst(*name), v),
            Kind::ConstantPathTargetNode { .. } => self.cpath_write(t, v),
            Kind::CallTargetNode { receiver, name } => {
                let r = self.c(receiver);
                let fl = if matches!(receiver.kind, Kind::SelfNode) { CF_FCALL } else { 0 };
                if t.flags & pa::CALL_NODE_FLAGS_SAFE_NAVIGATION != 0 {
                    // `a&.b = v` as a target: nothing happens for a nil receiver
                    let sl = self.hidden_slot();
                    let set = ALLOC_node_lasgn(sl, r);
                    let w = ALLOC_node_attrasgn(ALLOC_node_lvar(sl), leak(vec![v]), self.cst(*name), fl, line);
                    let test = ALLOC_node_special(SP_IS_NIL, ALLOC_node_lvar(sl));
                    return self.seq(vec![set, ALLOC_node_if(test, ALLOC_node_nil(), w)]);
                }
                ALLOC_node_attrasgn(r, leak(vec![v]), self.cst(*name), fl, line)
            }
            Kind::IndexTargetNode { receiver, arguments, .. } => {
                let r = self.c(receiver);
                let mut a: Vec<NodePtr> = match arguments.as_deref() { Some(PNode { kind: Kind::ArgumentsNode { arguments }, .. }) => arguments.iter().map(|x| self.c(x)).collect(), _ => vec![] };
                a.push(v);
                ALLOC_node_attrasgn(r, leak(a), s::aset, 0, line)
            }
            Kind::MultiTargetNode { lefts, rest, rights } => self.masgn(lefts, rest.as_deref(), rights, v, false),
            Kind::SplatNode { expression: Some(x) } => self.target(x, v),
            _ => self.unsupported(pa::kind_name(&t.kind), t.loc),
        }
    }

    // ---- case ------------------------------------------------------------------------------

    fn case_when(&mut self, pred: Option<&PNode>, conds: &[PNode], els: Option<&PNode>, _line: u32) -> NodePtr {
        let mut tail = match els { Some(e) => self.else_body(e), None => ALLOC_node_nil() };
        match pred {
            Some(p) => {
                let sl = self.hidden_slot();
                let pv = self.c(p);
                let set = ALLOC_node_lasgn(sl, pv);
                for w in conds.iter().rev() {
                    if let Kind::WhenNode { conditions, statements } = &w.kind {
                        let mut pats = vec![]; let mut kinds = vec![];
                        for cnd in conditions {
                            match &cnd.kind {
                                Kind::SplatNode { expression: Some(x) } => { pats.push(self.c(x)); kinds.push(1u8); }
                                _ => { pats.push(self.c(cnd)); kinds.push(0); }
                            }
                        }
                        let test = match lit_imm(&pats, &kinds) {
                            Some(v) => ALLOC_node_when_lit(sl, v),
                            None => ALLOC_node_when(sl, leak(pats), lstr(&kinds)),
                        };
                        let body = self.stmts(statements.as_deref());
                        tail = ALLOC_node_if(test, body, tail);
                    }
                }
                self.seq(vec![set, tail])
            }
            None => {
                for w in conds.iter().rev() {
                    if let Kind::WhenNode { conditions, statements } = &w.kind {
                        let mut test: Option<NodePtr> = None;
                        for cnd in conditions.iter().rev() {
                            let cv = self.c(cnd);
                            test = Some(match test { None => cv, Some(t) => ALLOC_node_or(cv, t) });
                        }
                        let body = self.stmts(statements.as_deref());
                        tail = ALLOC_node_if(test.unwrap_or_else(ALLOC_node_false), body, tail);
                    }
                }
                tail
            }
        }
    }

    fn case_in(&mut self, pred: Option<&PNode>, conds: &[PNode], els: Option<&PNode>, _line: u32) -> NodePtr {
        let sl = self.hidden_slot();
        let pv = match pred { Some(p) => self.c(p), None => ALLOC_node_nil() };
        let set = ALLOC_node_lasgn(sl, pv);
        if let (None, [PNode { kind: Kind::InNode { pattern, statements }, .. }]) = (els, conds) {
            // one clause, no else: the error says why it did not match
            let (test, fail) = self.single_pattern(pattern, sl);
            let body = self.stmts(statements.as_deref());
            return self.seq(vec![set, ALLOC_node_if(test, body, fail)]);
        }
        let mut tail = match els {
            Some(e) => self.else_body(e),
            None => ALLOC_node_special(SP_NOMATCH, ALLOC_node_lvar(sl)),
        };
        for w in conds.iter().rev() {
            if let Kind::InNode { pattern, statements } = &w.kind {
                let test = self.pattern(pattern, sl);
                let body = self.stmts(statements.as_deref());
                tail = ALLOC_node_if(test, body, tail);
            }
        }
        self.seq(vec![set, tail])
    }

    /// (test, raise) for a pattern whose failure raises with CRuby's descriptive message.
    fn single_pattern(&mut self, pattern: &PNode, sl: u32) -> (NodePtr, NodePtr) {
        let esl = self.hidden_slot();
        let saved = self.pm_err.replace(esl);
        let p = self.pattern(pattern, sl);
        self.pm_err = saved;
        let test = self.seq(vec![ALLOC_node_lasgn(esl, ALLOC_node_nil()), p]);
        let fail = ALLOC_node_special(SP_NOMATCH_MSG, ALLOC_node_ary(leak(vec![ALLOC_node_lvar(sl), ALLOC_node_lvar(esl)])));
        (test, fail)
    }

    /// `test`, recording why it failed (PM_* kind + args) when compiling a single pattern.
    fn pm_fail(&mut self, test: NodePtr, kind: i64, mut args: Vec<NodePtr>) -> NodePtr {
        let Some(esl) = self.pm_err else { return test };
        args.insert(0, ALLOC_node_lit(Value::fix(kind)));
        let rec = ALLOC_node_lasgn(esl, ALLOC_node_special(SP_PM_MSG, ALLOC_node_ary(leak(args))));
        ALLOC_node_or(test, self.seq(vec![rec, ALLOC_node_false()]))
    }

    /// `pat === subject` for a value pattern.
    fn eqq_pattern(&mut self, p: &PNode, sl: u32) -> NodePtr {
        let v = self.c(p);
        if self.pm_err.is_none() { return self.eqq_test(v, sl); }
        // keep the pattern value for the message (evaluated once)
        let psl = self.hidden_slot();
        let t = self.eqq_test(ALLOC_node_lasgn(psl, v), sl);
        self.pm_fail(t, PM_EQQ, vec![ALLOC_node_lvar(psl), ALLOC_node_lvar(sl)])
    }

    /// A pattern test against the value in slot `sl`; binds variables; evaluates to true/false.
    fn pattern(&mut self, p: &PNode, sl: u32) -> NodePtr {
        let subj = ALLOC_node_lvar(sl);
        match &p.kind {
            Kind::LocalVariableTargetNode { name, depth } => {
                let nm = self.cst(*name);
                let w = self.var_write(nm, *depth, subj);
                self.seq(vec![w, ALLOC_node_true()])
            }
            Kind::AlternationPatternNode { left, right } => {
                let a = self.pattern(left, sl); let b = self.pattern(right, sl);
                ALLOC_node_or(a, b)
            }
            Kind::CapturePatternNode { value, target } => {
                let a = self.pattern(value, sl);
                let w = self.target(target, ALLOC_node_lvar(sl));
                let t = self.seq(vec![w, ALLOC_node_true()]);
                ALLOC_node_and(a, t)
            }
            Kind::PinnedVariableNode { variable } => self.eqq_pattern(variable, sl),
            Kind::PinnedExpressionNode { expression } => self.eqq_pattern(expression, sl),
            Kind::IfNode { predicate, statements, .. } => {
                // pattern guard: `in pat if cond`
                let pat = statements.as_deref().and_then(|s| match &s.kind { Kind::StatementsNode { body } => body.first(), _ => None });
                let a = match pat { Some(pt) => self.pattern(pt, sl), None => ALLOC_node_true() };
                let cnd = self.c(predicate);
                let cnd = self.pm_fail(cnd, PM_GUARD, vec![]);
                ALLOC_node_and(a, cnd)
            }
            Kind::UnlessNode { predicate, statements, .. } => {
                let pat = statements.as_deref().and_then(|s| match &s.kind { Kind::StatementsNode { body } => body.first(), _ => None });
                let a = match pat { Some(pt) => self.pattern(pt, sl), None => ALLOC_node_true() };
                let cnd = self.c(predicate);
                let cnd = self.pm_fail(ALLOC_node_not(cnd), PM_GUARD, vec![]);
                ALLOC_node_and(a, cnd)
            }
            Kind::ArrayPatternNode { constant, requireds, rest, posts } => {
                let mut tests = vec![];
                if let Some(k) = constant { tests.push(self.eqq_pattern(k, sl)); }
                // deconstruct → array slot
                let asl = self.hidden_slot();
                let dec = ALLOC_node_special(SP_DECONSTRUCT, ALLOC_node_lvar(sl));
                let set = ALLOC_node_lasgn(asl, dec);
                let min = (requireds.len() + posts.len()) as i64;
                let exact = rest.is_none();
                let lenchk = ALLOC_node_special(if exact { SP_LEN_EQ } else { SP_LEN_GE }, ALLOC_node_ary(leak(vec![ALLOC_node_lvar(asl), ALLOC_node_lit(Value::fix(min))])));
                let lenchk = self.pm_fail(lenchk, PM_ARY, vec![ALLOC_node_lvar(sl), ALLOC_node_lvar(asl), ALLOC_node_lit(Value::fix(min)), if exact { ALLOC_node_true() } else { ALLOC_node_false() }]);
                tests.push(self.seq(vec![set, lenchk]));
                let post = posts.len() as u32;
                for (i, r) in requireds.iter().enumerate() {
                    let esl = self.hidden_slot();
                    let e = ALLOC_node_lasgn(esl, ALLOC_node_masgn_elem(asl, i as u32, 0, post));
                    let t = self.pattern(r, esl);
                    tests.push(self.seq(vec![e, t]));
                }
                if let Some(r) = rest {
                    if let Kind::SplatNode { expression: Some(x) } = &r.kind {
                        let esl = self.hidden_slot();
                        let e = ALLOC_node_lasgn(esl, ALLOC_node_masgn_elem(asl, requireds.len() as u32, 1, post));
                        let t = self.pattern(x, esl);
                        tests.push(self.seq(vec![e, t]));
                    }
                }
                for (i, r) in posts.iter().enumerate() {
                    let esl = self.hidden_slot();
                    let e = ALLOC_node_lasgn(esl, ALLOC_node_masgn_elem(asl, i as u32, 2, post));
                    let t = self.pattern(r, esl);
                    tests.push(self.seq(vec![e, t]));
                }
                self.and_all(tests)
            }
            Kind::FindPatternNode { constant, left, requireds, right } => {
                let mut tests = vec![];
                if let Some(k) = constant { tests.push(self.eqq_pattern(k, sl)); }
                let asl = self.hidden_slot();
                let dec = ALLOC_node_special(SP_DECONSTRUCT, ALLOC_node_lvar(sl));
                let dec = self.pm_fail(ALLOC_node_lasgn(asl, dec), PM_DECON, vec![ALLOC_node_lvar(sl)]);
                tests.push(dec);
                if self.pm_err.is_some() {
                    let nreq = ALLOC_node_lit(Value::fix(requireds.len() as i64));
                    let ge = ALLOC_node_special(SP_LEN_GE, ALLOC_node_ary(leak(vec![ALLOC_node_lvar(asl), nreq])));
                    let nreq = ALLOC_node_lit(Value::fix(requireds.len() as i64));
                    tests.push(self.pm_fail(ge, PM_ARY, vec![ALLOC_node_lvar(sl), ALLOC_node_lvar(asl), nreq, ALLOC_node_false()]));
                }
                // try each offset: implemented by a runtime loop over offsets with sub-tests re-run
                let isl = self.hidden_slot();
                let mut sub = vec![];
                let esls: Vec<u32> = requireds.iter().map(|_| self.hidden_slot()).collect();
                for (j, r) in requireds.iter().enumerate() {
                    let get = ALLOC_node_special(SP_FIND_ELEM, ALLOC_node_ary(leak(vec![ALLOC_node_lvar(asl), ALLOC_node_lvar(isl), ALLOC_node_lit(Value::fix(j as i64))])));
                    let e = ALLOC_node_lasgn(esls[j], get);
                    let t = self.pattern(r, esls[j]);
                    sub.push(self.seq(vec![e, t]));
                }
                let all = self.and_all(sub);
                let n_req = requireds.len() as i64;
                // loop: isl from 0..=len-n
                let init = ALLOC_node_lasgn(isl, ALLOC_node_lit(Value::fix(0)));
                let lim = ALLOC_node_special(SP_LEN, ALLOC_node_lvar(asl));
                let maxi = ALLOC_node_opt_minus(lim, ALLOC_node_lit(Value::fix(n_req)), 0);
                let cond = ALLOC_node_opt_le(ALLOC_node_lvar(isl), maxi, 0);
                let incr = ALLOC_node_lasgn(isl, ALLOC_node_opt_plus(ALLOC_node_lvar(isl), ALLOC_node_lit(Value::fix(1)), 0));
                let found_sl = self.hidden_slot();
                let mark = ALLOC_node_lasgn(found_sl, ALLOC_node_true());
                let brk = ALLOC_node_loop_break(ALLOC_node_nil());
                let body = ALLOC_node_if(all, self.seq(vec![mark, brk]), incr);
                let lp = ALLOC_node_while(cond, body, false);
                let reset = ALLOC_node_lasgn(found_sl, ALLOC_node_false());
                let mut binds = vec![];
                if let Kind::SplatNode { expression: Some(x) } = &left.kind {
                    let get = ALLOC_node_special(SP_FIND_PRE, ALLOC_node_ary(leak(vec![ALLOC_node_lvar(asl), ALLOC_node_lvar(isl)])));
                    binds.push(self.target(x, get));
                }
                if let Kind::SplatNode { expression: Some(x) } = &right.kind {
                    let get = ALLOC_node_special(SP_FIND_POST, ALLOC_node_ary(leak(vec![ALLOC_node_lvar(asl), ALLOC_node_lvar(isl), ALLOC_node_lit(Value::fix(n_req))])));
                    binds.push(self.target(x, get));
                }
                binds.push(ALLOC_node_true());
                let bind_all = self.seq(binds);
                let found = self.pm_fail(ALLOC_node_lvar(found_sl), PM_FIND, vec![ALLOC_node_lvar(asl)]);
                let res = ALLOC_node_and(found, bind_all);
                tests.push(self.seq(vec![reset, init, lp, res]));
                self.and_all(tests)
            }
            Kind::HashPatternNode { constant, elements, rest } => {
                let mut tests = vec![];
                if let Some(k) = constant { tests.push(self.eqq_pattern(k, sl)); }
                let hsl = self.hidden_slot();
                let keys: Vec<NodePtr> = elements.iter().filter_map(|e| match &e.kind {
                    Kind::AssocNode { key, .. } => match &key.kind { Kind::SymbolNode { unescaped } => Some(ALLOC_node_lit(Value::sym(intern(unescaped)))), _ => None },
                    _ => None,
                }).collect();
                // deconstruct_keys(nil): `**rest`, `**nil` and the empty pattern need every key
                let all_keys = match rest.as_deref().map(|r| &r.kind) {
                    Some(Kind::AssocSplatNode { value: Some(_) }) | Some(Kind::NoKeywordsParameterNode) => true,
                    Some(_) => false,
                    None => elements.is_empty(),
                };
                let keyarr = if all_keys { ALLOC_node_nil() } else { ALLOC_node_ary(leak(keys)) };
                let dec = ALLOC_node_special(SP_DECONSTRUCT_KEYS, ALLOC_node_ary(leak(vec![ALLOC_node_lvar(sl), keyarr])));
                let dec = self.pm_fail(ALLOC_node_lasgn(hsl, dec), PM_DECONK, vec![ALLOC_node_lvar(sl)]);
                tests.push(dec);
                let mut used = vec![];
                for e in elements {
                    if let Kind::AssocNode { key, value } = &e.kind {
                        if let Kind::SymbolNode { unescaped } = &key.kind {
                            let k = Value::sym(intern(unescaped));
                            used.push(ALLOC_node_lit(k));
                            let has = ALLOC_node_special(SP_HASH_HAS, ALLOC_node_ary(leak(vec![ALLOC_node_lvar(hsl), ALLOC_node_lit(k)])));
                            let has = self.pm_fail(has, PM_KEY, vec![ALLOC_node_lvar(hsl), ALLOC_node_lit(k)]);
                            let esl = self.hidden_slot();
                            let get = ALLOC_node_lasgn(esl, ALLOC_node_opt_aref(ALLOC_node_lvar(hsl), ALLOC_node_lit(k), 0));
                            let t = match &value.kind {
                                Kind::ImplicitNode { value: iv } => {
                                    // `key:` binds a local named key
                                    match &iv.kind {
                                        Kind::LocalVariableTargetNode { name, depth } => { let nm = self.cst(*name); let w = self.var_write(nm, *depth, ALLOC_node_lvar(esl)); self.seq(vec![w, ALLOC_node_true()]) }
                                        _ => ALLOC_node_true(),
                                    }
                                }
                                _ => self.pattern(value, esl),
                            };
                            tests.push(ALLOC_node_and(has, self.seq(vec![get, t])));
                        }
                    }
                }
                match rest.as_deref().map(|r| &r.kind) {
                    Some(Kind::AssocSplatNode { value: Some(v) }) => {
                        let restv = ALLOC_node_special(SP_HASH_EXCEPT, ALLOC_node_ary(leak(vec![ALLOC_node_lvar(hsl), ALLOC_node_ary(leak(used))])));
                        let w = self.target(v, restv);
                        tests.push(self.seq(vec![w, ALLOC_node_true()]));
                    }
                    Some(Kind::NoKeywordsParameterNode) => {
                        let rsl = self.hidden_slot();
                        let restv = ALLOC_node_special(SP_HASH_EXCEPT, ALLOC_node_ary(leak(vec![ALLOC_node_lvar(hsl), ALLOC_node_ary(leak(used))])));
                        let empty = ALLOC_node_special(SP_EMPTY_P, ALLOC_node_lasgn(rsl, restv));
                        tests.push(self.pm_fail(empty, PM_REST, vec![ALLOC_node_lvar(rsl)]));
                    }
                    None if elements.is_empty() => {
                        // `in {}` matches only an empty Hash
                        let empty = ALLOC_node_special(SP_EMPTY_P, ALLOC_node_lvar(hsl));
                        tests.push(self.pm_fail(empty, PM_NOT_EMPTY, vec![ALLOC_node_lvar(hsl)]));
                    }
                    _ => {}
                }
                self.and_all(tests)
            }
            Kind::SplatNode { .. } => ALLOC_node_true(),
            _ => self.eqq_pattern(p, sl),
        }
    }

    fn eqq_test(&mut self, pat: NodePtr, sl: u32) -> NodePtr {
        ALLOC_node_when(sl, leak(vec![pat]), lstr(&[0]))
    }
    fn and_all(&mut self, v: Vec<NodePtr>) -> NodePtr {
        let mut it = v.into_iter().rev();
        let mut acc = match it.next() { Some(x) => x, None => return ALLOC_node_true() };
        for x in it { acc = ALLOC_node_and(x, acc); }
        acc
    }

    fn rescue_clauses(&mut self, r: &PNode) -> Vec<NodePtr> {
        let mut out = vec![];
        let mut cur = Some(r);
        while let Some(PNode { kind: Kind::RescueNode { exceptions, reference, statements, subsequent }, .. }) = cur {
            let mut cls = vec![]; let mut kinds = vec![];
            for e in exceptions {
                match &e.kind {
                    Kind::SplatNode { expression: Some(x) } => { cls.push(self.c(x)); kinds.push(1u8); }
                    _ => { cls.push(self.c(e)); kinds.push(0); }
                }
            }
            let assign = match reference { Some(t) => self.target(t, ALLOC_node_errinfo()), None => ALLOC_node_nil() };
            let body = self.stmts(statements.as_deref());
            out.push(ALLOC_node_resbody(leak(cls), lstr(&kinds), assign, body));
            cur = subsequent.as_deref();
        }
        out
    }

    fn defined(&mut self, v: &PNode) -> NodePtr {
        let nil = ALLOC_node_nil;
        match &v.kind {
            Kind::LocalVariableReadNode { .. } | Kind::ItLocalVariableReadNode => self.dconst(DEF_LOCALVAR),
            Kind::InstanceVariableReadNode { name } => ALLOC_node_defined(DEF_IVAR, self.cst(*name), nil()),
            Kind::GlobalVariableReadNode { name } => ALLOC_node_defined(DEF_GVAR, self.cst(*name), nil()),
            Kind::ClassVariableReadNode { name } => ALLOC_node_defined(DEF_CVAR, self.cst(*name), nil()),
            Kind::ConstantReadNode { name } => ALLOC_node_defined(DEF_CONST, self.cst(*name), nil()),
            Kind::ConstantPathNode { .. } => { let (p, nm) = self.cpath_parts(v); ALLOC_node_defined(DEF_CPATH, nm, p) }
            Kind::CallNode { block: Some(b), .. } if matches!(b.kind, Kind::BlockNode { .. }) => self.dconst(DEF_EXPR),
            Kind::CallNode { receiver, name, arguments, block, .. } => {
                let nm = self.cst(*name);
                let call = match receiver {
                    None => ALLOC_node_defined(DEF_FCALL, nm, nil()),
                    Some(r) => {
                        let rv = self.defined_guard(r);
                        ALLOC_node_defined(DEF_METHOD, nm, rv)
                    }
                };
                // every argument must be defined too
                let mut tests = vec![];
                if let Some(PNode { kind: Kind::ArgumentsNode { arguments }, .. }) = arguments.as_deref() {
                    for a in arguments { self.defined_parts(a, &mut tests); }
                }
                if let Some(b) = block { self.defined_parts(b, &mut tests); }
                tests.push(call);
                self.and_all(tests)
            }
            Kind::ArrayNode { elements } | Kind::HashNode { elements } => {
                let mut tests = vec![];
                for e in elements { self.defined_parts(e, &mut tests); }
                tests.push(self.dconst(DEF_EXPR));
                self.and_all(tests)
            }
            Kind::ParenthesesNode { body: None } => self.dconst(DEF_NIL),
            Kind::YieldNode { .. } => ALLOC_node_defined(DEF_YIELD, Sym(0), nil()),
            Kind::SuperNode { .. } | Kind::ForwardingSuperNode { .. } => ALLOC_node_defined(DEF_SUPER, Sym(0), nil()),
            Kind::SelfNode => self.dconst(DEF_SELF),
            Kind::NilNode => self.dconst(DEF_NIL),
            Kind::TrueNode => self.dconst(DEF_TRUE),
            Kind::FalseNode => self.dconst(DEF_FALSE),
            Kind::BackReferenceReadNode { .. } | Kind::NumberedReferenceReadNode { .. } => {
                let e = self.c(v);
                ALLOC_node_if(e, self.dconst(DEF_BACKREF), ALLOC_node_nil())
            }
            Kind::LocalVariableWriteNode { .. } | Kind::InstanceVariableWriteNode { .. } | Kind::GlobalVariableWriteNode { .. }
            | Kind::ConstantWriteNode { .. } | Kind::MultiWriteNode { .. } | Kind::LocalVariableOperatorWriteNode { .. }
            | Kind::LocalVariableOrWriteNode { .. } | Kind::InstanceVariableOrWriteNode { .. } | Kind::ClassVariableWriteNode { .. }
            | Kind::ConstantPathWriteNode { .. } | Kind::IndexOperatorWriteNode { .. } | Kind::CallOperatorWriteNode { .. }
            | Kind::LocalVariableAndWriteNode { .. } | Kind::InstanceVariableOperatorWriteNode { .. } | Kind::InstanceVariableAndWriteNode { .. }
            | Kind::ClassVariableOperatorWriteNode { .. } | Kind::ClassVariableOrWriteNode { .. } | Kind::ClassVariableAndWriteNode { .. }
            | Kind::GlobalVariableOperatorWriteNode { .. } | Kind::GlobalVariableOrWriteNode { .. } | Kind::GlobalVariableAndWriteNode { .. }
            | Kind::ConstantOperatorWriteNode { .. } | Kind::ConstantOrWriteNode { .. } | Kind::ConstantAndWriteNode { .. }
            | Kind::ConstantPathOperatorWriteNode { .. } | Kind::ConstantPathOrWriteNode { .. } | Kind::ConstantPathAndWriteNode { .. }
            | Kind::IndexOrWriteNode { .. } | Kind::IndexAndWriteNode { .. } | Kind::CallOrWriteNode { .. } | Kind::CallAndWriteNode { .. } => self.dconst(DEF_ASGN),
            Kind::ParenthesesNode { body: Some(b) } => {
                if let Kind::StatementsNode { body } = &b.kind { if body.len() == 1 { return self.defined(&body[0]); } }
                self.dconst(DEF_EXPR)
            }
            _ => self.dconst(DEF_EXPR),
        }
    }

    /// defined? tests for the parts of an argument / element (splats, pairs, &blk look inside).
    fn defined_parts(&mut self, a: &PNode, tests: &mut Vec<NodePtr>) {
        match &a.kind {
            Kind::SplatNode { expression: Some(x) } | Kind::AssocSplatNode { value: Some(x) } => self.defined_parts(x, tests),
            Kind::BlockArgumentNode { .. } => {}
            Kind::KeywordHashNode { elements } => for e in elements { self.defined_parts(e, tests); },
            Kind::AssocNode { key, value } => { self.defined_parts(key, tests); self.defined_parts(value, tests); }
            Kind::IntegerNode { .. } | Kind::FloatNode { .. } | Kind::StringNode { .. } | Kind::SymbolNode { .. } | Kind::NilNode | Kind::TrueNode
            | Kind::FalseNode | Kind::SelfNode | Kind::LocalVariableReadNode { .. } | Kind::ImplicitNode { .. } => {}
            _ => tests.push(self.defined(a)),
        }
    }

    /// For `defined?(a.b)`: the receiver must itself be defined; evaluates to the receiver or UNDEF.
    fn defined_guard(&mut self, r: &PNode) -> NodePtr {
        match &r.kind {
            Kind::CallNode { receiver: _, .. } | Kind::ConstantReadNode { .. } | Kind::ConstantPathNode { .. } | Kind::InstanceVariableReadNode { .. } => {
                let d = self.defined(r);
                let val = self.c(r);
                // rescue errors in evaluating the receiver: defined?(a.b) where a raises → nil
                let rb = ALLOC_node_resbody(leak(vec![]), lstr(&[]), ALLOC_node_nil(), ALLOC_node_lit(UNDEF));
                let guarded = ALLOC_node_rescue(val, leak(vec![rb]), ALLOC_node_nil(), false);
                ALLOC_node_if(d, guarded, ALLOC_node_lit(UNDEF))
            }
            _ => self.c(r),
        }
    }

    // ---- for loops --------------------------------------------------------------------------

    fn for_loop(&mut self, n: &PNode, index: &PNode, coll: &PNode, body: Option<&PNode>, line: u32) -> NodePtr {
        let cv = self.c(coll);
        // the body is a block that shares the enclosing scope's variables (captured into env)
        let mut sc = Scope::new(SK::For, HashSet::new());
        sc.transparent = true;
        self.scopes.push(sc);
        let p = self.hidden_slot();
        let rd = ALLOC_node_lvar(p);
        let assign = match &index.kind {
            Kind::MultiTargetNode { lefts, rest, rights } => self.masgn(lefts, rest.as_deref(), rights, rd, false),
            Kind::SplatNode { .. } => self.masgn(&[], Some(index), &[], rd, false),
            _ => self.target(index, rd),
        };
        let b = self.stmts(body);
        let full = self.seq(vec![assign, b]);
        let mut params = Params::default();
        params.lead.push(p);
        if matches!(index.kind, Kind::MultiTargetNode { .. } | Kind::SplatNode { .. }) {
            // |*args| semantics: pack into an array when several values are yielded
            params.lead.clear();
            params.rest = Some(p);
            let mut v = vec![];
            v.push(ALLOC_node_lasgn(p, ALLOC_node_special(SP_FOR_UNPACK, ALLOC_node_lvar(p))));
            v.push(full);
            let f2 = self.seq(v);
            let is = self.finish_iseq(f2, params, IseqKind::Block, self.block_label(), line, Sym(0));
            let _ = n;
            return ALLOC_node_call_blk(cv, leak(vec![]), s::each, 0, line, is);
        }
        let is = self.finish_iseq(full, params, IseqKind::Block, self.block_label(), line, Sym(0));
        ALLOC_node_call_blk(cv, leak(vec![]), s::each, 0, line, is)
    }

    fn block_label(&self) -> String {
        let mut depth = 0;
        for sc in self.scopes.iter().rev() {
            match sc.kind {
                SK::Block | SK::Lambda | SK::For => depth += 1,
                _ => break,
            }
        }
        let base = self.method_label();
        if depth >= 1 { format!("block ({} levels) in {}", depth + 1, base) } else { format!("block in {}", base) }
    }

    fn method_label(&self) -> String {
        for sc in self.scopes.iter().rev() {
            match sc.kind {
                SK::Method => return METHOD_LABEL.with(|m| m.borrow().last().cloned().unwrap_or_default()),
                SK::Class => return CLASS_LABEL.with(|m| m.borrow().last().cloned().unwrap_or_default()),
                SK::Top | SK::Eval => return "<main>".into(),
                _ => {}
            }
        }
        "<main>".into()
    }

    // ---- scopes with code -------------------------------------------------------------------

    fn compile_def(&mut self, n: &PNode, name: Sym, params: Option<&PNode>, body: Option<&PNode>, locals: &[u32], line: u32) -> &'static Iseq {
        let saved = std::mem::take(&mut self.scopes);
        let mut bodies: Vec<&PNode> = vec![];
        if let Some(p) = params { bodies.push(p); }
        if let Some(b) = body { bodies.push(b); }
        self.analyze(n, &bodies);
        self.push_scope(SK::Method, n, locals);
        METHOD_LABEL.with(|m| m.borrow_mut().push(crate::symbol::sym_str(name)));
        self.in_def += 1;
        let ps = self.params(params, false);
        let b = self.stmts(body);
        self.in_def -= 1;
        let is = self.finish_iseq(b, ps, IseqKind::Method, crate::symbol::sym_str(name), line, name);
        METHOD_LABEL.with(|m| m.borrow_mut().pop());
        self.scopes = saved;
        is
    }

    fn compile_body(&mut self, n: &PNode, locals: &[u32], body: Option<&PNode>, kind: IseqKind, label: String, line: u32) -> &'static Iseq {
        let saved = std::mem::take(&mut self.scopes);
        let bodies: Vec<&PNode> = body.into_iter().collect();
        self.analyze(n, &bodies);
        self.push_scope(SK::Class, n, locals);
        CLASS_LABEL.with(|m| m.borrow_mut().push(label.clone()));
        let b = self.stmts(body);
        CLASS_LABEL.with(|m| m.borrow_mut().pop());
        let is = self.finish_iseq(b, Params::default(), kind, label, line, Sym(0));
        self.scopes = saved;
        is
    }

    fn compile_block(&mut self, n: &PNode, locals: &[u32], params: Option<&PNode>, body: Option<&PNode>, kind: IseqKind, line: u32) -> &'static Iseq {
        let sk = if kind == IseqKind::Lambda { SK::Lambda } else { SK::Block };
        let label = self.block_label();
        self.push_scope(sk, n, locals);
        let ps = match params {
            Some(PNode { kind: Kind::BlockParametersNode { parameters, locals: blocals }, .. }) => {
                for bl in blocals { if let Kind::BlockLocalVariableNode { name } = &bl.kind { let nm = self.cst(*name); self.declare(nm); } }
                self.params(parameters.as_deref(), true)
            }
            Some(PNode { kind: Kind::NumberedParametersNode { maximum }, .. }) => {
                let mut p = Params::default();
                for i in 1..=*maximum {
                    let nm = intern(format!("_{}", i).as_bytes());
                    let sl = self.param_slot(nm);
                    p.lead.push(sl);
                }
                if *maximum == 1 { p.has_trailing_comma = false; }
                p
            }
            Some(PNode { kind: Kind::ItParametersNode, .. }) => {
                let mut p = Params::default();
                let nm = intern(b"__it");
                self.declare(nm);
                let sl = self.param_slot(nm);
                p.lead.push(sl);
                p
            }
            Some(other) => self.params(Some(other), true),
            None => Params::default(),
        };
        let b = self.stmts(body);
        let is = self.finish_iseq(b, ps, kind, label, line, Sym(0));
        // the block's home frame: `return` in the block targets it by id
        if let Some(p) = self.scopes.last_mut() { p.needs_id = true; }
        is
    }

    /// The frame slot that receives a parameter.  A captured parameter gets a hidden
    /// slot and is copied into the env after setup.
    fn param_slot(&mut self, nm: Sym) -> u32 {
        if !self.cur().params_seen.insert(nm) { let sl = self.hidden_slot(); self.cur().param_names.push((sl, nm)); return sl; }
        let loc = match self.cur().vars.get(&nm) { Some(&l) => l, None => self.declare(nm) };
        let sl = match loc {
            VarLoc::Slot(i) => i,
            VarLoc::Env(_) => {
                let sl = self.hidden_slot();
                self.cur().param_env.push((nm, sl));
                sl
            }
        };
        self.cur().param_names.push((sl, nm));
        sl
    }

    fn params(&mut self, p: Option<&PNode>, is_block: bool) -> Params {
        let mut ps = Params::default();
        let Some(p) = p else { return ps; };
        let (requireds, optionals, rest, posts, keywords, keyword_rest, block) = match &p.kind {
            Kind::ParametersNode { requireds, optionals, rest, posts, keywords, keyword_rest, block } => (requireds, optionals, rest, posts, keywords, keyword_rest, block),
            _ => return ps,
        };
        let mut zs: Vec<(u8, Sym)> = vec![];
        for r in requireds {
            match &r.kind {
                Kind::RequiredParameterNode { name } => { let nm = self.cst(*name); ps.lead.push(self.param_slot(nm)); zs.push((AK_ARG, nm)); }
                Kind::MultiTargetNode { lefts, rest, rights } => {
                    let sl = self.hidden_slot();
                    ps.lead.push(sl);
                    let d = self.masgn(lefts, rest.as_deref(), rights, ALLOC_node_lvar(sl), false);
                    ps.destructure.push(d);
                    let hn = intern(format!("__destr{}", sl).as_bytes());
                    self.cur().vars.insert(hn, VarLoc::Slot(sl));
                    zs.push((AK_ARG, hn));
                }
                _ => {}
            }
        }
        for o in optionals {
            if let Kind::OptionalParameterNode { name, value } = &o.kind {
                let nm = self.cst(*name);
                let sl = self.param_slot(nm);
                let v = self.c(value);
                let asg = self.var_write(nm, 0, v);
                ps.opt.push((sl, asg));
                zs.push((AK_ARG, nm));
            }
        }
        if let Some(r) = rest {
            match &r.kind {
                Kind::RestParameterNode { name } => {
                    let nm = match name { Some(n) => self.cst(*n), None => { ps.anon_rest = true; intern(b"__fwd_rest") } };
                    if name.is_none() { self.declare(nm); }
                    ps.rest = Some(self.param_slot(nm));
                    zs.push((AK_SPLAT, nm));
                }
                Kind::ImplicitRestNode => { ps.has_trailing_comma = true; }
                _ => {}
            }
        }
        for r in posts {
            match &r.kind {
                Kind::RequiredParameterNode { name } => { let nm = self.cst(*name); ps.post.push(self.param_slot(nm)); zs.push((AK_ARG, nm)); }
                Kind::MultiTargetNode { lefts, rest, rights } => {
                    let sl = self.hidden_slot();
                    ps.post.push(sl);
                    let d = self.masgn(lefts, rest.as_deref(), rights, ALLOC_node_lvar(sl), false);
                    ps.destructure.push(d);
                }
                _ => {}
            }
        }
        for k in keywords {
            match &k.kind {
                Kind::RequiredKeywordParameterNode { name } => {
                    let nm = self.cst(*name);
                    let sl = self.param_slot(nm);
                    ps.kw.push(KwParam { name: nm, slot: sl, default: None, required: true, static_default: false });
                    zs.push((AK_KW, nm));
                }
                Kind::OptionalKeywordParameterNode { name, value } => {
                    let nm = self.cst(*name);
                    let sl = self.param_slot(nm);
                    // CRuby puts a literal default in place before any default expression runs
                    let static_default = matches!(value.kind, Kind::IntegerNode { .. } | Kind::FloatNode { .. } | Kind::RationalNode { .. }
                        | Kind::ImaginaryNode { .. } | Kind::SymbolNode { .. } | Kind::RegularExpressionNode { .. }
                        | Kind::NilNode | Kind::TrueNode | Kind::FalseNode | Kind::SourceLineNode);
                    let v = self.c(value);
                    let asg = self.var_write(nm, 0, v);
                    ps.kw.push(KwParam { name: nm, slot: sl, default: Some(asg), required: false, static_default });
                    zs.push((AK_KW, nm));
                }
                _ => {}
            }
        }
        if let Some(kr) = keyword_rest {
            match &kr.kind {
                Kind::KeywordRestParameterNode { name } => {
                    let nm = match name { Some(n) => self.cst(*n), None => { ps.anon_kwrest = true; intern(b"__fwd_kw") } };
                    if name.is_none() { self.declare(nm); }
                    ps.kwrest = Some(self.param_slot(nm));
                    zs.push((AK_KWSPLAT, nm));
                }
                Kind::NoKeywordsParameterNode => ps.no_kw = true,
                Kind::ForwardingParameterNode => {
                    for (nm, which) in [(b"__fwd_rest" as &[u8], 0), (b"__fwd_kw", 1), (b"__fwd_blk", 2)] {
                        let s = intern(nm);
                        self.declare(s);
                        let sl = self.param_slot(s);
                        match which { 0 => ps.rest = Some(sl), 1 => ps.kwrest = Some(sl), _ => ps.block = Some(sl) }
                    }
                    zs.push((AK_SPLAT, intern(b"__fwd_rest")));
                    zs.push((AK_KWSPLAT, intern(b"__fwd_kw")));
                    zs.push((AK_BLOCK, intern(b"__fwd_blk")));
                    self.cur().fwd = true;
                }
                _ => {}
            }
        }
        if let Some(b) = block {
            if let Kind::BlockParameterNode { name } = &b.kind {
                let nm = match name { Some(n) => self.cst(*n), None => { ps.anon_block = true; intern(b"__fwd_blk") } };
                if name.is_none() { self.declare(nm); }
                ps.block = Some(self.param_slot(nm));
                zs.push((AK_BLOCK, nm));
            }
        }
        if !is_block { self.cur().zsuper = zs; }
        ps
    }
}

/// Is every node of a method body one that cannot call, raise, allocate or look at its frame?
/// (By the names in its dump: a nested body prints as an operand, a symbol literal only makes it stricter.)
/// Does the tree hold a node that caches a literal object (whose identity a second copy would change)?
fn has_lit_cache(body: NodePtr) -> bool {
    let mut d = String::new();
    dump_node(&mut d, body);
    d.split(|ch: char| !(ch.is_alphanumeric() || ch == '_'))
        .any(|w| matches!(w, "node_fstr" | "node_regex" | "node_dregex" | "node_biglit" | "node_range_lit"))
}

fn leaf_body(body: NodePtr) -> bool {
    let mut d = String::new();
    dump_node(&mut d, body);
    d.split(|ch: char| !(ch.is_alphanumeric() || ch == '_')).filter(|w| w.starts_with("node_"))
        .all(|w| matches!(w, "node_nil" | "node_true" | "node_false" | "node_self" | "node_lit" | "node_lvar" | "node_ivar" | "node_block" | "node_if" | "node_and" | "node_or"))
}

/// Statements that can raise without passing through a node that records the line.
fn needs_line(n: &PNode) -> bool {
    match &n.kind {
        Kind::ArrayNode { .. } | Kind::HashNode { .. } | Kind::KeywordHashNode { .. } | Kind::CaseMatchNode { .. } | Kind::MatchRequiredNode { .. }
        | Kind::ClassVariableReadNode { .. } | Kind::ClassVariableWriteNode { .. } | Kind::ClassVariableOperatorWriteNode { .. }
        | Kind::ClassVariableOrWriteNode { .. } | Kind::ClassVariableAndWriteNode { .. }
        | Kind::ConstantPathWriteNode { .. } | Kind::ConstantPathOperatorWriteNode { .. } | Kind::ConstantPathOrWriteNode { .. }
        | Kind::ConstantPathAndWriteNode { .. } | Kind::ConstantOperatorWriteNode { .. } | Kind::SplatNode { .. } => true,
        Kind::LocalVariableWriteNode { value, .. } | Kind::InstanceVariableWriteNode { value, .. } | Kind::GlobalVariableWriteNode { value, .. } => needs_line(value),
        _ => false,
    }
}

pub const CF_ATTR: u32 = 64;

// node_special kinds
pub const SP_IS_NIL: u32 = 1;
pub const SP_NOMATCH: u32 = 2;
pub const SP_NAMED_CAPTURE: u32 = 3;
pub const SP_DECONSTRUCT: u32 = 4;
pub const SP_LEN_EQ: u32 = 5;
pub const SP_LEN_GE: u32 = 6;
pub const SP_DECONSTRUCT_KEYS: u32 = 7;
pub const SP_HASH_HAS: u32 = 8;
pub const SP_HASH_EXCEPT: u32 = 9;
pub const SP_EMPTY_P: u32 = 10;
pub const SP_FOR_UNPACK: u32 = 11;
pub const SP_FIND_ELEM: u32 = 12;
pub const SP_LEN: u32 = 13;
pub const SP_FIND_PRE: u32 = 14;
pub const SP_FIND_POST: u32 = 15;
pub const SP_NOMATCH_MSG: u32 = 16;
pub const SP_PM_MSG: u32 = 17;

// SP_PM_MSG kinds: why a single pattern did not match
pub const PM_EQQ: i64 = 0;
pub const PM_GUARD: i64 = 1;
pub const PM_ARY: i64 = 2;
pub const PM_DECON: i64 = 3;
pub const PM_FIND: i64 = 4;
pub const PM_DECONK: i64 = 5;
pub const PM_KEY: i64 = 6;
pub const PM_REST: i64 = 7;
pub const PM_NOT_EMPTY: i64 = 8;

thread_local! {
    static METHOD_LABEL: std::cell::RefCell<Vec<String>> = std::cell::RefCell::new(vec![]);
    static CLASS_LABEL: std::cell::RefCell<Vec<String>> = std::cell::RefCell::new(vec![]);
}

pub fn regex_opts(flags: u32) -> u32 {
    let mut o = 0;
    if flags & pa::REGULAR_EXPRESSION_FLAGS_IGNORE_CASE != 0 { o |= 1; }
    if flags & pa::REGULAR_EXPRESSION_FLAGS_EXTENDED != 0 { o |= 2; }
    if flags & pa::REGULAR_EXPRESSION_FLAGS_MULTI_LINE != 0 { o |= 4; }
    if flags & pa::REGULAR_EXPRESSION_FLAGS_FORCED_BINARY_ENCODING != 0 || flags & pa::REGULAR_EXPRESSION_FLAGS_ASCII_8BIT != 0 { o |= 32; }
    o
}

/// Parse and compile a source file into a top-level iseq.  Err = SyntaxError message.
pub fn compile_source(src: &[u8], file: &'static str, line: i32, kind: IseqKind, outer: Vec<Vec<(Sym, u32, u32)>>) -> Result<&'static Iseq, String> {
    let mut scopes: Vec<Vec<Vec<u8>>> = outer.iter().rev().map(|sc| sc.iter().map(|(n, _, _)| crate::symbol::sym_bytes(*n).to_vec()).collect()).collect();
    if !scopes.is_empty() { scopes.push(vec![]); }
    let pr = prism::parse(src, &prism::ParseOpts { filepath: file.as_bytes(), line, scopes, main_script: kind == IseqKind::Main, frozen_string_literal: 0 });
    if !pr.errors.is_empty() {
        let mut msg = String::new();
        for e in &pr.errors {
            if !msg.is_empty() { msg.push('\n'); }
            msg.push_str(&format!("{}:{}: {}", file, pr.line_of(e.loc.start), e.msg));
        }
        return Err(msg);
    }
    let mut cm = Compiler::new(&pr, file);
    if kind == IseqKind::Eval && !outer.is_empty() { cm.captures.entry(key(&pr.root)).or_default().insert(Sym(u32::MAX)); }
    cm.eval_outer = outer;
    if kind == IseqKind::Main && src.windows(16).any(|w| w == b"TOPLEVEL_BINDING") {
        cm.tl_binding = true;
        cm.captures.entry(key(&pr.root)).or_default().insert(Sym(u32::MAX));
    }
    let is = cm.compile_program(&pr.root, kind);
    if cm.errors.is_empty() { Ok(is) } else { Err(cm.errors.join("\n")) }
}

/// The value of a single non-splat `when` pattern that is an immediate literal.
fn lit_imm(pats: &[NodePtr], kinds: &[u8]) -> Option<Value> {
    if pats.len() != 1 || kinds[0] != 0 { return None; }
    let n = pats[0];
    if !std::ptr::eq(unsafe { (*n).head.kind }, &KIND_node_lit) { return None; }
    let v = unsafe { (*(n as *mut NodeOf<node_lit_struct>)).u.v };
    if v.is_fixnum() || v.is_special() { Some(v) } else { None }
}

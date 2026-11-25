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
use crate::fxhash::{HashMap, HashSet};
#[path = "compile_cov.rs"]
mod cov;

pub type IseqRef = &'static Iseq;
pub type CiRef = &'static CallInfo;
pub type Str = &'static [u8];

#[derive(Default)]
pub struct LitCache { pub v: std::cell::Cell<Option<Value>> }

/// Argument kinds of a general call (node_call_x / node_super / node_yield_x).
pub const AK_ARG: u8 = 0;
pub const AK_SPLAT: u8 = 1;
pub const AK_KW: u8 = 2;      // a keyword Hash built by the call site
pub const AK_KWSPLAT: u8 = 3; // **h
pub const AK_BLOCK: u8 = 4;   // &blk

pub struct CallInfo { pub kinds: Vec<u8>, pub blk: Option<&'static Iseq>, pub kw_splat_only: bool, pub kw_names: Vec<Sym> }

// Merkle hashes of the opaque operands: by content, never by an address or an id of this process,
// so the same code in another program (or at another place) gets the same SD (Reusable).

/// An immediate by its bits (value_bake bakes it), a Symbol by its name, a heap literal by kind
/// (compile.rs puts only immediates in Value operands; no payload is read: hashing has no context).
pub fn value_hash(v: Value) -> u64 {
    use crate::astrust::{hash_bytes, hash_merge, hash_str, hash_u64};
    if v.is_sym() { hash_merge(hash_str("sym"), hash_bytes(crate::symbol::sym_bytes(v.as_sym()))) }
    else if !v.is_heap() { hash_u64(v.raw()) }
    else if crate::heap::is_heap_float(v) { hash_merge(hash_str("float"), hash_u64(crate::heap::float_val(v).to_bits())) }
    else { hash_str("heap") }

}
pub fn value_bake(v: Value) -> Option<String> {
    if v.is_sym() || v.is_heap() { None } else { Some(format!("const {{ Value::imm({:#x}) }}", v.raw())) }
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
/// Encodings of file names that are not UTF-8 (the main script's is the locale's, eval's as given).
static FILE_ENCS: std::sync::Mutex<Option<HashMap<String, u8>>> = std::sync::Mutex::new(None);
pub fn set_file_enc(f: &str, e: u8) {
    let mut t = FILE_ENCS.lock().unwrap_or_else(|x| x.into_inner());
    let t = t.get_or_insert_with(HashMap::default);
    if e == crate::object::ENC_UTF8 { t.remove(f); } else { t.insert(f.to_string(), e); }
}
/// The encoding of `__FILE__` / Location#path for file name `f`.
pub fn file_enc(f: &str) -> u8 {
    FILE_ENCS.lock().unwrap_or_else(|x| x.into_inner()).as_ref().and_then(|t| t.get(f).copied()).unwrap_or(crate::object::ENC_UTF8)
}
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
    end_line: u32,                // the line of the scope's last byte (TracePoint return / end events)
    trace_line: u32,              // the line of the last line event put in (one per line, as CRuby's)
    cov_line: u32,                // Coverage: ISEQ_COMPILE_DATA(iseq)->last_line
    cov_pending: bool,            // Coverage: a line event waits for the next instruction
    cov_leaf: u32,                // Coverage: prism id of the node whose instruction has the last counter
}

impl Scope {
    fn new(kind: SK, captured: HashSet<Sym>) -> Scope {
        Scope { kind, vars: HashMap::default(), names: vec![], nslots: 2, env_size: 0, captured, loop_depth: 0,
                zsuper: vec![], fwd: false, transparent: kind == SK::For, param_env: vec![], children: vec![], params_seen: HashSet::default(), param_names: vec![], needs_id: false, end_line: 0, trace_line: 0, cov_line: 0, cov_pending: false, cov_leaf: u32::MAX }
    }
    fn has_env(&self) -> bool { self.env_size > 0 }
}

pub struct Compiler<'a> {
    pr: &'a ParseResult,
    file: &'static str,
    src: &'a [u8],
    src_key: std::cell::OnceCell<u64>,   // hash of src: the program key of its bodies in hopt_index.txt (aot.rs)
    scopes: Vec<Scope>,
    captures: HashMap<usize, HashSet<Sym>>,
    outer: HashMap<usize, Vec<(u32, Sym)>>,   // block -> outer variables it uses (prism constant id, name), `yield` too
    pending_outer: Vec<Sym>,      // the next finish_iseq's Iseq::outer_vars
    ractor_src: std::cell::OnceCell<bool>,   // the source names `Ractor` (only then are outer variables collected)
    pub eval_outer: Vec<Vec<(Sym, u32, u32)>>,  // eval: outer scopes' (name, hops, idx), innermost first
    in_def: u32,
    pm_err: Option<u32>,          // single-pattern match: slot receiving the failure description
    yields: bool,                 // a yield in the iseq being compiled (a block's includes its inner blocks')
    pm_dcache: Option<(u32, u32)>, // case/in: (subject slot, slot caching its #deconstruct)
    tl_binding: bool,
    required: bool,               // a loaded / required file: its top level is "<top (required)>"
    cov: Option<usize>,           // Coverage file id: lines / methods get node_cov counters
    cov_popped: bool,             // Coverage: the next c() compiles a statement whose value is unused
    trace: bool,                  // compiled for tracing (vm::TRACE_COMPILE): trace points, no entries, locals in envs
    flat_stmts: bool,             // the next statements are no lines of their own (modifier / ternary forms)
    tag: bool,                    // tag nodes with prism node ids: not saruby's own <internal:..> code (no path to
                                  // re-read), nor a parse whose ids overflow the tag (the root's id is the largest)
    pub errors: Vec<String>,
    csyms: Vec<std::cell::Cell<Sym>>,   // pr.constants interned, on first use (NOSYM: not yet)
    src_enc: u32,                       // the source encoding (magic comment)
}
const NOSYM: Sym = Sym(u32::MAX);

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
        let c = Compiler { pr, file, src: &[], src_key: std::cell::OnceCell::new(), scopes: vec![], captures: HashMap::default(), outer: HashMap::default(), pending_outer: vec![], ractor_src: std::cell::OnceCell::new(), eval_outer: vec![], in_def: 0, pm_err: None, yields: false, pm_dcache: None, tl_binding: false, required: false, cov: None, cov_popped: false, trace: false, flat_stmts: false, tag: !file.starts_with("<internal:") && pr.root.id < 1 << (32 - NF_TAG_SHIFT), errors: vec![],
                   csyms: vec![std::cell::Cell::new(NOSYM); pr.constants.len()],
                   src_enc: crate::builtins::encoding::find_enc(pr.encoding.as_bytes()).unwrap_or(crate::object::ENC_UTF8) as u32 };
        // identifiers of a non-UTF-8 source are symbols in its encoding (as its symbol literals)
        if c.src_enc != crate::object::ENC_UTF8 as u32 {
            for (i, b) in pr.constants.iter().enumerate() { if !b.is_ascii() { c.csyms[i].set(c.sym_lit(b, 0)); } }
        }
        c
    }
    /// Encoding of a string literal: the source's unless an escape forced one.
    fn lit_enc(&self, flags: u32) -> u32 {
        if flags & pa::STRING_FLAGS_FORCED_BINARY_ENCODING != 0 { crate::object::ENC_BINARY as u32 }
        else if flags & pa::STRING_FLAGS_FORCED_UTF8_ENCODING != 0 { crate::object::ENC_UTF8 as u32 } else { self.src_enc }
    }

    /// A symbol literal: a non-ASCII name in a source encoding other than UTF-8 keeps that encoding.
    fn sym_lit(&self, b: &[u8], flags: u32) -> Sym {
        let enc = if flags & pa::SYMBOL_FLAGS_FORCED_BINARY_ENCODING != 0 { crate::object::ENC_BINARY } else { self.src_enc as u8 };
        if enc == crate::object::ENC_UTF8 || flags & pa::SYMBOL_FLAGS_FORCED_UTF8_ENCODING != 0 || b.is_ascii() { intern(b) } else { crate::symbol::intern_enc(b, enc) }
    }

    fn cst(&self, c: u32) -> Sym {
        let s = self.csyms[c as usize].get();
        if s != NOSYM { return s; }
        let s = intern(&self.pr.constants[c as usize]);
        self.csyms[c as usize].set(s);
        s
    }
    fn line(&self, l: Loc) -> u32 { self.pr.line_of(l.start) }
    fn cur(&mut self) -> &mut Scope { self.scopes.last_mut().unwrap() }

    // ---- capture analysis -----------------------------------------------------------------

    /// Walk a hard scope's body; record variables that blocks access from outer scopes.
    fn analyze(&mut self, root: &PNode, body: &[&PNode]) {
        let mut stack: Vec<(usize, bool)> = vec![(key(root), false)];
        let mut uses_binding = false;
        for b in body { self.analyze_rec(b, &mut stack, &mut uses_binding); }
        if *self.ractor_src.get_or_init(|| self.pr.constants.iter().any(|c| &c[..] == b"Ractor")) {
            let mut os = vec![(key(root), false, vec![])];
            for b in body { self.outer_rec(b, &mut os, false); }
        }
        if uses_binding || self.trace {
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

    /// Iseq::outer_vars (Ractor.new's isolation check): per block, the outer variables it (or a block in it)
    /// reads or writes and `yield`; a read whose value is unused does not count (CRuby's compiler drops it).
    /// Its own walk, run only for sources that name Ractor: analyze_rec pays for every node it visits.
    fn outer_rec(&mut self, n: &PNode, os: &mut Vec<(usize, bool, Vec<(u32, Sym)>)>, void: bool) {
        let name_depth = match &n.kind {
            Kind::LocalVariableReadNode { name, depth } if !void => Some((*name, *depth)),
            Kind::LocalVariableTargetNode { name, depth } | Kind::LocalVariableWriteNode { name, depth, .. } | Kind::LocalVariableOperatorWriteNode { name, depth, .. }
            | Kind::LocalVariableOrWriteNode { name, depth, .. } | Kind::LocalVariableAndWriteNode { name, depth, .. } => Some((*name, *depth)),
            _ => None,
        };
        if let Some((name, depth)) = name_depth {
            let mut d = depth as i64;
            let mut i = os.len() as i64 - 1;
            while i >= 0 && (os[i as usize].1 || d > 0) { if !os[i as usize].1 { d -= 1; } i -= 1; }
            let nm = self.cst(name);
            for e in os.iter_mut().skip((i + 1).max(0) as usize) { if !e.1 { e.2.push((name, nm)); } }
        }
        let scope = |os: &mut Vec<(usize, bool, Vec<(u32, Sym)>)>, k: usize, t: bool| os.push((k, t, vec![]));
        match &n.kind {
            Kind::YieldNode { .. } => { for e in os.iter_mut().skip(1) { if !e.1 { e.2.push((u32::MAX, s::yield_)); } } }
            Kind::StatementsNode { body } => {
                for (i, ch) in body.iter().enumerate() { self.outer_rec(ch, os, i + 1 < body.len()); }
                return;
            }
            Kind::BlockNode { .. } | Kind::LambdaNode { .. } => {
                scope(os, key(n), false);
                for ch in n.children() { self.outer_rec(ch, os, false); }
                let (k, _, v) = os.pop().unwrap();
                if !v.is_empty() { self.outer.insert(k, v); }
                return;
            }
            Kind::ForNode { index, collection, statements } => {
                self.outer_rec(collection, os, false);
                scope(os, key(n), true);
                self.outer_rec(index, os, false);
                if let Some(st) = statements { self.outer_rec(st, os, false); }
                os.pop();
                return;
            }
            Kind::PostExecutionNode { statements } => {
                scope(os, key(n), true);
                if let Some(st) = statements { self.outer_rec(st, os, false); }
                os.pop();
                return;
            }
            Kind::DefNode { receiver, .. } => { if let Some(r) = receiver { self.outer_rec(r, os, false); } return; }
            Kind::ClassNode { constant_path, superclass, .. } => {
                self.outer_rec(constant_path, os, false);
                if let Some(s) = superclass { self.outer_rec(s, os, false); }
                return;
            }
            Kind::ModuleNode { constant_path, .. } => { self.outer_rec(constant_path, os, false); return; }
            Kind::SingletonClassNode { expression, .. } => { self.outer_rec(expression, os, false); return; }
            _ => {}
        }
        for ch in n.children() { self.outer_rec(ch, os, false); }
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
        if self.trace { sc.end_line = self.pr.line_of((n.loc.start + n.loc.len).saturating_sub(1)); }
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
        self.required = kind == IseqKind::Top && !self.file.starts_with("<internal:");
        self.push_scope(sk, root, &locals);
        // BEGIN { } (top level, or in the loop -n / -p wrap it in) runs first in its code unit; c_node leaves it
        // out of its place
        fn begins<'p>(n: &'p PNode, out: &mut Vec<&'p PNode>) {
            match &n.kind {
                Kind::StatementsNode { body } => for x in body { begins(x, out) },
                Kind::WhileNode { statements: Some(s), .. } => begins(s, out),
                Kind::PreExecutionNode { statements: Some(s) } => out.push(s),
                _ => {}
            }
        }
        let mut begins_v = vec![];
        begins(stmts, &mut begins_v);
        let begins = begins_v;
        let mut body = self.c(stmts);
        if !begins.is_empty() {
            let mut v: Vec<NodePtr> = begins.into_iter().map(|b| self.c(b)).collect();
            v.push(body);
            body = self.seq(v);
        }
        if self.tl_binding {
            // the main script mentions TOPLEVEL_BINDING: bind it before anything runs
            let b = self.fcall("binding", vec![], 1);
            body = self.seq(vec![ALLOC_node_special(SP_TOPLEVEL_BINDING, b), body]);
        }
        // eval: the label of the frame its binding came from (EVAL_LABEL)
        let label = if kind == IseqKind::Eval { EVAL_LABEL.with(|l| l.borrow().clone()).unwrap_or_else(|| "<main>".to_string()) } else { self.top_label().into() };
        self.finish_iseq(body, Params::default(), kind, label, 1, Sym(0))
    }

    fn finish_iseq(&mut self, body: NodePtr, mut params: Params, kind: IseqKind, label: String, line: u32, name: Sym) -> &'static Iseq {
        let mut sc = self.scopes.pop().unwrap();
        // traced code is run by invoke_iseq / run_block_iseq, which report its call events
        let lead_only_entry = !self.trace;
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
        let entry = if kind == IseqKind::Method && lead_only && lead_only_entry {
            let mut fl = 0;
            if sc.env_size > 0 { fl |= ME_ENV; }
            if sc.needs_id || sc.captured.contains(&Sym(u32::MAX)) { fl |= ME_ID; }
            if fl == 0 && sc.nslots as usize == 2 + params.lead.len() && leaf_body(body) { fl |= ME_LEAF; }
            if nocall_body(body) { fl |= ME_NOCALL; }
            ALLOC_node_method_entry(sc.nslots, params.lead.len() as u32, fl, line, body)
        } else { std::ptr::null_mut() };
        let bentry = if (kind == IseqKind::Block || kind == IseqKind::Lambda) && lead_only && lead_only_entry {
            let fl = if sc.env_size > 0 { ME_ENV } else { 0 } | if kind == IseqKind::Lambda { ME_LAMBDA } else { 0 };
            ALLOC_node_block_entry(sc.nslots, params.lead.len() as u32, fl, line, body)
        } else { std::ptr::null_mut() };
        // dense keyword layout: lead then keywords in declared order, nothing else.  kentry takes
        // them all as plain params (a site passing every keyword in order); dentry takes none.
        let dense = lead_only_entry && kind == IseqKind::Method && !params.kw.is_empty() && params.opt.is_empty() && params.rest.is_none()
            && params.post.is_empty() && params.kwrest.is_none() && params.block.is_none() && params.destructure.is_empty()
            && !params.no_kw && !params.forwarding && !params.has_trailing_comma
            && params.lead.iter().chain(params.kw.iter().map(|k| &k.slot)).enumerate().all(|(i, &s)| s == 2 + i as u32);
        let me_flags = |np: usize, b: NodePtr| {
            let mut fl = 0;
            if sc.env_size > 0 { fl |= ME_ENV; }
            if sc.needs_id || sc.captured.contains(&Sym(u32::MAX)) { fl |= ME_ID; }
            if fl == 0 && sc.nslots as usize == 2 + np && leaf_body(b) { fl |= ME_LEAF; }
            if nocall_body(b) { fl |= ME_NOCALL; }
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
            info: CodeInfo { file: self.file, label, line, end_line: sc.end_line, nid: node_tag_current() }, kind, simple, lead_only, name, local_names, arity, uses_block: self.yields, traced: self.trace,
            parent: std::cell::Cell::new(std::ptr::null()), ruby2_keywords: std::cell::Cell::new(false), arrow: std::cell::Cell::new(false), end_once: std::cell::Cell::new(0),
            outer_names: if kind == IseqKind::Eval { self.eval_outer.iter().flatten().cloned().collect() } else { vec![] },
            outer_vars: std::mem::take(&mut self.pending_outer),
        }));
        for ch in &sc.children { ch.parent.set(is); }
        // a method whose locals all live in its env (it calls eval / binding) may run an eval "super"
        if kind == IseqKind::Method && sc.captured.contains(&Sym(u32::MAX)) && !sc.zsuper.is_empty() {
            METHOD_ZSUPER.with(|m| m.borrow_mut().insert(is as *const Iseq as usize, sc.zsuper.clone()));
        }
        if let Some(p) = self.scopes.last_mut() { p.children.push(is); }
        crate::aot::register_iseq(is, &|| *self.src_key.get_or_init(|| crate::astrust::hash_bytes(self.src)));
        is
    }

    // ---- statements & expressions ----------------------------------------------------------

    fn stmts(&mut self, n: Option<&PNode>) -> NodePtr {
        match n { Some(n) => self.c(n), None => ALLOC_node_nil() }
    }
    fn stmts_flat(&mut self, n: Option<&PNode>, flat: bool) -> NodePtr {
        self.flat_stmts = flat;
        let r = self.stmts(n);
        self.flat_stmts = false;
        r
    }

    fn seq(&mut self, v: Vec<NodePtr>) -> NodePtr {
        match v.len() { 0 => ALLOC_node_nil(), 1 => v[0], _ => ALLOC_node_block(leak(v)) }
    }

    /// A flip-flop end: an Integer literal is compared with $. (CRuby).
    fn ff_cond(&mut self, x: &PNode) -> NodePtr {
        if matches!(x.kind, Kind::IntegerNode { .. }) {
            let l = self.line(x.loc);
            let v = self.c(x);
            return ALLOC_node_opt_eq(v, ALLOC_node_gvar(intern(b"$.")), l);
        }
        self.cond(x)
    }
    /// pm_compile_yield_node: a yield whose local scope is the top level or a class body is a SyntaxError.
    fn yield_invalid(&self) -> bool {
        match self.scopes.iter().rev().find(|s| !matches!(s.kind, SK::Block | SK::Lambda | SK::For)).map(|s| s.kind) {
            Some(SK::Top) | Some(SK::Class) => true,
            Some(SK::Eval) => EVAL_NO_YIELD.get(),
            _ => false,
        }
    }
    fn unsupported(&mut self, what: &str, l: Loc) -> NodePtr {
        let msg = format!("{}:{}: {} is not supported", self.file, self.line(l), what);
        ALLOC_node_unsupported(lstr(msg.as_bytes()))
    }

    fn dconst(&self, kind: u32) -> NodePtr { ALLOC_node_defined(kind, Sym(0), ALLOC_node_nil()) }

    /// n's nodes, tagged with its prism node id (CRuby's node id of an instruction: error_highlight)
    #[inline(always)]
    pub fn c(&mut self, n: &PNode) -> NodePtr {
        if self.cov.is_some() { return self.c_cov(n); }
        self.c_tagged(n)
    }
    #[inline(always)]
    fn c_tagged(&mut self, n: &PNode) -> NodePtr {
        if !self.tag { return self.c_node(n); }
        let saved = node_tag_swap(n.id);
        let r = self.c_node(n);
        node_tag_swap(saved);
        r
    }
    fn c_node(&mut self, n: &PNode) -> NodePtr {
        let line = self.line(n.loc);
        match &n.kind {
            Kind::StatementsNode { body } => self.statements(body),
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
                let enc = self.lit_enc(n.flags);
                if n.flags & pa::STRING_FLAGS_FROZEN != 0 { ALLOC_node_fstr(lstr(unescaped), enc) }
                else if n.flags & pa::STRING_FLAGS_MUTABLE != 0 { ALLOC_node_str(lstr(unescaped), enc) }
                else { ALLOC_node_cstr(lstr(unescaped), enc) }
            }
            Kind::XStringNode { unescaped } => {
                // the command is a frozen literal (putobject)
                let enc = self.lit_enc(n.flags);
                let s = ALLOC_node_fstr(lstr(unescaped), enc);
                self.fcall("`", vec![s], line)
            }
            Kind::InterpolatedXStringNode { parts } => {
                let s = self.dstr(parts);
                self.fcall("`", vec![s], line)
            }
            Kind::SymbolNode { unescaped } => ALLOC_node_lit(Value::sym(self.sym_lit(unescaped, n.flags))),
            Kind::InterpolatedStringNode { parts } => {
                // "a" "b": one chilled literal, as CRuby
                if n.flags & (pa::INTERPOLATED_STRING_NODE_FLAGS_FROZEN | pa::INTERPOLATED_STRING_NODE_FLAGS_MUTABLE) == 0 {
                    if let Some((b, enc)) = self.static_concat(parts) { return ALLOC_node_cstr(lstr(&b), enc); }
                }
                let d = self.dstr(parts);
                if n.flags & pa::INTERPOLATED_STRING_NODE_FLAGS_FROZEN != 0 { self.call_node(d, s::freeze, vec![], 0, line) } else { d }
            }
            Kind::InterpolatedSymbolNode { parts } => { let d = self.dstr(parts); ALLOC_node_dsym(d) }
            Kind::RegularExpressionNode { unescaped } => self.regex_lit(unescaped, n.flags, line),
            Kind::InterpolatedRegularExpressionNode { parts } => {
                let d = self.dstr(parts);
                ALLOC_node_dregex(d, regex_opts(n.flags, self.src_enc), n.flags & pa::REGULAR_EXPRESSION_FLAGS_ONCE != 0)
            }
            Kind::MatchLastLineNode { unescaped } => {
                let re = ALLOC_node_regex(lstr(unescaped), regex_opts(n.flags, self.src_enc));
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
            Kind::SourceFileNode { filepath } => {
                let e = file_enc(self.file) as u32;
                if n.flags & pa::STRING_FLAGS_MUTABLE != 0 { ALLOC_node_str(lstr(filepath), e) } else { ALLOC_node_cstr(lstr(filepath), e) }
            }
            Kind::SourceLineNode => ALLOC_node_lit(Value::fix(line as i32 as i64)),
            Kind::SourceEncodingNode => {
                let e = ALLOC_node_top_const(intern(b"Encoding"), line);
                let nm = crate::builtins::encoding::enc_name(self.src_enc as u8).to_ascii_uppercase().replace('-', "_");
                ALLOC_node_const_path(e, intern(nm.as_bytes()), line)
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
                if pure && !self.trace && (op == s::plus || op == s::minus) {
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
            Kind::GlobalVariableReadNode { name } => { let nm = self.cst(*name); let r = ALLOC_node_gvar(nm); if gvar_warns(nm) { ALLOC_node_line(line, r) } else { r } }
            Kind::GlobalVariableWriteNode { name, value } => {
                let nm = self.cst(*name); let v = self.c(value); let r = ALLOC_node_gasgn(nm, v);
                if gvar_warns(nm) { ALLOC_node_line(line, r) } else { r }
            }
            Kind::GlobalVariableOperatorWriteNode { name, value, binary_operator } => {
                let nm = self.cst(*name);
                let r = ALLOC_node_gvar(nm); let v = self.c(value);
                let e = self.binop(r, self.cst(*binary_operator), v, line);
                ALLOC_node_gasgn(nm, e)
            }
            Kind::GlobalVariableOrWriteNode { name, value } => {
                // defined?($x) && $x || ($x = v): no "not initialized" warning
                let nm = self.cst(*name); let v = self.c(value);
                let d = ALLOC_node_defined(DEF_GVAR, nm, ALLOC_node_nil());
                ALLOC_node_or(ALLOC_node_if(d, ALLOC_node_gvar(nm), ALLOC_node_nil()), ALLOC_node_gasgn(nm, v))
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
            Kind::ConstantWriteNode { name, value } => { let v = self.c(value); ALLOC_node_cdecl(self.cst(*name), v, line) }
            Kind::ConstantOrWriteNode { name, value } => {
                // defined?(C) && C || (C = v): a nil constant is overwritten
                let nm = self.cst(*name); let v = self.c(value);
                let d = ALLOC_node_defined(DEF_CONST, nm, ALLOC_node_nil());
                ALLOC_node_or(ALLOC_node_if(d, ALLOC_node_const(nm, line), ALLOC_node_nil()), ALLOC_node_cdecl(nm, v, line))
            }
            Kind::ConstantAndWriteNode { name, value } => {
                let nm = self.cst(*name); let v = self.c(value);
                ALLOC_node_and(ALLOC_node_const(nm, line), ALLOC_node_cdecl(nm, v, line))
            }
            Kind::ConstantOperatorWriteNode { name, value, binary_operator } => {
                let nm = self.cst(*name); let v = self.c(value);
                let e = self.binop(ALLOC_node_const(nm, line), self.cst(*binary_operator), v, line);
                ALLOC_node_cdecl(nm, e, line)
            }
            Kind::ConstantPathNode { parent, name } => {
                let nm = self.cst(name.unwrap_or(0));
                // CRuby's opt_getconstant_path: a path from a constant (or ::) is one instruction, at the whole path
                match parent { None => ALLOC_node_top_const(nm, line), Some(p) => { let pv = self.c(p); if static_cpath(p) { retag_cpath(pv, node_tag_current()); } ALLOC_node_const_path(pv, nm, line) } }
            }
            Kind::ConstantPathWriteNode { target, value } => {
                let v = self.c(value);
                self.cpath_write(target, v, line)
            }
            // P::C op= v: P is evaluated once
            Kind::ConstantPathOrWriteNode { target, value } => {
                let (pre, rd, w) = self.cpath_op_parts(target, line);
                let v = self.c(value);
                let w = w(v);
                let (p, nm) = match pre.1 { Some(sl) => (ALLOC_node_lvar(sl), pre.2), None => (ALLOC_node_top_const(intern(b"Object"), line), pre.2) };
                let d = ALLOC_node_defined(DEF_CPATH, nm, p);
                let e = ALLOC_node_or(ALLOC_node_if(d, rd, ALLOC_node_nil()), w);
                match pre.0 { Some(set) => self.seq(vec![set, e]), None => e }
            }
            Kind::ConstantPathAndWriteNode { target, value } => {
                let (pre, rd, w) = self.cpath_op_parts(target, line);
                let v = self.c(value);
                let e = ALLOC_node_and(rd, w(v));
                match pre.0 { Some(set) => self.seq(vec![set, e]), None => e }
            }
            Kind::ConstantPathOperatorWriteNode { target, value, binary_operator } => {
                let (pre, rd, w) = self.cpath_op_parts(target, line);
                let v = self.c(value);
                let e = self.binop(rd, self.cst(*binary_operator), v, line);
                let e = w(e);
                match pre.0 { Some(set) => self.seq(vec![set, e]), None => e }
            }
            Kind::ShareableConstantNode { write } => self.c(write),
            Kind::BackReferenceReadNode { name } => {
                let nm = &self.pr.constants[*name as usize];
                let k = match nm.as_slice() { b"$&" => 0, b"$`" => 1, b"$'" => 2, b"$+" => 3, _ => 0 };
                ALLOC_node_back_ref(k)
            }
            // prism gives 0 for a number too large to be a group
            Kind::NumberedReferenceReadNode { number } => if *number == 0 { ALLOC_node_nil() } else { ALLOC_node_nth_ref(*number) },
            // control flow
            Kind::IfNode { predicate, statements, subsequent, .. } if self.cov.is_some() => self.if_node(n, predicate, statements.as_deref(), subsequent.as_deref()),
            Kind::UnlessNode { predicate, statements, else_clause } if self.cov.is_some() => self.unless_node(n, predicate, statements.as_deref(), else_clause.as_deref()),
            Kind::IfNode { predicate, statements, subsequent, .. } => {
                let cnd = self.cond(predicate);
                // `x if c` / `c ? x : y`: no line events of their own (CRuby's newline nodes)
                let flat = n.loc.start >= predicate.loc.start;
                let t = self.stmts_flat(statements.as_deref(), flat);
                let e = match subsequent { Some(x) => { self.flat_stmts = flat; let e = self.else_body(x); self.flat_stmts = false; e } None => ALLOC_node_nil() };
                ALLOC_node_if(cnd, t, e)
            }
            Kind::UnlessNode { predicate, statements, else_clause } => {
                let cnd = self.cond(predicate);
                let t = self.stmts_flat(statements.as_deref(), n.loc.start >= predicate.loc.start);
                let e = match else_clause { Some(x) => self.else_body(x), None => ALLOC_node_nil() };
                ALLOC_node_if(cnd, e, t)
            }
            Kind::ElseNode { statements } => self.stmts(statements.as_deref()),
            Kind::AndNode { left, right } => { let a = self.c(left); let b = self.c(right); ALLOC_node_and(a, b) }
            Kind::OrNode { left, right } => { let a = self.c(left); let b = self.c(right); ALLOC_node_or(a, b) }
            Kind::WhileNode { predicate, statements } | Kind::UntilNode { predicate, statements } => {
                let until = matches!(n.kind, Kind::UntilNode { .. });
                let cnd = self.cond(predicate);
                let bb = self.bbase(if until { "until" } else { "while" }, n);
                self.cur().loop_depth += 1;
                let body = self.stmts_flat(statements.as_deref(), n.loc.start >= predicate.loc.start);
                self.cur().loop_depth -= 1;
                let body = self.btarget(bb, "body", |c| match statements { Some(s) => c.ploc(s), None => c.ploc(n) }, body);
                let body = if loop_needs_no_poll(cnd, body) { body } else { ALLOC_node_poll(self.line(predicate.loc), body) };
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
            Kind::ReturnNode { arguments } => {
                if arguments.is_some() && self.scopes.last().is_some_and(|s| s.kind == SK::Top) {
                    PARSE_WARNINGS.with(|w| w.borrow_mut().push((format!("{}: warning: argument of top-level return is ignored\n", self.file), false)));
                }
                let v = self.args_value(arguments.as_deref());
                // directly in eval code: its target is checked at run time (rt::srb_eval_return)
                if self.scopes.iter().rev().find(|s| s.kind != SK::For).is_some_and(|s| s.kind == SK::Eval) {
                    return ALLOC_node_line(line, ALLOC_node_special(SP_EVAL_RETURN, v));
                }
                ALLOC_node_return(v)
            }
            Kind::CaseNode { predicate, conditions, else_clause } => self.case_when(n, predicate.as_deref(), conditions, else_clause.as_deref(), line),
            Kind::CaseMatchNode { predicate, conditions, else_clause } => self.case_in(n, predicate.as_deref(), conditions, else_clause.as_deref(), line),
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
                let a = match left { Some(x) => self.ff_cond(x), None => ALLOC_node_nil() };
                let b1 = match right { Some(x) => self.ff_cond(x), None => ALLOC_node_nil() };
                let off = self.var_write(nm, depth, ALLOC_node_false());
                let on_state = self.seq(vec![ALLOC_node_if(b1, off, ALLOC_node_nil()), ALLOC_node_true()]);
                let on = self.var_write(nm, depth, ALLOC_node_true());
                let start = if excl { on } else {
                    let b2 = match right { Some(x) => self.ff_cond(x), None => ALLOC_node_nil() };
                    ALLOC_node_if(b2, ALLOC_node_nil(), on)
                };
                let off_state = ALLOC_node_if(a, self.seq(vec![start, ALLOC_node_true()]), ALLOC_node_false());
                let st = self.var_read(nm, depth);
                ALLOC_node_if(st, on_state, off_state)
            }
            // calls
            Kind::CallNode { receiver, name, arguments, block, message_start } => {
                let nm = self.cst(*name);
                // a call is at its message's line (CRuby: `x\n  .foo` raises at line 2)
                let m = *message_start;
                let next = self.pr.line_offsets.get((line as i32 - self.pr.start_line) as usize + 1).copied().unwrap_or(u32::MAX);
                let line = if m != u32::MAX && m >= next { self.pr.line_of(m) } else { line };
                self.call(n, receiver.as_deref(), nm, arguments.as_deref(), block.as_deref(), line)
            }
            Kind::SuperNode { arguments, block } => self.super_call(arguments.as_deref(), block.as_deref(), false, line),
            Kind::ForwardingSuperNode { block } => self.super_call(None, block.as_deref(), true, line),
            Kind::YieldNode { .. } if self.yield_invalid() => {
                self.errors.push(format!("{}:{}: Invalid yield", self.file, line));
                ALLOC_node_nil()
            }
            Kind::YieldNode { arguments } => {
                self.yields = true;
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
                let mut pre = vec![];
                let v = self.c(value);
                let splat_rhs = matches!(value.kind, Kind::ArrayNode { .. });
                let m = self.masgn_h(lefts, rest.as_deref(), rights, v, splat_rhs, Some(&mut pre));
                let m = ALLOC_node_line(line, m);
                if pre.is_empty() { m } else { pre.push(m); self.seq(pre) }
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
                is.arrow.set(true);
                ALLOC_node_lambda(is)
            }
            Kind::PreExecutionNode { .. } => ALLOC_node_nil(),
            Kind::PostExecutionNode { statements } => {
                // END { } → at_exit { }
                // no scope of its own in prism: like a `for` body it shares the enclosing variables
                let mut sc = Scope::new(SK::For, HashSet::default());
                sc.transparent = true;
                self.scopes.push(sc);
                let b = self.stmts(statements.as_deref());
                let is = self.finish_iseq(b, Params::default(), IseqKind::Block, self.block_label(), line, Sym(0));
                is.end_once.set(1);
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
        // rb_str_concat_literals: a result of 2+ parts starting with an interpolation takes the first part's encoding (flag 0x100)
        let lead = v.len() >= 2 && !std::ptr::eq(crate::node::node_kind(v[0]), &KIND_node_fstr);
        ALLOC_node_dstr(leak(v), self.src_enc | if lead { 0x100 } else { 0 }, cap as u32)
    }
    /// Bytes of an all-literal string concatenation whose parts share one encoding.
    fn static_concat(&self, parts: &[PNode]) -> Option<(Vec<u8>, u32)> {
        let mut b = vec![]; let mut enc = None;
        for p in parts {
            let Kind::StringNode { unescaped } = &p.kind else { return None };
            let e = self.lit_enc(p.flags);
            if *enc.get_or_insert(e) != e { return None; }
            b.extend_from_slice(unescaped);
        }
        Some((b, enc?))
    }
    fn dstr_parts(&mut self, parts: &[PNode], v: &mut Vec<NodePtr>) {
        for p in parts {
            match &p.kind {
                Kind::StringNode { unescaped } => {
                    let enc = self.lit_enc(p.flags);
                    v.push(ALLOC_node_fstr(lstr(unescaped), enc))
                }
                Kind::InterpolatedStringNode { parts } => {
                    // a lone "#{x}" is a UTF-8 String of its own in CRuby
                    if parts.len() == 1 && !matches!(parts[0].kind, Kind::StringNode { .. }) { v.push(ALLOC_node_fstr(lstr(b""), self.src_enc as u32)); }
                    self.dstr_parts(parts, v)
                }
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
    /// Index arguments with splats as one Array (a[*x] op= v, a[*x], b = ...).
    fn array_refs(&mut self, elements: &[&PNode]) -> NodePtr {
        let mut v = vec![]; let mut k = vec![];
        for e in elements {
            match &e.kind {
                Kind::SplatNode { expression } => {
                    v.push(match expression { Some(x) => self.c(x), None => self.anon_var(b"__fwd_rest") }); k.push(1u8);
                }
                _ => { v.push(self.c(e)); k.push(0); }
            }
        }
        ALLOC_node_ary_build(leak(v), lstr(&k))
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
            Kind::ConstantPathNode { parent: None, .. } => (ALLOC_node_top_const(intern(b"Object"), self.line(cp.loc)), true),
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
    /// The read of an op-write's target path: its parent at its own node, the path at the write's (CRuby).
    fn cpath_read(&mut self, t: &PNode, line: u32) -> NodePtr {
        match &t.kind {
            Kind::ConstantPathNode { parent: Some(p), name } => { let pv = self.c(p); ALLOC_node_const_path(pv, self.cst(name.unwrap_or(0)), line) }
            Kind::ConstantPathNode { parent: None, name } => ALLOC_node_top_const(self.cst(name.unwrap_or(0)), line),
            _ => self.c(t),
        }
    }
    /// P::C op= v: (P into a hidden slot, the slot, C), the read, and the writer.
    #[allow(clippy::type_complexity)]
    fn cpath_op_parts(&mut self, t: &PNode, line: u32) -> ((Option<NodePtr>, Option<u32>, Sym), NodePtr, Box<dyn Fn(NodePtr) -> NodePtr>) {
        match &t.kind {
            Kind::ConstantPathNode { parent: Some(p), name } => {
                let nm = self.cst(name.unwrap_or(0));
                let sl = self.hidden_slot();
                let pv = self.c(p);
                let rd = ALLOC_node_const_path(ALLOC_node_lvar(sl), nm, line);
                ((Some(ALLOC_node_lasgn(sl, pv)), Some(sl), nm), rd, Box::new(move |v| ALLOC_node_cdecl_path(ALLOC_node_lvar(sl), nm, v, line)))
            }
            _ => {
                let nm = match &t.kind { Kind::ConstantPathNode { name, .. } => self.cst(name.unwrap_or(0)), _ => Sym(0) };
                let rd = self.cpath_read(t, line);
                ((None, None, nm), rd, Box::new(move |v| ALLOC_node_cdecl_path(ALLOC_node_lit(UNDEF), nm, v, line)))
            }
        }
    }
    fn cpath_write(&mut self, target: &PNode, v: NodePtr, line: u32) -> NodePtr {
        let (p, nm) = self.cpath_parts(target);
        ALLOC_node_cdecl_path(p, nm, v, line)
    }

    // ---- calls ---------------------------------------------------------------------------

    fn fcall(&mut self, name: &str, args: Vec<NodePtr>, line: u32) -> NodePtr {
        ALLOC_node_call(ALLOC_node_self(), leak(args), intern(name.as_bytes()), CF_FCALL, line)
    }
    fn call_node(&mut self, recv: NodePtr, mid: Sym, args: Vec<NodePtr>, flags: u32, line: u32) -> NodePtr {
        ALLOC_node_call(recv, leak(args), mid, flags, line)
    }

    fn binop(&mut self, a: NodePtr, op: Sym, b: NodePtr, line: u32) -> NodePtr {
        if self.trace { return self.call_node(a, op, vec![b], 0, line); }
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
                    Kind::SymbolNode { unescaped } => { let s = self.sym_lit(unescaped, key.flags); if names.contains(&s) { return None; } names.push(s); }
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

    /// `recv.send(:m, args...)`: recv.m(args...) as a private-allowed call while recv's #send is Kernel's (node_send_sym).
    /// Receiver and arguments are evaluated first, into hidden slots unless reading them twice is the same.
    fn send_sym(&mut self, recv: Option<&PNode>, smid: Sym, m: Sym, args: &[&PNode], flags: u32, line: u32) -> NodePtr {
        let mut pre = vec![];
        let mut twice = |this: &mut Self, a: Option<&PNode>| -> [NodePtr; 3] {
            match a {
                None => [ALLOC_node_self(), ALLOC_node_self(), ALLOC_node_self()],
                Some(a) if matches!(a.kind, Kind::IntegerNode { .. } | Kind::FloatNode { .. } | Kind::SymbolNode { .. } | Kind::NilNode
                    | Kind::TrueNode | Kind::FalseNode | Kind::SelfNode | Kind::LocalVariableReadNode { .. }) => [this.c(a), this.c(a), this.c(a)],
                Some(a) => {
                    let sl = this.hidden_slot();
                    let v = this.c(a);
                    pre.push(ALLOC_node_lasgn(sl, v));
                    [ALLOC_node_lvar(sl), ALLOC_node_lvar(sl), ALLOC_node_lvar(sl)]
                }
            }
        };
        let [r0, r1, r2] = twice(self, recv);
        let (mut direct, mut general) = (vec![], vec![ALLOC_node_lit(Value::sym(m))]);
        for a in args { let [x, y, _] = twice(self, Some(a)); direct.push(x); general.push(y); }
        let call = ALLOC_node_call(r1, leak(direct), m, (flags | CF_FCALL) & !CF_VCALL, line);
        let gen = ALLOC_node_call(r2, leak(general), smid, flags, line);
        pre.push(ALLOC_node_send_sym(r0, smid, call, gen));
        if pre.len() == 1 { pre.pop().unwrap() } else { self.seq(pre) }
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
        if (mid == s::send || mid == s::__send__) && block.is_none() && !safe && !attr && args.iter().all(|a| self.simple_arg(a)) {
            if let Some((Kind::SymbolNode { unescaped }, f)) = args.first().map(|a| (&a.kind, a.flags)) {
                return self.send_sym(recv, mid, self.sym_lit(unescaped, f), &args[1..], flags, line);
            }
        }
        // [a, b].max / .min of a literal built at run time (no splat): CRuby's opt_newarray_send
        if (mid == s::max || mid == s::min) && args.is_empty() && block.is_none() && !safe {
            if let Some(PNode { kind: Kind::ArrayNode { elements }, flags: rf, .. }) = recv {
                let st = |e: &PNode| e.flags & pa::FLAG_STATIC_LITERAL != 0;
                // a run of 0x40 static elements is built by duparray + concat (pm_compile_array_node)
                let dup_run = (0..elements.len()).any(|i| i + 0x40 < elements.len() && (i == 0 || !st(&elements[i - 1])) && elements[i..i + 0x40].iter().all(st));
                if rf & pa::FLAG_STATIC_LITERAL == 0 && elements.len() < 0x100 && !dup_run
                    && !elements.iter().any(|e| matches!(e.kind, Kind::SplatNode { .. })) {
                    let a = self.array(elements);
                    return ALLOC_node_newarray_minmax(a, (mid == s::max) as u32, line);
                }
            }
        }
        // receiver
        let sl = if safe { Some(self.hidden_slot()) } else { None };
        let recv_node = match recv {
            // "lit".freeze / -"lit": the receiver is the interned literal
            Some(PNode { kind: Kind::StringNode { unescaped }, flags: rf, .. }) if args.is_empty() && block.is_none() && (mid == s::freeze || mid == s::uminus) => {
                let enc = self.lit_enc(*rf);
                ALLOC_node_fstr_recv(lstr(unescaped), enc, (mid == s::uminus) as u32)
            }
            Some(r) => self.c(r),
            None => ALLOC_node_self(),
        };
        let (recv_node, pre) = match sl { Some(sl) => (ALLOC_node_lvar(sl), Some(ALLOC_node_lasgn(sl, recv_node))), None => (recv_node, None) };
        // branch coverage of `&.`: the call without its block
        let qcall = if sl.is_some() && self.cov.is_some() {
            // the later of the closing paren / arguments when there are arguments, else the message (CRuby 4.0)
            let (ms, an) = match &n.kind { Kind::CallNode { message_start, arguments, .. } => (*message_start, arguments.as_deref()), _ => (n.loc.start, None) };
            let mut me = ms as usize;
            while self.src.get(me).is_some_and(|&b| b.is_ascii_alphanumeric() || b == b'_' || b >= 0x80) { me += 1; }
            if self.src.get(me).is_some_and(|&b| b == b'?' || b == b'!') { me += 1; }
            // an operator message (`&.[]`, `&.+`)
            if me == ms as usize {
                if self.src.get(me) == Some(&b'[') { me += 2; if self.src.get(me) == Some(&b'=') && self.src.get(me + 1) != Some(&b'=') { me += 1; } }
                else { while self.src.get(me).is_some_and(|b| b"=+-*/%<>!~^&|".contains(b)) { me += 1; } }
            }
            let end = match an {
                Some(a) if self.src.get(me) == Some(&b'(') => {
                    let mut end = match block { Some(b @ PNode { kind: Kind::BlockNode { .. }, .. }) => b.loc.start, _ => n.loc.start + n.loc.len };
                    while end > n.loc.start && self.src.get(end as usize - 1).is_some_and(|b| b.is_ascii_whitespace()) { end -= 1; }
                    end.max(a.loc.start + a.loc.len)
                }
                Some(a) => a.loc.start + a.loc.len,
                None => me as u32,
            };
            let p = self.pos4(n.loc.start, end);
            self.cov.and_then(|f| crate::builtins::coverage::branch_base(f, "&.", p)).map(|b| (b, p))
        } else { None };
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
        } else if simple && !self.trace && blk_iseq.is_none() && recv.is_some() && args.is_empty() && mid == s::not && !safe {
            ALLOC_node_not(recv_node)
        } else if simple && !self.trace && blk_iseq.is_none() && recv.is_some() && args.len() == 2 && mid == s::aset && flags & CF_FCALL == 0 && !safe {
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
                let (bb, p) = match qcall { Some((b, p)) => (Some(b), p), None => (None, [0; 4]) };
                let node = self.btarget(bb, "then", |_| p, node);
                let nil = self.btarget(bb, "else", |_| p, ALLOC_node_nil());
                let body = ALLOC_node_if(test, nil, node);
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
            // eval "super" in a method's binding: its parameters are the binding's outer variables
            let ev = if mi.is_none() { EVAL_ZSUPER.with(|z| z.borrow().clone()) } else { None };
            if mi.is_some() || ev.is_some() {
                let (zs, depth) = match mi {
                    Some(mi) => (self.scopes[mi].zsuper.clone(), self.scopes[mi + 1..].iter().filter(|s| !s.transparent).count() as u32),
                    None => (ev.unwrap_or_default(), self.scopes.iter().filter(|s| !s.transparent).count() as u32),
                };
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
                // known at run time: a define_method block's (RuntimeError) or none (NoMethodError)
                return ALLOC_node_line(line, ALLOC_node_special(SP_ZSUPER_NO_METHOD, ALLOC_node_nil()));
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
        // self[...] op= v: private #[] / #[]= are allowed
        let fl = if recv.is_none_or(|r| matches!(r.kind, Kind::SelfNode)) { CF_FCALL } else { 0 };
        let mut pre = vec![ALLOC_node_lasgn(rs, r)];
        let list: Vec<&PNode> = match args { Some(PNode { kind: Kind::ArgumentsNode { arguments }, .. }) => arguments.iter().collect(), _ => vec![] };
        if !list.iter().all(|a| self.simple_arg(a)) {
            // a[*x] op= v: the index list is built (and splatted) once
            let sl = self.hidden_slot();
            let ix = self.array_refs(&list);
            pre.push(ALLOC_node_lasgn(sl, ix));
            let ci = Box::leak(Box::new(CallInfo { kinds: vec![AK_SPLAT], blk: None, kw_splat_only: false, kw_names: vec![] }));
            let rd = ALLOC_node_call_x(ALLOC_node_lvar(rs), leak(vec![ALLOC_node_lvar(sl)]), s::aref, fl, line, ci);
            let mk = Box::new(move |_s: &mut Self, v: NodePtr| {
                let ci = Box::leak(Box::new(CallInfo { kinds: vec![AK_SPLAT, AK_ARG], blk: None, kw_splat_only: false, kw_names: vec![] }));
                ALLOC_node_call_x(ALLOC_node_lvar(rs), leak(vec![ALLOC_node_lvar(sl), v]), s::aset, fl | CF_ATTR, line, ci)
            });
            return (pre, rd, mk);
        }
        let mut slots = vec![];
        for a in list { let sl = self.hidden_slot(); let v = self.c(a); pre.push(ALLOC_node_lasgn(sl, v)); slots.push(sl); }
        let rd = if slots.len() == 1 && fl == 0 { ALLOC_node_opt_aref(ALLOC_node_lvar(rs), ALLOC_node_lvar(slots[0]), line) }
                 else { ALLOC_node_call(ALLOC_node_lvar(rs), leak(slots.iter().map(|&s| ALLOC_node_lvar(s)).collect()), s::aref, fl, line) };
        let slots2 = slots.clone();
        let mk = Box::new(move |_s: &mut Self, v: NodePtr| {
            let mut a: Vec<NodePtr> = slots2.iter().map(|&s| ALLOC_node_lvar(s)).collect();
            a.push(v);
            ALLOC_node_attrasgn(ALLOC_node_lvar(rs), leak(a), s::aset, fl, line)
        });
        (pre, rd, mk)
    }

    // ---- multiple assignment ----------------------------------------------------------------

    fn masgn(&mut self, lefts: &[PNode], rest: Option<&PNode>, rights: &[PNode], rhs: NodePtr, splat_rhs: bool) -> NodePtr {
        self.masgn_h(lefts, rest, rights, rhs, splat_rhs, None)
    }
    /// `pre` (a statement's masgn): target receivers and index arguments are evaluated into it, before the rhs (CRuby).
    fn masgn_h(&mut self, lefts: &[PNode], rest: Option<&PNode>, rights: &[PNode], rhs: NodePtr, splat_rhs: bool,
               mut pre: Option<&mut Vec<NodePtr>>) -> NodePtr {
        let tmp = self.hidden_slot();
        let mut targets = vec![];
        let post = rights.len() as u32;
        for (i, t) in lefts.iter().enumerate() {
            let e = ALLOC_node_masgn_elem(tmp, i as u32, 0, post);
            targets.push(self.target_h(t, e, pre.as_deref_mut()));
        }
        if let Some(r) = rest {
            if let Kind::SplatNode { expression: Some(x) } = &r.kind {
                let e = ALLOC_node_masgn_elem(tmp, lefts.len() as u32, 1, post);
                targets.push(self.target_h(x, e, pre.as_deref_mut()));
            }
        }
        for (i, t) in rights.iter().enumerate() {
            let e = ALLOC_node_masgn_elem(tmp, i as u32, 2 + lefts.len() as u32, post);
            targets.push(self.target_h(t, e, pre.as_deref_mut()));
        }
        let _ = splat_rhs;
        ALLOC_node_masgn(rhs, tmp, leak(targets), false)
    }

    /// x as is when re-reading it is the same; else evaluated into `pre` and read from a hidden slot.
    fn hoisted(&mut self, x: &PNode, pre: &mut Option<&mut Vec<NodePtr>>) -> NodePtr {
        let v = self.c(x);
        match pre {
            Some(p) if !matches!(x.kind, Kind::SelfNode | Kind::IntegerNode { .. } | Kind::SymbolNode { .. } | Kind::NilNode
                                 | Kind::TrueNode | Kind::FalseNode) => {
                let sl = self.hidden_slot();
                p.push(ALLOC_node_lasgn(sl, v));
                ALLOC_node_lvar(sl)
            }
            _ => v,
        }
    }

    /// Assignment of v to a target node (masgn / for / rescue => e).
    fn target(&mut self, t: &PNode, v: NodePtr) -> NodePtr { self.target_h(t, v, None) }
    fn target_h(&mut self, t: &PNode, v: NodePtr, mut pre: Option<&mut Vec<NodePtr>>) -> NodePtr {
        let line = self.line(t.loc);
        match &t.kind {
            Kind::LocalVariableTargetNode { name, depth } => { let nm = self.cst(*name); self.var_write(nm, *depth, v) }
            Kind::LocalVariableReadNode { name, depth } => { let nm = self.cst(*name); self.var_write(nm, *depth, v) }
            Kind::RequiredParameterNode { name } => { let nm = self.cst(*name); self.var_write(nm, 0, v) }
            Kind::InstanceVariableTargetNode { name } => ALLOC_node_iasgn(self.cst(*name), v),
            Kind::GlobalVariableTargetNode { name } => ALLOC_node_gasgn(self.cst(*name), v),
            Kind::ClassVariableTargetNode { name } => ALLOC_node_cvasgn(self.cst(*name), v),
            Kind::ConstantTargetNode { name } => ALLOC_node_cdecl(self.cst(*name), v, line),
            Kind::ConstantPathTargetNode { parent: Some(p), name } => {
                let pv = self.hoisted(p, &mut pre);
                ALLOC_node_cdecl_path(pv, self.cst(name.unwrap_or(0)), v, line)
            }
            Kind::ConstantPathTargetNode { .. } => self.cpath_write(t, v, line),
            Kind::CallTargetNode { receiver, name } => {
                let r = self.hoisted(receiver, &mut pre);
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
                let r = self.hoisted(receiver, &mut pre);
                let fl = if matches!(receiver.kind, Kind::SelfNode) { CF_FCALL } else { 0 };
                let list: Vec<&PNode> = match arguments.as_deref() { Some(PNode { kind: Kind::ArgumentsNode { arguments }, .. }) => arguments.iter().collect(), _ => vec![] };
                if list.iter().all(|a| self.simple_arg(a)) {
                    let mut a: Vec<NodePtr> = list.iter().map(|x| self.hoisted(x, &mut pre)).collect();
                    a.push(v);
                    return ALLOC_node_attrasgn(r, leak(a), s::aset, fl, line);
                }
                // a[*x] = v: the index list as one splatted Array
                let ix = self.array_refs(&list);
                let a = match pre.as_deref_mut() {
                    Some(p) => { let sl = self.hidden_slot(); p.push(ALLOC_node_lasgn(sl, ix)); ALLOC_node_lvar(sl) }
                    None => ix,
                };
                let ci = Box::leak(Box::new(CallInfo { kinds: vec![AK_SPLAT, AK_ARG], blk: None, kw_splat_only: false, kw_names: vec![] }));
                ALLOC_node_call_x(r, leak(vec![a, v]), s::aset, fl | CF_ATTR, line, ci)
            }
            Kind::MultiTargetNode { lefts, rest, rights } => self.masgn_h(lefts, rest.as_deref(), rights, v, false, pre),
            Kind::SplatNode { expression: Some(x) } => self.target_h(x, v, pre),
            _ => self.unsupported(pa::kind_name(&t.kind), t.loc),
        }
    }

    // ---- case ------------------------------------------------------------------------------

    fn case_when(&mut self, case: &PNode, pred: Option<&PNode>, conds: &[PNode], els: Option<&PNode>, _line: u32) -> NodePtr {
        let pv = pred.map(|p| self.c(p));
        let bb = self.bbase("case", case);
        let (mut bodies, mut tail) = self.clause_bodies(bb, "when", case, conds, els, ALLOC_node_nil());
        match pred {
            Some(_) => {
                let sl = self.hidden_slot();
                let set = ALLOC_node_lasgn(sl, pv.unwrap());
                for w in conds.iter().rev() {
                    let body = bodies.pop().unwrap();
                    if let Kind::WhenNode { conditions, .. } = &w.kind {
                        // one test per run of patterns on one line: === raises from its pattern's line
                        let mut groups: Vec<(u32, Vec<NodePtr>, Vec<u8>)> = vec![];
                        for cnd in conditions {
                            let l = self.line(cnd.loc);
                            if groups.last().is_none_or(|g| g.0 != l) { groups.push((l, vec![], vec![])); }
                            let g = groups.last_mut().unwrap();
                            match &cnd.kind {
                                Kind::SplatNode { expression: Some(x) } => { g.1.push(self.c(x)); g.2.push(1u8); }
                                _ => { g.1.push(self.c(cnd)); g.2.push(0); }
                            }
                        }
                        let mut test: Option<NodePtr> = None;
                        for (l, pats, kinds) in groups.into_iter().rev() {
                            let t = match lit_imm(&pats, &kinds) {
                                Some(v) => ALLOC_node_when_lit(sl, v, l),
                                None => ALLOC_node_when(sl, leak(pats), lstr(&kinds), l),
                            };
                            test = Some(match test { None => t, Some(rest) => ALLOC_node_or(t, rest) });
                        }
                        tail = ALLOC_node_if(test.unwrap_or_else(ALLOC_node_false), body, tail);
                    }
                }
                self.seq(vec![set, tail])
            }
            None => {
                for w in conds.iter().rev() {
                    let body = bodies.pop().unwrap();
                    if let Kind::WhenNode { conditions, .. } = &w.kind {
                        let mut test: Option<NodePtr> = None;
                        for cnd in conditions.iter().rev() {
                            // `when *a` without a subject: any element true
                            let cv = match &cnd.kind {
                                Kind::SplatNode { expression: Some(x) } => { let a = self.c(x); ALLOC_node_special(SP_SPLAT_ANY, a) }
                                _ => self.c(cnd),
                            };
                            test = Some(match test { None => cv, Some(t) => ALLOC_node_or(cv, t) });
                        }
                        tail = ALLOC_node_if(test.unwrap_or_else(ALLOC_node_false), body, tail);
                    }
                }
                tail
            }
        }
    }

    fn case_in(&mut self, case: &PNode, pred: Option<&PNode>, conds: &[PNode], els: Option<&PNode>, line: u32) -> NodePtr {
        let sl = self.hidden_slot();
        let pv = match pred { Some(p) => self.c(p), None => ALLOC_node_nil() };
        let set = ALLOC_node_lasgn(sl, pv);
        // the subject's #deconstruct is called once for all its array / find patterns
        let dsl = self.hidden_slot();
        let saved = self.pm_dcache.replace((sl, dsl));
        let r = self.case_in_body(case, conds, els, line, sl);
        self.pm_dcache = saved;
        self.seq(vec![set, ALLOC_node_lasgn(dsl, ALLOC_node_nil()), r])
    }
    fn case_in_body(&mut self, case: &PNode, conds: &[PNode], els: Option<&PNode>, _line: u32, sl: u32) -> NodePtr {
        let bb = self.bbase("case", case);
        // under Coverage the arms compile in source order (the line events follow it)
        if bb.is_some() || self.cov.is_some() {
            let (mut bodies, mut tail) = self.clause_bodies(bb, "in", case, conds, els, ALLOC_node_special(SP_NOMATCH, ALLOC_node_lvar(sl)));
            for w in conds.iter().rev() {
                let body = bodies.pop().unwrap();
                if let Kind::InNode { pattern, .. } = &w.kind { let test = self.pattern(pattern, sl); tail = ALLOC_node_if(test, body, tail); }
            }
            return tail;
        }
        if let (None, [PNode { kind: Kind::InNode { pattern, statements }, .. }]) = (els, conds) {
            // one clause, no else: the error says why it did not match
            let (test, fail) = self.single_pattern(pattern, sl);
            let body = self.stmts(statements.as_deref());
            return ALLOC_node_if(test, body, fail);
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
        tail
    }

    /// sl's #deconstruct: from the case's cache when sl is its subject.
    fn deconstruct(&mut self, sl: u32) -> NodePtr {
        let dec = ALLOC_node_special(SP_DECONSTRUCT, ALLOC_node_lvar(sl));
        match self.pm_dcache {
            Some((s, d)) if s == sl => ALLOC_node_or(ALLOC_node_lvar(d), ALLOC_node_lasgn(d, dec)),
            _ => dec,
        }
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
        let line = self.line(p.loc);
        if self.pm_err.is_none() { return self.eqq_test(v, sl, line); }
        // keep the pattern value for the message (evaluated once)
        let psl = self.hidden_slot();
        let t = self.eqq_test(ALLOC_node_lasgn(psl, v), sl, line);
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
                let dec = self.deconstruct(sl);
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
                let dec = self.deconstruct(sl);
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
                    Kind::AssocNode { key, .. } => match &key.kind { Kind::SymbolNode { unescaped } => Some(ALLOC_node_lit(Value::sym(self.sym_lit(unescaped, key.flags)))), _ => None },
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
                            let k = Value::sym(self.sym_lit(unescaped, key.flags));
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

    fn eqq_test(&mut self, pat: NodePtr, sl: u32, line: u32) -> NodePtr {
        ALLOC_node_when(sl, leak(vec![pat]), lstr(&[0]), line)
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
        while let Some(rn @ PNode { kind: Kind::RescueNode { exceptions, reference, statements, subsequent }, .. }) = cur {
            let mut cls = vec![]; let mut kinds = vec![];
            for e in exceptions {
                match &e.kind {
                    Kind::SplatNode { expression: Some(x) } => { cls.push(self.c(x)); kinds.push(1u8); }
                    _ => { cls.push(self.c(e)); kinds.push(0); }
                }
            }
            let assign = match reference { Some(t) => self.target(t, ALLOC_node_errinfo()), None => ALLOC_node_nil() };
            let body = self.stmts(statements.as_deref());
            let body = if self.trace { ALLOC_node_trace(EV_RESCUE, self.line(rn.loc), body) } else { body };
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
            Kind::ConstantPathNode { parent, name } => {
                // an undefined scope makes it undefined (no const_missing)
                let p = match parent { Some(p) => self.defined_guard(p), None => ALLOC_node_top_const(intern(b"Object"), self.line(v.loc)) };
                ALLOC_node_defined(DEF_CPATH, self.cst(name.unwrap_or(0)), p)
            }
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
            Kind::CallNode { receiver: _, .. } | Kind::ConstantReadNode { .. } | Kind::ConstantPathNode { .. } | Kind::InstanceVariableReadNode { .. }
            | Kind::GlobalVariableReadNode { .. } | Kind::ClassVariableReadNode { .. } => {
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
        let mut sc = Scope::new(SK::For, HashSet::default());
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

    fn top_label(&self) -> &'static str { if self.required { "<top (required)>" } else { "<main>" } }
    fn method_label(&self) -> String {
        for sc in self.scopes.iter().rev() {
            match sc.kind {
                SK::Method => return METHOD_LABEL.with(|m| m.borrow().last().cloned().unwrap_or_default()),
                SK::Class => return CLASS_LABEL.with(|m| m.borrow().last().cloned().unwrap_or_default()),
                SK::Top => return self.top_label().into(),
                SK::Eval => return EVAL_LABEL.with(|l| l.borrow().clone()).unwrap_or_else(|| "<main>".into()),
                _ => {}
            }
        }
        "<main>".into()
    }

    // ---- scopes with code -------------------------------------------------------------------

    fn compile_def(&mut self, n: &PNode, name: Sym, params: Option<&PNode>, body: Option<&PNode>, locals: &[u32], line: u32) -> &'static Iseq {
        let yields = std::mem::take(&mut self.yields);
        let is = self.compile_def_body(n, name, params, body, locals, line);
        self.yields = yields;
        is
    }
    fn compile_def_body(&mut self, n: &PNode, name: Sym, params: Option<&PNode>, body: Option<&PNode>, locals: &[u32], line: u32) -> &'static Iseq {
        let saved = std::mem::take(&mut self.scopes);
        let mut bodies: Vec<&PNode> = vec![];
        if let Some(p) = params { bodies.push(p); }
        if let Some(b) = body { bodies.push(b); }
        self.analyze(n, &bodies);
        self.push_scope(SK::Method, n, locals);
        METHOD_LABEL.with(|m| m.borrow_mut().push(crate::symbol::sym_str(name)));
        self.in_def += 1;
        let ps = self.params(params, false);
        // `def m = x`: no line of its own (it ends where the def does)
        let endless = body.is_some_and(|b| b.loc.start + b.loc.len == n.loc.start + n.loc.len);
        let b = self.stmts_flat(body, endless);
        self.in_def -= 1;
        let end = n.loc.start + n.loc.len;
        let mslot = self.cov.and_then(|f| crate::builtins::coverage::method_slot(f, [line, self.pr.col_of(n.loc.start), self.pr.line_of(end), self.pr.col_of(end)]));
        let b = match mslot { Some(s) => ALLOC_node_cov(s, b), None => b };
        let is = self.finish_iseq(b, ps, IseqKind::Method, crate::symbol::sym_str(name), line, name);
        if let Some(s) = mslot { crate::builtins::coverage::bind_iseq(is as *const Iseq as usize, s); }
        METHOD_LABEL.with(|m| m.borrow_mut().pop());
        self.scopes = saved;
        is
    }

    fn compile_body(&mut self, n: &PNode, locals: &[u32], body: Option<&PNode>, kind: IseqKind, label: String, line: u32) -> &'static Iseq {
        let yields = std::mem::take(&mut self.yields);
        let saved = std::mem::take(&mut self.scopes);
        let bodies: Vec<&PNode> = body.into_iter().collect();
        self.analyze(n, &bodies);
        self.push_scope(SK::Class, n, locals);
        CLASS_LABEL.with(|m| m.borrow_mut().push(label.clone()));
        let b = self.stmts(body);
        CLASS_LABEL.with(|m| m.borrow_mut().pop());
        let is = self.finish_iseq(b, Params::default(), kind, label, line, Sym(0));
        self.scopes = saved;
        self.yields = yields;
        is
    }

    /// A block's iseq; its uses_block (a yield in it or its inner blocks) also counts for the enclosing iseq.
    fn compile_block(&mut self, n: &PNode, locals: &[u32], params: Option<&PNode>, body: Option<&PNode>, kind: IseqKind, line: u32) -> &'static Iseq {
        let outer = std::mem::take(&mut self.yields);
        let is = self.compile_block_body(n, locals, params, body, kind, line);
        self.yields |= outer;
        is
    }
    fn compile_block_body(&mut self, n: &PNode, locals: &[u32], params: Option<&PNode>, body: Option<&PNode>, kind: IseqKind, line: u32) -> &'static Iseq {
        // the block's own node (its entry and frame line: error_highlight's block_node), not the call's
        let saved = if self.tag { node_tag_swap(n.id) } else { 0 };
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
        if !self.outer.is_empty() {
            // in CRuby's order (ID order ~ first appearance in the source)
            if let Some(mut ov) = self.outer.remove(&key(n)) { ov.sort_unstable(); ov.dedup_by_key(|x| x.1); self.pending_outer = ov.into_iter().map(|x| x.1).collect(); }
        }
        let is = self.finish_iseq(b, ps, kind, label, line, Sym(0));
        // the block's home frame: `return` in the block targets it by id
        if let Some(p) = self.scopes.last_mut() { p.needs_id = true; }
        if self.tag { node_tag_swap(saved); }
        is
    }

    /// The frame slot that receives a parameter.  A captured parameter gets a hidden
    /// slot and is copied into the env after setup.
    fn param_slot(&mut self, nm: Sym) -> u32 {
        if !self.cur().params_seen.insert(nm) {
            // a repeated `_`: its own slot, named for zsuper
            let sl = self.hidden_slot();
            self.cur().param_names.push((sl, nm));
            let hn = intern(format!("__dup{}", sl).as_bytes());
            self.cur().vars.insert(hn, VarLoc::Slot(sl));
            return sl;
        }
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

    /// The name zsuper reads parameter `nm` in slot `sl` by (a repeated one's own).
    fn zname(&mut self, nm: Sym, sl: u32) -> Sym {
        let sc = self.cur();
        // no repeated name in this parameter list (the common case)
        if sc.param_names.len() == sc.params_seen.len() { return nm; }
        let first = match sc.vars.get(&nm) {
            Some(&VarLoc::Slot(s)) => s == sl,
            Some(&VarLoc::Env(_)) => sc.param_env.iter().any(|&(n, s)| n == nm && s == sl),
            None => true,
        };
        if first { nm } else { intern(format!("__dup{}", sl).as_bytes()) }
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
                Kind::RequiredParameterNode { name } => { let nm = self.cst(*name); let sl = self.param_slot(nm); ps.lead.push(sl); let z = self.zname(nm, sl); zs.push((AK_ARG, z)); }
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
                let z = self.zname(nm, sl);
                zs.push((AK_ARG, z));
            }
        }
        if let Some(r) = rest {
            match &r.kind {
                Kind::RestParameterNode { name } => {
                    let nm = match name { Some(n) => self.cst(*n), None => { ps.anon_rest = true; intern(b"__fwd_rest") } };
                    if name.is_none() { self.declare(nm); }
                    let sl = self.param_slot(nm);
                    ps.rest = Some(sl);
                    let z = self.zname(nm, sl);
                    zs.push((AK_SPLAT, z));
                }
                Kind::ImplicitRestNode => { ps.has_trailing_comma = true; }
                _ => {}
            }
        }
        for r in posts {
            match &r.kind {
                Kind::RequiredParameterNode { name } => { let nm = self.cst(*name); let sl = self.param_slot(nm); ps.post.push(sl); let z = self.zname(nm, sl); zs.push((AK_ARG, z)); }
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
        .any(|w| matches!(w, "node_fstr" | "node_fstr_recv" | "node_regex" | "node_dregex" | "node_biglit" | "node_range_lit"))
}

fn leaf_body(body: NodePtr) -> bool {
    let mut d = String::new();
    dump_node(&mut d, body);
    d.split(|ch: char| !(ch.is_alphanumeric() || ch == '_')).filter(|w| w.starts_with("node_"))
        .all(|w| matches!(w, "node_nil" | "node_true" | "node_false" | "node_self" | "node_lit" | "node_lvar" | "node_ivar" | "node_block" | "node_if" | "node_and" | "node_or"))
}

/// No call site in the body (operators call only on their slow path).
fn nocall_body(body: NodePtr) -> bool {
    let mut d = String::new();
    dump_node(&mut d, body);
    d.split(|ch: char| !(ch.is_alphanumeric() || ch == '_')).filter(|w| w.starts_with("node_"))
        .all(|w| w.starts_with("node_opt_") || matches!(w, "node_nil" | "node_true" | "node_false" | "node_self" | "node_lit" | "node_lvar"
            | "node_lasgn" | "node_ivar" | "node_iasgn" | "node_block" | "node_line" | "node_if" | "node_and" | "node_or"))
}

/// A loop whose condition reads only variables, literals, constants and operators, one of the variables
/// a local its body assigns (a counter), ends or not by itself: it needs no time-slice check point
/// (node_poll).  A spin on another thread's state does, unless its condition always calls.
fn loop_needs_no_poll(cond: NodePtr, body: NodePtr) -> bool {
    let mut dc = String::new();
    dump_node(&mut dc, cond);
    let ok = dc.split(|ch: char| !(ch.is_alphanumeric() || ch == '_')).filter(|w| w.starts_with("node_"))
        .all(|w| w.starts_with("node_opt_") || matches!(w, "node_lvar" | "node_dvar" | "node_lit" | "node_const" | "node_true" | "node_false" | "node_nil" | "node_and" | "node_or" | "node_not" | "node_line"));
    if ok {
        let mut db = String::new();
        dump_node(&mut db, body);
        // "(node_lvar 3)" is written as "(node_lasgn 3 ...", "(node_dvar 1 2)" as "(node_dasgn 1 2 ..."
        let assigned = |kind: &str, set: &str| dc.split(kind).skip(1).filter_map(|r| r.split(')').next()).any(|v| db.contains(&format!("{}{} ", set, v)));
        return assigned("(node_lvar ", "(node_lasgn ") || assigned("(node_dvar ", "(node_dasgn ");
    }
    // a call site the condition always runs is a check point already (call_slow)
    dc.contains("(node_call ") && !["(node_and ", "(node_or ", "(node_if "].iter().any(|w| dc.contains(w))
}

/// Statements that can raise without passing through a node that records the line.
/// Globals whose read or write can warn: the warning names the line.
fn gvar_warns(nm: Sym) -> bool { matches!(crate::symbol::sym_bytes(nm), b"$/" | b"$-0" | b"$\\" | b"$," | b"$;" | b"$-F" | b"$=") }

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
pub const SP_ZSUPER_NO_METHOD: u32 = 18;
pub const SP_SPLAT_ANY: u32 = 19;
pub const SP_TOPLEVEL_BINDING: u32 = 20;
pub const SP_EVAL_RETURN: u32 = 21;

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

impl<'a> Compiler<'a> {
    /// A regexp literal (out of line: `c` is large).  An invalid one raises SyntaxError when run.
    #[inline(never)]
    fn regex_lit(&mut self, src: &[u8], flags: u32, line: u32) -> NodePtr {
        let s = lstr(src);
        if self.file != "<internal:prelude>" { crate::regexp::note_literal(s, self.file, line); }
        let opts = regex_opts(flags, self.src_enc);
        if self.file != "<internal:prelude>" {
            for m in crate::regexp::literal_warnings(s, opts) { PARSE_WARNINGS.with(|w| w.borrow_mut().push((format!("{}:{}: warning: {}\n", self.file, line, m), false))); }
        }
        ALLOC_node_regex(s, opts)
    }
}

/// i/x/m, /n, /e /s /u (fixed) and the source string's encoding (regexp.rs RE_LIT_ENC_SHIFT).
pub fn regex_opts(flags: u32, src_enc: u32) -> u32 {
    // FORCED_US_ASCII is left out: this prism sets it with non-ASCII group names
    use crate::object::{ENC_BINARY, ENC_UTF8};
    let mut o = 0;
    if flags & pa::REGULAR_EXPRESSION_FLAGS_IGNORE_CASE != 0 { o |= 1; }
    if flags & pa::REGULAR_EXPRESSION_FLAGS_EXTENDED != 0 { o |= 2; }
    if flags & pa::REGULAR_EXPRESSION_FLAGS_MULTI_LINE != 0 { o |= 4; }
    if flags & pa::REGULAR_EXPRESSION_FLAGS_ASCII_8BIT != 0 { o |= 32; }
    let kcode = pa::REGULAR_EXPRESSION_FLAGS_EUC_JP | pa::REGULAR_EXPRESSION_FLAGS_WINDOWS_31J | pa::REGULAR_EXPRESSION_FLAGS_UTF_8;
    if flags & kcode != 0 { o |= 16; }
    let enc = if flags & (pa::REGULAR_EXPRESSION_FLAGS_FORCED_BINARY_ENCODING | pa::REGULAR_EXPRESSION_FLAGS_ASCII_8BIT) != 0 { ENC_BINARY as u32 }
        else if flags & (pa::REGULAR_EXPRESSION_FLAGS_FORCED_UTF8_ENCODING | pa::REGULAR_EXPRESSION_FLAGS_UTF_8) != 0 { ENC_UTF8 as u32 }
        else if flags & pa::REGULAR_EXPRESSION_FLAGS_EUC_JP != 0 { crate::builtins::encoding::ENC_EUCJP as u32 }
        else if flags & pa::REGULAR_EXPRESSION_FLAGS_WINDOWS_31J != 0 { crate::builtins::encoding::find_enc(b"Windows-31J").unwrap_or(ENC_BINARY) as u32 }
        else { src_enc };
    o | ((enc + 1) << crate::regexp::RE_LIT_ENC_SHIFT)
}

thread_local! {
    /// An eval whose binding is not in a method: a yield in it is invalid (set around its compile).
    pub static EVAL_NO_YIELD: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// zsuper lists of methods that call eval / binding, by iseq address (iseqs are never freed).
    pub static METHOD_ZSUPER: std::cell::RefCell<std::collections::HashMap<usize, Vec<(u8, Sym)>>> = std::cell::RefCell::new(Default::default());
    /// An eval in a method's binding: that method's zsuper list (set around its compile).
    pub static EVAL_ZSUPER: std::cell::RefCell<Option<Vec<(u8, Sym)>>> = const { std::cell::RefCell::new(None) };
    /// An eval's frame label (its binding's frame's), set around its compile.
    pub static EVAL_LABEL: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
    /// The last parse's warnings (message, verbose-only), shown by flush_parse_warnings.
    static PARSE_WARNINGS: std::cell::RefCell<Vec<(String, bool)>> = const { std::cell::RefCell::new(Vec::new()) };
}
/// pm_warning_emit_callback: default-level warnings unless $VERBOSE is nil, verbose-level ones under $VERBOSE = true.
pub fn flush_parse_warnings(c: &mut Ctx) -> Result<(), Esc> {
    let ws = PARSE_WARNINGS.with(|w| std::mem::take(&mut *w.borrow_mut()));
    for (m, verbose_only) in ws {
        let v = c.vm().verbose.get();
        if v == NIL || (verbose_only && v != TRUE) { continue; }
        crate::builtins::kernel::warning_write(c, m.as_bytes())?;
    }
    Ok(())
}

/// A source that does not compile (pm_parse_process_error): prism's error level (0 SyntaxError,
/// 1 ArgumentError, 2 LoadError), the message and the name of its encoding (the source's).
/// `compile`: an error of the compiler (not the parser), whose backtrace is the path alone (finish_iseq_build).
pub struct CompileError { pub level: u8, pub msg: Vec<u8>, pub enc: String, pub compile: bool }
impl CompileError {
    fn syntax(msg: String) -> Self { CompileError { level: 0, msg: msg.into_bytes(), enc: String::new(), compile: false } }
    pub fn text(&self) -> String { String::from_utf8_lossy(&self.msg).into_owned() }
}

/// Parse and compile a source file into a top-level iseq.
pub fn compile_source(src: &[u8], file: &'static str, line: i32, kind: IseqKind, outer: Vec<Vec<(Sym, u32, u32)>>) -> Result<&'static Iseq, CompileError> {
    compile_source_enc(src, crate::object::ENC_UTF8, file, line, kind, outer)
}
/// compile_source of an eval'd String: its encoding is the source's unless a magic comment says otherwise.
#[inline(always)]
pub fn compile_source_enc(src: &[u8], enc: u8, file: &'static str, line: i32, kind: IseqKind, outer: Vec<Vec<(Sym, u32, u32)>>) -> Result<&'static Iseq, CompileError> {
    let ename = if enc == crate::object::ENC_UTF8 { "" } else { crate::builtins::encoding::enc_name(enc) };
    let mut scopes: Vec<Vec<Vec<u8>>> = outer.iter().rev().map(|sc| sc.iter().map(|(n, _, _)| crate::symbol::sym_bytes(*n).to_vec()).collect()).collect();
    // an eval always parses with an outer scope (prism's parsing_eval: no top-level unused-variable warnings)
    if !scopes.is_empty() || kind == IseqKind::Eval { scopes.push(vec![]); }
    let po = prism::ParseOpts { filepath: file.as_bytes(), line, scopes, main_script: kind == IseqKind::Main, frozen_string_literal: 0, encoding: ename.as_bytes() };
    let pr = prism::parse(src, &po);
    if !pr.errors.is_empty() {
        push_parse_warnings(&pr, file);
        // highlighted as CRuby's: when stderr is a tty at parse time (bold only under NO_COLOR)
        let fmt = if crate::builtins::sys::isatty(2) != 1 { 1 } else if std::env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty()) { 2 } else { 3 };
        let (level, msg, enc) = prism::format_errors(src, &po, fmt);
        return Err(CompileError { level, msg, enc, compile: false });
    }
    compile_parsed(&pr, src, file, kind, outer)
}

fn push_parse_warnings(pr: &ParseResult, file: &str) {
    if file == "<internal:prelude>" || pr.warnings.is_empty() { return; }
    PARSE_WARNINGS.with(|w| {
        let mut w = w.borrow_mut();
        for d in &pr.warnings { w.push((format!("{}:{}: warning: {}\n", file, pr.line_of(d.loc.start) as i32, d.msg), d.level != 0)); }
    });
}

/// compile_source of a parse already made (the prelude's is made at build time).
pub fn compile_parsed(pr: &ParseResult, src: &[u8], file: &'static str, kind: IseqKind, outer: Vec<Vec<(Sym, u32, u32)>>) -> Result<&'static Iseq, CompileError> {
    push_parse_warnings(pr, file);
    if let Some(e) = pr.errors.first() { return Err(CompileError::syntax(format!("{}:{}: {}", file, pr.line_of(e.loc.start) as i32, e.msg))); }
    let mut cm = Compiler::new(pr, file);
    cm.src = src;
    // compiled for tracing once a source names the hooks or the allocation tracer (zero cost for every other program)
    use std::sync::atomic::Ordering::Relaxed;
    if file != "<internal:prelude>" && (TRACE_COMPILE.load(Relaxed) || [&b"TracePoint"[..], b"set_trace_func", b"trace_object_allocations", b"objspace/trace"].iter().any(|w| src.windows(w.len()).any(|x| x == *w))) {
        TRACE_COMPILE.store(true, Relaxed);
        cm.trace = true;
    }
    if let Some(f) = crate::builtins::coverage::compile_wants(file, kind == IseqKind::Eval, src, pr.start_line) { cm.trace = true; cm.cov = Some(f); }
    if kind == IseqKind::Eval && !outer.is_empty() { cm.captures.entry(key(&pr.root)).or_default().insert(Sym(u32::MAX)); }
    cm.eval_outer = outer;
    if kind == IseqKind::Main && src.windows(16).any(|w| w == b"TOPLEVEL_BINDING") {
        cm.tl_binding = true;
        cm.captures.entry(key(&pr.root)).or_default().insert(Sym(u32::MAX));
    }
    let is = cm.compile_program(&pr.root, kind);
    if cm.errors.is_empty() { Ok(is) } else { Err(CompileError { compile: true, ..CompileError::syntax(cm.errors.join("\n")) }) }
}

/// The value of a single non-splat `when` pattern that is an immediate literal.
fn lit_imm(pats: &[NodePtr], kinds: &[u8]) -> Option<Value> {
    if pats.len() != 1 || kinds[0] != 0 { return None; }
    let n = pats[0];
    if !std::ptr::eq(unsafe { (*n).head.kind }, &KIND_node_lit) { return None; }
    let v = unsafe { (*(n as *mut NodeOf<node_lit_struct>)).u.v };
    if v.is_fixnum() || v.is_special() { Some(v) } else { None }
}

/// The nodes of a constant path's static parent take the path's tag (one instruction in CRuby).
fn retag_cpath(n: NodePtr, tag: u32) {
    node_set_flags(n, node_flags(n) & ((1 << NF_TAG_SHIFT) - 1) | tag << NF_TAG_SHIFT);
    if std::ptr::eq(node_kind(n), &KIND_node_const_path) { retag_cpath(node_u::<node_const_path_struct>(n).parent, tag); }
}

/// A constant path from a constant or `::` only (CRuby compiles it as one opt_getconstant_path).
fn static_cpath(n: &PNode) -> bool {
    match &n.kind {
        Kind::ConstantReadNode { .. } | Kind::ConstantPathNode { parent: None, .. } => true,
        Kind::ConstantPathNode { parent: Some(p), .. } => static_cpath(p),
        _ => false,
    }
}

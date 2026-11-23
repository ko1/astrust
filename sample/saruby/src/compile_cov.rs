// Compiler parts for line elision, Coverage and branch coverage (a module of compile.rs, kept apart
// from the hot compile code).
use super::*;

impl<'a> Compiler<'a> {
    #[inline(never)]
    pub(super) fn statements(&mut self, body: &[PNode]) -> NodePtr {
                let flat = std::mem::take(&mut self.flat_stmts);
                let last = body.len().saturating_sub(1);
                let v: Vec<NodePtr> = body.iter().enumerate().map(|(i, x)| {
                    let l = if self.trace { self.line(x.loc) } else { 0 };
                    // a side-effect-free statement whose value is unused compiles to nothing in CRuby: no line event
                    let elided = self.trace && i < last && pure_static(x);
                    let traced = self.trace && !flat && !elided && !matches!(x.kind, Kind::BeginNode { .. }) && std::mem::replace(&mut self.cur().trace_line, l) != l;
                    let void_read = i < last && matches!(x.kind, Kind::LocalVariableReadNode { .. });
                    if void_read { self.void_read = true; }
                    let r = if elided { self.trace = false; let r = self.c(x); self.trace = true; r } else { self.c(x) };
                    self.void_read = false;
                    if traced {
                        let t = ALLOC_node_trace(EV_LINE, l, r);
                        match self.cov { Some(f) => ALLOC_node_cov(crate::builtins::coverage::line_slot(f, l), t), None => t }
                    }
                    else if needs_line(x) { ALLOC_node_line(self.line(x.loc), r) } else { r }
                }).collect();
                self.seq(v)
                }
    #[inline(never)]
    pub(super) fn if_node(&mut self, n: &PNode, predicate: &PNode, statements: Option<&PNode>, subsequent: Option<&PNode>) -> NodePtr {

                let cnd = self.cond(predicate);
                let fold = static_truth(predicate);
                let bb = if fold.is_some() { None } else { self.bbase("if", n) };
                let saved_cov = self.cov;
                if fold == Some(false) { self.cov = None; }
                // `x if c` / `c ? x : y`: no line events of their own (CRuby's newline nodes)
                let flat = n.loc.start >= predicate.loc.start;
                let t = self.stmts_flat(statements, flat);
                let pe = predicate.loc.start + predicate.loc.len;
                let t = self.btarget(bb, "then", |c| match statements { Some(s) => c.ploc(s), None => c.pos4(pe, pe) }, t);
                self.cov = if fold == Some(true) { None } else { saved_cov };
                let e = match subsequent { Some(x) => { self.flat_stmts = flat; let e = self.else_body(x); self.flat_stmts = false; e } None => ALLOC_node_nil() };
                let e = self.btarget(bb, "else", |c| match subsequent { Some(x) => c.else_loc(x, x), None => c.ploc(n) }, e);
                self.cov = saved_cov;
                ALLOC_node_if(cnd, t, e)
                }
    #[inline(never)]
    pub(super) fn unless_node(&mut self, n: &PNode, predicate: &PNode, statements: Option<&PNode>, else_clause: Option<&PNode>) -> NodePtr {

                let cnd = self.cond(predicate);
                let fold = static_truth(predicate);
                let bb = if fold.is_some() { None } else { self.bbase("unless", n) };
                let saved_cov = self.cov;
                if fold == Some(true) { self.cov = None; }
                let t = self.stmts_flat(statements, n.loc.start >= predicate.loc.start);
                self.cov = if fold == Some(false) { None } else { saved_cov };
                let e = match else_clause { Some(x) => self.else_body(x), None => ALLOC_node_nil() };
                self.cov = saved_cov;
                let e = self.btarget(bb, "else", |c| match else_clause { Some(x) => c.else_loc(x, n), None => c.ploc(n) }, e);
                let t = self.btarget(bb, "then", |c| match statements { Some(s) => c.ploc(s), None => c.ploc(n) }, t);
                ALLOC_node_if(cnd, e, t)
                }

    /// [first_lineno, first_column, last_lineno, last_column] of bytes start..end.
    pub(super) fn pos4(&self, start: u32, end: u32) -> [u32; 4] { [self.pr.line_of(start), self.pr.col_of(start), self.pr.line_of(end), self.pr.col_of(end)] }
    pub(super) fn ploc(&self, n: &PNode) -> [u32; 4] { self.pos4(n.loc.start, n.loc.start + n.loc.len) }
    /// Branch coverage (Coverage with branches:): a base for `n`, if measured.
    pub(super) fn bbase(&mut self, ty: &'static str, n: &PNode) -> Option<usize> {
        let f = self.cov?;
        crate::builtins::coverage::branch_base(f, ty, self.ploc(n))
    }
    /// The target's location `p` is computed only when branches are measured.
    pub(super) fn btarget(&mut self, base: Option<usize>, label: &'static str, p: impl FnOnce(&Self) -> [u32; 4], body: NodePtr) -> NodePtr {
        match base { Some(b) => { let p = p(self); ALLOC_node_cov(crate::builtins::coverage::branch_target(b, label, p), body) } None => body }
    }
    /// The location of an else clause's body (the whole construct `parent` when it is empty).
    pub(super) fn else_loc(&self, e: &PNode, parent: &PNode) -> [u32; 4] {
        match &e.kind { Kind::ElseNode { statements: Some(s) } => self.ploc(s), Kind::ElseNode { statements: None } => self.ploc(parent), _ => self.ploc(e) }
    }

    /// Bodies of `when` / `in` clauses and the else, compiled in source order (branch ids follow it).
    pub(super) fn clause_bodies(&mut self, bb: Option<usize>, label: &'static str, case: &PNode, conds: &[PNode], els: Option<&PNode>, no_else: NodePtr) -> (Vec<NodePtr>, NodePtr) {
        let mut v = vec![];
        for w in conds {
            let (Kind::WhenNode { statements, .. } | Kind::InNode { statements, .. }) = &w.kind else { v.push(ALLOC_node_nil()); continue };
            let b = self.stmts(statements.as_deref());
            v.push(self.btarget(bb, label, |c| match statements { Some(s) => c.ploc(s), None => c.ploc(w) }, b));
        }
        let e = match els { Some(e) => self.else_body(e), None => no_else };
        let e = self.btarget(bb, "else", |c| match els { Some(e) => c.else_loc(e, e), None => c.ploc(case) }, e);
        (v, e)
    }

}

/// Statements CRuby compiles to nothing when their value is unused (`lvar`: local reads count).
pub(super) fn pure_static_in(n: &PNode, lvar: bool) -> bool {
    let p = |x: &PNode| pure_static_in(x, lvar);
    match &n.kind {
        Kind::IntegerNode { .. } | Kind::FloatNode { .. } | Kind::RationalNode { .. } | Kind::ImaginaryNode { .. } | Kind::StringNode { .. }
        | Kind::SymbolNode { .. } | Kind::NilNode | Kind::TrueNode | Kind::FalseNode | Kind::SelfNode
        | Kind::InstanceVariableReadNode { .. } | Kind::SourceLineNode | Kind::SourceEncodingNode | Kind::SourceFileNode { .. } => true,
        Kind::LocalVariableReadNode { .. } => lvar,
        // inside a container only literals stay static for folding ([self]; [a] are not)
        Kind::ArrayNode { elements, .. } => elements.iter().all(|x| if lvar { p(x) } else { lit_static(x) }),
        Kind::HashNode { elements } => elements.iter().all(|e| matches!(&e.kind, Kind::AssocNode { key, value, .. } if if lvar { p(key) && p(value) } else { lit_static(key) && lit_static(value) })),
        Kind::RangeNode { left, right, .. } => left.as_deref().is_none_or(p) && right.as_deref().is_none_or(p),
        Kind::ParenthesesNode { body: Some(b) } => match &b.kind { Kind::StatementsNode { body } => body.iter().all(p), _ => p(b) },
        _ => false,
    }
}
pub(super) fn pure_static(n: &PNode) -> bool { pure_static_in(n, true) }
pub(super) fn lit_static(n: &PNode) -> bool { !matches!(n.kind, Kind::SelfNode | Kind::InstanceVariableReadNode { .. } | Kind::LocalVariableReadNode { .. }) && pure_static_in(n, false) }
/// A condition CRuby folds at compile time (no branch coverage, the dead side not compiled): its truth.
pub(super) fn static_truth(n: &PNode) -> Option<bool> {
    match &n.kind {
        Kind::FalseNode | Kind::NilNode => Some(false),
        Kind::LambdaNode { .. } => Some(true),
        _ => paren_truth(n),
    }
}
pub(super) fn paren_truth(n: &PNode) -> Option<bool> {
    match &n.kind {
        Kind::TrueNode | Kind::IntegerNode { .. } | Kind::FloatNode { .. } | Kind::RationalNode { .. } | Kind::ImaginaryNode { .. }
        | Kind::SymbolNode { .. } | Kind::SourceLineNode | Kind::SourceEncodingNode => Some(true),
        // `(a; b; c)`: literal statements before a truthy constant
        Kind::ParenthesesNode { body: Some(b) } => match &b.kind {
            Kind::StatementsNode { body } if !body.is_empty() => {
                let (last, init) = body.split_last().unwrap();
                if init.iter().all(|x| pure_static_in(x, false)) { paren_truth(last) } else { None }
            }
            _ => paren_truth(b),
        },
        _ => None,
    }
}


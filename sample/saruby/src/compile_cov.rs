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
                    self.cov_popped = i < last;
                    // defined?(...) whose value is unused is not evaluated at all (CRuby)
                    let r = if i < last && matches!(x.kind, Kind::DefinedNode { .. }) { if self.cov.is_some() { self.cov_event(x, true); } ALLOC_node_nil() }
                        else if elided { self.trace = false; let r = self.c(x); self.trace = true; r } else { self.c(x) };
                    self.cov_popped = false;
                    if traced { ALLOC_node_trace(EV_LINE, l, r) }
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
                // a side the condition never takes is not compiled (its line events neither)
                if cond_never(predicate, true) { self.cov = None; }
                // `x if c` / `c ? x : y`: no line events of their own (CRuby's newline nodes)
                let flat = n.loc.start >= predicate.loc.start;
                let t = self.stmts_flat(statements, flat);
                let pe = predicate.loc.start + predicate.loc.len;
                let t = self.btarget(bb, "then", |c| match statements { Some(s) => c.ploc(s), None => c.pos4(pe, pe) }, t);
                self.cov = if cond_never(predicate, false) { None } else { saved_cov };
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
                if cond_never(predicate, false) { self.cov = None; }
                let t = self.stmts_flat(statements, n.loc.start >= predicate.loc.start);
                self.cov = if cond_never(predicate, true) { None } else { saved_cov };
                let e = match else_clause { Some(x) => self.else_body(x), None => ALLOC_node_nil() };
                self.cov = saved_cov;
                let e = self.btarget(bb, "else", |c| match else_clause { Some(x) => c.else_loc(x, n), None => c.ploc(n) }, e);
                let t = self.btarget(bb, "then", |c| match statements { Some(s) => c.ploc(s), None => c.ploc(n) }, t);
                ALLOC_node_if(cnd, e, t)
                }

    /// c() under Coverage: pm_compile_node's line event, its counter on the node of the next instruction.
    #[inline(never)]
    pub(super) fn c_cov(&mut self, n: &PNode) -> NodePtr {
        let popped = std::mem::take(&mut self.cov_popped);
        let f = self.cov;
        match self.cov_event(n, popped) {
            Some(Some(l)) => { let s = crate::builtins::coverage::line_slot(f.unwrap(), l); let r = self.c_tagged(n); ALLOC_node_cov(s, r) }
            Some(None) => self.c_tagged(n),
            // n compiles to no instruction: the waiting event goes past its parts
            None => { self.cov = None; let r = self.c_tagged(n); self.cov = f; r }
        }
    }
    /// A line event for n (prism's newline flag on a new line), and the line of the instruction that
    /// carries the waiting event when n compiles it first (events on one instruction share its counter).
    /// None: an event waits and n has no code.
    pub(super) fn cov_event(&mut self, n: &PNode, popped: bool) -> Option<Option<u32>> {
        if self.scopes.is_empty() { return Some(None); }
        if n.flags & crate::prism_ast::FLAG_NEWLINE != 0 {
            // a begin with only a rescue starts on the rescue's line (pm_compile_node)
            let l = match &n.kind { Kind::BeginNode { statements: None, rescue_clause: Some(r), .. } => self.line(r.loc), _ => self.line(n.loc) };
            let sc = self.cur();
            if sc.cov_line != l { sc.cov_line = l; sc.cov_pending = true; }
        }
        if !self.cur().cov_pending { return Some(None); }
        let (l, leaf) = self.first_insn(n, popped)?;
        let sc = self.cur();
        sc.cov_pending = false;
        if sc.cov_leaf == leaf { return Some(None); }
        sc.cov_leaf = leaf;
        Some(Some(l))
    }
    /// CRuby's first instruction for n: (its line, the prism id of the node that emits it); None: no code.
    pub(super) fn first_insn(&self, n: &PNode, popped: bool) -> Option<(u32, u32)> {
        let own = Some((self.line(n.loc), n.id));
        let f = |x: &PNode| self.first_insn(x, false);
        let opt = |x: Option<&PNode>, popped: bool| match x { Some(x) => self.first_insn(x, popped), None => if popped { None } else { own } };
        if popped && pure_static(n) { return None; }
        match &n.kind {
            Kind::StatementsNode { body } => {
                let k = body.len();
                body.iter().enumerate().find_map(|(i, x)| self.first_insn(x, popped || i + 1 < k)).or(if popped { None } else { own })
            }
            Kind::ParenthesesNode { body } => opt(body.as_deref(), popped),
            Kind::BeginNode { statements, .. } => opt(statements.as_deref(), popped),
            Kind::ElseNode { statements } => opt(statements.as_deref(), popped),
            Kind::EmbeddedStatementsNode { statements } => opt(statements.as_deref(), false),
            Kind::EmbeddedVariableNode { variable } => f(variable),
            Kind::IfNode { predicate, statements, subsequent } => match static_truth(predicate) {
                Some(true) => opt(statements.as_deref(), popped),
                Some(false) => opt(subsequent.as_deref(), popped),
                None => f(predicate),
            },
            Kind::UnlessNode { predicate, statements, else_clause } => match static_truth(predicate) {
                Some(false) => opt(statements.as_deref(), popped),
                Some(true) => opt(else_clause.as_deref(), popped),
                None => f(predicate),
            },
            // `x = x` whose value is unused: no code
            Kind::LocalVariableWriteNode { name, depth, value } if popped && matches!(&value.kind, Kind::LocalVariableReadNode { name: m, depth: d } if m == name && d == depth) => None,
            Kind::LocalVariableWriteNode { value, .. } | Kind::InstanceVariableWriteNode { value, .. } | Kind::GlobalVariableWriteNode { value, .. }
            | Kind::ClassVariableWriteNode { value, .. } | Kind::ConstantWriteNode { value, .. } => f(value),
            Kind::ConstantPathWriteNode { target, .. } | Kind::ConstantPathOperatorWriteNode { target, .. } | Kind::ConstantPathOrWriteNode { target, .. } | Kind::ConstantPathAndWriteNode { target, .. } => f(target),
            Kind::ConstantPathNode { parent: Some(p), .. } => f(p),
            Kind::CallNode { receiver: Some(r), .. } | Kind::CallOperatorWriteNode { receiver: Some(r), .. } | Kind::CallOrWriteNode { receiver: Some(r), .. }
            | Kind::CallAndWriteNode { receiver: Some(r), .. } | Kind::IndexOperatorWriteNode { receiver: Some(r), .. } | Kind::IndexOrWriteNode { receiver: Some(r), .. }
            | Kind::IndexAndWriteNode { receiver: Some(r), .. } => f(r),
            Kind::AndNode { left, .. } | Kind::OrNode { left, .. } => f(left),
            Kind::RangeNode { left: Some(l), .. } => f(l),
            Kind::ArrayNode { elements } | Kind::HashNode { elements } | Kind::KeywordHashNode { elements } => match elements.first() {
                Some(x) if n.flags & crate::prism_ast::FLAG_STATIC_LITERAL == 0 && x.flags & crate::prism_ast::FLAG_STATIC_LITERAL == 0
                    && !lit_static(x) && !matches!(&x.kind, Kind::AssocNode { key, value } if lit_static(key) && lit_static(value)) => f(x),
                _ => own,
            },
            Kind::AssocNode { key, .. } => f(key),
            Kind::AssocSplatNode { value: Some(v) } | Kind::SplatNode { expression: Some(v) } => f(v),
            Kind::ArgumentsNode { arguments } => arguments.first().map_or(own, f),
            Kind::YieldNode { arguments: Some(a) } | Kind::ReturnNode { arguments: Some(a) } | Kind::BreakNode { arguments: Some(a) } | Kind::NextNode { arguments: Some(a) } => f(a),
            // pm_interpolated_node_compile: a leading literal part (or the "" before a lone interpolation) is at the node's start
            Kind::InterpolatedStringNode { parts } | Kind::InterpolatedSymbolNode { parts } | Kind::InterpolatedRegularExpressionNode { parts }
            | Kind::InterpolatedMatchLastLineNode { parts } => match parts.first() {
                Some(x) if parts.len() > 1 && !matches!(x.kind, Kind::StringNode { .. }) && !embedded_string(x) => f(x),
                _ => own,
            },
            Kind::RescueModifierNode { expression, .. } => f(expression),
            Kind::CaseNode { predicate: Some(p), .. } | Kind::CaseMatchNode { predicate: Some(p), .. } => f(p),
            Kind::MatchPredicateNode { value, .. } | Kind::MatchRequiredNode { value, .. } => f(value),
            Kind::MatchWriteNode { call, .. } => f(call),
            Kind::SingletonClassNode { expression, .. } => f(expression),
            Kind::DefNode { receiver: Some(r), .. } => f(r),
            Kind::ForNode { collection, .. } => f(collection),
            Kind::MultiWriteNode { lefts, rest, rights, value } => lefts.iter().chain(rest.as_deref()).chain(rights.iter()).find_map(|t| match &t.kind {
                Kind::CallTargetNode { receiver, .. } | Kind::IndexTargetNode { receiver, .. } => f(receiver),
                Kind::ConstantPathTargetNode { parent: Some(p), .. } => f(p),
                _ => None,
            }).or_else(|| f(value)),
            _ => own,
        }
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
/// `#{"lit"}`: folded into the literal around it
fn embedded_string(n: &PNode) -> bool {
    matches!(&n.kind, Kind::EmbeddedStatementsNode { statements: Some(s) } if matches!(&s.kind, Kind::StatementsNode { body } if body.len() == 1 && matches!(body[0].kind, Kind::StringNode { .. })))
}
/// The condition can never be `want` (pm_compile_branch_condition leaves that side unreachable).
pub(super) fn cond_never(n: &PNode, want: bool) -> bool {
    match static_truth(n) {
        Some(t) => t != want,
        None => match &n.kind {
            Kind::AndNode { left, right } => if want { cond_never(left, true) || cond_never(right, true) } else { cond_never(left, false) && cond_never(right, false) },
            Kind::OrNode { left, right } => if want { cond_never(left, true) && cond_never(right, true) } else { cond_never(left, false) || cond_never(right, false) },
            _ => false,
        },
    }
}
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


// Module / Class helpers.
// On the context-borrowing heap accessors (heap.rs, docs/safe_heap.md).
#![forbid(unsafe_code)]
#[allow(unused_imports)]
use crate::heap::no_legacy::*;   // the old accessors cannot be named here
#[allow(unused_imports)]
use crate::heap::no_legacy_hn::*;
use crate::heap::{obj_klass, obj_set_klass};
use crate::value::*;
use crate::vm::*;

/// Module#dup / Class#dup: a fresh class with copies of the tables.
pub fn clone_class(c: &mut Ctx, k: Value) -> RV {
    if k == c.vm().c_basic_object.get() { return type_err(c, "can't copy the root class"); }
    let (is_module, sup) = { let cd = c.class(k); (cd.is_module, cd.superclass) };
    let n = if is_module { new_module(c) } else { new_class(c, sup) };
    if is_module { obj_set_klass(n, obj_klass(k)); }
    // entries are rebuilt for the new owner; plain data and Values are copied (no allocation until written)
    let mt: Vec<(Sym, MethodEntry)> = c.class(k).mtbl.iter()
        .filter_map(|(m, me)| c.me_ref(*me).map(|o| (*m, MethodEntry { name: o.name, owner: n, visi: o.visi, kind: o.kind, orig_owner: if o.orig_owner == k { n } else { o.orig_owner }, cref: o.cref })))
        .collect();
    let cd = c.class(k);
    let autoload = cd.autoload.clone();
    let (consts, const_order, cvars, includes, prepends, alloc) =
        (cd.consts.clone(), cd.const_order.clone(), cd.cvars.clone(), cd.includes.clone(), cd.prepends.clone(), cd.alloc);
    let nd = c.class_mut(n);
    for (m, e) in mt { nd.mtbl.insert(m, Box::into_raw(Box::new(e))); }
    for (a, b) in consts { nd.consts.insert(a, b); }
    nd.autoload = autoload;
    nd.const_order = const_order;
    for (a, b) in cvars { nd.cvars.insert(a, b); }
    nd.includes = includes;
    nd.prepends = prepends;
    nd.alloc = alloc;
    // singleton methods (class methods)
    let sk = obj_klass(k);
    if c.class(sk).is_singleton {
        let nsk = singleton_class(c, n);
        let tbl: Vec<(Sym, *const MethodEntry)> = c.class(sk).mtbl.iter().map(|(a, b)| (*a, *b)).collect();
        let nt = &mut c.class_mut(nsk).mtbl;
        for (m, me) in tbl { nt.insert(m, me); }
        // modules the class was extended with
        let inc = c.class(sk).includes.clone();
        c.class_mut(nsk).includes = inc;
    }
    bump_hierarchy();
    Ok(n)
}

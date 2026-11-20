// Module / Class helpers.
use crate::object::*;
use crate::value::*;
use crate::vm::*;

/// Module#dup / Class#dup: a fresh class with copies of the tables.
pub fn clone_class(_c: &mut Ctx, k: Value, cd: &ClassData) -> RV {
    let n = if cd.is_module { new_module() } else { new_class(cd.superclass) };
    obj(n).klass = if cd.is_module { obj(k).klass } else { obj(n).klass };
    let nd = rclass(n);
    for (m, me) in cd.mtbl.iter() {
        let o = unsafe { &**me };
        let ne = Box::into_raw(Box::new(MethodEntry { name: o.name, owner: n, visi: o.visi, kind: o.kind, orig_owner: o.orig_owner, cref: o.cref }));
        nd.mtbl.insert(*m, ne);
    }
    for (a, b) in cd.consts.iter() { nd.consts.insert(*a, *b); }
    nd.const_order = cd.const_order.clone();
    for (a, b) in cd.cvars.iter() { nd.cvars.insert(*a, *b); }
    nd.includes = cd.includes.clone();
    nd.prepends = cd.prepends.clone();
    nd.alloc = cd.alloc;
    // singleton methods (class methods)
    let sk = obj(k).klass;
    if rclass(sk).is_singleton {
        let nsk = singleton_class(n);
        let tbl: Vec<(Sym, *const MethodEntry)> = rclass(sk).mtbl.iter().map(|(a, b)| (*a, *b)).collect();
        for (m, me) in tbl { rclass(nsk).mtbl.insert(m, me); }
    }
    bump_hierarchy();
    Ok(n)
}

pub fn init() {}

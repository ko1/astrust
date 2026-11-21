// pp: pretty_inspect (wraps long arrays/hashes like CRuby's pp).
// On the context-borrowing heap accessors (heap.rs, docs/safe_heap.md).
#![forbid(unsafe_code)]
#[allow(unused_imports)]
use crate::heap::no_legacy::*;   // the old accessors cannot be named here
#[allow(unused_imports)]
use crate::heap::no_legacy_hn::*;
#[allow(unused_imports)]
use crate::heap::no_legacy_str::*;
use crate::array::*;
use crate::builtins::kernel::*;
use crate::value::*;
use crate::vm::*;

fn pp_rec(c: &mut Ctx, v: Value, indent: usize, width: usize, out: &mut String) -> Result<(), Esc> {
    let flat = inspect_str(c, v)?;
    if flat.len() + indent <= width || !(is_array(v) || is_hash(v)) { out.push_str(&flat); return Ok(()); }
    if is_array(v) {
        out.push('[');
        let mut i = 0;
        while let Some(&x) = c.ary(v).get(i) {   // inspect may change the Array
            if i > 0 { out.push_str(",\n"); out.push_str(&" ".repeat(indent + 1)); }
            pp_rec(c, x, indent + 1, width, out)?;
            i += 1;
        }
        out.push(']');
    } else {
        out.push('{');
        let (mut slot, mut i) = (0, 0);
        while let Some((k, x)) = c.hash_entry_at(v, slot) {   // slot by slot: inspect may change the Hash
            slot += 1;
            if k == UNDEF { continue; }
            if i > 0 { out.push_str(",\n"); out.push_str(&" ".repeat(indent + 1)); }
            i += 1;
            let ks = if k.is_sym() { let s = crate::string::sym_inspect(crate::symbol::sym_bytes(k.as_sym())); format!("{}: ", &s[1..]) } else { format!("{} => ", inspect_str(c, k)?) };
            out.push_str(&ks);
            let sub = inspect_str(c, x)?;
            if indent + 1 + ks.len() + sub.len() > width && (is_array(x) || is_hash(x)) {
                out.push('\n'); out.push_str(&" ".repeat(indent + 2));
                pp_rec(c, x, indent + 2, width, out)?;
            } else { out.push_str(&sub); }
        }
        out.push('}');
    }
    Ok(())
}

pub fn pretty_inspect(c: &mut Ctx, v: Value, width: usize) -> Result<String, Esc> {
    let mut out = String::new();
    pp_rec(c, v, 0, width, &mut out)?;
    out.push('\n');
    Ok(out)
}

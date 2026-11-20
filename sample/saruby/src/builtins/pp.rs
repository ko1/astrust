// pp: pretty_inspect (wraps long arrays/hashes like CRuby's pp).
use crate::array::*;
use crate::builtins::kernel::*;
use crate::value::*;
use crate::vm::*;

fn pp_rec(c: &mut Ctx, v: Value, indent: usize, width: usize, out: &mut String) -> Result<(), Esc> {
    let flat = inspect_str(c, v)?;
    if flat.len() + indent <= width || !(is_array(v) || is_hash(v)) { out.push_str(&flat); return Ok(()); }
    if is_array(v) {
        out.push('[');
        let n = ary_len(v);
        for i in 0..n {
            if i > 0 { out.push_str(",\n"); out.push_str(&" ".repeat(indent + 1)); }
            pp_rec(c, ary_get(v, i), indent + 1, width, out)?;
        }
        out.push(']');
    } else {
        out.push('{');
        for (i, (k, x)) in hash_pairs(v).into_iter().enumerate() {
            if i > 0 { out.push_str(",\n"); out.push_str(&" ".repeat(indent + 1)); }
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

// did_you_mean (prelude/lib/did_you_mean{.rb,/}, CRuby 4.0.2's): what gem_prelude's `require "did_you_mean"` makes
// visible, set up at boot without compiling the library; the library is compiled on first use.
#![forbid(unsafe_code)]
#[allow(unused_imports)]
use crate::heap::no_legacy::*;   // the old accessors cannot be named here
use crate::symbol::intern;
use crate::value::*;
use crate::vm::*;
use std::sync::atomic::{AtomicU8, Ordering::Relaxed};

// 0: not loaded, 1: loaded, 2: disabled (options are read on another thread), 3: loading
static STATE: AtomicU8 = AtomicU8::new(0);
pub fn disable() { STATE.store(2, Relaxed); }

const CONSTS: &[&str] = &["ClassNameChecker", "Formatter", "Jaro", "JaroWinkler", "KeyErrorChecker", "Levenshtein", "MethodNameChecker",
    "NameErrorCheckers", "NullChecker", "PatternKeyNameChecker", "PlainFormatter", "RequirePathChecker", "SpellChecker", "TreeSpellChecker",
    "VERSION", "VariableNameChecker"];

/// Boot: DidYouMean::Correctable prepended where the library's correct_error puts it (so a detailed_message defined
/// on those classes is behind it, as in CRuby), each public method a stub that loads the library and calls the real one.
pub fn boot(c: &mut Ctx) {
    if STATE.load(Relaxed) == 2 { return; }
    let m = define_module(c, "DidYouMean");
    let co = define_module_under(c, m, "Correctable");
    define_cfunc(c, co, "detailed_message", |c, s, a, b| stub_method(c, s, a, b, b"detailed_message"), -1);
    define_cfunc(c, co, "original_message", |c, s, a, b| stub_method(c, s, a, b, b"original_message"), 0);
    define_cfunc(c, co, "corrections", |c, s, a, b| stub_method(c, s, a, b, b"corrections"), 0);
    define_cfunc(c, co, "spell_checker", |c, s, a, b| stub_method(c, s, a, b, b"spell_checker"), 0);
    let v = c.vm();
    for k in [v.c_name_error.get(), v.c_key_error.get(), v.c_load_error.get(), v.c_no_matching_pattern_key_error.get()] { prepend_module(c, k, co); }
    define_singleton_cfunc(c, m, "spell_checkers", |c, s, a, b| stub_call(c, s, a, b, b"spell_checkers"), 0);
    define_singleton_cfunc(c, m, "correct_error", |c, s, a, b| stub_call(c, s, a, b, b"correct_error"), 2);
    define_singleton_cfunc(c, m, "formatter", |c, s, a, b| stub_call(c, s, a, b, b"formatter"), 0);
    define_singleton_cfunc(c, m, "formatter=", |c, s, a, b| stub_call(c, s, a, b, b"formatter="), 1);
    crate::builtins::lazy_consts(c, m, CONSTS, "did_you_mean");
}

fn module(c: &Ctx) -> Option<Value> { const_get_at(c, c.vm().c_object.get(), intern(b"DidYouMean")).filter(|&m| crate::array::is_class_or_module(m)) }

/// $LOAD_PATH once -I and RUBYLIB are in, when they put something there (RequirePathChecker's INITIAL_LOAD_PATH).
pub fn snapshot_load_path(c: &mut Ctx) {
    if STATE.load(Relaxed) == 2 { return; }
    let lp = c.vm().load_path.get();
    if c.ary_len(lp) == 0 { return; }
    let Some(m) = module(c) else { return };
    let v = c.ary(lp).to_vec();
    let d = c.ary_from_vec(v);
    ivar_set_raw(c, m, intern(b"__initial_load_path"), d);
}

/// DidYouMean.__initial_load_path: CRuby's $LOAD_PATH at boot, the reference ruby's dirs after the boot snapshot.
fn initial_load_path(c: &mut Ctx, slf: Value, _a: &[Value], _b: Blk) -> RV {
    let s = ivar_get(c, slf, intern(b"__initial_load_path"));
    let v = if s == NIL { vec![] } else { c.ary(s).to_vec() };
    let out = c.ary_from_vec(v);
    if let Some((lib, _)) = crate::builtins::prelude::stdlib_dirs() {
        let p = std::path::Path::new(&lib);
        let (base, ver) = (p.parent().map(|d| d.to_string_lossy().into_owned()).unwrap_or_default(), p.file_name().map(|v| v.to_string_lossy().into_owned()).unwrap_or_default());
        let arch = format!("{}-linux", if cfg!(target_arch = "aarch64") { "aarch64" } else { "x86_64" });
        for d in [format!("{base}/site_ruby/{ver}"), format!("{base}/site_ruby/{ver}/{arch}"), format!("{base}/site_ruby"), format!("{base}/vendor_ruby/{ver}"),
                  format!("{base}/vendor_ruby/{ver}/{arch}"), format!("{base}/vendor_ruby"), lib.clone(), format!("{lib}/{arch}")] {
            let s = c.str_new(d.as_bytes());
            c.ary_push(out, s);
        }
    }
    Ok(out)
}

/// Load the library once; false: it is not (yet) there, so a stub answers by itself.
fn load(c: &mut Ctx) -> Result<bool, Esc> {
    if STATE.load(Relaxed) == 0 {
        STATE.store(3, Relaxed);
        if let Some(m) = module(c) { let sk = singleton_class(c, m); define_private_cfunc(c, sk, "__initial_load_path", initial_load_path, 0); }
        if let Err(e) = crate::builtins::prelude::load_feature(c, "did_you_mean") { STATE.store(0, Relaxed); return Err(e); }
        STATE.store(1, Relaxed);
    }
    Ok(STATE.load(Relaxed) == 1)
}

/// `require "did_you_mean"` of a program: loaded at boot as in CRuby (false), compiled now if it was not.
pub fn require(c: &mut Ctx) -> Option<RV> {
    if STATE.load(Relaxed) == 2 { return None; }
    Some(load(c).map(|_| FALSE))
}

fn call(c: &mut Ctx, recv: Value, mid: &[u8], args: &[Value], b: Blk) -> RV {
    match kw_arg(args) {
        Some(_) => funcall_kw(c, recv, intern(mid), args, b, true),
        None => funcall(c, recv, intern(mid), args, b),
    }
}

/// A Correctable method before the library is loaded: the library's, bound to this exception (its super is then
/// what is behind Correctable, as for this call).
fn stub_method(c: &mut Ctx, slf: Value, a: &[Value], b: Blk, name: &[u8]) -> RV {
    let co = match load(c)? { true => module(c).and_then(|m| const_get_at(c, m, intern(b"Correctable"))), false => None };
    let Some(co) = co else {
        // while the library loads: as without it
        return match name {
            b"detailed_message" => crate::builtins::exception::exc_detailed_message(c, slf, a, b),
            b"original_message" => funcall(c, slf, intern(b"to_s"), &[], std::ptr::null()),
            _ => Ok(c.ary_new()),
        };
    };
    let um = funcall(c, co, intern(b"instance_method"), &[Value::sym(intern(name))], std::ptr::null())?;
    let mut args = Vec::with_capacity(a.len() + 1);
    args.push(slf); args.extend_from_slice(a);
    call(c, um, b"bind_call", &args, b)
}

/// DidYouMean.spell_checkers / .correct_error / .formatter / .formatter= before the library is loaded: the library's.
fn stub_call(c: &mut Ctx, slf: Value, a: &[Value], b: Blk, name: &[u8]) -> RV {
    if !load(c)? { return Ok(NIL); }
    call(c, slf, name, a, b)
}

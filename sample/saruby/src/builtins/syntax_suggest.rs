// The native part of syntax_suggest (prelude/lib/syntax_suggest/saruby.rb): prism's tokens and error messages for
// the gem's search, in place of the prism gem's C extension.  Loaded only when a SyntaxError's message is shown.
#![forbid(unsafe_code)]
#[allow(unused_imports)]
use crate::heap::no_legacy::*;   // the old accessors cannot be named here
use crate::symbol::intern;
use crate::value::*;
use crate::vm::*;

/// --disable=syntax_suggest / gems, and -c (no gem_prelude): SyntaxError#detailed_message is Exception's.
static DISABLED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
pub fn disable() { DISABLED.store(true, std::sync::atomic::Ordering::Relaxed); }
pub fn disabled() -> bool { DISABLED.load(std::sync::atomic::Ordering::Relaxed) }

fn opts(line: i32) -> crate::prism::ParseOpts<'static> {
    crate::prism::ParseOpts { filepath: b"", line, scopes: vec![], main_script: false, frozen_string_literal: 0, encoding: b"" }
}

/// __saruby__.ss_lex(source, line): [[type, value, start_line, start_column, start_offset, end_offset, lex_state], ...] in lexing order.
/// Token types are named as prism 1.8 (the default gem's LexCompat) names them.
fn ss_lex(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    let src = crate::string::expect_str(c, a[0])?;
    let line = if a[1].is_fixnum() { a[1].as_fix() as i32 } else { 1 };
    let (bytes, enc) = (c.str_bytes(src).to_vec(), c.str_enc(src));
    let mut starts = vec![0u32];
    starts.extend(bytes.iter().enumerate().filter(|x| *x.1 == b'\n').map(|x| x.0 as u32 + 1));
    let toks = crate::prism::lex(&bytes, &opts(line));
    let out = c.ary_new_capa(toks.len());
    for (ty, st, len, state) in toks {
        let name = crate::prism_ast::TOKEN_NAMES.get(ty as usize - 1).copied().unwrap_or("MISSING");
        let name = match name {
            "WORDS_SEP_IMPLICIT" => continue,
            "BRACE_LEFT_ARGUMENT" | "BRACE_LEFT_HASH" => "BRACE_LEFT",
            "KEYWORD_DO_BLOCK" | "KEYWORD_DO_LAMBDA" => "KEYWORD_DO",
            "PARENTHESIS_LEFT_GROUPING" => "PARENTHESIS_LEFT",
            "XSTRING_BEGIN" => "BACKTICK",
            "NEWLINE_TERMINATOR" => "IGNORED_NEWLINE",
            n => n,
        };
        let i = match starts.binary_search(&st) { Ok(i) => i, Err(i) => i - 1 };
        let (s, e) = (st as usize, (st + len) as usize);
        let v = c.str_new_enc(bytes.get(s..e).unwrap_or(&[]), enc);
        let t = c.ary_from_vec(vec![Value::sym(intern(name.as_bytes())), v, Value::fix(i as i64 + line as i64), Value::fix((st - starts[i]) as i64),
            Value::fix(st as i64), Value::fix(e as i64), Value::fix(state as i64)]);
        c.ary_push(out, t);
    }
    Ok(out)
}
/// __saruby__.ss_errors(source): the messages of prism's errors for the source.
fn ss_errors(c: &mut Ctx, _s: Value, a: &[Value], _b: Blk) -> RV {
    let src = crate::string::expect_str(c, a[0])?;
    let bytes = c.str_bytes(src).to_vec();
    let ms: Vec<Value> = crate::prism::error_messages(&bytes, &opts(1)).into_iter().map(|m| c.str_new(m.as_bytes())).collect();
    Ok(c.ary_from_vec(ms))
}

pub fn init(c: &mut Ctx) {
    let o = c.vm().c_object.get();
    let m = match const_get_at(c, o, intern(b"SyntaxSuggest")) { Some(m) if crate::array::is_class_or_module(m) => m, _ => define_module(c, "SyntaxSuggest") };
    let _ = define_module_under(c, m, "Prism");
    let im = c.vm().m_internal.get();
    define_singleton_cfunc(c, im, "ss_lex", ss_lex, 2);
    define_singleton_cfunc(c, im, "ss_errors", ss_errors, 1);
}

/// syntax_suggest's annotation of the file at `path` before `msg` (core_ext.rb's detailed_message); `msg` when the
/// search cannot run.  $! and any pending escape are kept as they were.
pub fn annotate(c: &mut Ctx, path: Value, highlight: bool, msg: Value) -> RV {
    let saved = (c.esc, c.esc_val, c.esc_tag, c.esc_tag_v, c.errinfo);
    let r = (|| {
        let f = c.str_new(b"syntax_suggest/saruby");
        crate::builtins::kernel::require_feature(c, f, false)?;
        let m = c.vm().m_internal.get();
        funcall(c, m, intern(b"ss_annotate"), &[path, Value::bool(highlight), msg], std::ptr::null())
    })();
    match r {
        Ok(v) if crate::object::is_string(v) => { c.errinfo = saved.4; Ok(v) }
        _ => { (c.esc, c.esc_val, c.esc_tag, c.esc_tag_v, c.errinfo) = saved; Ok(msg) }
    }
}

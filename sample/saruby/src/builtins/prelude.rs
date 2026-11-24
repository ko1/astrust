// Ruby-level prelude and lazily-loaded built-in features.
// On the context-borrowing heap accessors (heap.rs, docs/safe_heap.md).
#![forbid(unsafe_code)]
#[allow(unused_imports)]
use crate::heap::no_legacy::*;   // the old accessors cannot be named here
#[allow(unused_imports)]
use crate::heap::no_legacy_hn::*;
#[allow(unused_imports)]
use crate::heap::no_legacy_str::*;
use crate::value::*;
use crate::vm::*;

const CORE: &str = include_str!("../../prelude/core.rb");
/// prism's serialization of CORE, made by build.rs with the options `parse` would use.
const CORE_AST: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/core.prism"));

pub fn load_prelude(c: &mut Ctx) -> RV {
    let pr = crate::prism::read_serialized(CORE_AST);
    let is = match crate::compile::compile_parsed(&pr, CORE.as_bytes(), "<internal:prelude>", IseqKind::Top, vec![]) {
        Ok(is) => is,
        Err(m) => return err(c, c.vm().c_syntax_error.get(), m.text()),
    };
    crate::run_top_iseq(c, is, c.vm().main_obj.get())
}

macro_rules! lib { ($n:literal) => { ($n, include_str!(concat!("../../prelude/lib/", $n, ".rb"))) }; }
/// Features implemented by saruby itself: Ruby source compiled on first require.
const LIBS: &[(&str, &str)] = &[lib!("pp"), lib!("fcntl"), lib!("prettyprint"), lib!("etc"), lib!("timeout"), lib!("monitor"), lib!("time"), lib!("bundler"), ("bundler/setup", "require \"bundler\"; Bundler.setup\n"), lib!("date"), ("date_core", "require \"date\"\n"), lib!("pathname"), ("saruby/pathname", include_str!("../../prelude/lib/pathname_core.rb")), ("saruby/argf", include_str!("../../prelude/lib/argf.rb")), ("saruby/data", include_str!("../../prelude/lib/data.rb")), ("saruby/enum_product", include_str!("../../prelude/lib/enum_product.rb")), ("saruby/process", include_str!("../../prelude/lib/process_ext.rb")),
    ("coverage", "require 'coverage.so'\n"), lib!("objspace"), ("io/console/size", include_str!("../../prelude/lib/io_console_size.rb")), ("objspace/trace", include_str!("../../prelude/lib/objspace_trace.rb")), ("rbconfig/sizeof", include_str!("../../prelude/lib/rbconfig_sizeof.rb")),
    ("syslog", "require 'syslog_ext'\n"), lib!("syslog_ext"), ("syslog/logger", include_str!("../../prelude/lib/syslog_logger.rb")),
    lib!("digest"), ("digest/version", "module Digest\n  VERSION = \"3.2.1\"\nend\n"), ("digest/loader", "require 'saruby/digest'\n"),
    ("digest/md5", "require 'digest'\nrequire 'saruby/digest/md5'\n"), ("digest/sha1", "require 'digest'\nrequire 'saruby/digest/sha1'\n"),
    ("digest/rmd160", "require 'digest'\nrequire 'saruby/digest/rmd160'\n"), ("digest/bubblebabble", "require 'digest'\nrequire 'saruby/digest/bubblebabble'\n"),
    ("digest/sha2", include_str!("../../prelude/lib/digest_sha2.rb")), ("digest/sha2/loader", "require 'digest'\nrequire 'saruby/digest/sha2'\n"),
    lib!("error_highlight"), lib!("error_highlight/base"), lib!("error_highlight/core_ext"), lib!("error_highlight/formatter"), lib!("error_highlight/version"), lib!("error_highlight/saruby"), lib!("syntax_suggest/saruby"),
    lib!("did_you_mean"), lib!("did_you_mean/version"), lib!("did_you_mean/core_ext/name_error"), lib!("did_you_mean/spell_checker"),
    lib!("did_you_mean/levenshtein"), lib!("did_you_mean/jaro_winkler"), lib!("did_you_mean/spell_checkers/name_error_checkers"),
    lib!("did_you_mean/spell_checkers/name_error_checkers/class_name_checker"), lib!("did_you_mean/spell_checkers/name_error_checkers/variable_name_checker"),
    lib!("did_you_mean/spell_checkers/method_name_checker"), lib!("did_you_mean/spell_checkers/key_error_checker"), lib!("did_you_mean/spell_checkers/null_checker"),
    lib!("did_you_mean/spell_checkers/require_path_checker"), lib!("did_you_mean/spell_checkers/pattern_key_name_checker"), lib!("did_you_mean/formatter"),
    lib!("did_you_mean/tree_spell_checker"), lib!("did_you_mean/experimental"), lib!("did_you_mean/verbose"),
    lib!("did_you_mean/formatters/plain_formatter"), lib!("did_you_mean/formatters/verbose_formatter")];
/// Features whose implementation is native and always present (require is a no-op).
const NATIVE: &[&str] = &["set", "stringio", "rbconfig", "thread", "fiber", "enumerator", "rational", "complex", "continuation",
    "io/wait", "io/nonblock", "etc.so", "monitor.so", "strscan", "strscan.so", "bigdecimal.so", "psych.so", "socket.so", "json/ext/parser", "json/ext/parser.so", "json/ext/generator", "json/ext/generator.so", "zlib", "saruby/errno", "saruby/env", "saruby/rbconfig", "saruby/filetest", "saruby/stat", "saruby/sync", "saruby/econv", "saruby/tracepoint",
    "coverage.so", "objspace.so", "saruby/weakmap", "cgi/escape.so", "saruby/toplevel_binding", "saruby/etc", "saruby/syslog", "saruby/digest", "saruby/digest/md5", "saruby/digest/sha1", "saruby/digest/sha2", "saruby/digest/rmd160", "saruby/digest/bubblebabble", "saruby/io_buffer", "saruby/ractor", "saruby/error_highlight", "saruby/syntax_suggest"];

/// Constant `name` under `owner` whose native definition runs on first reference (boot time: builtins/mod.rs).
pub use crate::builtins::lazy_const;
/// A feature with Ruby code of saruby's own (CRuby has a .rb for it).
pub fn has_lib(name: &str) -> bool { LIBS.iter().any(|l| l.0 == name) }
pub fn has_feature(name: &str) -> bool { NATIVE.contains(&name) || LIBS.iter().any(|l| l.0 == name) }
pub fn load_feature(c: &mut Ctx, name: &str) -> RV {
    // lib/pp.rb shares the inspect recursion list (kernel::rec_enter)
    if name == "pp" { crate::builtins::kernel::REC_PP.store(true, std::sync::atomic::Ordering::Relaxed); }
    if let Some(l) = LIBS.iter().find(|l| l.0 == name) {
        let file: &'static str = Box::leak(format!("<internal:{}>", name).into_boxed_str());
        return crate::builtins::kernel::run_source(c, l.1.as_bytes(), file, 1);
    }
    // native libraries whose classes are only built on first require
    if name == "strscan" || name == "strscan.so" { crate::builtins::strscan::init(c); }
    // without the gem's Ruby code (no reference ruby), "json" is saruby's own approximation
    if name == "json" { crate::builtins::json::init(c); }
    if name.starts_with("json/ext/parser") { crate::builtins::json_parser::init(c)?; }
    if name.starts_with("json/ext/generator") { crate::builtins::json_generator::init(c)?; }
    if name == "zlib" { crate::builtins::zlib::init(c); }
    if name == "psych.so" { crate::builtins::psych::init(c)?; }
    if name == "bigdecimal.so" { crate::builtins::bigdecimal::init(c); }
    if name == "socket.so" { crate::builtins::socket::init(c); }
    match name {
        "saruby/errno" => crate::builtins::exception::define_all_errnos(c),
        "saruby/env" => crate::builtins::env::init(c),
        "saruby/rbconfig" | "rbconfig" => crate::builtins::misc::rbconfig_init(c),
        "saruby/filetest" => crate::builtins::file::init_filetest(c),
        "saruby/stat" => crate::builtins::file::init_stat(c),
        "saruby/sync" => crate::builtins::thread::init_sync(c),
        "saruby/econv" => crate::builtins::transcode::init(c),
        "saruby/tracepoint" => crate::builtins::tracepoint::init_class(c),
        "io/console" => crate::builtins::io_console::init(c),
        "pty" => crate::builtins::pty::init(c),
        "saruby/etc" => crate::builtins::etc::init(c),
        "cgi/escape.so" => crate::builtins::cgi_escape::init(c)?,
        "coverage.so" => crate::builtins::coverage::init(c),
        "objspace.so" => crate::builtins::objspace::init_ext(c),
        "saruby/weakmap" => crate::builtins::objspace::init_weak(c),
        "saruby/toplevel_binding" => crate::builtins::binding::init_toplevel(c),
        "saruby/syslog" => crate::builtins::syslog::init(c),
        "saruby/digest" => crate::builtins::digest::init(c),
        "saruby/digest/bubblebabble" => crate::builtins::digest::init_bubblebabble(c),
        _ if name.starts_with("saruby/digest/") => crate::builtins::digest::init_algo(c, &name[14..]),
        "io/nonblock" => crate::builtins::io::init_nonblock(c),
        "saruby/io_buffer" => crate::builtins::io_buffer::init(c),
        "saruby/ractor" => crate::builtins::ractor::init(c),
        "saruby/error_highlight" => crate::builtins::error_highlight::init(c),
        "saruby/syntax_suggest" => crate::builtins::syntax_suggest::init(c),
        "stringio" | "saruby/stringio" => crate::builtins::stringio::init_once(c),
        _ => {}
    }
    Ok(NIL)
}

/// (rubylibdir, gem dir) of the reference ruby: its pure-Ruby standard library is used as is.
pub fn stdlib_dirs() -> Option<(String, String)> {
    let v = std::env::var("SARUBY_STDLIB").ok().or_else(|| option_env!("SARUBY_STDLIB").map(|s| s.to_string()))?;
    let (lib, gem) = v.split_once(':')?;
    Some((lib.to_string(), gem.to_string()))
}
/// Bundled gems that live outside rubylibdir.
const BUNDLED: &[&str] = &["ostruct", "base64", "benchmark", "logger", "observer", "abbrev", "pstore", "csv", "matrix", "prime", "getoptlong", "mutex_m", "drb", "rinda", "racc", "bigdecimal", "nkf", "syslog", "rexml", "rss", "net-ftp", "net-imap", "net-pop", "net-smtp", "readline", "reline"];
const EXT_BACKED: &[&str] = &["openssl", "date", "ripper", "digest", "pathname", "pty", "coverage",
    "objspace", "nkf", "fiddle", "syslog", "io"];
/// Default gems that the reference ruby (rubygems) activates from a newer installed gem.
const GEM_FIRST: &[&str] = &["psych", "json"];
/// lib/<rb> of the highest installed version of gem `top` (the one activated: it has lib/<top>.rb).
fn newest_gem_file(gem: &str, top: &str, rb: &str) -> Option<std::path::PathBuf> {
    let ver = |v: &str| -> Vec<u64> { v.split('.').map(|x| x.parse().unwrap_or(0)).collect() };
    let mut best: Option<(Vec<u64>, std::path::PathBuf)> = None;
    for e in std::fs::read_dir(std::path::Path::new(gem).join("gems")).ok()?.flatten() {
        let n = e.file_name().to_string_lossy().into_owned();
        let Some(v) = n.strip_prefix(top).and_then(|r| r.strip_prefix('-')) else { continue };
        if !v.chars().all(|ch| ch.is_ascii_digit() || ch == '.') { continue; }
        let lib = e.path().join("lib");
        if lib.join(format!("{}.rb", top)).is_file() && best.as_ref().map(|b| b.0 < ver(v)).unwrap_or(true) { best = Some((ver(v), lib)); }
    }
    best.map(|b| b.1.join(rb)).filter(|f| f.is_file())
}
/// The stdlib / bundled gem file for `name`, looked up once per name (the gem dirs are read, as rubygems
/// reads its specs once).
pub fn stdlib_file(name: &str) -> Option<std::path::PathBuf> {
    static CACHE: std::sync::Mutex<Option<std::collections::HashMap<String, Option<std::path::PathBuf>>>> = std::sync::Mutex::new(None);
    if let Some(r) = CACHE.lock().unwrap().as_ref().and_then(|m| m.get(name)) { return r.clone(); }
    let r = stdlib_file_scan(name);
    CACHE.lock().unwrap().get_or_insert_with(Default::default).insert(name.to_string(), r.clone());
    r
}
fn stdlib_file_scan(name: &str) -> Option<std::path::PathBuf> {
    if EXT_BACKED.contains(&name.split('/').next().unwrap_or(name).trim_end_matches(".rb")) { return None; }
    let (lib, gem) = stdlib_dirs()?;
    let rb = if name.ends_with(".rb") { name.to_string() } else { format!("{}.rb", name) };
    let top = name.split('/').next().unwrap_or(name).trim_end_matches(".rb");
    if GEM_FIRST.contains(&top) { if let Some(f) = newest_gem_file(&gem, top, &rb) { return Some(f); } }
    let p = std::path::Path::new(&lib).join(&rb);
    if p.is_file() { return Some(p); }
    // net/ftp lives in the bundled gem net-ftp
    let dashed = name.trim_end_matches(".rb").split('/').take(2).collect::<Vec<_>>().join("-");
    let top = if BUNDLED.contains(&top) { top } else if BUNDLED.contains(&dashed.as_str()) { BUNDLED.iter().copied().find(|b| *b == dashed).unwrap() } else { return None };
    let mut best: Option<std::path::PathBuf> = None;
    for e in std::fs::read_dir(std::path::Path::new(&gem).join("gems")).ok()?.flatten() {
        let n = e.file_name().to_string_lossy().into_owned();
        let Some(ver) = n.strip_prefix(top).and_then(|r| r.strip_prefix('-')) else { continue };
        if !ver.starts_with(|ch: char| ch.is_ascii_digit()) { continue; }
        let f = e.path().join("lib").join(&rb);
        if f.is_file() && best.as_ref().map(|b| *b < f).unwrap_or(true) { best = Some(f); }
    }
    best
}

/// The reference ruby's default $LOAD_PATH: site, vendor, then rubylibdir and archdir (require resolves the
/// last two through stdlib_file, after saruby's own features).
pub fn default_load_path() -> Vec<String> {
    let Some((lib, _)) = stdlib_dirs() else { return vec![] };
    let p = std::path::Path::new(&lib);
    let (base, ver) = (p.parent().map(|d| d.to_string_lossy().into_owned()).unwrap_or_default(), p.file_name().map(|v| v.to_string_lossy().into_owned()).unwrap_or_default());
    let arch = format!("{}-linux", if cfg!(target_arch = "aarch64") { "aarch64" } else { "x86_64" });
    vec![format!("{base}/site_ruby/{ver}"), format!("{base}/site_ruby/{ver}/{arch}"), format!("{base}/site_ruby"), format!("{base}/vendor_ruby/{ver}"),
         format!("{base}/vendor_ruby/{ver}/{arch}"), format!("{base}/vendor_ruby"), lib.clone(), format!("{lib}/{arch}")]
}
/// $LOAD_PATH: -I dirs (already added), RUBYLIB, the reference ruby's default load path.
pub fn setup_load_path(c: &mut Ctx) {
    let lp = c.vm().load_path.get();
    if let Ok(p) = std::env::var("RUBYLIB") {
        for d in p.split(':') { let s = c.str_new(d.as_bytes()); c.ary_push(lp, s); }
    }
    // gem_prelude marks each default entry with itself (rubygems finds its insert position by it)
    for d in default_load_path() { let s = c.str_new(d.as_bytes()); crate::vm::ivar_set_raw(c, s, crate::symbol::intern(b"@gem_prelude_index"), s); c.ary_push(lp, s); }
}
/// rubylibdir and archdir of the reference ruby as the expanded load path has them (realpath).
pub fn stdlib_lp_dirs() -> &'static [String] {
    static D: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
    D.get_or_init(|| default_load_path().into_iter().skip(6).map(|d| std::fs::canonicalize(&d).map(|p| p.to_string_lossy().into_owned()).unwrap_or(d)).collect())
}

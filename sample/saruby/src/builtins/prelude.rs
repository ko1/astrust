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
        Err(msg) => return err(c, c.vm().c_syntax_error, msg),
    };
    crate::run_top_iseq(c, is, c.vm().main_obj)
}

macro_rules! lib { ($n:literal) => { ($n, include_str!(concat!("../../prelude/lib/", $n, ".rb"))) }; }
/// Features implemented by saruby itself: Ruby source compiled on first require.
const LIBS: &[(&str, &str)] = &[lib!("etc"), lib!("timeout"), lib!("monitor"), lib!("time"), lib!("bundler"), ("bundler/setup", "require \"bundler\"; Bundler.setup\n"), lib!("date"), ("date_core", "require \"date\"\n"), lib!("pathname"), ("saruby/pathname", include_str!("../../prelude/lib/pathname_core.rb")), ("saruby/argf", include_str!("../../prelude/lib/argf.rb"))];
/// Features whose implementation is native and always present (require is a no-op).
const NATIVE: &[&str] = &["set", "stringio", "rbconfig", "thread", "fiber", "enumerator", "rational", "complex", "continuation",
    "io/wait", "io/nonblock", "fcntl", "etc.so", "monitor.so", "strscan", "strscan.so", "psych.so", "json", "json/ext", "saruby/errno", "saruby/env", "saruby/rbconfig", "saruby/filetest", "saruby/stat", "saruby/sync", "saruby/econv", "saruby/tracepoint"];

/// Constant `name` under `owner` whose native definition runs on first reference (boot time: builtins/mod.rs).
pub use crate::builtins::lazy_const;
pub fn has_feature(name: &str) -> bool { NATIVE.contains(&name) || LIBS.iter().any(|l| l.0 == name) }
pub fn load_feature(c: &mut Ctx, name: &str) -> RV {
    if let Some(l) = LIBS.iter().find(|l| l.0 == name) {
        let file: &'static str = Box::leak(format!("<internal:{}>", name).into_boxed_str());
        return crate::builtins::kernel::run_source(c, l.1.as_bytes(), file, 1);
    }
    // native libraries whose classes are only built on first require
    if name == "strscan" || name == "strscan.so" { crate::builtins::strscan::init(c); }
    if name == "json" || name == "json/ext" { crate::builtins::json::init(c); }
    if name == "psych.so" { crate::builtins::psych::init(c)?; }
    match name {
        "saruby/errno" => crate::builtins::exception::define_all_errnos(c),
        "saruby/env" => crate::builtins::env::init(c),
        "saruby/rbconfig" | "rbconfig" => crate::builtins::misc::rbconfig_init(c),
        "saruby/filetest" => crate::builtins::file::init_filetest(c),
        "saruby/stat" => crate::builtins::file::init_stat(c),
        "saruby/sync" => crate::builtins::thread::init_sync(c),
        "saruby/econv" => crate::builtins::transcode::init(c),
        "saruby/tracepoint" => crate::builtins::tracepoint::init_class(c),
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
const BUNDLED: &[&str] = &["ostruct", "base64", "benchmark", "logger", "observer", "abbrev", "pstore", "csv", "matrix", "prime", "getoptlong", "mutex_m", "drb", "rinda", "racc", "bigdecimal", "nkf", "syslog", "rexml", "rss"];
const EXT_BACKED: &[&str] = &["socket", "openssl", "date", "zlib", "ripper", "bigdecimal", "digest", "pathname", "pty", "coverage",
    "objspace", "readline", "reline", "nkf", "fiddle", "syslog", "net", "open-uri", "resolv", "ipaddr", "io"];
/// Default gems that the reference ruby (rubygems) activates from a newer installed gem.
const GEM_FIRST: &[&str] = &["psych"];
/// lib/<rb> of the highest installed version of gem `top`.
fn newest_gem_file(gem: &str, top: &str, rb: &str) -> Option<std::path::PathBuf> {
    let ver = |v: &str| -> Vec<u64> { v.split('.').map(|x| x.parse().unwrap_or(0)).collect() };
    let mut best: Option<(Vec<u64>, std::path::PathBuf)> = None;
    for e in std::fs::read_dir(std::path::Path::new(gem).join("gems")).ok()?.flatten() {
        let n = e.file_name().to_string_lossy().into_owned();
        let Some(v) = n.strip_prefix(top).and_then(|r| r.strip_prefix('-')) else { continue };
        if !v.chars().all(|ch| ch.is_ascii_digit() || ch == '.') { continue; }
        let f = e.path().join("lib").join(rb);
        if f.is_file() && best.as_ref().map(|b| b.0 < ver(v)).unwrap_or(true) { best = Some((ver(v), f)); }
    }
    best.map(|b| b.1)
}
pub fn stdlib_file(name: &str) -> Option<std::path::PathBuf> {
    if EXT_BACKED.contains(&name.split('/').next().unwrap_or(name).trim_end_matches(".rb")) { return None; }
    let (lib, gem) = stdlib_dirs()?;
    let rb = if name.ends_with(".rb") { name.to_string() } else { format!("{}.rb", name) };
    let top = name.split('/').next().unwrap_or(name).trim_end_matches(".rb");
    if GEM_FIRST.contains(&top) { if let Some(f) = newest_gem_file(&gem, top, &rb) { return Some(f); } }
    let p = std::path::Path::new(&lib).join(&rb);
    if p.is_file() { return Some(p); }
    if !BUNDLED.contains(&top) { return None; }
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

/// $LOAD_PATH: -I dirs (already added), RUBYLIB.
pub fn setup_load_path(c: &mut Ctx) {
    if let Ok(p) = std::env::var("RUBYLIB") {
        for d in p.split(':') { let s = c.str_new(d.as_bytes()); let lp = c.vm().load_path; c.ary_push(lp, s); }
    }
}

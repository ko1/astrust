// Ruby-level prelude and lazily-loaded built-in features.
use crate::value::*;
use crate::vm::*;

const CORE: &str = include_str!("../../prelude/core.rb");

pub fn load_prelude(c: &mut Ctx) -> RV {
    crate::builtins::kernel::run_source(c, CORE.as_bytes(), "<internal:prelude>", 1)
}

macro_rules! lib { ($n:literal) => { ($n, include_str!(concat!("../../prelude/lib/", $n, ".rb"))) }; }
/// Features implemented by koruby itself: Ruby source compiled on first require.
const LIBS: &[(&str, &str)] = &[lib!("etc"), lib!("timeout"), lib!("monitor"), lib!("time"), ("koruby/argf", include_str!("../../prelude/lib/argf.rb"))];
/// Features whose implementation is native and always present (require is a no-op).
const NATIVE: &[&str] = &["set", "stringio", "rbconfig", "thread", "fiber", "enumerator", "rational", "complex", "continuation",
    "io/wait", "io/nonblock", "fcntl", "etc.so", "monitor.so", "strscan", "strscan.so", "json", "json/ext"];

pub fn has_feature(name: &str) -> bool { NATIVE.contains(&name) || LIBS.iter().any(|l| l.0 == name) }
pub fn load_feature(c: &mut Ctx, name: &str) -> RV {
    if let Some(l) = LIBS.iter().find(|l| l.0 == name) {
        let file: &'static str = Box::leak(format!("<internal:{}>", name).into_boxed_str());
        return crate::builtins::kernel::run_source(c, l.1.as_bytes(), file, 1);
    }
    // native libraries whose classes are only built on first require
    if name == "strscan" || name == "strscan.so" { crate::builtins::strscan::init(); }
    if name == "json" || name == "json/ext" { crate::builtins::json::init(); }
    Ok(NIL)
}

/// (rubylibdir, gem dir) of the reference ruby: its pure-Ruby standard library is used as is.
pub fn stdlib_dirs() -> Option<(String, String)> {
    let v = std::env::var("KORUBY_STDLIB").ok().or_else(|| option_env!("KORUBY_STDLIB").map(|s| s.to_string()))?;
    let (lib, gem) = v.split_once(':')?;
    Some((lib.to_string(), gem.to_string()))
}
/// Bundled gems that live outside rubylibdir.
const BUNDLED: &[&str] = &["ostruct", "base64", "benchmark", "logger", "observer", "abbrev", "pstore", "csv", "matrix", "prime", "getoptlong", "mutex_m", "drb", "rinda", "racc", "bigdecimal", "nkf", "syslog", "rexml", "rss"];
pub fn stdlib_file(name: &str) -> Option<std::path::PathBuf> {
    let (lib, gem) = stdlib_dirs()?;
    let rb = if name.ends_with(".rb") { name.to_string() } else { format!("{}.rb", name) };
    let p = std::path::Path::new(&lib).join(&rb);
    if p.is_file() { return Some(p); }
    let top = name.split('/').next().unwrap_or(name).trim_end_matches(".rb");
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
pub fn setup_load_path() {
    if let Ok(p) = std::env::var("RUBYLIB") {
        for d in p.split(':') { let s = crate::string::str_new(d.as_bytes()); crate::array::ary_push(vm().load_path, s); }
    }
}

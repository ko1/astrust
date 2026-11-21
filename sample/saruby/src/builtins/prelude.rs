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

pub fn load_prelude(c: &mut Ctx) -> RV {
    crate::builtins::kernel::run_source(c, CORE.as_bytes(), "<internal:prelude>", 1)
}

macro_rules! lib { ($n:literal) => { ($n, include_str!(concat!("../../prelude/lib/", $n, ".rb"))) }; }
/// Features implemented by saruby itself: Ruby source compiled on first require.
const LIBS: &[(&str, &str)] = &[lib!("etc"), lib!("timeout"), lib!("monitor"), lib!("time"), ("saruby/argf", include_str!("../../prelude/lib/argf.rb"))];
/// Features whose implementation is native and always present (require is a no-op).
const NATIVE: &[&str] = &["set", "stringio", "rbconfig", "thread", "fiber", "enumerator", "rational", "complex", "continuation",
    "io/wait", "io/nonblock", "fcntl", "etc.so", "monitor.so", "strscan", "strscan.so", "json", "json/ext", "saruby/errno", "saruby/env", "saruby/rbconfig", "saruby/filetest", "saruby/stat", "saruby/sync"];

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
    match name {
        "saruby/errno" => crate::builtins::exception::define_all_errnos(c),
        "saruby/env" => crate::builtins::env::init(c),
        "saruby/rbconfig" | "rbconfig" => crate::builtins::misc::rbconfig_init(c),
        "saruby/filetest" => crate::builtins::file::init_filetest(),
        "saruby/stat" => crate::builtins::file::init_stat(),
        "saruby/sync" => crate::builtins::thread::init_sync(c),
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
const EXT_BACKED: &[&str] = &["socket", "openssl", "date", "zlib", "ripper", "psych", "yaml", "bigdecimal", "digest", "pathname", "pty", "coverage",
    "objspace", "readline", "reline", "nkf", "fiddle", "syslog", "net", "open-uri", "resolv", "ipaddr", "io"];
pub fn stdlib_file(name: &str) -> Option<std::path::PathBuf> {
    if EXT_BACKED.contains(&name.split('/').next().unwrap_or(name).trim_end_matches(".rb")) { return None; }
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
pub fn setup_load_path(c: &mut Ctx) {
    if let Ok(p) = std::env::var("RUBYLIB") {
        for d in p.split(':') { let s = c.str_new(d.as_bytes()); let lp = c.vm().load_path; c.ary_push(lp, s); }
    }
}

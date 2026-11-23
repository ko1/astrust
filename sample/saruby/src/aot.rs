// saruby's use of the ASTrust code store (runtime/astrust.rs TieredStore): every compiled
// body (method / block / class / top) is an AOT entry; the prelude's bodies go to the
// shared preload_store next to the binary, a program's to code_store/ (or --code-store).
use crate::astrust::*;
use crate::node::*;
use crate::vm::*;

const PRELUDE_FILE: &str = "<internal:prelude>";
thread_local! {
    // leaked, as the static it replaces was never dropped; no drop glue, so atexit still sees it
    static STORE: std::cell::RefCell<Option<&'static mut TieredStore>> = const { std::cell::RefCell::new(None) };
}
/// `f(store)`; None under --plain.  A nested call panics (RefCell) instead of aliasing.
fn with_store<R>(f: impl FnOnce(&mut TieredStore) -> R) -> Option<R> { STORE.with_borrow_mut(|s| s.as_deref_mut().map(f)) }

fn exe_dir() -> std::path::PathBuf { std::env::current_exe().unwrap().parent().unwrap().to_path_buf() }

/// Called once at startup.  `program_dir` None = --plain (no code store at all).
pub fn init(program_dir: Option<&str>, collect: bool) {
    let mut st = TieredStore::new(exe_dir().join("preload_store"), program_dir.map(Into::into), env!("SARUBY_BUILD_ID"), collect);
    // SARUBY_PATCH=0: every body keeps the shared SD of all.so (no patch binding)
    st.patch = std::env::var("SARUBY_PATCH").map_or(true, |v| v != "0");
    if std::env::var_os("SARUBY_PATCH_STATS").is_some() {
        // SAFETY: registers a plain extern "C" fn with no captured state.
        unsafe { atexit(print_patch_stats); }
    }
    STORE.set(Some(Box::leak(Box::new(st))));
}

extern "C" { fn atexit(f: extern "C" fn()) -> std::os::raw::c_int; }
extern "C" fn print_patch_stats() {
    if let Some((n, failed, bytes)) = with_store(|st| st.patch_stats()) {
        eprintln!("saruby: patch bound={} failed={} bytes={}", n, failed, bytes);
    }
}

/// A body's dispatcher until its first dispatch: bind its own copy (or keep the shared SD), then run it.
fn first_dispatch(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV {
    let p = with_store(|st| st.bind(n as usize, env!("SARUBY_BUILD_ID"), first_dispatch as DispatchFn as usize, hole_value)).flatten();
    let Some(p) = p else { unreachable!("first_dispatch on a node that was not deferred") };
    // SAFETY: p is the entry of an SD for this node's shape, with the dispatcher's signature.
    let d = unsafe { std::mem::transmute::<*const std::os::raw::c_void, DispatchFn>(p) };
    // SAFETY: n is the node this dispatcher was installed on (a live, never freed node).
    unsafe { (*n).head.dispatcher = d; }
    d(c, n, fp)
}

/// A hole's value for the loader: a line as `Pos` keeps a bound one (vm.rs), an imm32 naming the node.
fn hole_value(op: &str, root: usize, off: usize) -> Option<u64> {
    if op != "line" { return None; }
    // SAFETY: off is a hole of this entry's binding descriptor: a u32 inside the laid-out tree at root,
    // which is never freed.
    let line: &'static u32 = unsafe { &*((root + off) as *const u32) };
    Pos::bind(line)
}

thread_local! {
    // a program's blocks and keyword methods that have an entry node over their body: the body is baked only if it ran on its own
    static BLOCKS: std::cell::RefCell<Vec<&'static Iseq>> = const { std::cell::RefCell::new(Vec::new()) };
}

static SD_NAMES: std::sync::OnceLock<bool> = std::sync::OnceLock::new();

pub fn register_iseq(is: &'static Iseq, src_key: &dyn Fn() -> u64) { with_store(|st| register_in(st, is, src_key)); }

fn register_in(st: &mut TieredStore, is: &'static Iseq, src_key: &dyn Fn() -> u64) {
    if st.program.is_none() { return; }   // --plain
    let prelude = is.info.file == PRELUDE_FILE;
    // an entry's program key in hopt_index.txt: its source text's hash (by content, not path: the same file
    // run from another path or as a copy finds its own Hopt), line and kind.  The prelude is part of the
    // build and its store is per build, so its text needs no hash.
    let src = std::cell::OnceCell::new();
    let src = || *src.get_or_init(|| if prelude { 0 } else { src_key() });
    let mut reg = |body: NodePtr, kind: u64, what: &str| {
        // SARUBY_SD_NAMES=1: which body each SD_<hash> is (to read a perf profile)
        if *SD_NAMES.get_or_init(|| std::env::var_os("SARUBY_SD_NAMES").is_some()) { eprintln!("SD_{:016x} {}{} {}:{}", hash_node(body), is.info.label, what, is.info.file, is.info.line); }
        // baked code assumes the layout of layout_tree (finish_iseq)
        if !node_laid(body) { return; }
        let key = || hash_merge(hash_merge(src(), is.info.line as u64), kind);
        let found = st.register(body as usize, || hash_node(body), key, prelude, env!("SARUBY_BUILD_ID"));
        if let Some((p, name)) = found {
            // patch binding: the body gets its own copy when it is first dispatched (first_dispatch)
            if st.defer(body as usize, name, prelude, p) {
                unsafe { (*body).head.dispatcher = first_dispatch; }
            } else {
                unsafe { (*body).head.dispatcher = std::mem::transmute::<*const std::os::raw::c_void, DispatchFn>(p); }
            }
            vm().sd_hits.set(vm().sd_hits.get() + 1);
        }
    };
    reg(if is.entry.is_null() { is.body } else { is.entry }, 0, "");
    if !is.bentry.is_null() || !is.kentry.is_null() {
        reg(if is.kentry.is_null() { is.bentry } else { is.kentry }, 1, " (entry)");
        if !prelude { BLOCKS.with(|b| b.borrow_mut().push(is)); }
    }
    if !is.dentry.is_null() { reg(is.dentry, 2, " (defaults entry)"); }
}

pub fn bake_config() -> BakeConfig {
    let dir = exe_dir();
    let sys = std::path::PathBuf::from(env!("SARUBY_RUSTC_SYSROOT")).join("lib/rustlib").join(env!("SARUBY_TARGET")).join("lib");
    BakeConfig {
        crate_name: "saruby".into(), crate_path: dir.join("libsaruby.so"),
        lib_dirs: vec![dir.clone(), dir.join("deps"), dir.join("target/release/deps")], rpaths: vec![sys, dir],
        prelude: "extern crate saruby;\nuse saruby::*;\nuse saruby::node::*;\nuse saruby::value::*;\nuse saruby::vm::*;\n".into(),
        build_id: env!("SARUBY_BUILD_ID").into(), verbose: std::env::var_os("SARUBY_AOT_VERBOSE").is_some(),
    }
}

pub fn bake(store_dir: &str) -> Result<usize, String> { with_store(|st| bake_in(st, store_dir)).ok_or("no code store")? }

fn bake_in(st: &mut TieredStore, store_dir: &str) -> Result<usize, String> {
    // a body that only ran through its entry node needs no separate copy
    let skip: Vec<usize> = BLOCKS.with(|b| b.borrow().iter().filter(|is| !is.body_used.get()).map(|is| is.body as usize).collect());
    st.program_entries.retain(|e| !skip.contains(e));
    // an entry's SD is named by its structure and what the bake reads from the caches (profile_node)
    let mut hopt = std::collections::HashMap::new();
    for &e in st.prelude_entries.iter().chain(st.program_entries.iter()) {
        hopt.insert(e, hash_merge(hash_node(e as NodePtr), profile_node(&mut ProfCtx::new(), e as NodePtr)));
    }
    st.bake(std::path::Path::new(store_dir), &bake_config(), |entries, patch| {
        let mut sc = SpecCtx::new();
        sc.patch = patch;
        sc.profiled = true;
        for &e in entries { sc.set_profile(hopt[&e]); specialize_entry(&mut sc, e as NodePtr); }
        sc.set_profile(0);
        specialize_finish(&mut sc);
        sc.out
    }, |e| (hash_node(e as NodePtr), hopt[&e]))
}

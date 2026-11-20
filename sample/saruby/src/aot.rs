// saruby's use of the ASTrust code store (runtime/astrust.rs TieredStore): every compiled
// body (method / block / class / top) is an AOT entry; the prelude's bodies go to the
// shared preload_store next to the binary, a program's to code_store/ (or --code-store).
use crate::astrust::*;
use crate::node::*;
use crate::vm::*;

const PRELUDE_FILE: &str = "<internal:prelude>";
pub static mut STORE: Option<TieredStore> = None;

fn exe_dir() -> std::path::PathBuf { std::env::current_exe().unwrap().parent().unwrap().to_path_buf() }

/// Called once at startup.  `program_dir` None = --plain (no code store at all).
#[allow(static_mut_refs)]
pub fn init(program_dir: Option<&str>, collect: bool) {
    unsafe { STORE = Some(TieredStore::new(exe_dir().join("preload_store"), program_dir.map(Into::into), env!("SARUBY_BUILD_ID"), collect)); }
}

thread_local! {
    // a program's blocks that have an entry node: their bodies are baked only if they ran on their own
    static BLOCKS: std::cell::RefCell<Vec<&'static Iseq>> = const { std::cell::RefCell::new(Vec::new()) };
}

#[allow(static_mut_refs)]
pub fn register_iseq(is: &'static Iseq) {
    let Some(st) = (unsafe { STORE.as_mut() }) else { return };
    let prelude = is.info.file == PRELUDE_FILE;
    let mut reg = |body: NodePtr, what: &str| {
        // SARUBY_SD_NAMES=1: which body each SD_<hash> is (to read a perf profile)
        if std::env::var_os("SARUBY_SD_NAMES").is_some() { eprintln!("SD_{:016x} {}{} {}:{}", hash_node(body), is.info.label, what, is.info.file, is.info.line); }
        if let Some(p) = st.register(body as usize, hash_node(body), prelude, env!("SARUBY_BUILD_ID")) {
            unsafe { (*body).head.dispatcher = std::mem::transmute::<*const std::os::raw::c_void, DispatchFn>(p); }
            vm().sd_hits += 1;
        }
    };
    reg(if is.entry.is_null() { is.body } else { is.entry }, "");
    if !is.bentry.is_null() {
        reg(is.bentry, " (entry)");
        if !prelude { BLOCKS.with(|b| b.borrow_mut().push(is)); }
    }
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

#[allow(static_mut_refs)]
pub fn bake(store_dir: &str) -> Result<usize, String> {
    let st = unsafe { STORE.as_mut() }.ok_or("no code store")?;
    // a block that only ran through its entry node needs no separate copy of its body
    let skip: Vec<usize> = BLOCKS.with(|b| b.borrow().iter().filter(|is| !is.body_used.get()).map(|is| is.body as usize).collect());
    st.program_entries.retain(|e| !skip.contains(e));
    st.bake(std::path::Path::new(store_dir), &bake_config(), |entries| {
        let mut sc = SpecCtx::new();
        for &e in entries { specialize_entry(&mut sc, e as NodePtr); }
        sc.out
    })
}

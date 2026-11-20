// koruby's use of the ASTrust code store (runtime/astrust.rs TieredStore): every compiled
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
    unsafe { STORE = Some(TieredStore::new(exe_dir().join("preload_store"), program_dir.map(Into::into), env!("KORUBY_BUILD_ID"), collect)); }
}

#[allow(static_mut_refs)]
pub fn register_iseq(is: &'static Iseq) {
    let Some(st) = (unsafe { STORE.as_mut() }) else { return };
    let body = if is.entry.is_null() { is.body } else { is.entry };
    // KORUBY_SD_NAMES=1: which body each SD_<hash> is (to read a perf profile)
    if std::env::var_os("KORUBY_SD_NAMES").is_some() { eprintln!("SD_{:016x} {} {}:{}", hash_node(body), is.info.label, is.info.file, is.info.line); }
    if let Some(p) = st.register(body as usize, hash_node(body), is.info.file == PRELUDE_FILE, env!("KORUBY_BUILD_ID")) {
        unsafe { (*body).head.dispatcher = std::mem::transmute::<*const std::os::raw::c_void, DispatchFn>(p); }
        vm().sd_hits += 1;
    }
}

pub fn bake_config() -> BakeConfig {
    let dir = exe_dir();
    let sys = std::path::PathBuf::from(env!("KORUBY_RUSTC_SYSROOT")).join("lib/rustlib").join(env!("KORUBY_TARGET")).join("lib");
    BakeConfig {
        crate_name: "koruby".into(), crate_path: dir.join("libkoruby.so"),
        lib_dirs: vec![dir.clone(), dir.join("deps"), dir.join("target/release/deps")], rpaths: vec![sys, dir],
        prelude: "extern crate koruby;\nuse koruby::*;\nuse koruby::node::*;\nuse koruby::value::*;\nuse koruby::vm::*;\n".into(),
        build_id: env!("KORUBY_BUILD_ID").into(), verbose: std::env::var_os("KORUBY_AOT_VERBOSE").is_some(),
    }
}

#[allow(static_mut_refs)]
pub fn bake(store_dir: &str) -> Result<usize, String> {
    let st = unsafe { STORE.as_mut() }.ok_or("no code store")?;
    st.bake(std::path::Path::new(store_dir), &bake_config(), |entries| {
        let mut sc = SpecCtx::new();
        for &e in entries { specialize_entry(&mut sc, e as NodePtr); }
        sc.out
    })
}

// AOT glue: every compiled body (method / block / class / top) is an entry point.
// --aot-compile collects them and bakes SD_<hash> for each at exit; at load, a
// body whose hash is in the code store gets its dispatcher swapped to the SD.
use crate::astrust::*;
use crate::node::*;
use crate::vm::*;

pub fn register_iseq(is: &'static Iseq) {
    let vm = vm();
    if vm.aot_collect { vm.aot_entries.push(is.body); }
    if let Some(cs) = &vm.code_store {
        if cs.loaded() {
            if let Some(p) = cs.lookup(hash_node(is.body)) {
                unsafe { (*is.body).head.dispatcher = std::mem::transmute::<*const std::os::raw::c_void, DispatchFn>(p); }
                vm.sd_hits += 1;
            }
        }
    }
}

pub fn bake_config() -> BakeConfig {
    let exe = std::env::current_exe().unwrap();
    let dir = exe.parent().unwrap().to_path_buf();
    let sys = std::path::PathBuf::from(env!("KORUBY_RUSTC_SYSROOT")).join("lib/rustlib").join(env!("KORUBY_TARGET")).join("lib");
    BakeConfig {
        crate_name: "koruby".into(), crate_path: dir.join("libkoruby.so"),
        lib_dirs: vec![dir.clone(), dir.join("deps")], rpaths: vec![sys, dir],
        prelude: "extern crate koruby;\nuse koruby::*;\nuse koruby::node::*;\nuse koruby::value::*;\nuse koruby::vm::*;\n".into(),
        build_id: env!("KORUBY_BUILD_ID").into(), verbose: std::env::var_os("KORUBY_AOT_VERBOSE").is_some(),
    }
}

pub fn bake(store_dir: &str) -> Result<usize, String> {
    let vm = vm();
    let mut sc = SpecCtx::new();
    let entries = std::mem::take(&mut vm.aot_entries);
    for &e in entries.iter() { specialize_entry(&mut sc, e); }
    std::fs::create_dir_all(store_dir).map_err(|e| e.to_string())?;
    let cs = CodeStore::new(store_dir);
    let added = cs.add_items(&sc.out).map_err(|e| e.to_string())?;
    cs.build(&bake_config())?;
    Ok(added)
}

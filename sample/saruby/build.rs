// Builds the vendored prism (C) into a static library, and bakes rpaths so the
// binary finds libsaruby.so ($ORIGIN) and the shared libstd (sysroot).
use std::process::Command;
use std::path::PathBuf;

fn main() {
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let src = PathBuf::from("vendor/prism/src");
    let inc = PathBuf::from("vendor/prism/include");
    let lib = out.join("libprism.a");
    let mut objs = vec![];
    if !lib.exists() {
    for e in std::fs::read_dir(&src).unwrap() {
        let p = e.unwrap().path();
        if p.extension().map(|x| x == "c").unwrap_or(false) {
            let o = out.join(p.file_stem().unwrap()).with_extension("o");
            let st = Command::new("gcc")
                .args(["-O2", "-fPIC", "-std=c99", "-w", "-c"])
                .arg("-I").arg(&inc)
                .arg(&p).arg("-o").arg(&o).status().unwrap();
            assert!(st.success(), "prism compile failed: {:?}", p);
            objs.push(o);
        }
    }
    let st = Command::new("ar").arg("crs").arg(&lib).args(&objs).status().unwrap();
    assert!(st.success());
    }
    // the reference ruby's stdlib: pure-Ruby libraries are loaded from there on require
    if let Ok(o) = Command::new("ruby").args(["-e", "print RbConfig::CONFIG['rubylibdir'], ':', Gem.default_dir"]).output() {
        if o.status.success() { println!("cargo:rustc-env=SARUBY_STDLIB={}", String::from_utf8_lossy(&o.stdout)); }
    }
    println!("cargo:rustc-link-search=native={}", out.display());
    println!("cargo:rustc-link-lib=static=prism");
    println!("cargo:rerun-if-changed=vendor/prism/src");
    println!("cargo:rerun-if-changed=build.rs");
    serialize_prelude(&out, &inc, &lib);

    let sysroot = String::from_utf8(Command::new(std::env::var("RUSTC").unwrap_or("rustc".into()))
        .args(["--print", "sysroot"]).output().unwrap().stdout).unwrap();
    let target = std::env::var("TARGET").unwrap();
    println!("cargo:rustc-link-arg=-Wl,-rpath,{}/lib/rustlib/{}/lib", sysroot.trim(), target);
    println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN");
    println!("cargo:rustc-link-arg=-ldl");
    // libsaruby's references to its own symbols are bound at link time (RELATIVE, not looked up by name at startup)
    println!("cargo:rustc-link-arg=-Wl,-Bsymbolic");
    println!("cargo:rustc-env=SARUBY_RUSTC_SYSROOT={}", sysroot.trim());
    println!("cargo:rustc-env=SARUBY_TARGET={}", target);
    // Build id = hash of the Rust sources and the GC backend (its allocation path is inlined
    // into baked code): AOT code baked against another build is refused.
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=../../runtime/gc");
    // the code store emits the SD crates and loads patch.o
    println!("cargo:rerun-if-changed=../../runtime/astrust.rs");
    println!("cargo:rerun-if-changed=../../runtime/astrust_loader.rs");
    let mut h: u64 = 0xcbf29ce484222325;
    // every cargo feature changes the ABI of baked code (GC backend, stack backend)
    let mut feats: Vec<String> = std::env::vars().map(|(k, _)| k).filter(|k| k.starts_with("CARGO_FEATURE_")).collect();
    feats.sort();
    for b in feats.join(",").bytes() { h = (h ^ b as u64).wrapping_mul(0x100000001b3); }
    let mut files: Vec<_> = walk(PathBuf::from("src"));
    files.extend(walk(PathBuf::from("../../runtime/gc")));
    files.push(PathBuf::from("../../runtime/astrust.rs"));
    files.push(PathBuf::from("../../runtime/astrust_loader.rs"));
    files.sort();
    for f in files { for b in std::fs::read(&f).unwrap() { h = (h ^ b as u64).wrapping_mul(0x100000001b3); } }
    let id = format!("{:016x}", h);
    println!("cargo:rustc-env=SARUBY_BUILD_ID={}", id);
}

fn walk(d: PathBuf) -> Vec<PathBuf> {
    let mut v = vec![];
    for e in std::fs::read_dir(d).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() { v.extend(walk(p)); } else if p.extension().map(|x| x == "rs").unwrap_or(false) { v.push(p); }
    }
    v
}

include!("src/prism_opts.rs");

/// OUT_DIR/core.prism: the prelude parsed as prism.rs `parse` would at startup (builtins/prelude.rs).
fn serialize_prelude(out: &std::path::Path, inc: &std::path::Path, lib: &std::path::Path) {
    let exe = out.join("prism_serialize");
    let st = Command::new("gcc").args(["-O2", "-std=c99", "-w"]).arg("-I").arg(inc)
        .arg("tools/prism_serialize.c").arg(lib).arg("-o").arg(&exe).status().unwrap();
    assert!(st.success(), "tools/prism_serialize.c: compile failed");
    let opts = out.join("core.options");
    let o = ParseOpts { filepath: b"<internal:prelude>", line: 1, scopes: vec![], main_script: false, frozen_string_literal: 0 };
    std::fs::write(&opts, build_options(&o)).unwrap();
    let st = Command::new(&exe).arg("prelude/core.rb").arg(&opts).arg(out.join("core.prism")).status().unwrap();
    assert!(st.success(), "prism_serialize prelude/core.rb failed");
    println!("cargo:rerun-if-changed=prelude/core.rb");
    println!("cargo:rerun-if-changed=tools/prism_serialize.c");
}

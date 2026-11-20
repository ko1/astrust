use std::process::Command;
fn main() {
    let sysroot = String::from_utf8(Command::new(std::env::var("RUSTC").unwrap_or("rustc".into()))
        .args(["--print", "sysroot"]).output().unwrap().stdout).unwrap();
    let target = std::env::var("TARGET").unwrap();
    // Only the `are` binary needs these (libstd.so from the sysroot, -ldl for the code store).
    println!("cargo:rustc-link-arg-bins=-Wl,-rpath,{}/lib/rustlib/{}/lib", sysroot.trim(), target);
    println!("cargo:rustc-link-arg-bins=-ldl");
    println!("cargo:rustc-env=ASTROGRE_SYSROOT={}", sysroot.trim());
    println!("cargo:rustc-env=ASTROGRE_TARGET={}", target);
    println!("cargo:rerun-if-changed=src");
    let mut h: u64 = 0xcbf29ce484222325;
    let mut files: Vec<_> = std::fs::read_dir("src").unwrap().map(|e| e.unwrap().path()).collect();
    files.sort();
    for f in files { for b in std::fs::read(f).unwrap() { h = (h ^ b as u64).wrapping_mul(0x100000001b3); } }
    println!("cargo:rustc-env=ASTROGRE_BUILD_ID={:016x}", h);
}

use std::process::Command;
fn main() {
    let sysroot = String::from_utf8(Command::new(std::env::var("RUSTC").unwrap_or("rustc".into()))
        .args(["--print", "sysroot"]).output().unwrap().stdout).unwrap();
    let target = std::env::var("TARGET").unwrap();
    println!("cargo:rustc-link-arg=-Wl,-rpath,{}/lib/rustlib/{}/lib", sysroot.trim(), target);
    println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN");
    println!("cargo:rustc-link-arg=-ldl");
    println!("cargo:rustc-env=CALC_SYSROOT={}", sysroot.trim());
    println!("cargo:rustc-env=CALC_TARGET={}", target);
    println!("cargo:rerun-if-changed=src");
    let mut h: u64 = 0xcbf29ce484222325;
    for e in std::fs::read_dir("src").unwrap() { for b in std::fs::read(e.unwrap().path()).unwrap() { h = (h ^ b as u64).wrapping_mul(0x100000001b3); } }
    println!("cargo:rustc-env=CALC_BUILD_ID={:016x}", h);
}

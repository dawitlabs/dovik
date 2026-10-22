use std::path::PathBuf;
use std::process::Command;

fn main() {
    let source = PathBuf::from("runtime-c/dovik_rt.c");
    let out = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR")).join("dovik_rt.o");

    println!("cargo:rerun-if-changed=runtime-c/dovik_rt.c");
    println!("cargo:rerun-if-changed=build.rs");

    let status = Command::new("clang")
        .args(["-O2", "-flto", "-fno-plt", "-c"])
        .arg(&source)
        .arg("-o")
        .arg(&out)
        .status()
        .expect("clang is required to build the Dovik runtime");

    assert!(status.success(), "failed to compile {}", source.display());
    println!("cargo:rustc-env=DOVIK_RUNTIME_OBJECT={}", out.display());
}

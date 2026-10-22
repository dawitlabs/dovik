use std::fmt::Write as _;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let out = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
    let header = out.join("dovik_case.h");
    std::fs::write(&header, case_tables()).expect("write the case tables");

    let source = PathBuf::from("runtime-c/dovik_rt.c");
    let object = out.join("dovik_rt.o");

    println!("cargo:rerun-if-changed=runtime-c/dovik_rt.c");
    println!("cargo:rerun-if-changed=build.rs");

    let status = Command::new("clang")
        .args(["-O2", "-flto", "-fno-plt", "-ffunction-sections", "-fdata-sections", "-c"])
        .arg("-I")
        .arg(&out)
        .arg(&source)
        .arg("-o")
        .arg(&object)
        .status()
        .expect("clang is required to build the Dovik runtime");

    assert!(status.success(), "failed to compile {}", source.display());
    println!("cargo:rustc-env=DOVIK_RUNTIME_OBJECT={}", object.display());
}

fn case_tables() -> String {
    let mut out = String::from("#include <stdint.h>\n\n");
    emit(&mut out, "upper", |scalar| scalar.to_uppercase().collect());
    emit(&mut out, "lower", |scalar| scalar.to_lowercase().collect());
    out
}

fn emit(out: &mut String, name: &str, mapping: impl Fn(char) -> Vec<char>) {
    let mut single: Vec<(u32, u32)> = Vec::new();
    let mut multi: Vec<(u32, Vec<char>)> = Vec::new();

    for code in 0u32..=0x10FFFF {
        let Some(scalar) = char::from_u32(code) else {
            continue;
        };

        let mapped = mapping(scalar);

        if mapped.len() == 1 {
            if mapped[0] != scalar {
                single.push((code, mapped[0] as u32));
            }
            continue;
        }

        multi.push((code, mapped));
    }

    let _ = writeln!(out, "static const uint32_t case_{name}[][2] = {{");
    for (from, to) in &single {
        let _ = writeln!(out, "    {{{from}, {to}}},");
    }
    let _ = writeln!(out, "}};\n");
    let _ = writeln!(
        out,
        "#define CASE_{}_COUNT {}\n",
        name.to_uppercase(),
        single.len()
    );

    let _ = writeln!(out, "static const uint32_t case_{name}_multi[][4] = {{");
    for (from, to) in &multi {
        let mut parts: Vec<String> = to.iter().map(|scalar| (*scalar as u32).to_string()).collect();
        while parts.len() < 3 {
            parts.push("0".to_string());
        }
        let _ = writeln!(out, "    {{{from}, {}}},", parts.join(", "));
    }
    let _ = writeln!(out, "}};\n");
    let _ = writeln!(
        out,
        "#define CASE_{}_MULTI_COUNT {}\n",
        name.to_uppercase(),
        multi.len()
    );
}

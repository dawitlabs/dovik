use std::path::PathBuf;
use std::process::Command;

use compiler::source::SourceMap;
use compiler::target::Target;
use compiler::{backend, check, cli, interp, ir, lexer, lower, modules, opt, parser, resolve};

fn lowered(source: &str) -> ir::Program {
    let (tokens, lex_errors) = lexer::lex(source);
    assert_eq!(lex_errors, [], "lex errors");
    let (ast, parse_errors) = parser::parse(source, &tokens);
    assert!(parse_errors.is_empty(), "parse errors: {parse_errors:?}");
    let graph = modules::unit_graph("test.dovk", source, ast);
    let (resolution, resolve_errors) = resolve::resolve(&graph);
    assert!(resolve_errors.is_empty(), "resolve errors: {resolve_errors:?}");
    let (typed, type_errors) = check::check(&graph, &resolution);
    assert!(type_errors.is_empty(), "type errors: {type_errors:?}");
    let mut program = lower::lower(&graph, &resolution, &typed);
    assert!(opt::optimize(&mut program).is_empty());
    program
}

fn toolchain() -> Option<(String, String)> {
    let archive = cli::wasm_runtime_archive()?;
    let sysroot = cli::wasi_sysroot()?;

    if !PathBuf::from(&sysroot).exists() {
        return None;
    }

    let node = Command::new("node").arg("--version").output().ok()?;
    node.status.success().then_some((archive, sysroot))
}

struct Output {
    stdout: Vec<String>,
    stderr: String,
    code: i32,
}

fn compile_and_run(toolchain: &(String, String), source: &str, name: &str) -> Output {
    let (archive, sysroot) = toolchain;
    let program = lowered(source);
    let sources = SourceMap::single("test.dovk", source);
    let llvm = backend::emit_for(&program, &sources, true, Target::Wasi);

    let dir = std::env::temp_dir().join("dovik-wasm-tests");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let ll = dir.join(format!("{name}.ll"));
    let module = dir.join(format!("{name}.wasm"));
    std::fs::write(&ll, llvm).expect("write ir");

    let build = Command::new("clang")
        .args(["--target=wasm32-wasip1", "-O2"])
        .arg(format!("--sysroot={sysroot}"))
        .arg("-o")
        .arg(&module)
        .arg(&ll)
        .arg(archive)
        .arg("-lm")
        .output()
        .expect("clang");

    assert!(
        build.status.success(),
        "clang rejected {name}:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let run = Command::new("node")
        .arg("--no-warnings")
        .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/wasi-host.mjs"))
        .arg(&module)
        .output()
        .expect("node");

    Output {
        stdout: String::from_utf8_lossy(&run.stdout)
            .lines()
            .map(str::to_string)
            .collect(),
        stderr: String::from_utf8_lossy(&run.stderr).to_string(),
        code: run.status.code().unwrap_or(-1),
    }
}

const CORPUS: &[(&str, &str)] = &[
    ("arith", "print(1 + 2 * 3)\nprint(10 - 4)\nprint(7 / 2)\nprint(10 % 3)\n"),
    ("floats", "print(1.5 + 2.25)\nprint(1.5 * 2.0)\nprint(-1.5)\nprint(7.0 / 2.0)\n"),
    ("bools", "print(true)\nprint(not false)\nprint(1 < 2)\nprint(2 == 2)\n"),
    ("conversion", "print(to_float(3))\nprint(to_int(3.9))\nprint(to_int(-3.9))\n"),
    ("negatives", "print(-7 % 3)\nprint(-9223372036854775808)\nprint(0 - 5)\n"),
    (
        "recursion",
        "function factorial(n: Int) -> Int:\n    if n <= 1:\n        return 1\n    return n * factorial(n - 1)\n\nprint(factorial(10))\n",
    ),
    (
        "mutual_recursion",
        "function is_even(n: Int) -> Bool:\n    if n == 0:\n        return true\n    return is_odd(n - 1)\n\nfunction is_odd(n: Int) -> Bool:\n    if n == 0:\n        return false\n    return is_even(n - 1)\n\nprint(is_even(10))\n",
    ),
    ("strings", "a = \"Hello, \"\nprint(a + \"Dawit\")\nprint(len(\"héllo\"))\n"),
    ("string_index", "s = \"héllo\"\nprint(s[0])\nprint(s[1])\n"),
    ("string_loop", "for c in \"héllo\":\n    print(c)\n"),
    ("lists", "xs = [1, 2, 3]\nprint(xs[1])\nprint(len(xs))\nxs[0] = 99\nprint(xs[0])\n"),
    ("list_loop", "total = 0\nfor n in [1, 2, 3, 4]:\n    total = total + n\nprint(total)\n"),
    ("list_strings", "for name in [\"Dawit\", \"Abel\"]:\n    print(\"Hi \" + name)\n"),
    ("aliasing", "a = [1, 2]\nb = a\nb[0] = 9\nprint(a[0])\n"),
    (
        "dicts",
        "d: Dict<String, Int> = {}\nd[\"a\"] = 1\nd[\"b\"] = 2\nprint(d[\"a\"] + d[\"b\"])\nprint(len(d))\n",
    ),
    ("text_ops", "print(upper(\"abc\"))\nprint(trim(\"  x  \"))\nprint(join(split(\"a,b\", \",\"), \"-\"))\n"),
    ("optionals", "x = parse_int(\"12\")\n\nif x != none:\n    print(x + 1)\nelse:\n    print(0)\n"),
    ("optional_absent", "x = parse_int(\"nope\")\n\nif x == none:\n    print(\"absent\")\nelse:\n    print(\"present\")\n"),
    ("sorting", "xs = [3, 1, 2]\nsort(xs)\nprint(xs[0])\nnames = [\"c\", \"a\"]\nsort(names)\nprint(join(names, \",\"))\n"),
];

const TRAPS: &[(&str, &str, &str)] = &[
    ("trap_overflow", "x = 9223372036854775807\nprint(x + 1)\n", "integer overflow"),
    ("trap_div", "d = 0\nprint(7 / d)\n", "division by zero"),
    ("trap_rem", "d = 0\nprint(7 % d)\n", "remainder by zero"),
    ("trap_bounds", "xs = [1]\ni = 5\nprint(xs[i])\n", "index out of bounds"),
    ("trap_string_bounds", "s = \"a\"\ni = 9\nprint(s[i])\n", "index out of bounds"),
    ("trap_key", "d = {\"a\": 1}\nk = \"b\"\nprint(d[k])\n", "key not found"),
    (
        "trap_convert",
        "big = 9999999999.0\nprint(to_int(big * big))\n",
        "invalid conversion",
    ),
];

#[test]
fn wasm_output_matches_the_interpreter() {
    let Some(toolchain) = toolchain() else {
        eprintln!("no WebAssembly toolchain: install wasi-libc and node, then rebuild");
        return;
    };

    for (name, source) in CORPUS {
        let interpreted = interp::run(&lowered(source));
        assert_eq!(
            interpreted.outcome,
            interp::Outcome::Finished,
            "{name} trapped in the interpreter"
        );

        let wasm = compile_and_run(&toolchain, source, name);

        assert_eq!(wasm.code, 0, "{name} exited {}: {}", wasm.code, wasm.stderr);
        assert_eq!(
            wasm.stdout, interpreted.output,
            "{name}: wasm output differs from the interpreter"
        );
    }
}

#[test]
fn every_runtime_check_still_traps_under_wasm() {
    let Some(toolchain) = toolchain() else {
        eprintln!("no WebAssembly toolchain: install wasi-libc and node, then rebuild");
        return;
    };

    for (name, source, expected) in TRAPS {
        let wasm = compile_and_run(&toolchain, source, name);

        assert_eq!(wasm.code, 1, "{name} should have trapped: {:?}", wasm.stdout);
        assert!(
            wasm.stderr.contains(expected),
            "{name}: expected {expected:?} in {:?}",
            wasm.stderr
        );
    }
}

#[test]
fn a_wasm_trap_reports_its_source_position() {
    let Some(toolchain) = toolchain() else {
        eprintln!("no WebAssembly toolchain: install wasi-libc and node, then rebuild");
        return;
    };

    let wasm = compile_and_run(&toolchain, "print(\"start\")\nd = 0\nprint(7 / d)\n", "position");

    assert_eq!(wasm.stdout, ["start"]);
    assert!(wasm.stderr.contains("3:7"), "{:?}", wasm.stderr);
}

#[test]
fn stack_exhaustion_is_reported_rather_than_trapping_the_module() {
    let Some(toolchain) = toolchain() else {
        eprintln!("no WebAssembly toolchain: install wasi-libc and node, then rebuild");
        return;
    };

    let source = "\
function deep(n: Int) -> Int:
    if n < 1:
        return 0

    return 1 + deep(n - 1)

print(deep(1000000))
";
    let wasm = compile_and_run(&toolchain, source, "exhaustion");

    assert_eq!(wasm.code, 1, "{:?}", wasm.stdout);
    assert!(wasm.stderr.contains("DVK5009"), "{:?}", wasm.stderr);
    assert!(
        !wasm.stderr.contains("wasi host:"),
        "the module must report exhaustion itself, not trap: {:?}",
        wasm.stderr
    );
}

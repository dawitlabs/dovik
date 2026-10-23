use std::process::Command;

use compiler::source::SourceMap;
use compiler::{backend, check, cli, ir, lexer, lower, modules, opt, parser, resolve};

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

fn findings(archive: &str, source: &str, name: &str) -> String {
    let program = lowered(source);
    let sources = SourceMap::single("test.dovk", source);

    let dir = std::env::temp_dir().join("dovik-sanitize-tests");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let ll = dir.join(format!("{name}.ll"));
    let binary = dir.join(name);
    std::fs::write(&ll, backend::emit(&program, &sources)).expect("write ir");

    let build = Command::new("clang")
        .args([
            "-fsanitize=address,undefined",
            "-fno-omit-frame-pointer",
            "-O1",
            "-g",
        ])
        .arg("-o")
        .arg(&binary)
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

    let run = Command::new(&binary)
        .env("ASAN_OPTIONS", "detect_leaks=0:abort_on_error=0")
        .env("UBSAN_OPTIONS", "print_stacktrace=1")
        .output()
        .expect("run");

    let stderr = String::from_utf8_lossy(&run.stderr).to_string();

    match stderr.contains("AddressSanitizer") || stderr.contains("runtime error:") {
        true => stderr,
        false => String::new(),
    }
}

const MEMORY: &[(&str, &str)] = &[
    (
        "concat_loop",
        "out = \"\"\ni = 0\nwhile i < 500:\n    out = out + \"abcdefgh\"\n    i = i + 1\nprint(len(out))\n",
    ),
    (
        "shared_then_extended",
        "s = \"abc\"\ni = 0\nwhile i < 200:\n    t = s\n    s = s + \"d\"\n    print(len(t) + len(s))\n    i = i + 1\n",
    ),
    (
        "list_churn",
        "xs: List<String> = []\ni = 0\nwhile i < 500:\n    append(xs, \"item\" + to_string(i))\n    i = i + 1\nprint(len(xs))\nprint(xs[499])\n",
    ),
    (
        "list_grow_and_shrink",
        "xs = [1, 2, 3]\ni = 0\nwhile i < 100:\n    append(xs, i)\n    if len(xs) > 20:\n        remove_at(xs, 0)\n    i = i + 1\nprint(len(xs))\n",
    ),
    (
        "dict_rehash",
        "d: Dict<String, Int> = {}\ni = 0\nwhile i < 500:\n    d[\"k\" + to_string(i)] = i\n    i = i + 1\nprint(len(d))\nprint(d[\"k250\"])\n",
    ),
    (
        "dict_remove_half",
        "d: Dict<String, List<Int>> = {}\ni = 0\nwhile i < 100:\n    v: List<Int> = [i, i + 1]\n    d[\"key\" + to_string(i)] = v\n    i = i + 1\n\ni = 0\nwhile i < 50:\n    remove_key(d, \"key\" + to_string(i * 2))\n    i = i + 1\n\nprint(len(d))\n",
    ),
    (
        "nested_lists_aliased",
        "a: List<List<Int>> = []\ni = 0\nwhile i < 50:\n    row: List<Int> = []\n    j = 0\n    while j < 10:\n        append(row, i * j)\n        j = j + 1\n    append(a, row)\n    i = i + 1\n\nb = a[25]\nb[0] = 777\nprint(a[25][0])\n",
    ),
    (
        "returned_strings_pooled",
        "function build(n: Int) -> String:\n    out = \"\"\n    i = 0\n    while i < n:\n        out = out + \"z\"\n        i = i + 1\n    return out\n\ni = 0\nwhile i < 200:\n    a = build(i % 20)\n    b = build(i % 7)\n    print(len(a) + len(b))\n    i = i + 1\n",
    ),
    (
        "mutated_global",
        "g: List<String> = []\n\nfunction push(v: String):\n    append(g, v)\n\ni = 0\nwhile i < 200:\n    push(\"x\" + to_string(i))\n    i = i + 1\n\nprint(len(g))\n",
    ),
    (
        "non_ascii_indexing",
        "s = \"héllo wörld ጤና ይስጥልኝ\"\ni = 0\ntotal = 0\nwhile i < len(s):\n    total = total + len(s[i])\n    i = i + 1\nprint(total)\nprint(upper(s))\nprint(join(split(s, \" \"), \"|\"))\n",
    ),
    (
        "sorted_strings",
        "xs: List<String> = []\ni = 0\nwhile i < 300:\n    append(xs, \"v\" + to_string(300 - i))\n    i = i + 1\n\nsort(xs)\nprint(xs[0])\nprint(xs[299])\n",
    ),
    (
        "params_and_returns",
        "function pick(xs: List<String>, n: Int) -> String:\n    if n >= len(xs):\n        return \"\"\n    return xs[n]\n\nnames = [\"alpha\", \"beta\", \"gamma\"]\ni = 0\nwhile i < 300:\n    v = pick(names, i % 4)\n    if len(v) > 0:\n        print(v)\n    i = i + 1\n",
    ),
];

const TRAPS: &[(&str, &str)] = &[
    ("trap_conversion", "big = 9999999999.0\nx = big * big\nprint(to_int(x))\n"),
    ("trap_negate_min", "x = -9223372036854775808\nprint(-x)\n"),
    ("trap_scalar_bounds", "s = \"ጤና\"\ni = 9\nprint(s[i])\n"),
    ("trap_missing_key", "d = {\"a\": 1}\nk = \"b\"\nprint(d[k])\n"),
];

#[test]
fn the_runtime_is_clean_under_address_and_undefined_behaviour_sanitizers() {
    let Some(archive) = cli::sanitized_runtime_archive() else {
        eprintln!("no sanitized runtime: clang could not build it, so nothing was checked");
        return;
    };

    for (name, source) in MEMORY {
        let report = findings(&archive, source, name);
        assert!(report.is_empty(), "{name}:\n{report}");
    }
}

#[test]
fn a_trapping_program_is_clean_up_to_the_point_it_traps() {
    let Some(archive) = cli::sanitized_runtime_archive() else {
        eprintln!("no sanitized runtime: clang could not build it, so nothing was checked");
        return;
    };

    for (name, source) in TRAPS {
        let report = findings(&archive, source, name);
        assert!(report.is_empty(), "{name}:\n{report}");
    }
}

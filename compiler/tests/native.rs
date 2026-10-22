use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

use compiler::source::SourceMap;
use compiler::{backend, check, interp, ir, lexer, lower, modules, opt, parser, resolve};

fn lowered(source: &str, optimize: bool) -> ir::Program {
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

    if optimize {
        let errors = opt::optimize(&mut program);
        assert!(errors.is_empty(), "fold errors: {errors:?}");
    }

    program
}

fn runtime_archive() -> &'static PathBuf {
    static ARCHIVE: OnceLock<PathBuf> = OnceLock::new();

    ARCHIVE.get_or_init(|| {
        let path = PathBuf::from(env!("DOVIK_RUNTIME_OBJECT"));
        assert!(path.exists(), "the runtime object is missing: {path:?}");
        path
    })
}

#[derive(Debug)]
struct Native {
    stdout: Vec<String>,
    stderr: String,
    code: i32,
}

fn compile_and_run(source: &str, name: &str) -> Native {
    let program = lowered(source, true);
    let llvm = backend::emit(&program, &SourceMap::single("test.dovk", source));

    let dir = std::env::temp_dir().join("dovik-native-tests");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let ll = dir.join(format!("{name}.ll"));
    let binary = dir.join(name);
    std::fs::write(&ll, &llvm).expect("write ir");

    let build = Command::new("clang")
        .args(["-O2", "-flto=full"])
        .arg("-o")
        .arg(&binary)
        .arg(&ll)
        .arg(runtime_archive())
        .arg("-lm")
        .output()
        .expect("clang");

    assert!(
        build.status.success(),
        "clang rejected the IR for {name}:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let run = Command::new(&binary).output().expect("run");

    Native {
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
    (
        "recursion",
        "function factorial(n: Int) -> Int:\n    if n <= 1:\n        return 1\n    return n * factorial(n - 1)\n\nprint(factorial(10))\n",
    ),
    (
        "mutual",
        "function is_even(n: Int) -> Bool:\n    if n == 0:\n        return true\n    return is_odd(n - 1)\n\nfunction is_odd(n: Int) -> Bool:\n    if n == 0:\n        return false\n    return is_even(n - 1)\n\nprint(is_even(10))\n",
    ),
    (
        "while_loop",
        "n = 0\ntotal = 0\nwhile n < 10:\n    n = n + 1\n    total = total + n\nprint(total)\n",
    ),
    (
        "break_continue",
        "total = 0\nn = 0\nwhile n < 10:\n    n = n + 1\n    if n == 3:\n        continue\n    if n > 6:\n        break\n    total = total + n\nprint(total)\n",
    ),
    ("strings", "a = \"Hello, \"\nprint(a + \"Dawit\")\nprint(len(\"héllo\"))\n"),
    ("string_index", "s = \"héllo\"\nprint(s[0])\nprint(s[1])\n"),
    ("string_loop", "for c in \"héllo\":\n    print(c)\n"),
    (
        "string_compare",
        "print(\"a\" < \"b\")\nprint(\"b\" < \"a\")\nprint(\"ab\" == \"ab\")\n",
    ),
    ("lists", "xs = [1, 2, 3]\nprint(xs[1])\nprint(len(xs))\nxs[0] = 99\nprint(xs[0])\n"),
    ("list_loop", "total = 0\nfor n in [1, 2, 3, 4]:\n    total = total + n\nprint(total)\n"),
    ("list_strings", "for name in [\"Dawit\", \"Abel\"]:\n    print(\"Hi \" + name)\n"),
    (
        "length_in_a_loop_condition",
        "s = \"abc\"\nj = 0\nwhile j < len(s):\n    j = j + 1\nprint(j)\nxs = [1, 2, 3]\ni = 0\ntotal = 0\nwhile i < len(xs):\n    total = total + xs[i]\n    i = i + 1\nprint(total)\n",
    ),
    (
        "float_and_bool_list_loop",
        "fs = [2.5, -1.5]\nfor f in fs:\n    print(f)\nbs = [true, false]\nfor b in bs:\n    print(b)\n",
    ),
    ("aliasing", "a = [1, 2]\nb = a\nb[0] = 9\nprint(a[0])\n"),
    (
        "dicts",
        "d = {\"a\": 1, \"b\": 2}\nprint(d[\"a\"])\nprint(len(d))\nd[\"c\"] = 3\nprint(d[\"c\"])\nprint(len(d))\n",
    ),
    (
        "structural_eq",
        "a: List<Int> = [1, 2]\nb: List<Int> = [1, 2]\nc: List<Int> = [1, 3]\nprint(a == b)\nprint(a == c)\n",
    ),
    (
        "short_circuit",
        "function loud() -> Bool:\n    print(\"ran\")\n    return true\n\nprint(false and loud())\nprint(true or loud())\n",
    ),
    (
        "globals",
        "limit = 10\n\nfunction under(n: Int) -> Bool:\n    return n < limit\n\nprint(under(5))\nprint(under(50))\n",
    ),
    (
        "nested_loops",
        "for word in [\"ab\", \"cd\"]:\n    for scalar in word:\n        print(scalar)\n",
    ),
    (
        "params",
        "function add(a: Int, b: Int) -> Int:\n    return a + b\n\nfunction mutate(xs: List<Int>):\n    xs[0] = 42\n\nitems = [1]\nmutate(items)\nprint(items[0])\nprint(add(20, 22))\n",
    ),
    ("negatives", "print(-7 % 3)\nprint(-9223372036854775808)\nprint(0 - 5)\n"),
    (
        "alias_shared_string",
        "a = \"x\"\nb = a\na = a + \"y\"\nprint(a)\nprint(b)\n",
    ),
    (
        "alias_self_concat",
        "s = \"ab\"\ns = s + s\nprint(s)\nprint(len(s))\n",
    ),
    (
        "alias_string_in_list",
        "s = \"a\"\nxs: List<String> = []\nappend(xs, s)\ns = s + \"b\"\nprint(s)\nprint(xs[0])\n",
    ),
    (
        "alias_literal_left",
        "out = \"\"\nfor n in [\"a\", \"b\"]:\n    out = out + (\"p\" + n)\nprint(out)\nprint(\"p\" + \"q\")\n",
    ),
    (
        "alias_named_literal",
        "prefix = \"hi \"\nfor n in [\"a\", \"b\"]:\n    print(prefix + n)\nprint(prefix)\n",
    ),
    (
        "alias_dict_value",
        "d: Dict<String, String> = {}\ns = \"v\"\nd[\"k\"] = s\ns = s + \"2\"\nprint(s)\nprint(d[\"k\"])\n",
    ),
    (
        "alias_returned",
        "function build(n: Int) -> String:\n    out = \"\"\n    i = 0\n    while i < n:\n        out = out + \"z\"\n        i = i + 1\n    return out\n\na = build(3)\nb = build(4)\nprint(a)\nprint(b)\n",
    ),
    (
        "alias_param_concat",
        "function tag(base: String) -> String:\n    return base + \"!\"\n\ns = \"hi\"\nprint(tag(s))\nprint(s)\n",
    ),
];

#[test]
fn native_output_matches_the_interpreter() {
    for (name, source) in CORPUS {
        let interpreted = interp::run(&lowered(source, true));
        assert_eq!(
            interpreted.outcome,
            interp::Outcome::Finished,
            "{name} trapped in the interpreter"
        );

        let native = compile_and_run(source, name);

        assert_eq!(native.code, 0, "{name} exited {} : {}", native.code, native.stderr);
        assert_eq!(
            native.stdout, interpreted.output,
            "{name}: native output differs from the interpreter"
        );
    }
}

const TRAPS: &[(&str, &str, &str)] = &[
    ("trap_overflow", "x = 9223372036854775807\nprint(x + 1)\n", "integer overflow"),
    ("trap_div", "d = 0\nprint(7 / d)\n", "division by zero"),
    ("trap_rem", "d = 0\nprint(7 % d)\n", "remainder by zero"),
    (
        "trap_convert",
        "big = 9999999999.0\nprint(to_int(big * big))\n",
        "invalid conversion",
    ),
    ("trap_bounds", "xs = [1]\ni = 5\nprint(xs[i])\n", "index out of bounds"),
    ("trap_string_bounds", "s = \"a\"\ni = 9\nprint(s[i])\n", "index out of bounds"),
    ("trap_key", "d = {\"a\": 1}\nk = \"b\"\nprint(d[k])\n", "key not found"),
    ("trap_negate", "x = -9223372036854775808\nprint(-x)\n", "integer overflow"),
];

#[test]
fn checks_survive_optimization() {
    for (name, source, expected) in TRAPS {
        let native = compile_and_run(source, name);

        assert_eq!(native.code, 1, "{name} should have trapped: {:?}", native.stdout);
        assert!(
            native.stderr.contains(expected),
            "{name}: expected {expected:?} in {:?}",
            native.stderr
        );
    }
}

#[test]
fn a_trap_reports_its_source_position() {
    let native = compile_and_run("print(\"start\")\nd = 0\nprint(7 / d)\n", "trap_position");
    assert_eq!(native.stdout, ["start"]);
    assert!(native.stderr.contains("3:7"), "{:?}", native.stderr);
}

#[test]
fn the_emitted_ir_is_valid() {
    for (name, source) in CORPUS {
        let program = lowered(source, true);
        let llvm = backend::emit(&program, &SourceMap::single("test.dovk", *source));

        let dir = std::env::temp_dir().join("dovik-native-tests");
        std::fs::create_dir_all(&dir).expect("temp dir");
        let ll = dir.join(format!("verify-{name}.ll"));
        std::fs::write(&ll, &llvm).expect("write");

        let check = Command::new("llc")
            .arg("-filetype=null")
            .arg(&ll)
            .output()
            .expect("llc");

        assert!(
            check.status.success(),
            "llc rejected {name}:\n{}",
            String::from_utf8_lossy(&check.stderr)
        );
    }
}

fn live_objects_at_exit(source: &str, name: &str) -> i64 {
    let program = lowered(source, true);
    let dir = std::env::temp_dir().join("dovik-native-tests");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let ll = dir.join(format!("leak-{name}.ll"));
    let binary = dir.join(format!("leak-{name}"));
    std::fs::write(&ll, backend::emit(&program, &SourceMap::single("test.dovk", source))).expect("write ir");

    let build = Command::new("clang")
        .args(["-O2", "-flto=full"])
        .arg("-o")
        .arg(&binary)
        .arg(&ll)
        .arg(runtime_archive())
        .arg("-lm")
        .output()
        .expect("clang");
    assert!(
        build.status.success(),
        "clang rejected {name}:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let run = Command::new(&binary)
        .env("DOVIK_CHECK_LEAKS", "1")
        .output()
        .expect("run");
    assert!(run.status.success(), "{name} exited {:?}", run.status.code());

    let stderr = String::from_utf8_lossy(&run.stderr).to_string();
    let reported = stderr
        .lines()
        .find_map(|line| line.strip_prefix("dovik: "))
        .and_then(|rest| rest.split_whitespace().next())
        .unwrap_or("missing");

    reported
        .parse()
        .unwrap_or_else(|_| panic!("{name}: no leak report in {stderr:?}"))
}

#[test]
fn every_program_frees_everything_it_allocates() {
    let programs: Vec<(&str, &str)> = CORPUS
        .iter()
        .copied()
        .chain([
            (
                "leak_concat_loop",
                "s = \"\"\nn = 0\nwhile n < 200:\n    s = s + \"x\"\n    n = n + 1\nprint(len(s))\n",
            ),
            (
                "leak_list_churn",
                "total = 0\nn = 0\nwhile n < 500:\n    xs = [n, n + 1]\n    total = total + xs[0]\n    n = n + 1\nprint(total)\n",
            ),
            (
                "leak_string_param",
                "function greet(who: String):\n    print(\"hi \" + who)\n\nfor name in [\"a\", \"b\"]:\n    greet(name)\n",
            ),
            (
                "leak_returned_string",
                "function label(n: Int) -> String:\n    return \"n=\" + to_string(n)\n\nk = 0\nwhile k < 50:\n    print(label(k))\n    k = k + 1\n",
            ),
            (
                "leak_nested_collections",
                "grid: List<List<Int>> = []\nn = 0\nwhile n < 50:\n    append(grid, [n, n + 1])\n    n = n + 1\nprint(len(grid))\n",
            ),
            (
                "leak_dict_overwrite",
                "d: Dict<String, String> = {}\nn = 0\nwhile n < 100:\n    d[\"k\"] = to_string(n)\n    n = n + 1\nprint(len(d))\nprint(remove_key(d, \"k\"))\n",
            ),
        ])
        .collect();

    for (name, source) in programs {
        assert_eq!(
            live_objects_at_exit(source, name),
            0,
            "{name} leaked at exit"
        );
    }
}

#[test]
fn deep_recursion_reports_a_diagnostic_rather_than_faulting() {
    let source = "\
function down(n: Int) -> Int:
    if n <= 0:
        return 0
    return 1 + down(n - 1)

print(down(100000000))
";

    let native = compile_and_run(source, "trap_stack");

    assert_eq!(
        native.code, 1,
        "a stack overflow must exit 1, not die on a signal: {native:?}"
    );
    assert!(
        native.stderr.contains("DVK5009"),
        "expected DVK5009, got {:?}",
        native.stderr
    );
    assert!(
        native.stderr.contains("recursion too deep"),
        "{:?}",
        native.stderr
    );
}

#[test]
fn recursion_that_fits_still_runs() {
    let source = "\
function down(n: Int) -> Int:
    if n <= 0:
        return 0
    return 1 + down(n - 1)

print(down(20000))
";

    let native = compile_and_run(source, "deep_but_fine");
    assert_eq!(native.code, 0, "{native:?}");
    assert_eq!(native.stdout, ["20000"]);
}

#[test]
fn a_function_mutates_shared_module_state() {
    let source = "\
counter: List<Int> = [0]
seen: Dict<String, Int> = {}

function bump():
    counter[0] = counter[0] + 1

function record(name: String):
    bump()

    if has_key(seen, name):
        seen[name] = seen[name] + 1
    else:
        seen[name] = 1

record(\"a\")
record(\"b\")
record(\"a\")

print(counter[0])
print(seen[\"a\"])
print(seen[\"b\"])
print(len(seen))
";

    let native = compile_and_run(source, "shared_state");
    assert_eq!(native.code, 0, "{native:?}");
    assert_eq!(native.stdout, ["3", "2", "1", "2"]);
}

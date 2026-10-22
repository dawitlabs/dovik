use std::process::Command;

use compiler::error::TypeErrorKind::{self, *};
use compiler::ir::RuntimeError;
use compiler::types::Type;
use compiler::{backend, check, interp, ir, lexer, lower, opt, parser, resolve};

fn lowered(source: &str) -> ir::Program {
    let (tokens, lex_errors) = lexer::lex(source);
    assert_eq!(lex_errors, [], "lex errors");
    let (ast, parse_errors) = parser::parse(source, &tokens);
    assert!(parse_errors.is_empty(), "parse errors: {parse_errors:?}");
    let (resolution, resolve_errors) = resolve::resolve(&ast);
    assert!(resolve_errors.is_empty(), "resolve errors: {resolve_errors:?}");
    let (typed, type_errors) = check::check(&ast, &resolution);
    assert!(type_errors.is_empty(), "type errors: {type_errors:?}");
    let mut program = lower::lower(&ast, &resolution, &typed);
    assert!(opt::optimize(&mut program).is_empty());
    program
}

fn output(source: &str) -> Vec<String> {
    let run = interp::run(&lowered(source));
    assert_eq!(run.outcome, interp::Outcome::Finished, "{run:?}");
    run.output
}

fn trap(source: &str) -> RuntimeError {
    interp::run(&lowered(source))
        .trap()
        .expect("expected a trap")
        .error
}

fn errors(source: &str) -> Vec<TypeErrorKind> {
    let (tokens, _) = lexer::lex(source);
    let (ast, parse_errors) = parser::parse(source, &tokens);
    assert!(parse_errors.is_empty(), "parse errors: {parse_errors:?}");
    let (resolution, resolve_errors) = resolve::resolve(&ast);
    assert!(resolve_errors.is_empty(), "resolve errors: {resolve_errors:?}");
    check::check(&ast, &resolution)
        .1
        .into_iter()
        .map(|error| error.kind)
        .collect()
}

#[test]
fn append_grows_a_list() {
    let source = "xs: List<Int> = []\nappend(xs, 1)\nappend(xs, 2)\nprint(len(xs))\nprint(xs[1])\n";
    assert_eq!(output(source), ["2", "2"]);
}

#[test]
fn append_through_a_parameter_reaches_the_caller() {
    let source = "\
function fill(target: List<Int>):
    append(target, 7)

xs: List<Int> = []
fill(xs)
print(len(xs))
print(xs[0])
";
    assert_eq!(output(source), ["1", "7"]);
}

#[test]
fn insert_and_remove_at() {
    let source = "\
xs = [1, 2, 3]
insert(xs, 0, 0)
print(xs[0])
insert(xs, 4, 4)
print(len(xs))
print(remove_at(xs, 0))
print(xs[0])
print(len(xs))
";
    assert_eq!(output(source), ["0", "5", "0", "1", "4"]);
}

#[test]
fn insert_at_the_end_is_allowed_but_beyond_it_traps() {
    assert_eq!(output("xs = [1]\ninsert(xs, 1, 2)\nprint(len(xs))\n"), ["2"]);
    assert_eq!(
        trap("xs = [1]\ni = 5\ninsert(xs, i, 2)\n"),
        RuntimeError::IndexOutOfBounds
    );
}

#[test]
fn remove_at_traps_out_of_bounds() {
    assert_eq!(
        trap("xs = [1]\ni = 3\nprint(remove_at(xs, i))\n"),
        RuntimeError::IndexOutOfBounds
    );
}

#[test]
fn dictionary_presence_and_removal() {
    let source = "\
d: Dict<String, Int> = {}
d[\"a\"] = 1
print(has_key(d, \"a\"))
print(has_key(d, \"b\"))
print(remove_key(d, \"a\"))
print(remove_key(d, \"a\"))
print(len(d))
";
    assert_eq!(output(source), ["true", "false", "true", "false", "0"]);
}

#[test]
fn to_string_covers_every_printable_type() {
    let source = "\
print(to_string(42))
print(to_string(-7))
print(to_string(1.5))
print(to_string(true))
print(to_string(\"text\"))
print(\"n=\" + to_string(7))
";
    assert_eq!(output(source), ["42", "-7", "1.5", "true", "text", "n=7"]);
}

#[test]
fn parsing_numbers() {
    let source = "\
print(parse_int(\"123\"))
print(parse_int(\"-7\"))
print(parse_float(\"1.5\"))
print(can_parse_int(\"42\"))
print(can_parse_int(\"abc\"))
print(can_parse_float(\"1.5\"))
print(can_parse_float(\"\"))
";
    assert_eq!(
        output(source),
        ["123", "-7", "1.5", "true", "false", "true", "false"]
    );
}

#[test]
fn parsing_surrounding_whitespace_is_tolerated() {
    assert_eq!(output("print(parse_int(\" 42 \"))\n"), ["42"]);
}

#[test]
fn a_failed_parse_traps() {
    assert_eq!(
        trap("text = \"abc\"\nprint(parse_int(text))\n"),
        RuntimeError::InvalidConversion
    );
    assert_eq!(
        trap("text = \"abc\"\nprint(parse_float(text))\n"),
        RuntimeError::InvalidConversion
    );
}

#[test]
fn can_parse_lets_a_program_avoid_the_trap() {
    let source = "\
text = \"abc\"
if can_parse_int(text):
    print(parse_int(text))
else:
    print(0)
";
    assert_eq!(output(source), ["0"]);
}

#[test]
fn element_types_are_enforced() {
    assert_eq!(
        errors("xs: List<Int> = []\nappend(xs, \"wrong\")\n"),
        [ArgumentType {
            name: "append".into(),
            position: 2,
            expected: Type::Int,
            found: Type::Str
        }]
    );
}

#[test]
fn the_first_argument_must_be_the_right_container() {
    assert_eq!(
        errors("append(1, 2)\n"),
        [BuiltinArgument { name: "append", found: Type::Int }]
    );
    assert_eq!(
        errors("xs: List<Int> = []\nprint(has_key(xs, 1))\n"),
        [BuiltinArgument { name: "has_key", found: Type::List(Box::new(Type::Int)) }]
    );
}

#[test]
fn arity_is_enforced() {
    assert_eq!(
        errors("xs: List<Int> = []\nappend(xs)\n"),
        [ArgumentCount { name: "append".into(), expected: 2, found: 1 }]
    );
    assert_eq!(
        errors("print(parse_int(\"1\", \"2\"))\n"),
        [ArgumentCount { name: "parse_int".into(), expected: 1, found: 2 }]
    );
}

#[test]
fn a_unit_returning_builtin_is_not_a_value() {
    assert_eq!(
        errors("xs: List<Int> = []\nx = append(xs, 1)\n"),
        [UnitAsValue]
    );
}

#[test]
fn builtin_names_cannot_be_redeclared() {
    let (tokens, _) = lexer::lex("append = 1\n");
    let (ast, _) = parser::parse("append = 1\n", &tokens);
    let (_, resolve_errors) = resolve::resolve(&ast);
    assert!(!resolve_errors.is_empty(), "append is a built-in");
}

const CORPUS: &[(&str, &str)] = &[
    (
        "std_list",
        "xs: List<Int> = []\nappend(xs, 1)\nappend(xs, 2)\ninsert(xs, 0, 0)\nprint(remove_at(xs, 0))\nprint(len(xs))\n",
    ),
    (
        "std_dict",
        "d: Dict<String, Int> = {}\nd[\"k\"] = 1\nprint(has_key(d, \"k\"))\nprint(remove_key(d, \"k\"))\nprint(len(d))\n",
    ),
    (
        "std_strings",
        "print(to_string(42))\nprint(to_string(1.5))\nprint(to_string(false))\nprint(\"v=\" + to_string(-3))\n",
    ),
    (
        "std_parse",
        "print(parse_int(\"77\"))\nprint(parse_float(\"2.25\"))\nprint(can_parse_int(\"x\"))\nprint(can_parse_float(\"3.5\"))\n",
    ),
    (
        "std_growth_loop",
        "xs: List<Int> = []\nn = 0\nwhile n < 5:\n    append(xs, n * n)\n    n = n + 1\nfor v in xs:\n    print(v)\n",
    ),
];

#[test]
fn native_matches_the_interpreter_for_the_standard_library() {
    let archive = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("debug")
        .join("libdovik_rt.a");

    if !archive.exists() {
        let status = Command::new("cargo")
            .args(["build", "-p", "dovik-rt"])
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .status()
            .expect("cargo build -p dovik-rt");
        assert!(status.success());
    }

    let dir = std::env::temp_dir().join("dovik-stdlib-tests");
    std::fs::create_dir_all(&dir).expect("temp dir");

    for (name, source) in CORPUS {
        let program = lowered(source);
        let expected = interp::run(&program);
        assert_eq!(expected.outcome, interp::Outcome::Finished, "{name}");

        let ll = dir.join(format!("{name}.ll"));
        let binary = dir.join(name);
        std::fs::write(&ll, backend::emit(&program, source)).expect("write ir");

        let build = Command::new("clang")
            .arg("-O2")
            .arg("-o")
            .arg(&binary)
            .arg(&ll)
            .arg(&archive)
            .args(["-lm", "-lpthread", "-ldl"])
            .output()
            .expect("clang");
        assert!(
            build.status.success(),
            "clang rejected {name}:\n{}",
            String::from_utf8_lossy(&build.stderr)
        );

        let run = Command::new(&binary).output().expect("run");
        let native: Vec<String> = String::from_utf8_lossy(&run.stdout)
            .lines()
            .map(str::to_string)
            .collect();

        assert_eq!(native, expected.output, "{name}: native differs");
    }
}

use compiler::error::TypeErrorKind::{self, *};
use compiler::source::SourceMap;
use compiler::types::Type;
use compiler::{backend, check, interp, ir, lexer, lower, modules, opt, parser, resolve};

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

fn output(source: &str) -> Vec<String> {
    let run = interp::run(&lowered(source));
    assert_eq!(run.outcome, interp::Outcome::Finished, "{run:?}");
    run.output
}

fn errors(source: &str) -> Vec<TypeErrorKind> {
    let (tokens, _) = lexer::lex(source);
    let (ast, parse_errors) = parser::parse(source, &tokens);
    assert!(parse_errors.is_empty(), "parse errors: {parse_errors:?}");
    let graph = modules::unit_graph("test.dovk", source, ast);
    let (resolution, resolve_errors) = resolve::resolve(&graph);
    assert!(resolve_errors.is_empty(), "resolve errors: {resolve_errors:?}");
    check::check(&graph, &resolution)
        .1
        .into_iter()
        .map(|error| error.kind)
        .collect()
}

fn parse_errors(source: &str) -> Vec<String> {
    let (tokens, _) = lexer::lex(source);
    parser::parse(source, &tokens)
        .1
        .into_iter()
        .map(|error| error.kind.code().to_string())
        .collect()
}

fn optional(inner: Type) -> Type {
    Type::Optional(Box::new(inner))
}

#[test]
fn none_and_some_round_trip() {
    let source = "\
missing: Int? = none
found: Int? = some(42)
print(missing == none)
print(found == none)
";
    assert_eq!(output(source), ["true", "false"]);
}

#[test]
fn narrowing_gives_the_inner_type() {
    let source = "\
found: Int? = some(42)

if found != none:
    print(found + 1)
else:
    print(0)
";
    assert_eq!(output(source), ["43"]);
}

#[test]
fn equality_to_none_narrows_the_else_branch() {
    let source = "\
name: String? = some(\"dovik\")

if name == none:
    print(\"absent\")
else:
    print(upper(name))
";
    assert_eq!(output(source), ["DOVIK"]);
}

#[test]
fn optionals_compare_by_content() {
    let source = "\
a: Int? = some(1)
b: Int? = some(1)
c: Int? = some(2)
d: Int? = none
e: Int? = none
print(a == b)
print(a == c)
print(d == e)
print(a == d)
";
    assert_eq!(output(source), ["true", "false", "true", "false"]);
}

#[test]
fn an_optional_reference_compares_structurally() {
    let source = "\
a: String? = some(\"x\")
b: String? = some(\"x\")
c: String? = some(\"y\")
print(a == b)
print(a == c)
";
    assert_eq!(output(source), ["true", "false"]);
}

#[test]
fn optionals_work_through_functions() {
    let source = "\
function doubled(value: Int?) -> Int?:
    if value != none:
        return some(value * 2)
    return none

a = doubled(some(21))
if a != none:
    print(a)

print(doubled(none) == none)
";
    assert_eq!(output(source), ["42", "true"]);
}

#[test]
fn narrowing_does_not_leak_out_of_the_block() {
    let source = "\
found: Int? = some(1)

if found != none:
    print(found)

print(found == none)
";
    assert_eq!(output(source), ["1", "false"]);
}

#[test]
fn a_bare_none_cannot_be_inferred() {
    assert_eq!(errors("x = none\n"), [CannotInferNone]);
}

#[test]
fn an_optional_is_not_its_inner_type() {
    assert_eq!(
        errors("x: Int = some(1)\n"),
        [AnnotationMismatch { annotated: Type::Int, found: optional(Type::Int) }]
    );
    assert_eq!(
        errors("x: Int? = some(1)\nprint(x)\n"),
        [BuiltinArgument { name: "print", found: optional(Type::Int) }]
    );
}

#[test]
fn an_optional_supports_no_arithmetic_or_ordering() {
    assert_eq!(
        errors("x: Int? = some(1)\ny = x + 1\n"),
        [BinaryOperands { op: "+", left: optional(Type::Int), right: Type::Int }]
    );
    assert_eq!(
        errors("x: Int? = some(1)\ny: Int? = some(2)\nz = x < y\n"),
        [BinaryOperands {
            op: "<",
            left: optional(Type::Int),
            right: optional(Type::Int)
        }]
    );
}

#[test]
fn an_optional_cannot_be_compared_to_its_inner_type() {
    assert_eq!(
        errors("x: Int? = some(1)\ny = x == 1\n"),
        [BinaryOperands { op: "==", left: optional(Type::Int), right: Type::Int }]
    );
}

#[test]
fn nesting_is_rejected() {
    assert_eq!(parse_errors("x: Int?? = none\n"), ["DVK2010"]);
    assert_eq!(
        errors("x: Int? = some(1)\ny: Int? = some(x)\n"),
        [BuiltinArgument { name: "some", found: optional(Type::Int) }]
    );
}

#[test]
fn optionals_cannot_live_in_collections() {
    assert_eq!(
        errors("xs: List<Int?> = []\n"),
        [OptionalInsideCollection(optional(Type::Int))]
    );
    assert_eq!(
        errors("d: Dict<String, Int?> = {}\n"),
        [OptionalInsideCollection(optional(Type::Int))]
    );
    assert_eq!(
        errors("d: Dict<Int?, String> = {}\n"),
        [
            InvalidDictKeyType(optional(Type::Int)),
            OptionalInsideCollection(optional(Type::Int))
        ]
    );
}

#[test]
fn a_narrowed_variable_cannot_be_assigned() {
    assert_eq!(
        errors("x: Int? = some(1)\nif x != none:\n    x = some(2)\n"),
        [AssignToNarrowed("x".to_string())]
    );
}

#[test]
fn narrowing_needs_a_variable_and_a_single_if() {
    let source = "\
function pick() -> Int?:
    return some(1)

if pick() != none:
    print(\"present\")
";
    assert_eq!(output(source), ["present"]);

    assert_eq!(
        errors("a: Int? = some(1)\nb: Int? = some(2)\nif a != none and b != none:\n    c = a + b\n"),
        [BinaryOperands {
            op: "+",
            left: optional(Type::Int),
            right: optional(Type::Int)
        }]
    );
}

#[test]
fn unit_cannot_be_optional() {
    assert_eq!(errors("x: Unit? = none\n"), [UnitAnnotation]);
}

#[test]
fn native_matches_the_interpreter_for_optionals() {
    let source = "\
function doubled(value: Int?) -> Int?:
    if value != none:
        return some(value * 2)
    return none

a = doubled(some(21))
if a != none:
    print(a)

print(doubled(none) == none)

name: String? = some(\"dovik\")
if name != none:
    print(upper(name))

n = parse_int(\"77\")
if n != none:
    print(n + 1)
print(parse_int(\"x\") == none)

at = find(\"key=value\", \"=\")
if at != none:
    print(substring(\"key=value\", 0, at))
print(find(\"abc\", \"z\") == none)

d: Dict<String, String> = {}
d[\"k\"] = \"v\"
hit = get(d, \"k\")
if hit != none:
    print(hit)
print(get(d, \"absent\") == none)
";

    let program = lowered(source);
    let expected = interp::run(&program);
    assert_eq!(expected.outcome, interp::Outcome::Finished, "{expected:?}");

    let dir = std::env::temp_dir().join("dovik-optional-tests");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let ll = dir.join("optional.ll");
    let binary = dir.join("optional");
    std::fs::write(&ll, backend::emit(&program, &SourceMap::single("test.dovk", source)))
        .expect("write ir");

    let build = std::process::Command::new("clang")
        .args(["-O2", "-flto=full"])
        .arg("-o")
        .arg(&binary)
        .arg(&ll)
        .arg(env!("DOVIK_RUNTIME_OBJECT"))
        .arg("-lm")
        .output()
        .expect("clang");
    assert!(
        build.status.success(),
        "clang rejected it:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let run = std::process::Command::new(&binary)
        .env("DOVIK_CHECK_LEAKS", "1")
        .output()
        .expect("run");

    let native = String::from_utf8_lossy(&run.stdout).to_string();
    let interpreted: String = expected
        .output
        .iter()
        .map(|line| format!("{line}\n"))
        .collect();

    assert_eq!(native, interpreted, "native differs from the interpreter");
    assert!(
        String::from_utf8_lossy(&run.stderr).contains("0 live object"),
        "leaked: {}",
        String::from_utf8_lossy(&run.stderr)
    );
}

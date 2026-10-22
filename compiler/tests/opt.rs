use compiler::error::TypeErrorKind;
use compiler::ir::RuntimeError;
use compiler::{check, interp, ir, lexer, lower, modules, opt, parser, resolve};

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
    lower::lower(&graph, &resolution, &typed)
}

fn folded(source: &str) -> ir::Program {
    let mut program = lowered(source);
    let errors = opt::optimize(&mut program);
    assert!(errors.is_empty(), "unexpected fold errors: {errors:?}");
    program
}

fn fold_errors(source: &str) -> Vec<TypeErrorKind> {
    let mut program = lowered(source);
    opt::optimize(&mut program)
        .into_iter()
        .map(|error| error.kind)
        .collect()
}

const CORPUS: &[&str] = &[
    "print(1 + 2 * 3)\n",
    "print(7 / 2)\n",
    "print(1.5 + 2.5)\n",
    "print(10 % 3)\n",
    "print(not true)\n",
    "print(1 < 2)\n",
    "print(to_float(3))\n",
    "print(to_int(3.9))\n",
    "x = 5\nif x > 3:\n    print(\"big\")\nelse:\n    print(\"small\")\n",
    "total = 0\nfor n in [1, 2, 3]:\n    total = total + n\nprint(total)\n",
    "n = 0\nwhile n < 3:\n    n = n + 1\nprint(n)\n",
    "for c in \"héllo\":\n    print(c)\n",
    "function f(n: Int) -> Int:\n    return n * 2\n\nprint(f(21))\n",
    "function g(n: Int) -> Int:\n    if n <= 1:\n        return 1\n    return n * g(n - 1)\n\nprint(g(6))\n",
    "a = [1, 2]\nb = a\nb[0] = 9\nprint(a[0])\n",
    "d = {\"k\": 1}\nd[\"j\"] = 2\nprint(len(d))\n",
    "print(true and false)\nprint(false or true)\n",
    "function loud() -> Bool:\n    print(\"ran\")\n    return true\n\nprint(false and loud())\n",
    "limit = 4\n\nfunction under(n: Int) -> Bool:\n    return n < limit\n\nprint(under(2))\n",
    "s = \"ab\"\nprint(s[0])\nprint(len(s))\n",
];

#[test]
fn optimization_never_changes_behaviour() {
    for source in CORPUS {
        let plain = interp::run(&lowered(source));
        let optimized = interp::run(&folded(source));

        assert_eq!(
            plain.output, optimized.output,
            "output changed for {source:?}"
        );
        assert_eq!(
            plain.outcome, optimized.outcome,
            "outcome changed for {source:?}"
        );
    }
}

#[test]
fn constant_arithmetic_is_folded_away() {
    let program = folded("print(1 + 2 * 3)\n");
    let arithmetic = program
        .functions
        .iter()
        .flat_map(|function| &function.blocks)
        .flat_map(|block| &block.instructions)
        .filter(|inst| {
            matches!(
                inst.kind,
                ir::InstKind::IntArith { .. } | ir::InstKind::FloatArith { .. }
            )
        })
        .count();

    assert_eq!(arithmetic, 0, "constants should leave no arithmetic behind");
    assert_eq!(interp::run(&program).output, ["7"]);
}

#[test]
fn a_fold_that_overflows_is_a_compile_error() {
    assert_eq!(
        fold_errors("x = 9223372036854775807 + 1\n"),
        [TypeErrorKind::FoldedCheckFailed(RuntimeError::IntegerOverflow)]
    );
    assert_eq!(
        fold_errors("x = 9223372036854775807 * 2\n"),
        [TypeErrorKind::FoldedCheckFailed(RuntimeError::IntegerOverflow)]
    );
}

#[test]
fn a_fold_that_divides_by_zero_is_a_compile_error() {
    assert_eq!(
        fold_errors("x = 7 / 0\n"),
        [TypeErrorKind::FoldedCheckFailed(RuntimeError::DivisionByZero)]
    );
    assert_eq!(
        fold_errors("x = 7.0 / 0.0\n"),
        [TypeErrorKind::FoldedCheckFailed(RuntimeError::DivisionByZero)]
    );
    assert_eq!(
        fold_errors("x = 0 / 0\n"),
        [TypeErrorKind::FoldedCheckFailed(RuntimeError::DivisionByZero)]
    );
}

#[test]
fn a_fold_with_a_zero_remainder_divisor_is_a_compile_error() {
    assert_eq!(
        fold_errors("x = 7 % 0\n"),
        [TypeErrorKind::FoldedCheckFailed(RuntimeError::RemainderByZero)]
    );
}

#[test]
fn an_invalid_constant_conversion_is_a_compile_error() {
    let source = "x = to_int(99999999999999999999999.0 * 10.0)\n";
    assert_eq!(
        fold_errors(source),
        [TypeErrorKind::FoldedCheckFailed(RuntimeError::InvalidConversion)]
    );
}

#[test]
fn float_overflow_still_folds_to_infinity() {
    let big = format!("{}.0", "9".repeat(38));
    let source = format!(
        "a = {big}\nb = a * a\nc = b * b\nd = c * c\nprint(d * d)\n"
    );
    assert!(fold_errors(&source).is_empty(), "spec 7.3 permits infinity");
    assert_eq!(interp::run(&folded(&source)).output, ["inf"]);
}

fn overflow_checks(source: &str) -> usize {
    folded(source)
        .checks()
        .iter()
        .filter(|check| **check == RuntimeError::IntegerOverflow)
        .count()
}

#[test]
fn a_check_survives_when_the_operand_is_unknown() {
    assert_eq!(
        overflow_checks("function f(n: Int) -> Int:\n    return n + 1\n\nprint(f(3))\n"),
        1,
        "a parameter could be anything, so the check must stay"
    );
    assert_eq!(
        overflow_checks("xs: List<Int> = [1]\ny = xs[0] + 1\nprint(y)\n"),
        1,
        "a list element could be anything"
    );
    assert_eq!(
        overflow_checks(
            "p = parse_int(\"5\")\nif p != none:\n    print(p * p)\n"
        ),
        1,
        "a parsed value could be anything"
    );
}

#[test]
fn a_loop_accumulator_keeps_its_check() {
    let source = "total = 0\ni = 0\nwhile i < 100:\n    total = total + i\n    i = i + 1\nprint(total)\n";
    assert!(
        overflow_checks(source) >= 1,
        "the accumulator is not bounded by the analysis, so its check must stay"
    );
}

#[test]
fn a_bounded_induction_variable_loses_its_check() {
    let source = "i = 0\nwhile i < 100:\n    i = i + 1\nprint(i)\n";
    assert_eq!(
        overflow_checks(source),
        0,
        "the loop guard bounds i, so i + 1 provably fits"
    );
}

#[test]
fn a_check_survives_near_the_integer_limit() {
    let source = "\
function risky(n: Int) -> Int:
    if n > 9000000000000000000:
        return n + n
    return 0

print(risky(1))
";
    assert!(
        overflow_checks(source) >= 1,
        "a guard that permits huge values must not license eliding"
    );
}

#[test]
fn multiplication_is_not_elided_without_tight_bounds() {
    let source = "\
function scale(n: Int) -> Int:
    if n < 1000:
        return n * n
    return 0

print(scale(3))
";
    assert_eq!(
        overflow_checks(source),
        1,
        "n < 1000 leaves n unbounded below, so n * n can overflow"
    );
}

#[test]
fn a_constant_branch_becomes_a_jump() {
    let program = folded("if true:\n    print(1)\nelse:\n    print(2)\n");
    let branches = program
        .functions
        .iter()
        .flat_map(|function| &function.blocks)
        .filter(|block| matches!(block.terminator, ir::Terminator::Branch { .. }))
        .count();

    assert_eq!(branches, 0);
    assert_eq!(interp::run(&program).output, ["1"]);
}

#[test]
fn folding_crosses_instructions_within_a_block() {
    assert_eq!(interp::run(&folded("print(1 + 2 + 3 + 4)\n")).output, ["10"]);
    assert_eq!(interp::run(&folded("print((1 + 2) * (3 + 4))\n")).output, ["21"]);
    assert_eq!(interp::run(&folded("print(-(2 * 3))\n")).output, ["-6"]);
}

#[test]
fn a_register_read_in_another_block_keeps_its_definition() {
    let source = "\
function loud() -> Bool:
    print(\"ran\")
    return true

print(true and loud())
";
    assert_eq!(interp::run(&folded(source)).output, ["ran", "true"]);
}

use compiler::ir::{InstKind, RuntimeError, Terminator};
use compiler::{check, ir, lexer, lower, parser, resolve};

fn build(source: &str) -> ir::Program {
    let (tokens, lex_errors) = lexer::lex(source);
    assert_eq!(lex_errors, [], "lex errors");
    let (ast, parse_errors) = parser::parse(source, &tokens);
    assert!(parse_errors.is_empty(), "parse errors: {parse_errors:?}");
    let (resolution, resolve_errors) = resolve::resolve(&ast);
    assert!(resolve_errors.is_empty(), "resolve errors: {resolve_errors:?}");
    let (typed, type_errors) = check::check(&ast, &resolution);
    assert!(type_errors.is_empty(), "type errors: {type_errors:?}");
    lower::lower(&ast, &resolution, &typed)
}

fn text(source: &str) -> String {
    build(source).to_string()
}

fn entry(program: &ir::Program) -> &ir::Function {
    &program.functions[program.main]
}

fn instructions(program: &ir::Program, function: &str) -> Vec<String> {
    program
        .functions
        .iter()
        .find(|f| f.name == function)
        .expect("function")
        .blocks
        .iter()
        .flat_map(|block| &block.instructions)
        .map(|inst| inst.kind.to_string())
        .collect()
}

#[test]
fn every_block_is_terminated() {
    let source = "\
function f(n: Int) -> Int:
    if n > 0:
        return 1
    while n < 10:
        n = n + 1
        if n == 5:
            break
    for x in [1, 2]:
        print(x)
    return 0
";
    for function in &build(source).functions {
        for (index, block) in function.blocks.iter().enumerate() {
            assert!(
                !matches!(block.terminator, Terminator::Unreachable)
                    || block.instructions.is_empty(),
                "{}: block{index} has instructions but no terminator",
                function.name
            );
        }
    }
}

#[test]
fn checked_arithmetic_is_explicit() {
    let program = build("x = 1 + 2\ny = 7 % 3\nz: Float = 7 / 2\nw = to_int(1.5)\n");
    let checks = program.checks();

    assert!(checks.contains(&RuntimeError::IntegerOverflow));
    assert!(checks.contains(&RuntimeError::RemainderByZero));
    assert!(checks.contains(&RuntimeError::DivisionByZero));
    assert!(checks.contains(&RuntimeError::InvalidConversion));
}

#[test]
fn indexing_carries_its_bounds_check() {
    let program = build("xs = [1, 2]\na = xs[0]\nxs[1] = 5\ns = \"ab\"\nc = s[0]\n");
    let checks = program.checks();
    let bounds = checks
        .iter()
        .filter(|check| **check == RuntimeError::IndexOutOfBounds)
        .count();

    assert_eq!(bounds, 3, "list get, list set, and string index");
}

#[test]
fn dictionary_reads_carry_a_missing_key_check() {
    let program = build("d = {\"k\": 1}\nv = d[\"k\"]\nd[\"j\"] = 2\n");
    let checks = program.checks();

    assert!(checks.contains(&RuntimeError::MissingKey));
    assert_eq!(
        checks
            .iter()
            .filter(|c| **c == RuntimeError::MissingKey)
            .count(),
        1
    );
}

#[test]
fn float_arithmetic_is_unchecked() {
    let program = build("x = 1.5 + 2.5\ny = 1.5 * 2.0\nz = -1.5\n");
    assert_eq!(program.checks(), []);
}

#[test]
fn integer_arithmetic_is_checked() {
    let program = build("x = 1 + 2\ny = -x\n");
    assert_eq!(
        program.checks(),
        [RuntimeError::IntegerOverflow, RuntimeError::IntegerOverflow]
    );
}

#[test]
fn division_widens_int_operands() {
    let rendered = text("x: Float = 7 / 2\n");
    assert!(rendered.contains("int.to_float 7"), "{rendered}");
    assert!(rendered.contains("int.to_float 2"), "{rendered}");
    assert!(rendered.contains("float.div"), "{rendered}");
}

#[test]
fn division_leaves_float_operands_alone() {
    let rendered = text("x: Float = 7.0 / 2.0\n");
    assert!(!rendered.contains("int.to_float"), "{rendered}");
    assert!(rendered.contains("float.div 7.0, 2.0"), "{rendered}");
}

#[test]
fn string_concatenation_is_not_integer_addition() {
    let rendered = text("x = \"a\" + \"b\"\n");
    assert!(rendered.contains("string.concat"), "{rendered}");
    assert!(!rendered.contains("int.add"), "{rendered}");
}

#[test]
fn the_integer_minimum_is_folded() {
    let program = build("x = -9223372036854775808\n");
    assert_eq!(program.checks(), []);
    assert!(text("x = -9223372036854775808\n").contains("-9223372036854775808"));
}

#[test]
fn logical_operators_short_circuit() {
    let program = build("function f() -> Bool:\n    return true\n\nx = false and f()\n");
    let main = entry(&program);
    let branches = main
        .blocks
        .iter()
        .filter(|block| matches!(block.terminator, Terminator::Branch { .. }))
        .count();

    assert!(branches >= 1, "expected a branch for `and`");

    let condition_block = main
        .blocks
        .iter()
        .find(|block| matches!(block.terminator, Terminator::Branch { .. }))
        .expect("branch");
    assert!(
        !condition_block
            .instructions
            .iter()
            .any(|inst| matches!(inst.kind, InstKind::Call { .. })),
        "the right operand was evaluated before the branch"
    );
}

#[test]
fn globals_are_separate_from_locals() {
    let program = build("limit = 10\n\nfunction under(n: Int) -> Bool:\n    return n < limit\n");
    assert_eq!(program.globals.len(), 1);
    assert_eq!(program.globals[0].name, "limit");

    let body = instructions(&program, "under");
    assert!(body.iter().any(|inst| inst.starts_with("r") && inst.contains("load @0")), "{body:?}");
    assert!(!body.iter().any(|inst| inst.contains("store @")), "{body:?}");
}

#[test]
fn top_level_statements_become_one_entry_point() {
    let program = build("a = 1\nb = 2\nprint(a + b)\n");
    let main = entry(&program);
    assert_eq!(main.name, "main");
    assert!(
        instructions(&program, "main")
            .iter()
            .any(|inst| inst.starts_with("print")),
        "the entry point holds the top-level work"
    );
}

#[test]
fn parameters_are_the_first_locals() {
    let program = build("function f(a: Int, b: String):\n    print(b)\n");
    let function = program
        .functions
        .iter()
        .find(|f| f.name == "f")
        .expect("f");

    assert_eq!(function.param_count, 2);
    assert_eq!(function.locals[0].name, "a");
    assert_eq!(function.locals[1].name, "b");
}

#[test]
fn iteration_over_a_list_walks_an_index() {
    let rendered = text("for x in [1, 2]:\n    print(x)\n");
    assert!(rendered.contains("list.get"), "{rendered}");
    assert!(rendered.contains("len ["), "{rendered}");
    assert!(rendered.contains("int.add"), "{rendered}");
}

#[test]
fn iteration_over_a_string_uses_a_cursor() {
    let rendered = text("for c in \"abc\":\n    print(c)\n");
    assert!(rendered.contains("string.next"), "{rendered}");
    assert!(rendered.contains("string.byte_len"), "{rendered}");
    assert!(!rendered.contains("string.index"), "{rendered}");
}

#[test]
fn break_jumps_to_the_loop_exit() {
    let program = build("while true:\n    break\n");
    let main = entry(&program);

    let exits: Vec<_> = main
        .blocks
        .iter()
        .enumerate()
        .filter(|(_, block)| matches!(block.terminator, Terminator::Jump(_)))
        .collect();

    assert!(!exits.is_empty(), "break produces a jump");
}

#[test]
fn continue_jumps_to_the_loop_header() {
    let program = build("n = 0\nwhile n < 3:\n    n = n + 1\n    continue\n");
    let main = entry(&program);
    assert!(
        main.blocks
            .iter()
            .any(|block| matches!(block.terminator, Terminator::Jump(_))),
        "continue produces a jump"
    );
}

#[test]
fn calls_point_at_functions_declared_later() {
    let program = build("\
function first() -> Int:
    return second()

function second() -> Int:
    return 1
");
    let body = instructions(&program, "first");
    assert!(body.iter().any(|inst| inst.contains("call fn1")), "{body:?}");
}

#[test]
fn a_unit_call_has_no_destination() {
    let program = build("function greet():\n    print(1)\n\ngreet()\n");
    let body = instructions(&program, "main");
    assert!(
        body.iter().any(|inst| inst == "call fn0()"),
        "no result register: {body:?}"
    );
}

#[test]
fn string_literals_are_interned_once() {
    let program = build("a = \"same\"\nb = \"same\"\nc = \"other\"\n");
    assert_eq!(program.strings, ["same", "other"]);
}

#[test]
fn structural_comparison_keeps_the_operand_type() {
    let rendered = text("a: List<Int> = [1]\nb: List<Int> = [2]\nc = a == b\n");
    assert!(rendered.contains("cmp.eq [List<Int>]"), "{rendered}");
}

#[test]
fn the_spec_example_program_lowers() {
    let program = build("\
function factorial(n: Int) -> Int:
    if n <= 1:
        return 1
    return n * factorial(n - 1)

names: List<String> = [\"Dawit\", \"Abel\"]

for person in names:
    print(\"Hello, \" + person)

total: Int = factorial(5) + len(names)
ratio: Float = total / 2
");
    assert_eq!(program.functions.len(), 2, "factorial plus the entry point");
    assert!(program.checks().contains(&RuntimeError::IntegerOverflow));
    assert!(program.checks().contains(&RuntimeError::DivisionByZero));
}

use compiler::ir::RuntimeError;
use compiler::{check, interp, ir, lexer, lower, modules, opt, parser, resolve, runtime};

fn build(source: &str, optimize: bool) -> ir::Program {
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

fn output(source: &str) -> Vec<String> {
    let run = interp::run(&build(source, true));
    assert_eq!(run.outcome, interp::Outcome::Finished, "{run:?}");
    run.output
}

fn trap(source: &str) -> RuntimeError {
    let run = interp::run(&build(source, true));
    run.trap().expect("expected a trap").error
}

#[test]
fn arithmetic_and_printing() {
    assert_eq!(output("print(1 + 2)\n"), ["3"]);
    assert_eq!(output("print(10 - 3 * 2)\n"), ["4"]);
    assert_eq!(output("print(1.5 + 2.25)\n"), ["3.75"]);
    assert_eq!(output("print(true)\nprint(false)\n"), ["true", "false"]);
    assert_eq!(output("print(\"text\")\n"), ["text"]);
}

#[test]
fn division_always_produces_a_float() {
    assert_eq!(output("print(7 / 2)\n"), ["3.5"]);
    assert_eq!(output("print(6 / 3)\n"), ["2"]);
    assert_eq!(output("print(7.0 / 2)\n"), ["3.5"]);
}

#[test]
fn remainder_takes_the_sign_of_the_left_operand() {
    assert_eq!(output("a = 7\nb = 3\nprint(-a % b)\n"), ["-1"]);
    assert_eq!(output("a = 7\nb = 3\nprint(a % b)\n"), ["1"]);
}

#[test]
fn recursion() {
    let source = "\
function factorial(n: Int) -> Int:
    if n <= 1:
        return 1
    return n * factorial(n - 1)

print(factorial(10))
";
    assert_eq!(output(source), ["3628800"]);
}

#[test]
fn mutual_recursion() {
    let source = "\
function is_even(n: Int) -> Bool:
    if n == 0:
        return true
    return is_odd(n - 1)

function is_odd(n: Int) -> Bool:
    if n == 0:
        return false
    return is_even(n - 1)

print(is_even(10))
print(is_odd(10))
";
    assert_eq!(output(source), ["true", "false"]);
}

#[test]
fn loops_and_break() {
    let source = "\
total = 0
for n in [1, 2, 3, 4, 5]:
    total = total + n
    if total > 9:
        break
print(total)
";
    assert_eq!(output(source), ["10"]);
}

#[test]
fn continue_skips_the_rest_of_the_iteration() {
    let source = "\
total = 0
n = 0
while n < 5:
    n = n + 1
    if n == 3:
        continue
    total = total + n
print(total)
";
    assert_eq!(output(source), ["12"]);
}

#[test]
fn string_iteration_is_by_scalar() {
    assert_eq!(output("for c in \"héllo\":\n    print(c)\n"), ["h", "é", "l", "l", "o"]);
}

#[test]
fn length_counts_scalars_not_bytes() {
    assert_eq!(output("print(len(\"héllo\"))\n"), ["5"]);
    assert_eq!(output("print(len(\"é\"))\n"), ["1"]);
}

#[test]
fn string_indexing_yields_one_scalar() {
    assert_eq!(output("s = \"héllo\"\nprint(s[1])\n"), ["é"]);
}

#[test]
fn collections() {
    assert_eq!(output("xs = [1, 2, 3]\nprint(xs[1])\nprint(len(xs))\n"), ["2", "3"]);
    assert_eq!(
        output("d = {\"a\": 1, \"b\": 2}\nprint(d[\"b\"])\nprint(len(d))\n"),
        ["2", "2"]
    );
}

#[test]
fn lists_are_reference_types() {
    let source = "\
a = [1, 2, 3]
b = a
b[0] = 99
print(a[0])
";
    assert_eq!(output(source), ["99"]);
}

#[test]
fn rebinding_a_parameter_does_not_reach_the_caller() {
    let source = "\
function rebind(xs: List<Int>):
    xs = [9]

items = [1, 2, 3]
rebind(items)
print(items[0])
";
    assert_eq!(output(source), ["1"]);
}

#[test]
fn mutating_a_parameter_does_reach_the_caller() {
    let source = "\
function mutate(xs: List<Int>):
    xs[0] = 99

items = [1, 2, 3]
mutate(items)
print(items[0])
";
    assert_eq!(output(source), ["99"]);
}

#[test]
fn logical_operators_short_circuit() {
    let source = "\
function loud() -> Bool:
    print(\"evaluated\")
    return true

a = false and loud()
print(a)
b = true or loud()
print(b)
";
    assert_eq!(output(source), ["false", "true"]);
}

#[test]
fn structural_equality() {
    let source = "\
a: List<Int> = [1, 2]
b: List<Int> = [1, 2]
c: List<Int> = [1, 3]
print(a == b)
print(a == c)
d = {\"x\": 1}
e = {\"x\": 1}
print(d == e)
";
    assert_eq!(output(source), ["true", "false", "true"]);
}

#[test]
fn string_ordering_is_by_scalar() {
    assert_eq!(output("print(\"a\" < \"b\")\nprint(\"b\" < \"a\")\n"), ["true", "false"]);
}

#[test]
fn integer_overflow_traps() {
    let source = "x = 9223372036854775807\nprint(x + 1)\n";
    assert_eq!(trap(source), RuntimeError::IntegerOverflow);
}

#[test]
fn negating_the_minimum_traps() {
    let source = "x = -9223372036854775808\nprint(-x)\n";
    assert_eq!(trap(source), RuntimeError::IntegerOverflow);
}

#[test]
fn division_by_zero_traps() {
    assert_eq!(trap("d = 0\nprint(7 / d)\n"), RuntimeError::DivisionByZero);
    assert_eq!(trap("d = 0.0\nprint(7.0 / d)\n"), RuntimeError::DivisionByZero);
    assert_eq!(trap("d = 0\nn = 0\nprint(n / d)\n"), RuntimeError::DivisionByZero);
}

#[test]
fn remainder_by_zero_traps() {
    assert_eq!(trap("d = 0\nprint(7 % d)\n"), RuntimeError::RemainderByZero);
}

#[test]
fn an_invalid_conversion_traps() {
    let source = "big = 9999999999.0\nprint(to_int(big * big))\n";
    assert_eq!(trap(source), RuntimeError::InvalidConversion);
}

#[test]
fn an_out_of_bounds_index_traps() {
    assert_eq!(trap("xs = [1]\ni = 5\nprint(xs[i])\n"), RuntimeError::IndexOutOfBounds);
    assert_eq!(trap("s = \"a\"\ni = 5\nprint(s[i])\n"), RuntimeError::IndexOutOfBounds);
    assert_eq!(
        trap("xs = [1]\ni = 1\nxs[i] = 2\n"),
        RuntimeError::IndexOutOfBounds
    );
}

#[test]
fn a_missing_key_traps() {
    let source = "d = {\"a\": 1}\nk = \"b\"\nprint(d[k])\n";
    assert_eq!(trap(source), RuntimeError::MissingKey);
}

#[test]
fn a_trap_reports_where_it_happened() {
    let source = "d = 0\nprint(7 / d)\n";
    let run = interp::run(&build(source, true));
    let trap = run.trap().expect("trap");
    assert_eq!(&source[trap.span.start..trap.span.end], "7 / d");
}

#[test]
fn output_stops_at_the_trap() {
    let source = "print(\"before\")\nd = 0\nprint(7 / d)\nprint(\"after\")\n";
    let run = interp::run(&build(source, true));
    assert_eq!(run.output, ["before"]);
    assert!(run.trap().is_some());
}

#[test]
fn float_formatting() {
    assert_eq!(runtime::float(f64::INFINITY), "inf");
    assert_eq!(runtime::float(f64::NEG_INFINITY), "-inf");
    assert_eq!(runtime::float(f64::NAN), "nan");
    assert_eq!(runtime::float(1.75), "1.75");
}

#[test]
fn nan_is_not_equal_to_itself() {
    let nan = compiler::runtime::Value::Float(f64::NAN);
    let result = runtime::compare(ir::CompareOp::Eq, &nan, &nan);
    assert!(!result.as_bool());
}

#[test]
fn globals_are_readable_from_functions() {
    let source = "\
limit = 10

function under(n: Int) -> Bool:
    return n < limit

print(under(5))
print(under(50))
";
    assert_eq!(output(source), ["true", "false"]);
}

#[test]
fn nested_loops() {
    let source = "\
for word in [\"ab\", \"cd\"]:
    for scalar in word:
        print(scalar)
";
    assert_eq!(output(source), ["a", "b", "c", "d"]);
}

#[test]
fn deep_recursion_traps_in_the_interpreter_too() {
    let source = "\
function down(n: Int) -> Int:
    if n <= 0:
        return 0
    return 1 + down(n - 1)

print(down(100000000))
";

    assert_eq!(trap(source), RuntimeError::StackExhausted);
}

use compiler::error::TypeErrorKind::{self, *};
use compiler::types::Type;
use compiler::{check, lexer, parser, resolve};

fn errors(source: &str) -> Vec<TypeErrorKind> {
    let (tokens, lex_errors) = lexer::lex(source);
    assert_eq!(lex_errors, [], "lex errors in {source:?}");
    let (program, parse_errors) = parser::parse(source, &tokens);
    assert!(parse_errors.is_empty(), "parse errors: {parse_errors:?}");
    let (resolution, resolve_errors) = resolve::resolve(&program);
    assert!(resolve_errors.is_empty(), "resolve errors: {resolve_errors:?}");
    check::check(&program, &resolution)
        .1
        .into_iter()
        .map(|e| e.kind)
        .collect()
}

fn ok(source: &str) {
    assert_eq!(errors(source), [], "expected no errors for {source:?}");
}

fn list(item: Type) -> Type {
    Type::List(Box::new(item))
}

fn dict(key: Type, value: Type) -> Type {
    Type::Dict(Box::new(key), Box::new(value))
}

#[test]
fn literal_inference() {
    ok("\
age = 23
name = \"Dawit\"
height = 1.75
is_student = true
");
}

#[test]
fn annotations_must_match_exactly() {
    assert_eq!(
        errors("height: Float = 2\n"),
        [AnnotationMismatch { annotated: Type::Float, found: Type::Int }]
    );
    ok("age: Int = 23\nheight: Float = 1.75\n");
}

#[test]
fn reassignment_cannot_change_type() {
    ok("age = 23\nage = 24\n");
    assert_eq!(
        errors("age = 23\nage = 25.5\n"),
        [ReassignMismatch { name: "age".into(), expected: Type::Int, found: Type::Float }]
    );
    assert_eq!(
        errors("age = 23\nage = \"Dawit\"\n"),
        [ReassignMismatch { name: "age".into(), expected: Type::Int, found: Type::Str }]
    );
}

#[test]
fn mixed_numeric_operands_are_rejected() {
    assert_eq!(
        errors("x = 1 + 2.5\n"),
        [BinaryOperands { op: "+", left: Type::Int, right: Type::Float }]
    );
    ok("x = to_float(1) + 2.5\n");
    ok("x = 1 + 2\ny = 1.5 - 0.5\nz = 2 * 3\n");
}

#[test]
fn string_concatenation() {
    ok("x = \"Hello, \" + \"Dawit\"\n");
    assert_eq!(
        errors("x = \"count: \" + 5\n"),
        [BinaryOperands { op: "+", left: Type::Str, right: Type::Int }]
    );
}

#[test]
fn plus_does_not_join_collections() {
    assert_eq!(
        errors("a: List<Int> = [1]\nb: List<Int> = [2]\nc = a + b\n"),
        [BinaryOperands { op: "+", left: list(Type::Int), right: list(Type::Int) }]
    );
}

#[test]
fn division_always_yields_float() {
    ok("result: Float = 7 / 2\nratio: Float = 7.0 / 2\n");
    assert_eq!(
        errors("x: Int = 7 / 2\n"),
        [AnnotationMismatch { annotated: Type::Int, found: Type::Float }]
    );
}

#[test]
fn remainder_is_int_only() {
    ok("x = 7 % 3\n");
    assert_eq!(
        errors("x = 7.0 % 3.0\n"),
        [BinaryOperands { op: "%", left: Type::Float, right: Type::Float }]
    );
}

#[test]
fn comparison_requires_matching_types() {
    ok("a = 1 < 2\nb = 1.5 >= 0.5\nc = \"a\" < \"b\"\n");
    assert_eq!(
        errors("x = 1 < 2.0\n"),
        [BinaryOperands { op: "<", left: Type::Int, right: Type::Float }]
    );
    assert_eq!(
        errors("x = 1 < 2 < 3\n"),
        [BinaryOperands { op: "<", left: Type::Bool, right: Type::Int }]
    );
}

#[test]
fn equality_covers_more_types_than_ordering() {
    ok("a = true == false\nb: List<Int> = [1]\nc = b == b\n");
    assert_eq!(
        errors("x = true < false\n"),
        [BinaryOperands { op: "<", left: Type::Bool, right: Type::Bool }]
    );
}

#[test]
fn logical_operators_need_bool() {
    ok("x = true and not false\n");
    assert_eq!(
        errors("x = 1 and true\n"),
        [BinaryOperands { op: "and", left: Type::Int, right: Type::Bool }]
    );
    assert_eq!(errors("x = not 1\n"), [UnaryOperand { op: "not", found: Type::Int }]);
}

#[test]
fn conditions_must_be_bool() {
    assert_eq!(errors("count = 1\nif count:\n    print(1)\n"), [ConditionNotBool(Type::Int)]);
    assert_eq!(errors("while 1:\n    print(1)\n"), [ConditionNotBool(Type::Int)]);
    ok("count = 1\nif count > 0:\n    print(1)\n");
}

#[test]
fn unary_minus() {
    ok("x = -5\ny = -1.5\n");
    assert_eq!(errors("x = -\"a\"\n"), [UnaryOperand { op: "-", found: Type::Str }]);
}

#[test]
fn integer_literal_range() {
    ok("x = 9223372036854775807\n");
    ok("y = -9223372036854775808\n");
    assert_eq!(errors("x = 9223372036854775808\n"), [IntegerLiteralOutOfRange]);
}

#[test]
fn list_elements_share_one_type() {
    ok("numbers = [1, 2, 3]\nnames = [\"Dawit\", \"Abel\"]\ngrid = [[1, 2], [3, 4]]\n");
    assert_eq!(
        errors("mixed = [1, \"hello\"]\n"),
        [MixedListElements { first: Type::Int, found: Type::Str }]
    );
    assert_eq!(
        errors("widened = [1, 2.0]\n"),
        [MixedListElements { first: Type::Int, found: Type::Float }]
    );
}

#[test]
fn empty_literals_need_an_annotation() {
    ok("numbers: List<Int> = []\nages: Dict<String, Int> = {}\n");
    assert_eq!(errors("values = []\n"), [CannotInferEmptyList]);
    assert_eq!(errors("values = {}\n"), [CannotInferEmptyDict]);
    ok("scores: List<List<Int>> = []\n");
}

#[test]
fn an_empty_literal_takes_a_parameter_type() {
    ok("function f(xs: List<Int>):\n    print(len(xs))\n\nf([])\n");
}

#[test]
fn dictionary_literals() {
    ok("ages = {\"Dawit\": 23, \"Abel\": 21}\n");
    assert_eq!(
        errors("d = {\"a\": 1, \"b\": \"two\"}\n"),
        [MixedDictValues { first: Type::Int, found: Type::Str }]
    );
    assert_eq!(
        errors("d = {\"a\": 1, 2: 3}\n"),
        [MixedDictKeys { first: Type::Str, found: Type::Int }]
    );
}

#[test]
fn float_keys_are_rejected() {
    assert_eq!(errors("d = {1.5: 1}\n"), [InvalidDictKeyType(Type::Float)]);
    assert_eq!(
        errors("d: Dict<Float, Int> = {}\n"),
        [InvalidDictKeyType(Type::Float)]
    );
    assert_eq!(
        errors("d: Dict<List<Int>, Int> = {}\n"),
        [InvalidDictKeyType(list(Type::Int))]
    );
    ok("a = {1: \"x\"}\nb = {true: \"y\"}\nc = {\"k\": \"z\"}\n");
}

#[test]
fn list_indexing() {
    ok("xs = [1, 2]\nfirst: Int = xs[0]\n");
    assert_eq!(
        errors("xs = [1, 2]\ny = xs[\"a\"]\n"),
        [IndexType { expected: Type::Int, found: Type::Str }]
    );
    assert_eq!(errors("x = 1\ny = x[0]\n"), [NotIndexable(Type::Int)]);
}

#[test]
fn dictionary_indexing() {
    ok("ages = {\"Dawit\": 23}\nage: Int = ages[\"Dawit\"]\n");
    assert_eq!(
        errors("ages = {\"Dawit\": 23}\nx = ages[1]\n"),
        [IndexType { expected: Type::Str, found: Type::Int }]
    );
}

#[test]
fn string_indexing_yields_a_string() {
    ok("s = \"abc\"\nfirst: String = s[0]\n");
    assert_eq!(
        errors("s = \"abc\"\nx: Int = s[0]\n"),
        [AnnotationMismatch { annotated: Type::Int, found: Type::Str }]
    );
}

#[test]
fn strings_cannot_be_mutated() {
    assert_eq!(errors("s = \"abc\"\ns[0] = \"x\"\n"), [StringsAreImmutable]);
}

#[test]
fn index_assignment_checks_the_element_type() {
    ok("xs = [1, 2]\nxs[0] = 10\n");
    assert!(matches!(
        errors("xs = [1, 2]\nxs[0] = \"a\"\n").as_slice(),
        [ReassignMismatch { expected: Type::Int, found: Type::Str, .. }]
    ));
    ok("ages = {\"Dawit\": 23}\nages[\"Marta\"] = 30\n");
}

#[test]
fn iteration() {
    ok("names = [\"Dawit\"]\nfor name in names:\n    print(name)\n");
    ok("for scalar in \"abc\":\n    print(scalar)\n");
    assert_eq!(
        errors("ages = {\"a\": 1}\nfor k in ages:\n    print(k)\n"),
        [NotIterable(dict(Type::Str, Type::Int))]
    );
    assert_eq!(errors("for x in 1:\n    print(x)\n"), [NotIterable(Type::Int)]);
}

#[test]
fn the_loop_variable_has_the_element_type() {
    assert_eq!(
        errors("for n in [1, 2]:\n    x: String = n\n"),
        [AnnotationMismatch { annotated: Type::Str, found: Type::Int }]
    );
}

#[test]
fn calls_check_arity_and_types() {
    let base = "function multiply(a: Int, b: Int):\n    return a * b\n\n";
    ok(&format!("{base}result: Int = multiply(3, 5)\n"));
    assert_eq!(
        errors(&format!("{base}multiply(3, \"five\")\n")),
        [ArgumentType { name: "multiply".into(), position: 2, expected: Type::Int, found: Type::Str }]
    );
    assert_eq!(
        errors(&format!("{base}multiply(3)\n")),
        [ArgumentCount { name: "multiply".into(), expected: 2, found: 1 }]
    );
}

#[test]
fn return_type_inference() {
    ok("function add(a: Int, b: Int):\n    return a + b\n\nx: Int = add(1, 2)\n");
    ok("function f() -> Bool:\n    return true\n\nb: Bool = f()\n");
}

#[test]
fn returns_must_agree() {
    assert_eq!(
        errors("function f(flag: Bool):\n    if flag:\n        return 1\n    return \"two\"\n"),
        [ReturnTypeMismatch { expected: Type::Int, found: Type::Str }]
    );
}

#[test]
fn an_annotated_return_type_is_enforced() {
    assert_eq!(
        errors("function f() -> Int:\n    return \"a\"\n"),
        [ReturnTypeMismatch { expected: Type::Int, found: Type::Str }]
    );
}

#[test]
fn unit_functions() {
    ok("function greet(name: String):\n    print(\"Hello, \" + name)\n\ngreet(\"Dawit\")\n");
    ok("function early(flag: Bool):\n    if flag:\n        return\n    print(1)\n");
}

#[test]
fn a_unit_result_is_not_a_value() {
    assert_eq!(errors("x = print(\"hi\")\n"), [UnitAsValue]);
    assert_eq!(
        errors("function f():\n    print(1)\n\nx = f()\n"),
        [UnitAsValue]
    );
    assert_eq!(errors("x: Unit = 1\n"), [UnitAnnotation]);
    assert_eq!(
        errors("function f(a: Unit):\n    print(1)\n"),
        [UnitAnnotation]
    );
}

#[test]
fn every_path_must_return_a_value() {
    assert!(matches!(
        errors("function f(flag: Bool) -> Int:\n    if flag:\n        return 1\n").as_slice(),
        [MissingReturn { .. }]
    ));
    ok("\
function f(flag: Bool) -> Int:
    if flag:
        return 1
    else:
        return 2
");
    ok("function f(flag: Bool) -> Int:\n    if flag:\n        return 1\n    return 2\n");
    assert!(matches!(
        errors("function f() -> Int:\n    while true:\n        return 1\n").as_slice(),
        [MissingReturn { .. }]
    ));
}

#[test]
fn mutual_recursion_infers_through_a_base_case() {
    ok("\
function is_even(n: Int) -> Bool:
    if n == 0:
        return true
    return is_odd(n - 1)

function is_odd(n: Int) -> Bool:
    if n == 0:
        return false
    return is_even(n - 1)
");
    ok("\
function is_even(n: Int) -> Bool:
    if n == 0:
        return true
    return is_odd(n - 1)

function is_odd(n: Int):
    if n == 0:
        return false
    return is_even(n - 1)
");
}

#[test]
fn a_cycle_without_a_base_case_needs_an_annotation() {
    let source = "\
function ping(n: Int):
    return pong(n)

function pong(n: Int):
    return ping(n)
";
    assert_eq!(
        errors(source),
        [
            CannotInferReturnType("ping".into()),
            CannotInferReturnType("pong".into())
        ]
    );
}

#[test]
fn direct_recursion_infers_from_its_base_case() {
    ok("\
function factorial(n: Int):
    if n <= 1:
        return 1
    return n * factorial(n - 1)
");
}

#[test]
fn builtins() {
    ok("print(1)\nprint(1.5)\nprint(true)\nprint(\"a\")\n");
    ok("a: Int = len(\"abc\")\nb: Int = len([1])\nc: Int = len({\"k\": 1})\n");
    ok("a: Int = to_int(1.5)\nb: Float = to_float(1)\n");

    assert_eq!(
        errors("print([1])\n"),
        [BuiltinArgument { name: "print", found: list(Type::Int) }]
    );
    assert_eq!(errors("x = len(1)\n"), [BuiltinArgument { name: "len", found: Type::Int }]);
    assert_eq!(
        errors("x = to_int(1)\n"),
        [BuiltinArgument { name: "to_int", found: Type::Int }]
    );
    assert_eq!(
        errors("x = to_float(1.5)\n"),
        [BuiltinArgument { name: "to_float", found: Type::Float }]
    );
    assert_eq!(
        errors("print(1, 2)\n"),
        [ArgumentCount { name: "print".into(), expected: 1, found: 2 }]
    );
}

#[test]
fn functions_are_not_values() {
    assert_eq!(
        errors("function f():\n    print(1)\n\nx = f\n"),
        [FunctionAsValue("f".into())]
    );
    assert_eq!(errors("x = print\n"), [BuiltinAsValue("print".into())]);
}

#[test]
fn calling_a_non_function() {
    assert_eq!(errors("x = 1\ny = x(2)\n"), [NotCallable(Type::Int)]);
}

#[test]
fn return_outside_a_function() {
    assert_eq!(errors("return 1\n"), [ReturnOutsideFunction]);
}

#[test]
fn one_mistake_reports_once() {
    let source = "\
function f(a: Int) -> Int:
    return a

x = f(\"wrong\")
y = x + 1
z = x * 2
";
    assert_eq!(errors(source).len(), 1);
}

#[test]
fn reference_semantics_do_not_copy() {
    ok("\
a = [1, 2, 3]
b = a
b[0] = 99
");
}

#[test]
fn the_spec_example_program_checks() {
    ok("\
function factorial(n: Int) -> Int:
    if n <= 1:
        return 1
    return n * factorial(n - 1)

function greet(name: String):
    print(\"Hello, \" + name)

names: List<String> = [\"Dawit\", \"Abel\"]
ages: Dict<String, Int> = {\"Dawit\": 23}

for person in names:
    greet(person)

count = 0
while count < 3:
    count = count + 1

ages[\"Marta\"] = 30
total: Int = factorial(5) + len(names) * 2 - 1
flag: Bool = not (count > 1 and total % 2 == 0)
ratio: Float = total / 2
");
}

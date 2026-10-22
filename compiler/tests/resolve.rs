use compiler::error::ResolveErrorKind::{self, *};
use compiler::resolve::{self, SymbolKind};
use compiler::{lexer, modules, parser};

fn errors(source: &str) -> Vec<ResolveErrorKind> {
    let (tokens, lex_errors) = lexer::lex(source);
    assert_eq!(lex_errors, [], "lex errors in {source:?}");
    let (program, parse_errors) = parser::parse(source, &tokens);
    assert!(parse_errors.is_empty(), "parse errors: {parse_errors:?}");
    resolve::resolve(&modules::unit_graph("test.dovk", source, program)).1.into_iter().map(|e| e.kind).collect()
}

fn errors_unchecked(source: &str) -> Vec<ResolveErrorKind> {
    let (tokens, _) = lexer::lex(source);
    let (program, _) = parser::parse(source, &tokens);
    resolve::resolve(&modules::unit_graph("test.dovk", source, program)).1.into_iter().map(|e| e.kind).collect()
}

fn ok(source: &str) {
    assert_eq!(errors(source), [], "expected no errors for {source:?}");
}

#[test]
fn declaration_then_reassignment() {
    ok("age = 23\nage = 24\n");
}

#[test]
fn reassignment_in_a_loop_does_not_declare() {
    let source = "total = 0\nwhile total < 10:\n    total = total + 1\n";
    ok(source);

    let (tokens, _) = lexer::lex(source);
    let (program, _) = parser::parse(source, &tokens);
    let (resolution, _) = resolve::resolve(&modules::unit_graph("test.dovk", source, program));

    let declaring = resolution.declares.len();
    assert_eq!(declaring, 1, "exactly one declaration of `total`");
}

#[test]
fn block_locals_are_not_visible_outside() {
    assert_eq!(
        errors("ready = true\nif ready:\n    message = \"go\"\nprint(message)\n"),
        [UndefinedName("message".into())]
    );
}

#[test]
fn undefined_names() {
    assert_eq!(errors("x = y + 1\n"), [UndefinedName("y".into())]);
    assert_eq!(errors("missing()\n"), [UndefinedName("missing".into())]);
}

#[test]
fn use_before_declaration_in_source_order() {
    assert_eq!(errors("print(later)\nlater = 1\n"), [UndefinedName("later".into())]);
}

#[test]
fn builtins_are_in_scope() {
    ok("print(len(\"abc\"))\nx = to_float(1)\ny = to_int(1.5)\n");
}

#[test]
fn builtins_cannot_be_redeclared() {
    assert_eq!(errors("print = 1\n"), [RedeclaresBuiltin("print".into())]);
    assert_eq!(
        errors("function f(len: Int):\n    return len\n"),
        [RedeclaresBuiltin("len".into())]
    );
}

#[test]
fn functions_are_visible_across_the_whole_module() {
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
}

#[test]
fn direct_recursion() {
    ok("function f(n: Int) -> Int:\n    return f(n - 1)\n");
}

#[test]
fn duplicate_declarations() {
    assert!(matches!(
        errors("function f():\n    return 1\nfunction f():\n    return 2\n").as_slice(),
        [DuplicateDeclaration { .. }]
    ));
    assert!(matches!(
        errors("function f(a: Int, a: Int):\n    return a\n").as_slice(),
        [DuplicateDeclaration { .. }]
    ));
}

#[test]
fn a_variable_and_a_function_cannot_share_a_name() {
    assert!(matches!(
        errors("f = 1\nfunction f():\n    return 2\n").as_slice(),
        [DuplicateDeclaration { .. }]
    ));
}

#[test]
fn assignment_to_a_visible_name_reassigns_rather_than_shadows() {
    ok("function f(x: Int):\n    if x > 0:\n        x = 1\n    return x\n");
    ok("\
function f(flag: Bool) -> Int:
    if flag:
        value = 1
        if flag:
            value = 2
        return value
    return 0
");
}

#[test]
fn a_loop_variable_cannot_shadow_an_enclosing_binding() {
    assert!(matches!(
        errors("function f(i: Int) -> Int:\n    for i in [1]:\n        print(i)\n    return i\n")
            .as_slice(),
        [Shadows { .. }]
    ));
}

#[test]
fn a_parameter_may_shadow_a_global() {
    ok("\
name = \"Dovik\"

function greet(name: String):
    print(\"Hello, \" + name)
");
}

#[test]
fn assignment_cannot_turn_a_global_into_a_local() {
    assert_eq!(
        errors("total = 0\n\nfunction f() -> Int:\n    total = 5\n    return total\n"),
        [AssignToGlobal("total".into())]
    );
}

#[test]
fn a_loop_variable_may_shadow_a_global() {
    ok("\
item = 0

function f():
    for item in [1]:
        print(item)
");
}

#[test]
fn a_body_cannot_assign_to_a_function_name() {
    assert_eq!(
        errors("function g():\n    return 1\nfunction f() -> Int:\n    g = 2\n    return g\n"),
        [NotAssignable { name: "g".into(), what: "a function" }]
    );
}

#[test]
fn a_loop_variable_may_not_shadow_a_function() {
    assert!(matches!(
        errors("function g():\n    return 1\nfunction f():\n    for g in [1]:\n        print(g)\n")
            .as_slice(),
        [Shadows { .. }]
    ));
}

#[test]
fn functions_cannot_assign_to_globals() {
    assert_eq!(
        errors("counter = 0\n\nfunction bump():\n    counter = counter + 1\n"),
        [AssignToGlobal("counter".into())]
    );
}

#[test]
fn module_scope_may_assign_to_globals() {
    ok("counter = 0\ncounter = counter + 1\n");
}

#[test]
fn a_function_cannot_read_a_global_declared_after_it() {
    assert_eq!(
        errors("function f() -> Int:\n    return later\n\nlater = 1\n"),
        [UndefinedName("later".into())]
    );
}

#[test]
fn a_module_call_cannot_reach_a_later_global() {
    let source = "\
function early() -> Int:
    return late_reader()

x = early()

limit = 10

function late_reader() -> Int:
    return limit
";
    assert_eq!(
        errors(source),
        [CallReachesLaterGlobal { callee: "late_reader".into(), global: "limit".into() }]
    );
}

#[test]
fn a_module_call_reaching_an_earlier_global_is_fine() {
    ok("\
limit = 10

function reader() -> Int:
    return limit

x = reader()
");
}

#[test]
fn reserved_type_names() {
    assert_eq!(errors("int = 5\n"), [ReservedTypeName("int".into())]);
    assert_eq!(errors("INT = 5\n"), [ReservedTypeName("INT".into())]);
    assert_eq!(errors("String = 5\n"), [ReservedTypeName("String".into())]);
    assert_eq!(
        errors("function string(a: Int):\n    return a\n"),
        [ReservedTypeName("string".into())]
    );
    assert_eq!(
        errors("items: List<Int> = []\nfor Dict in items:\n    print(Dict)\n"),
        [ReservedTypeName("Dict".into())]
    );
}

#[test]
fn break_and_continue_need_a_loop() {
    assert_eq!(errors("break\n"), [BreakOutsideLoop]);
    assert_eq!(errors("continue\n"), [ContinueOutsideLoop]);
    assert_eq!(
        errors("if true:\n    break\n"),
        [BreakOutsideLoop]
    );
    ok("while true:\n    break\n");
    ok("for x in [1]:\n    continue\n");
    ok("while true:\n    if true:\n        break\n");
}

#[test]
fn break_does_not_escape_a_function_into_an_enclosing_loop() {
    assert_eq!(
        errors_unchecked("while true:\n    function f():\n        break\n"),
        [BreakOutsideLoop]
    );
}

#[test]
fn the_loop_variable_cannot_be_assigned() {
    assert_eq!(
        errors("for name in [\"a\"]:\n    name = \"b\"\n"),
        [AssignToLoopVariable("name".into())]
    );
}

#[test]
fn the_loop_variable_is_scoped_to_the_body() {
    assert_eq!(
        errors("for name in [\"a\"]:\n    print(name)\nprint(name)\n"),
        [UndefinedName("name".into())]
    );
}

#[test]
fn the_loop_variable_may_not_shadow_a_local() {
    assert!(matches!(
        errors("function f(i: Int):\n    for i in [1]:\n        print(i)\n    return 0\n")
            .as_slice(),
        [Shadows { .. }]
    ));
}

#[test]
fn a_function_cannot_mutate_a_global_collection() {
    assert_eq!(
        errors("xs: List<Int> = [1]\n\nfunction f():\n    xs[0] = 2\n"),
        [MutateGlobalInFunction("xs".into())]
    );
}

#[test]
fn a_function_may_mutate_a_parameter() {
    ok("function mutate(xs: List<Int>):\n    xs[0] = 99\n");
}

#[test]
fn rebinding_a_parameter_is_allowed() {
    ok("function rebind(xs: List<Int>):\n    xs = [9]\n");
}

#[test]
fn module_scope_may_mutate_a_global() {
    ok("xs: List<Int> = [1]\nxs[0] = 2\n");
}

#[test]
fn assigning_a_function_name_at_module_scope_is_a_duplicate_declaration() {
    assert!(matches!(
        errors("function f():\n    return 1\nf = 2\n").as_slice(),
        [DuplicateDeclaration { .. }]
    ));
}

#[test]
fn symbol_kinds_are_recorded() {
    let source = "\
g = 1

function f(p: Int) -> Int:
    local = p
    for item in [1]:
        print(item)
    return local
";
    let (tokens, _) = lexer::lex(source);
    let (program, _) = parser::parse(source, &tokens);
    let (resolution, errors) = resolve::resolve(&modules::unit_graph("test.dovk", source, program));
    assert!(errors.is_empty(), "{errors:?}");

    let kind_of = |name: &str| {
        resolution
            .symbols
            .iter()
            .find(|s| s.name == name)
            .map(|s| s.kind)
    };

    assert_eq!(kind_of("g"), Some(SymbolKind::Global));
    assert_eq!(kind_of("f"), Some(SymbolKind::Function));
    assert_eq!(kind_of("p"), Some(SymbolKind::Param));
    assert_eq!(kind_of("local"), Some(SymbolKind::Local));
    assert_eq!(kind_of("item"), Some(SymbolKind::LoopVar));
    assert_eq!(kind_of("print"), Some(SymbolKind::Builtin));
}

#[test]
fn name_occurrences_resolve_to_their_declaration() {
    let source = "age = 23\nnext = age + 1\n";
    let (tokens, _) = lexer::lex(source);
    let (program, _) = parser::parse(source, &tokens);
    let (resolution, _) = resolve::resolve(&modules::unit_graph("test.dovk", source, program));

    let read = source.rfind("age").expect("the second `age`");
    let symbol = resolution
        .binding(compiler::token::Span::new(read, read + 3))
        .expect("the read resolves");
    assert_eq!(symbol.span, compiler::token::Span::new(0, 3));
}

#[test]
fn the_spec_example_program_resolves() {
    ok("\
function factorial(n: Int) -> Int:
    if n <= 1:
        return 1
    return n * factorial(n - 1)

function greet(name: String):
    print(\"Hello, \" + name)

names: List<String> = [\"Dawit\", \"Abel\"]

for person in names:
    greet(person)

count = 0
while count < 3:
    count = count + 1

total = factorial(5) + len(names) * 2 - 1
");
}

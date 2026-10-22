use compiler::ast::{
    AssignTarget, BinaryOp, Expr, ExprKind, Program, StmtKind, TypeKind, UnaryOp,
};
use compiler::error::{ParseError, ParseErrorKind::{self, *}};
use compiler::{lexer, parser};

fn parse(source: &str) -> Program {
    let (tokens, lex_errors) = lexer::lex(source);
    assert_eq!(lex_errors, [], "lex errors in {source:?}");
    let (program, errors) = parser::parse(source, &tokens);
    assert_eq!(
        errors.iter().map(|e| e.kind.clone()).collect::<Vec<_>>(),
        Vec::<ParseErrorKind>::new(),
        "parse errors in {source:?}"
    );
    program
}

fn errors(source: &str) -> Vec<ParseErrorKind> {
    let (tokens, _) = lexer::lex(source);
    let (_, errors) = parser::parse(source, &tokens);
    errors.into_iter().map(|e: ParseError| e.kind).collect()
}

fn only(source: &str) -> StmtKind {
    let mut program = parse(source);
    assert_eq!(program.statements.len(), 1, "expected one statement");
    program.statements.remove(0).kind
}

fn expr_of(source: &str) -> Expr {
    match only(&format!("x = {source}\n")) {
        StmtKind::Assign { value, .. } => value,
        other => panic!("expected an assignment, got {other:?}"),
    }
}

fn shape(expr: &Expr) -> String {
    match &expr.kind {
        ExprKind::Int(v) => v.to_string(),
        ExprKind::Float(v) => format!("{v}"),
        ExprKind::Str(v) => format!("{v:?}"),
        ExprKind::Bool(v) => v.to_string(),
        ExprKind::Name(n) => n.clone(),
        ExprKind::Unary { op, operand } => {
            let op = match op {
                UnaryOp::Neg => "-",
                UnaryOp::Not => "not ",
            };
            format!("({op}{})", shape(operand))
        }
        ExprKind::Binary { op, left, right } => {
            let op = match op {
                BinaryOp::Add => "+",
                BinaryOp::Sub => "-",
                BinaryOp::Mul => "*",
                BinaryOp::Div => "/",
                BinaryOp::Rem => "%",
                BinaryOp::Eq => "==",
                BinaryOp::Ne => "!=",
                BinaryOp::Lt => "<",
                BinaryOp::Gt => ">",
                BinaryOp::Le => "<=",
                BinaryOp::Ge => ">=",
                BinaryOp::And => "and",
                BinaryOp::Or => "or",
            };
            format!("({} {op} {})", shape(left), shape(right))
        }
        ExprKind::Call { callee, args } => {
            let args: Vec<_> = args.iter().map(shape).collect();
            format!("{}({})", shape(callee), args.join(", "))
        }
        ExprKind::Index { object, index } => format!("{}[{}]", shape(object), shape(index)),
        ExprKind::List(items) => {
            let items: Vec<_> = items.iter().map(shape).collect();
            format!("[{}]", items.join(", "))
        }
        ExprKind::Dict(entries) => {
            let entries: Vec<_> = entries
                .iter()
                .map(|(k, v)| format!("{}: {}", shape(k), shape(v)))
                .collect();
            format!("{{{}}}", entries.join(", "))
        }
    }
}

fn shape_of(source: &str) -> String {
    shape(&expr_of(source))
}

#[test]
fn declaration_without_annotation() {
    match only("age = 23\n") {
        StmtKind::Assign { target, ty, value } => {
            assert_eq!(target, AssignTarget::Name(name("age", 0, 3)));
            assert_eq!(ty, None);
            assert_eq!(value.kind, ExprKind::Int(23));
        }
        other => panic!("{other:?}"),
    }
}

fn name(text: &str, start: usize, end: usize) -> compiler::ast::Ident {
    compiler::ast::Ident {
        name: text.to_string(),
        span: compiler::token::Span::new(start, end),
    }
}

#[test]
fn annotated_declaration() {
    match only("count: Int = 0\n") {
        StmtKind::Assign { ty, .. } => {
            assert_eq!(ty.map(|t| t.kind), Some(TypeKind::Int))
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn generic_annotations() {
    let cases = [
        ("names: List<String> = []\n", "List<String>"),
        ("ages: Dict<String, Int> = {}\n", "Dict<String, Int>"),
        ("grid: List<List<Int>> = []\n", "List<List<Int>>"),
    ];

    for (source, expected) in cases {
        match only(source) {
            StmtKind::Assign { ty, .. } => {
                assert_eq!(render_type(&ty.expect("annotation").kind), expected)
            }
            other => panic!("{other:?}"),
        }
    }
}

fn render_type(kind: &TypeKind) -> String {
    match kind {
        TypeKind::Int => "Int".into(),
        TypeKind::Float => "Float".into(),
        TypeKind::Bool => "Bool".into(),
        TypeKind::Str => "String".into(),
        TypeKind::Unit => "Unit".into(),
        TypeKind::List(item) => format!("List<{}>", render_type(&item.kind)),
        TypeKind::Dict(key, value) => {
            format!("Dict<{}, {}>", render_type(&key.kind), render_type(&value.kind))
        }
    }
}

#[test]
fn type_names_are_case_insensitive() {
    for source in ["x: int = 1\n", "x: INT = 1\n", "x: Int = 1\n"] {
        match only(source) {
            StmtKind::Assign { ty, .. } => assert_eq!(ty.map(|t| t.kind), Some(TypeKind::Int)),
            other => panic!("{other:?}"),
        }
    }
}

#[test]
fn type_errors() {
    assert_eq!(errors("x: Blob = 1\n"), [UnknownType("Blob".into())]);
    assert_eq!(
        errors("x: List = 1\n"),
        [TypeArity { name: "List", expected: 1, found: 0 }]
    );
    assert_eq!(
        errors("x: Dict<Int> = 1\n"),
        [TypeArity { name: "Dict", expected: 2, found: 1 }]
    );
    assert_eq!(
        errors("x: Int<String> = 1\n"),
        [TypeArity { name: "Int", expected: 0, found: 1 }]
    );
}

#[test]
fn declaration_requires_an_initializer() {
    assert_eq!(
        errors("age: Int\n"),
        [Expected { what: "`=` — every declaration needs an initializer", found: "a line break" }]
    );
}

#[test]
fn precedence_matches_the_spec_table() {
    assert_eq!(shape_of("1 + 2 * 3"), "(1 + (2 * 3))");
    assert_eq!(shape_of("1 * 2 + 3"), "((1 * 2) + 3)");
    assert_eq!(shape_of("1 - 2 - 3"), "((1 - 2) - 3)");
    assert_eq!(shape_of("1 + 2 < 3 + 4"), "((1 + 2) < (3 + 4))");
    assert_eq!(shape_of("a < b == c < d"), "((a < b) == (c < d))");
    assert_eq!(shape_of("a and b or c and d"), "((a and b) or (c and d))");
    assert_eq!(shape_of("not a and b"), "((not a) and b)");
    assert_eq!(shape_of("-a * b"), "((-a) * b)");
    assert_eq!(shape_of("- - a"), "(-(-a))");
    assert_eq!(shape_of("(1 + 2) * 3"), "((1 + 2) * 3)");
}

#[test]
fn comparisons_do_not_chain() {
    assert_eq!(shape_of("a < b < c"), "((a < b) < c)");
}

#[test]
fn postfix_binds_tighter_than_unary() {
    assert_eq!(shape_of("-f(x)"), "(-f(x))");
    assert_eq!(shape_of("xs[0] + 1"), "(xs[0] + 1)");
    assert_eq!(shape_of("f(x)[0]"), "f(x)[0]");
    assert_eq!(shape_of("m[k](a, b)"), "m[k](a, b)");
}

#[test]
fn literals() {
    assert_eq!(shape_of("[1, 2, 3]"), "[1, 2, 3]");
    assert_eq!(shape_of("[]"), "[]");
    assert_eq!(shape_of("{}"), "{}");
    assert_eq!(shape_of("{\"a\": 1, \"b\": 2}"), "{\"a\": 1, \"b\": 2}");
    assert_eq!(shape_of("[[1, 2], [3, 4]]"), "[[1, 2], [3, 4]]");
    assert_eq!(shape_of("true"), "true");
    assert_eq!(shape_of("1.75"), "1.75");
}

#[test]
fn multi_line_literals_ignore_layout() {
    assert_eq!(shape_of("[\n    1,\n    2\n]"), "[1, 2]");
}

#[test]
fn index_assignment() {
    match only("numbers[0] = 10\n") {
        StmtKind::Assign { target: AssignTarget::Index { object, index, .. }, .. } => {
            assert_eq!(shape(&object), "numbers");
            assert_eq!(shape(&index), "0");
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn invalid_assignment_targets() {
    assert_eq!(errors("1 = 2\n"), [InvalidAssignTarget]);
    assert_eq!(errors("f(x) = 2\n"), [InvalidAssignTarget]);
    assert_eq!(errors("a + b = 2\n"), [InvalidAssignTarget]);
    assert_eq!(errors("xs[0]: Int = 1\n"), [AnnotationNeedsPlainName]);
}

#[test]
fn call_statement() {
    match only("print(\"hi\")\n") {
        StmtKind::Expr(expr) => assert_eq!(shape(&expr), "print(\"hi\")"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn if_else_chain() {
    let source = "\
if a:
    x = 1
else if b:
    x = 2
else:
    x = 3
";
    match only(source) {
        StmtKind::If { arms, else_block } => {
            assert_eq!(arms.len(), 2);
            assert_eq!(shape(&arms[0].0), "a");
            assert_eq!(shape(&arms[1].0), "b");
            assert_eq!(arms[0].1.statements.len(), 1);
            assert!(else_block.is_some());
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn if_without_else() {
    match only("if a:\n    x = 1\n") {
        StmtKind::If { arms, else_block } => {
            assert_eq!(arms.len(), 1);
            assert_eq!(else_block, None);
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn while_loop() {
    match only("while count < 5:\n    count = count + 1\n") {
        StmtKind::While { condition, body } => {
            assert_eq!(shape(&condition), "(count < 5)");
            assert_eq!(body.statements.len(), 1);
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn for_loop() {
    match only("for name in names:\n    print(name)\n") {
        StmtKind::For { variable, iterable, body } => {
            assert_eq!(variable.name, "name");
            assert_eq!(shape(&iterable), "names");
            assert_eq!(body.statements.len(), 1);
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn break_and_continue() {
    let program = parse("while a:\n    break\n    continue\n");
    match &program.statements[0].kind {
        StmtKind::While { body, .. } => {
            assert_eq!(body.statements[0].kind, StmtKind::Break);
            assert_eq!(body.statements[1].kind, StmtKind::Continue);
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn function_with_inferred_return_type() {
    match only("function add(a: Int, b: Int):\n    return a + b\n") {
        StmtKind::Function(decl) => {
            assert_eq!(decl.name.name, "add");
            assert_eq!(decl.params.len(), 2);
            assert_eq!(decl.params[0].name.name, "a");
            assert_eq!(decl.params[0].ty.kind, TypeKind::Int);
            assert_eq!(decl.return_type, None);
            assert_eq!(decl.body.statements.len(), 1);
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn function_with_annotated_return_type() {
    match only("function f() -> Bool:\n    return true\n") {
        StmtKind::Function(decl) => {
            assert!(decl.params.is_empty());
            assert_eq!(decl.return_type.map(|t| t.kind), Some(TypeKind::Bool));
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn bare_return() {
    match only("function f():\n    return\n") {
        StmtKind::Function(decl) => {
            assert_eq!(decl.body.statements[0].kind, StmtKind::Return(None));
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn nested_functions_are_rejected() {
    let source = "function outer():\n    function inner():\n        return 1\n";
    assert_eq!(errors(source), [NestedFunction]);
}

#[test]
fn parameters_need_types() {
    assert_eq!(
        errors("function f(a):\n    return a\n"),
        [Expected { what: "`:` — every parameter needs a type", found: "`)`" }]
    );
}

#[test]
fn block_must_be_indented() {
    assert_eq!(
        errors("if a:\nx = 1\n"),
        [Expected { what: "an indented block", found: "an identifier" }]
    );
}

#[test]
fn nested_blocks() {
    let source = "\
function factorial(n: Int) -> Int:
    if n <= 1:
        return 1
    return n * factorial(n - 1)
";
    match only(source) {
        StmtKind::Function(decl) => {
            assert_eq!(decl.body.statements.len(), 2);
            match &decl.body.statements[0].kind {
                StmtKind::If { arms, .. } => assert_eq!(arms[0].1.statements.len(), 1),
                other => panic!("{other:?}"),
            }
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn recovery_reports_one_error_per_bad_line() {
    let source = "a = +\nb = 2\nc = *\nd = 4\n";
    let (tokens, _) = lexer::lex(source);
    let (program, errors) = parser::parse(source, &tokens);
    assert_eq!(errors.len(), 2, "{errors:?}");
    assert_eq!(program.statements.len(), 2);
}

#[test]
fn an_error_does_not_swallow_the_next_line() {
    let source = "1 = 2\ny = [1, 2,]\n";
    assert_eq!(
        errors(source),
        [
            InvalidAssignTarget,
            Expected { what: "an expression", found: "`]`" }
        ]
    );
}

#[test]
fn recovery_keeps_block_structure() {
    let source = "\
if a:
    x = +
    y = 2
z = 3
";
    let (tokens, _) = lexer::lex(source);
    let (program, errors) = parser::parse(source, &tokens);
    assert_eq!(errors.len(), 1);
    assert_eq!(program.statements.len(), 2, "the `if` and `z = 3` both survive");
    match &program.statements[0].kind {
        StmtKind::If { arms, .. } => assert_eq!(arms[0].1.statements.len(), 1),
        other => panic!("{other:?}"),
    }
}

#[test]
fn spans_cover_the_statement() {
    let source = "age = 23\n";
    let program = parse(source);
    let span = program.statements[0].span;
    assert_eq!(&source[span.start..span.end], "age = 23");
}

#[test]
fn trailing_commas_are_rejected() {
    assert!(!errors("x = [1, 2,]\n").is_empty());
    assert!(!errors("function f(a: Int,):\n    return a\n").is_empty());
}

#[test]
fn empty_program() {
    assert_eq!(parse("").statements.len(), 0);
    assert_eq!(parse("# just a comment\n").statements.len(), 0);
}

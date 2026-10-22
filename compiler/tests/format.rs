use compiler::{check, format, interp, lexer, lower, opt, parser, resolve};

fn formatted(source: &str) -> String {
    format::format(source).expect("formatting should succeed")
}

fn runs_to(source: &str) -> Vec<String> {
    let (tokens, lex_errors) = lexer::lex(source);
    assert_eq!(lex_errors, [], "lex errors in {source:?}");
    let (ast, parse_errors) = parser::parse(source, &tokens);
    assert!(parse_errors.is_empty(), "parse errors: {parse_errors:?}");
    let (resolution, resolve_errors) = resolve::resolve(&ast);
    assert!(resolve_errors.is_empty(), "resolve errors: {resolve_errors:?}");
    let (typed, type_errors) = check::check(&ast, &resolution);
    assert!(type_errors.is_empty(), "type errors: {type_errors:?}");
    let mut program = lower::lower(&ast, &resolution, &typed);
    assert!(opt::optimize(&mut program).is_empty());
    let run = interp::run(&program);
    assert_eq!(run.outcome, interp::Outcome::Finished, "{run:?}");
    run.output
}

const CANONICAL: &str = "\
# a leading comment
function add(a: Int, b: Int) -> Int:
    return a + b

names: List<String> = [\"a\", \"b\"]
counts: Dict<String, Int> = {\"k\": 1}
grid: List<List<Int>> = []

for n in names:
    print(n)  # a trailing comment

x = add(1, 2) * 3

if x > 0 and not false:
    print(-x)
else:
    print(x % 2)
";

#[test]
fn canonical_source_is_unchanged() {
    assert_eq!(formatted(CANONICAL), CANONICAL);
}

#[test]
fn formatting_is_idempotent() {
    let messy = "function  f( a:int )->int:\n    return a+1\nx=f( 2 )\n";
    let once = formatted(messy);
    assert_eq!(formatted(&once), once);
}

#[test]
fn type_names_are_normalized_to_their_capitalized_spelling() {
    let source = "a: int = 1\nb: FLOAT = 1.5\nc: list<string> = []\nd: dict<STRING, int> = {}\n";
    assert_eq!(
        formatted(source),
        "a: Int = 1\nb: Float = 1.5\nc: List<String> = []\nd: Dict<String, Int> = {}\n"
    );
}

#[test]
fn operators_get_single_spaces_and_unary_minus_does_not() {
    assert_eq!(formatted("x=1+2*3\n"), "x = 1 + 2 * 3\n");
    assert_eq!(formatted("x = -5\n"), "x = -5\n");
    assert_eq!(formatted("x=0-5\n"), "x = 0 - 5\n");
    assert_eq!(formatted("y = not true\n"), "y = not true\n");
    assert_eq!(formatted("z=a>=b\n"), "z = a >= b\n");
}

#[test]
fn calls_and_indexing_stay_tight() {
    assert_eq!(formatted("x = f( 1 , 2 )\n"), "x = f(1, 2)\n");
    assert_eq!(formatted("y = xs[ 0 ]\n"), "y = xs[0]\n");
    assert_eq!(formatted("z = f( 1 )[ 0 ]\n"), "z = f(1)[0]\n");
}

#[test]
fn comments_keep_their_position() {
    let source = "# own line\nx = 1  # trailing\n# after\n";
    assert_eq!(formatted(source), "# own line\nx = 1  # trailing\n# after\n");
}

#[test]
fn a_comment_gets_one_space_after_the_hash() {
    assert_eq!(formatted("#tight\nx = 1\n"), "# tight\nx = 1\n");
    assert_eq!(formatted("#\nx = 1\n"), "#\nx = 1\n");
}

#[test]
fn runs_of_blank_lines_collapse_to_one() {
    assert_eq!(formatted("x = 1\n\n\n\ny = 2\n"), "x = 1\n\ny = 2\n");
    assert_eq!(formatted("x = 1\ny = 2\n"), "x = 1\ny = 2\n");
}

#[test]
fn indentation_becomes_four_spaces_per_level() {
    let source = "if true:\n    if true:\n        print(1)\n";
    assert_eq!(formatted(source), source);
}

#[test]
fn comments_inside_a_block_are_indented_with_it() {
    let source = "if true:\n    # inside\n    print(1)\n";
    assert_eq!(formatted(source), source);
}

#[test]
fn formatting_cannot_change_behaviour() {
    let programs = [
        "print(1 + 2 * 3)\n",
        "x=7\nprint(x%3)\nprint(-x)\n",
        "function f(n:int)->int:\n    if n<=1:\n        return 1\n    return n*f(n-1)\n\nprint(f(6))\n",
        "names:list<string>=[\"a\",\"b\"]\nfor n in names:\n    print(n)\n",
        "d:dict<string,int>={\"k\":1}\nprint(d[\"k\"])\n",
        "print( \"a\" + \"b\" )\n",
        "print(true and not false)\n",
        "total=0\nn=0\nwhile n<4:\n    n=n+1\n    if n==2:\n        continue\n    total=total+n\nprint(total)\n",
    ];

    for source in programs {
        let before = runs_to(source);
        let after = runs_to(&formatted(source));
        assert_eq!(before, after, "formatting changed behaviour for {source:?}");
    }
}

#[test]
fn a_file_that_does_not_lex_is_refused() {
    assert_eq!(format::format("x = \"unterminated\n"), None);
}

#[test]
fn string_contents_are_never_touched() {
    let source = "x = \"  spaced  #not a comment  \"\n";
    assert_eq!(formatted(source), source);
}

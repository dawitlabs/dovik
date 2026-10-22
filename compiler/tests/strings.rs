use std::process::Command;

use compiler::error::TypeErrorKind::{self, *};
use compiler::ir::RuntimeError;
use compiler::types::Type;
use compiler::source::SourceMap;
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
    let graph = modules::unit_graph("test.dovk", source, ast);
    let (resolution, resolve_errors) = resolve::resolve(&graph);
    assert!(resolve_errors.is_empty(), "resolve errors: {resolve_errors:?}");
    check::check(&graph, &resolution)
        .1
        .into_iter()
        .map(|error| error.kind)
        .collect()
}

#[test]
fn split_and_join_round_trip() {
    let source = "\
parts = split(\"a,b,c\", \",\")
print(len(parts))
print(join(parts, \",\"))
print(join(parts, \" - \"))
";
    assert_eq!(output(source), ["3", "a,b,c", "a - b - c"]);
}

#[test]
fn split_keeps_empty_fields() {
    assert_eq!(output("print(len(split(\"a,,b\", \",\")))\n"), ["3"]);
    assert_eq!(output("print(len(split(\",a\", \",\")))\n"), ["2"]);
    assert_eq!(output("print(len(split(\"a,\", \",\")))\n"), ["2"]);
}

#[test]
fn split_without_a_match_yields_the_whole_string() {
    let source = "parts = split(\"abc\", \",\")\nprint(len(parts))\nprint(parts[0])\n";
    assert_eq!(output(source), ["1", "abc"]);
}

#[test]
fn split_on_an_empty_separator_yields_scalars() {
    let source = "parts = split(\"héllo\", \"\")\nprint(len(parts))\nprint(join(parts, \"-\"))\n";
    assert_eq!(output(source), ["5", "h-é-l-l-o"]);
}

#[test]
fn join_handles_empty_and_single_lists() {
    let source = "\
empty: List<String> = []
print(\"[\" + join(empty, \",\") + \"]\")
one: List<String> = [\"only\"]
print(join(one, \",\"))
";
    assert_eq!(output(source), ["[]", "only"]);
}

#[test]
fn substring_counts_scalars_not_bytes() {
    assert_eq!(output("print(substring(\"héllo wörld\", 6, 5))\n"), ["wörld"]);
    assert_eq!(output("print(substring(\"héllo\", 1, 1))\n"), ["é"]);
    assert_eq!(output("print(\"[\" + substring(\"abc\", 3, 0) + \"]\")\n"), ["[]"]);
    assert_eq!(output("print(substring(\"abc\", 0, 3))\n"), ["abc"]);
}

#[test]
fn substring_out_of_range_traps() {
    assert_eq!(
        trap("n = 9\nprint(substring(\"abc\", 0, n))\n"),
        RuntimeError::IndexOutOfBounds
    );
    assert_eq!(
        trap("n = 5\nprint(substring(\"abc\", n, 1))\n"),
        RuntimeError::IndexOutOfBounds
    );
    assert_eq!(
        trap("n = 0 - 1\nprint(substring(\"abc\", n, 1))\n"),
        RuntimeError::IndexOutOfBounds
    );
}

#[test]
fn find_reports_a_scalar_position() {
    let source = "\
at = find(\"héllo\", \"llo\")

if at != none:
    print(at)
";
    assert_eq!(output(source), ["2"]);
    assert_eq!(output("print(find(\"abc\", \"z\") == none)
"), ["true"]);
}

#[test]
fn find_replaces_the_contains_guard() {
    let source = "\
line = \"key=value\"
at = find(line, \"=\")

if at != none:
    print(substring(line, 0, at))
else:
    print(line)
";
    assert_eq!(output(source), ["key"]);
}

#[test]
fn starts_with_and_ends_with() {
    assert_eq!(output("print(starts_with(\"dovik\", \"do\"))
"), ["true"]);
    assert_eq!(output("print(starts_with(\"dovik\", \"vik\"))
"), ["false"]);
    assert_eq!(output("print(ends_with(\"dovik\", \"vik\"))
"), ["true"]);
    assert_eq!(output("print(ends_with(\"dovik\", \"do\"))
"), ["false"]);
}

#[test]
fn trim_removes_surrounding_whitespace_only() {
    assert_eq!(output("print(\"[\" + trim(\"  a b  \") + \"]\")\n"), ["[a b]"]);
    assert_eq!(output("print(\"[\" + trim(\"\\t\\nx\\r\\n\") + \"]\")\n"), ["[x]"]);
    assert_eq!(output("print(\"[\" + trim(\"   \") + \"]\")\n"), ["[]"]);
}

#[test]
fn case_conversion_handles_ascii() {
    assert_eq!(output("print(upper(\"dovik 42\"))\n"), ["DOVIK 42"]);
    assert_eq!(output("print(lower(\"DOVIK 42\"))\n"), ["dovik 42"]);
}

#[test]
fn string_builtin_types_are_enforced() {
    assert_eq!(
        errors("xs = split(1, \",\")\n"),
        [BuiltinArgument { name: "split", found: Type::Int }]
    );
    assert_eq!(
        errors("print(join(\"a\", \",\"))\n"),
        [BuiltinArgument { name: "join", found: Type::Str }]
    );
    assert_eq!(
        errors("xs: List<Int> = []\nprint(join(xs, \",\"))\n"),
        [BuiltinArgument { name: "join", found: Type::List(Box::new(Type::Int)) }]
    );
    assert_eq!(
        errors("print(substring(\"abc\", \"0\", 1))\n"),
        [ArgumentType {
            name: "substring".into(),
            position: 2,
            expected: Type::Int,
            found: Type::Str
        }]
    );
    assert_eq!(
        errors("print(trim(\"a\", \"b\"))\n"),
        [ArgumentCount { name: "trim".into(), expected: 1, found: 2 }]
    );
}

#[test]
fn split_returns_a_list_of_strings() {
    assert_eq!(
        errors("xs: List<Int> = split(\"a\", \",\")\n"),
        [AnnotationMismatch {
            annotated: Type::List(Box::new(Type::Int)),
            found: Type::List(Box::new(Type::Str))
        }]
    );
}

#[test]
fn native_matches_the_interpreter_for_string_operations() {
    let source = "\
line = \"  the quick  brown fox  \"
print(\"[\" + trim(line) + \"]\")
parts = split(trim(line), \" \")
print(len(parts))
print(join(parts, \"|\"))
print(substring(\"héllo wörld\", 6, 5))
at = find(\"héllo wörld\", \"wörld\")
if at != none:
    print(at)
print(find(\"héllo\", \"éll\") == none)
print(starts_with(\"dovik\", \"dov\"))
print(ends_with(\"dovik\", \"vik\"))
print(upper(\"dovik\"))
print(lower(\"DOVIK\"))
print(join(split(\"héllo\", \"\"), \"-\"))
mixed = \"abc\" + \"é\"
print(len(mixed))
print(mixed[3])
j = 0
walked = \"\"
while j < len(mixed):
    walked = walked + mixed[j]
    j = j + 1
print(walked)
total = 0
for p in parts:
    total = total + len(p)
print(total)
";
    let expected = interp::run(&lowered(source));
    assert_eq!(expected.outcome, interp::Outcome::Finished, "{expected:?}");

    let dir = std::env::temp_dir().join("dovik-string-tests");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let ll = dir.join("prog.ll");
    let binary = dir.join("prog");
    std::fs::write(&ll, backend::emit(&lowered(source), &SourceMap::single("test.dovk", source))).expect("write ir");

    let build = Command::new("clang")
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

    let run = Command::new(&binary)
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

#[test]
fn replace_substitutes_every_occurrence() {
    assert_eq!(output("print(replace(\"a-b-c\", \"-\", \"+\"))\n"), ["a+b+c"]);
    assert_eq!(output("print(replace(\"aaa\", \"aa\", \"b\"))\n"), ["ba"]);
    assert_eq!(output("print(replace(\"abc\", \"z\", \"y\"))\n"), ["abc"]);
    assert_eq!(output("print(\"[\" + replace(\"aaa\", \"a\", \"\") + \"]\")\n"), ["[]"]);
    assert_eq!(output("print(replace(\"héllo\", \"é\", \"e\"))\n"), ["hello"]);
}

#[test]
fn replace_with_an_empty_needle_changes_nothing() {
    assert_eq!(output("print(replace(\"abc\", \"\", \"x\"))\n"), ["abc"]);
}

#[test]
fn assert_passes_quietly_and_fails_loudly() {
    assert_eq!(output("assert(1 == 1)\nprint(\"after\")\n"), ["after"]);
    assert_eq!(
        trap("a = 1\nb = 2\nassert(a == b)\n"),
        RuntimeError::AssertionFailed
    );
}

#[test]
fn assert_requires_a_bool_and_returns_unit() {
    assert_eq!(
        errors("assert(1)\n"),
        [BuiltinArgument { name: "assert", found: Type::Int }]
    );
    assert_eq!(errors("x = assert(true)\n"), [UnitAsValue]);
}

#[test]
fn case_conversion_covers_more_than_ascii() {
    assert_eq!(output("print(upper(\"héllo wörld\"))\n"), ["HÉLLO WÖRLD"]);
    assert_eq!(output("print(lower(\"HÉLLO WÖRLD\"))\n"), ["héllo wörld"]);
    assert_eq!(output("print(upper(\"αβγ\"))\n"), ["ΑΒΓ"]);
}

#[test]
fn case_conversion_may_change_the_scalar_count() {
    let source = "\
word = \"straße\"
print(len(word))
print(upper(word))
print(len(upper(word)))
";
    assert_eq!(output(source), ["6", "STRASSE", "7"]);
}

#[test]
fn case_conversion_is_locale_independent() {
    assert_eq!(output("print(lower(\"I\"))\n"), ["i"]);
    assert_eq!(output("print(upper(\"i\"))\n"), ["I"]);
}

#[test]
fn case_conversion_leaves_uncased_scalars_alone() {
    assert_eq!(output("print(upper(\"漢字 123 !\"))\n"), ["漢字 123 !"]);
    assert_eq!(output("print(lower(\"漢字 123 !\"))\n"), ["漢字 123 !"]);
}

#[test]
fn indexing_is_correct_across_the_ascii_boundary() {
    let source = "\
mixed = \"abc\" + \"é\"
print(len(mixed))
print(mixed[0])
print(mixed[3])

wide = \"héllo wörld\"
print(len(wide))
print(wide[1])
print(wide[6])

plain = \"abc\"
print(len(plain))
print(plain[2])
";
    assert_eq!(
        output(source),
        ["4", "a", "é", "11", "é", "w", "3", "c"]
    );
}

#[test]
fn appending_invalidates_the_ascii_answer() {
    let source = "\
s = \"ab\"
print(len(s))
s = s + \"é\"
print(len(s))
print(s[2])
s = s + \"c\"
print(len(s))
print(s[3])
";
    assert_eq!(output(source), ["2", "3", "é", "4", "c"]);
}

#[test]
fn a_positional_scan_matches_an_iterating_scan() {
    let source = "\
text = \"héllo wörld\"
by_index = \"\"
j = 0

while j < len(text):
    by_index = by_index + text[j]
    j = j + 1

by_loop = \"\"

for c in text:
    by_loop = by_loop + c

print(by_index == by_loop)
print(by_index == text)
";
    assert_eq!(output(source), ["true", "true"]);
}

#[test]
fn indexing_spans_every_scalar_width() {
    let source = "\
mixed = \"aé漢🎉bሀc\"
print(len(mixed))
print(mixed[0])
print(mixed[1])
print(mixed[2])
print(mixed[3])
print(mixed[5])
print(mixed[6])
";
    assert_eq!(
        output(source),
        ["7", "a", "é", "漢", "🎉", "ሀ", "c"]
    );
}

#[test]
fn indexing_crosses_the_index_chunk_boundary() {
    let source = "\
long = \"\"
k = 0

while k < 40:
    long = long + \"ሀ\"
    k = k + 1

print(len(long))
print(long[0] == long[15])
print(long[16] == long[17])
print(long[39] == long[0])
";
    assert_eq!(output(source), ["40", "true", "true", "true"]);
}

#[test]
fn a_positional_walk_rebuilds_a_mixed_width_string() {
    let source = "\
mixed = \"aé漢🎉bሀc\"
j = 0
rebuilt = \"\"

while j < len(mixed):
    rebuilt = rebuilt + mixed[j]
    j = j + 1

print(rebuilt == mixed)
";
    assert_eq!(output(source), ["true"]);
}

#[test]
fn growing_a_non_ascii_string_keeps_indexing_correct() {
    let source = "\
grown = \"ሀ\"
print(len(grown))
grown = grown + \"ለ\"
print(len(grown))
print(grown[1])
grown = grown + \"x\"
print(len(grown))
print(grown[2])
";
    assert_eq!(output(source), ["1", "2", "ለ", "3", "x"]);
}

#[test]
fn indexing_past_the_end_of_a_non_ascii_string_traps() {
    assert_eq!(
        trap("s = \"héllo\"\ni = 5\nprint(s[i])\n"),
        RuntimeError::IndexOutOfBounds
    );
}

#[test]
fn appending_to_a_non_ascii_string_keeps_its_length_right() {
    let source = "\
s = \"ሀ\"
i = 0
seen = 0

while i < 40:
    s = s + \"ለ\"
    seen = seen + len(s)
    i = i + 1

print(len(s))
print(seen)
print(s[0])
print(s[40])
";
    assert_eq!(output(source), ["41", "860", "ሀ", "ለ"]);
}

use compiler::error::LexErrorKind::{self, *};
use compiler::lexer::lex;
use compiler::token::TokenKind::{self, *};

fn kinds(source: &str) -> Vec<TokenKind> {
    lex(source).0.into_iter().map(|t| t.kind).collect()
}

fn errors(source: &str) -> Vec<LexErrorKind> {
    lex(source).1.into_iter().map(|e| e.kind).collect()
}

fn text(source: &str) -> String {
    let (tokens, errs) = lex(source);
    assert_eq!(errs, [], "unexpected errors for {source:?}");
    match &tokens[0].kind {
        Str(value) => value.clone(),
        other => panic!("expected a string token, got {other:?}"),
    }
}

#[test]
fn assignment() {
    assert_eq!(
        kinds("name = \"Dawit\"\n"),
        [Identifier, Eq, Str("Dawit".into()), Newline, Eof]
    );
}

#[test]
fn spans_point_at_source() {
    let source = "name = 1";
    let (tokens, _) = lex(source);
    assert_eq!(tokens[0].span.start..tokens[0].span.end, 0..4);
    assert_eq!(tokens[0].text(source), "name");
    assert_eq!(tokens[2].text(source), "1");
}

#[test]
fn keywords_are_case_sensitive() {
    assert_eq!(
        kinds("if If FUNCTION function not and or in"),
        [If, Identifier, Identifier, Function, Not, And, Or, In, Newline, Eof]
    );
}

#[test]
fn missing_trailing_newline_is_synthesized() {
    assert_eq!(kinds("a = 1"), [Identifier, Eq, Int(1), Newline, Eof]);
}

#[test]
fn empty_source() {
    assert_eq!(kinds(""), [Eof]);
}

#[test]
fn bom_is_ignored() {
    let source = "\u{FEFF}x = 1";
    let (tokens, errs) = lex(source);
    assert_eq!(errs, []);
    assert_eq!(tokens[0].text(source), "x");
}

#[test]
fn crlf_is_a_line_ending() {
    assert_eq!(
        kinds("a = 1\r\nb = 2\r\n"),
        [Identifier, Eq, Int(1), Newline, Identifier, Eq, Int(2), Newline, Eof]
    );
}

#[test]
fn blocks_emit_indent_and_dedent() {
    let source = "if x:\n    y = 1\nelse:\n    z = 2\n";
    assert_eq!(
        kinds(source),
        [
            If, Identifier, Colon, Newline,
            Indent, Identifier, Eq, Int(1), Newline, Dedent,
            Else, Colon, Newline,
            Indent, Identifier, Eq, Int(2), Newline, Dedent,
            Eof
        ]
    );
}

#[test]
fn nested_blocks_dedent_twice() {
    let source = "if a:\n    if b:\n        x = 1\ny = 2\n";
    assert_eq!(
        kinds(source),
        [
            If, Identifier, Colon, Newline,
            Indent, If, Identifier, Colon, Newline,
            Indent, Identifier, Eq, Int(1), Newline,
            Dedent, Dedent,
            Identifier, Eq, Int(2), Newline, Eof
        ]
    );
}

#[test]
fn blank_and_comment_lines_do_not_affect_blocks() {
    let source = "# header\nif x:\n\n    # note\n    y = 1  # trailing\n\n";
    assert_eq!(
        kinds(source),
        [
            If, Identifier, Colon, Newline,
            Indent, Identifier, Eq, Int(1), Newline, Dedent,
            Eof
        ]
    );
    assert_eq!(errors(source), []);
}

#[test]
fn brackets_suppress_layout() {
    let source = "numbers = [\n    1,\n    2\n]\n";
    assert_eq!(
        kinds(source),
        [
            Identifier, Eq, LBracket, Int(1), Comma, Int(2), RBracket, Newline, Eof
        ]
    );
    assert_eq!(errors(source), []);
}

#[test]
fn tabs_in_indentation_are_rejected() {
    assert_eq!(errors("if x:\n\ty = 1\n"), [TabInIndentation]);
}

#[test]
fn indentation_must_be_a_multiple_of_four() {
    assert_eq!(
        errors("if x:\n  y = 1\n"),
        [IndentNotMultipleOfFour(2)]
    );
}

#[test]
fn indentation_may_not_skip_a_level() {
    assert_eq!(
        errors("if x:\n        y = 1\n"),
        [IndentSkippedLevel { found: 8, expected: 4 }]
    );
}

#[test]
fn dedent_must_land_on_an_enclosing_level() {
    assert_eq!(
        errors("if a:\n        x = 1\n    y = 2\n"),
        [
            IndentSkippedLevel { found: 8, expected: 4 },
            InconsistentDedent(4)
        ]
    );
}

#[test]
fn indent_and_dedent_are_balanced() {
    for source in [
        "if x:\n    y = 1\n",
        "if a:\n    if b:\n        x = 1\ny = 2\n",
        "if a:\n        x = 1\n    y = 2\n",
        "if x:\n  y = 1\n",
    ] {
        let kinds = kinds(source);
        let indents = kinds.iter().filter(|k| **k == Indent).count();
        let dedents = kinds.iter().filter(|k| **k == Dedent).count();
        assert_eq!(indents, dedents, "unbalanced layout for {source:?}");
    }
}

#[test]
fn operators() {
    assert_eq!(
        kinds("-> == != <= >= < > = + - * / % , : ( ) [ ] { }"),
        [
            Arrow, EqEq, BangEq, LtEq, GtEq, Lt, Gt, Eq, Plus, Minus, Star, Slash,
            Percent, Comma, Colon, LParen, RParen, LBracket, RBracket, LBrace, RBrace,
            Newline, Eof
        ]
    );
}

#[test]
fn bare_bang_is_an_error() {
    assert_eq!(errors("!x"), [UnexpectedChar('!')]);
}

#[test]
fn unmatched_and_unclosed_brackets() {
    assert_eq!(errors(")"), [UnmatchedCloseBracket(')')]);
    assert_eq!(errors("f(1"), [UnclosedBracket('(')]);
}

#[test]
fn non_ascii_identifiers_are_rejected() {
    let source = "café = 1";
    assert_eq!(errors(source), [NonAsciiIdentifier]);
    assert_eq!(lex(source).0[0].text(source), "caf");
}

#[test]
fn unterminated_string() {
    assert_eq!(errors("\"abc"), [UnterminatedString]);
    assert!(!kinds("\"abc").iter().any(|k| matches!(k, Str(_))));
}

#[test]
fn string_may_not_span_lines() {
    assert_eq!(errors("\"ab\ncd\""), [NewlineInString, UnterminatedString]);
}

#[test]
fn hash_inside_a_string_is_text() {
    assert_eq!(text("\"a # b\""), "a # b");
}

#[test]
fn escapes() {
    assert_eq!(text(r#""\"\\\n\r\t\0""#), "\"\\\n\r\t\0");
}

#[test]
fn unicode_escapes() {
    assert_eq!(text(r#""\u{41}\u{1F600}\u{10FFFF}""#), "A\u{1F600}\u{10FFFF}");
}

#[test]
fn rejected_escapes() {
    assert_eq!(errors(r#""\q""#), [InvalidEscape('q')]);
    assert_eq!(errors("\"\\"), [TrailingBackslash, UnterminatedString]);
    assert_eq!(errors(r#""\u41}""#), [UnicodeEscapeMissingOpenBrace]);
    assert_eq!(errors(r#""\u{}""#), [UnicodeEscapeEmpty]);
    assert_eq!(errors(r#""\u{41""#), [UnicodeEscapeUnterminated]);
    assert_eq!(errors(r#""\u{4G}""#), [UnicodeEscapeInvalidDigit('G')]);
    assert_eq!(errors(r#""\u{1234567}""#), [UnicodeEscapeTooManyDigits]);
    assert_eq!(errors(r#""\u{110000}""#), [UnicodeEscapeOutOfRange(0x110000)]);
    assert_eq!(errors(r#""\u{D800}""#), [UnicodeEscapeOutOfRange(0xD800)]);
}

#[test]
fn integers() {
    assert_eq!(kinds("0"), [Int(0), Newline, Eof]);
    assert_eq!(kinds("23"), [Int(23), Newline, Eof]);
    assert_eq!(kinds("9223372036854775807"), [Int(i64::MAX as u64), Newline, Eof]);
}

#[test]
fn integer_magnitude_allows_the_folded_minimum() {
    assert_eq!(
        kinds("-9223372036854775808"),
        [Minus, Int(9_223_372_036_854_775_808), Newline, Eof]
    );
    assert_eq!(errors("9223372036854775809"), [IntegerTooLarge]);
    assert_eq!(errors("99999999999999999999999"), [IntegerTooLarge]);
}

#[test]
fn leading_zeros_are_rejected() {
    assert_eq!(errors("007"), [LeadingZero]);
    assert_eq!(errors("0"), []);
    assert_eq!(errors("0.5"), []);
    assert_eq!(errors("00.5"), [LeadingZero]);
}

#[test]
fn floats() {
    assert_eq!(kinds("1.75"), [Float(1.75), Newline, Eof]);
    assert_eq!(kinds("0.5"), [Float(0.5), Newline, Eof]);
    assert_eq!(kinds("3.0"), [Float(3.0), Newline, Eof]);
}

#[test]
fn floats_need_digits_on_both_sides() {
    assert_eq!(errors("3."), [FloatMissingFractionDigits]);
    assert_eq!(errors(".5"), [UnexpectedChar('.')]);
}

#[test]
fn float_overflow_is_rejected() {
    let huge = format!("{}.0", "9".repeat(400));
    assert_eq!(errors(&huge), [FloatOverflow]);
}

#[test]
fn errors_do_not_stop_the_scan() {
    let source = "a = ? b = 1\n";
    let (tokens, errs) = lex(source);
    assert_eq!(errs.len(), 1);
    assert_eq!(
        tokens.into_iter().map(|t| t.kind).collect::<Vec<_>>(),
        [Identifier, Eq, Identifier, Eq, Int(1), Newline, Eof]
    );
}

#[test]
fn spec_example_program() {
    let source = "\
function factorial(n: Int) -> Int:
    if n <= 1:
        return 1
    return n * factorial(n - 1)
";
    assert_eq!(errors(source), []);
    let kinds = kinds(source);
    assert_eq!(kinds.iter().filter(|k| **k == Indent).count(), 2);
    assert_eq!(kinds.iter().filter(|k| **k == Dedent).count(), 2);
    assert_eq!(kinds.last(), Some(&Eof));
}

#[test]
fn a_dot_outside_a_number_is_rejected() {
    assert_eq!(errors("xs.append(2)"), [UnexpectedChar('.')]);
    assert_eq!(errors("text.length"), [UnexpectedChar('.')]);
    assert_eq!(errors("module.function()"), [UnexpectedChar('.')]);
    assert_eq!(errors("1.5"), []);
}

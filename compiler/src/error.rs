use std::fmt;

use crate::token::Span;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexError {
    pub kind: LexErrorKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LexErrorKind {
    UnexpectedChar(char),
    NonAsciiIdentifier,
    TabInIndentation,
    IndentNotMultipleOfFour(usize),
    IndentSkippedLevel { found: usize, expected: usize },
    InconsistentDedent(usize),
    UnclosedBracket(char),
    UnmatchedCloseBracket(char),
    UnterminatedString,
    NewlineInString,
    TrailingBackslash,
    InvalidEscape(char),
    UnicodeEscapeMissingOpenBrace,
    UnicodeEscapeUnterminated,
    UnicodeEscapeEmpty,
    UnicodeEscapeTooManyDigits,
    UnicodeEscapeInvalidDigit(char),
    UnicodeEscapeOutOfRange(u32),
    IntegerTooLarge,
    LeadingZero,
    FloatMissingFractionDigits,
    FloatOverflow,
}

impl fmt::Display for LexErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedChar(c) => write!(f, "unexpected character `{c}`"),
            Self::NonAsciiIdentifier => {
                write!(f, "non-ASCII identifiers are not permitted")
            }
            Self::TabInIndentation => {
                write!(f, "tabs are not permitted in indentation; use four spaces per level")
            }
            Self::IndentNotMultipleOfFour(width) => {
                write!(f, "indentation of {width} spaces is not a multiple of four")
            }
            Self::IndentSkippedLevel { found, expected } => {
                write!(f, "indentation jumped to {found} spaces; expected {expected}")
            }
            Self::InconsistentDedent(width) => {
                write!(f, "indentation of {width} spaces matches no enclosing block")
            }
            Self::UnclosedBracket(c) => write!(f, "unclosed `{c}`"),
            Self::UnmatchedCloseBracket(c) => write!(f, "unmatched `{c}`"),
            Self::UnterminatedString => write!(f, "unterminated string literal"),
            Self::NewlineInString => write!(f, "string literal may not span lines"),
            Self::TrailingBackslash => write!(f, "backslash at end of input"),
            Self::InvalidEscape(c) => write!(f, "unknown escape sequence `\\{c}`"),
            Self::UnicodeEscapeMissingOpenBrace => {
                write!(f, "`\\u` must be followed by `{{`")
            }
            Self::UnicodeEscapeUnterminated => write!(f, "unterminated `\\u{{...}}` escape"),
            Self::UnicodeEscapeEmpty => write!(f, "`\\u{{}}` needs at least one hex digit"),
            Self::UnicodeEscapeTooManyDigits => {
                write!(f, "`\\u{{...}}` takes at most six hex digits")
            }
            Self::UnicodeEscapeInvalidDigit(c) => {
                write!(f, "`{c}` is not a hex digit")
            }
            Self::UnicodeEscapeOutOfRange(value) => {
                write!(f, "U+{value:X} is not a Unicode scalar value")
            }
            Self::IntegerTooLarge => {
                write!(f, "integer literal exceeds the range of `Int`")
            }
            Self::LeadingZero => write!(f, "integer literal may not have leading zeros"),
            Self::FloatMissingFractionDigits => {
                write!(f, "float literal needs at least one digit after `.`")
            }
            Self::FloatOverflow => write!(f, "float literal overflows to infinity"),
        }
    }
}

pub fn line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset.min(source.len())];
    let line = before.matches('\n').count() + 1;
    let line_start = before.rfind('\n').map_or(0, |i| i + 1);
    let column = source[line_start..offset.min(source.len())].chars().count() + 1;
    (line, column)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub kind: ParseErrorKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseErrorKind {
    Expected {
        what: &'static str,
        found: &'static str,
    },
    InvalidAssignTarget,
    AnnotationNeedsPlainName,
    UnknownType(String),
    TypeArity {
        name: &'static str,
        expected: usize,
        found: usize,
    },
    NestedFunction,
}

impl fmt::Display for ParseErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Expected { what, found } => write!(f, "expected {what}, found {found}"),
            Self::InvalidAssignTarget => {
                write!(f, "cannot assign to this expression; assign to a name or `xs[i]`")
            }
            Self::AnnotationNeedsPlainName => {
                write!(f, "a type annotation may only follow a plain variable name")
            }
            Self::UnknownType(name) => write!(f, "unknown type `{name}`"),
            Self::TypeArity {
                name,
                expected,
                found,
            } => write!(
                f,
                "`{name}` takes {expected} type parameter(s), found {found}"
            ),
            Self::NestedFunction => {
                write!(f, "functions may only be declared at module level")
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolveError {
    pub kind: ResolveErrorKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolveErrorKind {
    UndefinedName(String),
    DuplicateDeclaration { name: String, previous: Span },
    Shadows { name: String, previous: Span },
    ReservedTypeName(String),
    RedeclaresBuiltin(String),
    AssignToGlobal(String),
    MutateGlobalInFunction(String),
    AssignToLoopVariable(String),
    NotAssignable { name: String, what: &'static str },
    BreakOutsideLoop,
    ContinueOutsideLoop,
    CallReachesLaterGlobal { callee: String, global: String },
}

impl fmt::Display for ResolveErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UndefinedName(name) => write!(f, "`{name}` is not defined here"),
            Self::DuplicateDeclaration { name, .. } => {
                write!(f, "`{name}` is already declared in this scope")
            }
            Self::Shadows { name, .. } => {
                write!(f, "`{name}` is already visible here; shadowing is not permitted")
            }
            Self::ReservedTypeName(name) => {
                write!(f, "`{name}` is a built-in type name and cannot name a value")
            }
            Self::RedeclaresBuiltin(name) => {
                write!(f, "`{name}` is a built-in and cannot be redeclared")
            }
            Self::AssignToGlobal(name) => {
                write!(f, "a function cannot assign to the module-scope variable `{name}`")
            }
            Self::MutateGlobalInFunction(name) => write!(
                f,
                "a function cannot mutate the module-scope variable `{name}`; pass it as a parameter"
            ),
            Self::AssignToLoopVariable(name) => {
                write!(f, "the loop variable `{name}` cannot be assigned inside the loop")
            }
            Self::NotAssignable { name, what } => {
                write!(f, "`{name}` names {what} and cannot be assigned to")
            }
            Self::BreakOutsideLoop => write!(f, "`break` outside a loop"),
            Self::ContinueOutsideLoop => write!(f, "`continue` outside a loop"),
            Self::CallReachesLaterGlobal { callee, global } => write!(
                f,
                "this call reaches `{callee}`, which reads `{global}` before its initializer runs"
            ),
        }
    }
}

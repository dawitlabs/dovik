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

impl LexErrorKind {
    pub fn code(&self) -> &'static str {
        match self {
            Self::UnexpectedChar(_) => "DVK1001",
            Self::NonAsciiIdentifier => "DVK1002",
            Self::TabInIndentation => "DVK1003",
            Self::IndentNotMultipleOfFour(_) => "DVK1004",
            Self::IndentSkippedLevel { .. } => "DVK1005",
            Self::InconsistentDedent(_) => "DVK1006",
            Self::UnclosedBracket(_) => "DVK1007",
            Self::UnmatchedCloseBracket(_) => "DVK1008",
            Self::UnterminatedString => "DVK1009",
            Self::NewlineInString => "DVK1010",
            Self::TrailingBackslash => "DVK1011",
            Self::InvalidEscape(_) => "DVK1012",
            Self::UnicodeEscapeMissingOpenBrace => "DVK1013",
            Self::UnicodeEscapeUnterminated => "DVK1014",
            Self::UnicodeEscapeEmpty => "DVK1015",
            Self::UnicodeEscapeTooManyDigits => "DVK1016",
            Self::UnicodeEscapeInvalidDigit(_) => "DVK1017",
            Self::UnicodeEscapeOutOfRange(_) => "DVK1018",
            Self::IntegerTooLarge => "DVK1019",
            Self::LeadingZero => "DVK1020",
            Self::FloatMissingFractionDigits => "DVK1021",
            Self::FloatOverflow => "DVK1022",
        }
    }
}

use std::fmt;

use crate::token::Span;

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
    ImportNotFirst,
    EmptyImportList,
    AbsoluteImportPath,
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
            Self::ImportNotFirst => write!(
                f,
                "an `import` must come before every other statement in the file"
            ),
            Self::EmptyImportList => {
                write!(f, "an import list must name at least one name")
            }
            Self::AbsoluteImportPath => {
                write!(f, "an import path must be relative to this file")
            }
        }
    }
}

impl ParseErrorKind {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Expected { .. } => "DVK2001",
            Self::InvalidAssignTarget => "DVK2002",
            Self::AnnotationNeedsPlainName => "DVK2003",
            Self::UnknownType(_) => "DVK2004",
            Self::TypeArity { .. } => "DVK2005",
            Self::NestedFunction => "DVK2006",
            Self::ImportNotFirst => "DVK2007",
            Self::EmptyImportList => "DVK2008",
            Self::AbsoluteImportPath => "DVK2009",
        }
    }
}

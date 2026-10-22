use std::fmt;

use crate::token::Span;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleError {
    pub kind: ModuleErrorKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModuleErrorKind {
    Unreadable { path: String, reason: String },
    Cycle(Vec<String>),
}

impl fmt::Display for ModuleErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unreadable { path, reason } => {
                write!(f, "cannot import `{path}`: {reason}")
            }
            Self::Cycle(modules) => write!(
                f,
                "this import forms a cycle: {}. A module must be fully initialized before anything that imports it",
                modules.join(" -> ")
            ),
        }
    }
}

impl ModuleErrorKind {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Unreadable { .. } => "DVK6001",
            Self::Cycle(_) => "DVK6002",
        }
    }
}

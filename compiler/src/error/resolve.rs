use std::fmt;

use crate::token::Span;

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
    NoSuchExport { name: String, module: String },
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
            Self::NoSuchExport { name, module } => {
                write!(f, "`{module}` does not declare `{name}`")
            }
            Self::CallReachesLaterGlobal { callee, global } => write!(
                f,
                "this call reaches `{callee}`, which reads `{global}` before its initializer runs"
            ),
        }
    }
}

impl ResolveErrorKind {
    pub fn code(&self) -> &'static str {
        match self {
            Self::UndefinedName(_) => "DVK3001",
            Self::DuplicateDeclaration { .. } => "DVK3002",
            Self::Shadows { .. } => "DVK3003",
            Self::ReservedTypeName(_) => "DVK3004",
            Self::RedeclaresBuiltin(_) => "DVK3005",
            Self::AssignToGlobal(_) => "DVK3006",
            Self::MutateGlobalInFunction(_) => "DVK3007",
            Self::AssignToLoopVariable(_) => "DVK3008",
            Self::NotAssignable { .. } => "DVK3009",
            Self::BreakOutsideLoop => "DVK3010",
            Self::ContinueOutsideLoop => "DVK3011",
            Self::NoSuchExport { .. } => "DVK3013",
            Self::CallReachesLaterGlobal { .. } => "DVK3012",
        }
    }
}

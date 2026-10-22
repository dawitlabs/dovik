mod check;
mod lex;
mod module;
mod parse;
mod resolve;
mod runtime;

pub use check::{TypeError, TypeErrorKind};
pub use lex::{LexError, LexErrorKind};
pub use module::{ModuleError, ModuleErrorKind};
pub use parse::{ParseError, ParseErrorKind};
pub use resolve::{ResolveError, ResolveErrorKind};
pub use runtime::Trap;

pub use crate::source::line_col;

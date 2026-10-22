mod check;
mod lex;
mod parse;
mod resolve;

pub use check::{TypeError, TypeErrorKind};
pub use lex::{LexError, LexErrorKind};
pub use parse::{ParseError, ParseErrorKind};
pub use resolve::{ResolveError, ResolveErrorKind};

pub fn line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset.min(source.len())];
    let line = before.matches('\n').count() + 1;
    let line_start = before.rfind('\n').map_or(0, |i| i + 1);
    let column = source[line_start..offset.min(source.len())].chars().count() + 1;
    (line, column)
}

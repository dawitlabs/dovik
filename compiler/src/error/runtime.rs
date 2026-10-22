use std::fmt;

use crate::ir::RuntimeError;
use crate::token::Span;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trap {
    pub error: RuntimeError,
    pub span: Span,
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IntegerOverflow => write!(f, "integer overflow"),
            Self::DivisionByZero => write!(f, "division by zero"),
            Self::RemainderByZero => write!(f, "remainder by zero"),
            Self::InvalidConversion => {
                write!(f, "`to_int` cannot convert this value: NaN, infinity, or out of range")
            }
            Self::IndexOutOfBounds => write!(f, "index out of bounds"),
            Self::MissingKey => write!(f, "dictionary key not found"),
        }
    }
}

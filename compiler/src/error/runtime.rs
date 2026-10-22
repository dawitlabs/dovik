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
            Self::InvalidConversion => write!(
                f,
                "invalid conversion: not a number, not finite, or outside the range of `Int`"
            ),
            Self::IndexOutOfBounds => write!(f, "index out of bounds"),
            Self::MissingKey => write!(f, "dictionary key not found"),
            Self::InputOutput => write!(f, "could not read or write that file"),
            Self::AssertionFailed => write!(f, "assertion failed"),
            Self::StackExhausted => write!(f, "recursion too deep: the call stack is exhausted"),
            Self::OutOfMemory => write!(f, "out of memory"),
        }
    }
}

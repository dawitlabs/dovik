use std::fmt;

use crate::token::Span;
use crate::types::Type;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeError {
    pub kind: TypeErrorKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeErrorKind {
    AnnotationMismatch { annotated: Type, found: Type },
    ReassignMismatch { name: String, expected: Type, found: Type },
    BinaryOperands { op: &'static str, left: Type, right: Type },
    UnaryOperand { op: &'static str, found: Type },
    ConditionNotBool(Type),
    NotCallable(Type),
    ArgumentCount { name: String, expected: usize, found: usize },
    ArgumentType { name: String, position: usize, expected: Type, found: Type },
    BuiltinArgument { name: &'static str, found: Type },
    NotIndexable(Type),
    IndexType { expected: Type, found: Type },
    StringsAreImmutable,
    NotIterable(Type),
    MixedListElements { first: Type, found: Type },
    MixedDictKeys { first: Type, found: Type },
    MixedDictValues { first: Type, found: Type },
    InvalidDictKeyType(Type),
    CannotInferEmptyList,
    CannotInferEmptyDict,
    UnitAsValue,
    UnitAnnotation,
    FunctionAsValue(String),
    BuiltinAsValue(String),
    IntegerLiteralOutOfRange,
    ReturnTypeMismatch { expected: Type, found: Type },
    ReturnOutsideFunction,
    MissingReturn { name: String, declared: Type },
    CannotInferReturnType(String),
    FoldedCheckFailed(crate::ir::RuntimeError),
}

impl fmt::Display for TypeErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AnnotationMismatch { annotated, found } => write!(
                f,
                "annotation says `{annotated}` but the initializer is `{found}`; Dovik never converts implicitly"
            ),
            Self::ReassignMismatch { name, expected, found } => write!(
                f,
                "`{name}` has type `{expected}` and cannot be assigned a `{found}`"
            ),
            Self::BinaryOperands { op, left, right } => {
                write!(f, "`{op}` cannot be applied to `{left}` and `{right}`")
            }
            Self::UnaryOperand { op, found } => {
                write!(f, "`{op}` cannot be applied to `{found}`")
            }
            Self::ConditionNotBool(found) => {
                write!(f, "a condition must be `Bool`, found `{found}`")
            }
            Self::NotCallable(found) => write!(f, "`{found}` is not callable"),
            Self::ArgumentCount { name, expected, found } => write!(
                f,
                "`{name}` takes {expected} argument(s), found {found}"
            ),
            Self::ArgumentType { name, position, expected, found } => write!(
                f,
                "argument {position} of `{name}` expects `{expected}`, found `{found}`"
            ),
            Self::BuiltinArgument { name, found } => {
                write!(f, "`{name}` does not accept `{found}`")
            }
            Self::NotIndexable(found) => write!(f, "`{found}` cannot be indexed"),
            Self::IndexType { expected, found } => {
                write!(f, "this index must be `{expected}`, found `{found}`")
            }
            Self::StringsAreImmutable => {
                write!(f, "strings are immutable; `s[i] = v` is not permitted")
            }
            Self::NotIterable(found) => write!(
                f,
                "`{found}` is not iterable; `for` takes a `List` or a `String`"
            ),
            Self::MixedListElements { first, found } => write!(
                f,
                "list elements must share one type: started with `{first}`, found `{found}`"
            ),
            Self::MixedDictKeys { first, found } => write!(
                f,
                "dictionary keys must share one type: started with `{first}`, found `{found}`"
            ),
            Self::MixedDictValues { first, found } => write!(
                f,
                "dictionary values must share one type: started with `{first}`, found `{found}`"
            ),
            Self::InvalidDictKeyType(found) => write!(
                f,
                "`{found}` cannot be a dictionary key; keys are `Int`, `Bool`, or `String`"
            ),
            Self::CannotInferEmptyList => write!(
                f,
                "cannot infer the element type of `[]`; add an annotation, as in `List<Int>`"
            ),
            Self::CannotInferEmptyDict => write!(
                f,
                "cannot infer the types of `{{}}`; add an annotation, as in `Dict<String, Int>`"
            ),
            Self::UnitAsValue => write!(
                f,
                "this call returns `Unit`, which cannot be stored, passed, or compared"
            ),
            Self::UnitAnnotation => {
                write!(f, "`Unit` is a return type only and cannot annotate a value")
            }
            Self::FunctionAsValue(name) => write!(
                f,
                "`{name}` is a function; Dovik has no function values, so it can only be called"
            ),
            Self::BuiltinAsValue(name) => write!(
                f,
                "`{name}` is a built-in function and can only be called"
            ),
            Self::IntegerLiteralOutOfRange => write!(
                f,
                "integer literal is outside the range of `Int`; only `-9223372036854775808` reaches the minimum"
            ),
            Self::ReturnTypeMismatch { expected, found } => write!(
                f,
                "every `return` in a function must produce the same type: expected `{expected}`, found `{found}`"
            ),
            Self::ReturnOutsideFunction => write!(f, "`return` outside a function"),
            Self::MissingReturn { name, declared } => write!(
                f,
                "`{name}` returns `{declared}`, so every path must return a value"
            ),
            Self::CannotInferReturnType(name) => write!(
                f,
                "cannot infer the return type of `{name}`; annotate it with `->`"
            ),
            Self::FoldedCheckFailed(error) => write!(
                f,
                "this expression always fails at runtime: {error}"
            ),
        }
    }
}

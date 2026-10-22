use std::fmt;

use crate::ast::{TypeKind, TypeNode};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Int,
    Float,
    Bool,
    Str,
    Unit,
    List(Box<Type>),
    Dict(Box<Type>, Box<Type>),
    /// A type that could not be determined. It compares equal to everything, so one real
    /// error does not produce a second one everywhere the bad value flows.
    Unknown,
}

impl Type {
    pub fn from_node(node: &TypeNode) -> Self {
        match &node.kind {
            TypeKind::Int => Self::Int,
            TypeKind::Float => Self::Float,
            TypeKind::Bool => Self::Bool,
            TypeKind::Str => Self::Str,
            TypeKind::Unit => Self::Unit,
            TypeKind::List(item) => Self::List(Box::new(Self::from_node(item))),
            TypeKind::Dict(key, value) => Self::Dict(
                Box::new(Self::from_node(key)),
                Box::new(Self::from_node(value)),
            ),
        }
    }

    pub fn is_unknown(&self) -> bool {
        matches!(self, Self::Unknown)
    }

    pub fn is_numeric(&self) -> bool {
        matches!(self, Self::Int | Self::Float)
    }

    /// Spec 12.2: `Int`, `Bool`, and `String` are the only permitted key types.
    pub fn is_valid_dict_key(&self) -> bool {
        matches!(self, Self::Int | Self::Bool | Self::Str | Self::Unknown)
    }

    /// Spec 13: `==` and `!=` apply to every type but `Unit`.
    pub fn supports_equality(&self) -> bool {
        !matches!(self, Self::Unit)
    }

    /// Spec 6.4: `<` `>` `<=` `>=` apply to `Int`, `Float`, and `String`.
    pub fn supports_ordering(&self) -> bool {
        matches!(self, Self::Int | Self::Float | Self::Str | Self::Unknown)
    }

    /// Whether a value of this type is acceptable where `other` is required. `Unknown`
    /// matches anything, in either position and at any depth.
    pub fn matches(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Unknown, _) | (_, Self::Unknown) => true,
            (Self::List(a), Self::List(b)) => a.matches(b),
            (Self::Dict(ak, av), Self::Dict(bk, bv)) => ak.matches(bk) && av.matches(bv),
            _ => self == other,
        }
    }

    /// Whether this type mentions `Unit` anywhere. `Unit` is a return type only (5.3).
    pub fn mentions_unit(&self) -> bool {
        match self {
            Self::Unit => true,
            Self::List(item) => item.mentions_unit(),
            Self::Dict(key, value) => key.mentions_unit() || value.mentions_unit(),
            _ => false,
        }
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Int => write!(f, "Int"),
            Self::Float => write!(f, "Float"),
            Self::Bool => write!(f, "Bool"),
            Self::Str => write!(f, "String"),
            Self::Unit => write!(f, "Unit"),
            Self::List(item) => write!(f, "List<{item}>"),
            Self::Dict(key, value) => write!(f, "Dict<{key}, {value}>"),
            Self::Unknown => write!(f, "<unknown>"),
        }
    }
}

use crate::types::Type;

pub const NAMES: [&str; 14] = [
    "print",
    "len",
    "to_int",
    "to_float",
    "append",
    "insert",
    "remove_at",
    "has_key",
    "remove_key",
    "to_string",
    "parse_int",
    "parse_float",
    "can_parse_int",
    "can_parse_float",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Builtin {
    Print,
    Len,
    ToInt,
    ToFloat,
    Append,
    Insert,
    RemoveAt,
    HasKey,
    RemoveKey,
    ToString,
    ParseInt,
    ParseFloat,
    CanParseInt,
    CanParseFloat,
}

impl Builtin {
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "print" => Self::Print,
            "len" => Self::Len,
            "to_int" => Self::ToInt,
            "to_float" => Self::ToFloat,
            "append" => Self::Append,
            "insert" => Self::Insert,
            "remove_at" => Self::RemoveAt,
            "has_key" => Self::HasKey,
            "remove_key" => Self::RemoveKey,
            "to_string" => Self::ToString,
            "parse_int" => Self::ParseInt,
            "parse_float" => Self::ParseFloat,
            "can_parse_int" => Self::CanParseInt,
            "can_parse_float" => Self::CanParseFloat,
            _ => return None,
        })
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Print => "print",
            Self::Len => "len",
            Self::ToInt => "to_int",
            Self::ToFloat => "to_float",
            Self::Append => "append",
            Self::Insert => "insert",
            Self::RemoveAt => "remove_at",
            Self::HasKey => "has_key",
            Self::RemoveKey => "remove_key",
            Self::ToString => "to_string",
            Self::ParseInt => "parse_int",
            Self::ParseFloat => "parse_float",
            Self::CanParseInt => "can_parse_int",
            Self::CanParseFloat => "can_parse_float",
        }
    }

    pub fn arity(self) -> usize {
        match self {
            Self::Insert => 3,
            Self::Append | Self::RemoveAt | Self::HasKey | Self::RemoveKey => 2,
            _ => 1,
        }
    }

    pub fn mutates_first_argument(self) -> bool {
        matches!(self, Self::Append | Self::Insert | Self::RemoveAt | Self::RemoveKey)
    }

    pub fn result(self, first: &Type) -> Type {
        match self {
            Self::Print | Self::Append | Self::Insert => Type::Unit,
            Self::Len | Self::ToInt | Self::ParseInt => Type::Int,
            Self::ToFloat | Self::ParseFloat => Type::Float,
            Self::ToString => Type::Str,
            Self::HasKey | Self::RemoveKey | Self::CanParseInt | Self::CanParseFloat => Type::Bool,
            Self::RemoveAt => match first {
                Type::List(item) => (**item).clone(),
                _ => Type::Unknown,
            },
        }
    }
}

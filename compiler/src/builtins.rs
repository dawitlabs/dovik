use crate::types::Type;

pub const NAMES: [&str; 40] = [
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
    "read_file",
    "write_file",
    "append_file",
    "file_exists",
    "argument_count",
    "argument",
    "split",
    "join",
    "substring",
    "find",
    "trim",
    "starts_with",
    "ends_with",
    "upper",
    "lower",
    "replace",
    "read_input",
    "assert",
    "now",
    "clock",
    "sort",
    "list_dir",
    "is_dir",
    "create_dir",
    "remove_dir",
    "remove_file",
    "some",
    "get",
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
    ReadFile,
    WriteFile,
    AppendFile,
    FileExists,
    ArgumentCount,
    Argument,
    Split,
    Join,
    Substring,
    Find,
    Trim,
    StartsWith,
    EndsWith,
    Upper,
    Lower,
    Replace,
    ReadInput,
    Assert,
    Now,
    Clock,
    Sort,
    ListDir,
    IsDir,
    CreateDir,
    RemoveDir,
    RemoveFile,
    Some,
    Get,
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
            "read_file" => Self::ReadFile,
            "write_file" => Self::WriteFile,
            "append_file" => Self::AppendFile,
            "file_exists" => Self::FileExists,
            "argument_count" => Self::ArgumentCount,
            "argument" => Self::Argument,
            "split" => Self::Split,
            "join" => Self::Join,
            "substring" => Self::Substring,
            "find" => Self::Find,
            "trim" => Self::Trim,
            "starts_with" => Self::StartsWith,
            "ends_with" => Self::EndsWith,
            "upper" => Self::Upper,
            "lower" => Self::Lower,
            "replace" => Self::Replace,
            "read_input" => Self::ReadInput,
            "assert" => Self::Assert,
            "now" => Self::Now,
            "clock" => Self::Clock,
            "sort" => Self::Sort,
            "list_dir" => Self::ListDir,
            "is_dir" => Self::IsDir,
            "create_dir" => Self::CreateDir,
            "remove_dir" => Self::RemoveDir,
            "remove_file" => Self::RemoveFile,
            "some" => Self::Some,
            "get" => Self::Get,
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
            Self::ReadFile => "read_file",
            Self::WriteFile => "write_file",
            Self::AppendFile => "append_file",
            Self::FileExists => "file_exists",
            Self::ArgumentCount => "argument_count",
            Self::Argument => "argument",
            Self::Split => "split",
            Self::Join => "join",
            Self::Substring => "substring",
            Self::Find => "find",
            Self::Trim => "trim",
            Self::StartsWith => "starts_with",
            Self::EndsWith => "ends_with",
            Self::Upper => "upper",
            Self::Lower => "lower",
            Self::Replace => "replace",
            Self::ReadInput => "read_input",
            Self::Assert => "assert",
            Self::Now => "now",
            Self::Clock => "clock",
            Self::Sort => "sort",
            Self::ListDir => "list_dir",
            Self::IsDir => "is_dir",
            Self::CreateDir => "create_dir",
            Self::RemoveDir => "remove_dir",
            Self::RemoveFile => "remove_file",
            Self::Some => "some",
            Self::Get => "get",
        }
    }

    pub fn arity(self) -> usize {
        match self {
            Self::Insert | Self::Substring | Self::Replace => 3,
            Self::Append
            | Self::RemoveAt
            | Self::HasKey
            | Self::RemoveKey
            | Self::WriteFile
            | Self::AppendFile
            | Self::Split
            | Self::Join
            | Self::Find
            | Self::StartsWith
            | Self::EndsWith
            | Self::Get => 2,
            Self::ArgumentCount | Self::ReadInput | Self::Now | Self::Clock => 0,
            _ => 1,
        }
    }

    pub fn result(self, first: &Type) -> Type {
        match self {
            Self::Print
            | Self::Append
            | Self::Insert
            | Self::WriteFile
            | Self::AppendFile
            | Self::Assert
            | Self::Sort
            | Self::CreateDir
            | Self::RemoveDir
            | Self::RemoveFile => Type::Unit,
            Self::Len | Self::ToInt | Self::ArgumentCount => Type::Int,
            Self::ToFloat | Self::Now | Self::Clock => Type::Float,
            Self::ParseInt | Self::Find => Type::Optional(Box::new(Type::Int)),
            Self::ParseFloat => Type::Optional(Box::new(Type::Float)),
            Self::ReadFile => Type::Optional(Box::new(Type::Str)),
            Self::ToString
            | Self::Argument
            | Self::Join
            | Self::Substring
            | Self::Trim
            | Self::Upper
            | Self::Lower
            | Self::Replace
            | Self::ReadInput => Type::Str,
            Self::Split | Self::ListDir => Type::List(Box::new(Type::Str)),
            Self::HasKey
            | Self::RemoveKey
            | Self::FileExists
            | Self::StartsWith
            | Self::EndsWith
            | Self::IsDir => Type::Bool,
            Self::RemoveAt => match first {
                Type::List(item) => (**item).clone(),
                _ => Type::Unknown,
            },
            Self::Some => match first {
                Type::Unknown | Type::Unit | Type::Optional(_) => Type::Unknown,
                other => Type::Optional(Box::new(other.clone())),
            },
            Self::Get => match first {
                Type::Dict(_, value) => Type::Optional(value.clone()),
                _ => Type::Unknown,
            },
        }
    }
}

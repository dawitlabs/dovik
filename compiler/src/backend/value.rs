use crate::ir::Value;
use crate::types::Type;

pub(crate) const TAG_INT: u8 = 0;
pub(crate) const TAG_FLOAT: u8 = 1;
pub(crate) const TAG_BOOL: u8 = 2;
pub(crate) const TAG_STR: u8 = 3;
pub(crate) const TAG_LIST: u8 = 4;
pub(crate) const TAG_DICT: u8 = 5;
pub(crate) const TAG_CELL: u8 = 6;

pub(crate) fn llvm_type(ty: &Type) -> &'static str {
    match ty {
        Type::Float => "double",
        Type::Bool => "i1",
        Type::Unit => "void",
        _ => "i64",
    }
}

pub(crate) fn tag_of(ty: &Type) -> u8 {
    match ty {
        Type::Float => TAG_FLOAT,
        Type::Bool => TAG_BOOL,
        Type::Str => TAG_STR,
        Type::List(_) => TAG_LIST,
        Type::Dict(..) => TAG_DICT,
        Type::Optional(inner) => match inner.as_ref() {
            Type::Str => TAG_STR,
            Type::List(_) => TAG_LIST,
            Type::Dict(..) => TAG_DICT,
            _ => TAG_CELL,
        },
        _ => TAG_INT,
    }
}

pub(crate) fn literal(value: &Value) -> String {
    match value {
        Value::Reg(id) => format!("%r{id}"),
        Value::Int(inner) => inner.to_string(),
        Value::Bool(inner) => match inner {
            true => "1".to_string(),
            false => "0".to_string(),
        },
        Value::Float(inner) => format!("0x{:016X}", inner.to_bits()),
        Value::None => "0".to_string(),
        Value::Str(id) => format!("@str.{id}"),
    }
}

pub(crate) fn escape(text: &str) -> (String, usize) {
    let bytes = text.as_bytes();
    let mut encoded = String::with_capacity(bytes.len());

    for byte in bytes {
        match byte {
            b' '..=b'~' if *byte != b'"' && *byte != b'\\' => encoded.push(*byte as char),
            _ => encoded.push_str(&format!("\\{byte:02X}")),
        }
    }

    (encoded, bytes.len())
}

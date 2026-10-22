use super::Value;
use crate::ir::RuntimeError;

pub fn concat(lhs: &Value, rhs: &Value) -> Value {
    let mut joined = String::with_capacity(lhs.as_str().len() + rhs.as_str().len());
    joined.push_str(lhs.as_str());
    joined.push_str(rhs.as_str());
    Value::str(joined)
}

pub fn scalar_count(text: &str) -> i64 {
    text.chars().count() as i64
}

pub fn scalar_at(text: &str, index: i64) -> Result<Value, RuntimeError> {
    if index < 0 {
        return Err(RuntimeError::IndexOutOfBounds);
    }

    text.chars()
        .nth(index as usize)
        .map(|scalar| Value::str(scalar.to_string()))
        .ok_or(RuntimeError::IndexOutOfBounds)
}

pub fn byte_len(text: &str) -> i64 {
    text.len() as i64
}

pub fn next_scalar(text: &str, offset: i64) -> (Value, i64) {
    let start = offset.max(0) as usize;

    if start >= text.len() {
        return (Value::str(""), text.len() as i64);
    }

    match text[start..].chars().next() {
        Some(scalar) => (
            Value::str(scalar.to_string()),
            (start + scalar.len_utf8()) as i64,
        ),
        None => (Value::str(""), text.len() as i64),
    }
}

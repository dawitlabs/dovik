use super::{DictRef, ListRef, Value};
use crate::ir::RuntimeError;

pub fn list_get(list: &ListRef, index: i64) -> Result<Value, RuntimeError> {
    let items = list.borrow();

    if index < 0 || index as usize >= items.len() {
        return Err(RuntimeError::IndexOutOfBounds);
    }

    Ok(items[index as usize].clone())
}

pub fn list_set(list: &ListRef, index: i64, value: Value) -> Result<(), RuntimeError> {
    let mut items = list.borrow_mut();

    if index < 0 || index as usize >= items.len() {
        return Err(RuntimeError::IndexOutOfBounds);
    }

    items[index as usize] = value;
    Ok(())
}

pub fn dict_get(dict: &DictRef, key: &Value) -> Result<Value, RuntimeError> {
    dict.borrow()
        .get(&key.as_key())
        .cloned()
        .ok_or(RuntimeError::MissingKey)
}

pub fn dict_set(dict: &DictRef, key: &Value, value: Value) {
    dict.borrow_mut().insert(key.as_key(), value);
}

pub fn len(value: &Value) -> i64 {
    match value {
        Value::Str(text) => super::text::scalar_count(text),
        Value::List(items) => items.borrow().len() as i64,
        Value::Dict(entries) => entries.borrow().len() as i64,
        _ => 0,
    }
}

use super::{Fault, Frame, Interpreter};
use crate::builtins::Builtin;
use crate::error::Trap;
use crate::ir::{RuntimeError, Value as IrValue};
use crate::runtime::{self, Value};
use crate::token::Span;
use crate::types::Type;

impl Interpreter<'_> {
    pub(crate) fn run_builtin(
        &mut self,
        which: Builtin,
        args: &[(IrValue, Type)],
        frame: &Frame,
        span: Span,
    ) -> Result<Value, Fault> {
        let values: Vec<Value> = args
            .iter()
            .map(|(value, _)| self.value(value, frame))
            .collect();

        if which == Builtin::ArgumentCount {
            return Ok(Value::Int(self.arguments.len() as i64));
        }

        let result = match which {
            Builtin::Append => append(&values),
            Builtin::Insert => insert(&values),
            Builtin::RemoveAt => remove_at(&values),
            Builtin::HasKey => has_key(&values),
            Builtin::RemoveKey => remove_key(&values),
            Builtin::ToString => Ok(Value::str(rendered(&values[0], &args[0].1))),
            Builtin::ParseInt => parse_int(&values[0]),
            Builtin::ParseFloat => parse_float(&values[0]),
            Builtin::CanParseInt => Ok(Value::Bool(trimmed(&values[0]).parse::<i64>().is_ok())),
            Builtin::CanParseFloat => Ok(Value::Bool(
                trimmed(&values[0])
                    .parse::<f64>()
                    .is_ok_and(|value| value.is_finite()),
            )),
            Builtin::ReadFile => read_file(&values[0]),
            Builtin::WriteFile => write_file(&values[0], &values[1], false),
            Builtin::AppendFile => write_file(&values[0], &values[1], true),
            Builtin::FileExists => Ok(Value::Bool(
                std::path::Path::new(values[0].as_str()).is_file(),
            )),
            Builtin::Argument => self.argument(&values[0]),
            _ => Ok(Value::Unit),
        };

        result.map_err(|error| Fault::Trap(Trap { error, span }))
    }
}

impl Interpreter<'_> {
    fn argument(&self, index: &Value) -> Result<Value, RuntimeError> {
        let position = index.as_int();

        if position < 0 || position as usize >= self.arguments.len() {
            return Err(RuntimeError::IndexOutOfBounds);
        }

        Ok(Value::str(self.arguments[position as usize].as_str()))
    }
}

fn read_file(path: &Value) -> Result<Value, RuntimeError> {
    std::fs::read_to_string(path.as_str())
        .map(Value::str)
        .map_err(|_| RuntimeError::InputOutput)
}

fn write_file(path: &Value, contents: &Value, append: bool) -> Result<Value, RuntimeError> {
    use std::io::Write;

    let opened = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .append(append)
        .truncate(!append)
        .open(path.as_str());

    let Ok(mut file) = opened else {
        return Err(RuntimeError::InputOutput);
    };

    file.write_all(contents.as_str().as_bytes())
        .map(|()| Value::Unit)
        .map_err(|_| RuntimeError::InputOutput)
}

fn rendered(value: &Value, ty: &Type) -> String {
    match ty {
        Type::Float => runtime::float(value.as_float()),
        _ => value.to_string(),
    }
}

fn trimmed(value: &Value) -> &str {
    value.as_str().trim()
}

fn append(values: &[Value]) -> Result<Value, RuntimeError> {
    if let Value::List(items) = &values[0] {
        items.borrow_mut().push(values[1].clone());
    }

    Ok(Value::Unit)
}

fn insert(values: &[Value]) -> Result<Value, RuntimeError> {
    let Value::List(items) = &values[0] else {
        return Ok(Value::Unit);
    };

    let index = values[1].as_int();
    let mut borrowed = items.borrow_mut();

    if index < 0 || index as usize > borrowed.len() {
        return Err(RuntimeError::IndexOutOfBounds);
    }

    borrowed.insert(index as usize, values[2].clone());
    Ok(Value::Unit)
}

fn remove_at(values: &[Value]) -> Result<Value, RuntimeError> {
    let Value::List(items) = &values[0] else {
        return Ok(Value::Unit);
    };

    let index = values[1].as_int();
    let mut borrowed = items.borrow_mut();

    if index < 0 || index as usize >= borrowed.len() {
        return Err(RuntimeError::IndexOutOfBounds);
    }

    Ok(borrowed.remove(index as usize))
}

fn has_key(values: &[Value]) -> Result<Value, RuntimeError> {
    let Value::Dict(entries) = &values[0] else {
        return Ok(Value::Bool(false));
    };

    Ok(Value::Bool(
        entries.borrow().contains_key(&values[1].as_key()),
    ))
}

fn remove_key(values: &[Value]) -> Result<Value, RuntimeError> {
    let Value::Dict(entries) = &values[0] else {
        return Ok(Value::Bool(false));
    };

    Ok(Value::Bool(
        entries.borrow_mut().remove(&values[1].as_key()).is_some(),
    ))
}

fn parse_int(value: &Value) -> Result<Value, RuntimeError> {
    trimmed(value)
        .parse::<i64>()
        .map(Value::Int)
        .map_err(|_| RuntimeError::InvalidConversion)
}

fn parse_float(value: &Value) -> Result<Value, RuntimeError> {
    match trimmed(value).parse::<f64>() {
        Ok(parsed) if parsed.is_finite() => Ok(Value::Float(parsed)),
        _ => Err(RuntimeError::InvalidConversion),
    }
}

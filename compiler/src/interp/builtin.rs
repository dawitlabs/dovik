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
            _ => Ok(Value::Unit),
        };

        result.map_err(|error| Fault::Trap(Trap { error, span }))
    }
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

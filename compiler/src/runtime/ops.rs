use super::Value;
use crate::ir::{CompareOp, RuntimeError};

type Checked = Result<Value, RuntimeError>;

pub fn add(lhs: &Value, rhs: &Value) -> Checked {
    match (lhs, rhs) {
        (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
        _ => int(lhs.as_int().checked_add(rhs.as_int())),
    }
}

pub fn subtract(lhs: &Value, rhs: &Value) -> Checked {
    match (lhs, rhs) {
        (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a - b)),
        _ => int(lhs.as_int().checked_sub(rhs.as_int())),
    }
}

pub fn multiply(lhs: &Value, rhs: &Value) -> Checked {
    match (lhs, rhs) {
        (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a * b)),
        _ => int(lhs.as_int().checked_mul(rhs.as_int())),
    }
}

pub fn negate(value: &Value) -> Checked {
    match value {
        Value::Float(inner) => Ok(Value::Float(-inner)),
        _ => int(value.as_int().checked_neg()),
    }
}

pub fn remainder(lhs: &Value, rhs: &Value) -> Checked {
    let divisor = rhs.as_int();

    if divisor == 0 {
        return Err(RuntimeError::RemainderByZero);
    }

    int(lhs.as_int().checked_rem(divisor))
}

pub fn divide(lhs: &Value, rhs: &Value) -> Checked {
    let divisor = rhs.as_float();

    if divisor == 0.0 {
        return Err(RuntimeError::DivisionByZero);
    }

    Ok(Value::Float(lhs.as_float() / divisor))
}

pub fn to_int(value: &Value) -> Checked {
    let inner = value.as_float();

    if !inner.is_finite() {
        return Err(RuntimeError::InvalidConversion);
    }

    let truncated = inner.trunc();

    if truncated < i64::MIN as f64 || truncated > i64::MAX as f64 {
        return Err(RuntimeError::InvalidConversion);
    }

    Ok(Value::Int(truncated as i64))
}

pub fn compare(op: CompareOp, lhs: &Value, rhs: &Value) -> Value {
    if matches!(op, CompareOp::Eq | CompareOp::Ne) {
        let equal = equals(lhs, rhs);
        return Value::Bool(match op {
            CompareOp::Eq => equal,
            _ => !equal,
        });
    }

    let ordered = order(lhs, rhs);

    Value::Bool(match (op, ordered) {
        (_, None) => false,
        (CompareOp::Lt, Some(result)) => result.is_lt(),
        (CompareOp::Gt, Some(result)) => result.is_gt(),
        (CompareOp::Le, Some(result)) => result.is_le(),
        (CompareOp::Ge, Some(result)) => result.is_ge(),
        _ => false,
    })
}

fn equals(lhs: &Value, rhs: &Value) -> bool {
    lhs == rhs
}

fn order(lhs: &Value, rhs: &Value) -> Option<std::cmp::Ordering> {
    match (lhs, rhs) {
        (Value::Float(a), Value::Float(b)) => a.partial_cmp(b),
        (Value::Str(a), Value::Str(b)) => Some(a.chars().cmp(b.chars())),
        _ => Some(lhs.as_int().cmp(&rhs.as_int())),
    }
}

fn int(result: Option<i64>) -> Checked {
    result.map(Value::Int).ok_or(RuntimeError::IntegerOverflow)
}

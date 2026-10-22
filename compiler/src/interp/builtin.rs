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

        if which == Builtin::ReadInput {
            return Ok(Value::str(read_input()));
        }

        if which == Builtin::Now {
            return Ok(Value::Float(wall_clock()));
        }

        if which == Builtin::Clock {
            return Ok(Value::Float(monotonic()));
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
            Builtin::Split => Ok(split(values[0].as_str(), values[1].as_str())),
            Builtin::Join => Ok(join(&values[0], values[1].as_str())),
            Builtin::Substring => substring(values[0].as_str(), &values[1], &values[2]),
            Builtin::Contains => Ok(Value::Bool(
                values[0].as_str().contains(values[1].as_str()),
            )),
            Builtin::IndexOf => index_of(values[0].as_str(), values[1].as_str()),
            Builtin::Trim => Ok(Value::str(values[0].as_str().trim())),
            Builtin::StartsWith => Ok(Value::Bool(
                values[0].as_str().starts_with(values[1].as_str()),
            )),
            Builtin::EndsWith => Ok(Value::Bool(
                values[0].as_str().ends_with(values[1].as_str()),
            )),
            Builtin::Upper => Ok(Value::str(ascii_case(values[0].as_str(), true))),
            Builtin::Lower => Ok(Value::str(ascii_case(values[0].as_str(), false))),
            Builtin::Replace => Ok(Value::str(replace(
                values[0].as_str(),
                values[1].as_str(),
                values[2].as_str(),
            ))),
            Builtin::Sort => Ok(sort(&values[0])),
            Builtin::ListDir => list_dir(&values[0]),
            Builtin::IsDir => Ok(Value::Bool(
                std::path::Path::new(values[0].as_str()).is_dir(),
            )),
            Builtin::CreateDir => directory_change(std::fs::create_dir(values[0].as_str())),
            Builtin::RemoveDir => directory_change(std::fs::remove_dir(values[0].as_str())),
            Builtin::RemoveFile => directory_change(std::fs::remove_file(values[0].as_str())),
            Builtin::Assert => match values[0].as_bool() {
                true => Ok(Value::Unit),
                false => Err(RuntimeError::AssertionFailed),
            },
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

fn split(text: &str, separator: &str) -> Value {
    if separator.is_empty() {
        let pieces = text.chars().map(|c| Value::str(c.to_string())).collect();
        return Value::list(pieces);
    }

    let pieces = text.split(separator).map(Value::str).collect();
    Value::list(pieces)
}

fn join(parts: &Value, separator: &str) -> Value {
    let Value::List(items) = parts else {
        return Value::str("");
    };

    let rendered: Vec<String> = items
        .borrow()
        .iter()
        .map(|item| item.as_str().to_string())
        .collect();

    Value::str(rendered.join(separator))
}

fn substring(text: &str, start: &Value, count: &Value) -> Result<Value, RuntimeError> {
    let from = start.as_int();
    let take = count.as_int();

    if from < 0 || take < 0 {
        return Err(RuntimeError::IndexOutOfBounds);
    }

    let scalars: Vec<char> = text.chars().collect();
    let end = from.checked_add(take).ok_or(RuntimeError::IndexOutOfBounds)?;

    if from as usize > scalars.len() || end as usize > scalars.len() {
        return Err(RuntimeError::IndexOutOfBounds);
    }

    let piece: String = scalars[from as usize..end as usize].iter().collect();
    Ok(Value::str(piece))
}

fn index_of(text: &str, needle: &str) -> Result<Value, RuntimeError> {
    let Some(byte) = text.find(needle) else {
        return Err(RuntimeError::IndexOutOfBounds);
    };

    Ok(Value::Int(text[..byte].chars().count() as i64))
}

fn replace(text: &str, from: &str, to: &str) -> String {
    match from.is_empty() {
        true => text.to_string(),
        false => text.replace(from, to),
    }
}

fn wall_clock() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_secs_f64())
        .unwrap_or(0.0)
}

fn monotonic() -> f64 {
    use std::sync::OnceLock;
    static ORIGIN: OnceLock<std::time::Instant> = OnceLock::new();
    ORIGIN.get_or_init(std::time::Instant::now).elapsed().as_secs_f64()
}

fn order(left: &Value, right: &Value) -> std::cmp::Ordering {
    use std::cmp::Ordering;

    match (left, right) {
        (Value::Int(a), Value::Int(b)) => a.cmp(b),
        (Value::Str(a), Value::Str(b)) => a.as_bytes().cmp(b.as_bytes()),
        (Value::Float(a), Value::Float(b)) => match (a.is_nan(), b.is_nan()) {
            (true, true) => Ordering::Equal,
            (true, false) => Ordering::Greater,
            (false, true) => Ordering::Less,
            _ => a.partial_cmp(b).unwrap_or(Ordering::Equal),
        },
        _ => Ordering::Equal,
    }
}

fn sort(value: &Value) -> Value {
    if let Value::List(items) = value {
        items.borrow_mut().sort_by(order);
    }

    Value::Unit
}

fn list_dir(path: &Value) -> Result<Value, RuntimeError> {
    let Ok(entries) = std::fs::read_dir(path.as_str()) else {
        return Err(RuntimeError::InputOutput);
    };

    let mut names: Vec<Value> = entries
        .filter_map(Result::ok)
        .map(|entry| Value::str(entry.file_name().to_string_lossy().into_owned()))
        .collect();

    names.sort_by(order);
    Ok(Value::list(names))
}

fn directory_change(result: std::io::Result<()>) -> Result<Value, RuntimeError> {
    result.map(|()| Value::Unit).map_err(|_| RuntimeError::InputOutput)
}

fn read_input() -> String {
    use std::io::Read;
    let mut text = String::new();
    let _ = std::io::stdin().read_to_string(&mut text);
    text
}

fn ascii_case(text: &str, upper: bool) -> String {
    text.chars()
        .map(|scalar| match upper {
            true => scalar.to_ascii_uppercase(),
            false => scalar.to_ascii_lowercase(),
        })
        .collect()
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

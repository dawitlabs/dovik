mod collections;
mod ops;
mod text;

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

pub use collections::{dict_get, dict_set, len, list_get, list_set};
pub use ops::{add, compare, divide, multiply, negate, remainder, subtract, to_int};
pub use text::{byte_len, concat, next_scalar, scalar_at, scalar_count};

use crate::types::Type;

pub type ListRef = Rc<RefCell<Vec<Value>>>;
pub type DictRef = Rc<RefCell<HashMap<DictKey, Value>>>;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Unit,
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(Rc<str>),
    List(ListRef),
    Dict(DictRef),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DictKey {
    Int(i64),
    Bool(bool),
    Str(String),
}

impl Value {
    pub fn str(text: impl Into<Rc<str>>) -> Self {
        Self::Str(text.into())
    }

    pub fn list(items: Vec<Value>) -> Self {
        Self::List(Rc::new(RefCell::new(items)))
    }

    pub fn dict(entries: HashMap<DictKey, Value>) -> Self {
        Self::Dict(Rc::new(RefCell::new(entries)))
    }

    pub fn as_int(&self) -> i64 {
        match self {
            Self::Int(value) => *value,
            Self::Bool(value) => i64::from(*value),
            _ => 0,
        }
    }

    pub fn as_float(&self) -> f64 {
        match self {
            Self::Float(value) => *value,
            Self::Int(value) => *value as f64,
            _ => 0.0,
        }
    }

    pub fn as_bool(&self) -> bool {
        matches!(self, Self::Bool(true))
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::Str(text) => text,
            _ => "",
        }
    }

    pub fn as_key(&self) -> DictKey {
        match self {
            Self::Int(value) => DictKey::Int(*value),
            Self::Bool(value) => DictKey::Bool(*value),
            _ => DictKey::Str(self.as_str().to_string()),
        }
    }

    pub fn from_key(key: &DictKey) -> Self {
        match key {
            DictKey::Int(value) => Self::Int(*value),
            DictKey::Bool(value) => Self::Bool(*value),
            DictKey::Str(text) => Self::str(text.as_str()),
        }
    }

    pub fn zero_of(ty: &Type) -> Self {
        match ty {
            Type::Int => Self::Int(0),
            Type::Float => Self::Float(0.0),
            Type::Bool => Self::Bool(false),
            Type::Str => Self::str(""),
            Type::List(_) => Self::list(Vec::new()),
            Type::Dict(..) => Self::dict(HashMap::new()),
            _ => Self::Unit,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unit => Ok(()),
            Self::Int(value) => write!(f, "{value}"),
            Self::Bool(value) => write!(f, "{value}"),
            Self::Str(text) => write!(f, "{text}"),
            Self::Float(value) => write!(f, "{}", float(*value)),
            Self::List(items) => {
                let rendered: Vec<String> = items.borrow().iter().map(Value::to_string).collect();
                write!(f, "[{}]", rendered.join(", "))
            }
            Self::Dict(entries) => {
                let rendered: Vec<String> = entries
                    .borrow()
                    .iter()
                    .map(|(key, value)| format!("{}: {value}", Value::from_key(key)))
                    .collect();
                write!(f, "{{{}}}", rendered.join(", "))
            }
        }
    }
}

pub fn float(value: f64) -> String {
    if value.is_nan() {
        return "nan".to_string();
    }

    if value.is_infinite() {
        return match value.is_sign_negative() {
            true => "-inf".to_string(),
            false => "inf".to_string(),
        };
    }

    format!("{value}")
}

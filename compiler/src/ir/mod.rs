mod print;

use crate::token::Span;
use crate::types::Type;

pub type RegId = usize;
pub type LocalId = usize;
pub type GlobalId = usize;
pub type BlockId = usize;
pub type FuncId = usize;
pub type StrId = usize;

#[derive(Debug, Default)]
pub struct Program {
    pub strings: Vec<String>,
    pub globals: Vec<Global>,
    pub functions: Vec<Function>,
    pub entry_functions: Vec<FuncId>,
    pub main: FuncId,
}

#[derive(Debug)]
pub struct Global {
    pub name: String,
    pub ty: Type,
}

#[derive(Debug)]
pub struct Function {
    pub name: String,
    pub return_type: Type,
    pub param_count: usize,
    pub locals: Vec<Local>,
    pub registers: Vec<Type>,
    pub blocks: Vec<Block>,
}

#[derive(Debug)]
pub struct Local {
    pub name: String,
    pub ty: Type,
}

#[derive(Debug, Default)]
pub struct Block {
    pub instructions: Vec<Inst>,
    pub terminator: Terminator,
}

#[derive(Debug, Clone)]
pub struct Inst {
    pub kind: InstKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Reg(RegId),
    None,
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(StrId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArithOp {
    Add,
    Sub,
    Mul,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompareOp {
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeError {
    IntegerOverflow,
    DivisionByZero,
    RemainderByZero,
    InvalidConversion,
    IndexOutOfBounds,
    MissingKey,
    InputOutput,
    AssertionFailed,
    StackExhausted,
    OutOfMemory,
}

#[derive(Debug, Clone)]
pub enum InstKind {
    LoadLocal { dest: RegId, local: LocalId },
    StoreLocal { local: LocalId, value: Value },
    LoadGlobal { dest: RegId, global: GlobalId },
    StoreGlobal { global: GlobalId, value: Value },

    IntArith {
        dest: RegId,
        op: ArithOp,
        lhs: Value,
        rhs: Value,
        checked: bool,
    },
    FloatArith {
        dest: RegId,
        op: ArithOp,
        lhs: Value,
        rhs: Value,
    },
    OptionalSome { dest: RegId, value: Value, inner: Type },
    OptionalGet { dest: RegId, value: Value, inner: Type },
    IntRem { dest: RegId, lhs: Value, rhs: Value },
    FloatDiv { dest: RegId, lhs: Value, rhs: Value },
    IntNeg {
        dest: RegId,
        value: Value,
        checked: bool,
    },
    FloatNeg { dest: RegId, value: Value },
    Not { dest: RegId, value: Value },

    IntToFloat { dest: RegId, value: Value },
    FloatToInt { dest: RegId, value: Value },

    Compare {
        dest: RegId,
        op: CompareOp,
        ty: Type,
        lhs: Value,
        rhs: Value,
    },

    StringConcat { dest: RegId, lhs: Value, rhs: Value },
    StringIndex { dest: RegId, string: Value, index: Value },
    StringNext {
        scalar: RegId,
        next_offset: RegId,
        string: Value,
        offset: Value,
    },
    StringByteLen { dest: RegId, string: Value },

    ListNew {
        dest: RegId,
        element: Type,
        items: Vec<Value>,
    },
    ListGet {
        dest: RegId,
        list: Value,
        index: Value,
        checked: bool,
    },
    ListSet {
        list: Value,
        index: Value,
        value: Value,
    },

    DictNew {
        dest: RegId,
        key: Type,
        value: Type,
        entries: Vec<(Value, Value)>,
    },
    DictGet { dest: RegId, dict: Value, key: Value },
    DictSet {
        dict: Value,
        key: Value,
        value: Value,
    },

    Len { dest: RegId, ty: Type, value: Value },
    Print { ty: Type, value: Value },

    Call {
        dest: Option<RegId>,
        function: FuncId,
        args: Vec<Value>,
    },
    Builtin {
        dest: Option<RegId>,
        which: crate::builtins::Builtin,
        args: Vec<(Value, Type)>,
    },
}

impl RuntimeError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::IntegerOverflow => "DVK5001",
            Self::DivisionByZero => "DVK5002",
            Self::RemainderByZero => "DVK5003",
            Self::InvalidConversion => "DVK5004",
            Self::IndexOutOfBounds => "DVK5005",
            Self::MissingKey => "DVK5006",
            Self::InputOutput => "DVK5007",
            Self::AssertionFailed => "DVK5008",
            Self::StackExhausted => "DVK5009",
            Self::OutOfMemory => "DVK5010",
        }
    }
}

impl InstKind {
    pub fn runtime_error(&self) -> Option<RuntimeError> {
        match self {
            Self::IntArith { checked, .. } | Self::IntNeg { checked, .. } => match checked {
                true => Some(RuntimeError::IntegerOverflow),
                false => None,
            },
            Self::FloatDiv { .. } => Some(RuntimeError::DivisionByZero),
            Self::IntRem { .. } => Some(RuntimeError::RemainderByZero),
            Self::FloatToInt { .. } => Some(RuntimeError::InvalidConversion),
            Self::StringIndex { .. } | Self::ListSet { .. } => {
                Some(RuntimeError::IndexOutOfBounds)
            }
            Self::ListGet { checked, .. } => match checked {
                true => Some(RuntimeError::IndexOutOfBounds),
                false => None,
            },
            Self::DictGet { .. } => Some(RuntimeError::MissingKey),
            Self::Builtin { which, .. } => builtin_error(*which),
            _ => None,
        }
    }
}

fn builtin_error(which: crate::builtins::Builtin) -> Option<RuntimeError> {
    use crate::builtins::Builtin;

    match which {
        Builtin::Insert | Builtin::RemoveAt => Some(RuntimeError::IndexOutOfBounds),
        Builtin::ToInt => Some(RuntimeError::InvalidConversion),
        Builtin::WriteFile
        | Builtin::AppendFile
        | Builtin::ReadInput
        | Builtin::ListDir
        | Builtin::CreateDir
        | Builtin::RemoveDir
        | Builtin::RemoveFile => Some(RuntimeError::InputOutput),
        Builtin::Assert => Some(RuntimeError::AssertionFailed),
        Builtin::Argument | Builtin::Substring => Some(RuntimeError::IndexOutOfBounds),
        _ => None,
    }
}

#[derive(Debug, Default)]
pub enum Terminator {
    Jump(BlockId),
    Branch {
        condition: Value,
        then_block: BlockId,
        else_block: BlockId,
    },
    Return(Option<Value>),
    #[default]
    Unreachable,
}

impl Program {
    pub fn checks(&self) -> Vec<RuntimeError> {
        self.functions
            .iter()
            .flat_map(|function| &function.blocks)
            .flat_map(|block| &block.instructions)
            .filter_map(|inst| inst.kind.runtime_error())
            .collect()
    }
}

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
    /// Top-level statements, in source order, as a synthetic entry point (spec 8.5).
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
    /// The first `param_count` locals are the parameters, in order.
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

#[derive(Debug)]
pub struct Inst {
    pub kind: InstKind,
    /// Where this came from. A trapping instruction needs it to report what failed and
    /// where (spec 15.2).
    pub span: Span,
}

/// Variables live in slots and intermediates live in registers, so the IR needs no phi
/// nodes: a value that two paths produce is stored to a slot and loaded after the merge.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Reg(RegId),
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

/// The six failures in spec 15.2. Each is produced by exactly the instructions that carry
/// that check, so a pass can enumerate the checks a function still contains.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeError {
    IntegerOverflow,
    DivisionByZero,
    RemainderByZero,
    InvalidConversion,
    IndexOutOfBounds,
    MissingKey,
}

#[derive(Debug)]
pub enum InstKind {
    LoadLocal { dest: RegId, local: LocalId },
    StoreLocal { local: LocalId, value: Value },
    LoadGlobal { dest: RegId, global: GlobalId },
    StoreGlobal { global: GlobalId, value: Value },

    /// Spec 7.2: traps on overflow. The check is part of this instruction rather than a
    /// separate branch, so no pass can drop it without replacing the instruction.
    IntArith {
        dest: RegId,
        op: ArithOp,
        lhs: Value,
        rhs: Value,
    },
    /// Spec 7.3: float arithmetic never traps. Overflow yields infinity.
    FloatArith {
        dest: RegId,
        op: ArithOp,
        lhs: Value,
        rhs: Value,
    },
    /// Spec 6.3: two `Int` operands, traps when the divisor is zero.
    IntRem { dest: RegId, lhs: Value, rhs: Value },
    /// Spec 6.2: always yields `Float` and traps on a zero divisor, for every operand
    /// combination. `Int` operands are widened by an explicit `IntToFloat` first.
    FloatDiv { dest: RegId, lhs: Value, rhs: Value },
    /// Spec 7.2: negating `Int`'s minimum overflows, so this traps too.
    IntNeg { dest: RegId, value: Value },
    FloatNeg { dest: RegId, value: Value },
    Not { dest: RegId, value: Value },

    IntToFloat { dest: RegId, value: Value },
    /// Spec 7.5: truncates toward zero, and traps on NaN, infinity, or out of range.
    FloatToInt { dest: RegId, value: Value },

    /// `ty` is the operand type: for a `List` or `Dict` this is the structural comparison
    /// of spec 13, and for `Float` it is IEEE, where NaN is not equal to itself.
    Compare {
        dest: RegId,
        op: CompareOp,
        ty: Type,
        lhs: Value,
        rhs: Value,
    },

    StringConcat { dest: RegId, lhs: Value, rhs: Value },
    /// Spec 10.4: yields the single scalar at that index, and traps out of bounds.
    StringIndex { dest: RegId, string: Value, index: Value },
    /// Advances a byte cursor by one scalar. `for` over a string uses this rather than
    /// repeated indexing, which spec 10.4 notes would be quadratic.
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
    /// Spec 11.3: traps out of bounds. Negative indexing does not exist.
    ListGet { dest: RegId, list: Value, index: Value },
    /// Spec 11.3: traps out of bounds, including at `len(xs)` — it does not append.
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
    /// Spec 12.4: traps when the key is absent. There is no default and no insert on read.
    DictGet { dest: RegId, dict: Value, key: Value },
    /// Spec 12.3: inserts when absent, overwrites when present.
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
}

impl InstKind {
    /// The runtime failure this instruction can produce, if any. A verification pass uses
    /// this to confirm the checks spec 15.2 requires are still present after optimization.
    pub fn runtime_error(&self) -> Option<RuntimeError> {
        match self {
            Self::IntArith { .. } | Self::IntNeg { .. } => Some(RuntimeError::IntegerOverflow),
            Self::FloatDiv { .. } => Some(RuntimeError::DivisionByZero),
            Self::IntRem { .. } => Some(RuntimeError::RemainderByZero),
            Self::FloatToInt { .. } => Some(RuntimeError::InvalidConversion),
            Self::StringIndex { .. } | Self::ListGet { .. } | Self::ListSet { .. } => {
                Some(RuntimeError::IndexOutOfBounds)
            }
            Self::DictGet { .. } => Some(RuntimeError::MissingKey),
            _ => None,
        }
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
    /// A block that control cannot leave, because everything before it returned.
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

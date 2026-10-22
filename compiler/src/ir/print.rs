use std::fmt;

use super::{ArithOp, Block, CompareOp, Function, Inst, InstKind, Program, Terminator, Value};

impl fmt::Display for Program {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, global) in self.globals.iter().enumerate() {
            writeln!(f, "global @{index} {} : {}", global.name, global.ty)?;
        }

        if !self.globals.is_empty() {
            writeln!(f)?;
        }

        for (index, function) in self.functions.iter().enumerate() {
            if index == self.main {
                writeln!(f, "; entry point")?;
            }
            write!(f, "{function}")?;

            if index + 1 < self.functions.len() {
                writeln!(f)?;
            }
        }

        Ok(())
    }
}

impl fmt::Display for Function {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let params: Vec<String> = self.locals[..self.param_count]
            .iter()
            .enumerate()
            .map(|(index, local)| format!("%{index} {} : {}", local.name, local.ty))
            .collect();

        writeln!(
            f,
            "function {}({}) -> {}:",
            self.name,
            params.join(", "),
            self.return_type
        )?;

        for (index, local) in self.locals.iter().enumerate().skip(self.param_count) {
            writeln!(f, "  local %{index} {} : {}", local.name, local.ty)?;
        }

        for (index, block) in self.blocks.iter().enumerate() {
            writeln!(f, "  block{index}:")?;
            write!(f, "{block}")?;
        }

        Ok(())
    }
}

impl fmt::Display for Block {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for inst in &self.instructions {
            writeln!(f, "    {inst}")?;
        }

        writeln!(f, "    {}", self.terminator)
    }
}

impl fmt::Display for Inst {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let marker = match self.kind.runtime_error() {
            Some(_) => " ; checked",
            None => "",
        };

        write!(f, "{}{marker}", self.kind)
    }
}

impl fmt::Display for InstKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LoadLocal { dest, local } => write!(f, "r{dest} = load %{local}"),
            Self::StoreLocal { local, value } => write!(f, "store %{local}, {value}"),
            Self::LoadGlobal { dest, global } => write!(f, "r{dest} = load @{global}"),
            Self::StoreGlobal { global, value } => write!(f, "store @{global}, {value}"),
            Self::IntArith { dest, op, lhs, rhs, checked } => {
                let suffix = match checked {
                    true => "",
                    false => ".unchecked",
                };
                write!(f, "r{dest} = int.{op}{suffix} {lhs}, {rhs}")
            }
            Self::FloatArith { dest, op, lhs, rhs } => {
                write!(f, "r{dest} = float.{op} {lhs}, {rhs}")
            }
            Self::IntRem { dest, lhs, rhs } => write!(f, "r{dest} = int.rem {lhs}, {rhs}"),
            Self::FloatDiv { dest, lhs, rhs } => write!(f, "r{dest} = float.div {lhs}, {rhs}"),
            Self::IntNeg { dest, value, checked } => {
                let suffix = match checked {
                    true => "",
                    false => ".unchecked",
                };
                write!(f, "r{dest} = int.neg{suffix} {value}")
            }
            Self::FloatNeg { dest, value } => write!(f, "r{dest} = float.neg {value}"),
            Self::Not { dest, value } => write!(f, "r{dest} = not {value}"),
            Self::IntToFloat { dest, value } => write!(f, "r{dest} = int.to_float {value}"),
            Self::FloatToInt { dest, value } => write!(f, "r{dest} = float.to_int {value}"),
            Self::Compare { dest, op, ty, lhs, rhs } => {
                write!(f, "r{dest} = cmp.{op} [{ty}] {lhs}, {rhs}")
            }
            Self::StringConcat { dest, lhs, rhs } => {
                write!(f, "r{dest} = string.concat {lhs}, {rhs}")
            }
            Self::StringIndex { dest, string, index } => {
                write!(f, "r{dest} = string.index {string}, {index}")
            }
            Self::StringNext { scalar, next_offset, string, offset } => write!(
                f,
                "r{scalar}, r{next_offset} = string.next {string}, {offset}"
            ),
            Self::StringByteLen { dest, string } => {
                write!(f, "r{dest} = string.byte_len {string}")
            }
            Self::OptionalSome { dest, value, inner } => {
                write!(f, "r{dest} = optional.some [{inner}] {value}")
            }
            Self::OptionalGet { dest, value, inner } => {
                write!(f, "r{dest} = optional.get [{inner}] {value}")
            }
            Self::ListNew { dest, element, items } => {
                write!(f, "r{dest} = list.new [{element}] {}", list(items))
            }
            Self::ListGet { dest, list: target, index, checked } => {
                let suffix = match checked {
                    true => "",
                    false => ".unchecked",
                };
                write!(f, "r{dest} = list.get{suffix} {target}, {index}")
            }
            Self::ListSet { list: target, index, value } => {
                write!(f, "list.set {target}, {index}, {value}")
            }
            Self::DictNew { dest, key, value, entries } => {
                let entries: Vec<String> = entries
                    .iter()
                    .map(|(k, v)| format!("{k}: {v}"))
                    .collect();
                write!(
                    f,
                    "r{dest} = dict.new [{key}, {value}] ({})",
                    entries.join(", ")
                )
            }
            Self::DictGet { dest, dict, key } => write!(f, "r{dest} = dict.get {dict}, {key}"),
            Self::DictSet { dict, key, value } => {
                write!(f, "dict.set {dict}, {key}, {value}")
            }
            Self::Len { dest, ty, value } => write!(f, "r{dest} = len [{ty}] {value}"),
            Self::Print { ty, value } => write!(f, "print [{ty}] {value}"),
            Self::Builtin { dest, which, args } => {
                let prefix = match dest {
                    Some(dest) => format!("r{dest} = "),
                    None => String::new(),
                };
                let rendered: Vec<String> = args
                    .iter()
                    .map(|(value, ty)| format!("[{ty}] {value}"))
                    .collect();
                write!(f, "{prefix}builtin.{}({})", which.name(), rendered.join(", "))
            }
            Self::Call { dest, function, args } => {
                let prefix = match dest {
                    Some(dest) => format!("r{dest} = "),
                    None => String::new(),
                };
                write!(f, "{prefix}call fn{function}({})", joined(args))
            }
        }
    }
}

impl fmt::Display for Terminator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Jump(target) => write!(f, "jump block{target}"),
            Self::Branch { condition, then_block, else_block } => write!(
                f,
                "branch {condition} ? block{then_block} : block{else_block}"
            ),
            Self::Return(Some(value)) => write!(f, "return {value}"),
            Self::Return(None) => write!(f, "return"),
            Self::Unreachable => write!(f, "unreachable"),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Reg(id) => write!(f, "r{id}"),
            Self::Int(value) => write!(f, "{value}"),
            Self::Float(value) => write!(f, "{value:?}"),
            Self::Bool(value) => write!(f, "{value}"),
            Self::Str(id) => write!(f, "str{id}"),
            Self::None => write!(f, "none"),
        }
    }
}

impl fmt::Display for ArithOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Add => write!(f, "add"),
            Self::Sub => write!(f, "sub"),
            Self::Mul => write!(f, "mul"),
        }
    }
}

impl fmt::Display for CompareOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Eq => write!(f, "eq"),
            Self::Ne => write!(f, "ne"),
            Self::Lt => write!(f, "lt"),
            Self::Gt => write!(f, "gt"),
            Self::Le => write!(f, "le"),
            Self::Ge => write!(f, "ge"),
        }
    }
}

fn list(values: &[Value]) -> String {
    format!("({})", joined(values))
}

fn joined(values: &[Value]) -> String {
    let rendered: Vec<String> = values.iter().map(Value::to_string).collect();
    rendered.join(", ")
}

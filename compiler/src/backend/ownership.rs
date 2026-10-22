use std::collections::HashSet;

use crate::builtins::Builtin;
use crate::ir::{Block, Function, InstKind, RegId, Terminator, Value};
use crate::types::Type;

pub(crate) struct Plan {
    pub(crate) after: Vec<Vec<RegId>>,
    pub(crate) at_end: Vec<RegId>,
}

pub(crate) fn is_reference(ty: &Type) -> bool {
    matches!(ty, Type::Str | Type::List(_) | Type::Dict(..))
}

pub(crate) fn plan(function: &Function, block: &Block) -> Plan {
    let mut after: Vec<Vec<RegId>> = vec![Vec::new(); block.instructions.len()];
    let mut at_end = Vec::new();

    for (index, inst) in block.instructions.iter().enumerate() {
        let Some(register) = owned_result(&inst.kind) else {
            continue;
        };

        if !is_reference(&function.registers[register]) {
            continue;
        }

        match last_use(block, register, index) {
            Some(position) => after[position].push(register),
            None => at_end.push(register),
        }
    }

    Plan { after, at_end }
}

fn last_use(block: &Block, register: RegId, from: usize) -> Option<usize> {
    if reads_terminator(&block.terminator, register) {
        return None;
    }

    block
        .instructions
        .iter()
        .enumerate()
        .skip(from + 1)
        .filter(|(_, inst)| reads(&inst.kind).contains(&register))
        .map(|(index, _)| index)
        .next_back()
        .or(Some(from))
}

pub(crate) fn owned_result(kind: &InstKind) -> Option<RegId> {
    match kind {
        InstKind::StringConcat { dest, .. }
        | InstKind::StringIndex { dest, .. }
        | InstKind::ListNew { dest, .. }
        | InstKind::DictNew { dest, .. } => Some(*dest),
        InstKind::StringNext { scalar, .. } => Some(*scalar),
        InstKind::Call { dest, .. } => *dest,
        InstKind::Builtin { dest, which, .. } => match which {
            Builtin::ToString
            | Builtin::RemoveAt
            | Builtin::ReadFile
            | Builtin::Argument
            | Builtin::Split
            | Builtin::Join
            | Builtin::Substring
            | Builtin::Trim
            | Builtin::Upper
            | Builtin::Lower
            | Builtin::Replace
            | Builtin::ReadInput => *dest,
            _ => None,
        },
        _ => None,
    }
}

pub(crate) fn reads(kind: &InstKind) -> HashSet<RegId> {
    let mut found = HashSet::new();
    let mut note = |value: &Value| {
        if let Value::Reg(register) = value {
            found.insert(*register);
        }
    };

    match kind {
        InstKind::StoreLocal { value, .. } | InstKind::StoreGlobal { value, .. } => note(value),
        InstKind::IntArith { lhs, rhs, .. }
        | InstKind::FloatArith { lhs, rhs, .. }
        | InstKind::IntRem { lhs, rhs, .. }
        | InstKind::FloatDiv { lhs, rhs, .. }
        | InstKind::Compare { lhs, rhs, .. }
        | InstKind::StringConcat { lhs, rhs, .. } => {
            note(lhs);
            note(rhs);
        }
        InstKind::IntNeg { value, .. }
        | InstKind::FloatNeg { value, .. }
        | InstKind::Not { value, .. }
        | InstKind::IntToFloat { value, .. }
        | InstKind::FloatToInt { value, .. }
        | InstKind::Len { value, .. }
        | InstKind::Print { value, .. } => note(value),
        InstKind::StringIndex { string, index, .. } => {
            note(string);
            note(index);
        }
        InstKind::StringNext { string, offset, .. } => {
            note(string);
            note(offset);
        }
        InstKind::StringByteLen { string, .. } => note(string),
        InstKind::ListNew { items, .. } => items.iter().for_each(&mut note),
        InstKind::ListGet { list, index, .. } => {
            note(list);
            note(index);
        }
        InstKind::ListSet { list, index, value } => {
            note(list);
            note(index);
            note(value);
        }
        InstKind::DictNew { entries, .. } => entries.iter().for_each(|(key, value)| {
            note(key);
            note(value);
        }),
        InstKind::DictGet { dict, key, .. } => {
            note(dict);
            note(key);
        }
        InstKind::DictSet { dict, key, value } => {
            note(dict);
            note(key);
            note(value);
        }
        InstKind::Call { args, .. } => args.iter().for_each(&mut note),
        InstKind::Builtin { args, .. } => args.iter().for_each(|(value, _)| note(value)),
        InstKind::LoadLocal { .. } | InstKind::LoadGlobal { .. } => {}
    }

    found
}

fn reads_terminator(terminator: &Terminator, register: RegId) -> bool {
    match terminator {
        Terminator::Branch { condition, .. } => *condition == Value::Reg(register),
        Terminator::Return(Some(value)) => *value == Value::Reg(register),
        _ => false,
    }
}

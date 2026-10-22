use std::collections::{HashMap, HashSet};

use crate::ir::{Function, InstKind, RegId, Terminator, Value};

pub(crate) fn escaping_registers(function: &mut Function) -> HashSet<RegId> {
    let mut defined: HashMap<RegId, usize> = HashMap::new();

    for (index, block) in function.blocks.iter_mut().enumerate() {
        for inst in &mut block.instructions {
            for register in destinations(&inst.kind) {
                defined.insert(register, index);
            }
        }
    }

    let mut escaping = HashSet::new();

    for (index, block) in function.blocks.iter_mut().enumerate() {
        let mut reads: Vec<RegId> = Vec::new();

        for inst in &mut block.instructions {
            reads.extend(super::fold::read_registers(&mut inst.kind));
        }

        reads.extend(terminator_reads(&block.terminator));

        for register in reads {
            if defined.get(&register).is_some_and(|origin| *origin != index) {
                escaping.insert(register);
            }
        }
    }

    escaping
}

pub(crate) fn destinations(kind: &InstKind) -> Vec<RegId> {
    match kind {
        InstKind::OptionalSome { dest, .. }
        | InstKind::OptionalGet { dest, .. }
        | InstKind::LoadLocal { dest, .. }
        | InstKind::LoadGlobal { dest, .. }
        | InstKind::IntArith { dest, .. }
        | InstKind::FloatArith { dest, .. }
        | InstKind::IntRem { dest, .. }
        | InstKind::FloatDiv { dest, .. }
        | InstKind::IntNeg { dest, .. }
        | InstKind::FloatNeg { dest, .. }
        | InstKind::Not { dest, .. }
        | InstKind::IntToFloat { dest, .. }
        | InstKind::FloatToInt { dest, .. }
        | InstKind::Compare { dest, .. }
        | InstKind::StringConcat { dest, .. }
        | InstKind::StringIndex { dest, .. }
        | InstKind::StringByteLen { dest, .. }
        | InstKind::ListNew { dest, .. }
        | InstKind::ListGet { dest, .. }
        | InstKind::DictNew { dest, .. }
        | InstKind::DictGet { dest, .. }
        | InstKind::Len { dest, .. } => vec![*dest],
        InstKind::StringNext {
            scalar,
            next_offset,
            ..
        } => vec![*scalar, *next_offset],
        InstKind::Call { dest, .. } | InstKind::Builtin { dest, .. } => {
            dest.iter().copied().collect()
        }
        InstKind::StoreLocal { .. }
        | InstKind::StoreGlobal { .. }
        | InstKind::ListSet { .. }
        | InstKind::DictSet { .. }
        | InstKind::Print { .. } => Vec::new(),
    }
}

fn terminator_reads(terminator: &Terminator) -> Vec<RegId> {
    let value = match terminator {
        Terminator::Branch { condition, .. } => Some(condition),
        Terminator::Return(Some(returned)) => Some(returned),
        _ => None,
    };

    match value {
        Some(Value::Reg(register)) => vec![*register],
        _ => Vec::new(),
    }
}

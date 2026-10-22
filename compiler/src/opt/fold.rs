use std::collections::{HashMap, HashSet};

use super::rejected;
use crate::error::{TypeError, TypeErrorKind};
use crate::ir::{ArithOp, Block, Inst, InstKind, RegId, RuntimeError, Terminator, Value};
use crate::runtime::{self, Value as RtValue};
use crate::token::Span;

pub(crate) fn fold_block(
    block: &mut Block,
    escaping: &HashSet<RegId>,
    errors: &mut Vec<TypeError>,
) {
    let mut known: HashMap<RegId, Value> = HashMap::new();
    let mut kept: Vec<Inst> = Vec::with_capacity(block.instructions.len());

    for mut inst in std::mem::take(&mut block.instructions) {
        rewrite(&mut inst.kind, &known);

        let Some((dest, value)) = fold_inst(&inst, errors) else {
            kept.push(inst);
            continue;
        };

        known.insert(dest, value);

        if escaping.contains(&dest) {
            kept.push(inst);
        }
    }

    rewrite_terminator(&mut block.terminator, &known);
    block.instructions = kept;
}

fn fold_inst(inst: &Inst, errors: &mut Vec<TypeError>) -> Option<(usize, Value)> {
    match &inst.kind {
        InstKind::IntArith { dest, op, lhs, rhs } => {
            let (lhs, rhs) = (literal(lhs)?, literal(rhs)?);
            let result = apply(*op, &lhs, &rhs);
            Some((*dest, check(result, inst.span, errors)?))
        }
        InstKind::FloatArith { dest, op, lhs, rhs } => {
            let (lhs, rhs) = (literal(lhs)?, literal(rhs)?);
            let result = apply(*op, &lhs, &rhs);
            Some((*dest, check(result, inst.span, errors)?))
        }
        InstKind::IntRem { dest, lhs, rhs } => {
            let (lhs, rhs) = (literal(lhs)?, literal(rhs)?);
            let result = runtime::remainder(&lhs, &rhs);
            Some((*dest, check(result, inst.span, errors)?))
        }
        InstKind::FloatDiv { dest, lhs, rhs } => {
            let (lhs, rhs) = (literal(lhs)?, literal(rhs)?);
            let result = runtime::divide(&lhs, &rhs);
            Some((*dest, check(result, inst.span, errors)?))
        }
        InstKind::IntNeg { dest, value } | InstKind::FloatNeg { dest, value } => {
            let value = literal(value)?;
            let result = runtime::negate(&value);
            Some((*dest, check(result, inst.span, errors)?))
        }
        InstKind::Not { dest, value } => {
            let value = literal(value)?;
            Some((*dest, Value::Bool(!value.as_bool())))
        }
        InstKind::IntToFloat { dest, value } => {
            let value = literal(value)?;
            Some((*dest, Value::Float(value.as_float())))
        }
        InstKind::FloatToInt { dest, value } => {
            let value = literal(value)?;
            let result = runtime::to_int(&value);
            Some((*dest, check(result, inst.span, errors)?))
        }
        InstKind::Compare { dest, op, ty, lhs, rhs } if comparable(ty) => {
            let (lhs, rhs) = (literal(lhs)?, literal(rhs)?);
            let result = runtime::compare(*op, &lhs, &rhs);
            Some((*dest, Value::Bool(result.as_bool())))
        }
        _ => None,
    }
}

fn comparable(ty: &crate::types::Type) -> bool {
    use crate::types::Type;
    matches!(ty, Type::Int | Type::Float | Type::Bool)
}

fn apply(op: ArithOp, lhs: &RtValue, rhs: &RtValue) -> Result<RtValue, RuntimeError> {
    match op {
        ArithOp::Add => runtime::add(lhs, rhs),
        ArithOp::Sub => runtime::subtract(lhs, rhs),
        ArithOp::Mul => runtime::multiply(lhs, rhs),
    }
}

fn check(
    result: Result<RtValue, RuntimeError>,
    span: Span,
    errors: &mut Vec<TypeError>,
) -> Option<Value> {
    match result {
        Ok(value) => constant(&value),
        Err(error) => {
            errors.push(rejected(TypeErrorKind::FoldedCheckFailed(error), span));
            None
        }
    }
}

fn literal(value: &Value) -> Option<RtValue> {
    match value {
        Value::Int(inner) => Some(RtValue::Int(*inner)),
        Value::Float(inner) => Some(RtValue::Float(*inner)),
        Value::Bool(inner) => Some(RtValue::Bool(*inner)),
        _ => None,
    }
}

fn constant(value: &RtValue) -> Option<Value> {
    match value {
        RtValue::Int(inner) => Some(Value::Int(*inner)),
        RtValue::Float(inner) => Some(Value::Float(*inner)),
        RtValue::Bool(inner) => Some(Value::Bool(*inner)),
        _ => None,
    }
}

fn rewrite_terminator(terminator: &mut Terminator, known: &HashMap<RegId, Value>) {
    match terminator {
        Terminator::Branch { condition, .. } => swap(condition, known),
        Terminator::Return(Some(returned)) => swap(returned, known),
        _ => {}
    }
}

fn swap(slot: &mut Value, known: &HashMap<RegId, Value>) {
    let Value::Reg(register) = slot else {
        return;
    };

    if let Some(value) = known.get(register) {
        *slot = value.clone();
    }
}

fn rewrite(kind: &mut InstKind, known: &HashMap<RegId, Value>) {
    for slot in operands(kind) {
        swap(slot, known);
    }
}

pub(crate) fn read_registers(kind: &mut InstKind) -> Vec<RegId> {
    operands(kind)
        .into_iter()
        .filter_map(|value| match value {
            Value::Reg(register) => Some(*register),
            _ => None,
        })
        .collect()
}

fn operands(kind: &mut InstKind) -> Vec<&mut Value> {
    match kind {
        InstKind::StoreLocal { value, .. } | InstKind::StoreGlobal { value, .. } => vec![value],
        InstKind::IntArith { lhs, rhs, .. }
        | InstKind::FloatArith { lhs, rhs, .. }
        | InstKind::IntRem { lhs, rhs, .. }
        | InstKind::FloatDiv { lhs, rhs, .. }
        | InstKind::Compare { lhs, rhs, .. }
        | InstKind::StringConcat { lhs, rhs, .. } => vec![lhs, rhs],
        InstKind::IntNeg { value, .. }
        | InstKind::FloatNeg { value, .. }
        | InstKind::Not { value, .. }
        | InstKind::IntToFloat { value, .. }
        | InstKind::FloatToInt { value, .. }
        | InstKind::Len { value, .. }
        | InstKind::Print { value, .. } => vec![value],
        InstKind::StringIndex { string, index, .. } => vec![string, index],
        InstKind::StringNext { string, offset, .. } => vec![string, offset],
        InstKind::StringByteLen { string, .. } => vec![string],
        InstKind::ListNew { items, .. } => items.iter_mut().collect(),
        InstKind::ListGet { list, index, .. } => vec![list, index],
        InstKind::ListSet { list, index, value } => vec![list, index, value],
        InstKind::DictNew { entries, .. } => entries
            .iter_mut()
            .flat_map(|(key, value)| [key, value])
            .collect(),
        InstKind::DictGet { dict, key, .. } => vec![dict, key],
        InstKind::DictSet { dict, key, value } => vec![dict, key, value],
        InstKind::Call { args, .. } => args.iter_mut().collect(),
        InstKind::Builtin { args, .. } => args.iter_mut().map(|(value, _)| value).collect(),
        InstKind::LoadLocal { .. } | InstKind::LoadGlobal { .. } => Vec::new(),
    }
}

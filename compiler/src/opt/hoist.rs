use std::collections::HashSet;

use crate::builtins::Builtin;
use crate::ir::{BlockId, Function, InstKind, Terminator, Value};
use crate::types::Type;

pub(crate) fn hoist_lengths(function: &mut Function) -> usize {
    let mut hoisted = 0;

    for header in 0..function.blocks.len() {
        let Some(body) = natural_loop(function, header) else {
            continue;
        };

        if body.iter().any(|block| mutates(function, *block)) {
            continue;
        }

        let Some(preheader) = sole_entry(function, header, &body) else {
            continue;
        };

        hoisted += move_lengths(function, header, preheader, &body);
    }

    hoisted
}

fn successors(function: &Function, block: BlockId) -> Vec<BlockId> {
    match &function.blocks[block].terminator {
        Terminator::Jump(target) => vec![*target],
        Terminator::Branch { then_block, else_block, .. } => vec![*then_block, *else_block],
        _ => Vec::new(),
    }
}

fn natural_loop(function: &Function, header: BlockId) -> Option<HashSet<BlockId>> {
    let latches: Vec<BlockId> = (0..function.blocks.len())
        .filter(|block| successors(function, *block).contains(&header))
        .filter(|block| *block >= header)
        .collect();

    if latches.is_empty() {
        return None;
    }

    let mut body: HashSet<BlockId> = HashSet::new();
    body.insert(header);
    let mut pending = latches;

    while let Some(block) = pending.pop() {
        if !body.insert(block) {
            continue;
        }

        for candidate in 0..function.blocks.len() {
            if candidate != header && successors(function, candidate).contains(&block) {
                pending.push(candidate);
            }
        }
    }

    Some(body)
}

fn sole_entry(function: &Function, header: BlockId, body: &HashSet<BlockId>) -> Option<BlockId> {
    let outside: Vec<BlockId> = (0..function.blocks.len())
        .filter(|block| !body.contains(block))
        .filter(|block| successors(function, *block).contains(&header))
        .collect();

    match outside.len() {
        1 => Some(outside[0]),
        _ => None,
    }
}

fn mutates(function: &Function, block: BlockId) -> bool {
    function.blocks[block].instructions.iter().any(|inst| match &inst.kind {
        InstKind::ListSet { .. } | InstKind::DictSet { .. } => true,
        InstKind::Builtin { which, args, .. } => match which {
            Builtin::Append | Builtin::Insert | Builtin::RemoveAt | Builtin::RemoveKey => true,
            _ => args.iter().any(|(_, ty)| reference(ty)),
        },
        InstKind::Call { args, .. } => args
            .iter()
            .any(|arg| matches!(arg, Value::Reg(_)) || matches!(arg, Value::Str(_))),
        _ => false,
    })
}

fn reference(ty: &Type) -> bool {
    matches!(ty, Type::Str | Type::List(_) | Type::Dict(..))
}

fn move_lengths(
    function: &mut Function,
    header: BlockId,
    preheader: BlockId,
    body: &HashSet<BlockId>,
) -> usize {
    let mut moved = Vec::new();
    let mut kept = Vec::new();

    for inst in std::mem::take(&mut function.blocks[header].instructions) {
        let movable = match &inst.kind {
            InstKind::Len { value, .. } => invariant(function, value, body),
            _ => false,
        };

        match movable {
            true => moved.push(inst),
            false => kept.push(inst),
        }
    }

    function.blocks[header].instructions = kept;
    let count = moved.len();

    for inst in moved.into_iter().rev() {
        function.blocks[preheader].instructions.push(inst);
    }

    count
}

fn invariant(function: &Function, value: &Value, body: &HashSet<BlockId>) -> bool {
    let Value::Reg(register) = value else {
        return true;
    };

    !body.iter().any(|block| {
        function.blocks[*block]
            .instructions
            .iter()
            .any(|inst| super::escape::destinations(&inst.kind).contains(register))
    })
}

use crate::builtins::Builtin;
use crate::ir::{Function, InstKind, Program};
use crate::types::Type;

pub(crate) fn classify(program: &Program) -> Vec<bool> {
    let mut pure: Vec<bool> = program
        .functions
        .iter()
        .map(|function| !touches_memory(function))
        .collect();

    loop {
        let mut changed = false;

        for (index, function) in program.functions.iter().enumerate() {
            if !pure[index] {
                continue;
            }

            let calls_impure = function
                .blocks
                .iter()
                .flat_map(|block| &block.instructions)
                .any(|inst| match &inst.kind {
                    InstKind::Call { function: target, .. } => !pure[*target],
                    _ => false,
                });

            if calls_impure {
                pure[index] = false;
                changed = true;
            }
        }

        if !changed {
            return pure;
        }
    }
}

fn touches_memory(function: &Function) -> bool {
    if function.locals.iter().any(|local| reference(&local.ty)) {
        return true;
    }

    let can_trap = function
        .blocks
        .iter()
        .flat_map(|block| &block.instructions)
        .any(|inst| inst.kind.runtime_error().is_some());

    if can_trap {
        return true;
    }

    if reference(&function.return_type) {
        return true;
    }

    function
        .blocks
        .iter()
        .flat_map(|block| &block.instructions)
        .any(|inst| match &inst.kind {
            InstKind::LoadGlobal { .. }
            | InstKind::StoreGlobal { .. }
            | InstKind::ListNew { .. }
            | InstKind::ListGet { .. }
            | InstKind::ListSet { .. }
            | InstKind::DictNew { .. }
            | InstKind::DictGet { .. }
            | InstKind::DictSet { .. }
            | InstKind::StringConcat { .. }
            | InstKind::StringIndex { .. }
            | InstKind::StringNext { .. }
            | InstKind::StringByteLen { .. }
            | InstKind::Len { .. }
            | InstKind::Print { .. } => true,
            InstKind::Builtin { which, .. } => !matches!(which, Builtin::ToInt | Builtin::ToFloat),
            InstKind::Compare { ty, .. } => reference(ty),
            _ => false,
        })
}

fn reference(ty: &Type) -> bool {
    matches!(ty, Type::Str | Type::List(_) | Type::Dict(..))
}

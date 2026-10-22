use std::collections::HashSet;

use crate::ir::{GlobalId, InstKind, Local, Program};

pub(crate) fn demote_globals(program: &mut Program) -> usize {
    let entry = program.main;
    let mut shared: HashSet<GlobalId> = HashSet::new();

    for (index, function) in program.functions.iter().enumerate() {
        if index == entry {
            continue;
        }

        for inst in function.blocks.iter().flat_map(|block| &block.instructions) {
            match &inst.kind {
                InstKind::LoadGlobal { global, .. } | InstKind::StoreGlobal { global, .. } => {
                    shared.insert(*global);
                }
                _ => {}
            }
        }
    }

    let mut slots: Vec<Option<usize>> = vec![None; program.globals.len()];
    let mut demoted = 0;

    for (global, info) in program.globals.iter().enumerate() {
        if shared.contains(&global) {
            continue;
        }

        let slot = program.functions[entry].locals.len();
        program.functions[entry].locals.push(Local {
            name: info.name.clone(),
            ty: info.ty.clone(),
        });
        slots[global] = Some(slot);
        demoted += 1;
    }

    if demoted == 0 {
        return 0;
    }

    for block in &mut program.functions[entry].blocks {
        for inst in &mut block.instructions {
            match &inst.kind {
                InstKind::LoadGlobal { dest, global } => {
                    if let Some(slot) = slots[*global] {
                        inst.kind = InstKind::LoadLocal { dest: *dest, local: slot };
                    }
                }
                InstKind::StoreGlobal { global, value } => {
                    if let Some(slot) = slots[*global] {
                        inst.kind = InstKind::StoreLocal {
                            local: slot,
                            value: value.clone(),
                        };
                    }
                }
                _ => {}
            }
        }
    }

    demoted
}

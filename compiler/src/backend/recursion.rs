use crate::ir::{InstKind, Program};

pub(crate) fn classify(program: &Program) -> Vec<bool> {
    let callees: Vec<Vec<usize>> = program
        .functions
        .iter()
        .map(|function| {
            let mut targets: Vec<usize> = function
                .blocks
                .iter()
                .flat_map(|block| &block.instructions)
                .filter_map(|inst| match &inst.kind {
                    InstKind::Call { function: target, .. } => Some(*target),
                    _ => None,
                })
                .collect();

            targets.sort_unstable();
            targets.dedup();
            targets
        })
        .collect();

    let mut reaches: Vec<Vec<bool>> = callees
        .iter()
        .map(|targets| {
            let mut row = vec![false; callees.len()];

            for target in targets {
                row[*target] = true;
            }

            row
        })
        .collect();

    loop {
        let mut changed = false;

        for caller in 0..reaches.len() {
            let reached: Vec<usize> = (0..reaches.len()).filter(|at| reaches[caller][*at]).collect();

            for middle in reached {
                for target in 0..reaches.len() {
                    if reaches[middle][target] && !reaches[caller][target] {
                        reaches[caller][target] = true;
                        changed = true;
                    }
                }
            }
        }

        if !changed {
            return (0..reaches.len()).map(|index| reaches[index][index]).collect();
        }
    }
}

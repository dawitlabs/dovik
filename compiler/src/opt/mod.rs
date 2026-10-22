mod demote;
pub(crate) mod escape;
mod hoist;
mod range;
mod fold;

use crate::error::{TypeError, TypeErrorKind};
use crate::ir::{Function, Program, Terminator, Value};
use crate::token::Span;

pub fn optimize(program: &mut Program) -> Vec<TypeError> {
    let mut errors = Vec::new();

    demote::demote_globals(program);

    let signatures: Vec<Vec<crate::types::Type>> = program
        .functions
        .iter()
        .map(|function| {
            function.locals[..function.param_count]
                .iter()
                .map(|local| local.ty.clone())
                .collect()
        })
        .collect();

    for function in &mut program.functions {
        optimize_function(function, &signatures, &mut errors);
    }

    errors.sort_by_key(|error| error.span.start);
    errors
}

fn optimize_function(
    function: &mut Function,
    signatures: &[Vec<crate::types::Type>],
    errors: &mut Vec<TypeError>,
) {
    let escaping = escape::escaping_registers(function);

    for block in &mut function.blocks {
        fold::fold_block(block, &escaping, errors);
    }

    hoist::hoist_lengths(function, signatures);
    range::elide_checks(function);

    for block in &mut function.blocks {
        let Terminator::Branch {
            condition: Value::Bool(taken),
            then_block,
            else_block,
        } = &block.terminator
        else {
            continue;
        };

        let target = match taken {
            true => *then_block,
            false => *else_block,
        };
        block.terminator = Terminator::Jump(target);
    }
}

pub(crate) fn rejected(kind: TypeErrorKind, span: Span) -> TypeError {
    TypeError { kind, span }
}

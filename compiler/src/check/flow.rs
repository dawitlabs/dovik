use crate::ast::{Block, Stmt, StmtKind};

pub(super) fn has_valued_return(block: &Block) -> bool {
    block.statements.iter().any(statement_has_valued_return)
}

fn statement_has_valued_return(statement: &Stmt) -> bool {
    match &statement.kind {
        StmtKind::Return(value) => value.is_some(),
        StmtKind::If { arms, else_block } => {
            arms.iter().any(|(_, block)| has_valued_return(block))
                || else_block.as_ref().is_some_and(has_valued_return)
        }
        StmtKind::While { body, .. } | StmtKind::For { body, .. } => has_valued_return(body),
        _ => false,
    }
}

pub(super) fn always_returns(block: &Block) -> bool {
    block.statements.iter().any(statement_always_returns)
}

fn statement_always_returns(statement: &Stmt) -> bool {
    match &statement.kind {
        StmtKind::Return(_) => true,
        StmtKind::If { arms, else_block } => {
            let Some(else_block) = else_block else {
                return false;
            };

            arms.iter().all(|(_, block)| always_returns(block)) && always_returns(else_block)
        }
        _ => false,
    }
}

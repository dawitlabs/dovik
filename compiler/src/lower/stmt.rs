use super::{Lowerer, Place};
use crate::ast::{AssignTarget, Expr, Stmt, StmtKind};
use crate::ir;
use crate::token::Span;
use crate::types::Type;

impl Lowerer<'_> {
    pub(crate) fn lower_statements(&mut self, statements: &[Stmt]) {
        for statement in statements {
            if self.terminated() {
                return;
            }

            self.lower_stmt(statement);
        }
    }

    pub(crate) fn lower_stmt(&mut self, statement: &Stmt) {
        match &statement.kind {
            StmtKind::Assign { target, value, .. } => self.lower_assign(target, value),
            StmtKind::Expr(expr) => {
                self.lower_expr(expr);
            }
            StmtKind::Function(_) => {}
            StmtKind::If { arms, else_block } => self.lower_if(arms, else_block.as_ref()),
            StmtKind::While { condition, body } => {
                self.lower_while(condition, &body.statements)
            }
            StmtKind::For {
                variable,
                iterable,
                body,
            } => self.lower_for(variable, iterable, &body.statements),
            StmtKind::Return(value) => {
                let value = value.as_ref().map(|expr| self.lower_expr(expr));
                self.seal(ir::Terminator::Return(value));
            }
            StmtKind::Break => {
                if let Some(target) = self.loop_exit() {
                    self.seal(ir::Terminator::Jump(target));
                }
            }
            StmtKind::Continue => {
                if let Some(target) = self.loop_header() {
                    self.seal(ir::Terminator::Jump(target));
                }
            }
        }
    }

    fn lower_assign(&mut self, target: &AssignTarget, value: &Expr) {
        match target {
            AssignTarget::Name(name) => {
                let value = self.lower_expr(value);

                match self.place_of(name.span, &name.name) {
                    Place::Local(slot) => {
                        self.emit(ir::InstKind::StoreLocal { local: slot, value }, name.span)
                    }
                    Place::Global(global) => {
                        self.emit(ir::InstKind::StoreGlobal { global, value }, name.span)
                    }
                    Place::Missing => {}
                }
            }
            AssignTarget::Index {
                object,
                index,
                span,
            } => self.lower_index_assign(object, index, value, *span),
        }
    }

    fn lower_index_assign(&mut self, object: &Expr, index: &Expr, value: &Expr, span: Span) {
        let container = self.typed.type_of(object.span);
        let target = self.lower_expr(object);
        let key = self.lower_expr(index);
        let value = self.lower_expr(value);

        let kind = match container {
            Type::Dict(..) => ir::InstKind::DictSet {
                dict: target,
                key,
                value,
            },
            _ => ir::InstKind::ListSet {
                list: target,
                index: key,
                value,
            },
        };

        self.emit(kind, span);
    }

    fn lower_if(&mut self, arms: &[(Expr, crate::ast::Block)], else_block: Option<&crate::ast::Block>) {
        let merge = self.new_block();
        let mut pending: Option<ir::BlockId> = None;

        for (condition, block) in arms {
            if let Some(next) = pending.take() {
                self.switch_to(next);
            }

            let value = self.lower_expr(condition);
            let then_block = self.new_block();
            let else_target = self.new_block();

            self.seal(ir::Terminator::Branch {
                condition: value,
                then_block,
                else_block: else_target,
            });

            self.switch_to(then_block);
            self.lower_statements(&block.statements);
            self.seal(ir::Terminator::Jump(merge));

            pending = Some(else_target);
        }

        if let Some(next) = pending {
            self.switch_to(next);

            if let Some(block) = else_block {
                self.lower_statements(&block.statements);
            }

            self.seal(ir::Terminator::Jump(merge));
        }

        self.switch_to(merge);
    }

    fn loop_header(&self) -> Option<ir::BlockId> {
        self.loops.last().map(|(header, _)| *header)
    }

    fn loop_exit(&self) -> Option<ir::BlockId> {
        self.loops.last().map(|(_, exit)| *exit)
    }
}

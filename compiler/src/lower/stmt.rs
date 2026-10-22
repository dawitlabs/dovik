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
            // A `String` target is rejected by the checker (spec 10.3), so anything else
            // that reaches here indexes a list.
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

    fn lower_while(&mut self, condition: &Expr, body: &[Stmt]) {
        let header = self.new_block();
        let body_block = self.new_block();
        let exit = self.new_block();

        self.seal(ir::Terminator::Jump(header));
        self.switch_to(header);

        let value = self.lower_expr(condition);
        self.seal(ir::Terminator::Branch {
            condition: value,
            then_block: body_block,
            else_block: exit,
        });

        self.switch_to(body_block);
        self.loops.push((header, exit));
        self.lower_statements(body);
        self.loops.pop();
        self.seal(ir::Terminator::Jump(header));

        self.switch_to(exit);
    }

    /// Spec 8.3: a `List` yields its elements and a `String` yields one scalar at a time.
    /// The string form walks a byte cursor rather than indexing by position, which spec
    /// 10.4 notes would make the loop quadratic.
    fn lower_for(&mut self, variable: &crate::ast::Ident, iterable: &Expr, body: &[Stmt]) {
        let container = self.typed.type_of(iterable.span);
        let sequence = self.lower_expr(iterable);
        let element = self.type_of_binding(variable.span);
        let item = self.declare_slot(variable.span, &variable.name, element);

        match container {
            Type::Str => self.lower_string_loop(sequence, item, body, variable.span),
            _ => self.lower_list_loop(sequence, item, body, variable.span, container),
        }
    }

    fn lower_list_loop(
        &mut self,
        list: ir::Value,
        item: ir::LocalId,
        body: &[Stmt],
        span: Span,
        container: Type,
    ) {
        let cursor = self.temp_slot("index", Type::Int);
        self.emit(
            ir::InstKind::StoreLocal {
                local: cursor,
                value: ir::Value::Int(0),
            },
            span,
        );

        let header = self.new_block();
        let body_block = self.new_block();
        let exit = self.new_block();

        self.seal(ir::Terminator::Jump(header));
        self.switch_to(header);

        let length = self.register(Type::Int);
        self.emit(
            ir::InstKind::Len {
                dest: length,
                ty: container,
                value: list.clone(),
            },
            span,
        );
        let index = self.register(Type::Int);
        self.emit(ir::InstKind::LoadLocal { dest: index, local: cursor }, span);
        let test = self.register(Type::Bool);
        self.emit(
            ir::InstKind::Compare {
                dest: test,
                op: ir::CompareOp::Lt,
                ty: Type::Int,
                lhs: ir::Value::Reg(index),
                rhs: ir::Value::Reg(length),
            },
            span,
        );
        self.seal(ir::Terminator::Branch {
            condition: ir::Value::Reg(test),
            then_block: body_block,
            else_block: exit,
        });

        self.switch_to(body_block);
        let current = self.register(Type::Int);
        self.emit(ir::InstKind::LoadLocal { dest: current, local: cursor }, span);
        let value = self.register(Type::Unknown);
        self.emit(
            ir::InstKind::ListGet {
                dest: value,
                list,
                index: ir::Value::Reg(current),
            },
            span,
        );
        self.emit(
            ir::InstKind::StoreLocal {
                local: item,
                value: ir::Value::Reg(value),
            },
            span,
        );

        self.loops.push((header, exit));
        self.lower_statements(body);
        self.loops.pop();

        if !self.terminated() {
            let before = self.register(Type::Int);
            self.emit(ir::InstKind::LoadLocal { dest: before, local: cursor }, span);
            let next = self.register(Type::Int);
            self.emit(
                ir::InstKind::IntArith {
                    dest: next,
                    op: ir::ArithOp::Add,
                    lhs: ir::Value::Reg(before),
                    rhs: ir::Value::Int(1),
                },
                span,
            );
            self.emit(
                ir::InstKind::StoreLocal {
                    local: cursor,
                    value: ir::Value::Reg(next),
                },
                span,
            );
        }

        self.seal(ir::Terminator::Jump(header));
        self.switch_to(exit);
    }

    fn lower_string_loop(
        &mut self,
        string: ir::Value,
        item: ir::LocalId,
        body: &[Stmt],
        span: Span,
    ) {
        let cursor = self.temp_slot("offset", Type::Int);
        self.emit(
            ir::InstKind::StoreLocal {
                local: cursor,
                value: ir::Value::Int(0),
            },
            span,
        );

        let header = self.new_block();
        let body_block = self.new_block();
        let exit = self.new_block();

        self.seal(ir::Terminator::Jump(header));
        self.switch_to(header);

        let length = self.register(Type::Int);
        self.emit(
            ir::InstKind::StringByteLen {
                dest: length,
                string: string.clone(),
            },
            span,
        );
        let offset = self.register(Type::Int);
        self.emit(ir::InstKind::LoadLocal { dest: offset, local: cursor }, span);
        let test = self.register(Type::Bool);
        self.emit(
            ir::InstKind::Compare {
                dest: test,
                op: ir::CompareOp::Lt,
                ty: Type::Int,
                lhs: ir::Value::Reg(offset),
                rhs: ir::Value::Reg(length),
            },
            span,
        );
        self.seal(ir::Terminator::Branch {
            condition: ir::Value::Reg(test),
            then_block: body_block,
            else_block: exit,
        });

        self.switch_to(body_block);
        let current = self.register(Type::Int);
        self.emit(ir::InstKind::LoadLocal { dest: current, local: cursor }, span);
        let scalar = self.register(Type::Str);
        let next_offset = self.register(Type::Int);
        self.emit(
            ir::InstKind::StringNext {
                scalar,
                next_offset,
                string,
                offset: ir::Value::Reg(current),
            },
            span,
        );
        self.emit(
            ir::InstKind::StoreLocal {
                local: item,
                value: ir::Value::Reg(scalar),
            },
            span,
        );
        self.emit(
            ir::InstKind::StoreLocal {
                local: cursor,
                value: ir::Value::Reg(next_offset),
            },
            span,
        );

        self.loops.push((header, exit));
        self.lower_statements(body);
        self.loops.pop();

        self.seal(ir::Terminator::Jump(header));
        self.switch_to(exit);
    }

    fn loop_header(&self) -> Option<ir::BlockId> {
        self.loops.last().map(|(header, _)| *header)
    }

    fn loop_exit(&self) -> Option<ir::BlockId> {
        self.loops.last().map(|(_, exit)| *exit)
    }
}

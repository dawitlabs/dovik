use super::Lowerer;
use crate::ast::{Expr, Stmt};
use crate::ir;
use crate::token::Span;
use crate::types::Type;

impl Lowerer<'_> {
    pub(crate) fn lower_while(&mut self, condition: &Expr, body: &[Stmt]) {
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

    pub(crate) fn lower_for(&mut self, variable: &crate::ast::Ident, iterable: &Expr, body: &[Stmt]) {
        let container = self.typed.type_of(iterable.span);
        let produced = self.lower_expr(iterable);
        let holder = self.temp_slot("sequence", container.clone());
        self.emit(
            ir::InstKind::StoreLocal {
                local: holder,
                value: produced,
            },
            variable.span,
        );
        let loaded = self.register(container.clone());
        self.emit(
            ir::InstKind::LoadLocal {
                dest: loaded,
                local: holder,
            },
            variable.span,
        );
        let sequence = ir::Value::Reg(loaded);
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
                checked: false,
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
                    checked: true,
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
}

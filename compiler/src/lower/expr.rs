use super::{Lowerer, Place};
use crate::ast::{BinaryOp, Expr, ExprKind, UnaryOp};
use crate::ir;
use crate::token::Span;
use crate::types::Type;

impl Lowerer<'_> {
    pub(crate) fn lower_expr(&mut self, expr: &Expr) -> ir::Value {
        let ty = self.typed.type_of(expr.span);

        match &expr.kind {
            ExprKind::Int(value) => ir::Value::Int(*value as i64),
            ExprKind::Float(value) => ir::Value::Float(*value),
            ExprKind::Bool(value) => ir::Value::Bool(*value),
            ExprKind::Str(value) => {
                let id = self.intern(value);
                ir::Value::Str(id)
            }
            ExprKind::Name(name) => self.lower_name(name, expr.span, ty),
            ExprKind::Unary { op, operand } => self.lower_unary(*op, operand, expr.span, ty),
            ExprKind::Binary { op, left, right } => {
                self.lower_binary(*op, left, right, expr.span, ty)
            }
            ExprKind::Call { callee, args } => self.lower_call(callee, args, expr.span, ty),
            ExprKind::Index { object, index } => self.lower_index(object, index, expr.span, ty),
            ExprKind::List(items) => self.lower_list(items, expr.span, ty),
            ExprKind::Dict(entries) => self.lower_dict(entries, expr.span, ty),
        }
    }

    fn lower_name(&mut self, name: &str, span: Span, ty: Type) -> ir::Value {
        let dest = self.register(ty);

        match self.place_of(span, name) {
            Place::Local(local) => {
                self.emit(ir::InstKind::LoadLocal { dest, local }, span);
                ir::Value::Reg(dest)
            }
            Place::Global(global) => {
                self.emit(ir::InstKind::LoadGlobal { dest, global }, span);
                ir::Value::Reg(dest)
            }
            Place::Missing => ir::Value::Int(0),
        }
    }

    fn lower_unary(&mut self, op: UnaryOp, operand: &Expr, span: Span, ty: Type) -> ir::Value {
        if op == UnaryOp::Neg {
            if let ExprKind::Int(value) = operand.kind {
                return ir::Value::Int((value as i128 * -1) as i64);
            }
        }

        let value = self.lower_expr(operand);
        let operand_type = self.typed.type_of(operand.span);
        let dest = self.register(ty);

        let kind = match (op, &operand_type) {
            (UnaryOp::Not, _) => ir::InstKind::Not { dest, value },
            (UnaryOp::Neg, Type::Float) => ir::InstKind::FloatNeg { dest, value },
            (UnaryOp::Neg, _) => ir::InstKind::IntNeg { dest, value },
        };

        self.emit(kind, span);
        ir::Value::Reg(dest)
    }

    fn lower_binary(
        &mut self,
        op: BinaryOp,
        left: &Expr,
        right: &Expr,
        span: Span,
        ty: Type,
    ) -> ir::Value {
        if matches!(op, BinaryOp::And | BinaryOp::Or) {
            return self.lower_short_circuit(op, left, right, span);
        }

        let operand_type = self.typed.type_of(left.span);
        let lhs = self.lower_expr(left);
        let rhs = self.lower_expr(right);

        if let Some(arith) = arith_op(op) {
            return self.lower_arith(arith, operand_type, lhs, rhs, span, ty);
        }

        if op == BinaryOp::Div {
            return self.lower_division(left, right, lhs, rhs, span);
        }

        if op == BinaryOp::Rem {
            let dest = self.register(Type::Int);
            self.emit(ir::InstKind::IntRem { dest, lhs, rhs }, span);
            return ir::Value::Reg(dest);
        }

        let dest = self.register(Type::Bool);
        self.emit(
            ir::InstKind::Compare {
                dest,
                op: compare_op(op),
                ty: operand_type,
                lhs,
                rhs,
            },
            span,
        );
        ir::Value::Reg(dest)
    }

    fn lower_arith(
        &mut self,
        op: ir::ArithOp,
        operand_type: Type,
        lhs: ir::Value,
        rhs: ir::Value,
        span: Span,
        ty: Type,
    ) -> ir::Value {
        let dest = self.register(ty);

        let kind = match operand_type {
            Type::Str => ir::InstKind::StringConcat { dest, lhs, rhs },
            Type::Float => ir::InstKind::FloatArith { dest, op, lhs, rhs },
            _ => ir::InstKind::IntArith { dest, op, lhs, rhs },
        };

        self.emit(kind, span);
        ir::Value::Reg(dest)
    }

    fn lower_division(
        &mut self,
        left: &Expr,
        right: &Expr,
        lhs: ir::Value,
        rhs: ir::Value,
        span: Span,
    ) -> ir::Value {
        let lhs = self.widen(lhs, self.typed.type_of(left.span), span);
        let rhs = self.widen(rhs, self.typed.type_of(right.span), span);
        let dest = self.register(Type::Float);
        self.emit(ir::InstKind::FloatDiv { dest, lhs, rhs }, span);
        ir::Value::Reg(dest)
    }

    fn widen(&mut self, value: ir::Value, ty: Type, span: Span) -> ir::Value {
        if ty != Type::Int {
            return value;
        }

        let dest = self.register(Type::Float);
        self.emit(ir::InstKind::IntToFloat { dest, value }, span);
        ir::Value::Reg(dest)
    }

    fn lower_short_circuit(
        &mut self,
        op: BinaryOp,
        left: &Expr,
        right: &Expr,
        span: Span,
    ) -> ir::Value {
        let slot = self.temp_slot("logical", Type::Bool);
        let lhs = self.lower_expr(left);
        self.emit(
            ir::InstKind::StoreLocal {
                local: slot,
                value: lhs.clone(),
            },
            span,
        );

        let evaluate = self.new_block();
        let merge = self.new_block();

        let (then_block, else_block) = match op {
            BinaryOp::And => (evaluate, merge),
            _ => (merge, evaluate),
        };

        self.seal(ir::Terminator::Branch {
            condition: lhs,
            then_block,
            else_block,
        });

        self.switch_to(evaluate);
        let rhs = self.lower_expr(right);
        self.emit(
            ir::InstKind::StoreLocal {
                local: slot,
                value: rhs,
            },
            span,
        );
        self.seal(ir::Terminator::Jump(merge));

        self.switch_to(merge);
        let dest = self.register(Type::Bool);
        self.emit(ir::InstKind::LoadLocal { dest, local: slot }, span);
        ir::Value::Reg(dest)
    }

    fn lower_index(&mut self, object: &Expr, index: &Expr, span: Span, ty: Type) -> ir::Value {
        let container = self.typed.type_of(object.span);
        let target = self.lower_expr(object);
        let key = self.lower_expr(index);
        let dest = self.register(ty);

        let kind = match container {
            Type::Dict(..) => ir::InstKind::DictGet {
                dest,
                dict: target,
                key,
            },
            Type::Str => ir::InstKind::StringIndex {
                dest,
                string: target,
                index: key,
            },
            _ => ir::InstKind::ListGet {
                dest,
                list: target,
                index: key,
            },
        };

        self.emit(kind, span);
        ir::Value::Reg(dest)
    }
}

fn arith_op(op: BinaryOp) -> Option<ir::ArithOp> {
    match op {
        BinaryOp::Add => Some(ir::ArithOp::Add),
        BinaryOp::Sub => Some(ir::ArithOp::Sub),
        BinaryOp::Mul => Some(ir::ArithOp::Mul),
        _ => None,
    }
}

fn compare_op(op: BinaryOp) -> ir::CompareOp {
    match op {
        BinaryOp::Eq => ir::CompareOp::Eq,
        BinaryOp::Ne => ir::CompareOp::Ne,
        BinaryOp::Lt => ir::CompareOp::Lt,
        BinaryOp::Gt => ir::CompareOp::Gt,
        BinaryOp::Le => ir::CompareOp::Le,
        _ => ir::CompareOp::Ge,
    }
}

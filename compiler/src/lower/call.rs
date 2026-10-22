use super::Lowerer;
use crate::ast::{Expr, ExprKind};
use crate::ir;
use crate::token::Span;
use crate::types::Type;

impl Lowerer<'_> {
    pub(crate) fn lower_call(&mut self, callee: &Expr, args: &[Expr], span: Span, ty: Type) -> ir::Value {
        let ExprKind::Name(name) = &callee.kind else {
            return ir::Value::Int(0);
        };

        if let Some(value) = self.lower_builtin(name, args, span, ty.clone()) {
            return value;
        }

        let Some(function) = self.function_id(name) else {
            return ir::Value::Int(0);
        };

        let args: Vec<ir::Value> = args.iter().map(|arg| self.lower_expr(arg)).collect();

        let dest = match ty {
            Type::Unit => None,
            _ => Some(self.register(ty)),
        };

        self.emit(ir::InstKind::Call { dest, function, args }, span);

        match dest {
            Some(dest) => ir::Value::Reg(dest),
            None => ir::Value::Int(0),
        }
    }

    fn lower_builtin(
        &mut self,
        name: &str,
        args: &[Expr],
        span: Span,
        ty: Type,
    ) -> Option<ir::Value> {
        if self.function_id(name).is_some() {
            return None;
        }

        if args.len() != 1 {
            return self.lower_generic_builtin(name, args, span, ty);
        }

        let argument_type = self.typed.type_of(args[0].span);

        match name {
            "print" => {
                let value = self.lower_expr(&args[0]);
                self.emit(
                    ir::InstKind::Print {
                        ty: argument_type,
                        value,
                    },
                    span,
                );
                Some(ir::Value::Int(0))
            }
            "len" => {
                let value = self.lower_expr(&args[0]);
                let dest = self.register(Type::Int);
                self.emit(
                    ir::InstKind::Len {
                        dest,
                        ty: argument_type,
                        value,
                    },
                    span,
                );
                Some(ir::Value::Reg(dest))
            }
            "to_int" => {
                let value = self.lower_expr(&args[0]);
                let dest = self.register(Type::Int);
                self.emit(ir::InstKind::FloatToInt { dest, value }, span);
                Some(ir::Value::Reg(dest))
            }
            "to_float" => {
                let value = self.lower_expr(&args[0]);
                let dest = self.register(Type::Float);
                self.emit(ir::InstKind::IntToFloat { dest, value }, span);
                Some(ir::Value::Reg(dest))
            }
            _ => self.lower_generic_builtin(name, args, span, ty),
        }
    }

    fn lower_generic_builtin(
        &mut self,
        name: &str,
        args: &[Expr],
        span: Span,
        ty: Type,
    ) -> Option<ir::Value> {
        let which = crate::builtins::Builtin::from_name(name)?;
        let lowered: Vec<(ir::Value, Type)> = args
            .iter()
            .map(|arg| (self.lower_expr(arg), self.typed.type_of(arg.span)))
            .collect();

        let dest = match ty {
            Type::Unit => None,
            _ => Some(self.register(ty)),
        };

        self.emit(ir::InstKind::Builtin { dest, which, args: lowered }, span);

        Some(match dest {
            Some(dest) => ir::Value::Reg(dest),
            None => ir::Value::Int(0),
        })
    }
}

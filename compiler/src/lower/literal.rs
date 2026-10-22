use super::Lowerer;
use crate::ast::Expr;
use crate::ir;
use crate::token::Span;
use crate::types::Type;

impl Lowerer<'_> {
    pub(crate) fn lower_list(&mut self, items: &[Expr], span: Span, ty: Type) -> ir::Value {
        let values: Vec<ir::Value> = items.iter().map(|item| self.lower_expr(item)).collect();

        let element = match &ty {
            Type::List(item) => (**item).clone(),
            _ => Type::Unknown,
        };

        let dest = self.register(ty);
        self.emit(
            ir::InstKind::ListNew {
                dest,
                element,
                items: values,
            },
            span,
        );
        ir::Value::Reg(dest)
    }

    pub(crate) fn lower_dict(&mut self, entries: &[(Expr, Expr)], span: Span, ty: Type) -> ir::Value {
        let values: Vec<(ir::Value, ir::Value)> = entries
            .iter()
            .map(|(key, value)| (self.lower_expr(key), self.lower_expr(value)))
            .collect();

        let (key, value) = match &ty {
            Type::Dict(key, value) => ((**key).clone(), (**value).clone()),
            _ => (Type::Unknown, Type::Unknown),
        };

        let dest = self.register(ty);
        self.emit(
            ir::InstKind::DictNew {
                dest,
                key,
                value,
                entries: values,
            },
            span,
        );
        ir::Value::Reg(dest)
    }
}

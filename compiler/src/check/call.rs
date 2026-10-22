use super::Checker;
use crate::ast::{Expr, ExprKind};
use crate::error::TypeErrorKind;
use crate::resolve::SymbolKind;
use crate::token::Span;
use crate::types::Type;

impl Checker<'_> {
    pub(crate) fn check_call(&mut self, callee: &Expr, args: &[Expr], span: Span) -> Type {
        let ExprKind::Name(name) = &callee.kind else {
            return self.reject_callee(callee, args);
        };

        let Some(symbol) = self.resolution.binding(callee.span) else {
            for arg in args {
                self.value_of(arg, None);
            }
            return Type::Unknown;
        };

        match symbol.kind {
            SymbolKind::Builtin => match crate::builtins::Builtin::from_name(name) {
                Some(builtin) => self.check_builtin(builtin, args, span),
                None => Type::Unknown,
            },
            SymbolKind::Function => {
                let binding = self.resolution.binding_id(callee.span);
                self.check_function_call(name.clone(), binding, args, span)
            }
            _ => self.reject_callee(callee, args),
        }
    }

    fn reject_callee(&mut self, callee: &Expr, args: &[Expr]) -> Type {
        let found = self.value_of(callee, None);

        for arg in args {
            self.value_of(arg, None);
        }

        if !found.is_unknown() {
            self.report(TypeErrorKind::NotCallable(found), callee.span);
        }

        Type::Unknown
    }

    fn check_function_call(
        &mut self,
        name: String,
        binding: Option<crate::resolve::BindingId>,
        args: &[Expr],
        span: Span,
    ) -> Type {
        let index = binding.and_then(|binding| self.by_binding.get(&binding).copied());

        let Some(index) = index else {
            return Type::Unknown;
        };

        let params = self.signatures[index].params.clone();

        if params.len() != args.len() {
            let kind = TypeErrorKind::ArgumentCount {
                name,
                expected: params.len(),
                found: args.len(),
            };
            self.report(kind, span);

            for arg in args {
                self.value_of(arg, None);
            }

            return self.signatures[index].return_type.clone().unwrap_or(Type::Unknown);
        }

        for (position, (arg, expected)) in args.iter().zip(&params).enumerate() {
            let found = self.value_of(arg, Some(expected));

            if !found.matches(expected) {
                let kind = TypeErrorKind::ArgumentType {
                    name: name.clone(),
                    position: position + 1,
                    expected: expected.clone(),
                    found,
                };
                self.report(kind, arg.span);
            }
        }

        self.signatures[index].return_type.clone().unwrap_or(Type::Unknown)
    }
}

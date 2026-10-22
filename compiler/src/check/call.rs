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
            SymbolKind::Builtin => self.check_builtin_call(name.clone(), args, span),
            SymbolKind::Function => self.check_function_call(name.clone(), args, span),
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

    fn check_function_call(&mut self, name: String, args: &[Expr], span: Span) -> Type {
        let Some(&index) = self.by_name.get(&name) else {
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

    fn check_builtin_call(&mut self, name: String, args: &[Expr], span: Span) -> Type {
        if args.len() != 1 {
            let kind = TypeErrorKind::ArgumentCount {
                name,
                expected: 1,
                found: args.len(),
            };
            self.report(kind, span);

            for arg in args {
                self.value_of(arg, None);
            }

            return Type::Unknown;
        }

        let found = self.value_of(&args[0], builtin_hint(&name).as_ref());

        let (accepted, result) = match name.as_str() {
            "print" => (
                matches!(found, Type::Int | Type::Float | Type::Bool | Type::Str),
                Type::Unit,
            ),
            "len" => (
                matches!(found, Type::Str | Type::List(_) | Type::Dict(..)),
                Type::Int,
            ),
            "to_int" => (found == Type::Float, Type::Int),
            "to_float" => (found == Type::Int, Type::Float),
            _ => (true, Type::Unknown),
        };

        if !accepted && !found.is_unknown() {
            let kind = TypeErrorKind::BuiltinArgument {
                name: builtin_name(&name),
                found,
            };
            self.report(kind, args[0].span);
        }

        result
    }
}

fn builtin_hint(name: &str) -> Option<Type> {
    match name {
        "to_int" => Some(Type::Float),
        "to_float" => Some(Type::Int),
        _ => None,
    }
}

fn builtin_name(name: &str) -> &'static str {
    match name {
        "print" => "print",
        "len" => "len",
        "to_int" => "to_int",
        _ => "to_float",
    }
}

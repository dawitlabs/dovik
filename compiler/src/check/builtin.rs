use super::Checker;
use crate::ast::Expr;
use crate::builtins::Builtin;
use crate::error::TypeErrorKind;
use crate::token::Span;
use crate::types::Type;

impl Checker<'_> {
    pub(crate) fn check_builtin(
        &mut self,
        builtin: Builtin,
        args: &[Expr],
        span: Span,
    ) -> Type {
        if args.len() != builtin.arity() {
            let kind = TypeErrorKind::ArgumentCount {
                name: builtin.name().to_string(),
                expected: builtin.arity(),
                found: args.len(),
            };
            self.report(kind, span);

            for arg in args {
                self.value_of(arg, None);
            }

            return builtin.result(&Type::Unknown);
        }

        if args.is_empty() {
            return builtin.result(&Type::Unit);
        }

        let first = self.value_of(&args[0], hint(builtin).as_ref());

        if !accepts_first(builtin, &first) && !first.is_unknown() {
            let kind = TypeErrorKind::BuiltinArgument {
                name: builtin.name(),
                found: first.clone(),
            };
            self.report(kind, args[0].span);
        }

        self.check_rest(builtin, &first, args);
        builtin.result(&first)
    }

    fn check_rest(&mut self, builtin: Builtin, first: &Type, args: &[Expr]) {
        let expected = remaining_types(builtin, first);

        for (position, (arg, want)) in args.iter().skip(1).zip(expected).enumerate() {
            let found = self.value_of(arg, Some(&want));

            if !found.matches(&want) {
                let kind = TypeErrorKind::ArgumentType {
                    name: builtin.name().to_string(),
                    position: position + 2,
                    expected: want,
                    found,
                };
                self.report(kind, arg.span);
            }
        }
    }
}

fn hint(builtin: Builtin) -> Option<Type> {
    match builtin {
        Builtin::ToInt => Some(Type::Float),
        Builtin::ToFloat => Some(Type::Int),
        Builtin::ParseInt
        | Builtin::ParseFloat
        | Builtin::CanParseInt
        | Builtin::CanParseFloat
        | Builtin::ReadFile
        | Builtin::WriteFile
        | Builtin::AppendFile
        | Builtin::FileExists
        | Builtin::Split
        | Builtin::Substring
        | Builtin::Contains
        | Builtin::IndexOf
        | Builtin::Trim
        | Builtin::StartsWith
        | Builtin::EndsWith
        | Builtin::Upper
        | Builtin::Lower
        | Builtin::Replace => Some(Type::Str),
        Builtin::Assert => Some(Type::Bool),
        Builtin::Join => Some(Type::List(Box::new(Type::Str))),
        Builtin::Argument => Some(Type::Int),
        _ => None,
    }
}

fn accepts_first(builtin: Builtin, found: &Type) -> bool {
    match builtin {
        Builtin::Print | Builtin::ToString => {
            matches!(found, Type::Int | Type::Float | Type::Bool | Type::Str)
        }
        Builtin::Len => matches!(found, Type::Str | Type::List(_) | Type::Dict(..)),
        Builtin::ToInt => *found == Type::Float,
        Builtin::ToFloat => *found == Type::Int,
        Builtin::Append | Builtin::Insert | Builtin::RemoveAt => matches!(found, Type::List(_)),
        Builtin::HasKey | Builtin::RemoveKey => matches!(found, Type::Dict(..)),
        Builtin::ParseInt
        | Builtin::ParseFloat
        | Builtin::CanParseInt
        | Builtin::CanParseFloat
        | Builtin::ReadFile
        | Builtin::WriteFile
        | Builtin::AppendFile
        | Builtin::FileExists
        | Builtin::Split
        | Builtin::Substring
        | Builtin::Contains
        | Builtin::IndexOf
        | Builtin::Trim
        | Builtin::StartsWith
        | Builtin::EndsWith
        | Builtin::Upper
        | Builtin::Lower
        | Builtin::Replace => *found == Type::Str,
        Builtin::Assert => *found == Type::Bool,
        Builtin::ReadInput => true,
        Builtin::Join => *found == Type::List(Box::new(Type::Str)),
        Builtin::Argument => *found == Type::Int,
        Builtin::ArgumentCount => true,
    }
}

fn remaining_types(builtin: Builtin, first: &Type) -> Vec<Type> {
    let element = match first {
        Type::List(item) => (**item).clone(),
        _ => Type::Unknown,
    };
    let key = match first {
        Type::Dict(key, _) => (**key).clone(),
        _ => Type::Unknown,
    };

    match builtin {
        Builtin::Append => vec![element],
        Builtin::Insert => vec![Type::Int, element],
        Builtin::RemoveAt => vec![Type::Int],
        Builtin::HasKey | Builtin::RemoveKey => vec![key],
        Builtin::WriteFile
        | Builtin::AppendFile
        | Builtin::Split
        | Builtin::Join
        | Builtin::Contains
        | Builtin::IndexOf
        | Builtin::StartsWith
        | Builtin::EndsWith => vec![Type::Str],
        Builtin::Substring => vec![Type::Int, Type::Int],
        Builtin::Replace => vec![Type::Str, Type::Str],
        _ => Vec::new(),
    }
}

use super::Checker;
use crate::ast::{BinaryOp, Expr, ExprKind, UnaryOp};
use crate::error::TypeErrorKind;
use crate::resolve::SymbolKind;
use crate::token::Span;
use crate::types::Type;

/// One past `Int::MAX`; legal only as the operand of a unary minus (spec 2.8).
const INT_MIN_MAGNITUDE: u64 = 9_223_372_036_854_775_808;

impl Checker<'_> {
    /// Checks an expression in a position that consumes its value, which `Unit` may not
    /// reach (spec 5.3).
    pub(crate) fn value_of(&mut self, expr: &Expr, expected: Option<&Type>) -> Type {
        let found = self.check_expr(expr, expected);

        if found == Type::Unit {
            self.report(TypeErrorKind::UnitAsValue, expr.span);
            return Type::Unknown;
        }

        found
    }

    pub(crate) fn check_expr(&mut self, expr: &Expr, expected: Option<&Type>) -> Type {
        match &expr.kind {
            ExprKind::Int(value) => self.check_int_literal(*value, expr.span),
            ExprKind::Float(_) => Type::Float,
            ExprKind::Str(_) => Type::Str,
            ExprKind::Bool(_) => Type::Bool,
            ExprKind::Name(name) => self.check_name(name, expr.span),
            ExprKind::Unary { op, operand } => self.check_unary(*op, operand, expr.span),
            ExprKind::Binary { op, left, right } => self.check_binary(*op, left, right, expr.span),
            ExprKind::Call { callee, args } => self.check_call(callee, args, expr.span),
            ExprKind::Index { object, index } => self.check_index(object, index),
            ExprKind::List(items) => self.check_list(items, expected, expr.span),
            ExprKind::Dict(entries) => self.check_dict(entries, expected, expr.span),
        }
    }

    fn check_int_literal(&mut self, value: u64, span: Span) -> Type {
        if value >= INT_MIN_MAGNITUDE {
            self.report(TypeErrorKind::IntegerLiteralOutOfRange, span);
        }

        Type::Int
    }

    fn check_name(&mut self, name: &str, span: Span) -> Type {
        let Some(symbol) = self.resolution.binding(span) else {
            return Type::Unknown;
        };

        // Dovik has no function values, so a bare function or built-in name is not a value.
        match symbol.kind {
            SymbolKind::Function => {
                self.report(TypeErrorKind::FunctionAsValue(name.to_string()), span);
                Type::Unknown
            }
            SymbolKind::Builtin => {
                self.report(TypeErrorKind::BuiltinAsValue(name.to_string()), span);
                Type::Unknown
            }
            _ => self.type_of_binding(span),
        }
    }

    fn check_unary(&mut self, op: UnaryOp, operand: &Expr, span: Span) -> Type {
        if op == UnaryOp::Neg {
            // Spec 2.8 folds a minus into the literal, which is what makes
            // `-9223372036854775808` the one way to write `Int`'s minimum.
            if let ExprKind::Int(value) = operand.kind {
                if value > INT_MIN_MAGNITUDE {
                    self.report(TypeErrorKind::IntegerLiteralOutOfRange, span);
                }
                return Type::Int;
            }
        }

        let found = self.value_of(operand, None);

        match op {
            UnaryOp::Neg if found.is_numeric() || found.is_unknown() => found,
            UnaryOp::Not if found.matches(&Type::Bool) => Type::Bool,
            _ => {
                let kind = TypeErrorKind::UnaryOperand {
                    op: match op {
                        UnaryOp::Neg => "-",
                        UnaryOp::Not => "not",
                    },
                    found,
                };
                self.report(kind, span);
                Type::Unknown
            }
        }
    }

    fn check_binary(&mut self, op: BinaryOp, left: &Expr, right: &Expr, span: Span) -> Type {
        let lhs = self.value_of(left, None);
        let rhs = self.value_of(right, None);

        if lhs.is_unknown() || rhs.is_unknown() {
            return result_of(op, &lhs);
        }

        let accepted = match op {
            // Spec 6.1: same type on both sides; `+` also concatenates two strings.
            BinaryOp::Add => lhs == rhs && (lhs.is_numeric() || lhs == Type::Str),
            BinaryOp::Sub | BinaryOp::Mul => lhs == rhs && lhs.is_numeric(),
            // Spec 6.3: two `Int`s only.
            BinaryOp::Rem => lhs == Type::Int && rhs == Type::Int,
            // Spec 6.2: the one operator that mixes `Int` and `Float`.
            BinaryOp::Div => lhs.is_numeric() && rhs.is_numeric(),
            BinaryOp::Eq | BinaryOp::Ne => lhs == rhs && lhs.supports_equality(),
            BinaryOp::Lt | BinaryOp::Gt | BinaryOp::Le | BinaryOp::Ge => {
                lhs == rhs && lhs.supports_ordering()
            }
            BinaryOp::And | BinaryOp::Or => lhs == Type::Bool && rhs == Type::Bool,
        };

        if !accepted {
            let kind = TypeErrorKind::BinaryOperands {
                op: symbol_of(op),
                left: lhs,
                right: rhs,
            };
            self.report(kind, span);
            return Type::Unknown;
        }

        result_of(op, &lhs)
    }

    fn check_index(&mut self, object: &Expr, index: &Expr) -> Type {
        let container = self.value_of(object, None);

        let (key, element) = match &container {
            Type::List(item) => (Type::Int, (**item).clone()),
            Type::Dict(key, value) => ((**key).clone(), (**value).clone()),
            // Spec 10.4: indexing a string yields a single-scalar string.
            Type::Str => (Type::Int, Type::Str),
            Type::Unknown => (Type::Unknown, Type::Unknown),
            _ => {
                self.report(TypeErrorKind::NotIndexable(container), object.span);
                (Type::Unknown, Type::Unknown)
            }
        };

        let found = self.value_of(index, Some(&key));

        if !found.matches(&key) {
            let kind = TypeErrorKind::IndexType {
                expected: key,
                found,
            };
            self.report(kind, index.span);
        }

        element
    }

    /// Spec 11.4: an empty literal has no element type of its own and takes one from the
    /// surrounding annotation or parameter type.
    fn check_list(&mut self, items: &[Expr], expected: Option<&Type>, span: Span) -> Type {
        let hint = match expected {
            Some(Type::List(item)) => Some((**item).clone()),
            _ => None,
        };

        let Some((first, rest)) = items.split_first() else {
            return match hint {
                Some(item) => Type::List(Box::new(item)),
                None => {
                    self.report(TypeErrorKind::CannotInferEmptyList, span);
                    Type::Unknown
                }
            };
        };

        let element = self.value_of(first, hint.as_ref());

        for item in rest {
            let found = self.value_of(item, Some(&element));

            if !found.matches(&element) {
                let kind = TypeErrorKind::MixedListElements {
                    first: element.clone(),
                    found,
                };
                self.report(kind, item.span);
            }
        }

        Type::List(Box::new(element))
    }

    fn check_dict(&mut self, entries: &[(Expr, Expr)], expected: Option<&Type>, span: Span) -> Type {
        let hint = match expected {
            Some(Type::Dict(key, value)) => Some(((**key).clone(), (**value).clone())),
            _ => None,
        };

        let Some(((first_key, first_value), rest)) = entries.split_first() else {
            return match hint {
                Some((key, value)) => Type::Dict(Box::new(key), Box::new(value)),
                None => {
                    self.report(TypeErrorKind::CannotInferEmptyDict, span);
                    Type::Unknown
                }
            };
        };

        let (key_hint, value_hint) = match hint {
            Some((key, value)) => (Some(key), Some(value)),
            None => (None, None),
        };

        let key_type = self.value_of(first_key, key_hint.as_ref());
        let value_type = self.value_of(first_value, value_hint.as_ref());

        if !key_type.is_valid_dict_key() {
            let kind = TypeErrorKind::InvalidDictKeyType(key_type.clone());
            self.report(kind, first_key.span);
        }

        for (key, value) in rest {
            let found_key = self.value_of(key, Some(&key_type));
            if !found_key.matches(&key_type) {
                let kind = TypeErrorKind::MixedDictKeys {
                    first: key_type.clone(),
                    found: found_key,
                };
                self.report(kind, key.span);
            }

            let found_value = self.value_of(value, Some(&value_type));
            if !found_value.matches(&value_type) {
                let kind = TypeErrorKind::MixedDictValues {
                    first: value_type.clone(),
                    found: found_value,
                };
                self.report(kind, value.span);
            }
        }

        Type::Dict(Box::new(key_type), Box::new(value_type))
    }
}

fn result_of(op: BinaryOp, operand: &Type) -> Type {
    match op {
        // Spec 6.2: division always yields `Float`, whatever it was given.
        BinaryOp::Div => Type::Float,
        BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Rem => operand.clone(),
        _ => Type::Bool,
    }
}

fn symbol_of(op: BinaryOp) -> &'static str {
    match op {
        BinaryOp::Add => "+",
        BinaryOp::Sub => "-",
        BinaryOp::Mul => "*",
        BinaryOp::Div => "/",
        BinaryOp::Rem => "%",
        BinaryOp::Eq => "==",
        BinaryOp::Ne => "!=",
        BinaryOp::Lt => "<",
        BinaryOp::Gt => ">",
        BinaryOp::Le => "<=",
        BinaryOp::Ge => ">=",
        BinaryOp::And => "and",
        BinaryOp::Or => "or",
    }
}

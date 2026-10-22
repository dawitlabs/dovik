use super::Parser;
use crate::error::ParseErrorKind;
use crate::ast::{TypeKind, TypeNode};
use crate::token::{Span, TokenKind};

impl Parser<'_> {
    pub(super) fn parse_type(&mut self) -> Option<TypeNode> {
        let name_span = self.span();

        if !self.at(&TokenKind::Identifier) {
            self.error_here("a type name");
            return None;
        }
        self.advance();

        let name = &self.source[name_span.start..name_span.end];
        let lowered = name.to_ascii_lowercase();
        let mut args = self.parse_type_arguments()?;
        let span = Span::new(name_span.start, self.previous_end());

        let (canonical, arity) = match lowered.as_str() {
            "int" => ("Int", 0),
            "float" => ("Float", 0),
            "bool" => ("Bool", 0),
            "string" => ("String", 0),
            "unit" => ("Unit", 0),
            "list" => ("List", 1),
            "dict" => ("Dict", 2),
            _ => {
                self.error_at(ParseErrorKind::UnknownType(name.to_string()), name_span);
                return None;
            }
        };

        if args.len() != arity {
            self.error_at(
                ParseErrorKind::TypeArity {
                    name: canonical,
                    expected: arity,
                    found: args.len(),
                },
                span,
            );
            return None;
        }

        let kind = match canonical {
            "Int" => TypeKind::Int,
            "Float" => TypeKind::Float,
            "Bool" => TypeKind::Bool,
            "String" => TypeKind::Str,
            "Unit" => TypeKind::Unit,
            "List" => TypeKind::List(Box::new(args.remove(0))),
            _ => {
                let value = args.remove(1);
                TypeKind::Dict(Box::new(args.remove(0)), Box::new(value))
            }
        };

        let mut node = TypeNode { kind, span };

        while self.at(&TokenKind::Question) {
            let close = self.span();
            self.advance();

            if matches!(node.kind, TypeKind::Optional(_)) {
                self.error_at(ParseErrorKind::NestedOptional, close);
                return None;
            }

            node = TypeNode {
                kind: TypeKind::Optional(Box::new(node)),
                span: Span::new(span.start, close.end),
            };
        }

        Some(node)
    }

    fn parse_type_arguments(&mut self) -> Option<Vec<TypeNode>> {
        if !self.eat(&TokenKind::Lt) {
            return Some(Vec::new());
        }

        let mut args = vec![self.parse_type()?];
        while self.eat(&TokenKind::Comma) {
            args.push(self.parse_type()?);
        }

        if !self.eat_type_close() {
            self.error_here("`>` to close the type parameters");
            return None;
        }

        Some(args)
    }

    pub(super) fn previous_end(&self) -> usize {
        match self.pos.checked_sub(1) {
            Some(index) => self.tokens[index].span.end,
            None => 0,
        }
    }
}

use super::Parser;
use crate::ast::{BinaryOp, Expr, ExprKind, UnaryOp};
use crate::token::{Span, TokenKind};

impl Parser<'_> {
    pub(super) fn parse_expr(&mut self) -> Option<Expr> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Option<Expr> {
        let mut left = self.parse_and()?;

        while self.eat(&TokenKind::Or) {
            let right = self.parse_and()?;
            left = binary(BinaryOp::Or, left, right);
        }

        Some(left)
    }

    fn parse_and(&mut self) -> Option<Expr> {
        let mut left = self.parse_equality()?;

        while self.eat(&TokenKind::And) {
            let right = self.parse_equality()?;
            left = binary(BinaryOp::And, left, right);
        }

        Some(left)
    }

    fn parse_equality(&mut self) -> Option<Expr> {
        let mut left = self.parse_comparison()?;

        loop {
            let op = if self.eat(&TokenKind::EqEq) {
                BinaryOp::Eq
            } else if self.eat(&TokenKind::BangEq) {
                BinaryOp::Ne
            } else {
                return Some(left);
            };

            let right = self.parse_comparison()?;
            left = binary(op, left, right);
        }
    }

    fn parse_comparison(&mut self) -> Option<Expr> {
        let mut left = self.parse_additive()?;

        loop {
            let op = if self.eat(&TokenKind::Lt) {
                BinaryOp::Lt
            } else if self.eat(&TokenKind::Gt) {
                BinaryOp::Gt
            } else if self.eat(&TokenKind::LtEq) {
                BinaryOp::Le
            } else if self.eat(&TokenKind::GtEq) {
                BinaryOp::Ge
            } else {
                return Some(left);
            };

            let right = self.parse_additive()?;
            left = binary(op, left, right);
        }
    }

    fn parse_additive(&mut self) -> Option<Expr> {
        let mut left = self.parse_multiplicative()?;

        loop {
            let op = if self.eat(&TokenKind::Plus) {
                BinaryOp::Add
            } else if self.eat(&TokenKind::Minus) {
                BinaryOp::Sub
            } else {
                return Some(left);
            };

            let right = self.parse_multiplicative()?;
            left = binary(op, left, right);
        }
    }

    fn parse_multiplicative(&mut self) -> Option<Expr> {
        let mut left = self.parse_unary()?;

        loop {
            let op = if self.eat(&TokenKind::Star) {
                BinaryOp::Mul
            } else if self.eat(&TokenKind::Slash) {
                BinaryOp::Div
            } else if self.eat(&TokenKind::Percent) {
                BinaryOp::Rem
            } else {
                return Some(left);
            };

            let right = self.parse_unary()?;
            left = binary(op, left, right);
        }
    }

    fn parse_unary(&mut self) -> Option<Expr> {
        let start = self.span().start;

        let op = if self.at(&TokenKind::Minus) {
            UnaryOp::Neg
        } else if self.at(&TokenKind::Not) {
            UnaryOp::Not
        } else {
            return self.parse_postfix();
        };

        self.advance();
        let operand = self.parse_unary()?;
        let span = Span::new(start, operand.span.end);

        Some(Expr {
            kind: ExprKind::Unary {
                op,
                operand: Box::new(operand),
            },
            span,
        })
    }

    fn parse_postfix(&mut self) -> Option<Expr> {
        let mut expr = self.parse_primary()?;

        loop {
            if self.at(&TokenKind::LParen) {
                expr = self.parse_call(expr)?;
                continue;
            }

            if self.at(&TokenKind::LBracket) {
                expr = self.parse_index(expr)?;
                continue;
            }

            return Some(expr);
        }
    }

    fn parse_call(&mut self, callee: Expr) -> Option<Expr> {
        self.advance();
        let mut args = Vec::new();

        if !self.at(&TokenKind::RParen) {
            args.push(self.parse_expr()?);
            while self.eat(&TokenKind::Comma) {
                args.push(self.parse_expr()?);
            }
        }

        let end = self.expect(&TokenKind::RParen, "`)` to close the argument list")?.end;
        let span = Span::new(callee.span.start, end);

        Some(Expr {
            kind: ExprKind::Call {
                callee: Box::new(callee),
                args,
            },
            span,
        })
    }

    fn parse_index(&mut self, object: Expr) -> Option<Expr> {
        self.advance();
        let index = self.parse_expr()?;
        let end = self.expect(&TokenKind::RBracket, "`]` to close the index")?.end;
        let span = Span::new(object.span.start, end);

        Some(Expr {
            kind: ExprKind::Index {
                object: Box::new(object),
                index: Box::new(index),
            },
            span,
        })
    }

    fn parse_primary(&mut self) -> Option<Expr> {
        let span = self.span();

        if self.at(&TokenKind::LParen) {
            self.advance();
            let inner = self.parse_expr()?;
            self.expect(&TokenKind::RParen, "`)` to close the group")?;
            return Some(inner);
        }

        if self.at(&TokenKind::LBracket) {
            return self.parse_list_literal();
        }

        if self.at(&TokenKind::LBrace) {
            return self.parse_dict_literal();
        }

        let kind = match self.kind() {
            TokenKind::Int(value) => ExprKind::Int(*value),
            TokenKind::Float(value) => ExprKind::Float(*value),
            TokenKind::Str(value) => ExprKind::Str(value.clone()),
            TokenKind::True => ExprKind::Bool(true),
            TokenKind::False => ExprKind::Bool(false),
            TokenKind::NoneLiteral => ExprKind::None,
            TokenKind::Identifier => ExprKind::Name(self.source[span.start..span.end].to_string()),
            _ => {
                self.error_here("an expression");
                return None;
            }
        };

        self.advance();
        Some(Expr { kind, span })
    }

    fn parse_list_literal(&mut self) -> Option<Expr> {
        let start = self.span().start;
        self.advance();
        let mut elements = Vec::new();

        if !self.at(&TokenKind::RBracket) {
            elements.push(self.parse_expr()?);
            while self.eat(&TokenKind::Comma) {
                elements.push(self.parse_expr()?);
            }
        }

        let end = self.expect(&TokenKind::RBracket, "`]` to close the list")?.end;

        Some(Expr {
            kind: ExprKind::List(elements),
            span: Span::new(start, end),
        })
    }

    fn parse_dict_literal(&mut self) -> Option<Expr> {
        let start = self.span().start;
        self.advance();
        let mut entries = Vec::new();

        if !self.at(&TokenKind::RBrace) {
            entries.push(self.parse_dict_entry()?);
            while self.eat(&TokenKind::Comma) {
                entries.push(self.parse_dict_entry()?);
            }
        }

        let end = self.expect(&TokenKind::RBrace, "`}` to close the dictionary")?.end;

        Some(Expr {
            kind: ExprKind::Dict(entries),
            span: Span::new(start, end),
        })
    }

    fn parse_dict_entry(&mut self) -> Option<(Expr, Expr)> {
        let key = self.parse_expr()?;
        self.expect(&TokenKind::Colon, "`:` between a key and its value")?;
        let value = self.parse_expr()?;
        Some((key, value))
    }
}

fn binary(op: BinaryOp, left: Expr, right: Expr) -> Expr {
    let span = Span::new(left.span.start, right.span.end);

    Expr {
        kind: ExprKind::Binary {
            op,
            left: Box::new(left),
            right: Box::new(right),
        },
        span,
    }
}

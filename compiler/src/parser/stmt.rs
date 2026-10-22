use super::Parser;
use crate::ast::{FunctionDecl, Param, Stmt, StmtKind};
use crate::error::ParseErrorKind;
use crate::token::{Span, TokenKind};

impl Parser<'_> {
    pub(super) fn parse_keyword_statement(&mut self, kind: StmtKind) -> Option<Stmt> {
        let start = self.span().start;
        self.advance();
        let end = self.expect_end_of_line()?;
        Some(Stmt {
            kind,
            span: Span::new(start, end),
        })
    }

    pub(super) fn parse_function(&mut self) -> Option<Stmt> {
        let start = self.span().start;

        if self.block_depth > 0 {
            self.error_at(ParseErrorKind::NestedFunction, self.span());
        }

        self.advance();
        let name = self.expect_ident("a function name")?;
        self.expect(&TokenKind::LParen, "`(`")?;
        let params = self.parse_params()?;
        self.expect(&TokenKind::RParen, "`)`")?;

        let return_type = match self.eat(&TokenKind::Arrow) {
            true => Some(self.parse_type()?),
            false => None,
        };

        let body = self.parse_block()?;
        let span = Span::new(start, body.span.end);

        Some(Stmt {
            kind: StmtKind::Function(Box::new(FunctionDecl {
                name,
                params,
                return_type,
                body,
            })),
            span,
        })
    }

    pub(super) fn parse_params(&mut self) -> Option<Vec<Param>> {
        let mut params = Vec::new();

        if self.at(&TokenKind::RParen) {
            return Some(params);
        }

        loop {
            let name = self.expect_ident("a parameter name")?;
            self.expect(&TokenKind::Colon, "`:` — every parameter needs a type")?;
            let ty = self.parse_type()?;
            params.push(Param { name, ty });

            if !self.eat(&TokenKind::Comma) {
                return Some(params);
            }
        }
    }

    pub(super) fn parse_if(&mut self) -> Option<Stmt> {
        let start = self.span().start;
        self.advance();

        let mut arms = vec![(self.parse_expr()?, self.parse_block()?)];
        let mut else_block = None;
        let mut end = arms[0].1.span.end;

        while self.eat(&TokenKind::Else) {
            if self.eat(&TokenKind::If) {
                let condition = self.parse_expr()?;
                let block = self.parse_block()?;
                end = block.span.end;
                arms.push((condition, block));
                continue;
            }

            let block = self.parse_block()?;
            end = block.span.end;
            else_block = Some(block);
            break;
        }

        Some(Stmt {
            kind: StmtKind::If { arms, else_block },
            span: Span::new(start, end),
        })
    }

    pub(super) fn parse_while(&mut self) -> Option<Stmt> {
        let start = self.span().start;
        self.advance();
        let condition = self.parse_expr()?;
        let body = self.parse_block()?;

        Some(Stmt {
            span: Span::new(start, body.span.end),
            kind: StmtKind::While { condition, body },
        })
    }

    pub(super) fn parse_for(&mut self) -> Option<Stmt> {
        let start = self.span().start;
        self.advance();
        let variable = self.expect_ident("a loop variable")?;
        self.expect(&TokenKind::In, "`in`")?;
        let iterable = self.parse_expr()?;
        let body = self.parse_block()?;

        Some(Stmt {
            span: Span::new(start, body.span.end),
            kind: StmtKind::For {
                variable,
                iterable,
                body,
            },
        })
    }

    pub(super) fn parse_return(&mut self) -> Option<Stmt> {
        let start = self.span().start;
        self.advance();

        let value = match self.at(&TokenKind::Newline) || self.at(&TokenKind::Eof) {
            true => None,
            false => Some(self.parse_expr()?),
        };

        let end = self.expect_end_of_line()?;
        Some(Stmt {
            kind: StmtKind::Return(value),
            span: Span::new(start, end),
        })
    }
}

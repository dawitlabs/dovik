mod cursor;
mod expr;
mod stmt;
mod types;

use crate::ast::{
    AssignTarget, Block, Expr, ExprKind, Ident, Import, ImportName, Program, Stmt, StmtKind,
};
use crate::error::{ParseError, ParseErrorKind};
use crate::token::{Span, Token, TokenKind};

pub fn parse(source: &str, tokens: &[Token]) -> (Program, Vec<ParseError>) {
    let mut parser = Parser {
        source,
        tokens,
        pos: 0,
        block_depth: 0,
        errors: Vec::new(),
        split_eq: None,
        eq_token: TokenKind::Eq,
    };

    let imports = parser.parse_imports();
    let statements = parser.parse_statements_until_eof();
    (
        Program {
            imports,
            statements,
        },
        parser.errors,
    )
}

struct Parser<'src> {
    source: &'src str,
    tokens: &'src [Token],
    pos: usize,
    block_depth: usize,
    errors: Vec<ParseError>,
    split_eq: Option<Span>,
    eq_token: TokenKind,
}

impl<'src> Parser<'src> {
    fn parse_statements_until_eof(&mut self) -> Vec<Stmt> {
        let mut statements = Vec::new();

        while !self.at(&TokenKind::Eof) {
            if self.eat(&TokenKind::Newline) {
                continue;
            }

            if self.skip_orphaned_block() {
                continue;
            }

            if self.at(&TokenKind::Dedent) {
                self.note_orphaned_layout();
                self.advance();
                continue;
            }

            match self.parse_statement() {
                Some(statement) => statements.push(statement),
                None => self.synchronize(),
            }
        }

        statements
    }

    fn parse_imports(&mut self) -> Vec<Import> {
        let mut imports = Vec::new();

        loop {
            if self.eat(&TokenKind::Newline) {
                continue;
            }

            if !self.at(&TokenKind::Import) {
                return imports;
            }

            match self.parse_import() {
                Some(import) => imports.push(import),
                None => self.synchronize(),
            }
        }
    }

    fn parse_import(&mut self) -> Option<Import> {
        let start = self.span();
        self.advance();

        let path_span = self.span();
        let TokenKind::Str(path) = self.kind().clone() else {
            self.error_here("an import path in quotes");
            return None;
        };
        self.advance();

        if path.starts_with('/') {
            self.error_at(ParseErrorKind::AbsoluteImportPath, path_span);
            return None;
        }

        self.expect(&TokenKind::LParen, "`(` and the names to import")?;

        let mut names = Vec::new();

        while !self.at(&TokenKind::RParen) {
            let exported = self.expect_ident("a name to import")?;

            let alias = match self.at(&TokenKind::Identifier) && self.current_text() == "as" {
                true => {
                    self.advance();
                    Some(self.expect_ident("a local name after `as`")?)
                }
                false => None,
            };

            names.push(ImportName { exported, alias });

            if !self.eat(&TokenKind::Comma) {
                break;
            }
        }

        let close = self.span();
        self.expect(&TokenKind::RParen, "`)`")?;

        if names.is_empty() {
            self.error_at(ParseErrorKind::EmptyImportList, close);
            return None;
        }

        self.expect_end_of_line()?;

        Some(Import {
            path,
            path_span,
            names,
            span: Span::new(start.start, close.end),
        })
    }

    fn parse_statement(&mut self) -> Option<Stmt> {
        if self.at(&TokenKind::Import) {
            let span = self.span();
            self.error_at(ParseErrorKind::ImportNotFirst, span);
            self.parse_import();
            return None;
        }
        if self.at(&TokenKind::Function) {
            return self.parse_function();
        }
        if self.at(&TokenKind::If) {
            return self.parse_if();
        }
        if self.at(&TokenKind::While) {
            return self.parse_while();
        }
        if self.at(&TokenKind::For) {
            return self.parse_for();
        }
        if self.at(&TokenKind::Return) {
            return self.parse_return();
        }
        if self.at(&TokenKind::Break) {
            return self.parse_keyword_statement(StmtKind::Break);
        }
        if self.at(&TokenKind::Continue) {
            return self.parse_keyword_statement(StmtKind::Continue);
        }

        self.parse_assign_or_expr()
    }

    fn parse_assign_or_expr(&mut self) -> Option<Stmt> {
        let start = self.span().start;
        let target = self.parse_expr()?;

        if self.at(&TokenKind::Colon) {
            return self.parse_annotated_declaration(target, start);
        }

        if !self.eat(&TokenKind::Eq) {
            let end = self.expect_end_of_line()?;
            return Some(Stmt {
                kind: StmtKind::Expr(target),
                span: Span::new(start, end),
            });
        }

        let value = self.parse_expr()?;
        let end = self.expect_end_of_line()?;
        Some(Stmt {
            kind: StmtKind::Assign {
                target: self.into_assign_target(target)?,
                ty: None,
                value,
            },
            span: Span::new(start, end),
        })
    }

    fn parse_annotated_declaration(&mut self, target: Expr, start: usize) -> Option<Stmt> {
        self.advance();
        let ty = self.parse_type()?;
        self.expect(&TokenKind::Eq, "`=` — every declaration needs an initializer")?;
        let value = self.parse_expr()?;
        let end = self.expect_end_of_line()?;

        let target = match self.into_assign_target(target)? {
            AssignTarget::Name(name) => AssignTarget::Name(name),
            other => {
                self.error_at(ParseErrorKind::AnnotationNeedsPlainName, other.span());
                return None;
            }
        };

        Some(Stmt {
            kind: StmtKind::Assign {
                target,
                ty: Some(ty),
                value,
            },
            span: Span::new(start, end),
        })
    }

    fn into_assign_target(&mut self, expr: Expr) -> Option<AssignTarget> {
        let span = expr.span;
        match expr.kind {
            ExprKind::Name(name) => Some(AssignTarget::Name(Ident { name, span })),
            ExprKind::Index { object, index } => Some(AssignTarget::Index {
                object: *object,
                index: *index,
                span,
            }),
            _ => {
                self.error_at(ParseErrorKind::InvalidAssignTarget, span);
                None
            }
        }
    }

    fn parse_block(&mut self) -> Option<Block> {
        self.expect(&TokenKind::Colon, "`:` to open a block")?;
        self.expect(&TokenKind::Newline, "a line break after `:`")?;
        let start = self.expect(&TokenKind::Indent, "an indented block")?.start;

        self.block_depth += 1;
        let mut statements = Vec::new();
        let mut end = start;

        while !self.at(&TokenKind::Dedent) && !self.at(&TokenKind::Eof) {
            if self.eat(&TokenKind::Newline) || self.skip_orphaned_block() {
                continue;
            }

            match self.parse_statement() {
                Some(statement) => {
                    end = statement.span.end;
                    statements.push(statement);
                }
                None => self.synchronize(),
            }
        }

        self.block_depth -= 1;
        self.eat(&TokenKind::Dedent);

        Some(Block {
            statements,
            span: Span::new(start, end),
        })
    }
}

impl AssignTarget {
    fn span(&self) -> Span {
        match self {
            Self::Name(ident) => ident.span,
            Self::Index { span, .. } => *span,
        }
    }
}

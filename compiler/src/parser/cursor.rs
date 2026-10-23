use super::Parser;
use crate::ast::Ident;
use crate::error::{ParseError, ParseErrorKind};
use crate::token::{Span, TokenKind};

impl<'src> Parser<'src> {
    pub(super) fn kind(&self) -> &TokenKind {
        if self.split_eq.is_some() {
            return &self.eq_token;
        }

        let index = self.pos.min(self.tokens.len().saturating_sub(1));
        &self.tokens[index].kind
    }

    pub(super) fn span(&self) -> Span {
        if let Some(span) = self.split_eq {
            return span;
        }

        let index = self.pos.min(self.tokens.len().saturating_sub(1));
        self.tokens[index].span
    }

    pub(super) fn current_text(&self) -> &'src str {
        let span = self.span();
        &self.source[span.start..span.end]
    }

    pub(super) fn at(&self, kind: &TokenKind) -> bool {
        self.kind() == kind
    }

    pub(super) fn advance(&mut self) {
        if self.split_eq.take().is_some() {
            self.pos += usize::from(self.pos + 1 < self.tokens.len());
            return;
        }

        if self.pos + 1 < self.tokens.len() {
            self.pos += 1;
        }
    }

    pub(super) fn eat_type_close(&mut self) -> bool {
        if self.eat(&TokenKind::Gt) {
            return true;
        }

        if !self.at(&TokenKind::GtEq) {
            return false;
        }

        let span = self.span();
        self.split_eq = Some(Span::new(span.start + 1, span.end));
        true
    }

    pub(super) fn eat(&mut self, kind: &TokenKind) -> bool {
        if !self.at(kind) {
            return false;
        }

        self.advance();
        true
    }

    pub(super) fn expect(&mut self, kind: &TokenKind, what: &'static str) -> Option<Span> {
        let span = self.span();

        if !self.eat(kind) {
            self.error_here(what);
            return None;
        }

        Some(span)
    }

    pub(super) fn expect_ident(&mut self, what: &'static str) -> Option<Ident> {
        if !self.at(&TokenKind::Identifier) {
            self.error_here(what);
            return None;
        }

        let span = self.span();
        self.advance();

        Some(Ident {
            name: self.source[span.start..span.end].to_string(),
            span,
        })
    }

    pub(super) fn expect_end_of_line(&mut self) -> Option<usize> {
        if self.at(&TokenKind::Eof) {
            return Some(self.span().start);
        }

        let span = self.span();
        if !self.eat(&TokenKind::Newline) {
            self.error_here("a line break");
            return None;
        }

        Some(span.start)
    }

    fn at_statement_boundary(&self) -> bool {
        match self.pos.checked_sub(1) {
            None => true,
            Some(index) => matches!(
                self.tokens[index].kind,
                TokenKind::Newline | TokenKind::Indent | TokenKind::Dedent
            ),
        }
    }

    pub(super) fn error_here(&mut self, what: &'static str) {
        let kind = ParseErrorKind::Expected {
            what,
            found: self.kind().describe(),
        };
        self.error_at(kind, self.span());
    }

    pub(super) fn error_at(&mut self, kind: ParseErrorKind, span: Span) {
        self.errors.push(ParseError { kind, span });
    }

    pub(super) fn skip_orphaned_block(&mut self) -> bool {
        if !self.at(&TokenKind::Indent) {
            return false;
        }

        self.note_orphaned_layout();
        let mut depth = 0usize;

        while !self.at(&TokenKind::Eof) {
            if self.at(&TokenKind::Indent) {
                depth += 1;
            } else if self.at(&TokenKind::Dedent) {
                depth = depth.saturating_sub(1);
                self.advance();

                if depth == 0 {
                    return true;
                }
                continue;
            }

            self.advance();
        }

        true
    }

    pub(super) fn note_orphaned_layout(&mut self) {
        if self.errors.is_empty() {
            self.error_here("a statement");
        }
    }

    pub(super) fn progress(&self) -> (usize, bool) {
        (self.pos, self.split_eq.is_some())
    }

    pub(super) fn recover(&mut self, before: (usize, bool)) {
        self.synchronize();

        if self.progress() == before {
            self.advance();
        }
    }

    pub(super) fn synchronize(&mut self) {
        if self.at_statement_boundary() {
            return;
        }

        loop {
            if matches!(
                self.kind(),
                TokenKind::Eof | TokenKind::Indent | TokenKind::Dedent
            ) {
                return;
            }

            let at_newline = self.at(&TokenKind::Newline);
            self.advance();

            if at_newline {
                return;
            }
        }
    }
}

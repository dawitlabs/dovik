use super::Parser;
use crate::ast::Ident;
use crate::error::{ParseError, ParseErrorKind};
use crate::token::{Span, TokenKind};

impl<'src> Parser<'src> {
    pub(super) fn kind(&self) -> &TokenKind {
        // The lexer always terminates the stream with `Eof`, so clamping here means the
        // parser reads `Eof` forever instead of panicking at the end of input.
        let index = self.pos.min(self.tokens.len().saturating_sub(1));
        &self.tokens[index].kind
    }

    pub(super) fn span(&self) -> Span {
        let index = self.pos.min(self.tokens.len().saturating_sub(1));
        self.tokens[index].span
    }

    pub(super) fn at(&self, kind: &TokenKind) -> bool {
        self.kind() == kind
    }

    pub(super) fn advance(&mut self) {
        if self.pos + 1 < self.tokens.len() {
            self.pos += 1;
        }
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

    /// Consumes the line break that terminates a simple statement, returning the offset
    /// the statement ends at. End of file is accepted so a file need not end in a break.
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

    /// Discards an indented region that no block header claimed, which happens when the
    /// header itself failed to parse. Reporting every token inside it would bury the one
    /// error that matters.
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

    /// Orphaned layout is a consequence of an earlier error, not an error itself — unless
    /// nothing has gone wrong yet, in which case staying silent would hide a real bug.
    pub(super) fn note_orphaned_layout(&mut self) {
        if self.errors.is_empty() {
            self.error_here("a statement");
        }
    }

    /// Skips to the start of the next statement after an error, so one bad line does not
    /// cascade. Layout tokens are left in place: the block structure is still trustworthy.
    pub(super) fn synchronize(&mut self) {
        // A statement can fail after it has already consumed its own line break, which
        // leaves the cursor on the next statement. Skipping then would throw away a good
        // line and, with it, any error that line would have reported.
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

use super::Lexer;
use crate::error::{LexError, LexErrorKind};
use crate::token::{Span, Token, TokenKind};

const BOM: char = '\u{FEFF}';

impl<'src> Lexer<'src> {
    pub(super) fn rest(&self) -> &'src str {
        &self.source[self.pos..]
    }

    pub(super) fn peek(&self) -> Option<char> {
        self.rest().chars().next()
    }

    pub(super) fn peek_second(&self) -> Option<char> {
        self.rest().chars().nth(1)
    }

    pub(super) fn bump(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.pos += ch.len_utf8();
        Some(ch)
    }

    pub(super) fn eat(&mut self, expected: char) -> bool {
        if self.peek() != Some(expected) {
            return false;
        }

        self.pos += expected.len_utf8();
        true
    }

    pub(super) fn eat_line_terminator(&mut self) -> bool {
        if self.rest().starts_with("\r\n") {
            self.pos += 2;
            return true;
        }

        self.eat('\n')
    }

    pub(super) fn skip_bom(&mut self) {
        self.eat(BOM);
    }

    pub(super) fn emit(&mut self, kind: TokenKind, span: Span) {
        self.tokens.push(Token { kind, span });
    }

    pub(super) fn push(&mut self, kind: TokenKind, start: usize) {
        self.emit(kind, Span::new(start, self.pos));
    }

    pub(super) fn error(&mut self, kind: LexErrorKind, start: usize) {
        self.errors.push(LexError {
            kind,
            span: Span::new(start, self.pos),
        });
    }
}

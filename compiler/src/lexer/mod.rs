mod cursor;
mod layout;
mod literals;

use crate::error::{LexError, LexErrorKind};
use crate::token::{self, Span, Token, TokenKind};

const INDENT_WIDTH: usize = 4;

pub fn lex(source: &str) -> (Vec<Token>, Vec<LexError>) {
    let (tokens, _, errors) = lex_with_trivia(source);
    (tokens, errors)
}

pub fn lex_with_trivia(source: &str) -> (Vec<Token>, Vec<Span>, Vec<LexError>) {
    let mut lexer = Lexer::new(source);
    lexer.run();
    (lexer.tokens, lexer.comments, lexer.errors)
}

struct Lexer<'src> {
    source: &'src str,
    pos: usize,
    tokens: Vec<Token>,
    errors: Vec<LexError>,
    indents: Vec<usize>,
    brackets: Vec<(char, usize)>,
    at_line_start: bool,
    comments: Vec<Span>,
}

impl<'src> Lexer<'src> {
    fn new(source: &'src str) -> Self {
        Self {
            source,
            pos: 0,
            tokens: Vec::new(),
            errors: Vec::new(),
            indents: vec![0],
            brackets: Vec::new(),
            at_line_start: true,
            comments: Vec::new(),
        }
    }

    fn run(&mut self) {
        self.skip_bom();

        loop {
            if self.brackets.is_empty() && self.at_line_start {
                self.at_line_start = false;
                self.open_line();
            }

            let Some(ch) = self.peek() else { break };
            self.scan_token(ch);
        }

        self.finish();
    }

    fn scan_token(&mut self, ch: char) {
        let start = self.pos;

        match ch {
            ' ' => {
                self.bump();
            }
            '\n' | '\r' => self.scan_line_terminator(start),
            '#' => self.skip_comment(),
            '"' => self.scan_string(),
            '_' | 'a'..='z' | 'A'..='Z' => self.scan_identifier(),
            '0'..='9' => self.scan_number(),
            _ => self.scan_symbol(ch, start),
        }
    }

    fn scan_identifier(&mut self) {
        let start = self.pos;

        while self.peek().is_some_and(|c| c.is_ascii_alphanumeric() || c == '_') {
            self.bump();
        }

        let kind = token::keyword(&self.source[start..self.pos]).unwrap_or(TokenKind::Identifier);
        self.push(kind, start);
    }

    fn scan_symbol(&mut self, ch: char, start: usize) {
        self.bump();

        let kind = match ch {
            '+' => TokenKind::Plus,
            '*' => TokenKind::Star,
            '/' => TokenKind::Slash,
            '%' => TokenKind::Percent,
            ',' => TokenKind::Comma,
            ':' => TokenKind::Colon,
            '-' if self.eat('>') => TokenKind::Arrow,
            '-' => TokenKind::Minus,
            '=' if self.eat('=') => TokenKind::EqEq,
            '=' => TokenKind::Eq,
            '<' if self.eat('=') => TokenKind::LtEq,
            '<' => TokenKind::Lt,
            '>' if self.eat('=') => TokenKind::GtEq,
            '>' => TokenKind::Gt,
            '!' if self.eat('=') => TokenKind::BangEq,
            '(' | '[' | '{' => return self.open_bracket(ch, start),
            ')' | ']' | '}' => return self.close_bracket(ch, start),
            other if !other.is_ascii() && (other.is_alphanumeric() || other == '_') => {
                return self.scan_non_ascii_identifier(start);
            }
            other => return self.error(LexErrorKind::UnexpectedChar(other), start),
        };

        self.push(kind, start);
    }

    fn scan_non_ascii_identifier(&mut self, start: usize) {
        while self.peek().is_some_and(|c| c.is_alphanumeric() || c == '_') {
            self.bump();
        }

        self.error(LexErrorKind::NonAsciiIdentifier, start);
    }

    fn open_bracket(&mut self, ch: char, start: usize) {
        self.brackets.push((ch, start));

        let kind = match ch {
            '(' => TokenKind::LParen,
            '[' => TokenKind::LBracket,
            _ => TokenKind::LBrace,
        };
        self.push(kind, start);
    }

    fn close_bracket(&mut self, ch: char, start: usize) {
        if self.brackets.pop().is_none() {
            self.error(LexErrorKind::UnmatchedCloseBracket(ch), start);
        }

        let kind = match ch {
            ')' => TokenKind::RParen,
            ']' => TokenKind::RBracket,
            _ => TokenKind::RBrace,
        };
        self.push(kind, start);
    }

    fn scan_line_terminator(&mut self, start: usize) {
        if !self.eat_line_terminator() {
            self.bump();
            return self.error(LexErrorKind::UnexpectedChar('\r'), start);
        }

        if self.brackets.is_empty() {
            self.emit(TokenKind::Newline, Span::new(start, self.pos));
            self.at_line_start = true;
        }
    }

    fn skip_comment(&mut self) {
        let start = self.pos;

        while self.peek().is_some_and(|c| c != '\n' && c != '\r') {
            self.bump();
        }

        self.comments.push(Span::new(start, self.pos));
    }

    fn finish(&mut self) {
        if self.tokens.last().is_some_and(|t| t.kind != TokenKind::Newline) {
            self.emit(TokenKind::Newline, Span::new(self.pos, self.pos));
        }

        while self.indents.len() > 1 {
            self.indents.pop();
            self.emit(TokenKind::Dedent, Span::new(self.pos, self.pos));
        }

        for (ch, start) in std::mem::take(&mut self.brackets) {
            self.errors.push(LexError {
                kind: LexErrorKind::UnclosedBracket(ch),
                span: Span::new(start, start + ch.len_utf8()),
            });
        }

        self.emit(TokenKind::Eof, Span::new(self.pos, self.pos));
    }
}

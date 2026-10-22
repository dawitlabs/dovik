use super::Lexer;
use crate::error::LexErrorKind;
use crate::token::TokenKind;

/// One past `Int::MAX`. The lexer accepts it because spec 2.8 folds a unary minus into
/// an integer literal, which makes `-9223372036854775808` legal; rejecting the bare
/// positive form is the parser's job, since only it can see the minus.
const MAX_INT_MAGNITUDE: u64 = 9_223_372_036_854_775_808;

const MAX_UNICODE_ESCAPE_DIGITS: usize = 6;

impl Lexer<'_> {
    pub(super) fn scan_number(&mut self) {
        let start = self.pos;
        self.skip_digits();
        let integer_part = &self.source[start..self.pos];

        let has_dot = self.peek() == Some('.');
        let is_float = has_dot && self.peek_second().is_some_and(|c| c.is_ascii_digit());

        if has_dot && !is_float {
            self.bump();
            return self.error(LexErrorKind::FloatMissingFractionDigits, start);
        }

        if is_float {
            self.bump();
            self.skip_digits();
        }

        if integer_part.len() > 1 && integer_part.starts_with('0') {
            return self.error(LexErrorKind::LeadingZero, start);
        }

        let text = &self.source[start..self.pos];

        if is_float {
            return match text.parse::<f64>() {
                Ok(value) if value.is_finite() => self.push(TokenKind::Float(value), start),
                _ => self.error(LexErrorKind::FloatOverflow, start),
            };
        }

        match text.parse::<u64>() {
            Ok(value) if value <= MAX_INT_MAGNITUDE => self.push(TokenKind::Int(value), start),
            _ => self.error(LexErrorKind::IntegerTooLarge, start),
        }
    }

    fn skip_digits(&mut self) {
        while self.peek().is_some_and(|c| c.is_ascii_digit()) {
            self.bump();
        }
    }

    pub(super) fn scan_string(&mut self) {
        let start = self.pos;
        self.bump();
        let mut value = String::new();

        loop {
            match self.peek() {
                None => return self.error(LexErrorKind::UnterminatedString, start),
                Some('\n' | '\r') => return self.error(LexErrorKind::NewlineInString, start),
                Some('"') => {
                    self.bump();
                    return self.push(TokenKind::Str(value), start);
                }
                Some('\\') => self.scan_escape(&mut value),
                Some(ch) => {
                    self.bump();
                    value.push(ch);
                }
            }
        }
    }

    /// Decodes one escape into `out`. A bad escape is recorded and skipped so the rest of
    /// the literal still lexes.
    fn scan_escape(&mut self, out: &mut String) {
        let start = self.pos;
        self.bump();

        let Some(ch) = self.bump() else {
            return self.error(LexErrorKind::TrailingBackslash, start);
        };

        let decoded = match ch {
            '"' => '"',
            '\\' => '\\',
            'n' => '\n',
            'r' => '\r',
            't' => '\t',
            '0' => '\0',
            'u' => return self.scan_unicode_escape(out, start),
            other => return self.error(LexErrorKind::InvalidEscape(other), start),
        };

        out.push(decoded);
    }

    fn scan_unicode_escape(&mut self, out: &mut String, start: usize) {
        if !self.eat('{') {
            return self.error(LexErrorKind::UnicodeEscapeMissingOpenBrace, start);
        }

        let digits_start = self.pos;
        while self.peek().is_some_and(|c| c.is_ascii_hexdigit()) {
            self.bump();
        }
        let digits = &self.source[digits_start..self.pos];

        if !self.eat('}') {
            return match self.peek() {
                Some(ch) if ch != '\n' && ch != '\r' && ch != '"' => {
                    self.error(LexErrorKind::UnicodeEscapeInvalidDigit(ch), start)
                }
                _ => self.error(LexErrorKind::UnicodeEscapeUnterminated, start),
            };
        }

        if digits.is_empty() {
            return self.error(LexErrorKind::UnicodeEscapeEmpty, start);
        }

        if digits.len() > MAX_UNICODE_ESCAPE_DIGITS {
            return self.error(LexErrorKind::UnicodeEscapeTooManyDigits, start);
        }

        // Six hex digits always fit in a u32, so the fallback is unreachable; u32::MAX
        // would be rejected by `from_u32` anyway.
        let value = u32::from_str_radix(digits, 16).unwrap_or(u32::MAX);

        // `from_u32` rejects exactly the two cases spec 10.1 names: surrogates and
        // anything above U+10FFFF.
        match char::from_u32(value) {
            Some(ch) => out.push(ch),
            None => self.error(LexErrorKind::UnicodeEscapeOutOfRange(value), start),
        }
    }
}

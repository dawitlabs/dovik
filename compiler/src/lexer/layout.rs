use super::{INDENT_WIDTH, Lexer};
use crate::error::LexErrorKind;
use crate::token::{Span, TokenKind};

impl Lexer<'_> {
    pub(super) fn open_line(&mut self) {
        loop {
            let start = self.pos;
            let width = self.scan_indentation(start);

            if self.eat_line_terminator() {
                continue;
            }

            if self.peek() == Some('#') {
                self.skip_comment();
                self.eat_line_terminator();
                continue;
            }

            if self.peek().is_none() {
                return;
            }

            self.apply_indent(width, start);
            return;
        }
    }

    fn scan_indentation(&mut self, start: usize) -> usize {
        let mut width = 0;
        let mut has_tab = false;

        while let Some(ch) = self.peek() {
            match ch {
                ' ' => {
                    self.bump();
                    width += 1;
                }
                '\t' => {
                    self.bump();
                    has_tab = true;
                }
                _ => break,
            }
        }

        if has_tab {
            self.error(LexErrorKind::TabInIndentation, start);
        }

        width
    }

    fn apply_indent(&mut self, width: usize, start: usize) {
        let current = self.current_indent();

        if width % INDENT_WIDTH != 0 {
            return self.error(LexErrorKind::IndentNotMultipleOfFour(width), start);
        }

        if width == current {
            return;
        }

        if width > current {
            if width != current + INDENT_WIDTH {
                self.error(
                    LexErrorKind::IndentSkippedLevel {
                        found: width,
                        expected: current + INDENT_WIDTH,
                    },
                    start,
                );
            }
            return self.open_block(width, start);
        }

        while self.current_indent() > width {
            self.indents.pop();
            self.emit(TokenKind::Dedent, Span::new(start, self.pos));
        }

        if self.current_indent() != width {
            self.error(LexErrorKind::InconsistentDedent(width), start);
            self.open_block(width, start);
        }
    }

    fn open_block(&mut self, width: usize, start: usize) {
        self.indents.push(width);
        self.emit(TokenKind::Indent, Span::new(start, self.pos));
    }

    fn current_indent(&self) -> usize {
        self.indents.last().copied().unwrap_or(0)
    }
}

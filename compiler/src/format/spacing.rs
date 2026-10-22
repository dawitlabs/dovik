use crate::token::TokenKind;

pub(super) fn canonical_type_name(text: &str) -> String {
    match text.to_ascii_lowercase().as_str() {
        "int" => "Int".to_string(),
        "float" => "Float".to_string(),
        "bool" => "Bool".to_string(),
        "string" => "String".to_string(),
        "list" => "List".to_string(),
        "dict" => "Dict".to_string(),
        "unit" => "Unit".to_string(),
        _ => text.to_string(),
    }
}

#[derive(Default)]
pub(super) struct Context {
    pub(super) in_type: bool,
    angle: usize,
    previous_was_unary: bool,
    expect_operand: bool,
}

impl Context {
    pub(super) fn close_angle(&mut self) {
        self.angle = self.angle.saturating_sub(1);
    }

    pub(super) fn needs_space(&self, previous: &TokenKind, next: &TokenKind) -> bool {
        if self.previous_was_unary {
            return false;
        }

        if opens(previous) || closes(next) {
            return false;
        }

        if matches!(next, TokenKind::Comma | TokenKind::Colon) {
            return false;
        }

        if is_call_or_index(previous, next) {
            return false;
        }

        if self.in_type && angled(previous, next) {
            return false;
        }

        true
    }

    pub(super) fn advance(&mut self, kind: &TokenKind, next: Option<&TokenKind>) {
        self.previous_was_unary = *kind == TokenKind::Minus && self.expect_operand;

        match kind {
            TokenKind::Lt if self.in_type => self.angle += 1,
            TokenKind::Gt if self.in_type => {
                self.close_angle();
                if self.angle == 0 {
                    self.in_type = false;
                }
            }
            TokenKind::Eq => self.in_type = false,
            TokenKind::Comma | TokenKind::RParen | TokenKind::RBrace if self.angle == 0 => {
                self.in_type = false;
            }
            TokenKind::Colon | TokenKind::Arrow => {
                self.in_type = starts_a_type(next);
                self.angle = 0;
            }
            _ => {}
        }

        self.expect_operand = expects_operand(kind);
    }
}

fn starts_a_type(next: Option<&TokenKind>) -> bool {
    matches!(next, Some(TokenKind::Identifier))
}

fn expects_operand(kind: &TokenKind) -> bool {
    !matches!(
        kind,
        TokenKind::Identifier
            | TokenKind::Int(_)
            | TokenKind::Float(_)
            | TokenKind::Str(_)
            | TokenKind::True
            | TokenKind::False
            | TokenKind::RParen
            | TokenKind::RBracket
            | TokenKind::RBrace
    )
}

fn angled(previous: &TokenKind, next: &TokenKind) -> bool {
    matches!(previous, TokenKind::Lt | TokenKind::Gt)
        || matches!(next, TokenKind::Lt | TokenKind::Gt | TokenKind::GtEq)
}

fn opens(kind: &TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace
    )
}

fn closes(kind: &TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace
    )
}

fn is_call_or_index(previous: &TokenKind, next: &TokenKind) -> bool {
    let callable = matches!(
        previous,
        TokenKind::Identifier | TokenKind::RParen | TokenKind::RBracket
    );

    callable && matches!(next, TokenKind::LParen | TokenKind::LBracket)
}

pub(super) fn keyword(kind: &TokenKind) -> &'static str {
    match kind {
        TokenKind::And => "and",
        TokenKind::Break => "break",
        TokenKind::Continue => "continue",
        TokenKind::Else => "else",
        TokenKind::False => "false",
        TokenKind::For => "for",
        TokenKind::Function => "function",
        TokenKind::If => "if",
        TokenKind::Import => "import",
        TokenKind::In => "in",
        TokenKind::Not => "not",
        TokenKind::Or => "or",
        TokenKind::Return => "return",
        TokenKind::True => "true",
        TokenKind::While => "while",
        TokenKind::Plus => "+",
        TokenKind::Minus => "-",
        TokenKind::Star => "*",
        TokenKind::Slash => "/",
        TokenKind::Percent => "%",
        TokenKind::Eq => "=",
        TokenKind::EqEq => "==",
        TokenKind::BangEq => "!=",
        TokenKind::Lt => "<",
        TokenKind::Gt => ">",
        TokenKind::LtEq => "<=",
        TokenKind::GtEq => ">=",
        TokenKind::Arrow => "->",
        TokenKind::LParen => "(",
        TokenKind::RParen => ")",
        TokenKind::LBracket => "[",
        TokenKind::RBracket => "]",
        TokenKind::LBrace => "{",
        TokenKind::RBrace => "}",
        TokenKind::Comma => ",",
        TokenKind::Colon => ":",
        _ => "",
    }
}

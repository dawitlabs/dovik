#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Identifier,
    Int(u64),
    Float(f64),
    Str(String),

    And,
    Break,
    Continue,
    Else,
    False,
    For,
    Function,
    If,
    In,
    Not,
    Or,
    Return,
    True,
    While,

    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Eq,
    EqEq,
    BangEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    Arrow,

    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    Comma,
    Colon,

    Newline,
    Indent,
    Dedent,
    Eof,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn text<'src>(&self, source: &'src str) -> &'src str {
        &source[self.span.start..self.span.end]
    }
}

pub fn keyword(text: &str) -> Option<TokenKind> {
    Some(match text {
        "and" => TokenKind::And,
        "break" => TokenKind::Break,
        "continue" => TokenKind::Continue,
        "else" => TokenKind::Else,
        "false" => TokenKind::False,
        "for" => TokenKind::For,
        "function" => TokenKind::Function,
        "if" => TokenKind::If,
        "in" => TokenKind::In,
        "not" => TokenKind::Not,
        "or" => TokenKind::Or,
        "return" => TokenKind::Return,
        "true" => TokenKind::True,
        "while" => TokenKind::While,
        _ => return None,
    })
}

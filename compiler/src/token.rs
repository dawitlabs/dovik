#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

impl TokenKind {
    pub fn describe(&self) -> &'static str {
        match self {
            Self::Identifier => "an identifier",
            Self::Int(_) => "an integer literal",
            Self::Float(_) => "a float literal",
            Self::Str(_) => "a string literal",
            Self::And => "`and`",
            Self::Break => "`break`",
            Self::Continue => "`continue`",
            Self::Else => "`else`",
            Self::False => "`false`",
            Self::For => "`for`",
            Self::Function => "`function`",
            Self::If => "`if`",
            Self::In => "`in`",
            Self::Not => "`not`",
            Self::Or => "`or`",
            Self::Return => "`return`",
            Self::True => "`true`",
            Self::While => "`while`",
            Self::Plus => "`+`",
            Self::Minus => "`-`",
            Self::Star => "`*`",
            Self::Slash => "`/`",
            Self::Percent => "`%`",
            Self::Eq => "`=`",
            Self::EqEq => "`==`",
            Self::BangEq => "`!=`",
            Self::Lt => "`<`",
            Self::Gt => "`>`",
            Self::LtEq => "`<=`",
            Self::GtEq => "`>=`",
            Self::Arrow => "`->`",
            Self::LParen => "`(`",
            Self::RParen => "`)`",
            Self::LBracket => "`[`",
            Self::RBracket => "`]`",
            Self::LBrace => "`{`",
            Self::RBrace => "`}`",
            Self::Comma => "`,`",
            Self::Colon => "`:`",
            Self::Newline => "a line break",
            Self::Indent => "an indent",
            Self::Dedent => "a dedent",
            Self::Eof => "end of file",
        }
    }
}

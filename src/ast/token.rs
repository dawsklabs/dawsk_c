use std::fmt::{ Display, Formatter, Result };
use super::types::TypeKind;

#[derive(Debug, Clone, PartialEq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub fn len(&self) -> usize {
        self.end - self.start
    }

    // pub fn slice<'a>(&self, src: &'a str) -> &'a str {
    //     &src[self.start..self.end]
    // }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Keywords
    Keyword(Keyword),
    
    // Types
    Type(TypeKind),
    Identifier(String),
    Integer(i64),
    Float(f64),
    Char(char),
    String(String),
    Bool(bool),
    Unknown(char),
    
    // Operators
    Plus,
    Minus,
    Asterisk,
    Slash,
    Percent,

    PlusEquals,
    MinusEquals,
    AsteriskEquals,
    SlashEquals,

    Equals,
    DoubleEquals,
    ExclamationEquals,

    And,
    DoubleAnd,
    Pipe,
    DoublePipe,
    Caret,
    Tilde,

    // Punctuation
    Dot,
    Comma,
    Colon,
    Semicolon,
    Exclamation,
    Question,
    Underscore,

    // Brackets
    LParen,
    RParen,
    LBracket,
    RBracket,
    LCurly,
    RCurly,

    LAngle,
    DoubleLAngle,
    LAngleEquals,

    RAngle,
    DoubleRAngle,
    RAngleEquals,

    // Specials
    EOF,
}

impl Display for TokenKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            TokenKind::Type(t) => write!(f, "Type ({})", t),
            TokenKind::Integer(_) => write!(f, "Int"),
            TokenKind::Float(_) => write!(f, "Float"),
            TokenKind::String(_) => write!(f, "String"),
            TokenKind::Char(_) => write!(f, "Char"),
            TokenKind::Identifier(i) => write!(f, "Identifier ({})", i),
            TokenKind::Unknown(_) => write!(f, "<Unknown>"),
            _ => write!(f, "{:?}", self),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Keyword {
    Dec,
    Pub,
    Mut,
    Struct,
    Impl,
    Self_,
    Type,
    Enum,
    Trait,
    Func,
    True,
    False,
    As,
    For,
    While,
    If,
    Else,
    In,
}
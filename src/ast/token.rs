use crate::{source::Span /* types::Ty */};
use std::fmt::{Display, Formatter, Result};

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
    // Type(Ty),
    Identifier(String),
    Integer(u128),
    Float(f64),
    Byte(u8),
    Char(char),
    String(String),
    ByteString(Vec<u8>),
    RawString(String),
    RawByteString(Vec<u8>),
    Bool(bool),
    Unknown(char),

    // Operators
    Plus,
    Minus,
    Asterisk,
    Slash,
    Percent,

    DoublePlus,
    DoubleMinus,

    PlusEquals,
    MinusEquals,
    AsteriskEquals,
    SlashEquals,

    Equals,
    DoubleEquals,
    ExclamationEquals,

    And,
    Pipe,
    Caret,
    DoubleAnd,
    DoublePipe,

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

    // Path
    DoubleColon,

    // Specials
    Error,
    EOF,
}

impl Display for TokenKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            // TokenKind::Type(_) => write!(f, "TYPE"),
            TokenKind::Integer(_) => write!(f, "INT"),
            TokenKind::Float(_) => write!(f, "FLOAT"),
            TokenKind::String(_) => write!(f, "STR"),
            TokenKind::Char(_) => write!(f, "CHAR"),
            TokenKind::Identifier(_) => write!(f, "IDENTIFIER"),
            TokenKind::Unknown(_) => write!(f, "<UNKNOWN>"),
            _ => write!(f, "{}", format!("{:?}", self).to_uppercase()),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Keyword {
    Dec,
    Pub,
    Mut,
    Const,
    Struct,
    Extend,
    Inst,
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

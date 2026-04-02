use crate::{ast::macros::SyntaxContext, source::Span /* types::Ty */};
use std::fmt::{Display, Formatter, Result};

#[derive(Debug, Clone, PartialEq)]
pub enum NumSuffix {
    // float
    F32,
    F64,
    // signed int
    I8,
    I16,
    I32,
    I64,
    I128,
    Isize,
    // unsigned int
    U8,
    U16,
    U32,
    U64,
    U128,
    Usize,
}

impl NumSuffix {
    pub fn is_float(&self) -> bool {
        matches!(self, Self::F32 | Self::F64)
    }

    pub fn is_integer(&self) -> bool {
        !self.is_float()
    }
}

impl NumSuffix {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "f32" => Some(Self::F32),
            "f64" => Some(Self::F64),
            "i8" => Some(Self::I8),
            "i16" => Some(Self::I16),
            "i32" => Some(Self::I32),
            "i64" => Some(Self::I64),
            "i128" => Some(Self::I128),
            "isize" => Some(Self::Isize),
            "u8" => Some(Self::U8),
            "u16" => Some(Self::U16),
            "u32" => Some(Self::U32),
            "u64" => Some(Self::U64),
            "u128" => Some(Self::U128),
            "usize" => Some(Self::Usize),
            _ => None,
        }
    }
}

impl Display for NumSuffix {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let s = match self {
            Self::F32 => "f32",
            Self::F64 => "f64",
            Self::I8 => "i8",
            Self::I16 => "i16",
            Self::I32 => "i32",
            Self::I64 => "i64",
            Self::I128 => "i128",
            Self::Isize => "isize",
            Self::U8 => "u8",
            Self::U16 => "u16",
            Self::U32 => "u32",
            Self::U64 => "u64",
            Self::U128 => "u128",
            Self::Usize => "usize",
        };
        write!(f, "{s}")
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
    pub ctx: SyntaxContext, // neu
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Keywords
    Keyword(Keyword),

    // Types
    // Type(Ty),
    Identifier(String),
    Integer(u128, Option<NumSuffix>),
    Float(f64, Option<NumSuffix>),
    Byte(u8),
    Char(char),
    String(String),
    ByteString(Vec<u8>),
    RawString(String),
    RawByteString(Vec<u8>),

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

    // Arrows
    Arrow,
    FatArrow,

    // Macros
    Dollar,

    // Specials
    Error,
    EndOfFile,
}

impl Display for TokenKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            // TokenKind::Type(_) => write!(f, "TYPE"),
            TokenKind::Integer(..) => write!(f, "INT"),
            TokenKind::Float(..) => write!(f, "FLOAT"),
            TokenKind::String(_) => write!(f, "STR"),
            TokenKind::Char(_) => write!(f, "CHAR"),
            TokenKind::Identifier(_) => write!(f, "IDENTIFIER"),
            TokenKind::EndOfFile => write!(f, "EOF"),
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
    Macro,
    True,
    False,
    As,
    For,
    While,
    If,
    Else,
    In,
}

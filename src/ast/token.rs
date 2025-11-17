use std::fmt::{ Display, Formatter, Result };

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

    pub fn slice<'a>(&self, src: &'a str) -> &'a str {
        &src[self.start..self.end]
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Position {
    pub line: usize,
    pub span: Span, // column: start..end
    pub file: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub pos: Position,
}

impl Token {
    pub fn new(kind: TokenKind, pos: Position) -> Self {
        Self { kind, pos }
    }

    pub fn pos(&self) {
        todo!()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Keywords
    Keyword(Keyword),
    Marker(Marker),

    // Types
    Identifier(String),
    Integer(i64),
    Float(f64),
    Char(char),
    String(String),
    Bool(bool),

    // Operators
    Plus,
    Minus,
    Asterisk,
    Slash,
    Modulus,

    And,
    Pipe,
    Caret,
    Tilde,

    Equals,

    // Punctuation
    Dot,
    Comma,
    Colon,
    Semicolon,
    Exclamation,
    Question,

    // Brackets
    LParen,
    RParen,
    LBracket,
    RBracket,
    LCurly,
    RCurly,
    LAngle,
    RAngle,

    Underscore,

    // Specials
    EOF,
    Unknown(char),
}

impl Display for TokenKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            TokenKind::Keyword(k) => write!(f, "Keyword ({})", k),
            TokenKind::Identifier(i) => write!(f, "Identifier ({})", i),
            _ => write!(f, ""),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Keyword {
    Dec,
    Decex,
    Ex,
    Mod,
    Struct,
    Impl,
    SelfKw,
    Type,
    Enum,
    Func,
    As,
    For,
    While,
    If,
    Else,
    In,
}

impl Display for Keyword {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Keyword::Dec => write!(f, "dec"),
            Keyword::Mod => write!(f, "mod"),
            _ => write!(f, "not yet implemented"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Marker {
    Override,
}

// pub enum FloatType {
//     F32(f32),
//     F64(f64),
// }

// pub enum IntType {
//     I8(i8),
//     I16(i16),
//     I32(i32),
//     I64(i64),
// }
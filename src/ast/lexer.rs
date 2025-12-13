use super::token::{Token, TokenKind, Span, Keyword};
use crate::file;

#[derive(Clone)]
pub struct Lexer {
    input: Vec<u8>,
    position: usize,
}

impl Lexer {
    pub fn new() -> Self {
        Self {
            input: file::content(),
            position: 0,
        }
    }

    fn create_token(&mut self, kind: TokenKind, start: usize, end: usize) -> Token {
        Token {
            kind,
            span: Span::new(start, end),
        }
    }

    fn peek(&self, n: usize) -> Option<u8> {
        self.input.get(self.position + n).copied()
    }

    fn advance(&mut self, n: usize) {
        for _ in 0..n {
            self.position += 1;
        }
    }

    fn advance_until(&mut self, pattern: &[u8]) {
        let pat_len = pattern.len();

        while self.position + pat_len <= self.input.len() {
            if &self.input[self.position..self.position + pat_len] == pattern {
                self.advance(pat_len);
                return;
            }
            self.advance(1);
        }
    }

    pub fn next_token(&mut self) -> Token {
        self.skip_whitespace();
        let start = self.position;

        let (kind, len) = match self.peek(0) {
            Some(c) if (c as char).is_ascii_alphabetic() => self.read_identifier_or_keyword(),
            // Some(c) if c == b'@' => self.read_marker(),
            Some(c) if (c as char).is_numeric() => self.read_number(),
            Some(b'+') => (TokenKind::Plus, 1),
            Some(b'-') => (TokenKind::Minus, 1),
            Some(b'*') => (TokenKind::Asterisk, 1),
            Some(b'/') => {
                // Comments
                if self.peek(1) == Some(b'/') {
                    self.advance(2);
                    self.advance_until(b"\n");
                    return self.next_token();
                } else if self.peek(1) == Some(b'*') {
                    self.advance(2);
                    self.advance_until(b"*/");
                    return self.next_token();
                } else {
                    (TokenKind::Slash, 1)
                }
            },
            Some(b'%') => (TokenKind::Percent, 1),
            Some(b'=') => (TokenKind::Equals, 1),
            Some(b'&') => (TokenKind::And, 1),
            Some(b'|') => (TokenKind::Pipe, 1),
            Some(b'^') => (TokenKind::Caret, 1),
            Some(b'(') => (TokenKind::LParen, 1),
            Some(b')') => (TokenKind::RParen, 1),
            Some(b'[') => (TokenKind::LBracket, 1),
            Some(b']') => (TokenKind::RBracket, 1),
            Some(b'{') => (TokenKind::LCurly, 1),
            Some(b'}') => (TokenKind::RCurly, 1),
            Some(b'<') => (TokenKind::LAngle, 1),
            Some(b'>') => (TokenKind::RAngle, 1),
            Some(b'!') => (TokenKind::Exclamation, 1),
            Some(b'?') => (TokenKind::Question, 1),
            Some(b'~') => (TokenKind::Tilde, 1),
            Some(b'.') => (TokenKind::Dot, 1),
            Some(b',') => (TokenKind::Comma, 1),
            Some(b':') => (TokenKind::Colon, 1),
            Some(b';') => (TokenKind::Semicolon, 1),
            Some(b'\'') => self.read_char(),
            Some(b'"') => self.read_string(),
            Some(b'_') => (TokenKind::Underscore, 1),
            Some(c) => (TokenKind::Unknown(c as char), 1),
            None => (TokenKind::EOF, 0),
        };
        self.advance(len);
        self.create_token(kind, start, self.position)
    }

    fn skip_whitespace(&mut self) {
        while let Some(&c) = self.input.get(self.position) {
            if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' {
                self.advance(1);
            } else {
                break;
            }
        }
    }

    fn read_identifier_or_keyword(&mut self) -> (TokenKind, usize) {
        let mut i = 0;

        // First character must be a letter
        if let Some(c) = self.peek(0) {
            if !(c as char).is_ascii_alphabetic() {
                panic!("Identifier must start with a letter, got '{}'", c as char);
            }
        }

        while let Some(c) = self.peek(i) {
            if (c as char).is_ascii_alphanumeric() || c == b'_' {
                i += 1;
            } else {
                break;
            }
        }

        let raw: String = self.input[self.position..self.position + i]
            .iter()
            .map(|&b| b as char)
            .collect();
        let kind = match raw.as_str() {
            "dec" => TokenKind::Keyword(Keyword::Dec), // e.g. dec mod x: i8 = 16;
            "mut" => TokenKind::Keyword(Keyword::Mut), // mutable
            "publy" => TokenKind::Keyword(Keyword::Publy), // publy = public
            "struct" => TokenKind::Keyword(Keyword::Struct),
            "impl" => TokenKind::Keyword(Keyword::Impl),
            "self" => TokenKind::Keyword(Keyword::Self_), // self reference
            "type" => TokenKind::Keyword(Keyword::Type),
            "enum" => TokenKind::Keyword(Keyword::Enum),
            "func" => TokenKind::Keyword(Keyword::Func),
            "as" => TokenKind::Keyword(Keyword::As), // casting
            "for" => TokenKind::Keyword(Keyword::For),
            "while" => TokenKind::Keyword(Keyword::While),
            "if" => TokenKind::Keyword(Keyword::If),
            "else" => TokenKind::Keyword(Keyword::Else),
            "in" => TokenKind::Keyword(Keyword::In),
            "false" => TokenKind::Keyword(Keyword::False),
            "true" => TokenKind::Keyword(Keyword::True),
            _ => TokenKind::Identifier(raw.clone()),
        };

        (kind, i)
    }

    fn read_number(&mut self) -> (TokenKind, usize) {
        if self.peek(0) == Some(b'0') && matches!(self.peek(1), Some(b'x') | Some(b'X')) {
            self.advance(2);
            return self.read_hex_integer();
        }
        self.read_decimal_or_scientific()
    }

    fn read_hex_integer(&mut self) -> (TokenKind, usize) {
        let mut i = 0;
        while let Some(c) = self.peek(i) {
            if (c as char).is_digit(16) || c == b'_' {
                i += 1;
            } else { break; }
        }
        let raw: String = self.input[self.position..self.position + i].iter().map(|&b| b as char).collect();
        let value = i64::from_str_radix(&raw.replace('_', ""), 16).unwrap();
        (TokenKind::Integer(value), i)
    }

    fn read_decimal_or_scientific(&mut self) -> (TokenKind, usize) {
        let mut i = 0;
        let mut has_dot = false;

        while let Some(c) = self.peek(i) {
            if (c as char).is_numeric() || c == b'_' {
                i += 1;
            } else if c == b'.' && !has_dot {
                has_dot = true;
                i += 1;
            } else if c == b'e' || c == b'E' {
                i += 1;
                if let Some(sign) = self.peek(i) {
                    if sign == b'+' || sign == b'-' { i += 1; }
                }
            } else { break; }
        }

        let raw: String = self.input[self.position..self.position + i].iter().map(|&b| b as char).collect();
        let s = raw.replace('_', "");
        if has_dot || s.contains('e') || s.contains('E') {
            (TokenKind::Float(s.parse::<f64>().unwrap()), i)
        } else {
            (TokenKind::Integer(s.parse::<i64>().unwrap()), i)
        }
    }

    fn read_char(&mut self) -> (TokenKind, usize) {
        let mut i = 1;

        let c = match self.peek(i) {
            Some(b'\\') => {
                i += 1;
                match self.peek(i) {
                    Some(b'n') => { i += 1; '\n' },
                    Some(b't') => { i += 1; '\t' },
                    Some(b'r') => { i += 1; '\r' },
                    Some(b'\\') => { i += 1; '\\' },
                    Some(b'"') => { i += 1; '"' },
                    Some(b'\'') => { i += 1; '\'' },
                    Some(b'u') => {
                        // Unicode escape \u{XXXX}
                        i += 1;
                        if self.peek(i) != Some(b'{') {
                            panic!("Invalid unicode escape");
                        }
                        i += 1;
                        let mut codepoint = String::new();
                        while let Some(ch) = self.peek(0) {
                            if ch == b'}' { break; }
                            codepoint.push(ch as char);
                            i += 1;
                        }
                        if self.peek(i) != Some(b'}') {
                            panic!("Unterminated unicode escape");
                        }
                        i += 1;
                        std::char::from_u32(u32::from_str_radix(&codepoint, 16).unwrap())
                            .expect("Invalid unicode codepoint")
                    },
                    Some(other) => { i += 1; other as char },
                    None => panic!("Unvollständiges Escape-Zeichen"),
                }
            },
            Some(c) => { i += 1; c as char },
            None => panic!("Unvollständiges Char-Literal"),
        };

        if self.peek(i) != Some(b'\'') {
            panic!("Char-Literal muss genau ein Zeichen enthalten");
        }
        i += 1; // skip closing '

        (TokenKind::Char(c), i)
    }

    fn read_string(&mut self) -> (TokenKind, usize) {
        let mut i = 1;

        let mut result = String::new();
        while let Some(c) = self.peek(i) {
            if c == b'"' {
                i += 1;
                break;
            }
            if c == b'\\' {
                i += 1;
                let esc = match self.peek(i).unwrap() {
                    b'n' => '\n',
                    b't' => '\t',
                    b'r' => '\r',
                    b'\\' => '\\',
                    b'"' => '"',
                    b'\'' => '\'',
                    other => other as char,
                };
                i += 1;
                result.push(esc);
            } else {
                i += 1;
                result.push(c as char);
            }
        }

        (TokenKind::String(result), i)
    }
}

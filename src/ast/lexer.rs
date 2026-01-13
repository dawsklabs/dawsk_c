use super::token::{Keyword, Span, Token, TokenKind};
use crate::diagnostics::{DiagnosticBagCell, DiagnosticBuilder, DiagnosticKind};
use crate::file;

#[derive(Clone)]
pub struct Lexer {
    input: Vec<u8>,
    position: usize,
    diagnostics_bag: DiagnosticBagCell,
}

impl Lexer {
    pub fn new(diagnostics_bag: DiagnosticBagCell) -> Self {
        Self {
            input: file::content(),
            position: 0,
            diagnostics_bag,
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
        self.position += n;
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
            // raw string r"..."
            Some(b'r') if self.peek(1) == Some(b'#') && self.peek(2) == Some(b'"') => {
                self.read_raw_string()
            }

            // byte string / char / raw byte string
            Some(b'b') => match self.peek(1) {
                Some(b'r') if self.peek(2) == Some(b'#') && self.peek(3) == Some(b'"') => {
                    self.read_raw_byte_string()
                } // br"..." zuerst prüfen
                Some(b'"') => self.read_byte_string(), // b"..." Byte-String
                Some(b'\'') => self.read_byte_char(),  // b'...' Byte-Char
                _ => self.read_identifier_or_keyword(),
            },

            // Operatoren und Symbole
            Some(b'+') => match self.peek(1) {
                Some(b'+') => (TokenKind::DoublePlus, 2),
                Some(b'=') => (TokenKind::PlusEquals, 2),
                _ => (TokenKind::Plus, 1),
            },
            Some(b'-') => match self.peek(1) {
                Some(b'-') => (TokenKind::DoubleMinus, 2),
                Some(b'=') => (TokenKind::MinusEquals, 2),
                _ => (TokenKind::Minus, 1),
            },
            Some(b'*') => match self.peek(1) {
                Some(b'=') => (TokenKind::AsteriskEquals, 2),
                _ => (TokenKind::Asterisk, 1),
            },
            Some(b'/') => match self.peek(1) {
                Some(b'/') => {
                    self.advance(2);
                    self.advance_until(b"\n");
                    return self.next_token();
                }
                Some(b'*') => {
                    self.advance(2);
                    self.advance_until(b"*/");
                    return self.next_token();
                }
                Some(b'=') => (TokenKind::SlashEquals, 2),
                _ => (TokenKind::Slash, 1),
            },
            Some(b'%') => (TokenKind::Percent, 1),
            Some(b'=') => match self.peek(1) {
                Some(b'=') => (TokenKind::DoubleEquals, 2),
                _ => (TokenKind::Equals, 1),
            },
            Some(b'&') => match self.peek(1) {
                Some(b'&') => (TokenKind::DoubleAnd, 2),
                _ => (TokenKind::And, 1),
            },
            Some(b'|') => match self.peek(1) {
                Some(b'|') => (TokenKind::DoublePipe, 2),
                _ => (TokenKind::Pipe, 1),
            },
            Some(b'^') => (TokenKind::Caret, 1),
            Some(b'(') => (TokenKind::LParen, 1),
            Some(b')') => (TokenKind::RParen, 1),
            Some(b'[') => (TokenKind::LBracket, 1),
            Some(b']') => (TokenKind::RBracket, 1),
            Some(b'{') => (TokenKind::LCurly, 1),
            Some(b'}') => (TokenKind::RCurly, 1),
            Some(b'<') => match self.peek(1) {
                Some(b'<') => (TokenKind::DoubleLAngle, 2),
                Some(b'=') => (TokenKind::LAngleEquals, 2),
                _ => (TokenKind::LAngle, 1),
            },
            Some(b'>') => match self.peek(1) {
                Some(b'>') => (TokenKind::DoubleRAngle, 2),
                Some(b'=') => (TokenKind::RAngleEquals, 2),
                _ => (TokenKind::RAngle, 1),
            },
            Some(b'!') => match self.peek(1) {
                Some(b'!') => {
                    let span = Span::new(self.position, self.position + 2);
                    self.diagnostics_bag.push(
                        DiagnosticBuilder::error(DiagnosticKind::UnknownCharacter, span.clone())
                            .label(span.clone(), "Double exclamation mark is not allowed!")
                            .note("Did you mean to use a single exclamation mark? Or wrap the inner expression in parentheses.")
                            .build(),
                    );
                    self.advance(1);
                    return self.create_token(TokenKind::Error, span.start, span.end);
                }
                Some(b'=') => (TokenKind::ExclamationEquals, 2),
                _ => (TokenKind::Exclamation, 1),
            },
            Some(b'?') => (TokenKind::Question, 1),
            Some(b'.') => (TokenKind::Dot, 1),
            Some(b',') => (TokenKind::Comma, 1),
            Some(b':') => (TokenKind::Colon, 1),
            Some(b';') => (TokenKind::Semicolon, 1),

            Some(b'"') => self.read_string(), // normale Strings
            Some(b'\'') => self.read_char(),  // Char-Literal

            Some(b'_') => (TokenKind::Underscore, 1),

            Some(c) if (c as char).is_ascii_alphabetic() => self.read_identifier_or_keyword(),
            Some(c) if (c as char).is_numeric() => self.read_number(),

            Some(_) => {
                let span = Span::new(self.position, self.position + 1);
                self.diagnostics_bag.push(
                    DiagnosticBuilder::error(DiagnosticKind::UnknownCharacter, span.clone())
                        .label(span.clone(), "Unknown character!")
                        .build(),
                );
                self.advance(1);
                return self.create_token(TokenKind::Error, span.start, span.end);
            }

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
            if !(c as char).is_ascii_alphabetic() && c != b'_' {
                let span = Span::new(self.position, self.position + 1);
                self.diagnostics_bag.push(
                    DiagnosticBuilder::error(DiagnosticKind::InvalidCharacter, span.clone())
                        .label(span, "Identifier must start with a letter or underscore")
                        .build(),
                )
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
            "dec" => TokenKind::Keyword(Keyword::Dec), // e.g. dec x: i8 = 16;
            "mut" => TokenKind::Keyword(Keyword::Mut), // mutable
            "pub" => TokenKind::Keyword(Keyword::Pub), // pub = public
            "struct" => TokenKind::Keyword(Keyword::Struct),
            "impl" => TokenKind::Keyword(Keyword::Impl),
            "self" => TokenKind::Keyword(Keyword::Self_), // self reference
            "type" => TokenKind::Keyword(Keyword::Type),
            "enum" => TokenKind::Keyword(Keyword::Enum),
            "trait" => TokenKind::Keyword(Keyword::Trait), // trait = interface
            "func" => TokenKind::Keyword(Keyword::Func),
            "as" => TokenKind::Keyword(Keyword::As), // casting
            "for" => TokenKind::Keyword(Keyword::For),
            "while" => TokenKind::Keyword(Keyword::While),
            "if" => TokenKind::Keyword(Keyword::If),
            "else" => TokenKind::Keyword(Keyword::Else),
            "in" => TokenKind::Keyword(Keyword::In),
            "false" => TokenKind::Keyword(Keyword::False),
            "true" => TokenKind::Keyword(Keyword::True),
            _ => TokenKind::Identifier(Box::leak(raw.clone().into_boxed_str())),
        };

        (kind, i)
    }

    fn read_number(&mut self) -> (TokenKind, usize) {
        if self.peek(0) == Some(b'0') && matches!(self.peek(1), Some(b'x') | Some(b'X')) {
            // self.advance(2);
            return self.read_hex_integer();
        }
        self.read_decimal_or_scientific()
    }

    fn read_hex_integer(&mut self) -> (TokenKind, usize) {
        let mut i = 2;
        while let Some(c) = self.peek(i) {
            if (c as char).is_digit(16) {
                i += 1;
            } else {
                break;
            }
        }
        let raw: String = self.input[self.position..self.position + i]
            .iter()
            .map(|&b| b as char)
            .collect();
        let value = u64::from_str_radix(&raw, 16).unwrap();
        (TokenKind::Integer(value), i)
    }

    fn read_decimal_or_scientific(&mut self) -> (TokenKind, usize) {
        let start = self.position;
        let mut i = 0;
        let mut has_dot = false;

        while let Some(c) = self.peek(i) {
            match c {
                b'0'..=b'9' | b'_' => i += 1,
                b'.' if !has_dot => {
                    has_dot = true;
                    i += 1;
                }
                b'e' | b'E' => {
                    i += 1;
                    if matches!(self.peek(i), Some(b'+' | b'-')) {
                        i += 1;
                    }
                }
                _ => break,
            }
        }

        let raw = &self.input[start..start + i];
        let text: String = raw.iter().map(|&b| b as char).collect();

        if has_dot || text.contains('e') || text.contains('E') {
            match text.parse::<f64>() {
                Ok(v) => (TokenKind::Float(v), i),
                Err(_) => {
                    let span = Span::new(start, start + i);
                    self.diagnostics_bag.push(
                        DiagnosticBuilder::error(DiagnosticKind::InvalidValue, span.clone())
                            .label(span, "Invalid floating-point literal!")
                            .build(),
                    );
                    (TokenKind::Error, i)
                }
            }
        } else {
            match text.parse::<u64>() {
                Ok(v) => (TokenKind::Integer(v), i),
                Err(_) => {
                    let span = Span::new(start, start + i);
                    self.diagnostics_bag.push(
                        DiagnosticBuilder::error(DiagnosticKind::InvalidValue, span.clone())
                            .label(span, "Invalid integer literal!")
                            .build(),
                    );
                    (TokenKind::Error, i)
                }
            }
        }
    }

    fn read_char(&mut self) -> (TokenKind, usize) {
        let start = self.position;
        let mut i = 1; // skip opening '

        let c = match self.peek(i) {
            Some(b'\\') => {
                i += 1;
                match self.peek(i) {
                    Some(b'n') => '\n',
                    Some(b't') => '\t',
                    Some(b'r') => '\r',
                    Some(b'\\') => '\\',
                    Some(b'\'') => '\'',
                    Some(b'"') => '"',
                    Some(other) => other as char, // unknown escape, interpret as literal
                    None => {
                        // EOF nach \
                        let span = Span::new(start, start + i);
                        self.diagnostics_bag.push(
                            DiagnosticBuilder::error(DiagnosticKind::InvalidValue, span.clone())
                                .label(
                                    span.clone(),
                                    "unterminated or invalid escape in char literal",
                                )
                                .build(),
                        );
                        return (TokenKind::Error, i);
                    }
                }
            }
            Some(byte) => byte as char,
            None => {
                // EOF direkt nach '
                let span = Span::new(start, start + i);
                self.diagnostics_bag.push(
                    DiagnosticBuilder::error(DiagnosticKind::InvalidValue, span.clone())
                        .label(span.clone(), "unterminated char literal")
                        .build(),
                );
                return (TokenKind::Error, i);
            }
        };

        i += 1; // move past the character or escape

        // Check closing '
        if self.peek(i) == Some(b'\'') {
            i += 1;
            (TokenKind::Char(c), i)
        } else {
            let span = Span::new(start, start + i);
            self.diagnostics_bag.push(
                DiagnosticBuilder::error(DiagnosticKind::InvalidValue, span.clone())
                    .label(span.clone(), "unterminated char literal")
                    .build(),
            );
            (TokenKind::Error, i)
        }
    }

    fn read_byte_char(&mut self) -> (TokenKind, usize) {
        let start = self.position;
        let mut i = 2; // b'

        let value = match self.peek(i) {
            Some(b'\\') => {
                i += 1;
                let v = match self.peek(i) {
                    Some(b'n') => b'\n',
                    Some(b't') => b'\t',
                    Some(b'r') => b'\r',
                    Some(b'\\') => b'\\',
                    Some(b'\'') => b'\'',
                    Some(other) => other,
                    None => return (TokenKind::Error, i),
                };
                i += 1;
                v
            }
            Some(c) => {
                i += 1;
                c
            }
            None => return (TokenKind::Error, i),
        };

        if self.peek(i) == Some(b'\'') {
            i += 1;
            return (TokenKind::Byte(value), i);
        }

        let span = Span::new(start, start + i);
        self.diagnostics_bag.push(
            DiagnosticBuilder::error(DiagnosticKind::InvalidValue, span.clone())
                .label(span, "invalid byte char literal")
                .build(),
        );

        (TokenKind::Error, i)
    }

    fn read_string(&mut self) -> (TokenKind, usize) {
        let start = self.position;
        let mut i = 1;
        let mut result = String::new();

        while let Some(c) = self.peek(i) {
            match c {
                b'"' => {
                    i += 1;
                    return (TokenKind::String(result), i);
                }
                b'\n' => break,
                b'\\' => {
                    i += 1;
                    let esc = match self.peek(i) {
                        Some(b'n') => '\n',
                        Some(b't') => '\t',
                        Some(b'r') => '\r',
                        Some(b'\\') => '\\',
                        Some(b'"') => '"',
                        Some(other) => other as char, // unknown escape
                        None => break,
                    };
                    result.push(esc);
                    i += 1;
                }
                b'\'' => {
                    // <-- einfach ins String-Ergebnis pushen
                    result.push('\'');
                    i += 1;
                }
                other => {
                    result.push(other as char);
                    i += 1;
                }
            }
        }

        let span = Span::new(start, start + i);
        self.diagnostics_bag.push(
            DiagnosticBuilder::error(DiagnosticKind::InvalidValue, span.clone())
                .label(span, "unterminated string literal")
                .build(),
        );

        (TokenKind::Error, i)
    }

    fn read_raw_string(&mut self) -> (TokenKind, usize) {
        let start = self.position;
        let mut i = 2; // r"

        let mut result = String::new();
        while let Some(c) = self.peek(i) {
            match c {
                b'\n' => break,
                b'\'' => {
                    // <-- einfach ins String-Ergebnis pushen
                    result.push('\'');
                    i += 1;
                }
                b'"' if self.peek(i + 1) == Some(b'"') => {
                    i += 2;
                    return (TokenKind::RawString(result), i);
                }
                other => {
                    result.push(other as char);
                    i += 1;
                }
            }
        }

        let span = Span::new(start, start + i);
        self.diagnostics_bag.push(
            DiagnosticBuilder::error(DiagnosticKind::InvalidValue, span.clone())
                .label(span, "unterminated raw string literal")
                .build(),
        );

        (TokenKind::Error, i)
    }

    fn read_byte_string(&mut self) -> (TokenKind, usize) {
        let start = self.position;
        let mut i = 2; // b"
        let mut bytes = Vec::new();

        while let Some(c) = self.peek(i) {
            match c {
                b'\n' => break,
                b'\\' => {
                    i += 1;
                    let esc = match self.peek(i) {
                        Some(b'n') => b'\n',
                        Some(b't') => b'\t',
                        Some(b'r') => b'\r',
                        Some(b'\\') => b'\\',
                        Some(b'"') => b'"',
                        Some(other) => other, // unknown escape
                        None => break,
                    };
                    bytes.push(esc);
                    i += 1;
                }
                b'\'' => {
                    // <-- einfach ins String-Ergebnis pushen
                    bytes.push(b'\'');
                    i += 1;
                }
                b'"' => {
                    i += 1;
                    return (TokenKind::ByteString(bytes), i);
                }
                other => {
                    bytes.push(other);
                    i += 1;
                }
            }
        }

        let span = Span::new(start, start + i);
        self.diagnostics_bag.push(
            DiagnosticBuilder::error(DiagnosticKind::InvalidValue, span.clone())
                .label(span, "unterminated byte string literal")
                .build(),
        );

        (TokenKind::Error, i)
    }

    fn read_raw_byte_string(&mut self) -> (TokenKind, usize) {
        let start = self.position;
        let mut i = 4; // br#"
        let mut bytes = Vec::new();

        while let Some(c) = self.peek(i) {
            if c == b'"' && self.peek(i + 1) == Some(b'#') {
                i += 2;
                return (TokenKind::RawByteString(bytes), i);
            }
            bytes.push(c);
            i += 1;
        }

        let span = Span::new(start, start + i);
        self.diagnostics_bag.push(
            DiagnosticBuilder::error(DiagnosticKind::InvalidValue, span.clone())
                .label(span, "unterminated raw byte string literal")
                .build(),
        );

        (TokenKind::Error, i)
    }
}

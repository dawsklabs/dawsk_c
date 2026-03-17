use std::collections::VecDeque;

use super::token::{Keyword, Token, TokenKind};
use crate::color::{RED_COLOR, YELLOW_COLOR};
use crate::reports::{Label, Report, ReportKind};
use crate::source::Span;
use crate::Compiler;

#[derive(Clone, Copy)]
pub enum LexMode {
    Default,
    Generic,
}

pub struct Lexer<'a> {
    pub compiler: &'a mut Compiler,
    mode: VecDeque<LexMode>,
    file_id: usize,
    pos: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(compiler: &'a mut Compiler, file_id: usize) -> Self {
        let mut mode = VecDeque::new();
        mode.push_back(LexMode::Default);
        Self {
            compiler,
            mode,
            file_id,
            pos: 0,
        }
    }

    // pub fn get_mode(&self) -> LexMode {
    //     *self.mode.back().unwrap()
    // }

    pub fn add_mode(&mut self, m: LexMode) {
        self.mode.push_back(m);
    }

    pub fn remove_mode(&mut self) {
        self.mode.pop_back();
    }

    fn file(&self) -> &crate::source::SourceFile {
        self.compiler.sourcemap.file(self.file_id)
    }

    fn input(&self) -> &[u8] {
        self.file().text.as_bytes()
    }

    fn create_token(&mut self, kind: TokenKind, start: usize, end: usize) -> Token {
        Token {
            kind,
            span: Span::new(start, end, self.file_id),
        }
    }

    fn peek(&self, n: usize) -> Option<u8> {
        self.input().get(self.pos + n).copied()
    }

    fn advance(&mut self, n: usize) {
        self.pos += n;
    }

    fn advance_until(&mut self, pattern: &[u8]) {
        let pat_len = pattern.len();

        while self.pos + pat_len <= self.input().len() {
            if &self.input()[self.pos..self.pos + pat_len] == pattern {
                self.advance(pat_len);
                return;
            }
            self.advance(1);
        }
    }

    pub fn next_token(&mut self) -> Token {
        self.skip_whitespace();
        let start = self.pos;

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
            Some(b'>') => {
                match (self.peek(1), self.mode.back()) {
                    // Im Generic-Mode: '>>' → zwei einzelne '>'
                    (Some(b'>'), Some(LexMode::Generic)) => (TokenKind::RAngle, 1), // nur das erste '>' als Token
                    (Some(b'>'), _) => (TokenKind::DoubleRAngle, 2), // sonst DoubleRAngle
                    (Some(b'='), _) => (TokenKind::RAngleEquals, 2),
                    _ => (TokenKind::RAngle, 1),
                }
            }
            Some(b'!') => match self.peek(1) {
                Some(b'=') => (TokenKind::ExclamationEquals, 2),
                _ => (TokenKind::Exclamation, 1),
            },
            Some(b'?') => (TokenKind::Question, 1),
            Some(b'.') => (TokenKind::Dot, 1),
            Some(b',') => (TokenKind::Comma, 1),
            Some(b':') => match self.peek(1) {
                Some(b':') => (TokenKind::DoubleColon, 2),
                _ => (TokenKind::Colon, 1),
            },
            Some(b';') => (TokenKind::Semicolon, 1),

            Some(b'"') => self.read_string(), // normale Strings
            Some(b'\'') => self.read_char(),  // Char-Literal

            Some(b'_') => (TokenKind::Underscore, 1),

            Some(c) if (c as char).is_ascii_alphabetic() => self.read_identifier_or_keyword(),
            Some(c) if (c as char).is_numeric() => self.read_number(),

            Some(_) => {
                let span = Span::new(self.pos, self.pos + 1, self.file_id);
                self.compiler.shared.reports.push(
                    Report::build(ReportKind::Error, span)
                        .with_label(
                            Label::new(span)
                                .with_message("unknown or non ascii character")
                                .with_color(RED_COLOR),
                        )
                        .finish(),
                );
                self.advance(1);
                return self.create_token(TokenKind::Error, span.start, span.end);
            }

            None => (TokenKind::EndOfFile, 0),
        };

        self.advance(len);
        self.create_token(kind, start, self.pos)
    }

    fn skip_whitespace(&mut self) {
        while let Some(&c) = self.input().get(self.pos) {
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
                let span = Span::new(self.pos, self.pos + 1, self.file_id);
                self.compiler.shared.reports.push(
                    Report::build(ReportKind::Error, span)
                        .with_message("invalid identifier start")
                        .with_label(
                            Label::new(span)
                                .with_message("must start with a letter or underscore")
                                .with_color(RED_COLOR),
                        )
                        .finish(),
                );
            }
        }

        while let Some(c) = self.peek(i) {
            if (c as char).is_ascii_alphanumeric() || c == b'_' {
                i += 1;
            } else {
                break;
            }
        }

        let raw: String = self.input()[self.pos..self.pos + i]
            .iter()
            .map(|&b| b as char)
            .collect();
        let kind = match raw.as_str() {
            "dec" => TokenKind::Keyword(Keyword::Dec), // e.g. dec x: i8 = 16;
            "mut" => TokenKind::Keyword(Keyword::Mut), // mutable
            "pub" => TokenKind::Keyword(Keyword::Pub), // pub = public
            "const" => TokenKind::Keyword(Keyword::Const),
            "struct" => TokenKind::Keyword(Keyword::Struct),
            "extend" => TokenKind::Keyword(Keyword::Extend),
            "inst" => TokenKind::Keyword(Keyword::Inst), // instance reference
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
            _ => TokenKind::Identifier(raw),
        };

        (kind, i)
    }

    fn read_number(&mut self) -> (TokenKind, usize) {
        if self.peek(0) == Some(b'0') {
            match self.peek(1) {
                Some(b'x') | Some(b'X') => return self.read_radix_integer(16, 2),
                Some(b'o') | Some(b'O') => return self.read_radix_integer(8, 2),
                Some(b'b') | Some(b'B') => return self.read_radix_integer(2, 2),
                _ => {}
            }
        }

        self.read_decimal_or_scientific()
    }

    fn read_radix_integer(&mut self, radix: u32, prefix_len: usize) -> (TokenKind, usize) {
        let start = self.pos;
        let mut i = prefix_len;
        let mut saw_digit = false;
        let mut invalid_found = false;

        let mut first_digit_index: Option<usize> = None;
        let mut last_digit_index: Option<usize> = None;

        let mut underscore_positions = Vec::new(); // alle `_` speichern

        while let Some(c) = self.peek(i) {
            let ch = c as char;

            if ch == '_' {
                underscore_positions.push(i);
                i += 1;
                continue;
            }

            if ch.is_ascii_alphanumeric() {
                if ch.is_digit(radix) {
                    if first_digit_index.is_none() {
                        first_digit_index = Some(i);
                    }
                    last_digit_index = Some(i);
                } else {
                    invalid_found = true;
                }
                saw_digit = true;
                i += 1;
                continue;
            }

            break;
        }

        // ---------- WARNINGS ----------
        if let Some(first) = first_digit_index {
            if let Some(last) = last_digit_index {
                // Prefix underscores (zwischen Prefix und erster Ziffer)
                let leading_us: Vec<_> = underscore_positions
                    .iter()
                    .copied()
                    .filter(|&pos| pos < first)
                    .collect();
                if !leading_us.is_empty() {
                    let span = Span::new(
                        start + leading_us[0],
                        start + leading_us[leading_us.len() - 1] + 1,
                        self.file_id,
                    );

                    self.compiler.shared.reports.push(
                        Report::build(ReportKind::Warning, span)
                            .with_message("underscore directly after base prefix")
                            .with_label(Label::new(span).with_color(YELLOW_COLOR))
                            .with_help("consider: remove")
                            .finish(),
                    );
                }

                // Trailing underscores (nach letzter Ziffer)
                let trailing_us: Vec<_> = underscore_positions
                    .iter()
                    .copied()
                    .filter(|&pos| pos > last)
                    .collect();
                if !trailing_us.is_empty() {
                    let span = Span::new(
                        start + trailing_us[0],
                        start + trailing_us[trailing_us.len() - 1] + 1,
                        self.file_id,
                    );
                    self.compiler.shared.reports.push(
                        Report::build(ReportKind::Warning, span)
                            .with_message("trailing underscore in number literal")
                            .with_label(Label::new(span).with_color(YELLOW_COLOR))
                            .with_help("consider: remove")
                            .finish(),
                    );
                }

                // Mittlere underscores zwischen erster und letzter Ziffer
                let middle_us: Vec<_> = underscore_positions
                    .iter()
                    .copied()
                    .filter(|&p| p > first && p < last)
                    .collect();

                let mut groups = Vec::new();
                let mut group_start: Option<usize> = None;
                let mut prev: Option<usize> = None;

                for pos in middle_us {
                    if let Some(prev_pos) = prev {
                        if pos == prev_pos + 1 {
                            // fortlaufende Gruppe
                        } else {
                            // Gruppe endet
                            if let Some(start) = group_start {
                                groups.push((start, prev_pos));
                            }
                            group_start = Some(pos);
                        }
                    } else {
                        group_start = Some(pos);
                    }
                    prev = Some(pos);
                }

                // letzte Gruppe prüfen
                if let (Some(start), Some(end)) = (group_start, prev) {
                    groups.push((start, end));
                }

                let mut labels: Vec<Label<Span>> = Vec::new();

                for (s, e) in groups.iter() {
                    if e - s >= 1 {
                        let span = Span::new(start + s, start + e + 1, self.file_id);

                        labels.push(Label::new(span).with_color(YELLOW_COLOR));
                    }
                }

                if !labels.is_empty() {
                    self.compiler.shared.reports.push(
                        Report::build(ReportKind::Warning, labels[0].span)
                            .with_message("multiple consecutive underscores in number literal")
                            .with_labels(labels)
                            .with_help("consider: remove or reduce to one underscore")
                            .finish(),
                    );
                }
            }
        }

        // ---------- ERRORS ----------
        if !saw_digit {
            let span = Span::new(start, start + i, self.file_id);
            self.compiler.shared.reports.push(
                Report::build(ReportKind::Error, span)
                    .with_message("non-supported digit(s) found in numeric literal")
                    .with_label(
                        Label::new(span)
                            .with_message("non-supported digit(s)")
                            .with_color(RED_COLOR),
                    )
                    .finish(),
            );
            return (TokenKind::Error, i);
        }

        if invalid_found {
            let span = Span::new(start, start + i, self.file_id);
            self.compiler.shared.reports.push(
                Report::build(ReportKind::Error, span)
                    .with_message("invalid digit(s) in literal")
                    .with_label(
                        Label::new(span)
                            .with_message("invalid digit(s)")
                            .with_color(RED_COLOR),
                    )
                    .finish(),
            );
            return (TokenKind::Error, i);
        }

        // ---------- PARSE ----------
        let raw = &self.input()[start + prefix_len..start + i];
        let mut text: String = raw.iter().map(|&b| b as char).collect();
        text.retain(|c| c != '_');

        match u128::from_str_radix(&text, radix) {
            Ok(v) => (TokenKind::Integer(v), i),
            Err(_) => {
                let span = Span::new(start, start + i, self.file_id);
                self.compiler.shared.reports.push(
                    Report::build(ReportKind::Error, span)
                        .with_message("digits not storable")
                        .with_label(
                            Label::new(span)
                                .with_message("this number is too big")
                                .with_color(RED_COLOR),
                        )
                        .finish(),
                );
                (TokenKind::Error, i)
            }
        }
    }

    fn read_decimal_or_scientific(&mut self) -> (TokenKind, usize) {
        let start = self.pos;
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

        let raw = &self.input()[start..start + i];
        let mut text: String = raw.iter().map(|&b| b as char).collect();
        text.retain(|c| c != '_');

        if has_dot || text.contains('e') || text.contains('E') {
            match text.parse::<f64>() {
                Ok(v) => (TokenKind::Float(v), i),
                Err(_) => {
                    let span = Span::new(start, start + i, self.file_id);
                    self.compiler.shared.reports.push(
                        Report::build(ReportKind::Error, span)
                            .with_message("invalid floating-point literal")
                            .with_label(
                                Label::new(span)
                                    .with_message("not parsable as float")
                                    .with_color(RED_COLOR),
                            )
                            .finish(),
                    );
                    (TokenKind::Error, i)
                }
            }
        } else {
            match text.parse::<u128>() {
                Ok(v) => (TokenKind::Integer(v), i),
                Err(_) => {
                    let span = Span::new(start, start + i, self.file_id);
                    self.compiler.shared.reports.push(
                        Report::build(ReportKind::Error, span)
                            .with_message("invalid integer literal")
                            .with_label(
                                Label::new(span)
                                    .with_message("not parsable as integer")
                                    .with_color(RED_COLOR),
                            )
                            .finish(),
                    );
                    (TokenKind::Error, i)
                }
            }
        }
    }

    fn read_char(&mut self) -> (TokenKind, usize) {
        let start = self.pos;
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
                        // EndOfFile nach \
                        let span = Span::new(start, start + i, self.file_id);
                        self.compiler.shared.reports.push(
                            Report::build(ReportKind::Error, span)
                                .with_message("unterminated or invalid escape in char literal")
                                .with_label(Label::new(span).with_color(RED_COLOR))
                                .finish(),
                        );
                        return (TokenKind::Error, i);
                    }
                }
            }
            Some(byte) => byte as char,
            None => {
                // EndOfFile direkt nach '
                let span = Span::new(start, start + i, self.file_id);
                self.compiler.shared.reports.push(
                    Report::build(ReportKind::Error, span)
                        .with_message("unvalid char literal")
                        .with_label(
                            Label::new(span)
                                .with_message("not parsable as char (u8)")
                                .with_color(RED_COLOR),
                        )
                        .finish(),
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
            let span = Span::new(start, start + i, self.file_id);
            self.compiler.shared.reports.push(
                Report::build(ReportKind::Error, span)
                    .with_message("unterminated char literal")
                    .with_label(Label::new(span).with_color(RED_COLOR))
                    .finish(),
            );
            (TokenKind::Error, i)
        }
    }

    fn read_byte_char(&mut self) -> (TokenKind, usize) {
        let start = self.pos;
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

        let span = Span::new(start, start + i, self.file_id);
        self.compiler.shared.reports.push(
            Report::build(ReportKind::Error, span)
                .with_message("invalid byte char literal")
                .with_label(Label::new(span).with_color(RED_COLOR))
                .finish(),
        );

        (TokenKind::Error, i)
    }

    fn read_string(&mut self) -> (TokenKind, usize) {
        let start = self.pos;
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

        let span = Span::new(start, start + i, self.file_id);
        self.compiler.shared.reports.push(
            Report::build(ReportKind::Error, span)
                .with_message("unterminated string literal")
                .with_label(Label::new(span).with_color(RED_COLOR))
                .finish(),
        );

        (TokenKind::Error, i)
    }

    fn read_raw_string(&mut self) -> (TokenKind, usize) {
        let start = self.pos;
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
                b'"' => {
                    i += 1;
                    return (TokenKind::RawString(result), i);
                }
                other => {
                    result.push(other as char);
                    i += 1;
                }
            }
        }

        let span = Span::new(start, start + i, self.file_id);
        self.compiler.shared.reports.push(
            Report::build(ReportKind::Error, span)
                .with_message("unterminated string literal")
                .with_label(Label::new(span).with_color(RED_COLOR))
                .finish(),
        );

        (TokenKind::Error, i)
    }

    fn read_byte_string(&mut self) -> (TokenKind, usize) {
        let start = self.pos;
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

        let span = Span::new(start, start + i, self.file_id);
        self.compiler.shared.reports.push(
            Report::build(ReportKind::Error, span)
                .with_message("unterminated string literal")
                .with_label(Label::new(span).with_color(RED_COLOR))
                .finish(),
        );

        (TokenKind::Error, i)
    }

    fn read_raw_byte_string(&mut self) -> (TokenKind, usize) {
        let start = self.pos;
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

        let span = Span::new(start, start + i, self.file_id);
        self.compiler.shared.reports.push(
            Report::build(ReportKind::Error, span)
                .with_message("unterminated string literal")
                .with_label(Label::new(span).with_color(RED_COLOR))
                .finish(),
        );

        (TokenKind::Error, i)
    }
}

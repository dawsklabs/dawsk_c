pub mod token;

use std::collections::VecDeque;

use crate::color::{RED_COLOR, YELLOW_COLOR};
use crate::macros::SyntaxContext;
use crate::reports::{Label, Report, ReportKind};
use crate::source::{Span, SpanSource};
use crate::Compiler;
use token::{Keyword, NumSuffix, Token, TokenKind};

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
        self.file().source.text.as_bytes()
    }

    fn create_token(&mut self, kind: TokenKind, start: usize, end: usize) -> Token {
        Token {
            kind,
            span: Span::new(start, end, self.file_id, SpanSource::Source),
            ctx: SyntaxContext::ROOT,
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
                Some(b'>') => (TokenKind::Arrow, 2),
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
                Some(b'>') => (TokenKind::FatArrow, 2),
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

            Some(b'"') => self.read_string(), // normal string literal
            Some(b'\'') => self.read_char(),  // normal char literal

            Some(b'_') => (TokenKind::Underscore, 1),

            Some(b'$') => (TokenKind::Dollar, 1),

            Some(c) if (c as char).is_ascii_alphabetic() => self.read_identifier_or_keyword(),
            Some(c) if (c as char).is_numeric() => self.read_number(),

            Some(_) => {
                let span = Span::new(self.pos, self.pos + 1, self.file_id, SpanSource::Source);
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
                let span = Span::new(self.pos, self.pos + 1, self.file_id, SpanSource::Source);
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

        let raw = &self.input()[self.pos..self.pos + i];
        let kind = match raw {
            b"dec" => TokenKind::Keyword(Keyword::Dec), // e.g. dec x: i8 = 16;
            b"mut" => TokenKind::Keyword(Keyword::Mut), // mutable
            b"pub" => TokenKind::Keyword(Keyword::Pub), // pub = public
            b"const" => TokenKind::Keyword(Keyword::Const),
            b"struct" => TokenKind::Keyword(Keyword::Struct),
            b"extend" => TokenKind::Keyword(Keyword::Extend),
            b"inst" => TokenKind::Keyword(Keyword::Inst), // instance reference
            b"type" => TokenKind::Keyword(Keyword::Type),
            b"enum" => TokenKind::Keyword(Keyword::Enum),
            b"trait" => TokenKind::Keyword(Keyword::Trait), // trait = interface
            b"func" => TokenKind::Keyword(Keyword::Func),
            b"require" => TokenKind::Keyword(Keyword::Require),
            b"macro" => TokenKind::Keyword(Keyword::Macro),
            b"include" => TokenKind::Keyword(Keyword::Include),
            b"import" => TokenKind::Keyword(Keyword::Import),
            b"super" => TokenKind::Keyword(Keyword::Super),
            b"pkg" => TokenKind::Keyword(Keyword::Pkg),
            b"as" => TokenKind::Keyword(Keyword::As), // casting
            b"for" => TokenKind::Keyword(Keyword::For),
            b"while" => TokenKind::Keyword(Keyword::While),
            b"if" => TokenKind::Keyword(Keyword::If),
            b"else" => TokenKind::Keyword(Keyword::Else),
            b"in" => TokenKind::Keyword(Keyword::In),
            b"false" => TokenKind::Keyword(Keyword::False),
            b"true" => TokenKind::Keyword(Keyword::True),
            _ => {
                // Borrow von input() hier beenden durch owned String
                let s = std::str::from_utf8(raw).unwrap().to_owned();
                let _ = raw; // immutable borrow endet hier
                TokenKind::Identifier(self.compiler.string_pool.intern(&s))
            }
        };

        (kind, i)
    }

    fn read_num_suffix(&mut self, offset: usize) -> (Option<NumSuffix>, usize) {
        let mut len = 0;
        while let Some(c) = self.peek(offset + len) {
            if (c as char).is_ascii_alphanumeric() {
                len += 1;
            } else {
                break;
            }
        }
        if len == 0 {
            return (None, 0);
        }
        let raw = &self.input()[self.pos + offset..self.pos + offset + len];
        match NumSuffix::parse(raw) {
            Some(s) => (Some(s), len),
            None => {
                let span = Span::new(
                    self.pos + offset,
                    self.pos + offset + len,
                    self.file_id,
                    SpanSource::Source,
                );
                self.compiler.shared.reports.push(
                    Report::build(ReportKind::Error, span)
                        .with_message(format!(
                            "unknown numeric suffix `{}`",
                            std::str::from_utf8(raw).unwrap().to_owned()
                        ))
                        .with_label(Label::new(span).with_color(RED_COLOR))
                        .finish(),
                );
                (None, len)
            }
        }
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
            match c {
                b'_' => {
                    underscore_positions.push(i);
                }
                c if c.is_ascii_alphanumeric() => {
                    let is_valid = match radix {
                        2 => matches!(c, b'0'..=b'1'),
                        8 => matches!(c, b'0'..=b'7'),
                        16 => matches!(c, b'0'..=b'9' | b'a'..=b'f' | b'A'..=b'F'),
                        _ => c.is_ascii_digit(),
                    };
                    if is_valid {
                        first_digit_index.get_or_insert(i);
                        last_digit_index = Some(i);
                    } else {
                        invalid_found = true;
                    }
                    saw_digit = true;
                }
                _ => break,
            }
            i += 1;
        }

        // ---------- WARNINGS ----------
        if let (Some(first), Some(last)) = (first_digit_index, last_digit_index) {
            let leading_end = underscore_positions.partition_point(|&p| p < first);
            let trailing_start = underscore_positions.partition_point(|&p| p <= last);

            // Leading
            if leading_end > 0 {
                let us = &underscore_positions[..leading_end];
                let span = Span::new(
                    start + us[0],
                    start + us[us.len() - 1] + 1,
                    self.file_id,
                    SpanSource::Source,
                );
                self.compiler.shared.reports.push(
                    Report::build(ReportKind::Warning, span)
                        .with_message("underscore directly after base prefix")
                        .with_label(Label::new(span).with_color(YELLOW_COLOR))
                        .with_help("consider: remove")
                        .finish(),
                );
            }

            // Trailing
            if trailing_start < underscore_positions.len() {
                let us = &underscore_positions[trailing_start..];
                let span = Span::new(
                    start + us[0],
                    start + us[us.len() - 1] + 1,
                    self.file_id,
                    SpanSource::Source,
                );
                self.compiler.shared.reports.push(
                    Report::build(ReportKind::Warning, span)
                        .with_message("trailing underscore in number literal")
                        .with_label(Label::new(span).with_color(YELLOW_COLOR))
                        .with_help("consider: remove")
                        .finish(),
                );
            }

            // Middle — ein Report pro konsekutiver Gruppe
            let middle = &underscore_positions[leading_end..trailing_start];
            let mut group_start: Option<usize> = None;
            let mut prev: Option<usize> = None;

            for &pos in middle {
                match (group_start, prev) {
                    (Some(_), Some(p)) if pos == p + 1 => {
                        // Gruppe läuft weiter
                    }
                    (Some(s), Some(p)) => {
                        if p - s >= 1 {
                            let span = Span::new(
                                start + s,
                                start + p + 1,
                                self.file_id,
                                SpanSource::Source,
                            );
                            self.compiler.shared.reports.push(
                                Report::build(ReportKind::Warning, span)
                                    .with_message(
                                        "multiple consecutive underscores in number literal",
                                    )
                                    .with_label(Label::new(span).with_color(YELLOW_COLOR))
                                    .with_help("consider: remove or reduce to one underscore")
                                    .finish(),
                            );
                        }
                        group_start = Some(pos);
                    }
                    _ => {
                        group_start = Some(pos);
                    }
                }
                prev = Some(pos);
            }

            // Letzte Gruppe
            if let (Some(s), Some(e)) = (group_start, prev) {
                if e - s >= 1 {
                    let span =
                        Span::new(start + s, start + e + 1, self.file_id, SpanSource::Source);
                    self.compiler.shared.reports.push(
                        Report::build(ReportKind::Warning, span)
                            .with_message("multiple consecutive underscores in number literal")
                            .with_label(Label::new(span).with_color(YELLOW_COLOR))
                            .with_help("consider: remove or reduce to one underscore")
                            .finish(),
                    );
                }
            }
        }

        // ---------- ERRORS ----------
        if !saw_digit {
            let span = Span::new(start, start + i, self.file_id, SpanSource::Source);
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
            let span = Span::new(start, start + i, self.file_id, SpanSource::Source);
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
        let (suffix, suffix_len) = self.read_num_suffix(i);
        i += suffix_len;

        let raw = &self.input()[start + prefix_len..start + i - suffix_len];
        let mut text: String = raw.iter().map(|&b| b as char).collect();
        text.retain(|c| c != '_');

        match u128::from_str_radix(&text, radix) {
            Ok(v) => (TokenKind::Integer(v, suffix), i),
            Err(_) => {
                let span = Span::new(start, start + i, self.file_id, SpanSource::Source);
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

        let (suffix, suffix_len) = self.read_num_suffix(i);
        i += suffix_len;

        let raw = &self.input()[start..start + i - suffix_len];
        let mut text: String = raw.iter().map(|&b| b as char).collect();
        text.retain(|c| c != '_');

        if has_dot || text.contains('e') || text.contains('E') {
            match text.parse::<f64>() {
                Ok(v) => (TokenKind::Float(v, suffix), i),
                Err(_) => {
                    let span = Span::new(start, start + i, self.file_id, SpanSource::Source);
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
                Ok(v) => (TokenKind::Integer(v, suffix), i),
                Err(_) => {
                    let span = Span::new(start, start + i, self.file_id, SpanSource::Source);
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
                        let span = Span::new(start, start + i, self.file_id, SpanSource::Source);
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
                let span = Span::new(start, start + i, self.file_id, SpanSource::Source);
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
            let span = Span::new(start, start + i, self.file_id, SpanSource::Source);
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

        let span = Span::new(start, start + i, self.file_id, SpanSource::Source);
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
        // Kapazität schätzen — die meisten Strings sind kürzer als der Rest der Datei
        let mut result = String::with_capacity(32);

        while let Some(c) = self.peek(i) {
            match c {
                b'"' => {
                    i += 1;
                    result.shrink_to_fit(); // überschüssige Kapazität freigeben
                    return (TokenKind::String(result.into_boxed_str()), i);
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
                        Some(other) => other as char,
                        None => break,
                    };
                    result.push(esc);
                    i += 1;
                }
                _ => {
                    // Batch-Kopie aller normalen Bytes bis zum nächsten Sonderzeichen
                    let batch_start = i;
                    while let Some(c) = self.peek(i) {
                        match c {
                            b'"' | b'\n' | b'\\' | b'\'' => break,
                            _ => i += 1,
                        }
                    }
                    let bytes = &self.input()[self.pos + batch_start..self.pos + i];
                    result.push_str(std::str::from_utf8(bytes).unwrap());
                }
            }
        }

        let span = Span::new(start, start + i, self.file_id, SpanSource::Source);
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

        while let Some(c) = self.peek(i) {
            match c {
                b'\n' => break,
                b'"' => {
                    // Slice direkt aus dem Input, keine Allokation
                    let s = std::str::from_utf8(&self.input()[start + 2..self.pos + i])
                        .unwrap()
                        .to_owned(); // noch eine Allokation, aber Box<str> möglich
                    return (TokenKind::RawString(s.into_boxed_str()), i + 1);
                }
                _ => {
                    i += 1;
                }
            }
        }

        let span = Span::new(start, start + i, self.file_id, SpanSource::Source);
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
                    return (TokenKind::ByteString(bytes.into_boxed_slice()), i);
                }
                other => {
                    bytes.push(other);
                    i += 1;
                }
            }
        }

        let span = Span::new(start, start + i, self.file_id, SpanSource::Source);
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
                return (TokenKind::RawByteString(bytes.into_boxed_slice()), i);
            }
            bytes.push(c);
            i += 1;
        }

        let span = Span::new(start, start + i, self.file_id, SpanSource::Source);
        self.compiler.shared.reports.push(
            Report::build(ReportKind::Error, span)
                .with_message("unterminated string literal")
                .with_label(Label::new(span).with_color(RED_COLOR))
                .finish(),
        );

        (TokenKind::Error, i)
    }
}

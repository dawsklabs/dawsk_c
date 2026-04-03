use smallvec::SmallVec;

use crate::{ast::{ASTMacroRule, ASTStmt, CaptureKind, MacroBodyToken, MacroBracketKind, MacroPatternToken, RepKind}, color::RED_COLOR, lexer::token::{Keyword, Token, TokenKind}, parser::Parser, reports::{Label, Report, ReportKind}, source::Span};

impl<'a> Parser<'a> {
    pub(super) fn parse_macro_stmt(&mut self) -> ASTStmt {
        let public = self.parse_visibility();
        self.consume_check(TokenKind::Keyword(Keyword::Macro));

        let ident_token = self.consume_identifier();
        let ident = self.make_ident(&ident_token);

        // Bestimme Klammer-Typ
        let bracket_kind = match self.peek(0).kind {
            TokenKind::LParen => {
                self.advance(1);
                MacroBracketKind::Paren
            }
            TokenKind::LBracket => {
                self.advance(1);
                MacroBracketKind::Square
            }
            TokenKind::LCurly => {
                self.advance(1);
                MacroBracketKind::Curly
            }
            _ => {
                let span = self.peek(0).span;
                self.lexer.compiler.shared.reports.push(
                    Report::build(ReportKind::Error, span)
                        .with_message("expected bracket type after macro name")
                        .finish(),
                );
                MacroBracketKind::Paren
            }
        };

        let end_bracket = match bracket_kind {
            MacroBracketKind::Paren => TokenKind::RParen,
            MacroBracketKind::Square => TokenKind::RBracket,
            MacroBracketKind::Curly => TokenKind::RCurly,
        };

        let mut rules: SmallVec<[ASTMacroRule; 4]> = SmallVec::new();

        while self.peek(0).kind != end_bracket && self.peek(0).kind != TokenKind::EndOfFile {
            rules.push(self.parse_macro_rule());
            if self.peek(0).kind != end_bracket {
                self.consume_check(TokenKind::Semicolon);
            }
        }

        self.consume_check(end_bracket);
        ASTStmt::macro_dec(ident, public, rules.into_boxed_slice(), bracket_kind) // ← Pass bracket_kind
    }

    fn parse_macro_rule(&mut self) -> ASTMacroRule {
        self.consume_check(TokenKind::LParen);
        let pattern = self.parse_macro_pattern(TokenKind::RParen);
        self.consume_check(TokenKind::RParen);

        self.consume_check(TokenKind::FatArrow);

        self.consume_check(TokenKind::LCurly);
        let body = self.parse_macro_body(TokenKind::RCurly);
        self.consume_check(TokenKind::RCurly);

        ASTMacroRule { pattern, body }
    }

    fn parse_macro_pattern(&mut self, end: TokenKind) -> Box<[MacroPatternToken]> {
        let mut tokens: SmallVec<[MacroPatternToken; 4]> = SmallVec::new();

        while self.peek(0).kind != end && self.peek(0).kind != TokenKind::EndOfFile {
            match self.peek(0).kind.clone() {
                TokenKind::Dollar => {
                    self.advance(1);

                    if self.peek(0).kind == TokenKind::LParen {
                        // Repetition: $( ... )sep* oder +
                        self.advance(1);
                        let inner = self.parse_macro_pattern(TokenKind::RParen);
                        self.consume_check(TokenKind::RParen);

                        let separator = match self.peek(0).kind.clone() {
                            TokenKind::Comma | TokenKind::Semicolon => Some(self.consume().kind),
                            _ => None,
                        };

                        let kind = match self.peek(0).kind {
                            TokenKind::Asterisk => {
                                self.advance(1);
                                RepKind::ZeroOrMore
                            }
                            TokenKind::Plus => {
                                self.advance(1);
                                RepKind::OneOrMore
                            }
                            _ => {
                                let span = self.peek(0).span;
                                self.lexer.compiler.shared.reports.push(
                                    Report::build(ReportKind::Error, span)
                                        .with_message("expected `*` or `+` after repetition")
                                        .with_label(Label::new(span).with_color(RED_COLOR))
                                        .finish(),
                                );
                                RepKind::ZeroOrMore
                            }
                        };

                        tokens.push(MacroPatternToken::Repetition {
                            tokens: inner,
                            separator,
                            kind,
                        });
                    } else {
                        // $name:kind
                        let name_tok = self.consume_identifier();
                        let name_id = self.make_ident(&name_tok).id;

                        self.consume_check(TokenKind::Colon);

                        let kind_tok = self.consume_identifier();
                        let kind = match &kind_tok.kind {
                            TokenKind::Identifier(k) => match self.lexer.compiler.string_pool.get(*k).unwrap() {
                                "expr" => CaptureKind::Expr,
                                "ident" => CaptureKind::Ident,
                                "ty" => CaptureKind::Ty,
                                "literal" => CaptureKind::Literal,
                                "stmt" => CaptureKind::Stmt,
                                _ => {
                                    self.lexer.compiler.shared.reports.push(
                                        Report::build(ReportKind::Error, kind_tok.span)
                                            .with_message(format!("unknown capture kind `{}`", k))
                                            .with_label(
                                                Label::new(kind_tok.span).with_color(RED_COLOR),
                                            )
                                            .finish(),
                                    );
                                    CaptureKind::Expr
                                }
                            },
                            _ => CaptureKind::Expr,
                        };

                        tokens.push(MacroPatternToken::Capture {
                            name: name_id,
                            kind,
                            span: name_tok.span,
                        });
                    }
                }
                _ => {
                    let tok = self.consume();
                    tokens.push(MacroPatternToken::Literal(tok.kind));
                }
            }
        }

        tokens.into_boxed_slice()
    }

    fn parse_macro_body(&mut self, end: TokenKind) -> Box<[MacroBodyToken]> {
        let mut tokens: SmallVec<[MacroBodyToken; 4]> = SmallVec::new();

        while self.peek(0).kind != end && self.peek(0).kind != TokenKind::EndOfFile {
            match self.peek(0).kind.clone() {
                TokenKind::Dollar => {
                    self.advance(1);

                    if self.peek(0).kind == TokenKind::LParen {
                        // Repetition im Body
                        self.advance(1);
                        let inner = self.parse_macro_body(TokenKind::RParen);
                        self.consume_check(TokenKind::RParen);

                        let separator = match self.peek(0).kind.clone() {
                            TokenKind::Comma | TokenKind::Semicolon => Some(self.consume().kind),
                            _ => None,
                        };

                        let kind = match self.peek(0).kind {
                            TokenKind::Asterisk => {
                                self.advance(1);
                                RepKind::ZeroOrMore
                            }
                            TokenKind::Plus => {
                                self.advance(1);
                                RepKind::OneOrMore
                            }
                            _ => RepKind::ZeroOrMore,
                        };

                        tokens.push(MacroBodyToken::Repetition {
                            tokens: inner,
                            separator,
                            kind,
                        });
                    } else {
                        // $name
                        let name_tok = self.consume_identifier();
                        let name_id = self.make_ident(&name_tok).id;
                        tokens.push(MacroBodyToken::Var(name_id, name_tok.span));
                    }
                }
                _ => {
                    let tok = self.consume();
                    tokens.push(MacroBodyToken::Literal(tok.kind, tok.span));
                }
            }
        }

        tokens.into_boxed_slice()
    }

    pub(super) fn collect_macro_args(&mut self, name_span: Span) -> (Vec<Token>, Span) {
        // Erkenne welche Art von Klammer kommt
        let (open_kind, close_kind) = match self.peek(0).kind {
            TokenKind::LParen => (TokenKind::LParen, TokenKind::RParen),
            TokenKind::LBracket => (TokenKind::LBracket, TokenKind::RBracket),
            TokenKind::LCurly => (TokenKind::LCurly, TokenKind::RCurly),
            _ => {
                let span = self.peek(0).span;
                self.lexer.compiler.shared.reports.push(
                    Report::build(ReportKind::Error, span)
                        .with_message("expected ( [ or { after macro name")
                        .finish(),
                );
                return (vec![], name_span);
            }
        };

        self.advance(1); // consume opening bracket
        let mut tokens = Vec::new();
        let mut depth = 0usize;

        loop {
            let tok = self.consume();
            match tok.kind.clone() {
                k if k == open_kind => {
                    depth += 1;
                    tokens.push(tok);
                }
                k if k == close_kind => {
                    if depth == 0 {
                        let call_span = Span::merge(name_span, tok.span);
                        return (tokens, call_span);
                    }
                    depth -= 1;
                    tokens.push(tok);
                }
                TokenKind::EndOfFile => break,
                _ => tokens.push(tok),
            }
        }

        (tokens, name_span)
    }
}
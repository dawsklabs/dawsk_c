use smallvec::{SmallVec, smallvec};

use crate::{ast::{ASTExpr, ASTStmt}, color::RED_COLOR, lexer::token::{Keyword, NumSuffix, Token, TokenKind}, macros::{SyntaxContext, expander::ExpandError}, parser::Parser, reports::{Label, Report, ReportKind}, source::Span};

impl<'a> Parser<'a> {
    pub(super) fn parse_atom(&mut self) -> ASTExpr {
        let token = self.consume();
        match token.kind {
            TokenKind::Integer(v, suffix) => {
                if suffix.as_ref().is_some_and(NumSuffix::is_float) {
                    self.lexer.compiler.shared.reports.push(
                        Report::build(ReportKind::Error, token.span)
                            .with_message(format!(
                                "suffix `{}` is not valid on an integer literal",
                                suffix.as_ref().unwrap()
                            ))
                            .with_label(
                                Label::new(token.span)
                                    .with_message("expected integer suffix (u8, i32, ...)")
                                    .with_color(RED_COLOR),
                            )
                            .finish(),
                    );
                }
                ASTExpr::int(v, suffix, token.span)
            }

            TokenKind::Float(v, suffix) => {
                if suffix.as_ref().is_some_and(NumSuffix::is_integer) {
                    self.lexer.compiler.shared.reports.push(
                        Report::build(ReportKind::Error, token.span)
                            .with_message(format!(
                                "suffix `{}` is not valid on a float literal",
                                suffix.as_ref().unwrap()
                            ))
                            .with_label(
                                Label::new(token.span)
                                    .with_message("expected float suffix (f32, f64)")
                                    .with_color(RED_COLOR),
                            )
                            .finish(),
                    );
                }
                ASTExpr::float(v, suffix, token.span)
            }
            TokenKind::Byte(v) => ASTExpr::byte(v, token.span),
            TokenKind::Char(v) => ASTExpr::char(v, token.span),
            TokenKind::String(v) | TokenKind::RawString(v) => ASTExpr::string(v, token.span),
            TokenKind::ByteString(v) | TokenKind::RawByteString(v) => {
                ASTExpr::byte_string(v, token.span)
            }
            TokenKind::Keyword(Keyword::True) => ASTExpr::bool(true, token.span),
            TokenKind::Keyword(Keyword::False) => ASTExpr::bool(false, token.span),
            TokenKind::Identifier(name) => {
                if self.peek(0).kind == TokenKind::Exclamation {
                    self.advance(1); // '!'

                    if self.expander.macros.contains_key(&name) {
                        let (raw_tokens, call_span) = self.collect_macro_args(token.span);

                        let result: Result<Vec<Token>, ExpandError> = {
                            let source_map = &mut self.lexer.compiler.sourcemap;
                            let ctx_table = &mut self.lexer.compiler.syntax_contexts;
                            self.expander.expand(
                                name,
                                raw_tokens,
                                call_span,
                                SyntaxContext::ROOT,
                                source_map,
                                ctx_table,
                            )
                        };

                        match result {
                            Ok(expanded) => {
                                let expanded: Vec<Token> = expanded;
                                for tok in expanded.into_iter().rev() {
                                    self.buffer.push_front(tok);
                                }
                                return self.parse_expr();
                            }
                            Err(ExpandError::RecursionLimit) => {
                                self.lexer.compiler.shared.reports.push(
                                    Report::build(ReportKind::Error, call_span)
                                        .with_message(format!(
                                            "macro `{}!` exceeded recursion limit",
                                            name
                                        ))
                                        .finish(),
                                );
                                return ASTExpr::error(call_span.file_id);
                            }
                            Err(ExpandError::NoMatch(_)) => {
                                return ASTExpr::error(call_span.file_id);
                            }
                        }
                    }

                    // Fallback: Builtin-Macro
                    let (args, close) = self.parse_call_args();
                    let call_span = Span::merge(token.span, close.span);
                    return ASTExpr::macro_call(name, args, call_span);
                }
                ASTExpr::variable(name, token.span)
            }

            TokenKind::LParen => {
                // Unit-Tupel () oder geklammerten Ausdruck
                if self.peek(0).kind == TokenKind::RParen {
                    let close = self.consume();
                    return ASTExpr::unit(Span::merge(token.span, close.span));
                }
                let expr = self.parse_expr();
                // Tupel: (a, b, c)
                if self.peek(0).kind == TokenKind::Comma {
                    let mut elems = vec![expr];
                    while self.peek(0).kind == TokenKind::Comma {
                        self.advance(1);
                        if self.peek(0).kind == TokenKind::RParen {
                            break;
                        }
                        elems.push(self.parse_expr());
                    }
                    let close = self.consume_check(TokenKind::RParen);
                    let span = Span::merge(token.span, close.span);
                    return ASTExpr::tuple(elems.into_boxed_slice(), span);
                }
                let close = self.consume_check(TokenKind::RParen);
                ASTExpr::parenthesized(expr, Span::merge(token.span, close.span))
            }
            // WICHTIG: LCurly hier NICHT konsumieren — parse_block_expr macht das selbst.
            // Da token schon konsumiert ist, müssen wir den Span weitergeben.
            TokenKind::LCurly => self.parse_block_body(token),
            _ => {
                // token wurde bereits konsumiert, also token.span verwenden
                self.lexer.compiler.shared.reports.push(
                    Report::build(ReportKind::Error, token.span)
                        .with_message("unexpected token in type position")
                        .with_label(
                            Label::new(token.span)
                                .with_message("expected TYPE")
                                .with_color(RED_COLOR),
                        )
                        .finish(),
                );
                ASTExpr::error(token.span.file_id)
            }
        }
    }

    pub(super) fn parse_block_body(&mut self, curly: Token) -> ASTExpr {
        let mut stmts = Vec::new();
        let mut tail_expr = None;

        while self.peek(0).kind != TokenKind::RCurly && self.peek(0).kind != TokenKind::EndOfFile {
            // dec-Variablen sind in Blöcken immer Statements
            let offset = if matches!(self.peek(0).kind, TokenKind::Keyword(Keyword::Pub)) {
                1
            } else {
                0
            };
            match &self.peek(offset).kind {
                TokenKind::Keyword(Keyword::Dec) => {
                    stmts.push(self.parse_var_stmt());
                }
                _ => {
                    let expr = self.parse_expr();
                    if self.peek(0).kind == TokenKind::Semicolon {
                        self.advance(1);
                        stmts.push(ASTStmt::expr(expr));
                    } else {
                        tail_expr = Some(expr);
                        break;
                    }
                }
            }
        }

        let close = self.consume_check(TokenKind::RCurly);
        let span = Span::merge(curly.span, close.span);
        ASTExpr::block(stmts.into_boxed_slice(), tail_expr, span)
    }



    pub(super) fn parse_fields<T, F>(&mut self, end: TokenKind, mut parse_field: F) -> Box<[T]>
    where
        F: FnMut(&mut Self) -> T,
    {
        let mut fields: SmallVec<[T; 4]> = smallvec![];

        while self.peek(0).kind != end && self.peek(0).kind != TokenKind::EndOfFile {
            fields.push(parse_field(self));
        }

        fields.into_boxed_slice()
    }
}
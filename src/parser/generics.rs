use smallvec::{SmallVec, smallvec};

use crate::{
    ast::{ASTGenericParam, ASTType}, color::RED_COLOR, lexer::token::TokenKind, parser::Parser, reports::{Label, Report, ReportKind}
};

impl<'a> Parser<'a> {
    pub(super) fn parse_generics(&mut self) -> Box<[ASTType]> {
        self.consume_check(TokenKind::LAngle);
        let mut elems: SmallVec<[ASTType; 2]> = smallvec![];

        loop {
            elems.push(self.parse_type());

            match self.peek(0).kind {
                TokenKind::Comma => {
                    self.consume();
                }
                TokenKind::RAngle => {
                    self.consume();
                    break;
                }
                _ => {
                    let span = self.peek(0).span;
                    self.lexer.compiler.shared.reports.push(
                        Report::build(ReportKind::Error, span)
                            .with_message("unexpected token")
                            .with_label(
                                Label::new(span)
                                    .with_message("expected CLOSING ANGLE BRACKET or COMMA")
                                    .with_color(RED_COLOR),
                            )
                            .finish(),
                    );

                    break;
                }
            }
        }

        elems.into_boxed_slice()
    }

    pub(super) fn parse_generics_defs(&mut self) -> SmallVec<[ASTGenericParam; 2]> {
        self.consume_check(TokenKind::LAngle);
        let mut params = smallvec![];

        loop {
            // Name des Typparameters (T, U, ...)
            let ident_token = self.consume_identifier();
            let ident = self.make_ident(&ident_token);

            // optionaler Default: = Type
            let default = if self.peek(0).kind == TokenKind::Equals {
                self.advance(1); // '='
                Some(self.parse_type())
            } else {
                None
            };

            params.push(ASTGenericParam::new(ident, default));

            match self.peek(0).kind {
                TokenKind::Comma => {
                    self.advance(1);
                }
                TokenKind::RAngle => {
                    self.advance(1);
                    break;
                }
                _ => {
                    let span = self.peek(0).span;
                    self.lexer.compiler.shared.reports.push(
                        Report::build(ReportKind::Error, span)
                            .with_message("unexpected token")
                            .with_label(
                                Label::new(span)
                                    .with_message("expected CLOSING ANGLE BRACKET or COMMA")
                                    .with_color(RED_COLOR),
                            )
                            .finish(),
                    );
                    break;
                }
            }
        }

        params
    }
}
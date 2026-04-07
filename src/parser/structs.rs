use crate::{ast::{ASTStmt, ASTStructField, ASTTupleStructField, Publicity}, color::RED_COLOR, lexer::token::{Keyword, TokenKind}, parser::Parser, reports::{Label, Report, ReportKind}};

impl<'a> Parser<'a> {
    pub(super) fn parse_struct_stmt(&mut self) -> Result<ASTStmt, ()> {
        let public = self.parse_visibility();
        self.consume_check(TokenKind::Keyword(Keyword::Struct))?;

        let ident_token = self.consume_identifier();
        let ident = self.make_ident(&ident_token);

        let generics = if self.peek(0).kind == TokenKind::LAngle {
            self.parse_generics_defs()?
        } else {
            Box::new([])
        };

        if self.peek(0).kind == TokenKind::LCurly {
            self.advance(1);
            let fields = self.parse_fields(TokenKind::RCurly, Self::parse_struct_field)?;
            self.consume_check(TokenKind::RCurly)?;
            // no Semicolon
            Ok(ASTStmt::struct_dec(ident, public, generics, fields))
        } else if self.peek(0).kind == TokenKind::LParen {
            self.advance(1);
            let fields = self.parse_fields(TokenKind::RParen, Self::parse_tuple_struct_field)?;
            self.consume_check(TokenKind::RParen)?;
            self.consume_check(TokenKind::Semicolon)?; // Tuple-Struct takes ;
            Ok(ASTStmt::tuple_struct_dec(ident, public, generics, fields))
        } else {
            // Unit struct: struct Foo;
            self.consume_check(TokenKind::Semicolon)?; // Unit-Struct takes ;
            Ok(ASTStmt::unit_struct_dec(ident, public))
        }
    }

    pub(super) fn parse_struct_field(&mut self) -> Result<ASTStructField, ()> {
        let public = if self.parse_optional_token(TokenKind::Keyword(Keyword::Pub)) {
            Publicity::Public
        } else {
            Publicity::Private
        };

        // name
        let ident_token = self.consume_identifier();
        let ident = self.make_ident(&ident_token);

        // :
        self.consume_check(TokenKind::Colon)?;

        // type
        let ty = self.parse_type()?;

        self.parse_field_end(TokenKind::RCurly);

        Ok(ASTStructField { ident, public, ty })
    }

    pub(super) fn parse_tuple_struct_field(&mut self) -> Result<ASTTupleStructField, ()> {
        let public = if self.parse_optional_token(TokenKind::Keyword(Keyword::Pub)) {
            Publicity::Public
        } else {
            Publicity::Private
        };

        // type
        let ty = self.parse_type()?;

        self.parse_field_end(TokenKind::RParen);

        Ok(ASTTupleStructField { public, ty })
    }

    fn parse_field_end(&mut self, end: TokenKind) {
        let current = self.peek(0);
        match &current.kind {
            TokenKind::Comma => {
                self.advance(1);
            }
            k if *k == end => {
                // ok, letztes Feld
            }
            _ => {
                let span = self.peek(0).span;
                self.lexer.compiler.shared.reports.push(
                    Report::build(ReportKind::Error, span)
                        .with_message("unexpected token")
                        .with_label(
                            Label::new(span)
                                .with_message("expected COMMA or DECLARATION END")
                                .with_color(RED_COLOR),
                        )
                        .finish(),
                );
            }
        }
    }
}
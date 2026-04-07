use crate::{ast::{ASTStmt, Mutability}, color::RED_COLOR, lexer::token::{Keyword, TokenKind}, parser::Parser, reports::{Label, Report, ReportKind}};

impl<'a> Parser<'a> {
    pub(super) fn parse_var_stmt(&mut self) -> Result<ASTStmt, ()> {
        if self.peek(0).kind == TokenKind::Keyword(Keyword::Pub) {
            let span = self.peek(0).span;
            self.lexer.compiler.shared.reports.push(
                Report::build(ReportKind::Error, span)
                    .with_message("PUB is not allowed on variable declarations")
                    .with_label(
                        Label::new(span)
                            .with_message("remove PUB here")
                            .with_color(RED_COLOR),
                    )
                    .finish(),
            );
            self.advance(1); // konsumieren und weitermachen
        }

        self.consume_check(TokenKind::Keyword(Keyword::Dec))?;

        let mutable = if self.parse_optional_token(TokenKind::Keyword(Keyword::Mut)) {
            Mutability::Mutable
        } else {
            Mutability::Immutable
        };

        let ident_token = self.consume_identifier();
        let ident = self.make_ident(&ident_token);

        let ty = if self.peek(0).kind == TokenKind::Colon {
            self.advance(1);
            Some(self.parse_type()?)
        } else {
            None
        };

        self.consume_check(TokenKind::Equals)?;

        let expr = if self.peek(0).kind == TokenKind::LCurly {
            let curly = self.consume();
            self.parse_block_body(curly)?
        } else {
            self.parse_expr()?
        };

        self.consume_check(TokenKind::Semicolon)?;
        Ok(ASTStmt::var_dec(ident, mutable, ty, expr))
    }

    pub(super) fn parse_const_stmt(&mut self) -> Result<ASTStmt, ()> {
        let public = self.parse_visibility();
        self.consume_check(TokenKind::Keyword(Keyword::Const))?;

        let ident_token = self.consume_identifier();
        let ident = self.make_ident(&ident_token);

        let ty = if self.peek(0).kind == TokenKind::Colon {
            self.advance(1);
            Some(self.parse_type()?)
        } else {
            None
        };

        self.consume_check(TokenKind::Equals)?;
        let expr = self.parse_expr()?;
        self.consume_check(TokenKind::Semicolon)?;

        Ok(ASTStmt::const_dec(ident, public, ty, expr))
    }
}
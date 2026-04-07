use crate::{
    ast::ASTStmt, lexer::token::{Keyword, TokenKind}, parser::Parser
};

impl<'a> Parser<'a> {
    pub(super) fn parse_type_alias_stmt(&mut self) -> Result<ASTStmt, ()> {
        let public = self.parse_visibility();
        self.consume_check(TokenKind::Keyword(Keyword::Type))?;

        let ident_token = self.consume_identifier();
        let ident = self.make_ident(&ident_token);

        // optionale Generics: type Data<A> = ...
        let generics = if self.peek(0).kind == TokenKind::LAngle {
            self.parse_generics_defs()? // <A, B> als Namen, nicht als Typen
        } else {
            Box::new([])
        };

        self.consume_check(TokenKind::Equals)?;
        let ty = self.parse_type()?;
        self.consume_check(TokenKind::Semicolon)?;

        Ok(ASTStmt::type_alias(ident, public, generics, ty))
    }
}
use smallvec::{SmallVec, smallvec};

use crate::{ast::{ASTEnumVariant, ASTEnumVariantKind, ASTStmt}, lexer::token::{Keyword, TokenKind}, parser::Parser};

impl<'a> Parser<'a> {
    pub(super) fn parse_enum_stmt(&mut self) -> Result<ASTStmt, ()> {
        let public = self.parse_visibility();
        self.consume_check(TokenKind::Keyword(Keyword::Enum))?;

        let ident_token = self.consume_identifier();
        let ident = self.make_ident(&ident_token);

        let generics = if self.peek(0).kind == TokenKind::LAngle {
            self.parse_generics_defs()?
        } else {
            Box::new([])
        };

        self.consume_check(TokenKind::LCurly)?;
        let mut variants: SmallVec<[ASTEnumVariant; 4]> = smallvec![];

        while self.peek(0).kind != TokenKind::RCurly && self.peek(0).kind != TokenKind::EndOfFile {
            variants.push(self.parse_enum_variant()?);
        }

        self.consume_check(TokenKind::RCurly)?;
        Ok(ASTStmt::enum_dec(ident, public, generics, variants.into_boxed_slice()))
    }

    fn parse_enum_variant(&mut self) -> Result<ASTEnumVariant, ()> {
        let ident_token = self.consume_identifier();
        let ident = self.make_ident(&ident_token);

        let kind = match self.peek(0).kind {
            TokenKind::LParen => {
                // Tuple-Variante: A(Type, Type)
                self.advance(1);
                let fields = self.parse_fields(TokenKind::RParen, Self::parse_tuple_struct_field)?;
                self.consume_check(TokenKind::RParen)?;
                ASTEnumVariantKind::Tuple(fields)
            }
            TokenKind::LCurly => {
                // Struct-Variante: C { a: T, b: U }
                self.advance(1);
                let fields = self.parse_fields(TokenKind::RCurly, Self::parse_struct_field)?;
                self.consume_check(TokenKind::RCurly)?;
                ASTEnumVariantKind::Struct(fields)
            }
            _ => ASTEnumVariantKind::Unit,
        };

        // optionales Komma zwischen Varianten
        self.parse_optional_token(TokenKind::Comma);

        Ok(ASTEnumVariant { ident, kind })
    }
}
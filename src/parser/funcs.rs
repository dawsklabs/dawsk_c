use smallvec::{smallvec, SmallVec};

use crate::{
    ast::{ASTExpr, ASTExprKind, ASTFuncParam, ASTStmt, Mutability},
    lexer::token::{Keyword, Token, TokenKind},
    parser::Parser,
    source::Span,
};

impl<'a> Parser<'a> {
    pub(super) fn parse_func_stmt(&mut self) -> ASTStmt {
        let public = self.parse_visibility();
        self.consume_check(TokenKind::Keyword(Keyword::Func));

        let ident_token = self.consume_identifier();
        let ident = self.make_ident(&ident_token);

        let generics = if self.peek(0).kind == TokenKind::LAngle {
            self.parse_generics_defs()
        } else {
            smallvec![]
        };

        // Parameter: (mut name: Type, name: Type, ...)
        self.consume_check(TokenKind::LParen);
        let params = self.parse_func_params();
        self.consume_check(TokenKind::RParen);

        // optionaler Rückgabetyp: -> Type
        let return_ty = if self.peek(0).kind == TokenKind::Arrow {
            self.advance(1);
            Some(self.parse_type())
        } else {
            None
        };

        // Body
        let curly = self.consume_check(TokenKind::LCurly);
        let body_expr = self.parse_block_body(curly);

        // body_expr ist ASTExpr::Block, wir brauchen ASTBlockExpr
        let body = match body_expr.kind {
            ASTExprKind::Block(b) => b,
            _ => unreachable!(),
        };

        ASTStmt::func_dec(ident, public, generics, params, return_ty, body)
    }

    fn parse_func_params(&mut self) -> SmallVec<[ASTFuncParam; 4]> {
        let mut params = smallvec![];

        while self.peek(0).kind != TokenKind::RParen && self.peek(0).kind != TokenKind::EndOfFile {
            // Receiver: &inst oder &mut inst
            if self.peek(0).kind == TokenKind::And {
                let start_span = self.peek(0).span;

                let is_mut = self.peek(1).kind == TokenKind::Keyword(Keyword::Mut)
                    && self.peek(2).kind == TokenKind::Keyword(Keyword::Inst);
                let is_ref = self.peek(1).kind == TokenKind::Keyword(Keyword::Inst);

                if is_mut {
                    let end_span = self.peek(2).span;
                    self.advance(3); // &, mut, inst
                    let span = Span::merge(start_span, end_span);
                    params.push(ASTFuncParam::Receiver {
                        mutable: Mutability::Mutable,
                        span,
                    });
                } else if is_ref {
                    let end_span = self.peek(1).span;
                    self.advance(2); // &, inst
                    let span = Span::merge(start_span, end_span);
                    params.push(ASTFuncParam::Receiver {
                        mutable: Mutability::Immutable,
                        span,
                    });
                } else {
                    // & aber kein inst dahinter — normaler Typ-Parameter, fällt durch
                    // zum Named-Arm (der dann einen Fehler wirft, ist ok)
                    let ident_token = self.consume_identifier();
                    let ident = self.make_ident(&ident_token);
                    self.consume_check(TokenKind::Colon);
                    let ty = self.parse_type();
                    let span = Span::merge(start_span, self.backpeek(1).span);
                    params.push(ASTFuncParam::Named {
                        ident,
                        mutable: Mutability::Immutable,
                        ty,
                        span,
                    });
                }
            } else {
                let start_span = self.peek(0).span;

                // Normaler Parameter: [mut] name: Type
                let mutable = if self.parse_optional_token(TokenKind::Keyword(Keyword::Mut)) {
                    Mutability::Mutable
                } else {
                    Mutability::Immutable
                };

                let ident_token = self.consume_identifier();
                let ident = self.make_ident(&ident_token);
                self.consume_check(TokenKind::Colon);
                let ty = self.parse_type();
                let span = Span::merge(start_span, self.backpeek(1).span);
                params.push(ASTFuncParam::Named {
                    ident,
                    mutable,
                    ty,
                    span,
                });
            }

            if self.peek(0).kind == TokenKind::Comma {
                self.advance(1);
            } else {
                break;
            }
        }

        params
    }

    pub(super) fn parse_call_args(&mut self) -> (Box<[ASTExpr]>, Token) {
        self.consume_check(TokenKind::LParen);
        let mut args: SmallVec<[ASTExpr; 4]> = SmallVec::new();

        while self.peek(0).kind != TokenKind::RParen && self.peek(0).kind != TokenKind::EndOfFile {
            args.push(self.parse_expr());
            if self.peek(0).kind == TokenKind::Comma {
                self.advance(1);
            } else {
                break;
            }
        }

        let close = self.consume_check(TokenKind::RParen);
        (args.into_vec().into_boxed_slice(), close)
    }
}

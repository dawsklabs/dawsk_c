use crate::{
    ast::{
        ASTType,
        Ident,
        Mutability,
    },
    color::RED_COLOR,
    lexer::{LexMode, token::{Keyword, TokenKind}},
    parser::Parser,
    reports::{Label, Report, ReportKind}
};

impl<'a> Parser<'a> {
    pub(super) fn parse_type(&mut self) -> ASTType {
        let mut base = self.parse_type_atom();

        // Generic-Argumente
        if self.peek(0).kind == TokenKind::LAngle {
            self.lexer.add_mode(LexMode::Generic);
            let prev_span = self.backpeek(1).span;
            let next_span = self.peek(0).span;
            self.check_whitespace(prev_span, next_span);

            let args = self.parse_generics();

            self.lexer.remove_mode();

            base = ASTType::Generic {
                base: Box::new(base),
                args,
            };
        }

        base
    }

    fn parse_type_atom(&mut self) -> ASTType {
        if self.peek(0).kind == TokenKind::And {
            self.advance(1);

            let mutable = if self.parse_optional_token(TokenKind::Keyword(Keyword::Mut)) {
                Mutability::Mutable
            } else {
                Mutability::Immutable
            };

            let inner = self.parse_type();
            return ASTType::Ref {
                mutable,
                inner: Box::new(inner),
            };
        }

        let token = self.consume();

        match token.kind {
            TokenKind::Identifier(name) => {
                let mut segments = vec![Ident::new(
                    name,
                    token.span,
                )];

                while self.peek(0).kind == TokenKind::DoubleColon {
                    self.consume(); // '::'
                    let seg_token = self.consume_identifier();
                    let seg = self.make_ident(&seg_token);
                    segments.push(seg);
                }
                ASTType::Path(segments.into_boxed_slice()) // oder Path(Ident)
            }

            TokenKind::LParen => {
                let mut elems = Vec::new();

                if self.peek(0).kind != TokenKind::RParen {
                    loop {
                        elems.push(self.parse_type());

                        if self.peek(0).kind == TokenKind::Comma {
                            self.advance(1);
                        } else {
                            break;
                        }
                    }
                }

                self.consume_check(TokenKind::RParen);
                ASTType::Tuple(elems.into_boxed_slice())
            }

            _ => {
                let span = self.peek(0).span;
                self.lexer.compiler.shared.reports.push(
                    Report::build(ReportKind::Error, span)
                        .with_message("unexpected token")
                        .with_label(
                            Label::new(span)
                                .with_message("expected TYPE")
                                .with_color(RED_COLOR),
                        )
                        .finish(),
                );
                ASTType::Error
            }
        }
    }
}
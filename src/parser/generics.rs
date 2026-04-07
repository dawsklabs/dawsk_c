use smallvec::{smallvec, SmallVec};

use crate::{
    ast::{strings::StringId, ASTGenericParam, ASTRequirePredicate, ASTTraitBound, ASTType},
    color::{BLUE_COLOR, RED_COLOR, YELLOW_COLOR},
    lexer::token::{Keyword, TokenKind},
    parser::Parser,
    reports::{Label, Report, ReportKind},
    source::Span,
};

impl<'a> Parser<'a> {
    pub(super) fn parse_generics(&mut self) -> Result<Box<[ASTType]>, ()> {
        self.consume_check(TokenKind::LAngle)?;
        let mut elems: SmallVec<[ASTType; 2]> = smallvec![];

        loop {
            elems.push(self.parse_type()?);

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
                                    .with_message(format!(
                                        "expected {} or {}",
                                        TokenKind::RAngle,
                                        TokenKind::Comma
                                    ))
                                    .with_color(RED_COLOR),
                            )
                            .finish(),
                    );
                    break;
                }
            }
        }

        Ok(elems.into_boxed_slice())
    }

    pub(super) fn parse_generics_defs(&mut self) -> Result<Box<[ASTGenericParam]>, ()> {
        self.consume_check(TokenKind::LAngle)?;
        let mut params: SmallVec<[ASTGenericParam; 2]> = smallvec![];

        loop {
            let start_span = self.peek(0).span;

            let ident_token = self.consume_identifier();
            let ident = self.make_ident(&ident_token);

            let default = if self.peek(0).kind == TokenKind::Equals {
                self.advance(1);
                Some(self.parse_type()?)
            } else {
                None
            };

            let span = Span::merge(start_span, self.backpeek(1).span);
            params.push(ASTGenericParam::new(ident, default, span));

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
                                    .with_message(format!(
                                        "expected {} or {}",
                                        TokenKind::RAngle,
                                        TokenKind::Comma
                                    ))
                                    .with_color(RED_COLOR),
                            )
                            .finish(),
                    );
                    break;
                }
            }
        }

        Ok(params.into_boxed_slice())
    }

    pub(super) fn parse_require_clause(
        &mut self,
        generics: &[ASTGenericParam], // kein mut mehr nötig
    ) -> Result<Box<[ASTRequirePredicate]>, ()> {
        if self.peek(0).kind != TokenKind::Keyword(Keyword::Require) {
            return Ok(Box::new([]));
        }
        self.advance(1);

        let mut predicates: SmallVec<[ASTRequirePredicate; 2]> = smallvec![];

        while !matches!(self.peek(0).kind, TokenKind::LCurly | TokenKind::EndOfFile) {
            let start_span = self.peek(0).span;

            let ident_token = self.consume_identifier();
            let ty = self.make_ident(&ident_token);

            self.consume_check(TokenKind::Colon)?;

            let raw_bounds = self.parse_trait_bounds();
            let ty_name = self
                .lexer
                .compiler
                .string_pool
                .get(ty.id)
                .unwrap()
                .to_owned();

            // Nur noch self-dups im require-Block selbst prüfen
            let mut seen: SmallVec<[(u32, Span); 4]> = smallvec![];
            let mut dup_groups: SmallVec<[(u32, Span, Vec<Span>); 4]> = smallvec![];

            let bounds: Box<[ASTTraitBound]> = raw_bounds
                .iter()
                .filter(|bound| {
                    let id = bound.path.id.0;
                    if let Some((_, first_span)) = seen.iter().find(|(sid, _)| *sid == id) {
                        if let Some((_, _, spans)) =
                            dup_groups.iter_mut().find(|(did, _, _)| *did == id)
                        {
                            spans.push(bound.span);
                        } else {
                            dup_groups.push((id, *first_span, vec![bound.span]));
                        }
                        false
                    } else {
                        seen.push((id, bound.span));
                        true
                    }
                })
                .cloned()
                .collect::<Vec<_>>()
                .into_boxed_slice();

            for (dup_id, first_span, dup_spans) in &dup_groups {
                let bound_name = self
                    .lexer
                    .compiler
                    .string_pool
                    .get(StringId(*dup_id))
                    .unwrap_or("<unknown>");

                let mut report = Report::build(ReportKind::Warning, dup_spans[0])
                    .with_message(format!("redundant bound `{}` on `{}`", bound_name, ty_name))
                    .with_label(
                        Label::new(*first_span)
                            .with_message("already defined here")
                            .with_color(BLUE_COLOR),
                    );

                for dup_span in dup_spans {
                    report = report.with_label(
                        Label::new(*dup_span)
                            .with_color(YELLOW_COLOR),
                    );
                }

                self.lexer.compiler.shared.reports.push(report.finish());
            }

            // Typparameter existiert nicht in generics — später Fehler im Typchecker
            if !generics.iter().any(|g| g.name.id == ty.id) {
                let span = self.peek(0).span;
                self.lexer.compiler.shared.reports.push(
                    Report::build(ReportKind::Error, span)
                        .with_message(format!("unknown type parameter `{}`", ty_name))
                        .with_label(
                            Label::new(ty.span)
                                .with_message("not found in generic params")
                                .with_color(RED_COLOR),
                        )
                        .finish(),
                );
            }

            let span = Span::merge(start_span, self.backpeek(1).span);
            predicates.push(ASTRequirePredicate { ty, bounds, span });

            if self.peek(0).kind == TokenKind::Comma {
                self.advance(1);
            } else {
                break;
            }
        }

        Ok(predicates.into_boxed_slice())
    }

    fn parse_trait_bounds(&mut self) -> Box<[ASTTraitBound]> {
        let mut bounds: SmallVec<[ASTTraitBound; 2]> = smallvec![];

        loop {
            let span = self.peek(0).span;
            let ident_token = self.consume_identifier();
            let path = self.make_ident(&ident_token);
            bounds.push(ASTTraitBound { path, span });

            if self.peek(0).kind == TokenKind::Plus {
                self.advance(1);
            } else {
                break;
            }
        }

        bounds.into_boxed_slice()
    }
}

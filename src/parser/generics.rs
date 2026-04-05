use smallvec::{smallvec, SmallVec};

use crate::{
    ast::{ASTGenericParam, ASTRequirePredicate, ASTTraitBound, ASTType},
    color::{BLUE_COLOR, RED_COLOR, YELLOW_COLOR},
    lexer::token::{Keyword, TokenKind},
    parser::Parser,
    reports::{Label, Report, ReportKind},
    source::Span,
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
                            .with_message("unexpected tok3en")
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

    fn dedup_trait_bounds(
        &mut self,
        ty_name: &str,
        bounds: &[ASTTraitBound],
    ) -> Box<[ASTTraitBound]> {
        let mut seen: SmallVec<[u32; 4]> = smallvec![];
        let mut dup_spans: Vec<(Span, Span)> = vec![];

        let deduped = bounds
            .iter()
            .filter(|bound| {
                if seen.contains(&bound.path.id.0) {
                    let first = bounds.iter().find(|b| b.path.id == bound.path.id).unwrap();
                    dup_spans.push((bound.span, first.span));
                    false
                } else {
                    seen.push(bound.path.id.0);
                    true
                }
            })
            .cloned()
            .collect::<Vec<_>>()
            .into_boxed_slice();

        if !dup_spans.is_empty() {
                    let mut report = Report::build(ReportKind::Warning, dup_spans[0].0)
                        .with_message(format!(
                            "{} redundant bound{} on `{}`",
                            dup_spans.len(),
                            if dup_spans.len() == 1 { "" } else { "s" },
                            ty_name
                        ));

                    // Original nur einmal labeln
                    report = report.with_label(
                        Label::new(dup_spans[0].1)
                            .with_message("first defined here")
                            .with_color(BLUE_COLOR),
                    );

                    // Alle redundanten Stellen labeln
                    for (dup_span, _) in &dup_spans {
                        report = report.with_label(
                            Label::new(*dup_span)
                                .with_message("redundant here")
                                .with_color(YELLOW_COLOR),
                        );
                    }

                    self.lexer.compiler.shared.reports.push(report.finish());
                }

        deduped
    }

    pub(super) fn parse_generics_defs(&mut self) -> Box<[ASTGenericParam]> {
        self.consume_check(TokenKind::LAngle);
        let mut params: SmallVec<[ASTGenericParam; 2]> = smallvec![];

        loop {
            let start_span = self.peek(0).span;

            let ident_token = self.consume_identifier();
            let ident = self.make_ident(&ident_token);

            let bounds = if self.peek(0).kind == TokenKind::Colon {
                self.advance(1);
                let raw = self.parse_trait_bounds();
                let ty_name = self
                    .lexer
                    .compiler
                    .string_pool
                    .get(ident.id)
                    .unwrap()
                    .to_owned();
                self.dedup_trait_bounds(&ty_name, &raw)
            } else {
                Box::new([])
            };

            let default = if self.peek(0).kind == TokenKind::Equals {
                self.advance(1);
                Some(self.parse_type())
            } else {
                None
            };

            let span = Span::merge(start_span, self.backpeek(1).span);
            params.push(ASTGenericParam::new(ident, bounds, default, span));

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

        params.into_boxed_slice()
    }

    pub(super) fn parse_require_clause(
        &mut self,
        generics: &mut [ASTGenericParam],
    ) -> Box<[ASTRequirePredicate]> {
        if self.peek(0).kind != TokenKind::Keyword(Keyword::Require) {
            return Box::new([]);
        }
        self.advance(1);

        let mut predicates: SmallVec<[ASTRequirePredicate; 2]> = smallvec![];

        while !matches!(self.peek(0).kind, TokenKind::LCurly | TokenKind::EndOfFile) {
            let start_span = self.peek(0).span;

            let ident_token = self.consume_identifier();
            let ty = self.make_ident(&ident_token);

            self.consume_check(TokenKind::Colon);

            let raw_bounds = self.parse_trait_bounds();
            let ty_name = self
                .lexer
                .compiler
                .string_pool
                .get(ty.id)
                .unwrap()
                .to_owned();

            if let Some(generic) = generics.iter_mut().find(|g| g.name.id == ty.id) {
                // Gegen bestehende inline-bounds filtern
                let mut dup_spans: Vec<(Span, Span)> = vec![];
                let new_bounds: Box<[ASTTraitBound]> = raw_bounds
                    .iter()
                    .filter(|bound| {
                        if let Some(existing) =
                            generic.bounds.iter().find(|b| b.path.id == bound.path.id)
                        {
                            dup_spans.push((bound.span, existing.span));
                            false
                        } else {
                            true
                        }
                    })
                    .cloned()
                    .collect::<Vec<_>>()
                    .into_boxed_slice();

                if !dup_spans.is_empty() {
                    let mut report = Report::build(ReportKind::Warning, dup_spans[0].0)
                        .with_message(format!(
                            "{} redundant bound{} on `{}`",
                            dup_spans.len(),
                            if dup_spans.len() == 1 { "" } else { "s" },
                            ty_name
                        ));

                    // Original nur einmal labeln
                    report = report.with_label(
                        Label::new(dup_spans[0].1)
                            .with_message("first defined here")
                            .with_color(BLUE_COLOR),
                    );

                    // Alle redundanten Stellen labeln
                    for (dup_span, _) in &dup_spans {
                        report = report.with_label(
                            Label::new(*dup_span)
                                .with_message("redundant here")
                                .with_color(YELLOW_COLOR),
                        );
                    }

                    self.lexer.compiler.shared.reports.push(report.finish());
                }

                // Neue bounds nochmal gegen sich selbst deduplizieren ...
                let new_bounds = self.dedup_trait_bounds(&ty_name, &new_bounds);

                // ... und in den GenericParam mergen
                let merged = generic
                    .bounds
                    .iter()
                    .chain(new_bounds.iter())
                    .cloned()
                    .collect::<Vec<_>>()
                    .into_boxed_slice();
                generic.bounds = merged;

                if !new_bounds.is_empty() {
                    let span = Span::merge(start_span, self.backpeek(1).span);
                    predicates.push(ASTRequirePredicate {
                        ty,
                        bounds: new_bounds,
                        span,
                    });
                }
            } else {
                // Typparameter nicht in generics — nur gegen sich selbst deduplizieren
                let bounds = self.dedup_trait_bounds(&ty_name, &raw_bounds);
                let span = Span::merge(start_span, self.backpeek(1).span);
                predicates.push(ASTRequirePredicate { ty, bounds, span });
            }

            if self.peek(0).kind == TokenKind::Comma {
                self.advance(1);
            } else {
                break;
            }
        }

        predicates.into_boxed_slice()
    }

    fn parse_trait_bounds(&mut self) -> Box<[ASTTraitBound]> {
        let mut bounds: SmallVec<[ASTTraitBound; 2]> = smallvec![];

        loop {
            let span = self.peek(0).span;
            let ident_token = self.consume_identifier();
            let path = self.make_ident(&ident_token);
            bounds.push(ASTTraitBound { path, span });

            if self.peek(0).kind == TokenKind::Plus {
                self.advance(1); // '+'
            } else {
                break;
            }
        }

        bounds.into_boxed_slice()
    }
}

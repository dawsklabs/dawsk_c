use crate::ast::strings::StringId;
use crate::ast::{ASTImportSegment, ASTImportStmt, ASTImportTree, ASTIncludeStmt, ASTItem, Ident};
use crate::color::RED_COLOR;
use crate::lexer::token::{Keyword, TokenKind};
use crate::parser::Parser;
use crate::reports::{Label, Report, ReportKind};
use crate::source::Span;
use smallvec::SmallVec;

impl<'a> Parser<'a> {
    pub(super) fn parse_include_stmt(&mut self) -> Result<ASTItem, ()> {
        let start = self.consume(); // 'include'

        let modules: Box<[Ident]> = if self.peek(0).kind == TokenKind::LCurly {
            self.advance(1); // '{'
            let mut items = SmallVec::<[Ident; 4]>::new();
            while self.peek(0).kind != TokenKind::RCurly
                && self.peek(0).kind != TokenKind::EndOfFile
            {
                let tok = self.consume_identifier();
                items.push(self.make_ident(&tok));
                if self.peek(0).kind == TokenKind::Comma {
                    self.advance(1);
                } else {
                    break;
                }
            }
            self.consume_check(TokenKind::RCurly)?;
            items.into_vec().into_boxed_slice()
        } else {
            let tok = self.consume_identifier();
            let ident = self.make_ident(&tok);
            Box::new([ident])
        };

        let semi = self.consume_check(TokenKind::Semicolon)?;
        let span = Span::merge(start.span, semi.span);
        Ok(ASTItem::Include(ASTIncludeStmt { modules, span }))
    }

    pub(super) fn parse_import_stmt(&mut self) -> Result<ASTItem, ()> {
        let start = self.consume(); // 'import'
        let root = self.parse_import_segments()?;
        let semi = self.consume_check(TokenKind::Semicolon)?;
        let span = Span::merge(start.span, semi.span);
        Ok(ASTItem::Import(ASTImportStmt { root, span }))
    }

    fn parse_import_segments(&mut self) -> Result<Box<[ASTImportSegment]>, ()> {
        let mut segments = SmallVec::<[ASTImportSegment; 2]>::new();
        loop {
            segments.push(self.parse_import_segment()?);
            if self.peek(0).kind == TokenKind::Comma {
                self.advance(1);
                if matches!(
                    self.peek(0).kind,
                    TokenKind::RCurly | TokenKind::Semicolon | TokenKind::EndOfFile
                ) {
                    break;
                }
            } else {
                break;
            }
        }
        Ok(segments.into_vec().into_boxed_slice())
    }

    fn parse_import_segment(&mut self) -> Result<ASTImportSegment, ()> {
        let tok = self.consume();
        let ident = match &tok.kind {
            TokenKind::Identifier(_) => self.make_ident(&tok),
            TokenKind::Keyword(Keyword::Inst) => {
                let id = self.lexer.compiler.string_pool.intern("inst");
                Ident::new(id, tok.span)
            }
            TokenKind::Keyword(Keyword::Super) => {
                let id = self.lexer.compiler.string_pool.intern("super");
                Ident::new(id, tok.span)
            }
            TokenKind::Keyword(Keyword::Pkg) => {
                let id = self.lexer.compiler.string_pool.intern("pkg");
                Ident::new(id, tok.span)
            }
            _ => {
                self.lexer.compiler.shared.reports.push(
                    Report::build(ReportKind::Error, tok.span)
                        .with_message("expected identifier")
                        .with_label(Label::new(tok.span).with_color(RED_COLOR))
                        .finish(),
                );
                Ident::new(StringId::EMPTY, tok.span)
            }
        };

        let tree = if self.peek(0).kind == TokenKind::Dot {
            self.advance(1); // '.'
            if self.peek(0).kind == TokenKind::LCurly {
                self.advance(1); // '{'
                let inner = self.parse_import_segments()?;
                self.consume_check(TokenKind::RCurly)?;
                Some(ASTImportTree::Grouped(inner))
            } else {
                let inner = self.parse_import_segment()?;
                Some(ASTImportTree::Single(Box::new(inner)))
            }
        } else {
            None
        };

        let alias = if tree.is_none() && self.peek(0).kind == TokenKind::Keyword(Keyword::As) {
            self.advance(1);
            let tok = self.consume_identifier();
            Some(self.make_ident(&tok))
        } else {
            None
        };

        Ok(ASTImportSegment { ident, tree, alias })
    }
}

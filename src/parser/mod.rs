pub mod macros;
pub mod alias;
pub mod vars;
pub mod enums;
pub mod exprs;
pub mod funcs;
pub mod generics;
pub mod simple;
pub mod structs;
pub mod types;
pub mod imports;

use std::{collections::VecDeque};

use crate::{Compiler, abort, args, ast::{ASTExpr, ASTItem, ASTStmt, ASTStmtKind, Ident, Publicity, strings::StringId}, color::RED_COLOR, lexer::{Lexer, token::{Keyword, Token, TokenKind}}, macros::expander::MacroExpander, reports::{Label, Report, ReportKind}, source::Span};

pub struct Parser<'a> {
    pub lexer: Lexer<'a>,
    buffer: VecDeque<Token>,
    pub expander: MacroExpander, // owned, kein Lifetime
    log_tokens: bool,
}

impl<'a> Parser<'a> {
    pub(crate) fn new(compiler: &'a mut Compiler, file_id: usize) -> Self {
        let lexer = Lexer::new(compiler, file_id);
        Self {
            lexer,
            buffer: VecDeque::with_capacity(4),
            expander: MacroExpander::new(),
            log_tokens: args::step_enabled(args::step::TOKEN),
        }
    }

    fn ensure_buffered(&mut self, n: usize) {
        while self.buffer.len() <= n {
            let tok = self.lexer.next_token();
            if self.log_tokens {
                println!("{:?} @ {:?}", tok.kind, tok.span);
            }
            self.buffer.push_back(tok);
        }
    }

    pub(crate) fn peek(&mut self, n: usize) -> &Token {
        self.ensure_buffered(n);
        &self.buffer[n]
    }

    pub(crate) fn backpeek(&self, n: usize) -> &Token {
        assert!(n > 0);
        assert!(
            n <= self.buffer.len(),
            "backpeek({n}) out of bounds (len={})",
            self.buffer.len()
        );
        &self.buffer[self.buffer.len() - n]
    }

    pub(crate) fn advance(&mut self, n: usize) {
        for _ in 0..n {
            if self.buffer.pop_front().is_none() {
                let tok = self.lexer.next_token();
                drop(tok); // direkt verwerfen ohne buffern
            }
        }
    }

    pub(crate) fn consume(&mut self) -> Token {
        self.ensure_buffered(0);
        self.buffer.pop_front().unwrap()
    }

    pub(crate) fn consume_check(&mut self, expected: TokenKind) -> Result<Token, ()> {
        let token = self.consume();
        if token.kind == expected {
            return Ok(token);
        }
        let span = token.span;
        self.lexer.compiler.shared.reports.push(
            Report::build(ReportKind::Error, span)
                .with_message(format!(
                    "expected `{expected}`, found `{}`",
                    token.kind
                ))
                .with_label(
                    Label::new(span)
                        .with_message(format!("expected {expected}"))
                        .with_color(RED_COLOR),
                )
                .finish(),
        );
        Err(())
    }

    fn check_whitespace(&mut self, starter: Span, next: Span) {
        if next.start <= starter.end {
            return;
        }
        self.lexer.compiler.shared.reports.push(
            Report::build(ReportKind::Warning, Span::merge(starter, next))
                .with_message("unexpected whitespace")
                .with_label(Label::new(Span::merge(starter, next)).with_color(RED_COLOR))
                .finish(),
        );
    }

    fn parse_optional_token(&mut self, kind: TokenKind) -> bool {
        if self.peek(0).kind == kind {
            self.advance(1);
            true
        } else {
            false
        }
    }

    fn consume_identifier(&mut self) -> Token {
        let tok = self.consume();
        let span = tok.span;
        if !matches!(tok.kind, TokenKind::Identifier(_)) {
            self.lexer.compiler.shared.reports.push(
                Report::build(ReportKind::Error, span)
                    .with_message("expected identifier")
                    .with_label(
                        Label::new(span)
                            .with_message("expected IDENTIFIER")
                            .with_color(RED_COLOR),
                    )
                    .finish(),
            );
        }
        tok
    }

    fn make_ident(&mut self, token: &Token) -> Ident {
        let id = match &token.kind {
            TokenKind::Identifier(n) => *n,
            _ => StringId::EMPTY,
        };
        Ident::new(id, token.span)
    }

    fn parse_visibility(&mut self) -> Publicity {
        if self.parse_optional_token(TokenKind::Keyword(Keyword::Pub)) {
            Publicity::Public
        } else {
            Publicity::Private
        }
    }

    pub(crate) fn next_item(&mut self) -> Result<Option<ASTItem>, ()> {
        if abort::is_aborted() {
            return Ok(None);
        }

        while self.peek(0).kind == TokenKind::Semicolon {
            self.advance(1);
        }

        if matches!(
            self.peek(0).kind,
            TokenKind::EndOfFile | TokenKind::Error | TokenKind::RCurly
        ) {
            return Ok(None);
        }

        let offset = if matches!(self.peek(0).kind, TokenKind::Keyword(Keyword::Pub)) {
            1
        } else {
            0
        };

        match &self.peek(offset).kind {
            TokenKind::Keyword(Keyword::Include) => {
                return Ok(Some(self.parse_include_stmt()?));
            }
            TokenKind::Keyword(Keyword::Import) => {
                return Ok(Some(self.parse_import_stmt()?));
            }
            _ => {}
        }

        let stmt = self.parse_stmt()?;

        if let ASTStmtKind::MacroDec(ref def) = stmt.kind {
            self.expander.register(def.ident.id, def.as_ref().clone());
            return self.next_item();
        }

        Ok(Some(ASTItem::Stmt(stmt)))
    }

    fn parse_stmt(&mut self) -> Result<ASTStmt, ()> {
        let offset = if matches!(self.peek(0).kind, TokenKind::Keyword(Keyword::Pub)) {
            1
        } else {
            0
        };
        match &self.peek(offset).kind {
            TokenKind::Keyword(Keyword::Dec) => Ok(self.parse_var_stmt()?),
            TokenKind::Keyword(Keyword::Const) => Ok(self.parse_const_stmt()?),
            TokenKind::Keyword(Keyword::Type) => Ok(self.parse_type_alias_stmt()?),
            TokenKind::Keyword(Keyword::Struct) => Ok(self.parse_struct_stmt()?),
            TokenKind::Keyword(Keyword::Enum) => Ok(self.parse_enum_stmt()?),
            TokenKind::Keyword(Keyword::Func) => Ok(self.parse_func_stmt()?),
            TokenKind::Keyword(Keyword::Macro) => Ok(self.parse_macro_stmt()?),
            // TokenKind::Keyword(Keyword::Trait) => self.parse_trait_stmt(),
            // TokenKind::Keyword(Keyword::Extend) => self.parse_extend_stmt(),
            _ => Ok(self.parse_expr_stmt()?),
        }
    }

    fn parse_expr_stmt(&mut self) -> Result<ASTStmt, ()> {
        let expr = self.parse_expr()?;

        self.consume_check(TokenKind::Semicolon)?;

        Ok(ASTStmt::expr(expr))
    }

    fn parse_expr(&mut self) -> Result<ASTExpr, ()> {
        if let TokenKind::Identifier(_) = self.peek(0).kind {
            match self.peek(1).kind {
                TokenKind::Equals
                | TokenKind::PlusEquals
                | TokenKind::MinusEquals
                | TokenKind::AsteriskEquals
                | TokenKind::SlashEquals => {
                    let ident_token = self.consume();
                    let var = ASTExpr::variable(
                        match &ident_token.kind {
                            TokenKind::Identifier(n) => *n,
                            _ => unreachable!(),
                        },
                        ident_token.span,
                    );
                    return self.parse_assignment(var);
                }
                _ => {}
            }
        }
        self.parse_binary_expr(0)
    }
}
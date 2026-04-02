use smallvec::{smallvec, SmallVec};

use super::lexer::{LexMode, Lexer};
use super::token::{Keyword, Token, TokenKind};
use super::{
    ASTBinaryOperator, ASTBinaryOperatorKind, ASTExpr, ASTStmt, ASTStructField,
    ASTTupleStructField, ASTType, ASTUnaryOperator, ASTUnaryOperatorKind, Mutability, Publicity,
};
use crate::ast::expander::{ExpandError, MacroExpander};
use crate::ast::macros::SyntaxContext;
use crate::ast::strings::StringId;
use crate::ast::token::NumSuffix;
use crate::ast::{
    ASTEnumVariant, ASTEnumVariantKind, ASTExprKind, ASTFuncParam, ASTGenericParam, ASTMacroRule,
    ASTStmtKind, CaptureKind, Ident, MacroBodyToken, MacroBracketKind, MacroPatToken, RepKind,
};
use crate::color::RED_COLOR;
use crate::reports::{Label, Report, ReportKind};

use crate::source::{Span, SpanSource};
use crate::{abort, args, Compiler};
use std::collections::VecDeque;

pub struct Parser<'a> {
    lexer: Lexer<'a>,
    buffer: VecDeque<Token>,
    pub expander: MacroExpander, // owned, kein Lifetime
    log_tokens: bool,
}

impl<'a> Parser<'a> {
    pub fn new(compiler: &'a mut Compiler, file_id: usize) -> Self {
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

    pub fn peek(&mut self, n: usize) -> &Token {
        self.ensure_buffered(n);
        &self.buffer[n]
    }

    pub fn backpeek(&self, n: usize) -> &Token {
        assert!(n > 0);
        assert!(
            n <= self.buffer.len(),
            "backpeek({n}) out of bounds (len={})",
            self.buffer.len()
        );
        &self.buffer[self.buffer.len() - n]
    }

    pub fn advance(&mut self, n: usize) {
        for _ in 0..n {
            if self.buffer.pop_front().is_none() {
                let tok = self.lexer.next_token();
                drop(tok); // direkt verwerfen ohne buffern
            }
        }
    }

    pub fn consume(&mut self) -> Token {
        self.ensure_buffered(0);
        self.buffer.pop_front().unwrap()
    }

    pub fn consume_check(&mut self, expected: TokenKind) -> Token {
        let token = self.consume();
        if token.kind == expected {
            return token;
        }
        let span = token.span; // span speichern, bevor wir
        self.lexer.compiler.shared.reports.push(
            Report::build(ReportKind::Error, span)
                .with_message(format!(
                    "expected `{expected}`, found `{}`",
                    token.kind.clone()
                ))
                .with_label(
                    Label::new(span)
                        .with_message(format!("expected {expected}"))
                        .with_color(RED_COLOR),
                )
                .finish(),
        );
        token
    }

    pub fn check_whitespace(&mut self, starter: Span, next: Span) {
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
            TokenKind::Identifier(name) => self.lexer.compiler.string_pool.intern(name),
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

    pub fn next_stmt(&mut self) -> Option<ASTStmt> {
        if abort::is_aborted() {
            return None;
        }

        while self.peek(0).kind == TokenKind::Semicolon {
            self.advance(1);
        }

        if matches!(
            self.peek(0).kind,
            TokenKind::EndOfFile | TokenKind::Error | TokenKind::RCurly
        ) {
            return None;
        }

        let stmt = self.parse_stmt();

        // MacroDef sofort registrieren, nicht in den AST aufnehmen
        if let ASTStmtKind::MacroDec(ref def) = stmt.kind {
            let name = self
                .lexer
                .compiler
                .string_pool
                .get(def.ident.id)
                .unwrap_or("")
                .to_string();
            self.expander.register(name, def.clone());
            return self.next_stmt();
        }

        Some(stmt)
    }

    fn parse_stmt(&mut self) -> ASTStmt {
        let offset = if matches!(self.peek(0).kind, TokenKind::Keyword(Keyword::Pub)) {
            1
        } else {
            0
        };
        match &self.peek(offset).kind {
            TokenKind::Keyword(Keyword::Dec) => self.parse_var_stmt(),
            TokenKind::Keyword(Keyword::Const) => self.parse_const_stmt(),
            TokenKind::Keyword(Keyword::Type) => self.parse_type_alias_stmt(),
            TokenKind::Keyword(Keyword::Struct) => self.parse_struct_stmt(),
            TokenKind::Keyword(Keyword::Enum) => self.parse_enum_stmt(),
            TokenKind::Keyword(Keyword::Func) => self.parse_func_stmt(),
            TokenKind::Keyword(Keyword::Macro) => self.parse_macro_stmt(),
            // TokenKind::Keyword(Keyword::Trait) => self.parse_trait_stmt(),
            // TokenKind::Keyword(Keyword::Extend) => self.parse_extend_stmt(),
            _ => self.parse_expr_stmt(),
        }
    }

    fn parse_block_body(&mut self, curly: Token) -> ASTExpr {
        let mut stmts = Vec::new();
        let mut tail_expr = None;

        while self.peek(0).kind != TokenKind::RCurly && self.peek(0).kind != TokenKind::EndOfFile {
            // dec-Variablen sind in Blöcken immer Statements
            let offset = if matches!(self.peek(0).kind, TokenKind::Keyword(Keyword::Pub)) {
                1
            } else {
                0
            };
            match &self.peek(offset).kind {
                TokenKind::Keyword(Keyword::Dec) => {
                    stmts.push(self.parse_var_stmt());
                }
                _ => {
                    let expr = self.parse_expr();
                    if self.peek(0).kind == TokenKind::Semicolon {
                        self.advance(1);
                        stmts.push(ASTStmt::expr(expr));
                    } else {
                        tail_expr = Some(expr);
                        break;
                    }
                }
            }
        }

        let close = self.consume_check(TokenKind::RCurly);
        let span = Span::merge(curly.span, close.span);
        ASTExpr::block(stmts.into_boxed_slice(), tail_expr, span)
    }

    fn parse_struct_field(&mut self) -> ASTStructField {
        let pub_ = if self.parse_optional_token(TokenKind::Keyword(Keyword::Pub)) {
            Publicity::Public
        } else {
            Publicity::Private
        };

        // name
        let ident_token = self.consume_identifier();
        let ident = self.make_ident(&ident_token);

        // :
        self.consume_check(TokenKind::Colon);

        // type
        let ty = self.parse_type();

        self.parse_field_end(TokenKind::RCurly);

        ASTStructField { ident, pub_, ty }
    }

    fn parse_tuple_struct_field(&mut self) -> ASTTupleStructField {
        let pub_ = if self.parse_optional_token(TokenKind::Keyword(Keyword::Pub)) {
            Publicity::Public
        } else {
            Publicity::Private
        };

        // type
        let ty = self.parse_type();

        self.parse_field_end(TokenKind::RParen);

        ASTTupleStructField { pub_, ty }
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

    fn parse_fields<T, F>(&mut self, end: TokenKind, mut parse_field: F) -> SmallVec<[T; 4]>
    where
        F: FnMut(&mut Self) -> T,
    {
        let mut fields = smallvec![];

        while self.peek(0).kind != end && self.peek(0).kind != TokenKind::EndOfFile {
            fields.push(parse_field(self));
        }

        fields
    }

    fn parse_var_stmt(&mut self) -> ASTStmt {
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

        self.consume_check(TokenKind::Keyword(Keyword::Dec));

        let mut_ = if self.parse_optional_token(TokenKind::Keyword(Keyword::Mut)) {
            Mutability::Mutable
        } else {
            Mutability::Immutable
        };

        let ident_token = self.consume_identifier();
        let ident = self.make_ident(&ident_token);

        let ty = if self.peek(0).kind == TokenKind::Colon {
            self.advance(1);
            Some(self.parse_type())
        } else {
            None
        };

        self.consume_check(TokenKind::Equals);

        let expr = if self.peek(0).kind == TokenKind::LCurly {
            let curly = self.consume();
            self.parse_block_body(curly)
        } else {
            self.parse_expr()
        };

        self.consume_check(TokenKind::Semicolon);
        ASTStmt::var_dec(ident, mut_, ty, expr)
    }

    fn parse_const_stmt(&mut self) -> ASTStmt {
        let pub_ = self.parse_visibility();
        self.consume_check(TokenKind::Keyword(Keyword::Const));

        let ident_token = self.consume_identifier();
        let ident = self.make_ident(&ident_token);

        let ty = if self.peek(0).kind == TokenKind::Colon {
            self.advance(1);
            Some(self.parse_type())
        } else {
            None
        };

        self.consume_check(TokenKind::Equals);
        let expr = self.parse_expr();
        self.consume_check(TokenKind::Semicolon);

        ASTStmt::const_dec(ident, pub_, ty, expr)
    }

    fn parse_type_alias_stmt(&mut self) -> ASTStmt {
        let pub_ = self.parse_visibility();
        self.consume_check(TokenKind::Keyword(Keyword::Type));

        let ident_token = self.consume_identifier();
        let ident = self.make_ident(&ident_token);

        // optionale Generics: type Data<A> = ...
        let generics = if self.peek(0).kind == TokenKind::LAngle {
            self.parse_generic_param_defs() // <A, B> als Namen, nicht als Typen
        } else {
            smallvec![]
        };

        self.consume_check(TokenKind::Equals);
        let ty = self.parse_type();
        self.consume_check(TokenKind::Semicolon);

        ASTStmt::type_alias(ident, pub_, generics, ty)
    }

    fn parse_struct_stmt(&mut self) -> ASTStmt {
        let pub_ = self.parse_visibility();
        self.consume_check(TokenKind::Keyword(Keyword::Struct));

        let ident_token = self.consume_identifier();
        let ident = self.make_ident(&ident_token);

        let generics = if self.peek(0).kind == TokenKind::LAngle {
            self.parse_generic_param_defs()
        } else {
            smallvec![]
        };

        if self.peek(0).kind == TokenKind::LCurly {
            self.advance(1);
            let fields = self.parse_fields(TokenKind::RCurly, Self::parse_struct_field);
            self.consume_check(TokenKind::RCurly);
            // no Semicolon
            ASTStmt::struct_dec(ident, pub_, generics, fields)
        } else if self.peek(0).kind == TokenKind::LParen {
            self.advance(1);
            let fields = self.parse_fields(TokenKind::RParen, Self::parse_tuple_struct_field);
            self.consume_check(TokenKind::RParen);
            self.consume_check(TokenKind::Semicolon); // Tuple-Struct takes ;
            ASTStmt::tuple_struct_dec(ident, pub_, generics, fields)
        } else {
            // Unit struct: struct Foo;
            self.consume_check(TokenKind::Semicolon); // Unit-Struct takes ;
            ASTStmt::unit_struct_dec(ident, pub_)
        }
    }

    fn parse_enum_stmt(&mut self) -> ASTStmt {
        let pub_ = self.parse_visibility();
        self.consume_check(TokenKind::Keyword(Keyword::Enum));

        let ident_token = self.consume_identifier();
        let ident = self.make_ident(&ident_token);

        let generics = if self.peek(0).kind == TokenKind::LAngle {
            self.parse_generic_param_defs()
        } else {
            smallvec![]
        };

        self.consume_check(TokenKind::LCurly);
        let mut variants = smallvec![];

        while self.peek(0).kind != TokenKind::RCurly && self.peek(0).kind != TokenKind::EndOfFile {
            variants.push(self.parse_enum_variant());
        }

        self.consume_check(TokenKind::RCurly);
        ASTStmt::enum_dec(ident, pub_, generics, variants)
    }

    fn parse_enum_variant(&mut self) -> ASTEnumVariant {
        let ident_token = self.consume_identifier();
        let ident = self.make_ident(&ident_token);

        let kind = match self.peek(0).kind {
            TokenKind::LParen => {
                // Tuple-Variante: A(Type, Type)
                self.advance(1);
                let fields = self.parse_fields(TokenKind::RParen, Self::parse_tuple_struct_field);
                self.consume_check(TokenKind::RParen);
                ASTEnumVariantKind::Tuple(fields)
            }
            TokenKind::LCurly => {
                // Struct-Variante: C { a: T, b: U }
                self.advance(1);
                let fields = self.parse_fields(TokenKind::RCurly, Self::parse_struct_field);
                self.consume_check(TokenKind::RCurly);
                ASTEnumVariantKind::Struct(fields)
            }
            _ => ASTEnumVariantKind::Unit,
        };

        // optionales Komma zwischen Varianten
        self.parse_optional_token(TokenKind::Comma);

        ASTEnumVariant { ident, kind }
    }

    fn parse_func_stmt(&mut self) -> ASTStmt {
        let pub_ = self.parse_visibility();
        self.consume_check(TokenKind::Keyword(Keyword::Func));

        let ident_token = self.consume_identifier();
        let ident = self.make_ident(&ident_token);

        let generics = if self.peek(0).kind == TokenKind::LAngle {
            self.parse_generic_param_defs()
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

        ASTStmt::func_dec(ident, pub_, generics, params, return_ty, body)
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

    fn parse_macro_stmt(&mut self) -> ASTStmt {
        let pub_ = self.parse_visibility();
        self.consume_check(TokenKind::Keyword(Keyword::Macro));

        let ident_token = self.consume_identifier();
        let ident = self.make_ident(&ident_token);

        // Bestimme Klammer-Typ
        let bracket_kind = match self.peek(0).kind {
            TokenKind::LParen => {
                self.advance(1);
                MacroBracketKind::Paren
            }
            TokenKind::LBracket => {
                self.advance(1);
                MacroBracketKind::Square
            }
            TokenKind::LCurly => {
                self.advance(1);
                MacroBracketKind::Curly
            }
            _ => {
                let span = self.peek(0).span;
                self.lexer.compiler.shared.reports.push(
                    Report::build(ReportKind::Error, span)
                        .with_message("expected bracket type after macro name")
                        .finish(),
                );
                MacroBracketKind::Paren
            }
        };

        let end_bracket = match bracket_kind {
            MacroBracketKind::Paren => TokenKind::RParen,
            MacroBracketKind::Square => TokenKind::RBracket,
            MacroBracketKind::Curly => TokenKind::RCurly,
        };

        let mut rules = Vec::new();

        while self.peek(0).kind != end_bracket && self.peek(0).kind != TokenKind::EndOfFile {
            rules.push(self.parse_macro_rule());
            if self.peek(0).kind != end_bracket {
                self.consume_check(TokenKind::Semicolon);
            }
        }

        self.consume_check(end_bracket);
        ASTStmt::macro_dec(ident, pub_, rules, bracket_kind) // ← Pass bracket_kind
    }

    fn parse_macro_rule(&mut self) -> ASTMacroRule {
        self.consume_check(TokenKind::LParen);
        let pattern = self.parse_macro_pattern(TokenKind::RParen);
        self.consume_check(TokenKind::RParen);

        self.consume_check(TokenKind::FatArrow);

        self.consume_check(TokenKind::LCurly);
        let body = self.parse_macro_body(TokenKind::RCurly);
        self.consume_check(TokenKind::RCurly);

        ASTMacroRule { pattern, body }
    }

    fn parse_macro_pattern(&mut self, end: TokenKind) -> Vec<MacroPatToken> {
        let mut tokens = Vec::new();

        while self.peek(0).kind != end && self.peek(0).kind != TokenKind::EndOfFile {
            match self.peek(0).kind.clone() {
                TokenKind::Dollar => {
                    self.advance(1);

                    if self.peek(0).kind == TokenKind::LParen {
                        // Repetition: $( ... )sep* oder +
                        self.advance(1);
                        let inner = self.parse_macro_pattern(TokenKind::RParen);
                        self.consume_check(TokenKind::RParen);

                        let separator = match self.peek(0).kind.clone() {
                            TokenKind::Comma | TokenKind::Semicolon => Some(self.consume().kind),
                            _ => None,
                        };

                        let kind = match self.peek(0).kind {
                            TokenKind::Asterisk => {
                                self.advance(1);
                                RepKind::ZeroOrMore
                            }
                            TokenKind::Plus => {
                                self.advance(1);
                                RepKind::OneOrMore
                            }
                            _ => {
                                let span = self.peek(0).span;
                                self.lexer.compiler.shared.reports.push(
                                    Report::build(ReportKind::Error, span)
                                        .with_message("expected `*` or `+` after repetition")
                                        .with_label(Label::new(span).with_color(RED_COLOR))
                                        .finish(),
                                );
                                RepKind::ZeroOrMore
                            }
                        };

                        tokens.push(MacroPatToken::Repetition {
                            tokens: inner,
                            separator,
                            kind,
                        });
                    } else {
                        // $name:kind
                        let name_tok = self.consume_identifier();
                        let name_id = self.make_ident(&name_tok).id;

                        self.consume_check(TokenKind::Colon);

                        let kind_tok = self.consume_identifier();
                        let kind = match &kind_tok.kind {
                            TokenKind::Identifier(k) => match k.as_str() {
                                "expr" => CaptureKind::Expr,
                                "ident" => CaptureKind::Ident,
                                "ty" => CaptureKind::Ty,
                                "literal" => CaptureKind::Literal,
                                "stmt" => CaptureKind::Stmt,
                                _ => {
                                    self.lexer.compiler.shared.reports.push(
                                        Report::build(ReportKind::Error, kind_tok.span)
                                            .with_message(format!("unknown capture kind `{}`", k))
                                            .with_label(
                                                Label::new(kind_tok.span).with_color(RED_COLOR),
                                            )
                                            .finish(),
                                    );
                                    CaptureKind::Expr
                                }
                            },
                            _ => CaptureKind::Expr,
                        };

                        tokens.push(MacroPatToken::Capture {
                            name: name_id,
                            kind,
                            span: name_tok.span,
                        });
                    }
                }
                _ => {
                    let tok = self.consume();
                    tokens.push(MacroPatToken::Literal(tok.kind));
                }
            }
        }

        tokens
    }

    fn parse_macro_body(&mut self, end: TokenKind) -> Vec<MacroBodyToken> {
        let mut tokens = Vec::new();

        while self.peek(0).kind != end && self.peek(0).kind != TokenKind::EndOfFile {
            match self.peek(0).kind.clone() {
                TokenKind::Dollar => {
                    self.advance(1);

                    if self.peek(0).kind == TokenKind::LParen {
                        // Repetition im Body
                        self.advance(1);
                        let inner = self.parse_macro_body(TokenKind::RParen);
                        self.consume_check(TokenKind::RParen);

                        let separator = match self.peek(0).kind.clone() {
                            TokenKind::Comma | TokenKind::Semicolon => Some(self.consume().kind),
                            _ => None,
                        };

                        let kind = match self.peek(0).kind {
                            TokenKind::Asterisk => {
                                self.advance(1);
                                RepKind::ZeroOrMore
                            }
                            TokenKind::Plus => {
                                self.advance(1);
                                RepKind::OneOrMore
                            }
                            _ => RepKind::ZeroOrMore,
                        };

                        tokens.push(MacroBodyToken::Repetition {
                            tokens: inner,
                            separator,
                            kind,
                        });
                    } else {
                        // $name
                        let name_tok = self.consume_identifier();
                        let name_id = self.make_ident(&name_tok).id;
                        tokens.push(MacroBodyToken::Var(name_id, name_tok.span));
                    }
                }
                _ => {
                    let tok = self.consume();
                    tokens.push(MacroBodyToken::Literal(tok.kind, tok.span));
                }
            }
        }

        tokens
    }

    fn collect_macro_args(&mut self, name_span: Span) -> (Vec<Token>, Span) {
        // Erkenne welche Art von Klammer kommt
        let (open_kind, close_kind) = match self.peek(0).kind {
            TokenKind::LParen => (TokenKind::LParen, TokenKind::RParen),
            TokenKind::LBracket => (TokenKind::LBracket, TokenKind::RBracket),
            TokenKind::LCurly => (TokenKind::LCurly, TokenKind::RCurly),
            _ => {
                let span = self.peek(0).span;
                self.lexer.compiler.shared.reports.push(
                    Report::build(ReportKind::Error, span)
                        .with_message("expected ( [ or { after macro name")
                        .finish(),
                );
                return (vec![], name_span);
            }
        };

        self.advance(1); // consume opening bracket
        let mut tokens = Vec::new();
        let mut depth = 0usize;

        loop {
            let tok = self.consume();
            match tok.kind.clone() {
                k if k == open_kind => {
                    depth += 1;
                    tokens.push(tok);
                }
                k if k == close_kind => {
                    if depth == 0 {
                        let call_span = Span::merge(name_span, tok.span);
                        return (tokens, call_span);
                    }
                    depth -= 1;
                    tokens.push(tok);
                }
                TokenKind::EndOfFile => break,
                _ => tokens.push(tok),
            }
        }

        (tokens, name_span)
    }

    // fn parse_extend_stmt(&mut self) -> ASTStmt {
    //     if self.peek(0).kind == TokenKind::Keyword(Keyword::Pub) {
    //         let span = self.peek(0).span;
    //         self.lexer.compiler.reports.push(
    //             Report::build(ReportKind::Error, span)
    //                 .with_message("`pub` is not allowed on `extend`")
    //                 .with_label(
    //                     Label::new(span)
    //                         .with_message("remove `pub` here")
    //                         .with_color(color::RED_COLOR),
    //                 )
    //                 .finish(),
    //         );
    //         self.advance(1); // konsumieren, dann normal weitermachen
    //     }

    //     // kein pub auf extend
    //     self.consume_check(TokenKind::Keyword(Keyword::Extend));

    //     let target = self.consume_identifier(); // MyStruct

    //     let target_generics = if self.peek(0).kind == TokenKind::LAngle {
    //         self.parse_generic_params()
    //     } else {
    //         smallvec![]
    //     };

    //     // optionaler Trait-Bound: : TraitPath<Assoc = Type>
    //     let trait_bound = if self.parse_optional_token(TokenKind::Colon) {
    //         Some(self.parse_trait_bound())
    //     } else {
    //         None
    //     };

    //     self.consume_check(TokenKind::LCurly);
    //     let items = self.parse_extend_items();
    //     self.consume_check(TokenKind::RCurly);

    //     ASTStmt::extend(target, target_generics, trait_bound, items)
    // }

    // /// Parst Trait-Name + optionale Generics inkl. Assoziierte-Typ-Bindungen
    // /// Beispiel: Add<T = Inst>  oder  pkg::super::Printable
    // fn parse_trait_bound(&mut self) -> ASTTraitBound {
    //     let path = self.parse_typath(); // pkg::super::Printable

    //     let args = if self.peek(0).kind == TokenKind::LAngle {
    //         self.lexer.add_mode(LexMode::Generic);
    //         self.consume(); // '<'
    //         let mut args = vec![];

    //         loop {
    //             // Assoziierter-Typ-Binding: Name = Type
    //             if matches!(self.peek(0).kind, TokenKind::Identifier(_))
    //                 && self.peek(1).kind == TokenKind::Equals
    //             {
    //                 let name = self.consume_identifier();
    //                 self.consume(); // '='
    //                 let ty = self.parse_type();
    //                 args.push(ASTGenericArg::AssocType { name, ty });
    //             } else {
    //                 args.push(ASTGenericArg::Type(self.parse_type()));
    //             }

    //             match self.peek(0).kind {
    //                 TokenKind::Comma => {
    //                     self.consume();
    //                 }
    //                 TokenKind::RAngle => {
    //                     self.consume();
    //                     break;
    //                 }
    //                 _ => break,
    //             }
    //         }
    //         self.lexer.remove_mode();
    //         args
    //     } else {
    //         vec![]
    //     };

    //     ASTTraitBound { path, args }
    // }

    // fn parse_extend_items(&mut self) -> Vec<ASTExtendItem> {
    //     let mut items = vec![];
    //     while self.peek(0).kind != TokenKind::RCurly && self.peek(0).kind != TokenKind::EndOfFile {
    //         // Semicolons überspringen
    //         while self.parse_optional_token(TokenKind::Semicolon) {}

    //         match self.peek(0).kind {
    //             TokenKind::Keyword(Keyword::Type) => {
    //                 // type Output = Inst;
    //                 self.consume();
    //                 let name = self.consume_identifier();
    //                 self.consume_check(TokenKind::Equals);
    //                 let ty = self.parse_type();
    //                 self.consume_check(TokenKind::Semicolon);
    //                 items.push(ASTExtendItem::AssocType { name, ty });
    //             }
    //             _ => {
    //                 // func (mit optionalem pub)
    //                 let pub_ = self.parse_visibility();
    //                 let func = self.parse_func_def(pub_);
    //                 items.push(ASTExtendItem::Func(func));
    //             }
    //         }
    //     }
    //     items
    // }

    fn parse_type(&mut self) -> ASTType {
        let mut base = self.parse_type_atom();

        // Generic-Argumente
        if self.peek(0).kind == TokenKind::LAngle {
            self.lexer.add_mode(LexMode::Generic);
            let prev_span = self.backpeek(1).span;
            let next_span = self.peek(0).span;
            self.check_whitespace(prev_span, next_span);

            let args = self.parse_generic_params();

            self.lexer.remove_mode();

            base = ASTType::Generic {
                base: Box::new(base),
                args: args.into_vec(),
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
                    self.lexer.compiler.string_pool.intern(&name),
                    token.span,
                )];

                while self.peek(0).kind == TokenKind::DoubleColon {
                    self.consume(); // '::'
                    let seg_token = self.consume_identifier();
                    let seg = self.make_ident(&seg_token);
                    segments.push(seg);
                }
                ASTType::Path(segments) // oder Path(Ident)
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
                ASTType::Tuple(elems)
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

    fn parse_generic_params(&mut self) -> SmallVec<[ASTType; 2]> {
        self.consume_check(TokenKind::LAngle);
        let mut elems = smallvec![];

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

        elems
    }

    fn parse_generic_param_defs(&mut self) -> SmallVec<[ASTGenericParam; 2]> {
        self.consume_check(TokenKind::LAngle);
        let mut params = smallvec![];

        loop {
            // Name des Typparameters (T, U, ...)
            let ident_token = self.consume_identifier();
            let ident = self.make_ident(&ident_token);

            // optionaler Default: = Type
            let default = if self.peek(0).kind == TokenKind::Equals {
                self.advance(1); // '='
                Some(self.parse_type())
            } else {
                None
            };

            params.push(ASTGenericParam::new(ident, default));

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

        params
    }

    fn parse_expr_stmt(&mut self) -> ASTStmt {
        let expr = self.parse_expr();

        self.consume_check(TokenKind::Semicolon);

        ASTStmt::expr(expr)
    }

    fn parse_expr(&mut self) -> ASTExpr {
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
                            TokenKind::Identifier(n) => n.clone(),
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

    fn parse_assignment(&mut self, target: ASTExpr) -> ASTExpr {
        let op_token = self.consume();
        let op_kind = match op_token.kind {
            TokenKind::Equals => ASTBinaryOperatorKind::Assign,
            TokenKind::PlusEquals => ASTBinaryOperatorKind::AddAssign,
            TokenKind::MinusEquals => ASTBinaryOperatorKind::SubtractAssign,
            TokenKind::AsteriskEquals => ASTBinaryOperatorKind::MultiplyAssign,
            TokenKind::SlashEquals => ASTBinaryOperatorKind::DivideAssign,
            _ => unreachable!(),
        };
        let rhs = self.parse_binary_expr(0);
        let span = Span::merge(target.span, rhs.span);
        ASTExpr::assignment(target, op_kind, rhs, span)
    }

    fn parse_unary_expr(&mut self) -> ASTExpr {
        let mut ops = Vec::new();

        // 1. Sammle ALLE Prefix-Operatoren
        loop {
            match self.peek(0).kind {
                TokenKind::DoubleAnd => {
                    let tok = self.consume();

                    // Splitte in zwei '&'
                    ops.push(ASTUnaryOperator::new(
                        ASTUnaryOperatorKind::Ref,
                        Span::new(
                            tok.span.start,
                            tok.span.start + 1,
                            tok.span.end,
                            SpanSource::Source,
                        ),
                    ));

                    ops.push(ASTUnaryOperator::new(
                        ASTUnaryOperatorKind::Ref,
                        Span::new(
                            tok.span.start + 1,
                            tok.span.start,
                            tok.span.end,
                            SpanSource::Source,
                        ),
                    ));
                }
                TokenKind::And => {
                    let span = self.consume().span;
                    if self.peek(0).kind == TokenKind::Keyword(Keyword::Mut) {
                        self.advance(1);
                        ops.push(ASTUnaryOperator::new(ASTUnaryOperatorKind::RefMut, span));
                    } else {
                        ops.push(ASTUnaryOperator::new(ASTUnaryOperatorKind::Ref, span));
                    }
                }
                TokenKind::Asterisk => {
                    let span = self.consume().span;
                    ops.push(ASTUnaryOperator::new(ASTUnaryOperatorKind::Deref, span));
                }
                TokenKind::Minus => {
                    let span = self.consume().span;
                    ops.push(ASTUnaryOperator::new(ASTUnaryOperatorKind::Negate, span));
                }
                TokenKind::Exclamation => {
                    let span = self.consume().span;
                    ops.push(ASTUnaryOperator::new(ASTUnaryOperatorKind::Not, span));
                }
                _ => break,
            }
        }

        // 2. Parse das eigentliche Operand
        let mut expr = self.parse_primary_expr();

        // 3. Wende Operatoren RÜCKWÄRTS an
        for op in ops.into_iter().rev() {
            let span = Span::merge(op.span, expr.span);
            expr = ASTExpr::unary(op, expr, span);
        }

        expr
    }

    fn parse_cast_expr(&mut self) -> ASTExpr {
        let mut expr = self.parse_unary_expr();

        while self.peek(0).kind == TokenKind::Keyword(Keyword::As) {
            self.consume(); // 'as'

            let ty = self.parse_type();
            let end = self.backpeek(1).span;
            let span = Span::merge(expr.span, end);

            expr = ASTExpr::cast(expr, ty, span);
        }

        expr
    }

    fn parse_binary_expr(&mut self, precedence: u8) -> ASTExpr {
        let mut left = self.parse_cast_expr();

        loop {
            let op_token = self.peek(0);
            let op_kind = match &op_token.kind {
                TokenKind::Plus => ASTBinaryOperatorKind::Add,
                TokenKind::Minus => ASTBinaryOperatorKind::Subtract,
                TokenKind::Asterisk => ASTBinaryOperatorKind::Multiply,
                TokenKind::Slash => ASTBinaryOperatorKind::Divide,
                TokenKind::Percent => ASTBinaryOperatorKind::Remainder,

                // Vergleichsoperatoren
                TokenKind::DoubleEquals => ASTBinaryOperatorKind::Equal,
                TokenKind::ExclamationEquals => ASTBinaryOperatorKind::NotEqual,

                TokenKind::LAngle => ASTBinaryOperatorKind::Less,
                TokenKind::LAngleEquals => ASTBinaryOperatorKind::LessEqual,
                TokenKind::DoubleLAngle => ASTBinaryOperatorKind::LBitShift,

                TokenKind::RAngle => ASTBinaryOperatorKind::Greater,
                TokenKind::RAngleEquals => ASTBinaryOperatorKind::GreaterEqual,
                TokenKind::DoubleRAngle => ASTBinaryOperatorKind::RBitShift,

                TokenKind::And => ASTBinaryOperatorKind::BitAnd,
                TokenKind::Pipe => ASTBinaryOperatorKind::BitOr,
                TokenKind::Caret => ASTBinaryOperatorKind::LogicXor,

                TokenKind::DoubleAnd => ASTBinaryOperatorKind::LogicAnd,
                TokenKind::DoublePipe => ASTBinaryOperatorKind::LogicOr,

                _ => break,
            };

            let op = ASTBinaryOperator::new(op_kind, op_token.span);
            let op_prec = op.prec();
            if op_prec <= precedence {
                break;
            }

            // Consume Header Token
            self.advance(1);

            // right hand side Expression
            let right = self.parse_binary_expr(op_prec);
            let span = Span::merge(left.span, right.span);
            left = ASTExpr::binary(left, right, op, span);
        }

        left
    }

    /// Parst einen primären Ausdruck + alle Postfix-Operatoren.
    /// Postfix: .field, .method(args), (args), [index], ::segment
    fn parse_primary_expr(&mut self) -> ASTExpr {
        let mut expr = self.parse_atom();

        // Suffix-Loop
        loop {
            match self.peek(0).kind {
                // Feldzugriff oder Methodenaufruf:  expr.name  /  expr.name(args)
                TokenKind::Dot => {
                    let _ = self.consume();
                    let field_ident = self.consume_identifier();
                    let field = self.make_ident(&field_ident);
                    let field_span = field.span;

                    if self.peek(0).kind == TokenKind::LParen {
                        // Methodenaufruf: expr.method(arg1, arg2)
                        let (args, close) = self.parse_call_args();
                        let span = Span::merge(expr.span, close.span);
                        expr = ASTExpr::method_call(expr, field, args, span);
                    } else {
                        // Feldzugriff: expr.field
                        let span = Span::merge(expr.span, field_span);
                        expr = ASTExpr::field_access(expr, field, span);
                    }
                }

                // Freier Aufruf: expr(arg1, arg2)
                TokenKind::LParen => {
                    let (args, close) = self.parse_call_args();
                    let span = Span::merge(expr.span, close.span);
                    expr = ASTExpr::call(expr, args, span);
                }

                // Index-Zugriff: expr[idx]
                TokenKind::LBracket => {
                    self.advance(1); // '['
                    let idx = self.parse_expr();
                    let close = self.consume_check(TokenKind::RBracket);
                    let span = Span::merge(expr.span, close.span);
                    expr = ASTExpr::index(expr, idx, span);
                }

                // Pfad-Fortsetzung: expr::segment  (z.B. pkg::sub::func)
                // Nur sinnvoll wenn expr ein Pfad/Ident ist — Typchecker klärt den Rest
                TokenKind::DoubleColon => {
                    self.advance(1); // '::'
                    let ident_token = self.consume_identifier();
                    let ident = self.make_ident(&ident_token);
                    let span = Span::merge(expr.span, ident.span);
                    expr = ASTExpr::path_segment(expr, ident, span);
                }

                _ => break,
            }
        }

        expr
    }

    /// Parst eine Argumentliste: `( expr, expr, ... )`
    fn parse_call_args(&mut self) -> (Vec<ASTExpr>, Token) {
        self.consume_check(TokenKind::LParen);
        let mut args = Vec::new();

        while self.peek(0).kind != TokenKind::RParen && self.peek(0).kind != TokenKind::EndOfFile {
            args.push(self.parse_expr());
            if self.peek(0).kind == TokenKind::Comma {
                self.advance(1);
            } else {
                break;
            }
        }

        let close = self.consume_check(TokenKind::RParen);
        (args, close)
    }

    /// Das eigentliche Atom ohne Postfix (bisheriger parse_primary_expr-Body)
    fn parse_atom(&mut self) -> ASTExpr {
        let token = self.consume();
        match token.kind {
            TokenKind::Integer(v, suffix) => {
                if suffix.as_ref().is_some_and(NumSuffix::is_float) {
                    self.lexer.compiler.shared.reports.push(
                        Report::build(ReportKind::Error, token.span)
                            .with_message(format!(
                                "suffix `{}` is not valid on an integer literal",
                                suffix.as_ref().unwrap()
                            ))
                            .with_label(
                                Label::new(token.span)
                                    .with_message("expected integer suffix (u8, i32, ...)")
                                    .with_color(RED_COLOR),
                            )
                            .finish(),
                    );
                }
                ASTExpr::int(v, suffix, token.span)
            }
            TokenKind::Float(v, suffix) => {
                if suffix.as_ref().is_some_and(NumSuffix::is_integer) {
                    self.lexer.compiler.shared.reports.push(
                        Report::build(ReportKind::Error, token.span)
                            .with_message(format!(
                                "suffix `{}` is not valid on a float literal",
                                suffix.as_ref().unwrap()
                            ))
                            .with_label(
                                Label::new(token.span)
                                    .with_message("expected float suffix (f32, f64)")
                                    .with_color(RED_COLOR),
                            )
                            .finish(),
                    );
                }
                ASTExpr::float(v, suffix, token.span)
            }
            TokenKind::Byte(v) => ASTExpr::byte(v, token.span),
            TokenKind::Char(v) => ASTExpr::char(v, token.span),
            TokenKind::String(v) | TokenKind::RawString(v) => ASTExpr::string(v, token.span),
            TokenKind::ByteString(v) | TokenKind::RawByteString(v) => {
                ASTExpr::byte_string(v, token.span)
            }
            TokenKind::Keyword(Keyword::True) => ASTExpr::bool(true, token.span),
            TokenKind::Keyword(Keyword::False) => ASTExpr::bool(false, token.span),
            TokenKind::Identifier(ref name) => {
                if self.peek(0).kind == TokenKind::Exclamation {
                    let name = name.clone();
                    self.advance(1); // '!'

                    if self.expander.macros.contains_key(&name) {
                        let (raw_tokens, call_span) = self.collect_macro_args(token.span);

                        let result = {
                            let source_map = &mut self.lexer.compiler.sourcemap;
                            let ctx_table = &mut self.lexer.compiler.syntax_contexts;
                            self.expander.expand(
                                &name,
                                raw_tokens,
                                call_span,
                                SyntaxContext::ROOT,
                                source_map,
                                ctx_table,
                            )
                        };

                        match result {
                            Ok(expanded) => {
                                for tok in expanded.into_iter().rev() {
                                    self.buffer.push_front(tok);
                                }
                                return self.parse_expr();
                            }
                            Err(ExpandError::RecursionLimit) => {
                                self.lexer.compiler.shared.reports.push(
                                    Report::build(ReportKind::Error, call_span)
                                        .with_message(format!(
                                            "macro `{}!` exceeded recursion limit",
                                            name
                                        ))
                                        .finish(),
                                );
                                return ASTExpr::error(call_span.file_id);
                            }
                            Err(ExpandError::NoMatch(_)) => {
                                return ASTExpr::error(call_span.file_id);
                            }
                        }
                    }

                    // Fallback: Builtin-Macro
                    let (args, close) = self.parse_call_args();
                    let call_span = Span::merge(token.span, close.span);
                    return ASTExpr::macro_call(name, args, call_span);
                }
                ASTExpr::variable(name.clone(), token.span)
            }
            TokenKind::LParen => {
                // Unit-Tupel () oder geklammerten Ausdruck
                if self.peek(0).kind == TokenKind::RParen {
                    let close = self.consume();
                    return ASTExpr::unit(Span::merge(token.span, close.span));
                }
                let expr = self.parse_expr();
                // Tupel: (a, b, c)
                if self.peek(0).kind == TokenKind::Comma {
                    let mut elems = vec![expr];
                    while self.peek(0).kind == TokenKind::Comma {
                        self.advance(1);
                        if self.peek(0).kind == TokenKind::RParen {
                            break;
                        }
                        elems.push(self.parse_expr());
                    }
                    let close = self.consume_check(TokenKind::RParen);
                    let span = Span::merge(token.span, close.span);
                    return ASTExpr::tuple(elems, span);
                }
                let close = self.consume_check(TokenKind::RParen);
                ASTExpr::parenthesized(expr, Span::merge(token.span, close.span))
            }
            // WICHTIG: LCurly hier NICHT konsumieren — parse_block_expr macht das selbst.
            // Da token schon konsumiert ist, müssen wir den Span weitergeben.
            TokenKind::LCurly => self.parse_block_body(token),
            _ => {
                // token wurde bereits konsumiert, also token.span verwenden
                self.lexer.compiler.shared.reports.push(
                    Report::build(ReportKind::Error, token.span)
                        .with_message("unexpected token in type position")
                        .with_label(
                            Label::new(token.span)
                                .with_message("expected TYPE")
                                .with_color(RED_COLOR),
                        )
                        .finish(),
                );
                ASTExpr::error(token.span.file_id)
            }
        }
    }
}

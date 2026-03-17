use smallvec::{smallvec, SmallVec};

use super::lexer::{LexMode, Lexer};
use super::token::{Keyword, Token, TokenKind};
use super::{
    ASTBinaryOperator, ASTBinaryOperatorKind, ASTExpr, ASTStmt, ASTStructField,
    ASTTupleStructField, ASTType, ASTUnaryOperator, ASTUnaryOperatorKind, Mutability, Publicity,
};
use crate::ast::strings::{StringId, StringPool};
use crate::ast::{ASTEnumVariant, ASTEnumVariantKind, ASTGenericParam, Ident};
use crate::color::RED_COLOR;
use crate::reports::{Label, Report, ReportKind};

use crate::source::Span;
use crate::{Compiler, abort, args};
use std::collections::VecDeque;
use std::sync::atomic::Ordering;

pub struct Parser<'a> {
    lexer: Lexer<'a>,
    buffer: VecDeque<Token>,
    pub string_pool: StringPool,
    log_tokens: bool,
}

impl<'a> Parser<'a> {
    pub fn new(compiler: &'a mut Compiler, file_id: usize) -> Self {
        let lexer = Lexer::new(compiler, file_id);
        Self {
            lexer,
            buffer: VecDeque::with_capacity(4),
            string_pool: StringPool::new(),
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
        self.lexer.compiler.shared.reports.push(
            Report::build(ReportKind::Error, token.span)
                .with_message(format!("expected `{expected}`, found `{}`", token.kind))
                .with_label(
                    Label::new(token.span)
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
                .with_label(Label::new(Span::merge(starter, next))
                .with_color(RED_COLOR))
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
        if !matches!(tok.kind, TokenKind::Identifier(_)) {
            self.lexer.compiler.shared.reports.push(
                Report::build(ReportKind::Error, tok.span)
                    .with_message("expected identifier")
                    .with_label(
                        Label::new(tok.span)
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
            TokenKind::Identifier(name) => self.string_pool.intern(name),
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

        // Semicolons zwischen Statements ignorieren
        while self.peek(0).kind == TokenKind::Semicolon {
            self.advance(1);
        }

        if self.peek(0).kind == TokenKind::EndOfFile
            || self.peek(0).kind == TokenKind::Error
            || self.peek(0).kind == TokenKind::RCurly
        {
            return None;
        }

        Some(self.parse_stmt())
    }

    fn parse_stmt(&mut self) -> ASTStmt {
        let offset = if matches!(self.peek(0).kind, TokenKind::Keyword(Keyword::Pub)) {
            1
        } else {
            0
        };
        match &self.peek(offset).kind {
            TokenKind::Keyword(Keyword::Dec) => self.parse_var_dec_stmt(),
            TokenKind::Keyword(Keyword::Const) => self.parse_const_stmt(),
            TokenKind::Keyword(Keyword::Type) => self.parse_type_alias_stmt(),
            TokenKind::Keyword(Keyword::Struct) => self.parse_struct_stmt(),
            TokenKind::Keyword(Keyword::Enum) => self.parse_enum_stmt(),
            // TokenKind::Keyword(Keyword::Trait) => self.parse_trait_stmt(),
            // TokenKind::Keyword(Keyword::Func)   => self.parse_func_stmt(),
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
                    stmts.push(self.parse_var_dec_stmt());
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
                                .with_message("expected COMMA or end of declaration")
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

    fn parse_var_dec_stmt(&mut self) -> ASTStmt {
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
                let mut segments = vec![name];
                // pkg::super::Type
                while self.peek(0).kind == TokenKind::DoubleColon {
                    self.consume();
                    let seg = self.consume_identifier();
                    if let TokenKind::Identifier(s) = seg.kind {
                        segments.push(s);
                    }
                }
                if segments.len() == 1 {
                    ASTType::Path(segments[0].clone())
                } else {
                    ASTType::QualifiedPath(segments)
                }
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

            params.push(ASTGenericParam {
                name: ident,
                default,
            });

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
                    let ident_token = self.consume_identifier();
                    let ident = self.make_ident(&ident_token);
                    return self.parse_assignment(ident);
                }
                _ => {}
            }
        }

        // Other --> Binary Expr (incl. '==', '!=', '<=', '>=')
        self.parse_binary_expr(0)
    }

    fn parse_assignment(&mut self, ident: Ident) -> ASTExpr {
        // Operator lesen
        let op = self.consume();

        // RHS
        let rhs = self.parse_expr();

        // Span
        let span = Span::merge(ident.span, rhs.span);

        ASTExpr::assignment(ident, op.kind, rhs, span)
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
                        Span::new(tok.span.start, tok.span.start + 1, tok.span.end),
                    ));

                    ops.push(ASTUnaryOperator::new(
                        ASTUnaryOperatorKind::Ref,
                        Span::new(tok.span.start + 1, tok.span.start, tok.span.end),
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
            let _ = self.consume(); // 'as'

            // self.check_whitespace(expr.span, self.peek(0).span.clone());

            let ty = self.parse_type();
            let start = expr.span;
            let end = self.peek(0).span;

            expr = ASTExpr::cast(expr, ty, Span::merge(start, end));
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
            TokenKind::Integer(v) => ASTExpr::int(v, token.span),
            TokenKind::Float(v) => ASTExpr::float(v, token.span),
            TokenKind::Byte(v) => ASTExpr::byte(v, token.span),
            TokenKind::Char(v) => ASTExpr::char(v, token.span),
            TokenKind::String(v) | TokenKind::RawString(v) => ASTExpr::string(v, token.span),
            TokenKind::ByteString(v) | TokenKind::RawByteString(v) => {
                ASTExpr::byte_string(v, token.span)
            }
            TokenKind::Keyword(Keyword::True) => ASTExpr::bool(true, token.span),
            TokenKind::Keyword(Keyword::False) => ASTExpr::bool(false, token.span),
            TokenKind::Identifier(ref name) => {
                // Makro: name!( ... )
                if self.peek(0).kind == TokenKind::Exclamation
                    && self.peek(1).kind == TokenKind::LParen
                {
                    self.advance(1); // '!'
                    let (args, close) = self.parse_call_args();
                    let span = Span::merge(token.span, close.span);
                    return ASTExpr::macro_call(name.clone(), args, span);
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
                self.lexer.compiler.shared.reports.push(
                    Report::build(ReportKind::Error, token.span)
                        .with_message("unexpected expression")
                        .with_label(
                            Label::new(token.span)
                                .with_message("expected EXPRESSION")
                                .with_color(RED_COLOR),
                        )
                        .finish(),
                );
                ASTExpr::error(token.span.file_id)
            }
        }
    }
}

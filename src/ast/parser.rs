use smallvec::{smallvec, SmallVec};

use super::scope::Symbol;
use super::token::{Keyword, Token, TokenKind};
use super::{
    ASTBinaryOperator, ASTBinaryOperatorKind, ASTExpr, ASTStmt, ASTStructField,
    ASTTupleStructField, ASTUnaryOperator, ASTUnaryOperatorKind,
};
use crate::reports::{Label, Report, ReportKind};
use crate::types::{Primitive, Ty, TyKind};

use crate::source::Span;
use crate::{abort, Compiler};
use std::collections::VecDeque;

pub struct Parser<'a> {
    tokens: &'a [Token],
    index: usize,
    pushed_back: VecDeque<Token>,
    compiler: &'a mut Compiler,
}

impl<'a> Parser<'a> {
    pub fn new(tokens: &'a [Token], compiler: &'a mut Compiler) -> Self {
        Self {
            tokens,
            index: 0,
            pushed_back: VecDeque::new(),
            compiler,
        }
    }

    pub fn peek(&self, offset: isize) -> Token {
        if offset == 0 {
            if let Some(tok) = self.pushed_back.front() {
                return tok.clone();
            }
        }

        let base = self.index as isize + offset - self.pushed_back.len() as isize;

        let idx = base.clamp(0, (self.tokens.len() - 1) as isize) as usize;
        self.tokens[idx].clone()
    }

    pub fn advance(&mut self, n: usize) {
        self.index += n;
    }

    pub fn consume(&mut self) -> Token {
        if abort::is_aborted() {
            return Token {
                kind: TokenKind::EOF,
                span: self
                    .tokens
                    .last()
                    .map(|t| t.span)
                    .unwrap_or(Span::new(0, 0, 0)),
            };
        }

        if let Some(tok) = self.pushed_back.pop_front() {
            return tok;
        }

        self.index += 1;
        self.peek(-1)
    }

    pub fn consume_check(&mut self, expected: TokenKind) -> Token {
        let token = self.consume();
        if token.kind != expected {
            self.compiler.reports.push(
                Report::build(ReportKind::Error, token.span)
                    .with_message("unexpected token")
                    .with_label(Label::new(token.span).with_message("found here"))
                    .finish(),
            );
        }
        token
    }

    fn push_back(&mut self, token: Token) {
        self.pushed_back.push_front(token);
    }

    pub fn check_whitespace(&mut self, starter: Span, next: Span) {
        if next.start != starter.end {
            let span = Span::merge(starter, next);
            self.compiler.reports.push(
                Report::build(ReportKind::Warning, span)
                    .with_message("unexpected whitespace")
                    .with_label(Label::new(span))
                    .finish(),
            );
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

        if self.peek(0).kind == TokenKind::EOF
            || self.peek(0).kind == TokenKind::Error
            || self.peek(0).kind == TokenKind::RCurly
        {
            return None;
        }

        Some(self.parse_stmt())
    }

    fn parse_stmt(&mut self) -> ASTStmt {
        match self.peek(0).kind {
            TokenKind::Keyword(Keyword::Dec) => self.parse_declare_stmt(),
            _ => self.parse_expr_stmt(),
        }
    }

    fn parse_block_expr(&mut self) -> ASTExpr {
        let curly = self.consume(); // '{'
        self.compiler.scopes.push();

        let mut stmts = Vec::new();
        let mut tail_expr = None;

        while self.peek(0).kind != TokenKind::RCurly && self.peek(0).kind != TokenKind::EOF {
            match self.peek(0).kind {
                TokenKind::Keyword(Keyword::Dec) => {
                    // declarations are ALWAYS statements
                    stmts.push(self.parse_declare_stmt());
                }

                _ => {
                    let expr = self.parse_expr();

                    if self.peek(0).kind == TokenKind::Semicolon {
                        self.advance(1);
                        stmts.push(ASTStmt::expr(expr));
                    } else {
                        // no semicolon → must be tail expr
                        tail_expr = Some(expr);
                        break;
                    }
                }
            }
        }

        self.consume_check(TokenKind::RCurly);
        self.compiler.scopes.pop();

        let span = Span::merge(curly.span, self.peek(-1).span);
        ASTExpr::block(stmts, tail_expr, span)
    }

    fn parse_declare_stmt(&mut self) -> ASTStmt {
        self.advance(1); // consume 'dec' token

        if self.peek(0).kind == TokenKind::LParen {
            self.check_whitespace(self.peek(-1).span.clone(), self.peek(0).span.clone());
            match self.peek(1).kind {
                TokenKind::Keyword(Keyword::Struct) => self.parse_struct_dec(),
                // TokenKind::Keyword(Keyword::Enum) => self.parse_enum_declaration(),
                // TokenKind::Keyword(Keyword::Trait) => self.parse_trait_declaration(),
                // TokenKind::Keyword(Keyword::Type) => self.parse_type_declaration(),
                // TokenKind::Keyword(Keyword::Const) => self.parse_const_declaration(),
                _ => unreachable!(),
            }
        } else {
            self.parse_var_declaration()
        }
    }

    fn parse_optional_token(&mut self, kind: TokenKind) -> bool {
        if self.peek(0).kind == kind {
            self.advance(1);
            true
        } else {
            false
        }
    }

    fn parse_identifier(&mut self) -> Token {
        let tok = self.consume();
        if !matches!(tok.kind, TokenKind::Identifier(_)) {
            self.compiler.reports.push(
                Report::build(ReportKind::Error, tok.span)
                    .with_message("expected identifier")
                    .with_label(Label::new(tok.span).with_message("expected IDENTIFIER"))
                    .finish(),
            );
        }
        tok
    }

    fn parse_struct_field(&mut self) -> ASTStructField {
        let pub_ = self.parse_optional_token(TokenKind::Keyword(Keyword::Pub));

        // name
        let identifier = self.parse_identifier();

        // :
        self.consume_check(TokenKind::Colon);

        // type
        let type_ = self.parse_type();

        self.parse_field_end(TokenKind::RCurly);

        ASTStructField {
            identifier,
            pub_,
            type_,
        }
    }

    fn parse_tuple_struct_field(&mut self) -> ASTTupleStructField {
        let pub_ = self.parse_optional_token(TokenKind::Keyword(Keyword::Pub));

        // type
        let type_ = self.parse_type();

        self.parse_field_end(TokenKind::RParen);

        ASTTupleStructField { pub_, type_ }
    }

    fn parse_field_end(&mut self, end: TokenKind) {
        match self.peek(0).kind {
            TokenKind::Comma => {
                self.advance(1);
            }
            k if k == end => {
                // ok, letztes Feld
            }
            _ => {
                let span = self.peek(0).span;
                self.compiler.reports.push(
                    Report::build(ReportKind::Error, span)
                        .with_message("unexpected token")
                        .with_label(Label::new(span).with_message("expected COLON"))
                        .finish(),
                );
            }
        }
    }

    fn parse_struct_dec(&mut self) -> ASTStmt {
        // (struct)
        self.consume_check(TokenKind::LParen);
        self.consume_check(TokenKind::Keyword(Keyword::Struct));
        self.consume_check(TokenKind::RParen);

        let public = self.parse_optional_token(TokenKind::Keyword(Keyword::Pub));
        let identifier = self.parse_identifier();

        // optional generics: <T, U>
        let generics = if self.peek(0).kind == TokenKind::LAngle {
            self.check_whitespace(identifier.span.clone(), self.peek(0).span.clone());

            let args = self.parse_generic_args();

            // checkwhitespace between  '>' and '('
            if self.peek(0).kind == TokenKind::LParen {
                self.check_whitespace(
                    self.tokens[self.index - 1].span.clone(),
                    self.peek(0).span.clone(),
                );
            }

            args
        } else {
            Box::new([])
        };

        if self.peek(0).kind == TokenKind::LCurly {
            // {
            self.advance(1);

            // Fields
            let fields = self.parse_fields(TokenKind::RCurly, Self::parse_struct_field);

            // }
            self.consume_check(TokenKind::RCurly);

            // ;
            if self.peek(0).kind == TokenKind::Semicolon {
                self.advance(1);
            } else {
                let span = self.peek(0).span.clone();
                self.compiler.reports.push(
                    Report::build(ReportKind::Error, span)
                        .with_message("missing semicolon")
                        .with_label(Label::new(span).with_message("expected SEMICOLON"))
                        .finish(),
                );
            }

            ASTStmt::struct_dec(identifier, public, generics, fields)
        } else {
            // (
            self.consume_check(TokenKind::LParen);

            // Fields
            let fields = self.parse_fields(TokenKind::RParen, Self::parse_tuple_struct_field);

            // )
            self.consume_check(TokenKind::RParen);

            // ;
            if self.peek(0).kind == TokenKind::Semicolon {
                self.advance(1);
            } else {
                let span = self.peek(0).span;
                self.compiler.reports.push(
                    Report::build(ReportKind::Error, span)
                        .with_message("unexpected token")
                        .with_label(Label::new(span).with_message("expected COLON"))
                        .finish(),
                );
            }

            ASTStmt::tuple_struct_dec(identifier, public, generics, fields)
        }
    }

    fn parse_fields<F, T>(&mut self, end: TokenKind, parse_field: F) -> Box<[T]>
    where
        F: Fn(&mut Self) -> T,
    {
        let mut fields = Vec::new();

        while self.peek(0).kind != end && self.peek(0).kind != TokenKind::EOF {
            fields.push(parse_field(self));
        }

        fields.into_boxed_slice()
    }

    fn parse_var_declaration(&mut self) -> ASTStmt {
        let public = self.parse_optional_token(TokenKind::Keyword(Keyword::Pub));
        let mutable = self.parse_optional_token(TokenKind::Keyword(Keyword::Mut));

        // identifier: consume and validate
        let identifier_token = self.consume();
        match &identifier_token.kind.clone() {
            TokenKind::Identifier(name) => {
                let sym_id_pre = self.compiler.name_interner.intern(&name);
                let sym_id_opt = { self.compiler.scopes.lookup_current(sym_id_pre) };

                if let Some(sym_id) = sym_id_opt {
                    self.compiler.reports.push(
                        Report::build(ReportKind::Error, identifier_token.span)
                            .with_message("already defined")
                            .with_label(
                                Label::new(identifier_token.span).with_message("redefined here"),
                            )
                            .with_label(
                                Label::new(self.compiler.scopes.symbol_span(sym_id))
                                    .with_message("defined here"),
                            )
                            .finish(),
                    );
                }
            }
            _ => {
                self.compiler.reports.push(
                    Report::build(ReportKind::Error, identifier_token.span)
                        .with_message("unexpected token")
                        .with_label(
                            Label::new(identifier_token.span).with_message("expected IDENTIFIER"),
                        )
                        .finish(),
                );
            }
        }

        let ident_name = match &identifier_token.kind.clone() {
            TokenKind::Identifier(name) => name,
            _ => "__ERROR<Identifier>",
        };

        // Optional Type
        let mut type_ = None;
        if self.peek(0).kind == TokenKind::Colon {
            self.advance(1); // skip ':'
            type_ = Some(self.parse_type());
        }

        // require '='
        if self.peek(0).kind != TokenKind::Equals {
            let span = self.peek(0).span;
            self.compiler.reports.push(
                Report::build(ReportKind::Error, span)
                    .with_message("unexpected token")
                    .with_label(Label::new(span).with_message("expected EQUALS"))
                    .finish(),
            );
            self.advance(1);
        } else {
            self.advance(1); // consume '='
        }

        let expr = if self.peek(0).kind == TokenKind::LCurly {
            self.parse_block_expr()
        } else {
            self.parse_expr()
        };

        // Semicolon required here
        if self.peek(0).kind != TokenKind::Semicolon {
            let span = self.peek(0).span;
            self.compiler.reports.push(
                Report::build(ReportKind::Error, span)
                    .with_message("unexpected token")
                    .with_label(Label::new(span).with_message("expected SEMICOLON"))
                    .finish(),
            );
        } else {
            self.advance(1); // consume ';'
        }

        let _ = self.compiler.scopes.define_symbol(Symbol {
            name: self.compiler.name_interner.intern(ident_name),
            type_: type_
                .clone()
                .unwrap_or(self.compiler.ty_interner.intern(TyKind::Error)),
            mut_: mutable,
            pub_: public,
            span: identifier_token.span.clone(),
        });

        // ASTStmt zurückgeben
        ASTStmt::var_dec(identifier_token, public, mutable, type_, expr)
    }

    fn parse_type(&mut self) -> Ty {
        let ty = self.parse_type_atom();

        // Solange ein LAngle folgt, Generic-Argumente parsen
        if self.peek(0).kind == TokenKind::LAngle {
            self.check_whitespace(self.peek(-1).span.clone(), self.peek(0).span.clone());

            let name = match *self.compiler.ty_interner.kind(ty) {
                TyKind::Custom(n) => n,
                _ => {
                    let span = self.peek(-1).span;
                    self.compiler.reports.push(
                        Report::build(ReportKind::Error, span)
                            .with_message("invalid generic base")
                            .with_label(Label::new(span).with_message("expected IDENTIFIER"))
                            .finish(),
                    );
                    self.compiler.symbol_interner.intern("__ERROR".into())
                }
            };

            let args = self.parse_generic_args();
            return self
                .compiler
                .ty_interner
                .intern(TyKind::Generic { base: name, args });
        }

        ty
    }

    fn parse_type_atom(&mut self) -> Ty {
        if self.peek(0).kind == TokenKind::And {
            self.advance(1); // '&'

            if self.peek(0).kind == TokenKind::Keyword(Keyword::Mut) {
                self.advance(1);
                let inner = self.parse_type();
                return self.compiler.ty_interner.intern(TyKind::MutRef(inner));
            } else {
                let inner = self.parse_type();
                return self.compiler.ty_interner.intern(TyKind::Ref(inner));
            }
        }

        let token = self.consume();

        match token.kind {
            TokenKind::Type(tk) => tk,

            TokenKind::Identifier(name) => {
                // Primitive als Identifier erlaubt
                let prim = match name {
                    "i8" => Some(Primitive::I8),
                    "i16" => Some(Primitive::I16),
                    "i32" => Some(Primitive::I32),
                    "i64" => Some(Primitive::I64),
                    "i128" => Some(Primitive::I128),
                    "isize" => Some(Primitive::ISize),
                    "u8" => Some(Primitive::U8),
                    "u16" => Some(Primitive::U16),
                    "u32" => Some(Primitive::U32),
                    "u64" => Some(Primitive::U64),
                    "u128" => Some(Primitive::U128),
                    "usize" => Some(Primitive::USize),
                    "f32" => Some(Primitive::F32),
                    "f64" => Some(Primitive::F64),
                    "bool" => Some(Primitive::Bool),
                    "char" => Some(Primitive::Char),
                    "String" => Some(Primitive::String),
                    _ => None,
                };

                self.compiler
                    .ty_interner
                    .intern(prim.map(TyKind::Primitive).unwrap_or_else(|| {
                        // Leak the string once to get a 'static reference
                        let static_name: &'static str = name;
                        TyKind::Custom(self.compiler.symbol_interner.intern(static_name.into()))
                    }))
            }

            TokenKind::LParen => {
                let mut elems: SmallVec<[Ty; 2]> = SmallVec::new();

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
                self.compiler.ty_interner.intern(TyKind::Tuple(elems))
            }

            _ => {
                self.compiler.reports.push(
                    Report::build(ReportKind::Error, token.span)
                        .with_message("unexpected token")
                        .with_label(Label::new(token.span).with_message("expected TYPE"))
                        .finish(),
                );
                self.compiler.ty_interner.intern(TyKind::Custom(
                    self.compiler.symbol_interner.intern("__ERROR"),
                ))
            }
        }
    }

    fn parse_generic_args(&mut self) -> Box<[Ty]> {
        self.consume_check(TokenKind::LAngle);
        let mut elems: SmallVec<[Ty; 2]> = smallvec![];

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
                TokenKind::DoubleRAngle => {
                    let tok = self.consume();
                    self.push_back(Token {
                        kind: TokenKind::RAngle,
                        span: Span {
                            start: tok.span.start + 1,
                            end: tok.span.end,
                            file_id: tok.span.file_id,
                        },
                    });
                    break;
                }
                _ => {
                    let span = self.peek(0).span;
                    self.compiler.reports.push(
                        Report::build(ReportKind::Error, span)
                            .with_message("unexpected token")
                            .with_label(
                                Label::new(span).with_message("expected CLOSING ANGLE BRACKET"),
                            )
                            .finish(),
                    );

                    break;
                }
            }
        }

        elems.into_boxed_slice()
    }

    fn parse_expr_stmt(&mut self) -> ASTStmt {
        let expr = self.parse_expr();

        if self.peek(0).kind == TokenKind::Semicolon {
            self.advance(1);
            ASTStmt::expr(expr)
        } else {
            let span = self.peek(0).span;
            self.compiler.reports.push(
                Report::build(ReportKind::Error, span)
                    .with_message("missing semicolon")
                    .with_label(Label::new(span).with_message("expected SEMICOLON"))
                    .finish(),
            );

            ASTStmt::expr(expr)
        }
    }

    fn parse_expr(&mut self) -> ASTExpr {
        if let TokenKind::Identifier(_) = self.peek(0).kind {
            match self.peek(1).kind {
                TokenKind::Equals
                | TokenKind::PlusEquals
                | TokenKind::MinusEquals
                | TokenKind::AsteriskEquals
                | TokenKind::SlashEquals => {
                    return self.parse_assignment(self.peek(0));
                }
                _ => {}
            }
        }

        // Other --> Binary Expr (incl. '==', '!=', '<=', '>=')
        self.parse_binary_expr(0)
    }

    fn parse_assignment(&mut self, ident: Token) -> ASTExpr {
        // Symbol prüfen
        if let TokenKind::Identifier(ref name_str) = ident.kind {
            let sym_id_pre = self.compiler.name_interner.intern(name_str);

            match self.compiler.scopes.lookup_symbol(sym_id_pre) {
                Some(sym_id) => {
                    self.compiler.scopes.with_symbol(sym_id, |sym| {
                        if !sym.mut_ {
                            let span = ident.span;
                            self.compiler.reports.push(
                                Report::build(ReportKind::Error, span)
                                    .with_message("immutable variable")
                                    .with_label(Label::new(span).with_message("immutable"))
                                    .with_label(
                                        Label::new(sym.span)
                                            .with_message("consider: make variable mutable"),
                                    )
                                    .finish(),
                            );
                        }
                    });
                }
                None => {
                    self.compiler.reports.push(
                        Report::build(ReportKind::Error, ident.span)
                            .with_message("undefined variable")
                            .with_label(Label::new(ident.span))
                            .with_help("variables must be declared before using it")
                            .finish(),
                    );
                }
            }
        }

        self.consume();

        // Operator lesen
        let op_tok = self.consume();
        let op = op_tok.kind;

        // RHS
        let rhs = self.parse_expr();

        // Span
        let span = Span::merge(ident.span, rhs.span);

        ASTExpr::assignment(ident, op, rhs, span)
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
                        Token {
                            kind: TokenKind::And,
                            span: Span::new(tok.span.start, tok.span.start + 1, tok.span.end),
                        },
                    ));

                    ops.push(ASTUnaryOperator::new(
                        ASTUnaryOperatorKind::Ref,
                        Token {
                            kind: TokenKind::And,
                            span: Span::new(tok.span.start + 1, tok.span.start, tok.span.end),
                        },
                    ));
                }
                TokenKind::And => {
                    let tok = self.consume();
                    if self.peek(0).kind == TokenKind::Keyword(Keyword::Mut) {
                        self.consume();
                        ops.push(ASTUnaryOperator::new(ASTUnaryOperatorKind::RefMut, tok));
                    } else {
                        ops.push(ASTUnaryOperator::new(ASTUnaryOperatorKind::Ref, tok));
                    }
                }
                TokenKind::Asterisk => {
                    let tok = self.consume();
                    ops.push(ASTUnaryOperator::new(ASTUnaryOperatorKind::Deref, tok));
                }
                TokenKind::Minus => {
                    let tok = self.consume();
                    ops.push(ASTUnaryOperator::new(ASTUnaryOperatorKind::Negate, tok));
                }
                TokenKind::Exclamation => {
                    let tok = self.consume();
                    ops.push(ASTUnaryOperator::new(ASTUnaryOperatorKind::Not, tok));
                }
                _ => break,
            }
        }

        // 2. Parse das eigentliche Operand
        let mut expr = self.parse_primary_expr();

        // 3. Wende Operatoren RÜCKWÄRTS an
        for op in ops.into_iter().rev() {
            expr = ASTExpr::unary(
                op.clone(),
                expr.clone(),
                Span::merge(op.token.span, expr.span),
            )
        }

        expr
    }

    fn parse_cast_expr(&mut self) -> ASTExpr {
        let mut expr = self.parse_unary_expr();

        while self.peek(0).kind == TokenKind::Keyword(Keyword::As) {
            let _ = self.consume(); // 'as'
                                    // self.check_whitespace(expr.span.clone(), self.peek(0).span.clone());

            let ty = self.parse_type();
            let start = expr.span;
            let end = self.peek(-1).span;

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

            let op = ASTBinaryOperator::new(op_kind, op_token.clone());
            let op_prec = op.prec();
            if op_prec <= precedence {
                break;
            }

            // Consume Header Token
            self.advance(1);

            // right hand side Expression
            let right = self.parse_binary_expr(op_prec);
            left = ASTExpr::binary(left, right, op, op_token.span);
        }

        left
    }

    fn parse_primary_expr(&mut self) -> ASTExpr {
        let token = self.consume();

        return match token.kind {
            TokenKind::Integer(v) => ASTExpr::int(v, token.span),
            TokenKind::Float(v) => ASTExpr::float(v, token.span),
            TokenKind::Byte(v) => ASTExpr::byte(v, token.span),
            TokenKind::Char(v) => ASTExpr::char(v, token.span),
            TokenKind::String(v) | TokenKind::RawString(v) => ASTExpr::string(v, token.span),
            TokenKind::ByteString(v) | TokenKind::RawByteString(v) => {
                ASTExpr::byte_string(v, token.span)
            }
            TokenKind::Keyword(kw) => match kw {
                Keyword::True => ASTExpr::bool(true, token.span),
                Keyword::False => ASTExpr::bool(false, token.span),
                _ => ASTExpr::error(token.span.file_id),
            },
            TokenKind::Identifier(name) => {
                let sym_id = self.compiler.name_interner.intern(&name);
                if self.compiler.scopes.lookup_symbol(sym_id).is_none() {
                    self.compiler.reports.push(
                        Report::build(ReportKind::Error, token.span)
                            .with_message("unknown identifier")
                            .with_label(Label::new(token.span))
                            .finish(),
                    );
                    return ASTExpr::error(token.span.file_id);
                }
                ASTExpr::variable(name, token.span)
            }
            TokenKind::LParen => {
                let expr = self.parse_expr();
                let rparen = self.consume();
                if rparen.kind != TokenKind::RParen {
                    self.compiler.reports.push(
                        Report::build(ReportKind::Error, token.span)
                            .with_message("unexpected token")
                            .with_label(
                                Label::new(token.span).with_message("expected CLOSING PAREN"),
                            )
                            .finish(),
                    );

                    return ASTExpr::error(token.span.file_id);
                }

                ASTExpr::parenthesized(expr, Span::merge(token.span, rparen.span))
            }
            TokenKind::LCurly => self.parse_block_expr(),
            TokenKind::Error => {
                self.consume();
                ASTExpr::error(token.span.file_id)
            }
            _ => {
                dbg!(&token.kind);
                self.compiler.reports.push(
                    Report::build(ReportKind::Error, token.span)
                        .with_message("unexpected expression")
                        .with_label(Label::new(token.span).with_message("expected EXPRESSION"))
                        .finish(),
                );

                return ASTExpr::error(token.span.file_id);
            }
        };
    }
}

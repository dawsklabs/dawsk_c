use smallvec::{SmallVec, smallvec};

use super::scope::Symbol;
use super::token::{Keyword, Span, Token, TokenKind};
use super::types::{Primitive, Ty, TyKind};
use super::{
    ASTBinaryOperator, ASTBinaryOperatorKind, ASTExpr, ASTStmt, ASTStructField, ASTUnaryOperator,
    ASTUnaryOperatorKind,
};

use crate::{abort, Compiler};
use crate::diagnostics::{ DiagnosticBuilder, DiagnosticKind };

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;

pub struct Counter {
    index: Cell<usize>,
}

impl Counter {
    pub fn new() -> Self {
        Self {
            index: Cell::new(0),
        }
    }

    pub fn incr(&self, n: usize) {
        self.index.set(self.index.get() + n);
    }

    pub fn get(&self) -> usize {
        self.index.get()
    }
}

pub struct Parser<'a> {
    tokens: Vec<Token>,
    index: Counter,
    pushed_back: RefCell<VecDeque<Token>>,
    compiler: &'a mut Compiler
}

impl<'a> Parser<'a> {
    pub fn new(
        tokens: Vec<Token>,
        compiler: &'a mut Compiler,
    ) -> Self {
        Self {
            tokens,
            index: Counter::new(),
            pushed_back: RefCell::new(VecDeque::new()),
            compiler,
        }
    }

    pub fn peek(&self, offset: isize) -> Token {
        if offset == 0 {
            if let Some(tok) = self.pushed_back.borrow().front() {
                return tok.clone();
            }
        }

        let base = self.index.get() as isize + offset - self.pushed_back.borrow().len() as isize;

        let idx = base.clamp(0, (self.tokens.len() - 1) as isize) as usize;
        self.tokens[idx].clone()
    }

    pub fn advance(&self, n: usize) {
        self.index.incr(n);
    }

    pub fn consume(&self) -> Token {
        if abort::is_aborted() {
            return Token {
                kind: TokenKind::EOF,
                span: Span { start: 0, end: 0 },
            };
        }

        if let Some(tok) = self.pushed_back.borrow_mut().pop_front() {
            return tok;
        }

        self.index.incr(1);
        self.peek(-1)
    }

    pub fn consume_check(&self, expected: TokenKind) -> Token {
        let token = self.consume();
        if token.kind != expected {
            self.compiler.diagnostics.push(
                DiagnosticBuilder::error(
                    DiagnosticKind::UnexpectedToken {
                        given: token.kind.clone(),
                    },
                    token.span.clone(),
                )
                .build(),
            )
        }
        token
    }

    fn push_back(&self, token: Token) {
        self.pushed_back.borrow_mut().push_front(token);
    }

    pub fn check_whitespace(&self, starter: Span, next: Span) {
        if next.start != starter.end {
            self.compiler.diagnostics.push(
                DiagnosticBuilder::error(
                    DiagnosticKind::UnexpectedWhitespace,
                    Span {
                        start: starter.end,
                        end: next.start,
                    },
                )
                .build(),
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
        let token = self.peek(0);
        if let TokenKind::Unknown(_) = &token.kind {
            self.compiler.diagnostics.push(
                DiagnosticBuilder::error(
                    DiagnosticKind::UnexpectedToken {
                        given: token.kind.clone(),
                    },
                    self.peek(0).span,
                )
                .build(),
            );
        }

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

        let start = curly.span.start;
        let end = self.peek(-1).span.end;

        ASTExpr::block(stmts, tail_expr, Span { start, end })
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
                _ => panic!("Unexpected token after 'dec'"),
            }
        } else {
            self.parse_var_declaration()
        }
    }

    fn parse_struct_dec(&mut self) -> ASTStmt {
        // (struct)
        self.consume_check(TokenKind::LParen);
        self.consume_check(TokenKind::Keyword(Keyword::Struct));
        self.consume_check(TokenKind::RParen);

        // optional pub
        let public = if self.peek(0).kind == TokenKind::Keyword(Keyword::Pub) {
            self.advance(1);
            true
        } else {
            false
        };

        // Struct-Name
        let identifier = self.consume();
        if !matches!(identifier.kind, TokenKind::Identifier(_)) {
            self.compiler.diagnostics.push(
                DiagnosticBuilder::error(
                    DiagnosticKind::ExpectedToken {
                        expected: vec!["Identifier".into()],
                    },
                    self.peek(0).span,
                )
                .build(),
            );
        }

        // optionale Generics: <T, U>
        let generics = if self.peek(0).kind == TokenKind::LAngle {
            self.check_whitespace(identifier.span.clone(), self.peek(0).span.clone());

            let args = self.parse_generic_args();

            // Prüfe Whitespace zwischen '>' und '('
            if self.peek(0).kind == TokenKind::LParen {
                self.check_whitespace(
                    self.tokens[self.index.get() - 1].span.clone(),
                    self.peek(0).span.clone(),
                );
            }

            Some(args)
        } else {
            None
        };

        if self.peek(0).kind == TokenKind::LCurly {
            // {
            self.advance(1);

            // Fields
            let mut fields = Vec::new();
            while self.peek(0).kind != TokenKind::RCurly && self.peek(0).kind != TokenKind::EOF {
                fields.push(self.parse_struct_field());
            }

            // }
            self.consume_check(TokenKind::RCurly);

            // ;
            if self.peek(0).kind == TokenKind::Semicolon {
                self.advance(1);
            } else {
                let span = self.peek(0).span.clone();

                self.compiler.diagnostics.push(
                    DiagnosticBuilder::error(DiagnosticKind::MissingSemicolon, span.clone())
                        .label(span.clone(), "expected `;` here")
                        .suggestion(span, ";", "add a semicolon")
                        .build(),
                );
            }

            ASTStmt::struct_dec(identifier, public, generics, fields)
        } else {
            // (
            self.consume_check(TokenKind::LParen);

            // Fields
            let mut fields = SmallVec::new();
            if self.peek(0).kind != TokenKind::RParen {
                loop {
                    fields.push(self.parse_type());

                    match self.peek(0).kind {
                        TokenKind::Comma => {
                            self.advance(1);

                            // trailing comma erlaubt
                            if self.peek(0).kind == TokenKind::RParen {
                                break;
                            }
                        }
                        TokenKind::RParen => break,
                        _ => {
                            self.compiler.diagnostics.push(
                                DiagnosticBuilder::error(
                                    DiagnosticKind::ExpectedToken {
                                        expected: vec![
                                            "Closing/Right Parenthesis ')'".into(),
                                            "Comma ','".into(),
                                        ],
                                    },
                                    self.peek(0).span,
                                )
                                .build(),
                            );
                            break;
                        }
                    }
                }
            }

            // )
            self.consume_check(TokenKind::RParen);

            // ;
            if self.peek(0).kind == TokenKind::Semicolon {
                self.advance(1);
            } else {
                self.compiler.diagnostics.push(
                    DiagnosticBuilder::error(
                        DiagnosticKind::ExpectedToken {
                            expected: vec!["Colon ':'".into()],
                        },
                        self.peek(0).span,
                    )
                    .build(),
                );
            }

            ASTStmt::tuple_struct_dec(identifier, public, generics, fields)
        }
    }

    fn parse_struct_field(&mut self) -> ASTStructField {
        // optional pub
        let pub_ = if self.peek(0).kind == TokenKind::Keyword(Keyword::Pub) {
            self.advance(1);
            true
        } else {
            false
        };

        // Feldname
        let identifier = self.consume();
        if !matches!(identifier.kind, TokenKind::Identifier(_)) {
            self.compiler.diagnostics.push(
                DiagnosticBuilder::error(
                    DiagnosticKind::ExpectedToken {
                        expected: vec!["Identifier".into()],
                    },
                    self.peek(0).span,
                )
                .build(),
            );
        }

        // :
        self.consume_check(TokenKind::Colon);

        // Typ
        let type_ = self.parse_type();

        // ,
        match self.peek(0).kind {
            TokenKind::Comma => {
                self.advance(1);
            }
            TokenKind::RCurly => {
                // ok: letztes Feld ohne Komma
            }
            _ => {
                self.compiler.diagnostics.push(
                    DiagnosticBuilder::error(
                        DiagnosticKind::ExpectedToken {
                            expected: vec!["Comma ','".into()],
                        },
                        self.peek(0).span,
                    )
                    .build(),
                );
            }
        }

        ASTStructField {
            identifier,
            pub_,
            type_,
        }
    }

    fn parse_var_declaration(&mut self) -> ASTStmt {
        let mut public = false;
        let mut mutable = false;

        if self.peek(0).kind == TokenKind::Keyword(Keyword::Pub) {
            // pub (gr.: public)
            public = true;
            self.advance(1);
        }

        if self.peek(0).kind == TokenKind::Keyword(Keyword::Mut) {
            mutable = true;
            self.advance(1);
        }

        // identifier: consume and validate
        let identifier_token = self.consume();
        match &identifier_token.kind.clone() {
            TokenKind::Identifier(name) => {
                let sym_id_pre = self.compiler.name_interner.intern(&name);
                let sym_id_opt = {
                    self.compiler.scopes.lookup_current(sym_id_pre)
                };

                if let Some(sym_id) = sym_id_opt {
                    self.compiler.diagnostics.push(
                        DiagnosticBuilder::error(
                            DiagnosticKind::AlreadyDefined {
                                name: name.to_string(),
                            },
                            identifier_token.span.clone(),
                        )
                        .help(self.compiler.scopes.symbol_span(sym_id), "Already defined here!")
                        .build(),
                    );
                }
            }
            _ => {
                self.compiler.diagnostics.push(
                    DiagnosticBuilder::error(
                        DiagnosticKind::UnexpectedToken {
                            given: identifier_token.kind.clone(),
                        },
                        identifier_token.span.clone(),
                    )
                    .build(),
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
            self.compiler.diagnostics.push(
                DiagnosticBuilder::error(
                    DiagnosticKind::ExpectedToken {
                        expected: vec!["Equals '='".to_string()],
                    },
                    self.peek(0).span.clone(),
                )
                .build(),
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
            self.compiler.diagnostics.push(
                DiagnosticBuilder::error(
                    DiagnosticKind::ExpectedToken {
                        expected: vec!["Semicolon ';'".to_string()],
                    },
                    self.peek(0).span.clone(),
                )
                .build(),
            );
        } else {
            self.advance(1); // consume ';'
        }

        let _ = self.compiler.scopes.define(Symbol {
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
                    self.compiler.diagnostics.push(
                        DiagnosticBuilder::error(
                            DiagnosticKind::InvalidGenericBase,
                            self.peek(-1).span.clone(),
                        )
                        .build(),
                    );
                    self.compiler.symbol_interner.intern("__ERROR".into())
                }
            };

            let args = self.parse_generic_args();
            return self.compiler.ty_interner.intern(TyKind::Generic { base: name, args });
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

                self.compiler.ty_interner
                    .intern(prim.map(TyKind::Primitive).unwrap_or_else(|| {
                        // Leak the string once to get a 'static reference
                        let static_name: &'static str = name;
                        TyKind::Custom(self.compiler.symbol_interner.intern(static_name.into()))
                    }
                ))
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
                self.compiler.diagnostics.push(
                    DiagnosticBuilder::error(
                        DiagnosticKind::ExpectedToken {
                            expected: vec!["Type".into()],
                        },
                        token.span,
                    )
                    .build(),
                );
                self.compiler.ty_interner.intern(TyKind::Custom(self.compiler.symbol_interner.intern("__ERROR")))
            }
        }
    }

    fn parse_generic_args(&mut self) -> SmallVec<[Ty; 2]> {
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
                TokenKind::DoubleRAngle => {
                    let tok = self.consume();
                    self.push_back(Token {
                        kind: TokenKind::RAngle,
                        span: Span {
                            start: tok.span.start + 1,
                            end: tok.span.end,
                        },
                    });
                    break;
                }
                _ => {
                    self.compiler.diagnostics.push(
                        DiagnosticBuilder::error(
                            DiagnosticKind::ExpectedToken {
                                expected: vec![">".into()],
                            },
                            self.peek(0).span.clone(),
                        )
                        .build(),
                    );
                    break;
                }
            }
        }

        elems
    }

    fn parse_expr_stmt(&mut self) -> ASTStmt {
        let expr = self.parse_expr();

        if self.peek(0).kind == TokenKind::Semicolon {
            self.advance(1);
            ASTStmt::expr(expr)
        } else {
            self.compiler.diagnostics.push(
                DiagnosticBuilder::error(
                    DiagnosticKind::UnexpectedToken {
                        given: self.peek(0).kind.clone(),
                    },
                    self.peek(0).span,
                )
                .build(),
            );
            ASTStmt::expr(expr)
        }
    }

    fn parse_expr(&mut self) -> ASTExpr {
        if let TokenKind::Identifier(ref name) = self.peek(0).kind {
            match self.peek(1).kind {
                TokenKind::Equals
                | TokenKind::PlusEquals
                | TokenKind::MinusEquals
                | TokenKind::AsteriskEquals
                | TokenKind::SlashEquals => {
                    return self.parse_assignment(name.to_string());
                }
                _ => {}
            }
        }

        // Other --> Binary Expr (incl. '==', '!=', '<=', '>=')
        self.parse_binary_expr(0)
    }

    fn parse_assignment(&mut self, name: String) -> ASTExpr {
        if let TokenKind::Identifier(ref name) = self.consume().kind {
            let sym_id_pre = self.compiler.name_interner.intern(&name);
            match self.compiler.scopes.lookup(sym_id_pre) {
                Some(sym_id) => {
                    self.compiler.scopes.with_symbol(sym_id, |sym| {
                        if !sym.mut_ {
                            self.compiler.diagnostics.push(
                                DiagnosticBuilder::error(
                                    DiagnosticKind::ImmutableVariable,
                                    self.peek(-1).span,
                                )
                                .help(
                                    sym.span.clone(),
                                    "Variable defined here. Consider making it mutable!",
                                )
                                .build(),
                            );
                        }
                    });
                }
                None => {
                    self.compiler.diagnostics.push(
                        DiagnosticBuilder::error(
                            DiagnosticKind::UnknownIdentifier {
                                identifier: name.to_string(),
                            },
                            self.peek(-1).span,
                        )
                        .note("Variables must be declared before assigning a value!")
                        .build(),
                    );
                }
            }
        }

        // Consume Operator Token
        let op = self.consume().kind;

        let rhs = self.parse_expr();

        let start = self.peek(-2).span.start; // Identifier
        let end = rhs.span.end;

        ASTExpr::assignment(name, op, rhs, Span { start, end })
    }

    fn parse_unary_expr(&mut self) -> ASTExpr {
        match self.peek(0).kind {
            TokenKind::Minus | TokenKind::Exclamation | TokenKind::And | TokenKind::Asterisk => {
                let tok = self.consume();
                let start = tok.span.start;

                let kind = match tok.kind {
                    TokenKind::Minus => ASTUnaryOperatorKind::Negate,
                    TokenKind::Exclamation => ASTUnaryOperatorKind::Not,
                    TokenKind::And => {
                        if self.peek(0).kind == TokenKind::Keyword(Keyword::Mut) {
                            self.advance(1);
                            ASTUnaryOperatorKind::RefMut
                        } else {
                            ASTUnaryOperatorKind::Ref
                        }
                    }
                    TokenKind::Asterisk => ASTUnaryOperatorKind::Deref,
                    _ => unreachable!(),
                };

                let op = ASTUnaryOperator::new(kind, tok);
                let expr = self.parse_unary_expr();
                let end = expr.span.end;

                ASTExpr::unary(op, expr, Span { start, end })
            }
            _ => self.parse_primary_expr(),
        }
    }

    fn parse_cast_expr(&mut self) -> ASTExpr {
        let mut expr = self.parse_unary_expr();

        while self.peek(0).kind == TokenKind::Keyword(Keyword::As) {
            let _ = self.consume(); // 'as'
            // self.check_whitespace(expr.span.clone(), self.peek(0).span.clone());

            let ty = self.parse_type();
            let start = expr.span.start;
            let end = self.peek(-1).span.end;

            expr = ASTExpr::cast(expr, ty, Span { start, end });
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
                TokenKind::Percent => ASTBinaryOperatorKind::Modulus,

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
                _ => ASTExpr::error(),
            },
            TokenKind::Identifier(name) => {
                let sym_id = self.compiler.name_interner.intern(&name);
                if self.compiler.scopes.lookup(sym_id).is_none() {
                    self.compiler.diagnostics.push(
                        DiagnosticBuilder::error(
                            DiagnosticKind::UnknownIdentifier {
                                identifier: name.to_string(),
                            },
                            token.span.clone(),
                        )
                        .label(
                            token.span.clone(),
                            "unknown identifier or variable not defined",
                        )
                        .build(),
                    );
                    return ASTExpr::error();
                }
                ASTExpr::variable(name, token.span)
            }
            TokenKind::LParen => {
                let expr = self.parse_expr();
                let rparen = self.consume();
                if rparen.kind != TokenKind::RParen {
                    self.compiler.diagnostics.push(
                        DiagnosticBuilder::error(
                            DiagnosticKind::UnexpectedToken {
                                given: token.kind.clone(),
                            },
                            token.span.clone(),
                        )
                        .label(token.span, "expected ')'")
                        .build(),
                    );
                    return ASTExpr::error();
                }
                let start = token.span.start;
                let end = rparen.span.end;

                ASTExpr::parenthesized(expr, Span { start, end })
            }
            TokenKind::LCurly => self.parse_block_expr(),
            TokenKind::Error => {
                self.consume();
                ASTExpr::error()
            }
            _ => {
                dbg!(&token.kind);
                self.compiler.diagnostics.push(
                    DiagnosticBuilder::error(
                        DiagnosticKind::ExpectedExpression,
                        token.span.clone(),
                    )
                    .build(),
                );
                return ASTExpr::error();
            }
        };
    }
}

use super::token::{ Token, TokenKind, Keyword, Span };
use super::{ ASTStmt, ASTStructField, ASTExpr, ASTBinaryOperator, ASTBinaryOperatorKind, ASTUnaryOperator, ASTUnaryOperatorKind };
use super::types::{ TypeKind, Primitive };
use super::scope::{ScopeStack, Symbol};

use crate::{ Diagnostic, DiagnosticType, DiagnosticKind, DiagnosticBagCell };
use crate::abort;

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;

pub struct Counter {
    index: Cell<usize>,
}

impl Counter {
    pub fn new() -> Self {
        Self { index: Cell::new(0) }
    }

    pub fn incr(&self, n: usize) {
        self.index.set(self.index.get() + n);
    }

    pub fn get(&self) -> usize {
        self.index.get()
    }
}

pub struct Parser {
    tokens: Vec<Token>,
    index: Counter,
    diagnostics_bag: DiagnosticBagCell,
    scopes: ScopeStack,
    pushed_back: RefCell<VecDeque<Token>>,
}

impl Parser {
    pub fn new(tokens: Vec<Token>, diagnostics_bag: DiagnosticBagCell) -> Self {
        Self {
            tokens,
            index: Counter::new(),
            diagnostics_bag,
            scopes: ScopeStack::new(),
            pushed_back: RefCell::new(VecDeque::new()),
        }
    }

    pub fn peek(&self, offset: isize) -> Token {
        if offset == 0 {
            if let Some(tok) = self.pushed_back.borrow().front() {
                return tok.clone();
            }
        }

        let base = self.index.get() as isize + offset
            - self.pushed_back.borrow().len() as isize;

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
                span: Span {
                    start: 0,
                    end: 0,
                },
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
            self.diagnostics_bag.add(
                Diagnostic::new(
                    DiagnosticType::Error(DiagnosticKind::UnexpectedToken { 
                        given: token.kind.clone(),
                    }),
                    token.span.clone(),
                )
            );
        }
        token
    }

    fn push_back(&self, token: Token) {
        self.pushed_back.borrow_mut().push_front(token);
    }

    pub fn expect<F>(&self, check: F, expected_name: String, offset: isize) -> bool
    where
        F: Fn(&TokenKind) -> bool,
    {
        let token = self.peek(offset);
        if !check(&token.kind) {
            self.diagnostics_bag.add(
                Diagnostic::new(
                    DiagnosticType::Error(DiagnosticKind::UnexpectedToken {
                        given: token.kind.clone(),
                    }),
                    token.span.clone(),
                )
            );
            self.diagnostics_bag.add(
                Diagnostic::new(
                    DiagnosticType::Error(DiagnosticKind::ExpectedToken {
                        expected: vec![expected_name], // nur Name für Meldung
                    }),
                    token.span.clone(),
                )
            );
            return false;
        }
        true
    }

    pub fn check_whitespace(&self, starter: Span, next: Span) {
        if next.start != starter.end {
            self.diagnostics_bag.add(Diagnostic::new(
                DiagnosticType::Error(DiagnosticKind::UnexpectedWhitespace),
                Span {
                    start: starter.end,
                    end: next.start,
                },
            ));
        }
    }

    pub fn next_stmt(&self) -> Option<ASTStmt> {
        if abort::is_aborted() {
            return None;
        }

        // Semicolons zwischen Statements ignorieren
        while self.peek(0).kind == TokenKind::Semicolon {
            self.consume();
        }

        if self.peek(0).kind == TokenKind::EOF
            || self.peek(0).kind == TokenKind::RCurly
        {
            return None;
        }

        Some(self.parse_stmt())
    }

    fn parse_stmt(&self) -> ASTStmt {
        let token = self.peek(0);
        if let TokenKind::Unknown(c) = &token.kind {
            self.diagnostics_bag.add(
                Diagnostic::new(
                    DiagnosticType::Error(
                        DiagnosticKind::UnknownIdentifier { identifier: c.to_string() }
                    ),
                    token.span.clone()
                )
            );
        }

        match self.peek(0).kind {
            TokenKind::Keyword(Keyword::Dec) => self.parse_declare_stmt(),
            _ => self.parse_expr_stmt(),
        }
    }

    fn parse_block_expr(&self) -> ASTExpr {
        self.consume(); // '{'
        self.scopes.push();

        let mut stmts = Vec::new();
        let mut tail_expr = None;

        while self.peek(0).kind != TokenKind::RCurly
            && self.peek(0).kind != TokenKind::EOF
        {
            match self.peek(0).kind {
                TokenKind::Keyword(Keyword::Dec) => {
                    // declarations are ALWAYS statements
                    stmts.push(self.parse_declare_stmt());
                }

                _ => {
                    let expr = self.parse_expr();

                    if self.peek(0).kind == TokenKind::Semicolon {
                        self.consume();
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
        self.scopes.pop();

        ASTExpr::block(stmts, tail_expr)
    }
    
    fn parse_declare_stmt(&self) -> ASTStmt {
        self.consume(); // consume 'dec' token

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

    fn parse_struct_dec(&self) -> ASTStmt {
        // (struct)
        self.consume_check(TokenKind::LParen);
        self.consume_check(TokenKind::Keyword(Keyword::Struct));
        self.consume_check(TokenKind::RParen);

        // optional pub
        let public = if self.peek(0).kind == TokenKind::Keyword(Keyword::Pub) {
            self.consume();
            true
        } else {
            false
        };

        // Struct-Name
        let identifier = self.consume();
        if !matches!(identifier.kind, TokenKind::Identifier(_)) {
            self.diagnostics_bag.add(Diagnostic::new(
                DiagnosticType::Error(DiagnosticKind::ExpectedToken {
                    expected: vec!["Struct identifier".into()],
                }),
                identifier.span.clone(),
            ));
        }

        // optionale Generics: <T, U>
        let generics = if self.peek(0).kind == TokenKind::LAngle {
            self.check_whitespace(identifier.span.clone(), self.peek(0).span.clone());

            let args = self.parse_generic_args();

            // Prüfe Whitespace zwischen '>' und '('
            if self.peek(0).kind == TokenKind::LParen {
                self.check_whitespace(self.tokens[self.index.get() - 1].span.clone(), self.peek(0).span.clone());
            }

            Some(args)
        } else {
            None
        };

        if self.peek(0).kind == TokenKind::LCurly {
            // {
            self.consume();

            // Fields
            let mut fields = Vec::new();
            while self.peek(0).kind != TokenKind::RCurly
                && self.peek(0).kind != TokenKind::EOF
            {
                fields.push(self.parse_struct_field());
            }

            // }
            self.consume_check(TokenKind::RCurly);

            // ;
            if self.peek(0).kind == TokenKind::Semicolon {
                self.consume();
            } else {
                self.diagnostics_bag.add(Diagnostic::new(
                    DiagnosticType::Error(DiagnosticKind::ExpectedToken {
                        expected: vec!["Semicolon ';'".into()],
                    }),
                    self.peek(0).span.clone(),
                ));
            }

            ASTStmt::structDec(identifier, public, generics, fields)
        } else {
            // (
            self.consume_check(TokenKind::LParen);

            // Fields
            let mut fields = Vec::new();
            if self.peek(0).kind != TokenKind::RParen {
                loop {
                    fields.push(self.parse_type());

                    match self.peek(0).kind {
                        TokenKind::Comma => {
                            self.consume();

                            // trailing comma erlaubt
                            if self.peek(0).kind == TokenKind::RParen {
                                break;
                            }
                        }
                        TokenKind::RParen => break,
                        _ => {
                            self.diagnostics_bag.add(Diagnostic::new(
                                DiagnosticType::Error(DiagnosticKind::ExpectedToken {
                                    expected: vec![",".into(), ")".into()],
                                }),
                                self.peek(0).span.clone(),
                            ));
                            break;
                        }
                    }
                }
            }

            // )
            self.consume_check(TokenKind::RParen);

            // ;
            if self.peek(0).kind == TokenKind::Semicolon {
                self.consume();
            } else {
                self.diagnostics_bag.add(Diagnostic::new(
                    DiagnosticType::Error(DiagnosticKind::ExpectedToken {
                        expected: vec!["Semicolon ';'".into()],
                    }),
                    self.peek(0).span.clone(),
                ));
            }

            ASTStmt::tupleStructDec(identifier, public, generics, fields)
        }
    }

    fn parse_struct_field(&self) -> ASTStructField {
        // optional pub
        let pub_ = if self.peek(0).kind == TokenKind::Keyword(Keyword::Pub) {
            self.consume();
            true
        } else {
            false
        };

        // Feldname
        let identifier = self.consume();
        if !matches!(identifier.kind, TokenKind::Identifier(_)) {
            self.diagnostics_bag.add(Diagnostic::new(
                DiagnosticType::Error(DiagnosticKind::ExpectedToken {
                    expected: vec!["Field identifier".into()],
                }),
                identifier.span.clone(),
            ));
        }

        // :
        self.consume_check(TokenKind::Colon);

        // Typ
        let type_ = self.parse_type();

        // ,
        match self.peek(0).kind {
            TokenKind::Comma => {
                self.consume();
            }
            TokenKind::RCurly => {
                // ok: letztes Feld ohne Komma
            }
            _ => {
                self.diagnostics_bag.add(Diagnostic::new(
                    DiagnosticType::Error(DiagnosticKind::ExpectedToken {
                        expected: vec![",".into(), "}".into()],
                    }),
                    self.peek(0).span.clone(),
                ));
            }
        }

        ASTStructField {
            identifier,
            pub_,
            type_,
        }
    }

    fn parse_var_declaration(&self) -> ASTStmt {
        let mut public = false;
        let mut mutable = false;

        if self.peek(0).kind == TokenKind::Keyword(Keyword::Pub) { // pub (gr.: public)
            public = true;
            self.consume();
        }

        if self.peek(0).kind == TokenKind::Keyword(Keyword::Mut) {
            mutable = true;
            self.consume();
        }

        // identifier: consume and validate
        let identifier_token = self.consume();
        match &identifier_token.kind {
            TokenKind::Identifier(name) => {
                if self.scopes.lookup(&name).is_some() {
                    self.diagnostics_bag.add(
                        Diagnostic::new(
                            DiagnosticType::Error(DiagnosticKind::AlreadyDefined {
                                name: name.to_string(),
                            }),
                            identifier_token.span.clone(),
                        )
                    );
                }
            },
            _ => {
                self.diagnostics_bag.add(Diagnostic::new(
                    DiagnosticType::Error(DiagnosticKind::ExpectedToken {
                        expected: vec!["Identifier".to_string()]
                    }),
                    identifier_token.span.clone()
                ));
            }
        }

        let ident_name = match &identifier_token.kind {
            TokenKind::Identifier(name) => name.clone(),
            _ => "__ERROR<Identifier>".to_string(),
        };

        // Optional Type
        let mut type_ = None;
        if self.peek(0).kind == TokenKind::Colon {
            self.consume(); // skip ':'
            type_ = Some(self.parse_type());
        }

        // require '='
        if self.peek(0).kind != TokenKind::Equals {
            self.diagnostics_bag.add(Diagnostic::new(
                DiagnosticType::Error(DiagnosticKind::UnexpectedToken {
                    given: self.peek(0).kind
                }),
                self.peek(0).span.clone()
            ));
            if self.peek(0).kind == TokenKind::Equals { } else { self.consume(); }
        } else {
            self.consume(); // consume '='
        }
        
        let expr = if self.peek(0).kind == TokenKind::LCurly {
            self.parse_block_expr()
        } else {
            self.parse_expr()
        };

        // Semicolon required here
        if self.peek(0).kind != TokenKind::Semicolon {
            self.diagnostics_bag.add(Diagnostic::new(
                DiagnosticType::Error(DiagnosticKind::ExpectedToken {
                    expected: vec!["Semicolon ';'".to_string()],
                }),
                self.peek(0).span.clone(),
            ));
        } else {
            self.consume(); // consume ';'
        }

        let _ = self.scopes.define(Symbol {
            name: ident_name.clone(),
            type_: type_.clone().unwrap_or(TypeKind::Untyped),
            mut_: mutable,
            pub_: public,
        });

        // ASTStmt zurückgeben
        ASTStmt::varDec(identifier_token, public, mutable, type_, expr)
    }

    fn parse_type(&self) -> TypeKind {
        let mut ty = self.parse_type_atom();

        // Solange ein LAngle folgt, Generic-Argumente parsen
        if self.peek(0).kind == TokenKind::LAngle {
            self.check_whitespace(self.peek(-1).span.clone(), self.peek(0).span.clone());

            let name = match ty {
                TypeKind::Custom(n) => n,
                _ => {
                    self.diagnostics_bag.add(Diagnostic::new(
                        DiagnosticType::Error(DiagnosticKind::InvalidGenericBase),
                        self.peek(-1).span.clone(),
                    ));
                    "__ERROR<Generic>".into()
                }
            };

            let args = self.parse_generic_args();
            return TypeKind::Generic { base: name, args };
        }

        ty
    }

    fn parse_type_atom(&self) -> TypeKind {
        if self.peek(0).kind == TokenKind::And {
            self.consume(); // '&'

            if self.peek(0).kind == TokenKind::Keyword(Keyword::Mut) {
                self.consume();
                let inner = self.parse_type();
                return TypeKind::MutRef(Box::new(inner));
            } else {
                let inner = self.parse_type();
                return TypeKind::Ref(Box::new(inner));
            }
        }

        let token = self.consume();

        match token.kind {
            TokenKind::Type(tk) => tk,

            TokenKind::Identifier(name) => {
                // Primitive als Identifier erlaubt
                let prim = match name.as_str() {
                    "i8"   => Some(Primitive::I8),
                    "i16"  => Some(Primitive::I16),
                    "i32"  => Some(Primitive::I32),
                    "i64"  => Some(Primitive::I64),
                    "u8"   => Some(Primitive::U8),
                    "u16"  => Some(Primitive::U16),
                    "u32"  => Some(Primitive::U32),
                    "u64"  => Some(Primitive::U64),
                    "f32"  => Some(Primitive::F32),
                    "f64"  => Some(Primitive::F64),
                    "bool" => Some(Primitive::Bool),
                    "char" => Some(Primitive::Char),
                    "String"  => Some(Primitive::String),
                    _      => None,
                };

                prim.map(TypeKind::Primitive)
                    .unwrap_or(TypeKind::Custom(name))
            }

            TokenKind::LParen => {
                let mut elems = Vec::new();

                if self.peek(0).kind != TokenKind::RParen {
                    loop {
                        elems.push(self.parse_type());

                        if self.peek(0).kind == TokenKind::Comma {
                            self.consume();
                        } else {
                            break;
                        }
                    }
                }

                self.consume_check(TokenKind::RParen);
                TypeKind::Tuple(elems)
            }

            _ => {
                self.diagnostics_bag.add(Diagnostic::new(
                    DiagnosticType::Error(DiagnosticKind::ExpectedToken {
                        expected: vec!["Type".into()],
                    }),
                    token.span,
                ));
                TypeKind::Custom("__ERROR<Type>".into())
            }
        }
    }

    fn parse_generic_args(&self) -> Vec<TypeKind> {
        self.consume_check(TokenKind::LAngle); // consume '<'
        let mut args = Vec::new();

        loop {
            // Ein Type als Argument
            args.push(self.parse_type());

            match self.peek(0).kind {
                TokenKind::Comma => {
                    self.consume();
                }

                TokenKind::RAngle => {
                    self.consume(); // normales '>'
                    break;
                }

                TokenKind::DoubleRAngle => {
                    // '>>' aufteilen in zwei '>'
                    let tok = self.consume();

                    let second = Token {
                        kind: TokenKind::RAngle,
                        span: Span { start: tok.span.start + 1, end: tok.span.end },
                    };

                    // erstes > verwenden, zweites pushback
                    self.push_back(second);
                    break;
                }

                _ => {
                    self.diagnostics_bag.add(Diagnostic::new(
                        DiagnosticType::Error(DiagnosticKind::ExpectedToken {
                            expected: vec![">".into()],
                        }),
                        self.peek(0).span.clone(),
                    ));
                    break;
                }
            }
        }

        args
    }

    fn parse_expr_stmt(&self) -> ASTStmt {
        let expr = self.parse_expr();

        if self.peek(0).kind == TokenKind::Semicolon {
            self.consume();
            ASTStmt::expr(expr)
        } else {
            self.diagnostics_bag.add(Diagnostic::new(
                DiagnosticType::Error(DiagnosticKind::UnexpectedToken {
                    given: self.peek(0).kind,
                }),
                self.peek(0).span.clone(),
            ));
            ASTStmt::expr(expr)
        }
    }

    fn parse_expr(&self) -> ASTExpr {
        if let TokenKind::Identifier(ref name) = self.peek(0).kind {
            match self.peek(1).kind {
                TokenKind::Equals | TokenKind::PlusEquals | TokenKind::MinusEquals | TokenKind::AsteriskEquals | TokenKind::SlashEquals => {
                    return self.parse_assignment(name.clone());
                }
                _ => {}
            }
        }

        // Other --> Binary Expr (incl. '==', '!=', '<=', '>=')
        self.parse_binary_expr(0)
    }

    fn parse_assignment(&self, name: String) -> ASTExpr {
        self.consume(); // Identifier

        // Consume Operator Token
        let op = self.consume().kind;

        let rhs = self.parse_expr();

        ASTExpr::assignment(name, op, rhs)
    }

    fn parse_unary_expr(&self) -> ASTExpr {
        match self.peek(0).kind {
            TokenKind::Minus => {
                let tok = self.consume();
                let op = ASTUnaryOperator::new(ASTUnaryOperatorKind::Negate, tok);
                ASTExpr::unary(op, self.parse_unary_expr())
            }

            TokenKind::Exclamation => {
                let tok = self.consume();
                let op = ASTUnaryOperator::new(ASTUnaryOperatorKind::Not, tok);
                ASTExpr::unary(op, self.parse_unary_expr())
            }

            TokenKind::Tilde => {
                let tok = self.consume();
                let op = ASTUnaryOperator::new(ASTUnaryOperatorKind::BitNot, tok);
                ASTExpr::unary(op, self.parse_unary_expr())
            }

            TokenKind::And => {
                let amp = self.consume();

                let kind = if self.peek(0).kind == TokenKind::Keyword(Keyword::Mut) {
                    self.consume();
                    ASTUnaryOperatorKind::RefMut
                } else {
                    ASTUnaryOperatorKind::Ref
                };

                let op = ASTUnaryOperator::new(kind, amp);
                ASTExpr::unary(op, self.parse_primary_expr())
            }

            TokenKind::Asterisk => {
                let tok = self.consume();
                let op = ASTUnaryOperator::new(ASTUnaryOperatorKind::Deref, tok);
                ASTExpr::unary(op, self.parse_primary_expr())
            }

            _ => self.parse_primary_expr(),
        }
    }

    fn parse_binary_expr(&self, precedence: u8) -> ASTExpr {
        let mut left = self.parse_unary_expr();

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
                TokenKind::Caret => ASTBinaryOperatorKind::BitXor,

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
            self.consume();

            // right hand side Expression
            let right = self.parse_binary_expr(op_prec);
            left = ASTExpr::binary(left, right, op);
        }

        left
    }

    fn parse_primary_expr(&self) -> ASTExpr {
        let token = self.consume();

        return match token.kind {
            TokenKind::Integer(v) => ASTExpr::int(v),
            TokenKind::Float(v) => ASTExpr::float(v),
            TokenKind::String(v) => ASTExpr::string(v),
            TokenKind::Char(v) => ASTExpr::char(v),
            TokenKind::Keyword(kw) => match kw {
                Keyword::True => ASTExpr::bool(true),
                Keyword::False => ASTExpr::bool(false),
                _ => ASTExpr::error(),
            }
            TokenKind::Identifier(name) => {
                if self.scopes.lookup(&name).is_none() {
                    self.diagnostics_bag.add(Diagnostic::new(
                        DiagnosticType::Error(DiagnosticKind::UnknownIdentifier { identifier: name.clone() }),
                        token.span.clone(),
                    ));
                    return ASTExpr::error();
                }
                ASTExpr::variable(name)
            }
            TokenKind::LParen => {
                let expr = self.parse_expr();
                let rparen = self.consume();
                if rparen.kind != TokenKind::RParen {
                    self.diagnostics_bag.add(
                        Diagnostic::new(
                            DiagnosticType::Error(DiagnosticKind::UnexpectedToken {
                                given: token.kind.clone(),
                            }),
                            token.span.clone(),
                        )
                    );
                    self.diagnostics_bag.add(
                        Diagnostic::new(
                            DiagnosticType::Error(DiagnosticKind::ExpectedToken {
                                expected: vec!["Right/Closing Parenthesis ')'".to_string()],
                            }),
                            token.span.clone(),
                        )
                    );
                    return ASTExpr::error();
                }
                ASTExpr::parenthesized(expr)
            }
            TokenKind::LCurly => self.parse_block_expr(),
            _ => {
                dbg!(&token.kind);
                self.diagnostics_bag.add(
                    Diagnostic::new(
                        DiagnosticType::Error(DiagnosticKind::ExpectedExpression {
                            expected: vec![
                                "Expression".to_string(),
                            ],
                        }),
                        token.span.clone()
                    )
                );
                return ASTExpr::error();
            }
        }
    }
}

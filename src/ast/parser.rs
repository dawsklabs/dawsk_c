use super::token::{ Token, TokenKind, Keyword };
use super::{ ASTStmt, ASTExpr, ASTBinaryOperator, ASTBinaryOperatorKind, ASTUnaryOperator, ASTUnaryOperatorKind };
use super::types::{ TypeKind, Primitive };
use super::scope::{ScopeStack, Symbol};

use crate::{ Diagnostic, DiagnosticType, DiagnosticKind, DiagnosticBagCell };

use std::cell::Cell;

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
    scopes: ScopeStack, // neu
}

impl Parser {
    pub fn new(tokens: Vec<Token>, diagnostics_bag: DiagnosticBagCell) -> Self {
        Self { tokens, index: Counter::new(), diagnostics_bag, scopes: ScopeStack::new() }
    }

    pub fn peek(&self, offset: isize) -> Token {
        let idx = self.index.get() as isize + offset;
        let idx = idx.clamp(0, (self.tokens.len() - 1) as isize) as usize;

        self.tokens[idx].clone()
    }
    
    pub fn advance(&self, n: usize) {
        self.index.incr(n);
    }

    pub fn consume(&self) -> Token {
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
        token.clone()
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

    pub fn next_stmt(&self) -> Option<ASTStmt> {
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
        let mut visible = false;
        let mut mutable = false;

        self.consume(); // consume 'dec' token

        if self.peek(0).kind == TokenKind::Keyword(Keyword::Publy) {
            visible = true;
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
            _ => "<error>".to_string(),
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
                DiagnosticType::Error(DiagnosticKind::ExpectedToken {
                    expected: vec!["Equals '='".to_string()]
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
            vis: visible,
        });

        // ASTStmt zurückgeben
        ASTStmt::dec(identifier_token, visible, mutable, type_, expr)
    }

    fn parse_type(&self) -> TypeKind {
        let token = self.peek(0);

        match &token.kind {
            // Already parsed primitive or type tokens
            TokenKind::Type(tk) => {
                self.consume();
                return tk.clone();
            }

            // Custom type
            TokenKind::Identifier(name) => {
                let primitive = match name.as_str() {
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
                    "str"  => Some(Primitive::String),
                    _      => None,
                };

                if let Some(p) = primitive {
                    self.consume();
                    return TypeKind::Primitive(p);
                }

                match name.as_str() {
                    "Vec" | "Set" | "Map" => self.parse_generic_type(name.clone(), |parser| parser.parse_type()),
                    _ => {
                        self.consume();
                        TypeKind::Custom(name.clone())
                    }
                }
            }

            // Tuple (<>, <>, ...)
            TokenKind::LParen => {
                self.consume(); // '('

                let mut elems = Vec::new();

                // Empty tuple ()
                if self.peek(0).kind == TokenKind::RParen {
                    self.consume();
                    return TypeKind::Tuple(elems);
                }

                loop {
                    let ty = self.parse_type();
                    elems.push(ty);

                    if self.peek(0).kind == TokenKind::Comma {
                        self.consume();
                        continue;
                    }

                    break;
                }

                // Expect ')'
                let r = self.consume();
                if r.kind != TokenKind::RParen {
                    self.diagnostics_bag.add(Diagnostic::new(
                        DiagnosticType::Error(DiagnosticKind::ExpectedToken {
                            expected: vec!["Right/Closing Paren ')'".to_string()],
                        }),
                        r.span.clone(),
                    ));
                }

                return TypeKind::Tuple(elems);
            }

            _ => {
                // Unknown / invalid type
                self.consume();
                self.diagnostics_bag.add(Diagnostic::new(
                    DiagnosticType::Error(DiagnosticKind::ExpectedToken {
                        expected: vec![ "Type".into() ],
                    }),
                    token.span.clone(),
                ));
                return TypeKind::Custom("<error>".into());
            }
        }
    }

    fn parse_generic_type<F>(&self, name: String, param_parser: F) -> TypeKind
    where
        F: Fn(&Self) -> TypeKind,
    {
        self.consume(); // consume typename (z.B. Vec, Set, Map)

        // Prüfen auf '<'
        if !self.expect(|k| matches!(k, TokenKind::LAngle), "<".into(), 0) {
            return TypeKind::Custom("<error>".into());
        }
        self.consume(); // consume '<'

        // Typen parsen
        let mut generics = Vec::new();
        loop {
            let ty = param_parser(self);
            generics.push(ty);

            match self.peek(0).kind {
                TokenKind::Comma => {
                    self.consume();
                    continue;
                }
                TokenKind::RAngle => break,
                _ => {
                    self.diagnostics_bag.add(Diagnostic::new(
                        DiagnosticType::Error(DiagnosticKind::ExpectedToken {
                            expected: vec![",".into(), ">".into()],
                        }),
                        self.peek(0).span.clone(),
                    ));
                    break;
                }
            }
        }

        // '>' konsumieren
        if !self.expect(|k| matches!(k, TokenKind::RAngle), ">".into(), 0) {
            return TypeKind::Custom("<error>".into());
        }
        self.consume(); // '>'

        // Spezifischen Typ zurückgeben
        match name.as_str() {
            "Vec" if generics.len() == 1 => TypeKind::Vector(Box::new(generics.remove(0))),
            "Set" if generics.len() == 1 => TypeKind::Set(Box::new(generics.remove(0))),
            "Map" if generics.len() == 2 => TypeKind::Map(Box::new(generics.remove(0)), Box::new(generics.remove(0))),
            _ => TypeKind::Custom(name),
        }
    }

    fn parse_expr_stmt(&self) -> ASTStmt {
        let expr = self.parse_expr();

        if self.peek(0).kind == TokenKind::Semicolon {
            self.consume();
            ASTStmt::expr(expr)
        } else {
            self.diagnostics_bag.add(Diagnostic::new(
                DiagnosticType::Error(DiagnosticKind::ExpectedToken {
                    expected: vec![";".into()],
                }),
                self.peek(0).span.clone(),
            ));
            ASTStmt::expr(expr)
        }
    }

    fn parse_expr(&self) -> ASTExpr {
        if let TokenKind::Identifier(ref name) = self.peek(0).kind {
            // Nächstes Token prüfen: '=' oder Operator + '='
            let next = self.peek(1).kind.clone();

            match next {
                TokenKind::Equals => {
                    return self.parse_assignment(name.clone(), TokenKind::Equals);
                }
                TokenKind::Plus | TokenKind::Minus | TokenKind::Asterisk | TokenKind::Slash | TokenKind::Percent => {
                    // Prüfen, ob danach '=' kommt
                    if self.peek(2).kind == TokenKind::Equals {
                        let op_kind = next;
                        return self.parse_assignment(name.clone(), op_kind);
                    }
                }
                _ => {}
            }
        }

        self.parse_binary_expr(0)
    }

    fn parse_assignment(&self, name: String, op: TokenKind) -> ASTExpr {
        self.consume(); // Identifier

        // Wenn kombinierter Operator, zwei Tokens überspringen
        if op != TokenKind::Equals {
            self.consume(); // Operator (+, -, ...) 
            self.consume(); // '='
        } else {
            self.consume(); // '='
        }

        let rhs = self.parse_expr();

        ASTExpr::assignment(name, op, rhs)
    }

    fn parse_unary_expr(&self) -> ASTExpr {
        let token = self.peek(0);

        let op_kind = match token.kind {
            TokenKind::Minus => Some(ASTUnaryOperatorKind::Negate),
            TokenKind::Exclamation => Some(ASTUnaryOperatorKind::Not),
            _ => None,
        };

        if let Some(kind) = op_kind {
            let op_token = self.consume();
            let expr = self.parse_unary_expr(); // Rekursiv für mehrere unäre Operatoren
            let op = ASTUnaryOperator::new(kind, op_token);
            return ASTExpr::unary(op, expr);
        }

        self.parse_primary_expr()
    }

    fn parse_binary_expr(&self, precedence: u8) -> ASTExpr {
        let mut left = self.parse_unary_expr();

        loop {
            let op_token = self.peek(0);

            let op_kind = match op_token.kind {
                TokenKind::Plus => ASTBinaryOperatorKind::Add,
                TokenKind::Minus => ASTBinaryOperatorKind::Subtract,
                TokenKind::Asterisk => ASTBinaryOperatorKind::Multiply,
                TokenKind::Slash => ASTBinaryOperatorKind::Divide,
                TokenKind::Percent => ASTBinaryOperatorKind::Modulus,
                TokenKind::Equals => if self.peek(1).kind == TokenKind::Equals {
                    self.consume(); // '=='
                    ASTBinaryOperatorKind::Equal
                } else {
                    break;
                },
                TokenKind::Exclamation => if self.peek(1).kind == TokenKind::Equals {
                    self.consume(); // '!='
                    ASTBinaryOperatorKind::NotEqual
                } else {
                    break;
                },
                TokenKind::LAngle => if self.peek(1).kind == TokenKind::Equals {
                    self.consume(); // '<='
                    ASTBinaryOperatorKind::LessEqual
                } else {
                    ASTBinaryOperatorKind::Less
                },
                TokenKind::RAngle => if self.peek(1).kind == TokenKind::Equals {
                    self.consume(); // '>='
                    ASTBinaryOperatorKind::GreaterEqual
                } else {
                    ASTBinaryOperatorKind::Greater
                },
                _ => break,
            };

            let op = ASTBinaryOperator::new(op_kind, op_token.clone());
            let op_prec = op.prec();

            if op_prec <= precedence {
                break;
            }

            self.consume(); // consume operator
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
                                "Value".to_string(),
                                "Function Call".to_string(),
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

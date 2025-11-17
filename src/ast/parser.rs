use super::token::{ Token, TokenKind, Position };
use super::{ ASTStmt, ASTExpr, ASTBinaryOperator, ASTBinaryOperatorKind, Span };
use crate::{ Diagnostic, DiagnosticType, DiagnosticKind, DiagnosticBag };
use crate::file;

use std::{rc::Rc, cell::{RefCell, Cell}};

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
    diagnostics_bag: Rc<RefCell<DiagnosticBag>>,
}

impl Parser {
    pub fn new(diagnostics_bag: Rc<RefCell<DiagnosticBag>>) -> Self {
        Self { tokens: vec![], index: Counter::new(), diagnostics_bag }
    }

    pub fn from_tokens(tokens: Vec<Token>, diagnostics_bag: Rc<RefCell<DiagnosticBag>>) -> Self {
        Self { tokens, index: Counter::new(), diagnostics_bag }
    }

    pub fn peek(&self, offset: isize) -> &Token {
        let index = self.index.get() as isize + offset;
        if index < 0 || index >= self.tokens.len() as isize {
            dbg!(index, self.tokens.len() as isize);
            panic!("peeked out of bound!")
        }
        &self.tokens[index as usize]
    }

    pub fn advance(&self, n: usize) {
        self.index.incr(n);
    }

    pub fn consume(&self) -> &Token {
        self.index.incr(1);
        self.peek(-1)
    }

    pub fn consume_check(&mut self, expected: TokenKind) -> &Token {
        let token = self.consume();
        if token.kind != expected {
            self.diagnostics_bag.borrow_mut().add(Diagnostic::new(DiagnosticType::Error(DiagnosticKind::UnexpectedToken{ given: token.kind.clone(), expected: vec![expected] }), token.pos.clone()));
        }
        token
    }

    pub fn is_at_end(&self) -> bool {
        self.index.get() as usize >= self.tokens.len()
    }

    pub fn expect(&mut self, kind: TokenKind) -> bool {
        let token = self.peek(0);
        if token.kind != kind {
            self.diagnostics_bag.borrow_mut().add(
                Diagnostic::new(
                    DiagnosticType::Error(DiagnosticKind::UnexpectedToken {
                        given: token.kind.clone(),
                        expected: vec![kind]
                    }),
                    token.pos.clone()
                )
            );
        }
        true
    }

    pub fn next_stmt(&mut self) -> Option<ASTStmt> {
        if let tok = self.peek(0) {
            if tok.kind == TokenKind::EOF {
                return None; // Ende der Token-Liste erreicht
            }
        } else {
            return None; // Out-of-bounds
        }

        Some(self.parse_stmt())
    }


    fn parse_stmt(&mut self) -> ASTStmt {
        let expr = self.parse_expr(); // <-- keine Borrows hier

        let token = self.consume(); // <-- token ist jetzt komplett von RefCell unabhängig

        // nur hier kurz borrow_mut, sofort freigeben
        if let TokenKind::Unknown(c) = &token.kind {
            self.diagnostics_bag.borrow_mut().add(
                Diagnostic::new(
                    DiagnosticType::Error(
                        DiagnosticKind::UnknownIdentifier { identifier: c.to_string() }
                    ),
                    token.pos.clone()
                )
            );
        }

        // jetzt ist der Borrow vorbei, wir können erneut borrow_mut aufrufen
        if token.kind != TokenKind::Semicolon {
            self.diagnostics_bag.borrow_mut().add(
                Diagnostic::new(
                    DiagnosticType::Error(DiagnosticKind::UnexpectedToken {
                        given: token.kind.clone(),
                        expected: vec![TokenKind::Semicolon]
                    }),
                    token.pos.clone()
                )
            );
        }

        ASTStmt::expr(expr)
    }

    fn parse_expr(&mut self) -> ASTExpr {
        self.parse_binary_expr(0)
    }

    fn parse_binary_expr(&mut self, precedence: u8) -> ASTExpr {
        let mut left = self.parse_primary_expr();

        loop {
            let op_token = self.peek(0);

            match op_token.kind {
                TokenKind::Plus | TokenKind::Minus | TokenKind::Asterisk
                | TokenKind::Slash | TokenKind::Modulus => {}
                _ => {
                    dbg!(&op_token); // nur zur Info
                }
            }

            let op_kind = match op_token.kind {
                TokenKind::Plus => ASTBinaryOperatorKind::Add,
                TokenKind::Minus => ASTBinaryOperatorKind::Subtract,
                TokenKind::Asterisk => ASTBinaryOperatorKind::Multiply,
                TokenKind::Slash => ASTBinaryOperatorKind::Divide,
                TokenKind::Modulus => ASTBinaryOperatorKind::Modulus,
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

    fn parse_binary_operator(&mut self) -> ASTBinaryOperator {
        let token = self.consume();
        let kind = match token.kind {
            TokenKind::Plus => ASTBinaryOperatorKind::Add,
            TokenKind::Minus => ASTBinaryOperatorKind::Subtract,
            TokenKind::Asterisk => ASTBinaryOperatorKind::Multiply,
            TokenKind::Slash => ASTBinaryOperatorKind::Divide,
            TokenKind::Modulus => ASTBinaryOperatorKind::Modulus,
            _ => panic!("Unexpected token: {:?}", token.kind),
        };
        ASTBinaryOperator::new(kind, token.clone())
    }

    fn parse_primary_expr(&mut self) -> ASTExpr {
        let token = self.consume().clone(); // <-- hier clone statt Referenz

        match token.kind {
            TokenKind::Integer(v) => ASTExpr::int(v),
            TokenKind::Float(v) => ASTExpr::float(v),
            TokenKind::LParen => {
                let expr = self.parse_expr();
                let rparen = self.consume();
                if rparen.kind != TokenKind::RParen {
                    self.diagnostics_bag.borrow_mut().add(
                        Diagnostic::new(
                            DiagnosticType::Error(DiagnosticKind::UnexpectedToken {
                                given: token.kind.clone(),
                                expected: vec![TokenKind::RParen],
                            }),
                            token.pos.clone(),
                        )
                    );
                    return ASTExpr::error(token.pos.clone());
                }
                ASTExpr::parenthesized(expr)
            }
            _ => {
                dbg!(token.kind);
                // ASTExpr::error(token.pos.clone()) // <-- hier error statt panic
                panic!("Unexpected token in primary expression")
            }
        }
    }
}
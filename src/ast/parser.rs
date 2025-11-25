use super::token::{ Token, TokenKind, Keyword };
use super::{ ASTStmt, ASTStmtKind, ASTExpr, ASTExprKind, ASTBinaryOperator, ASTBinaryOperatorKind, ASTParenExpr };
use super::types::TypeKind;
use crate::{ Diagnostic, DiagnosticType, DiagnosticKind, DiagnosticBagCell };

use std::fmt::Display;
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
}

impl Parser {
    pub fn new(tokens: Vec<Token>, diagnostics_bag: DiagnosticBagCell) -> Self {
        Self { tokens, index: Counter::new(), diagnostics_bag }
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

    pub fn is_at_end(&self) -> bool {
        self.index.get() as usize >= self.tokens.len()
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

        if self.peek(0).kind == TokenKind::EOF {
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
        let identifier_token = match identifier_token.kind {
            TokenKind::Identifier(_) => identifier_token,
            _ => {
                // report diagnostic, but keep the token for span info
                self.diagnostics_bag.add(Diagnostic::new(
                    DiagnosticType::Error(DiagnosticKind::ExpectedToken {
                        expected: vec!["Identifier".to_string()]
                    }),
                    identifier_token.span.clone()
                ));
                identifier_token // keep it so AST has some span
            }
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
            // try to recover: if next token is '=', continue, otherwise advance to avoid infinite loop
            if self.peek(0).kind == TokenKind::Equals {
                // fine
            } else {
                self.consume();
            }
        } else {
            self.consume(); // consume '='
        }

        let expr = self.parse_expr();

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

        // ASTStmt::dec erwartet: identifier token, public, mutable, type, initializer
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
                // Special-case Vec<T>
                if name == "Vec" {
                    self.consume(); // Vec

                    if !self.expect(|k| matches!(k, TokenKind::LAngle), "<".into(), 0) {
                        return TypeKind::Custom("<error>".into());
                    }
                    self.consume(); // <

                    let inner = self.parse_type();

                    if !self.expect(|k| matches!(k, TokenKind::RAngle), ">".into(), 0) {
                        return TypeKind::Vector(Box::new(inner));
                    }
                    self.consume(); // >

                    return TypeKind::Vector(Box::new(inner));
                }

                self.consume();
                return TypeKind::Custom(name.clone());
            }

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
                            expected: vec![")".to_string()],
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

    fn parse_expr_stmt(&self) -> ASTStmt {
        let expr = self.parse_expr();
        return ASTStmt::expr(expr)
    }

    fn parse_expr(&self) -> ASTExpr {
        self.parse_binary_expr(0)
    }

    fn parse_binary_expr(&self, precedence: u8) -> ASTExpr {
        let mut left = self.parse_primary_expr();

        loop {
            let op_token = self.peek(0);

            let op_kind = match op_token.kind {
                TokenKind::Plus => ASTBinaryOperatorKind::Add,
                TokenKind::Minus => ASTBinaryOperatorKind::Subtract,
                TokenKind::Asterisk => ASTBinaryOperatorKind::Multiply,
                TokenKind::Slash => ASTBinaryOperatorKind::Divide,
                TokenKind::Percent => ASTBinaryOperatorKind::Modulus,
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

    fn parse_binary_operator(&self) -> ASTBinaryOperator {
        let token = self.consume();
        let kind = match token.kind {
            TokenKind::Plus => ASTBinaryOperatorKind::Add,
            TokenKind::Minus => ASTBinaryOperatorKind::Subtract,
            TokenKind::Asterisk => ASTBinaryOperatorKind::Multiply,
            TokenKind::Slash => ASTBinaryOperatorKind::Divide,
            TokenKind::Percent => ASTBinaryOperatorKind::Modulus,
            _ => {
                self.diagnostics_bag.add(
                    Diagnostic::new(
                        DiagnosticType::Error(
                            DiagnosticKind::UnexpectedToken {
                                given: token.kind.clone(),
                            }
                        ),
                        token.span.clone(),
                    )
                );
                panic!()
            }
        };
        ASTBinaryOperator::new(kind, token.clone())
    }

    fn parse_primary_expr(&self) -> ASTExpr {
        let token = self.consume();

        return match token.kind {
            TokenKind::Integer(v) => ASTExpr::int(v),
            TokenKind::Float(v) => ASTExpr::float(v),
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
                                expected: vec!["Right/Opening Parenthesis".to_string()],
                            }),
                            token.span.clone(),
                        )
                    );
                    return ASTExpr::error();
                }
                ASTExpr::parenthesized(expr)
            }
            _ => {
                dbg!(&token.kind);
                self.diagnostics_bag.add(
                    Diagnostic::new(
                        DiagnosticType::Error(DiagnosticKind::ExpectedExpression {
                            expected: vec![
                                "Left/Opening Parenthesis".to_string(),
                                "Integer Literal".to_string(),
                                "Float Literal".to_string()
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

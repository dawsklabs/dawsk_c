pub mod eval;
pub mod lexer;
pub mod parser;
pub mod token;
pub mod types;

use std::fmt::{Display, Formatter, Result};

use token::{ Token, TokenKind, Span };
use types::TypeKind;
use crate::text::Source;

pub struct AST {
    pub stmts: Vec<ASTStmt>
}

impl AST {
    pub fn new() -> Self {
        Self { stmts: Vec::new() }
    }

    pub fn add_stmt(&mut self, stmt: ASTStmt) {
        self.stmts.push(stmt);
    }

    pub fn visit(&self, visitor: &mut dyn ASTVisitor) {
        for stmt in &self.stmts {
            visitor.visit_stmt(stmt);
        }
    }

    pub fn visualize(&self) {
        let mut printer = ASTPrinter { indent: 0 };
        self.visit(&mut printer);
    }
}

pub trait ASTVisitor {
    fn do_visit_stmt(&mut self, stmt: &ASTStmt) {
        match &stmt.kind {
            ASTStmtKind::Expr(expr) => self.visit_expr(expr),
            ASTStmtKind::Dec(dec) => self.visit_dec(dec),
            _ => {},
        }
    }

    fn visit_stmt(&mut self, stmt: &ASTStmt) {
        self.do_visit_stmt(stmt);
    }

    fn do_visit_expr(&mut self, expr: &ASTExpr) {
        match &expr.kind {
            ASTExprKind::Integer(_) | ASTExprKind::Float(_) | ASTExprKind::String(_) | ASTExprKind::Char(_) | ASTExprKind::Bool(_) => {
                self.visit_valued(&expr.kind);
            }
            ASTExprKind::Binary(bin) => self.visit_binary_expr(&bin),
            ASTExprKind::Parenthesized(expr) => self.visit_paren_expr(&expr),
            ASTExprKind::Error => self.visit_error(),
        }
    }

    fn visit_expr(&mut self, expr: &ASTExpr) {
        self.do_visit_expr(expr);
    }

    fn visit_dec(&mut self, expr: &ASTDecExpr) {
        self.visit_dec(expr);
    }

    fn visit_binary_expr(&mut self, expr: &ASTBinaryExpr) {
        self.visit_expr(&expr.left);
        self.visit_expr(&expr.right);
    }

    fn visit_paren_expr(&mut self, expr: &ASTParenExpr) {
        self.visit_expr(&expr.expr);
    }

    fn visit_error(&self);

    fn visit_valued(&self, kind: &ASTExprKind);
}

pub struct ASTPrinter {
    indent: usize,
}

pub const INDENT_SIZE: usize = 1;

impl ASTVisitor for ASTPrinter {
    fn visit_stmt(&mut self, stmt: &ASTStmt) {
        self.print_indent("Stmt:");
        self.indent += INDENT_SIZE;
        ASTVisitor::do_visit_stmt(self, stmt);
        self.indent -= INDENT_SIZE;
    }

    fn visit_expr(&mut self, expr: &ASTExpr) {
        self.print_indent("Expr:");
        self.indent += INDENT_SIZE;
        ASTVisitor::do_visit_expr(self, expr);
        self.indent -= INDENT_SIZE;
    }

    fn visit_binary_expr(&mut self, expr: &ASTBinaryExpr) {
        self.print_indent("Bin:");
        self.indent += INDENT_SIZE;
        self.print_indent(&format!("Operator: {:?}", expr.operator.kind));
        self.visit_expr(&expr.left);
        self.visit_expr(&expr.right);
        self.indent -= INDENT_SIZE;
    }

    fn visit_paren_expr(&mut self, expr: &ASTParenExpr) {
        self.print_indent("Paren:");
        self.indent += INDENT_SIZE;
        self.visit_expr(&expr.expr);
        self.indent -= INDENT_SIZE;
    }

    fn visit_error(&self) {
        self.print_indent("Error");
    }

    fn visit_dec(&mut self, dec: &ASTDecExpr) {
        if let TokenKind::Identifier(name) = &dec.identifier.kind {
            self.print_indent("Dec:");
            self.indent += INDENT_SIZE;
            self.print_indent(&format!("Var({})", name));
            self.print_indent(&format!("Vis={}", if dec.vis { "1" } else { "0" }));
            self.print_indent(&format!("Mut={}", if dec.mut_ { "1" }  else { "0" }));
            self.print_indent(&format!("Type: {:?}", dec.type_));
            self.visit_expr(&dec.initializer);
            self.indent -= INDENT_SIZE;
        }
    }

    // fn visit_cast_expr(&mut self, expr: &ASTCastExpr) {
    //     self.print_indent("Cast:");
    //     self.indent += INDENT_SIZE;
    //     self.print_indent(&format!("To: {:?}", expr.target.kind));
    //     self.visit_expr(&expr.expr);
    //     self.indent -= INDENT_SIZE;
    // }

    fn visit_valued(&self, kind: &ASTExprKind) {
        match kind {
            ASTExprKind::Integer(v) => self.print_indent(&format!("Int({})", v)),
            ASTExprKind::Float(v) => self.print_indent(&format!("Float({})", v)),
            ASTExprKind::Char(v) => self.print_indent(&format!("Char({})", v)),
            ASTExprKind::String(v) => self.print_indent(&format!("Str({})", v)),
            ASTExprKind::Bool(v) => self.print_indent(&format!("Bool({})", v)),
            ASTExprKind::Error => self.print_indent("Error"),
            _ => {}
        }
    }
}

impl ASTPrinter {
    fn print_indent(&self, text: &str) {
        let prefix = if self.indent == 0 {
            String::new()
        } else {
            let s = " |> ";
            " ".repeat((self.indent - 1) * s.len()) + s
        };

        println!("{}{}", prefix, text);
    }
}

pub enum ASTStmtKind {
    Expr(ASTExpr),
    Dec(ASTDecExpr),
}

pub struct ASTStmt {
    pub kind: ASTStmtKind,
}

impl ASTStmt {
    pub fn new(kind: ASTStmtKind) -> Self {
        Self { kind }
    }

    pub fn expr(expr: ASTExpr) -> Self {
        Self::new(ASTStmtKind::Expr(expr))
    }

    pub fn dec(identifier: Token, public: bool, mutable: bool, type_: Option<TypeKind>, initializer: ASTExpr) -> Self {
        Self::new(ASTStmtKind::Dec(ASTDecExpr::new(identifier, public, mutable, type_, initializer)))
    }
}

#[derive(Debug)]
pub enum ASTExprKind {
    Integer(i64),
    Float(f64),
    Char(char),
    String(String),
    Bool(bool),
    Binary(ASTBinaryExpr),
    Parenthesized(ASTParenExpr),
    Error,
    // Cast(ASTCastExpr),
}

impl Display for ASTExprKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            ASTExprKind::Integer(_) => write!(f, "Integer"),
            ASTExprKind::Float(_) => write!(f, "Float"),
            ASTExprKind::Char(_) => write!(f, "Char"),
            ASTExprKind::String(_) => write!(f, "String"),
            ASTExprKind::Bool(_) => write!(f, "Bool"),
            ASTExprKind::Parenthesized(_) => write!(f, "Parenthesized Expression"),
            ASTExprKind::Binary(_) => write!(f, "Binary Expression"),
            ASTExprKind::Error => write!(f, "ExprError"),
        }
    }
}

#[derive(Debug)]
pub struct ASTExpr {
    pub kind: ASTExprKind,
}

impl ASTExpr {
    pub fn new(kind: ASTExprKind) -> Self {
        Self { kind }
    }

    pub fn int(value: i64) -> Self {
        Self::new(ASTExprKind::Integer(value))
    }

    pub fn float(value: f64) -> Self {
        Self::new(ASTExprKind::Float(value))
    }

    pub fn char(value: char) -> Self {
        Self::new(ASTExprKind::Char(value))
    }

    pub fn string(value: String) -> Self {
        Self::new(ASTExprKind::String(value))
    }

    pub fn bool(value: bool) -> Self {
        Self::new(ASTExprKind::Bool(value))
    }

    pub fn binary(left: ASTExpr, right: ASTExpr, operator: ASTBinaryOperator) -> Self {
        Self::new(ASTExprKind::Binary(ASTBinaryExpr::new(left, right, operator)))
    }

    pub fn parenthesized(expr: ASTExpr) -> Self {
        Self::new(ASTExprKind::Parenthesized(ASTParenExpr::new(expr)))
    }

    pub fn error() -> Self {
        Self::new(ASTExprKind::Error)
    }

    // pub fn cast(expr: ASTExpr, target: TokenKind) -> Self {
    //     Self::new(ASTExprKind::As(ASTCastExpr { expr: Box::new(expr), target }))
    // }
}

#[derive(Debug)]
pub struct ASTBinaryExpr {
    pub left: Box<ASTExpr>,
    pub right: Box<ASTExpr>,
    pub operator: ASTBinaryOperator,
}

impl ASTBinaryExpr {
    pub fn new(left: ASTExpr, right: ASTExpr, operator: ASTBinaryOperator) -> Self {
        Self { left: Box::new(left), right: Box::new(right), operator }
    }
}

#[derive(Debug)]
pub struct ASTBinaryOperator {
    pub kind: ASTBinaryOperatorKind,
    pub token: Token,
}

impl ASTBinaryOperator {
    pub fn new(kind: ASTBinaryOperatorKind, token: Token) -> Self {
        Self { kind, token }
    }

    pub fn prec(&self) -> u8 {
        match self.kind {
            ASTBinaryOperatorKind::Add => 1,
            ASTBinaryOperatorKind::Subtract => 1,
            ASTBinaryOperatorKind::Multiply => 2,
            ASTBinaryOperatorKind::Divide => 2,
            ASTBinaryOperatorKind::Modulus => 2,
        }
    }
}

#[derive(Debug)]
pub enum ASTBinaryOperatorKind {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulus,
}

#[derive(Debug)]
pub struct ASTParenExpr {
    pub expr: Box<ASTExpr>,
}

impl ASTParenExpr {
    pub fn new(expr: ASTExpr) -> Self {
        Self { expr: Box::new(expr) }
    }
}

pub struct ASTDecExpr {
    identifier: Token,
    vis: bool,
    mut_: bool,
    type_: Option<TypeKind>,
    initializer: ASTExpr,
}

impl ASTDecExpr {
    pub fn new(identifier: Token, vis: bool, mut_: bool, type_: Option<TypeKind>, initializer: ASTExpr) -> Self {
        Self { identifier, vis, mut_, type_, initializer }
    }
}

// pub struct ASTCastExpr {
//     pub expr: Box<ASTExpr>,
//     pub target: TokenKind, // hier speichern wir den Zieltyp
// }

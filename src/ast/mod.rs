pub mod eval;
pub mod lexer;
pub mod parser;
pub mod token;
pub mod types;
pub mod scope;

use std::fmt::{Display, Formatter, Result};

use token::{ Token, TokenKind };
use types::TypeKind;
use crate::color::Color;

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
        #[warn(unreachable_patterns)]
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
            ASTExprKind::Integer(_) | ASTExprKind::Float(_) | ASTExprKind::String(_) | ASTExprKind::Char(_) | ASTExprKind::Bool(_) | ASTExprKind::Variable(_) => {
                self.visit_valued(&expr.kind);
            }
            ASTExprKind::Binary(bin) => self.visit_binary_expr(&bin),
            ASTExprKind::Parenthesized(expr) => self.visit_paren_expr(&expr),
            ASTExprKind::Assignment(expr) => self.visit_assignment_expr(expr),
            ASTExprKind::Unary(unary) => self.visit_unary_expr(&unary),
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

    fn visit_assignment_expr(&mut self, expr: &ASTAssignmentExpr) {
        self.visit_assignment_expr(&expr);
    }

    fn visit_unary_expr(&mut self, expr: &ASTUnaryExpr) {
        self.visit_unary_expr(&expr);
    }

    fn visit_error(&self);

    fn visit_valued(&self, kind: &ASTExprKind);
}

pub struct ASTPrinter {
    indent: usize,
}

pub const INDENT_SIZE: usize = 1;
pub const MOUVE_COLOR: Color = Color::FgHex("#cba6f7");
pub const LAVENDAR_COLOR: Color = Color::FgHex("#b4befe");
pub const BLUE_COLOR: Color = Color::FgHex("#89b4fa");
pub const GREEN_COLOR: Color = Color::FgHex("#a6e3a1");
pub const FLAMINGO_COLOR: Color = Color::FgHex("#f2cdcd");
pub const PEACH_COLOR: Color = Color::FgHex("#fab387");
pub const MAROON_COLOR: Color = Color::FgHex("#eba0ac");
pub const RED_COLOR: Color = Color::FgHex("#f38ba8");
pub const SUBTEXT_COLOR: Color = Color::FgHex("#a6adc8");
pub const RESET_COLOR: Color = Color::Reset;

impl ASTVisitor for ASTPrinter {
    fn visit_stmt(&mut self, stmt: &ASTStmt) {
        self.print_indent(&format!("{}Stmt{}:", SUBTEXT_COLOR, RESET_COLOR));
        self.indent += INDENT_SIZE;
        ASTVisitor::do_visit_stmt(self, stmt);
        self.indent -= INDENT_SIZE;
    }

    fn visit_expr(&mut self, expr: &ASTExpr) {
        self.print_indent(&format!("{}Expr{}:", SUBTEXT_COLOR, RESET_COLOR));
        self.indent += INDENT_SIZE;
        ASTVisitor::do_visit_expr(self, expr);
        self.indent -= INDENT_SIZE;
    }

    fn visit_binary_expr(&mut self, expr: &ASTBinaryExpr) {
        self.print_indent(&format!("{}Binary{}:", SUBTEXT_COLOR, RESET_COLOR));
        self.indent += INDENT_SIZE;
        self.print_indent(&format!("{}Operator{}: {}{}{}", SUBTEXT_COLOR, RESET_COLOR, BLUE_COLOR, expr.operator.kind, RESET_COLOR));
        self.visit_expr(&expr.left);
        self.visit_expr(&expr.right);
        self.indent -= INDENT_SIZE;
    }

    fn visit_paren_expr(&mut self, expr: &ASTParenExpr) {
        self.print_indent(&format!("{}Parenthesized{}:", SUBTEXT_COLOR, RESET_COLOR));
        self.indent += INDENT_SIZE;
        self.visit_expr(&expr.expr);
        self.indent -= INDENT_SIZE;
    }

    fn visit_assignment_expr(&mut self, expr: &ASTAssignmentExpr) {
        self.print_indent(&format!("{}Assignment{}:", SUBTEXT_COLOR, RESET_COLOR));
        self.indent += INDENT_SIZE;
        self.print_indent(&format!("{}Name{}: {}{}{}", SUBTEXT_COLOR, RESET_COLOR, LAVENDAR_COLOR, expr.name, RESET_COLOR));
        self.print_indent(&format!("{}Operator{}: {}{}{}", SUBTEXT_COLOR, RESET_COLOR, BLUE_COLOR, expr.op, RESET_COLOR));
        self.visit_expr(&expr.value);
        self.indent -= INDENT_SIZE;
    }

    fn visit_error(&self) {
        self.print_indent(&format!("{}Error{}", RED_COLOR, RESET_COLOR));
    }

    fn visit_dec(&mut self, dec: &ASTDecExpr) {
        if let TokenKind::Identifier(name) = &dec.identifier.kind {
            self.print_indent(&format!("{}Dec{}:", SUBTEXT_COLOR, RESET_COLOR));
            self.indent += INDENT_SIZE;
            self.print_indent(&format!("{}Name{}: {}{}{}", SUBTEXT_COLOR, RESET_COLOR, LAVENDAR_COLOR, name, RESET_COLOR));
            self.print_indent(&format!("{}Vis{}: {}{}{}", SUBTEXT_COLOR, RESET_COLOR, RED_COLOR, if dec.vis { "0x1" } else { "0x0" }, RESET_COLOR));
            self.print_indent(&format!("{}Mut{}: {}{}{}", SUBTEXT_COLOR, RESET_COLOR, RED_COLOR, if dec.mut_ { "0x1" } else { "0x0" }, RESET_COLOR));
            self.print_indent(&format!("{}Type{}: {}", SUBTEXT_COLOR, RESET_COLOR, dec.type_.clone().unwrap()));
            self.visit_expr(&dec.initializer);
            self.indent -= INDENT_SIZE;
        }
    }

    fn visit_unary_expr(&mut self, expr: &ASTUnaryExpr) {
        self.print_indent(&format!("{}Unary{}:", SUBTEXT_COLOR, RESET_COLOR));
        self.indent += INDENT_SIZE;
        self.print_indent(&format!("{}Operator{}: {}{}{}", SUBTEXT_COLOR, RESET_COLOR, BLUE_COLOR, expr.op.kind, RESET_COLOR));
        self.visit_expr(&expr.expr);
        self.indent -= INDENT_SIZE;
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
            ASTExprKind::Integer(v) => self.print_indent(&format!("{}Int{}({}{}{})", BLUE_COLOR, RESET_COLOR, PEACH_COLOR, v, RESET_COLOR)),
            ASTExprKind::Float(v) => self.print_indent(&format!("{}Float{}({}{}{})", BLUE_COLOR, RESET_COLOR, PEACH_COLOR, v, RESET_COLOR)),
            ASTExprKind::Char(v) => self.print_indent(&format!("{}Char{}({}'{}'{})", BLUE_COLOR, RESET_COLOR, GREEN_COLOR, v, RESET_COLOR)),
            ASTExprKind::String(v) => self.print_indent(&format!("{}Str{}({}\"{}\"{})", BLUE_COLOR, RESET_COLOR, GREEN_COLOR, v, RESET_COLOR)),
            ASTExprKind::Bool(v) => self.print_indent(&format!("{}Bool{}({}{}{})", BLUE_COLOR, RESET_COLOR, RED_COLOR, v, RESET_COLOR)),
            ASTExprKind::Variable(v) => self.print_indent(&format!("{}Var{}({}{}{})", BLUE_COLOR, RESET_COLOR, LAVENDAR_COLOR, v, RESET_COLOR)),
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

#[derive(Debug, Clone)]
pub enum ASTExprKind {
    Integer(i64),
    Float(f64),
    Char(char),
    String(String),
    Bool(bool),
    Binary(ASTBinaryExpr),
    Parenthesized(ASTParenExpr),
    Assignment(ASTAssignmentExpr),
    Variable(String),
    Unary(ASTUnaryExpr),
    Error,
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
            ASTExprKind::Assignment(_) => write!(f, "Assignment Expression"),
            ASTExprKind::Variable(_) => write!(f, "Variable"),
            ASTExprKind::Unary(_) => write!(f, "Unary Expression"),
            ASTExprKind::Error => write!(f, "ExprError"),
        }
    }
}

#[derive(Debug, Clone)]
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

    pub fn assignment(name: String, op: TokenKind, value: ASTExpr) -> Self {
        let op_kind = match op {
            TokenKind::Equals       => ASTBinaryOperatorKind::Assign,
            TokenKind::Plus         => ASTBinaryOperatorKind::AddAssign,
            TokenKind::Minus        => ASTBinaryOperatorKind::SubtractAssign,
            TokenKind::Asterisk     => ASTBinaryOperatorKind::MultiplyAssign,
            TokenKind::Slash        => ASTBinaryOperatorKind::DivideAssign,
            _ => panic!("unsupported assignment operator"),
        };

        Self::new(ASTExprKind::Assignment(ASTAssignmentExpr::new(name, op_kind, value)))
    }

    pub fn variable(name: String) -> Self {
        Self::new(ASTExprKind::Variable(name))
    }

    pub fn unary(op: ASTUnaryOperator, expr: ASTExpr) -> Self {
        Self::new(ASTExprKind::Unary(ASTUnaryExpr::new(op, expr)))
    }

    pub fn error() -> Self {
        Self::new(ASTExprKind::Error)
    }
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
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
            _ => 0,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ASTBinaryOperatorKind {
    Assign,
    Add,
    AddAssign,
    Subtract,
    SubtractAssign,
    Multiply,
    MultiplyAssign,
    Divide,
    DivideAssign,
    Modulus,
}

impl Display for ASTBinaryOperatorKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            ASTBinaryOperatorKind::Assign => write!(f, "Assign (=)"),
            ASTBinaryOperatorKind::Add => write!(f, "Add (+)"),
            ASTBinaryOperatorKind::AddAssign => write!(f, "Add (+) & Assign (=)"),
            ASTBinaryOperatorKind::Subtract => write!(f, "Subtract (-)"),
            ASTBinaryOperatorKind::SubtractAssign => write!(f, "Subtract (-) & Assign (=)"),
            ASTBinaryOperatorKind::Multiply => write!(f, "Multiply (*)"),
            ASTBinaryOperatorKind::MultiplyAssign => write!(f, "Multiply (*) & Assign (=)"),
            ASTBinaryOperatorKind::Divide => write!(f, "Divide (/)"),
            ASTBinaryOperatorKind::DivideAssign => write!(f, "Divide (/) & Assign (=)"),
            ASTBinaryOperatorKind::Modulus => write!(f, "Modulus (%)"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ASTParenExpr {
    pub expr: Box<ASTExpr>,
}

impl ASTParenExpr {
    pub fn new(expr: ASTExpr) -> Self {
        Self { expr: Box::new(expr) }
    }
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub struct ASTAssignmentExpr {
    name: String,
    op: ASTBinaryOperatorKind,
    value: Box<ASTExpr>,
}

impl ASTAssignmentExpr {
    pub fn new(name: String, op: ASTBinaryOperatorKind, value: ASTExpr) -> Self {
        Self { name, op, value: Box::new(value) }
    }
}

#[derive(Debug, Clone)]
pub struct ASTUnaryExpr {
    op: ASTUnaryOperator,
    expr: Box<ASTExpr>,
}

impl ASTUnaryExpr {
    pub fn new(op: ASTUnaryOperator, expr: ASTExpr) -> Self {
        Self { op, expr: Box::new(expr) }
    }
}

#[derive(Debug, Clone)]
pub struct ASTUnaryOperator {
    pub kind: ASTUnaryOperatorKind,
    pub token: Token,
}

#[derive(Debug, Clone)]
pub enum ASTUnaryOperatorKind {
    Negate,   // -
    Not,      // !
}

impl Display for ASTUnaryOperatorKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            ASTUnaryOperatorKind::Negate => write!(f, "Negate (-)"),
            ASTUnaryOperatorKind::Not => write!(f, "Logical Not (!)"),
            _ => unreachable!(), // should never happen
        }
    }
}

impl ASTUnaryOperator {
    pub fn new(kind: ASTUnaryOperatorKind, token: Token) -> Self {
        Self { kind, token }
    }
}

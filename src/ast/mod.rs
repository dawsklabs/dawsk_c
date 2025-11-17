pub mod eval;
pub mod lexer;
pub mod parser;
pub mod token;

use token::{ Token, Position, Span };

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
            ASTExprKind::Binary(bin) => {
                self.visit_binary_expr(&bin);
            }
            ASTExprKind::Parenthesized(expr) => {
                self.visit_paren_expr(&expr);
            }
            ASTExprKind::Error(pos) => {
                self.visit_error(&pos);
            }
        }
    }

    fn visit_expr(&mut self, expr: &ASTExpr) {
        self.do_visit_expr(expr);
    }

    fn visit_binary_expr(&mut self, expr: &ASTBinaryExpr) {
        self.visit_expr(&expr.left);
        self.visit_expr(&expr.right);
    }

    fn visit_paren_expr(&mut self, expr: &ASTParenExpr) {
        self.visit_expr(&expr.expr);
    }

    fn visit_error(&mut self, kind: &Position);

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

    fn visit_error(&mut self, pos: &Position) {
        self.print_indent("Error:");
        self.indent += INDENT_SIZE;
        self.print_indent(&format!("{}:{}-{}", pos.file, pos.span.start, pos.span.end));
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
            ASTExprKind::Integer(v) => self.print_indent(&format!("Int({})", v)),
            ASTExprKind::Float(v) => self.print_indent(&format!("Float({})", v)),
            ASTExprKind::Char(v) => self.print_indent(&format!("Char({})", v)),
            ASTExprKind::String(v) => self.print_indent(&format!("Str({})", v)),
            ASTExprKind::Bool(v) => self.print_indent(&format!("Bool({})", v)),
            ASTExprKind::Error(p) => self.print_indent(&format!("Error on {}-{}", p.span.start, p.span.end)),
            _ => {}
        }
    }
}

impl ASTPrinter {
    fn print_indent(&self, text: &str) {
        let prefix = if self.indent == 0 {
            String::new()
        } else {
            let s = "'- ";
            " ".repeat((self.indent - 1) * s.len()) + s
        };

        println!("{}{}", prefix, text);
    }
}

pub enum ASTStmtKind {
    Expr(ASTExpr),
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
}

pub enum ASTExprKind {
    Integer(i64),
    Float(f64),
    Char(char),
    String(String),
    Bool(bool),
    Binary(ASTBinaryExpr),
    Parenthesized(ASTParenExpr),
    Error(Position),
    // Cast(ASTCastExpr),
}

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
        Self::new(ASTExprKind::Binary(ASTBinaryExpr { left: Box::new(left), right: Box::new(right), operator }))
    }

    pub fn parenthesized(expr: ASTExpr) -> Self {
        Self::new(ASTExprKind::Parenthesized(ASTParenExpr { expr: Box::new(expr) }))
    }

    pub fn error(pos: Position) -> Self {
        Self::new(ASTExprKind::Error(pos))
    }

    // pub fn cast(expr: ASTExpr, target: TokenKind) -> Self {
    //     Self::new(ASTExprKind::As(ASTCastExpr { expr: Box::new(expr), target }))
    // }
}

pub struct ASTBinaryExpr {
    pub left: Box<ASTExpr>,
    pub right: Box<ASTExpr>,
    pub operator: ASTBinaryOperator,
}

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

pub struct ASTParenExpr {
    pub expr: Box<ASTExpr>,
}

// pub struct ASTCastExpr {
//     pub expr: Box<ASTExpr>,
//     pub target: TokenKind, // hier speichern wir den Zieltyp
// }
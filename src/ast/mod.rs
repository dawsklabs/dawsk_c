pub mod lexer;
pub mod parser;
pub mod scope;
pub mod token;
pub mod traits;
pub mod typechecker;
pub mod types;

use std::fmt::{Display, Formatter, Result};

use crate::color::{
    Color, BLUE_COLOR, GREEN_COLOR, LAVENDAR_COLOR, PEACH_COLOR, RED_COLOR, SUBTEXT_COLOR,
};
use token::{Span, Token, TokenKind};
use types::TypeId;

pub struct AST {
    pub stmts: Vec<ASTStmt>,
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
            ASTStmtKind::Return(_ret) => {}
            ASTStmtKind::VarDec(dec) => self.visit_var_dec(dec),
            ASTStmtKind::StructDec(dec) => self.visit_struct_dec(dec),
            ASTStmtKind::TupleStructDec(dec) => self.visit_tuple_struct_dec(dec),
        }
    }

    fn visit_stmt(&mut self, stmt: &ASTStmt) {
        self.do_visit_stmt(stmt);
    }

    fn do_visit_expr(&mut self, expr: &ASTExpr) {
        match &expr.kind {
            ASTExprKind::Integer(_)
            | ASTExprKind::Float(_)
            | ASTExprKind::Byte(_)
            | ASTExprKind::Char(_)
            | ASTExprKind::String(_)
            | ASTExprKind::ByteString(_)
            | ASTExprKind::Bool(_)
            | ASTExprKind::Variable(_) => {
                self.visit_valued(&expr.kind);
            }
            ASTExprKind::Unary(unary) => self.visit_unary_expr(&unary),
            ASTExprKind::Binary(bin) => self.visit_binary_expr(&bin),
            ASTExprKind::Parenthesized(expr) => self.visit_paren_expr(&expr),
            ASTExprKind::Assignment(expr) => self.visit_assignment_expr(expr),
            ASTExprKind::Cast(expr) => self.visit_cast_expr(expr),
            ASTExprKind::Block(block) => self.visit_block_expr(block),
            ASTExprKind::Error => self.visit_error(),
        }
    }

    fn visit_expr(&mut self, expr: &ASTExpr) {
        self.do_visit_expr(expr);
    }

    fn visit_var_dec(&mut self, expr: &ASTVarDecExpr);

    fn visit_struct_dec(&mut self, expr: &ASTStructDecExpr);

    fn visit_tuple_struct_dec(&mut self, expr: &ASTTupleStructDecExpr);

    fn visit_binary_expr(&mut self, expr: &ASTBinaryExpr) {
        self.visit_expr(&expr.left);
        self.visit_expr(&expr.right);
    }

    fn visit_paren_expr(&mut self, expr: &ASTParenExpr);

    fn visit_assignment_expr(&mut self, expr: &ASTAssignmentExpr);

    fn visit_unary_expr(&mut self, expr: &ASTUnaryExpr);

    fn visit_cast_expr(&mut self, expr: &ASTCastExpr);

    fn visit_block_expr(&mut self, expr: &ASTBlockExpr);

    fn visit_error(&self);

    fn visit_valued(&self, kind: &ASTExprKind);
}

pub struct ASTPrinter {
    indent: usize,
}

const INDENT_SIZE: usize = 1;

impl ASTVisitor for ASTPrinter {
    fn visit_stmt(&mut self, stmt: &ASTStmt) {
        self.print_indent(&format!("{}Statement{}:", SUBTEXT_COLOR, Color::ResetAll));
        self.indent += INDENT_SIZE;
        ASTVisitor::do_visit_stmt(self, stmt);
        self.indent -= INDENT_SIZE;
        println!();
    }

    fn visit_expr(&mut self, expr: &ASTExpr) {
        self.print_indent(&format!("{}Expression{}:", SUBTEXT_COLOR, Color::ResetAll));
        self.indent += INDENT_SIZE;
        ASTVisitor::do_visit_expr(self, expr);
        self.indent -= INDENT_SIZE;
    }

    fn visit_binary_expr(&mut self, expr: &ASTBinaryExpr) {
        self.print_indent(&format!("{}Binary{}:", SUBTEXT_COLOR, Color::ResetAll));
        self.indent += INDENT_SIZE;
        self.print_indent(&format!(
            "{}Operator{}: {}{}{}",
            SUBTEXT_COLOR,
            Color::ResetAll,
            BLUE_COLOR,
            expr.operator.kind,
            Color::ResetAll
        ));
        self.visit_expr(&expr.left);
        self.visit_expr(&expr.right);
        self.indent -= INDENT_SIZE;
    }

    fn visit_paren_expr(&mut self, expr: &ASTParenExpr) {
        self.print_indent(&format!(
            "{}Parenthesized{}:",
            SUBTEXT_COLOR,
            Color::ResetAll
        ));
        self.indent += INDENT_SIZE;
        self.visit_expr(&expr.expr);
        self.indent -= INDENT_SIZE;
    }

    fn visit_assignment_expr(&mut self, expr: &ASTAssignmentExpr) {
        self.print_indent(&format!("{}Assignment{}:", SUBTEXT_COLOR, Color::ResetAll));
        self.indent += INDENT_SIZE;
        self.print_indent(&format!(
            "{}Name{}: {}{}{}",
            SUBTEXT_COLOR,
            Color::ResetAll,
            LAVENDAR_COLOR,
            expr.name,
            Color::ResetAll
        ));
        self.print_indent(&format!(
            "{}Operator{}: {}{}{}",
            SUBTEXT_COLOR,
            Color::ResetAll,
            BLUE_COLOR,
            expr.op,
            Color::ResetAll
        ));
        self.visit_expr(&expr.value);
        self.indent -= INDENT_SIZE;
    }

    fn visit_cast_expr(&mut self, expr: &ASTCastExpr) {
        self.print_indent(&format!("{}Cast{}:", SUBTEXT_COLOR, Color::ResetAll));
        self.indent += INDENT_SIZE;
        self.print_indent(&format!(
            "{}Type{}: {}",
            SUBTEXT_COLOR,
            Color::ResetAll,
            expr.target.to_string()
        ));
        self.visit_expr(&expr.expr);
        self.indent -= INDENT_SIZE;
    }

    fn visit_var_dec(&mut self, dec: &ASTVarDecExpr) {
        if let TokenKind::Identifier(name) = &dec.identifier.kind {
            self.print_indent(&format!(
                "{}Variable Declaration{}:",
                SUBTEXT_COLOR,
                Color::ResetAll
            ));
            self.indent += INDENT_SIZE;
            self.print_indent(&format!(
                "{}Identifier{}: {}{}{}",
                SUBTEXT_COLOR,
                Color::ResetAll,
                LAVENDAR_COLOR,
                name,
                Color::ResetAll
            ));
            self.print_indent_sub(&format!(
                "{}Publicity{}: {}{}{}",
                SUBTEXT_COLOR,
                Color::ResetAll,
                RED_COLOR,
                if dec.pub_ {
                    "0x1 (true)"
                } else {
                    "0x0 (false)"
                },
                Color::ResetAll
            ));
            self.print_indent_sub(&format!(
                "{}Mutability{}: {}{}{}",
                SUBTEXT_COLOR,
                Color::ResetAll,
                RED_COLOR,
                if dec.mut_ {
                    "0x1 (true)"
                } else {
                    "0x0 (false)"
                },
                Color::ResetAll
            ));
            self.print_indent_sub(&format!(
                "{}Type{}: {}",
                SUBTEXT_COLOR,
                Color::ResetAll,
                dec.type_.clone().unwrap_or(0)
            ));
            self.visit_expr(&dec.initializer);
            self.indent -= INDENT_SIZE;
        }
    }

    fn visit_struct_dec(&mut self, dec: &ASTStructDecExpr) {
        if let TokenKind::Identifier(name) = &dec.identifier.kind {
            self.print_indent(&format!(
                "{}Struct Declaration{}:",
                SUBTEXT_COLOR,
                Color::ResetAll
            ));
            self.indent += INDENT_SIZE;
            self.print_indent(&format!(
                "{}Identifier{}: {}{}{}",
                SUBTEXT_COLOR,
                Color::ResetAll,
                LAVENDAR_COLOR,
                name,
                Color::ResetAll
            ));
            self.print_indent_sub(&format!(
                "{}Publicity{}: {}{}{}",
                SUBTEXT_COLOR,
                Color::ResetAll,
                RED_COLOR,
                if dec.pub_ {
                    "0x1 (true)"
                } else {
                    "0x0 (false)"
                },
                Color::ResetAll
            ));
            match &dec.generics {
                Some(generics) => {
                    self.print_indent(&format!("{}Generics{}:", SUBTEXT_COLOR, Color::ResetAll));
                    self.indent += INDENT_SIZE;
                    for generic in generics.iter() {
                        self.print_indent_sub(&format!("{}", generic));
                    }
                    self.indent -= INDENT_SIZE;
                }
                None => {}
            }
            self.print_indent(&format!("{}Fields{}:", SUBTEXT_COLOR, Color::ResetAll));
            self.indent += INDENT_SIZE;
            for field in dec.fields.iter() {
                if let TokenKind::Identifier(field_name) = &field.identifier.kind {
                    self.print_indent(&format!(
                        "{}Identifier{}: {}{}{}",
                        SUBTEXT_COLOR,
                        Color::ResetAll,
                        LAVENDAR_COLOR,
                        field_name,
                        Color::ResetAll
                    ));
                    self.print_indent_sub(&format!(
                        "{}Publicity{}: {}{}{}",
                        SUBTEXT_COLOR,
                        Color::ResetAll,
                        RED_COLOR,
                        if field.pub_ {
                            "0x1 (true)"
                        } else {
                            "0x0 (false)"
                        },
                        Color::ResetAll
                    ));
                    self.print_indent_sub(&format!(
                        "{}Type{}: {}{}{}",
                        SUBTEXT_COLOR,
                        Color::ResetAll,
                        LAVENDAR_COLOR,
                        field.type_,
                        Color::ResetAll
                    ));
                }
            }
            self.indent -= INDENT_SIZE * 2;
        }
    }

    fn visit_tuple_struct_dec(&mut self, dec: &ASTTupleStructDecExpr) {
        if let TokenKind::Identifier(name) = &dec.identifier.kind {
            self.print_indent(&format!(
                "{}Tuple Struct Declaration{}:",
                SUBTEXT_COLOR,
                Color::ResetAll
            ));
            self.indent += INDENT_SIZE;
            self.print_indent(&format!(
                "{}Identifier{}: {}{}{}",
                SUBTEXT_COLOR,
                Color::ResetAll,
                LAVENDAR_COLOR,
                name,
                Color::ResetAll
            ));
            self.print_indent_sub(&format!(
                "{}Publicity{}: {}{}{}",
                SUBTEXT_COLOR,
                Color::ResetAll,
                RED_COLOR,
                if dec.pub_ {
                    "0x1 (true)"
                } else {
                    "0x0 (false)"
                },
                Color::ResetAll
            ));
            match &dec.generics {
                Some(generics) => {
                    self.print_indent(&format!("{}Generics{}:", SUBTEXT_COLOR, Color::ResetAll));
                    self.indent += INDENT_SIZE;
                    for generic in generics.iter() {
                        self.print_indent_sub(&format!("{}", generic));
                    }
                    self.indent -= INDENT_SIZE;
                }
                None => {}
            }
            self.print_indent(&format!("{}Fields{}:", SUBTEXT_COLOR, Color::ResetAll));
            self.indent += INDENT_SIZE;
            for field in dec.fields.iter() {
                self.print_indent_sub(&format!("{}", field));
            }
            self.indent -= INDENT_SIZE * 2;
        }
    }

    fn visit_block_expr(&mut self, block: &ASTBlockExpr) {
        self.print_indent(&format!("{}Block{}:", SUBTEXT_COLOR, Color::ResetAll));
        self.indent += INDENT_SIZE;

        for stmt in block.statements.iter() {
            self.visit_stmt(stmt);
        }

        if let Some(expr) = &block.tail_expr {
            self.print_indent(&format!("{}Tail{}:", SUBTEXT_COLOR, Color::ResetAll));
            self.indent += INDENT_SIZE;
            self.visit_expr(expr);
            self.indent -= INDENT_SIZE;
        }

        self.indent -= INDENT_SIZE;
    }

    fn visit_unary_expr(&mut self, expr: &ASTUnaryExpr) {
        self.print_indent(&format!("{}Unary{}:", SUBTEXT_COLOR, Color::ResetAll));
        self.indent += INDENT_SIZE;
        self.print_indent(&format!(
            "{}Operator{}: {}{}{}",
            SUBTEXT_COLOR,
            Color::ResetAll,
            BLUE_COLOR,
            expr.op.kind,
            Color::ResetAll
        ));
        self.visit_expr(&expr.expr);
        self.indent -= INDENT_SIZE;
    }

    fn visit_error(&self) {
        self.print_indent(&format!("{}! Error !{}", RED_COLOR, Color::ResetAll));
    }

    fn visit_valued(&self, kind: &ASTExprKind) {
        match kind {
            ASTExprKind::Integer(v) => self.print_indent_sub(&format!(
                "{}Unsigned Integer{}({}{}{})",
                BLUE_COLOR,
                Color::ResetAll,
                PEACH_COLOR,
                v,
                Color::ResetAll
            )),
            ASTExprKind::Float(v) => self.print_indent_sub(&format!(
                "{}Float{}({}{}{})",
                BLUE_COLOR,
                Color::ResetAll,
                PEACH_COLOR,
                v,
                Color::ResetAll
            )),
            ASTExprKind::Byte(v) => self.print_indent_sub(&format!(
                "{}Byte{}({}{:02X}{})",
                BLUE_COLOR,
                Color::ResetAll,
                PEACH_COLOR,
                v,
                Color::ResetAll
            )),
            ASTExprKind::Char(v) => {
                let escaped = escape_char(*v);
                self.print_indent_sub(&format!(
                    "{}Char{}({}'{}'{})",
                    BLUE_COLOR,
                    Color::ResetAll,
                    GREEN_COLOR,
                    escaped,
                    Color::ResetAll
                ));
            }
            ASTExprKind::String(v) => {
                let escaped = escape_string(v);
                self.print_indent_sub(&format!(
                    "{}String{}({}\"{}\"{})",
                    BLUE_COLOR,
                    Color::ResetAll,
                    GREEN_COLOR,
                    escaped,
                    Color::ResetAll
                ));
            }
            ASTExprKind::ByteString(b) => self.print_indent_sub(&format!(
                "{}Byte String{}([{}{}{}])",
                BLUE_COLOR,
                Color::ResetAll,
                PEACH_COLOR,
                b.iter()
                    .map(|b| format!("{:02X}", b))
                    .collect::<Vec<_>>()
                    .join(", "),
                Color::ResetAll
            )),
            ASTExprKind::Bool(v) => self.print_indent_sub(&format!(
                "{}Bool{}({}{}{})",
                BLUE_COLOR,
                Color::ResetAll,
                RED_COLOR,
                v,
                Color::ResetAll
            )),
            ASTExprKind::Variable(v) => self.print_indent_sub(&format!(
                "{}Var{}({}{}{})",
                BLUE_COLOR,
                Color::ResetAll,
                LAVENDAR_COLOR,
                v,
                Color::ResetAll
            )),
            _ => {}
        }
    }
}

impl ASTPrinter {
    fn print_indent(&self, text: &str) {
        let prefix = if self.indent == 0 {
            ""
        } else {
            let s = " |> ";
            &(" ".repeat((self.indent - 1) * s.len()) + s)
        };

        println!("{}{}", prefix, text);
    }

    fn print_indent_sub(&self, text: &str) {
        let prefix = if self.indent == 0 {
            ""
        } else {
            &(" ".repeat(self.indent * 4))
        };

        println!("{}{}", prefix, text);
    }
}

fn escape_string(s: &str) -> String {
    s.chars()
        .flat_map(|c| match c {
            '\n' => "\\n".chars().collect::<Vec<_>>(),
            '\r' => "\\r".chars().collect(),
            '\t' => "\\t".chars().collect(),
            '\\' => "\\\\".chars().collect(),
            '"' => "\\\"".chars().collect(),
            c => vec![c],
        })
        .collect()
}

fn escape_char(c: char) -> String {
    match c {
        '\n' => "\\n".to_string(),
        '\r' => "\\r".to_string(),
        '\t' => "\\t".to_string(),
        '\\' => "\\\\".to_string(),
        '\'' => "\\\'".to_string(),
        c => c.to_string(),
    }
}

#[derive(Debug, Clone)]
pub enum ASTStmtKind {
    Expr(ASTExpr),
    Return(ASTExpr),
    VarDec(ASTVarDecExpr),
    StructDec(ASTStructDecExpr),
    TupleStructDec(ASTTupleStructDecExpr),
}

#[derive(Debug, Clone)]
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

    pub fn var_dec(
        identifier: Token,
        public: bool,
        mutable: bool,
        type_: Option<TypeId>,
        initializer: ASTExpr,
    ) -> Self {
        Self::new(ASTStmtKind::VarDec(ASTVarDecExpr::new(
            identifier,
            public,
            mutable,
            type_,
            initializer,
        )))
    }

    pub fn struct_dec(
        identifier: Token,
        public: bool,
        generics: Option<Vec<TypeId>>,
        fields: Vec<ASTStructField>,
    ) -> Self {
        Self::new(ASTStmtKind::StructDec(ASTStructDecExpr::new(
            identifier, public, generics, fields,
        )))
    }

    pub fn tuple_struct_dec(
        identifier: Token,
        public: bool,
        generics: Option<Vec<TypeId>>,
        fields: Vec<TypeId>,
    ) -> Self {
        Self::new(ASTStmtKind::TupleStructDec(ASTTupleStructDecExpr::new(
            identifier, public, generics, fields,
        )))
    }
}

#[derive(Debug, Clone)]
pub enum ASTExprKind {
    Integer(u64),
    Float(f64),
    Byte(u8),            // b''
    Char(char),          // ''
    String(String),      // "", r""
    ByteString(Vec<u8>), // b"", br""
    Bool(bool),
    Unary(ASTUnaryExpr),
    Binary(ASTBinaryExpr),
    Cast(ASTCastExpr),
    Parenthesized(ASTParenExpr),
    Assignment(ASTAssignmentExpr),
    Variable(&'static str),
    Block(ASTBlockExpr),
    Error,
}

impl Display for ASTExprKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            ASTExprKind::Integer(_) => write!(f, "Integer"),
            ASTExprKind::Float(_) => write!(f, "Float"),
            ASTExprKind::Byte(_) => write!(f, "Byte Char"),
            ASTExprKind::Char(_) => write!(f, "Char"),
            ASTExprKind::String(_) => write!(f, "String"),
            ASTExprKind::ByteString(_) => write!(f, "Byte String"),
            ASTExprKind::Bool(_) => write!(f, "Bool"),
            ASTExprKind::Parenthesized(_) => write!(f, "Parenthesized Expression"),
            ASTExprKind::Unary(_) => write!(f, "Unary Expression"),
            ASTExprKind::Binary(_) => write!(f, "Binary Expression"),
            ASTExprKind::Cast(_) => write!(f, "Cast Expression"),
            ASTExprKind::Assignment(_) => write!(f, "Assignment Expression"),
            ASTExprKind::Variable(_) => write!(f, "Variable"),
            ASTExprKind::Block(_) => write!(f, "Block Expression"),
            ASTExprKind::Error => write!(f, "ExprError"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ASTExpr {
    pub kind: ASTExprKind,
    pub span: Span,
}

impl ASTExpr {
    pub fn new(kind: ASTExprKind, span: Span) -> Self {
        Self { kind, span }
    }

    pub fn int(value: u64, span: Span) -> Self {
        Self::new(ASTExprKind::Integer(value), span)
    }

    pub fn float(value: f64, span: Span) -> Self {
        Self::new(ASTExprKind::Float(value), span)
    }

    pub fn byte(value: u8, span: Span) -> Self {
        Self::new(ASTExprKind::Byte(value), span)
    }

    pub fn char(value: char, span: Span) -> Self {
        Self::new(ASTExprKind::Char(value), span)
    }

    pub fn string(value: String, span: Span) -> Self {
        Self::new(ASTExprKind::String(value), span)
    }

    pub fn byte_string(value: Vec<u8>, span: Span) -> Self {
        Self::new(ASTExprKind::ByteString(value), span)
    }

    pub fn bool(value: bool, span: Span) -> Self {
        Self::new(ASTExprKind::Bool(value), span)
    }

    pub fn binary(left: ASTExpr, right: ASTExpr, operator: ASTBinaryOperator, span: Span) -> Self {
        Self::new(
            ASTExprKind::Binary(ASTBinaryExpr::new(left, right, operator)),
            span,
        )
    }

    pub fn parenthesized(expr: ASTExpr, span: Span) -> Self {
        Self::new(ASTExprKind::Parenthesized(ASTParenExpr::new(expr)), span)
    }

    pub fn cast(expr: ASTExpr, target: TypeId, span: Span) -> Self {
        Self::new(ASTExprKind::Cast(ASTCastExpr::new(expr, target)), span)
    }

    pub fn assignment(name: String, op: TokenKind, value: ASTExpr, span: Span) -> Self {
        let op_kind = match op {
            TokenKind::Equals => ASTBinaryOperatorKind::Assign,
            TokenKind::PlusEquals => ASTBinaryOperatorKind::AddAssign,
            TokenKind::MinusEquals => ASTBinaryOperatorKind::SubtractAssign,
            TokenKind::AsteriskEquals => ASTBinaryOperatorKind::MultiplyAssign,
            TokenKind::SlashEquals => ASTBinaryOperatorKind::DivideAssign,
            _ => panic!("unsupported assignment operator"),
        };

        Self::new(
            ASTExprKind::Assignment(ASTAssignmentExpr::new(name, op_kind, value)),
            span,
        )
    }

    pub fn variable(name: &'static str, span: Span) -> Self {
        Self::new(ASTExprKind::Variable(name), span)
    }

    pub fn unary(op: ASTUnaryOperator, expr: ASTExpr, span: Span) -> Self {
        Self::new(ASTExprKind::Unary(ASTUnaryExpr::new(op, expr)), span)
    }

    pub fn block(stmts: Vec<ASTStmt>, tail: Option<ASTExpr>, span: Span) -> Self {
        Self::new(ASTExprKind::Block(ASTBlockExpr::new(stmts, tail)), span)
    }

    pub fn error() -> Self {
        Self::new(ASTExprKind::Error, Span { start: 0, end: 0 })
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
        Self {
            left: Box::new(left),
            right: Box::new(right),
            operator,
        }
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
            // Arithmetic
            ASTBinaryOperatorKind::Multiply
            | ASTBinaryOperatorKind::Divide
            | ASTBinaryOperatorKind::Modulus => 3,
            ASTBinaryOperatorKind::Add | ASTBinaryOperatorKind::Subtract => 2,

            // Vergleich
            ASTBinaryOperatorKind::Less
            | ASTBinaryOperatorKind::Greater
            | ASTBinaryOperatorKind::LessEqual
            | ASTBinaryOperatorKind::GreaterEqual
            | ASTBinaryOperatorKind::Equal
            | ASTBinaryOperatorKind::NotEqual
            | ASTBinaryOperatorKind::LogicOr
            | ASTBinaryOperatorKind::LogicAnd => 1,

            // Assignment
            ASTBinaryOperatorKind::Assign
            | ASTBinaryOperatorKind::AddAssign
            | ASTBinaryOperatorKind::SubtractAssign
            | ASTBinaryOperatorKind::MultiplyAssign
            | ASTBinaryOperatorKind::DivideAssign => 0,

            _ => 0,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ASTBinaryOperatorKind {
    // Arithmetic
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulus,

    // Assignment
    Assign,
    AddAssign,
    SubtractAssign,
    MultiplyAssign,
    DivideAssign,

    // Comparison
    Equal,        // ==
    NotEqual,     // !=
    Less,         // <
    Greater,      // >
    LessEqual,    // <=
    GreaterEqual, // >=

    // Bitwise
    BitAnd, // &
    BitOr,  // |

    // Logical
    LogicOr,  // ||
    LogicAnd, // &&
    LogicXor, // ^

    LBitShift, // <<
    RBitShift, // >>
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

            ASTBinaryOperatorKind::Equal => write!(f, "[?] Equal (==)"),
            ASTBinaryOperatorKind::NotEqual => write!(f, "[?] Not Equal (!=)"),
            ASTBinaryOperatorKind::Less => write!(f, "[?] Less (<)"),
            ASTBinaryOperatorKind::Greater => write!(f, "[?] Greater (>)"),
            ASTBinaryOperatorKind::LessEqual => write!(f, "[?] Less (<) || Equal (==)"),
            ASTBinaryOperatorKind::GreaterEqual => write!(f, "[?] Greater (>) || Equal (==)"),

            ASTBinaryOperatorKind::BitAnd => write!(f, "Bitwise And (&)"),
            ASTBinaryOperatorKind::BitOr => write!(f, "Bitwise Or (|)"),

            ASTBinaryOperatorKind::LogicAnd => write!(f, "Logical And (&&)"),
            ASTBinaryOperatorKind::LogicOr => write!(f, "Logical Or (||)"),
            ASTBinaryOperatorKind::LogicXor => write!(f, "Logical Xor (^)"),

            ASTBinaryOperatorKind::LBitShift => write!(f, "Left Bit Shift (<<)"),
            ASTBinaryOperatorKind::RBitShift => write!(f, "Right Bit Shift (>>)"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ASTParenExpr {
    pub expr: Box<ASTExpr>,
}

impl ASTParenExpr {
    pub fn new(expr: ASTExpr) -> Self {
        Self {
            expr: Box::new(expr),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ASTVarDecExpr {
    identifier: Token,
    pub_: bool,
    mut_: bool,
    type_: Option<TypeId>,
    initializer: ASTExpr,
}

impl ASTVarDecExpr {
    pub fn new(
        identifier: Token,
        pub_: bool,
        mut_: bool,
        type_: Option<TypeId>,
        initializer: ASTExpr,
    ) -> Self {
        Self {
            identifier,
            pub_,
            mut_,
            type_,
            initializer,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ASTStructDecExpr {
    identifier: Token,
    pub_: bool,
    generics: Option<Vec<TypeId>>,
    fields: Vec<ASTStructField>,
}

impl ASTStructDecExpr {
    pub fn new(
        identifier: Token,
        pub_: bool,
        generics: Option<Vec<TypeId>>,
        fields: Vec<ASTStructField>,
    ) -> Self {
        Self {
            identifier,
            pub_,
            generics,
            fields,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ASTStructField {
    // identifier ist optional: bei Tuple Struct None
    identifier: Token,
    pub_: bool,
    type_: TypeId,
}

#[derive(Debug, Clone)]
pub struct ASTTupleStructDecExpr {
    identifier: Token,
    pub_: bool,
    generics: Option<Vec<TypeId>>,
    fields: Vec<TypeId>,
}

impl ASTTupleStructDecExpr {
    pub fn new(
        identifier: Token,
        pub_: bool,
        generics: Option<Vec<TypeId>>,
        fields: Vec<TypeId>,
    ) -> Self {
        Self {
            identifier,
            pub_,
            generics,
            fields,
        }
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
        Self {
            name,
            op,
            value: Box::new(value),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ASTCastExpr {
    pub expr: Box<ASTExpr>,
    pub target: TypeId,
}

impl ASTCastExpr {
    pub fn new(expr: ASTExpr, target: TypeId) -> Self {
        Self {
            expr: Box::new(expr),
            target,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ASTUnaryExpr {
    op: ASTUnaryOperator,
    expr: Box<ASTExpr>,
}

impl ASTUnaryExpr {
    pub fn new(op: ASTUnaryOperator, expr: ASTExpr) -> Self {
        Self {
            op,
            expr: Box::new(expr),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ASTUnaryOperator {
    pub kind: ASTUnaryOperatorKind,
    pub token: Token,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ASTUnaryOperatorKind {
    Negate, // -
    Not,    // !

    PreIncrement,
    PostIncrement,

    PreDecrement,
    PostDecrement,

    Ref,    // &
    RefMut, // &mut
    Deref,  // *
}

impl Display for ASTUnaryOperatorKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            ASTUnaryOperatorKind::Negate => write!(f, "Negate (-)"),
            ASTUnaryOperatorKind::Not => write!(f, "Logical Not (!)"),

            ASTUnaryOperatorKind::PreIncrement => write!(f, "Pre-Increment (++...)"),
            ASTUnaryOperatorKind::PostIncrement => write!(f, "Post-Increment (...++)"),

            ASTUnaryOperatorKind::PreDecrement => write!(f, "Pre-Decrement (--...)"),
            ASTUnaryOperatorKind::PostDecrement => write!(f, "Post-Decrement (...--)"),

            ASTUnaryOperatorKind::Ref => write!(f, "Reference (&)"),
            ASTUnaryOperatorKind::RefMut => write!(f, "Mutable Reference (&mut)"),
            ASTUnaryOperatorKind::Deref => write!(f, "Dereference (*)"),
        }
    }
}

impl ASTUnaryOperator {
    pub fn new(kind: ASTUnaryOperatorKind, token: Token) -> Self {
        Self { kind, token }
    }
}

#[derive(Debug, Clone)]
pub struct ASTBlockExpr {
    pub statements: Vec<ASTStmt>,
    pub tail_expr: Option<Box<ASTExpr>>,
}

impl ASTBlockExpr {
    pub fn new(stmts: Vec<ASTStmt>, tail: Option<ASTExpr>) -> Self {
        Self {
            statements: stmts,
            tail_expr: tail.map(|t| Box::new(t)),
        }
    }
}

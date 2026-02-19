pub mod lexer;
pub mod parser;
pub mod scope;
pub mod token;
pub mod traits;
pub mod typechecker;

use std::fmt::{Display, Formatter, Result};
use std::io::{self, Write};

use crate::color::{
    Color, BLUE_COLOR, GREEN_COLOR, LAVENDAR_COLOR, PEACH_COLOR, RED_COLOR, SUBTEXT_COLOR,
};
use crate::source::Span;
use crate::types::Ty;
use token::{Token, TokenKind};

pub struct AST {
    pub items: Vec<ASTItem>,
}

impl AST {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn add_item(&mut self, item: ASTItem) {
        self.items.push(item);
    }

    pub fn visit(&self, visitor: &mut dyn ASTVisitor) -> io::Result<()> {
        for item in &self.items {
            match item {
                ASTItem::Stmt(stmt) => visitor.visit_stmt(stmt)?,
                _ => todo!(),
            }
        }
        Ok(())
    }

    pub fn visualize(&self) {
        let mut output = io::stdout();
        let mut printer = ASTPrinter {
            indent: 0,
            out: &mut output,
        };
        let _ = self.visit(&mut printer);
    }
}

pub enum ASTItem {
    Stmt(ASTStmt),
    Use(ASTUse),
    Mod(ASTMod),
    // später: Fn, Struct, etc.
}

#[derive(Debug, Clone)]
pub struct ASTUse {
    pub path: Box<[String]>,
    pub alias: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ASTMod {
    pub name: String,
}

pub trait ASTVisitor {
    fn do_visit_stmt(&mut self, stmt: &ASTStmt) -> io::Result<()> {
        match &stmt.kind {
            ASTStmtKind::Expr(expr) => self.visit_expr(expr),
            ASTStmtKind::Return(_ret) => todo!(),
            ASTStmtKind::VarDec(dec) => self.visit_var_dec(dec),
            ASTStmtKind::StructDec(dec) => self.visit_struct_dec(dec),
            ASTStmtKind::TupleStructDec(dec) => self.visit_tuple_struct_dec(dec),
        }
    }

    fn visit_stmt(&mut self, stmt: &ASTStmt) -> io::Result<()> {
        let _ = self.do_visit_stmt(stmt);
        Ok(())
    }

    fn do_visit_expr(&mut self, expr: &ASTExpr) -> io::Result<()> {
        match expr.kind.as_ref() {
            ASTExprKind::Integer(_)
            | ASTExprKind::Float(_)
            | ASTExprKind::Byte(_)
            | ASTExprKind::Char(_)
            | ASTExprKind::String(_)
            | ASTExprKind::ByteString(_)
            | ASTExprKind::Bool(_)
            | ASTExprKind::Variable(_) => {
                let _ = self.visit_valued(expr.kind.as_ref());
                Ok(())
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

    fn visit_expr(&mut self, expr: &ASTExpr) -> io::Result<()> {
        let _ = self.do_visit_expr(expr);
        Ok(())
    }

    fn visit_var_dec(&mut self, expr: &ASTVarDecExpr) -> io::Result<()>;

    fn visit_struct_dec(&mut self, expr: &ASTStructDecExpr) -> io::Result<()>;

    fn visit_tuple_struct_dec(&mut self, expr: &ASTTupleStructDecExpr) -> io::Result<()>;

    fn visit_binary_expr(&mut self, expr: &ASTBinaryExpr) -> io::Result<()> {
        let _ = self.visit_expr(&expr.left);
        let _ = self.visit_expr(&expr.right);
        Ok(())
    }

    fn visit_paren_expr(&mut self, expr: &ASTParenExpr) -> io::Result<()>;

    fn visit_assignment_expr(&mut self, expr: &ASTAssignmentExpr) -> io::Result<()>;

    fn visit_unary_expr(&mut self, expr: &ASTUnaryExpr) -> io::Result<()>;

    fn visit_cast_expr(&mut self, expr: &ASTCastExpr) -> io::Result<()>;

    fn visit_block_expr(&mut self, expr: &ASTBlockExpr) -> io::Result<()>;

    fn visit_error(&mut self) -> io::Result<()>;

    fn visit_valued(&mut self, kind: &ASTExprKind) -> io::Result<()>;
}

pub struct ASTPrinter<'a, W: Write> {
    indent: usize,
    out: &'a mut W,
}

const INDENT_SIZE: usize = 1;

impl<'a, W: Write> ASTVisitor for ASTPrinter<'a, W> {
    fn visit_stmt(&mut self, stmt: &ASTStmt) -> io::Result<()> {
        let _ = self.line(format_args!(
            "{}Statement{}:",
            SUBTEXT_COLOR,
            Color::ResetAll
        ));
        self.indent += INDENT_SIZE;
        let _ = ASTVisitor::do_visit_stmt(self, stmt);
        self.indent -= INDENT_SIZE;
        println!();
        Ok(())
    }

    fn visit_expr(&mut self, expr: &ASTExpr) -> io::Result<()> {
        let _ = self.line(format_args!(
            "{}Expression{}:",
            SUBTEXT_COLOR,
            Color::ResetAll
        ));
        self.indent += INDENT_SIZE;
        let _ = ASTVisitor::do_visit_expr(self, expr);
        self.indent -= INDENT_SIZE;
        Ok(())
    }

    fn visit_binary_expr(&mut self, expr: &ASTBinaryExpr) -> io::Result<()> {
        let _ = self.line(format_args!("{}Binary{}:", SUBTEXT_COLOR, Color::ResetAll));
        self.indent += INDENT_SIZE;
        let _ = self.line(format_args!(
            "{}Operator{}: {}{}{}",
            SUBTEXT_COLOR,
            Color::ResetAll,
            BLUE_COLOR,
            expr.operator.kind,
            Color::ResetAll
        ));
        let _ = self.visit_expr(&expr.left);
        let _ = self.visit_expr(&expr.right);
        self.indent -= INDENT_SIZE;
        Ok(())
    }

    fn visit_paren_expr(&mut self, expr: &ASTParenExpr) -> io::Result<()> {
        let _ = self.line(format_args!(
            "{}Parenthesized{}:",
            SUBTEXT_COLOR,
            Color::ResetAll
        ));
        self.indent += INDENT_SIZE;
        let _ = self.visit_expr(&expr.expr);
        self.indent -= INDENT_SIZE;
        Ok(())
    }

    fn visit_assignment_expr(&mut self, expr: &ASTAssignmentExpr) -> io::Result<()> {
        if let TokenKind::Identifier(name) = &expr.target.kind {
            let _ = self.line(format_args!(
                "{}Assignment{}:",
                SUBTEXT_COLOR,
                Color::ResetAll
            ));
            self.indent += INDENT_SIZE;
            let _ = self.line(format_args!(
                "{}Name{}: {}{}{}",
                SUBTEXT_COLOR,
                Color::ResetAll,
                LAVENDAR_COLOR,
                name,
                Color::ResetAll
            ));
            let _ = self.line(format_args!(
                "{}Operator{}: {}{}{}",
                SUBTEXT_COLOR,
                Color::ResetAll,
                BLUE_COLOR,
                expr.op,
                Color::ResetAll
            ));
            let _ = self.visit_expr(&expr.value);
            self.indent -= INDENT_SIZE;
        }
        Ok(())
    }

    fn visit_cast_expr(&mut self, expr: &ASTCastExpr) -> io::Result<()> {
        let _ = self.line(format_args!("{}Cast{}:", SUBTEXT_COLOR, Color::ResetAll));
        self.indent += INDENT_SIZE;
        let _ = self.line(format_args!(
            "{}Type{}: {:?}",
            SUBTEXT_COLOR,
            Color::ResetAll,
            expr.target
        ));
        let _ = self.visit_expr(&expr.expr);
        self.indent -= INDENT_SIZE;
        Ok(())
    }

    fn visit_var_dec(&mut self, dec: &ASTVarDecExpr) -> io::Result<()> {
        if let TokenKind::Identifier(name) = &dec.identifier.kind {
            let _ = self.line(format_args!(
                "{}Variable Declaration{}:",
                SUBTEXT_COLOR,
                Color::ResetAll
            ));
            self.indent += INDENT_SIZE;
            let _ = self.line(format_args!(
                "{}Identifier{}: {}{}{}",
                SUBTEXT_COLOR,
                Color::ResetAll,
                LAVENDAR_COLOR,
                name,
                Color::ResetAll
            ));
            let _ = self.line_sub(format_args!(
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
            let _ = self.line_sub(format_args!(
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
            let _ = self.line_sub(format_args!(
                "{}Type{}: {:?}",
                SUBTEXT_COLOR,
                Color::ResetAll,
                dec.type_.clone().unwrap_or(Ty(0))
            ));
            let _ = self.visit_expr(&dec.initializer);
            self.indent -= INDENT_SIZE;
        }
        Ok(())
    }

    fn visit_struct_dec(&mut self, dec: &ASTStructDecExpr) -> io::Result<()> {
        if let TokenKind::Identifier(name) = &dec.identifier.kind {
            let _ = self.line(format_args!(
                "{}Struct Declaration{}:",
                SUBTEXT_COLOR,
                Color::ResetAll
            ));
            self.indent += INDENT_SIZE;
            let _ = self.line(format_args!(
                "{}Identifier{}: {}{}{}",
                SUBTEXT_COLOR,
                Color::ResetAll,
                LAVENDAR_COLOR,
                name,
                Color::ResetAll
            ));
            let _ = self.line_sub(format_args!(
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
            if !dec.generics.is_empty() {
                let _ = self.line(format_args!(
                    "{}Generics{}:",
                    SUBTEXT_COLOR,
                    Color::ResetAll
                ));
                self.indent += INDENT_SIZE;
                for generic in dec.generics.iter() {
                    let _ = self.line_sub(format_args!("{:?}", generic));
                }
                self.indent -= INDENT_SIZE;
            }
            let _ = self.line(format_args!("{}Fields{}:", SUBTEXT_COLOR, Color::ResetAll));
            self.indent += INDENT_SIZE;
            for field in dec.fields.iter() {
                if let TokenKind::Identifier(field_name) = &field.identifier.kind {
                    let _ = self.line(format_args!(
                        "{}Identifier{}: {}{}{}",
                        SUBTEXT_COLOR,
                        Color::ResetAll,
                        LAVENDAR_COLOR,
                        field_name,
                        Color::ResetAll
                    ));
                    let _ = self.line_sub(format_args!(
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
                    let _ = self.line_sub(format_args!(
                        "{}Type{}: {}{:?}{}",
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
        Ok(())
    }

    fn visit_tuple_struct_dec(&mut self, dec: &ASTTupleStructDecExpr) -> io::Result<()> {
        if let TokenKind::Identifier(name) = &dec.identifier.kind {
            let _ = self.line(format_args!(
                "{}Tuple Struct Declaration{}:",
                SUBTEXT_COLOR,
                Color::ResetAll
            ));
            self.indent += INDENT_SIZE;
            let _ = self.line(format_args!(
                "{}Identifier{}: {}{}{}",
                SUBTEXT_COLOR,
                Color::ResetAll,
                LAVENDAR_COLOR,
                name,
                Color::ResetAll
            ));
            let _ = self.line_sub(format_args!(
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
            if !dec.generics.is_empty() {
                let _ = self.line(format_args!(
                    "{}Generics{}:",
                    SUBTEXT_COLOR,
                    Color::ResetAll
                ));
                self.indent += INDENT_SIZE;
                for generic in dec.generics.iter() {
                    let _ = self.line_sub(format_args!("{:?}", generic));
                }
                self.indent -= INDENT_SIZE;
            }
            let _ = self.line(format_args!("{}Fields{}:", SUBTEXT_COLOR, Color::ResetAll));
            self.indent += INDENT_SIZE;
            for field in dec.fields.iter() {
                let _ = self.line_sub(format_args!("{:?}", field));
            }
            self.indent -= INDENT_SIZE * 2;
        }
        Ok(())
    }

    fn visit_block_expr(&mut self, block: &ASTBlockExpr) -> io::Result<()> {
        let _ = self.line(format_args!("{}Block{}:", SUBTEXT_COLOR, Color::ResetAll));
        self.indent += INDENT_SIZE;

        for stmt in block.statements.iter() {
            let _ = self.visit_stmt(stmt);
        }

        if let Some(expr) = &block.tail_expr {
            let _ = self.line(format_args!("{}Tail{}:", SUBTEXT_COLOR, Color::ResetAll));
            self.indent += INDENT_SIZE;
            let _ = self.visit_expr(expr);
            self.indent -= INDENT_SIZE;
        }

        self.indent -= INDENT_SIZE;
        Ok(())
    }

    fn visit_unary_expr(&mut self, expr: &ASTUnaryExpr) -> io::Result<()> {
        let _ = self.line(format_args!("{}Unary{}:", SUBTEXT_COLOR, Color::ResetAll));
        self.indent += INDENT_SIZE;
        let _ = self.line(format_args!(
            "{}Operator{}: {}{}{}",
            SUBTEXT_COLOR,
            Color::ResetAll,
            BLUE_COLOR,
            expr.op.kind,
            Color::ResetAll
        ));
        let _ = self.visit_expr(&expr.expr);
        self.indent -= INDENT_SIZE;
        Ok(())
    }

    fn visit_error(&mut self) -> io::Result<()> {
        let _ = self.line(format_args!("{}! Error !{}", RED_COLOR, Color::ResetAll));
        Ok(())
    }

    fn visit_valued(&mut self, kind: &ASTExprKind) -> io::Result<()> {
        match kind {
            ASTExprKind::Integer(v) => self.line_sub(format_args!(
                "{}Integer{}({}{}{})",
                BLUE_COLOR,
                Color::ResetAll,
                PEACH_COLOR,
                v,
                Color::ResetAll
            )),
            ASTExprKind::Float(v) => self.line_sub(format_args!(
                "{}Float{}({}{}{})",
                BLUE_COLOR,
                Color::ResetAll,
                PEACH_COLOR,
                v,
                Color::ResetAll
            )),
            ASTExprKind::Byte(v) => self.line_sub(format_args!(
                "{}Byte{}({}{:02X}{})",
                BLUE_COLOR,
                Color::ResetAll,
                PEACH_COLOR,
                v,
                Color::ResetAll
            )),
            ASTExprKind::Char(c) => {
                let _ = self.write_indent_sub();
                let _ = write!(self.out, "{}", BLUE_COLOR);
                let _ = write!(self.out, "Char");
                let _ = write!(self.out, "{}", Color::ResetAll);
                let _ = write!(self.out, "(");
                let _ = write!(self.out, "{}", GREEN_COLOR);
                let _ = escape_char(self.out, *c);
                let _ = write!(self.out, "{}", Color::ResetAll);
                let _ = write!(self.out, ")");
                writeln!(self.out)
            }
            ASTExprKind::String(s) => {
                let _ = self.write_indent_sub();
                let _ = write!(self.out, "{}", BLUE_COLOR);
                let _ = write!(self.out, "String");
                let _ = write!(self.out, "{}", Color::ResetAll);
                let _ = write!(self.out, "(");
                let _ = write!(self.out, "{}", GREEN_COLOR);
                let _ = escape_string(self.out, s);
                let _ = write!(self.out, "{}", Color::ResetAll);
                let _ = write!(self.out, ")");
                writeln!(self.out)
            }
            ASTExprKind::ByteString(b) => {
                let _ = self.write_indent_sub();
                let _ = write!(self.out, "{}", BLUE_COLOR);
                let _ = write!(self.out, "Byte String");
                let _ = write!(self.out, "{}", Color::ResetAll);
                let _ = write!(self.out, "[");
                let _ = write!(self.out, "{}", PEACH_COLOR);

                for (i, byte) in b.iter().enumerate() {
                    if i > 0 {
                        let _ = write!(self.out, ", "); // comma between bytes
                    }
                    let _ = write!(self.out, "{:02X}", byte); // hex output
                }

                let _ = write!(self.out, "{}", Color::ResetAll);
                let _ = write!(self.out, "]");
                writeln!(self.out)
            }

            ASTExprKind::Bool(v) => self.line_sub(format_args!(
                "{}Bool{}({}{}{})",
                BLUE_COLOR,
                Color::ResetAll,
                RED_COLOR,
                v,
                Color::ResetAll
            )),
            ASTExprKind::Variable(v) => self.line_sub(format_args!(
                "{}Var{}({}{}{})",
                BLUE_COLOR,
                Color::ResetAll,
                LAVENDAR_COLOR,
                v,
                Color::ResetAll
            )),
            _ => unreachable!(),
        }
    }

    fn do_visit_stmt(&mut self, stmt: &ASTStmt) -> io::Result<()> {
        match &stmt.kind {
            ASTStmtKind::Expr(expr) => self.visit_expr(expr),
            ASTStmtKind::Return(_ret) => todo!("return statement"),
            ASTStmtKind::VarDec(dec) => self.visit_var_dec(dec),
            ASTStmtKind::StructDec(dec) => self.visit_struct_dec(dec),
            ASTStmtKind::TupleStructDec(dec) => self.visit_tuple_struct_dec(dec),
        }
    }

    fn do_visit_expr(&mut self, expr: &ASTExpr) -> io::Result<()> {
        match expr.kind.as_ref() {
            ASTExprKind::Integer(_)
            | ASTExprKind::Float(_)
            | ASTExprKind::Byte(_)
            | ASTExprKind::Char(_)
            | ASTExprKind::String(_)
            | ASTExprKind::ByteString(_)
            | ASTExprKind::Bool(_)
            | ASTExprKind::Variable(_) => {
                let _ = self.visit_valued(expr.kind.as_ref());
                Ok(())
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
}

const SPACES: [u8; 256] = [b' '; 256];

impl<'a, W: Write> ASTPrinter<'a, W> {
    #[inline(always)]
    fn write_spaces(out: &mut impl Write, count: usize) -> io::Result<()> {
        let mut remaining = count;
        while remaining > 0 {
            let chunk = remaining.min(SPACES.len());
            let _ = out.write_all(&SPACES[..chunk]);
            remaining -= chunk;
        }
        Ok(())
    }

    #[inline(always)]
    fn write_indent(&mut self) -> io::Result<()> {
        if self.indent > 0 {
            let _ = Self::write_spaces(self.out, (self.indent - 1) * 4);
            let _ = self.out.write_all(b" |> ");
        }
        Ok(())
    }

    #[inline(always)]
    fn write_indent_sub(&mut self) -> io::Result<()> {
        Self::write_spaces(self.out, self.indent * 4)
    }

    #[inline(always)]
    fn line(&mut self, args: std::fmt::Arguments) -> io::Result<()> {
        self.write_indent()?;
        self.out.write_fmt(args)?;
        self.out.write_all(b"\n")
    }

    #[inline(always)]
    fn line_sub(&mut self, args: std::fmt::Arguments) -> io::Result<()> {
        self.write_indent_sub()?;
        self.out.write_fmt(args)?;
        self.out.write_all(b"\n")
    }
}

fn escape_string(out: &mut impl Write, s: &str) -> io::Result<()> {
    for c in s.chars() {
        match c {
            '\n' => out.write_all(b"\\n"),
            '\r' => out.write_all(b"\\r"),
            '\t' => out.write_all(b"\\t"),
            '\\' => out.write_all(b"\\\\"),
            '"' => out.write_all(b"\\\""),
            c => write!(out, "{}", c),
        }?
    }
    Ok(())
}

fn escape_char(out: &mut impl Write, c: char) -> io::Result<()> {
    match c {
        '\n' => out.write_all(b"\\n"),
        '\r' => out.write_all(b"\\r"),
        '\t' => out.write_all(b"\\t"),
        '\\' => out.write_all(b"\\\\"),
        '\'' => out.write_all(b"\\\'"),
        c => write!(out, "{}", c),
    }?;
    Ok(())
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
        type_: Option<Ty>,
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
        generics: Box<[Ty]>,
        fields: Box<[ASTStructField]>,
    ) -> Self {
        Self::new(ASTStmtKind::StructDec(ASTStructDecExpr::new(
            identifier, public, generics, fields,
        )))
    }

    pub fn tuple_struct_dec(
        identifier: Token,
        public: bool,
        generics: Box<[Ty]>,
        fields: Box<[ASTTupleStructField]>,
    ) -> Self {
        Self::new(ASTStmtKind::TupleStructDec(ASTTupleStructDecExpr::new(
            identifier, public, generics, fields,
        )))
    }
}

#[derive(Debug, Clone)]
pub enum ASTExprKind {
    Integer(u128),
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
    Block(Box<ASTBlockExpr>),
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
    pub kind: Box<ASTExprKind>,
    pub span: Span,
}

impl ASTExpr {
    pub fn new(kind: ASTExprKind, span: Span) -> Self {
        Self {
            kind: Box::new(kind),
            span,
        }
    }

    pub fn int(value: u128, span: Span) -> Self {
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

    pub fn cast(expr: ASTExpr, target: Ty, span: Span) -> Self {
        Self::new(ASTExprKind::Cast(ASTCastExpr::new(expr, target)), span)
    }

    pub fn assignment(target: Token, op: TokenKind, value: ASTExpr, span: Span) -> Self {
        let op_kind = match op {
            TokenKind::Equals => ASTBinaryOperatorKind::Assign,
            TokenKind::PlusEquals => ASTBinaryOperatorKind::AddAssign,
            TokenKind::MinusEquals => ASTBinaryOperatorKind::SubtractAssign,
            TokenKind::AsteriskEquals => ASTBinaryOperatorKind::MultiplyAssign,
            TokenKind::SlashEquals => ASTBinaryOperatorKind::DivideAssign,
            _ => {
                dbg!(op);
                panic!("Invalid assignment operator")
            }
        };

        Self::new(
            ASTExprKind::Assignment(ASTAssignmentExpr::new(target, op_kind, value)),
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
        Self::new(
            ASTExprKind::Block(Box::new(ASTBlockExpr::new(stmts, tail))),
            span,
        )
    }

    pub fn error(file_id: usize) -> Self {
        Self::new(ASTExprKind::Error, Span::new(0, 0, file_id))
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

#[derive(Debug, Clone, PartialEq)]
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
            | ASTBinaryOperatorKind::Remainder => 3,
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

#[derive(Debug, Clone, PartialEq)]
pub enum ASTBinaryOperatorKind {
    // Arithmetic
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,

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
            ASTBinaryOperatorKind::Remainder => write!(f, "Remainder (%)"),

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
    pub expr: ASTExpr,
}

impl ASTParenExpr {
    pub fn new(expr: ASTExpr) -> Self {
        Self { expr }
    }
}

#[derive(Debug, Clone)]
pub struct ASTVarDecExpr {
    identifier: Token,
    pub_: bool,
    mut_: bool,
    type_: Option<Ty>,
    initializer: ASTExpr,
}

impl ASTVarDecExpr {
    pub fn new(
        identifier: Token,
        pub_: bool,
        mut_: bool,
        type_: Option<Ty>,
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
    pub identifier: Token,
    pub pub_: bool,
    pub generics: Box<[Ty]>,
    pub fields: Box<[ASTStructField]>,
}

impl ASTStructDecExpr {
    pub fn new(
        identifier: Token,
        pub_: bool,
        generics: Box<[Ty]>,
        fields: Box<[ASTStructField]>,
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
    pub identifier: Token,
    pub pub_: bool,
    pub type_: Ty,
}

#[derive(Debug, Clone)]
pub struct ASTTupleStructDecExpr {
    pub identifier: Token,
    pub pub_: bool,
    pub generics: Box<[Ty]>,
    pub fields: Box<[ASTTupleStructField]>,
}

impl ASTTupleStructDecExpr {
    pub fn new(
        identifier: Token,
        pub_: bool,
        generics: Box<[Ty]>,
        fields: Box<[ASTTupleStructField]>,
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
pub struct ASTTupleStructField {
    pub pub_: bool,
    pub type_: Ty,
}

#[derive(Debug, Clone)]
pub struct ASTAssignmentExpr {
    target: Token,
    op: ASTBinaryOperatorKind,
    value: Box<ASTExpr>,
}

impl ASTAssignmentExpr {
    pub fn new(target: Token, op: ASTBinaryOperatorKind, value: ASTExpr) -> Self {
        Self {
            target,
            op,
            value: Box::new(value),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ASTCastExpr {
    pub expr: Box<ASTExpr>,
    pub target: Ty,
}

impl ASTCastExpr {
    pub fn new(expr: ASTExpr, target: Ty) -> Self {
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

    PreIncrement,  // ++x
    PostIncrement, // x++

    PreDecrement,  // --x
    PostDecrement, // x--

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

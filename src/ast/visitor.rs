use std::fmt::Arguments;
use std::io::{self, Write};

use smallvec::SmallVec;

use crate::ast::strings::{StringId, StringPool};
use crate::ast::{
    ASTAssignmentExpr, ASTBinaryExpr, ASTBlockExpr, ASTCallExpr, ASTCastExpr, ASTConstDecExpr,
    ASTEnumDecExpr, ASTEnumVariant, ASTEnumVariantKind, ASTExpr, ASTExprKind, ASTFieldAccessExpr,
    ASTFuncDecExpr, ASTFuncParam, ASTGenericParam, ASTIndexExpr, ASTMacroCallExpr,
    ASTMethodCallExpr, ASTParenExpr, ASTPathSegmentExpr, ASTStmt, ASTStmtKind, ASTStructDecExpr,
    ASTStructField, ASTTupleExpr, ASTTupleStructDecExpr, ASTTupleStructField, ASTType,
    ASTTypeAliasDecExpr, ASTUnaryExpr, ASTUnitStructDecExpr, ASTVarDecExpr, Mutability, Publicity,
};
use crate::color::{
    c, BLUE_COLOR, BOLD, GREEN_COLOR, LAVENDAR_COLOR, PEACH_COLOR, RED_COLOR, RESET, SUBTEXT_COLOR,
    YELLOW_COLOR,
};

// ---------------------------------------------------------------------------
// Walk functions
// ---------------------------------------------------------------------------

/// Dispatches a statement node to the appropriate visitor method.
///
/// This is the default traversal logic for [`ASTVisitor::visit_stmt`]. Implementors
/// can call this from their own `visit_stmt` override to retain default behaviour
/// while adding pre- or post-processing.
pub fn walk_stmt<V: ASTVisitor + ?Sized>(v: &mut V, stmt: &ASTStmt) -> Result<(), V::Error> {
    match &stmt.kind {
        ASTStmtKind::Expr(expr) => v.visit_expr(expr),
        // ASTStmtKind::Return(expr) => v.visit_expr(expr),
        ASTStmtKind::VarDec(expr) => v.visit_var_dec(expr),
        ASTStmtKind::ConstDec(expr) => v.visit_const_dec(expr),
        ASTStmtKind::StructDec(expr) => v.visit_struct_dec(expr),
        ASTStmtKind::TupleStructDec(expr) => v.visit_tuple_struct_dec(expr),
        ASTStmtKind::UnitStructDec(expr) => v.visit_unit_struct_dec(expr),
        ASTStmtKind::EnumDec(expr) => v.visit_enum_dec(expr),
        ASTStmtKind::TypeAliasDec(expr) => v.visit_type_alias(expr),
        ASTStmtKind::FuncDec(expr) => v.visit_func_dec(expr),
        ASTStmtKind::MacroDec(_expr) => Ok(()),
    }
}

/// Dispatches an expression node to the appropriate visitor method.
///
/// This is the default traversal logic for [`ASTVisitor::visit_expr`]. Implementors
/// can call this from their own `visit_expr` override to retain default behaviour
/// while adding pre- or post-processing.
pub fn walk_expr<V: ASTVisitor + ?Sized>(v: &mut V, expr: &ASTExpr) -> Result<(), V::Error> {
    match &expr.kind {
        ASTExprKind::Integer(..)
        | ASTExprKind::Float(..)
        | ASTExprKind::Byte(_)
        | ASTExprKind::Char(_)
        | ASTExprKind::String(_)
        | ASTExprKind::ByteString(_)
        | ASTExprKind::Bool(_)
        | ASTExprKind::Variable(..) => v.visit_valued(&expr.kind),

        ASTExprKind::Unary(expr) => v.visit_unary_expr(expr),
        ASTExprKind::Binary(expr) => v.visit_binary_expr(expr),
        ASTExprKind::Parenthesized(expr) => v.visit_paren_expr(expr),
        ASTExprKind::Assignment(expr) => v.visit_assignment_expr(expr),
        ASTExprKind::Cast(expr) => v.visit_cast_expr(expr),
        ASTExprKind::Block(expr) => v.visit_block_expr(expr),
        ASTExprKind::FieldAccess(expr) => v.visit_field_access(expr),
        ASTExprKind::MethodCall(expr) => v.visit_method_call(expr),
        ASTExprKind::Call(expr) => v.visit_call(expr),
        ASTExprKind::Index(expr) => v.visit_index(expr),
        ASTExprKind::PathSegment(expr) => v.visit_path_segment(expr),
        ASTExprKind::MacroCall(e) => v.visit_macro_call(e),
        ASTExprKind::Tuple(e) => v.visit_tuple(e),
        ASTExprKind::Unit => v.visit_unit(),
        ASTExprKind::Error => v.visit_error(),
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// A buffer of spaces used by [`write_spaces`] to avoid repeated allocations
/// when writing indentation.
const SPACES: [u8; 256] = [b' '; 256];

/// Number of spaces added per indentation level.
const INDENT_SIZE: usize = 4;

/// Writes `count` space characters to `out` using a fixed-size buffer,
/// avoiding repeated small writes.
fn write_spaces(out: &mut impl Write, count: usize) -> io::Result<()> {
    let mut remaining = count;
    while remaining > 0 {
        let chunk = remaining.min(SPACES.len());
        out.write_all(&SPACES[..chunk])?;
        remaining -= chunk;
    }
    Ok(())
}

/// Writes a string literal to `out` with common escape sequences applied
/// (`\n`, `\r`, `\t`, `\\`, `\"`).
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

/// Writes a character literal to `out` with common escape sequences applied
/// (`\n`, `\r`, `\t`, `\\`, `\'`).
fn escape_char(out: &mut impl Write, c: char) -> io::Result<()> {
    match c {
        '\n' => out.write_all(b"\\n"),
        '\r' => out.write_all(b"\\r"),
        '\t' => out.write_all(b"\\t"),
        '\\' => out.write_all(b"\\\\"),
        '\'' => out.write_all(b"\\\'"),
        c => write!(out, "{}", c),
    }
}

/// Renders an [`ASTType`] node as a coloured string suitable for terminal output.
///
/// The returned `String` uses ANSI escape codes from the `color` module and is
/// intended to be embedded directly in [`format_args!`] calls inside the printer.
fn format_type(ty: &ASTType, col: &Colors, string_pool: &StringPool) -> String {
    match ty {
        ASTType::Path(idents) => {
            let mut s = String::new();
            for (i, ident) in idents.iter().enumerate() {
                if i > 0 {
                    s.push_str(&format!("{}:{}", col.subtext, col.reset));
                }
                let ident_str = string_pool.get(ident.id).unwrap();
                if ["pkg", "super"].contains(&ident_str) {
                    s.push_str(&format!("{}{}{}", col.lavender, &ident_str, col.reset));
                } else {
                    s.push_str(&format!("{}{}{}", col.yellow, &ident_str, col.reset));
                }
            }
            s
        }
        ASTType::Ref { mutable, inner } => {
            if *mutable == Mutability::Mutable {
                format!(
                    "{}&mut{} {}",
                    col.lavender,
                    col.reset,
                    format_type(inner, col, string_pool)
                )
            } else {
                format!(
                    "{}&{}{}",
                    col.lavender,
                    col.reset,
                    format_type(inner, col, string_pool)
                )
            }
        }
        ASTType::Tuple(elems) => {
            let inner = elems
                .iter()
                .map(|t| format_type(t, col, string_pool))
                .collect::<Vec<_>>()
                .join(&format!("{},{} ", col.subtext, col.reset));
            format!(
                "{}({}{}{}){}",
                col.subtext, col.reset, inner, col.subtext, col.reset
            )
        }
        ASTType::Generic { base, args } => {
            let args_s = args
                .iter()
                .map(|t| format_type(t, col, string_pool))
                .collect::<Vec<_>>()
                .join(&format!("{},{} ", col.subtext, col.reset));
            format!(
                "{}{}<{}{}{}>{}",
                format_type(base, col, string_pool),
                col.subtext,
                col.reset,
                args_s,
                col.subtext,
                col.reset
            )
        }
        ASTType::Error => "<error>".to_string(),
    }
}

// ---------------------------------------------------------------------------
// ASTVisitor trait
// ---------------------------------------------------------------------------

/// A visitor over the AWH abstract syntax tree.
///
/// Each method corresponds to a single AST node kind. The default implementations
/// of [`visit_stmt`](ASTVisitor::visit_stmt) and [`visit_expr`](ASTVisitor::visit_expr)
/// delegate to [`walk_stmt`] and [`walk_expr`] respectively, which in turn call the
/// more specific `visit_*` methods. Override only what you need.
///
/// # Errors
///
/// Every method returns `Result<(), Self::Error>`. Returning `Err` aborts the
/// current traversal path — the caller is responsible for deciding whether to
/// propagate or recover.
pub trait ASTVisitor {
    /// The error type produced by this visitor.
    type Error;

    /// Visits a top-level statement node.
    ///
    /// The default implementation delegates to [`walk_stmt`].
    fn visit_stmt(&mut self, stmt: &ASTStmt) -> Result<(), Self::Error> {
        walk_stmt(self, stmt)
    }

    /// Visits an expression node.
    ///
    /// The default implementation delegates to [`walk_expr`].
    fn visit_expr(&mut self, expr: &ASTExpr) -> Result<(), Self::Error> {
        walk_expr(self, expr)
    }

    /// Returns a human-readable string representation of a [`Publicity`] value.
    ///
    /// Returns `"0x1 (true)"` for [`Publicity::Public`] and `"0x0 (false)"` otherwise.
    /// The return value is `&'static str` so it can be embedded into `format_args!`
    /// without allocation.
    fn publicity_str(&self, vis: &Publicity) -> &'static str;

    /// Returns a human-readable string representation of a [`Mutability`] value.
    ///
    /// Returns `"0x1 (true)"` for [`Publicity::Public`] and `"0x0 (false)"` otherwise.
    /// The return value is `&'static str` so it can be embedded into `format_args!`
    /// without allocation.
    fn mutablility_str(&self, vis: &Mutability) -> &'static str;

    /// Visits a list of generic type parameter definitions (e.g. `<T, U = Default>`).
    ///
    /// The list may be empty; implementations should handle that case gracefully.
    fn visit_generics(
        &mut self,
        generics: &SmallVec<[ASTGenericParam; 2]>,
    ) -> Result<(), Self::Error>;

    /// Visits a local variable declaration (`dec [mut] name [: Type] = expr;`).
    fn visit_var_dec(&mut self, expr: &ASTVarDecExpr) -> Result<(), Self::Error>;

    /// Visits a constant declaration (`const name [: Type] = expr;`).
    fn visit_const_dec(&mut self, expr: &ASTConstDecExpr) -> Result<(), Self::Error>;

    /// Visits the named fields of a labelled struct.
    fn visit_struct_fields(
        &mut self,
        fields: &SmallVec<[ASTStructField; 4]>,
    ) -> Result<(), Self::Error>;

    /// Visits the positional fields of a tuple struct.
    fn visit_tuple_struct_fields(
        &mut self,
        fields: &SmallVec<[ASTTupleStructField; 4]>,
    ) -> Result<(), Self::Error>;

    /// Visits a labelled struct declaration (`struct Name { ... }`).
    fn visit_struct_dec(&mut self, expr: &ASTStructDecExpr) -> Result<(), Self::Error>;

    /// Visits a tuple struct declaration (`struct Name(Type, ...);`).
    fn visit_tuple_struct_dec(&mut self, expr: &ASTTupleStructDecExpr) -> Result<(), Self::Error>;

    /// Visits a unit struct declaration (`struct Name;`).
    fn visit_unit_struct_dec(&mut self, expr: &ASTUnitStructDecExpr) -> Result<(), Self::Error>;

    /// Visits an enum declaration (`enum Name { ... }`).
    fn visit_enum_dec(&mut self, expr: &ASTEnumDecExpr) -> Result<(), Self::Error>;

    /// Visits the list of variants inside an enum declaration.
    fn visit_enum_variants(
        &mut self,
        variants: &SmallVec<[ASTEnumVariant; 4]>,
    ) -> Result<(), Self::Error>;

    /// Visits a type alias declaration (`type Name = Type;`).
    fn visit_type_alias(&mut self, expr: &ASTTypeAliasDecExpr) -> Result<(), Self::Error>;

    /// Visits a type alias declaration (`func Name { ... }`).
    fn visit_func_dec(&mut self, expr: &ASTFuncDecExpr) -> Result<(), Self::Error>;

    /// Visits a type alias declaration (`func Name { ... }`).
    fn visit_func_params(&mut self, expr: &SmallVec<[ASTFuncParam; 4]>) -> Result<(), Self::Error>;

    /// Visits the functions body (`{ stmts... [tail] }`).
    fn visit_func_body(&mut self, expr: &ASTBlockExpr) -> Result<(), Self::Error>;

    /// Visits a binary expression (`lhs op rhs`).
    ///
    /// The default implementation visits the left operand first, then the right.
    fn visit_binary_expr(&mut self, expr: &ASTBinaryExpr) -> Result<(), Self::Error> {
        self.visit_expr(&expr.left)?;
        self.visit_expr(&expr.right)
    }

    /// Visits a parenthesized expression (`(expr)`).
    fn visit_paren_expr(&mut self, expr: &ASTParenExpr) -> Result<(), Self::Error>;

    /// Visits an assignment expression (`name op= expr`).
    fn visit_assignment_expr(&mut self, expr: &ASTAssignmentExpr) -> Result<(), Self::Error>;

    /// Visits a unary expression (`op expr`).
    fn visit_unary_expr(&mut self, expr: &ASTUnaryExpr) -> Result<(), Self::Error>;

    /// Visits a type cast expression (`expr as Type`).
    fn visit_cast_expr(&mut self, expr: &ASTCastExpr) -> Result<(), Self::Error>;

    /// Visits a block expression (`{ stmts... [tail] }`).
    fn visit_block_expr(&mut self, expr: &ASTBlockExpr) -> Result<(), Self::Error>;

    /// Visits a field access expression (`expr.field`).
    fn visit_field_access(&mut self, expr: &ASTFieldAccessExpr) -> Result<(), Self::Error>;

    /// Visits a method call expression (`expr.method(args...)`).
    fn visit_method_call(&mut self, expr: &ASTMethodCallExpr) -> Result<(), Self::Error>;

    /// Visits a free function call expression (`expr(args...)`).
    fn visit_call(&mut self, expr: &ASTCallExpr) -> Result<(), Self::Error>;

    /// Visits an index expression (`expr[idx]`).
    fn visit_index(&mut self, expr: &ASTIndexExpr) -> Result<(), Self::Error>;

    /// Visits a path segment expression (`expr::segment`).
    fn visit_path_segment(&mut self, expr: &ASTPathSegmentExpr) -> Result<(), Self::Error>;

    /// Visits a macro invocation (`name!(args...)`).
    fn visit_macro_call(&mut self, expr: &ASTMacroCallExpr) -> Result<(), Self::Error>;

    /// Visits a tuple literal expression (`(a, b, ...)`).
    fn visit_tuple(&mut self, expr: &ASTTupleExpr) -> Result<(), Self::Error>;

    /// Visits the unit literal `()`.
    fn visit_unit(&mut self) -> Result<(), Self::Error>;

    /// Visits an error node produced by the parser when recovery occurred.
    fn visit_error(&mut self) -> Result<(), Self::Error>;

    /// Visits a type annotation node.
    fn visit_type(&mut self, ty: &ASTType) -> Result<(), Self::Error>;

    /// Visits a primitive-valued expression (integer, float, byte, char, string,
    /// byte string, bool, or variable reference).
    ///
    /// `kind` is guaranteed to be one of the value variants; any other variant
    /// will cause the default implementation to panic via `unreachable!()`.
    fn visit_valued(&mut self, kind: &ASTExprKind) -> Result<(), Self::Error>;
}

// ---------------------------------------------------------------------------
// LineKind
// ---------------------------------------------------------------------------

/// Controls which indentation style is used when writing a line.
///
/// - [`Normal`](LineKind::Normal) writes a leading `  - ` bullet marker at the
///   current indent level, used for tree nodes.
/// - [`Sub`](LineKind::Sub) writes plain leading spaces, used for properties and
///   annotations that belong to the preceding node.
#[derive(Clone, Copy)]
pub enum LineKind {
    /// Tree-node line: indented with a `  - ` bullet prefix.
    Normal,
    /// Property line: indented with plain spaces, no bullet.
    Sub,
}

/// Saves the color value if color is activated.
pub struct Colors {
    pub blue: &'static str,
    pub green: &'static str,
    pub red: &'static str,
    pub yellow: &'static str,
    pub lavender: &'static str,
    pub peach: &'static str,
    pub subtext: &'static str,
    pub bold: &'static str,
    pub reset: &'static str,
}

impl Colors {
    pub fn new() -> Self {
        Self {
            blue: c(BLUE_COLOR),
            green: c(GREEN_COLOR),
            red: c(RED_COLOR),
            yellow: c(YELLOW_COLOR),
            lavender: c(LAVENDAR_COLOR),
            peach: c(PEACH_COLOR),
            subtext: c(SUBTEXT_COLOR),
            bold: c(BOLD),
            reset: c(RESET),
        }
    }
}

// ---------------------------------------------------------------------------
// ASTPrinter
// ---------------------------------------------------------------------------

/// A pretty-printer that walks the AST and writes a coloured, indented tree
/// representation to an arbitrary [`Write`] sink.
///
/// Each node is printed as a labelled bullet point; child nodes are indented by
/// [`INDENT_SIZE`] spaces. ANSI colour codes from the `color` module are used to
/// distinguish node kinds, identifiers, types, and literals.
///
/// # Example
///
/// ```ignore
/// let mut out = std::io::stdout();
/// let mut printer = ASTPrinter {
///     indent: 0,
///     out: &mut out,
///     string_pool: &pool,
/// };
/// printer.visit_stmt(&stmt)?;
/// ```
pub struct ASTPrinter<'a, W: Write> {
    /// Current indentation depth in spaces.
    pub indent: usize,
    /// Output sink where the formatted tree is written.
    pub out: &'a mut W,
    /// String pool used to resolve interned identifiers back to their source text.
    pub string_pool: &'a StringPool,
    /// Colors
    pub color: Colors,
}

impl<'a, W: Write> ASTPrinter<'a, W> {
    /// Increases the indentation level, executes `f`, then restores the original
    /// level — even if `f` returns an error.
    fn indented<F>(&mut self, f: F) -> io::Result<()>
    where
        F: FnOnce(&mut Self) -> io::Result<()>,
    {
        self.indent += INDENT_SIZE;
        let result = f(self);
        self.indent -= INDENT_SIZE;
        result
    }

    /// Writes the bullet-style indent prefix (`  - `) for the current depth.
    ///
    /// Does nothing when `self.indent` is zero (top-level nodes).
    fn write_indent(&mut self) -> io::Result<()> {
        if self.indent > 0 {
            write_spaces(self.out, self.indent - INDENT_SIZE)?;
            self.out.write_all(b"  - ")?;
        }
        Ok(())
    }

    /// Writes plain leading spaces for the current indent depth (no bullet).
    fn write_indent_sub(&mut self) -> io::Result<()> {
        write_spaces(self.out, self.indent)
    }

    /// Writes a line using either the bullet or plain indent style, determined by
    /// `kind`. Appends a newline at the end.
    fn write_line(&mut self, kind: LineKind, args: Arguments) -> io::Result<()> {
        match kind {
            LineKind::Normal => self.line(args),
            LineKind::Sub => self.line_sub(args),
        }
    }

    /// Writes a bullet-indented line followed by a newline.
    fn line(&mut self, args: Arguments) -> io::Result<()> {
        self.write_indent()?;
        self.out.write_fmt(args)?;
        self.out.write_all(b"\n")
    }

    /// Writes a plain-indented line followed by a newline.
    fn line_sub(&mut self, args: Arguments) -> io::Result<()> {
        self.write_indent_sub()?;
        self.out.write_fmt(args)?;
        self.out.write_all(b"\n")
    }

    /// Writes a coloured section label as a bullet-indented line (e.g. `label:`).
    fn label(&mut self, label: &str) -> io::Result<()> {
        let (subtext, reset) = (self.color.subtext, self.color.reset);
        self.write_line(
            LineKind::Normal,
            format_args!("{}{}{}:", subtext, label, reset),
        )
    }

    /// Writes a coloured section label as a plain-indented line (e.g. `label:`).
    fn label_sub(&mut self, label: &str) -> io::Result<()> {
        let (subtext, reset) = (self.color.subtext, self.color.reset);
        self.write_line(
            LineKind::Sub,
            format_args!("{}{}{}:", subtext, label, reset),
        )
    }

    /// Writes an `name: '<value>'` line at the bullet indent level.
    fn ident_line(&mut self, name: &str) -> io::Result<()> {
        let (subtext, reset) = (self.color.subtext, self.color.reset);
        self.line(format_args!("{}name{}: '{}'", subtext, reset, name))
    }

    /// Resolves a [`StringId`] to its source text using the printer's string pool.
    ///
    /// Returns `"<unknown>"` if the id is not present in the pool.
    fn get_name(&self, id: StringId) -> &str {
        self.string_pool.get(id).unwrap_or("<unknown>")
    }
}

impl<'a, W: Write> ASTVisitor for ASTPrinter<'a, W> {
    type Error = io::Error;

    fn visit_stmt(&mut self, stmt: &ASTStmt) -> Result<(), Self::Error> {
        self.label("statement")?;
        self.indented(|s| walk_stmt(s, stmt))?;
        writeln!(self.out)
    }

    fn visit_expr(&mut self, expr: &ASTExpr) -> Result<(), Self::Error> {
        self.label("expression")?;
        self.indented(|s| walk_expr(s, expr))
    }

    fn publicity_str(&self, vis: &Publicity) -> &'static str {
        if *vis == Publicity::Public {
            "0x1 (true)"
        } else {
            "0x0 (false)"
        }
    }

    fn mutablility_str(&self, mutbl: &Mutability) -> &'static str {
        if *mutbl == Mutability::Mutable {
            "0x1 (true)"
        } else {
            "0x0 (false)"
        }
    }

    fn visit_generics(
        &mut self,
        generics: &SmallVec<[ASTGenericParam; 2]>,
    ) -> Result<(), Self::Error> {
        if generics.is_empty() {
            return Ok(());
        }
        self.label_sub("generics")?;
        let ids: Vec<_> = generics.iter().map(|g| g.name.id).collect();
        self.indented(|s| {
            for (i, generic) in generics.iter().enumerate() {
                let name = s.get_name(ids[i]).to_owned();
                match &generic.default {
                    Some(default) => {
                        s.line(format_args!(
                            "{} (def){}",
                            name,
                            format_type(default, &s.color, s.string_pool)
                        ))?;
                    }
                    None => s.line(format_args!("{}", name))?,
                }
            }
            Ok(())
        })
    }

    fn visit_binary_expr(&mut self, expr: &ASTBinaryExpr) -> Result<(), Self::Error> {
        let (subtext, blue, reset) = (self.color.subtext, self.color.blue, self.color.reset);
        self.label("binary")?;
        self.indented(|s| {
            s.line(format_args!(
                "{}operator{}: {}{}{}",
                subtext, reset, blue, expr.operator.kind, reset
            ))?;
            s.label("left")?;
            s.indented(|s| s.visit_expr(&expr.left))?;
            s.label("right")?;
            s.indented(|s| s.visit_expr(&expr.right))
        })
    }

    fn visit_paren_expr(&mut self, expr: &ASTParenExpr) -> Result<(), Self::Error> {
        self.label("parenthesized")?;
        self.indented(|s| s.visit_expr(&expr.expr))
    }

    fn visit_assignment_expr(&mut self, expr: &ASTAssignmentExpr) -> Result<(), Self::Error> {
        let (subtext, blue, reset) = (self.color.subtext, self.color.blue, self.color.reset);
        self.label("assignment")?;
        self.indented(|s| {
            s.line(format_args!("{}to{}:", subtext, reset))?;
            s.visit_expr(&expr.target)?;
            s.line(format_args!(
                "{}operator{}: {}{}{}",
                subtext, reset, blue, expr.op, reset
            ))?;
            s.visit_expr(&expr.value)
        })
    }

    fn visit_cast_expr(&mut self, expr: &ASTCastExpr) -> Result<(), Self::Error> {
        self.label("cast")?;
        self.indented(|s| {
            s.visit_type(&expr.target)?;
            s.visit_expr(&expr.expr)
        })
    }

    fn visit_var_dec(&mut self, expr: &ASTVarDecExpr) -> Result<(), Self::Error> {
        let (subtext, red, reset) = (self.color.subtext, self.color.red, self.color.reset);
        let name = self.get_name(expr.ident.id).to_owned();
        self.label("var")?;
        self.indented(|s| {
            s.ident_line(&name)?;
            let mutable = s.mutablility_str(&expr.mutable).to_owned();
            s.line_sub(format_args!(
                "{}mut{}: {}{}{}",
                subtext, reset, red, mutable, reset
            ))?;
            if let Some(ty) = &expr.ty {
                s.visit_type(ty)?;
            }
            s.visit_expr(&expr.initializer)
        })
    }

    fn visit_const_dec(&mut self, expr: &ASTConstDecExpr) -> Result<(), Self::Error> {
        let (subtext, red, reset) = (self.color.subtext, self.color.red, self.color.reset);
        let name = self.get_name(expr.ident.id).to_owned();
        self.label("const")?;
        self.indented(|s| {
            s.ident_line(&name)?;
            let public = s.publicity_str(&expr.public);
            s.line(format_args!(
                "{}pub{}: {}{}{}",
                subtext, reset, red, public, reset
            ))?;
            if let Some(ty) = &expr.ty {
                s.visit_type(ty)?;
            }
            s.visit_expr(&expr.initializer)
        })
    }

    fn visit_struct_fields(
        &mut self,
        fields: &SmallVec<[ASTStructField; 4]>,
    ) -> Result<(), Self::Error> {
        let (subtext, red, reset) = (self.color.subtext, self.color.red, self.color.reset);
        for field in fields.iter() {
            let field_name = self.get_name(field.ident.id).to_owned();
            self.ident_line(&field_name)?;
            let public = self.publicity_str(&field.public);
            self.line_sub(format_args!(
                "{}pub{}: {}{}{}",
                subtext, reset, red, public, reset
            ))?;
            self.visit_type(&field.ty)?;
        }
        Ok(())
    }

    fn visit_tuple_struct_fields(
        &mut self,
        fields: &SmallVec<[ASTTupleStructField; 4]>,
    ) -> Result<(), Self::Error> {
        let (subtext, red, reset) = (self.color.subtext, self.color.red, self.color.reset);
        for field in fields.iter() {
            let public = self.publicity_str(&field.public);
            self.line(format_args!(
                "{}pub{}: {}{}{}",
                subtext, reset, red, public, reset
            ))?;
            self.visit_type(&field.ty)?;
        }
        Ok(())
    }

    fn visit_struct_dec(&mut self, expr: &ASTStructDecExpr) -> Result<(), Self::Error> {
        let (subtext, red, reset) = (self.color.subtext, self.color.red, self.color.reset);
        let name = self.get_name(expr.ident.id).to_owned();
        self.label("labeled struct")?;
        self.indented(|s| {
            s.ident_line(&name)?;
            let public = s.publicity_str(&expr.public);
            s.line_sub(format_args!(
                "{}pub{}: {}{}{}",
                subtext, reset, red, public, reset
            ))?;
            s.visit_generics(&expr.generics)?;
            s.label_sub("fields")?;
            s.indented(|s| s.visit_struct_fields(&expr.fields))
        })
    }

    fn visit_tuple_struct_dec(&mut self, expr: &ASTTupleStructDecExpr) -> Result<(), Self::Error> {
        let (subtext, red, reset) = (self.color.subtext, self.color.red, self.color.reset);
        let name = self.get_name(expr.ident.id).to_owned();
        self.label("tuple struct")?;
        self.indented(|s| {
            s.ident_line(&name)?;
            let public = s.publicity_str(&expr.public);
            s.line(format_args!(
                "{}pub{}: {}{}{}",
                subtext, reset, red, public, reset
            ))?;
            s.visit_generics(&expr.generics)?;
            s.label_sub("fields")?;
            s.indented(|s| s.visit_tuple_struct_fields(&expr.fields))
        })
    }

    fn visit_unit_struct_dec(&mut self, expr: &ASTUnitStructDecExpr) -> Result<(), Self::Error> {
        let (subtext, red, reset) = (self.color.subtext, self.color.red, self.color.reset);
        let name = self.get_name(expr.ident.id).to_owned();
        self.label("unit struct")?;
        self.indented(|s| {
            s.ident_line(&name)?;
            let public = s.publicity_str(&expr.public);
            s.line(format_args!(
                "{}pub{}: {}{}{}",
                subtext, reset, red, public, reset
            ))
        })
    }

    fn visit_enum_dec(&mut self, expr: &ASTEnumDecExpr) -> Result<(), Self::Error> {
        let (subtext, red, reset) = (self.color.subtext, self.color.red, self.color.reset);
        let name = self.get_name(expr.ident.id).to_owned();
        self.label("enum")?;
        self.indented(|s| {
            s.ident_line(&name)?;
            let public = s.publicity_str(&expr.public);
            s.line_sub(format_args!(
                "{}pub{}: {}{}{}",
                subtext, reset, red, public, reset
            ))?;
            s.visit_generics(&expr.generics)?;
            s.label_sub("fields")?;
            s.visit_enum_variants(&expr.variants)
        })
    }

    fn visit_enum_variants(
        &mut self,
        variants: &SmallVec<[ASTEnumVariant; 4]>,
    ) -> Result<(), Self::Error> {
        if variants.is_empty() {
            return Ok(());
        }
        let ids: Vec<StringId> = variants.iter().map(|v| v.ident.id).collect();
        let (subtext, blue, reset) = (self.color.subtext, self.color.blue, self.color.reset);
        self.label_sub("variants")?;
        self.indented(|s| {
            for (i, variant) in variants.iter().enumerate() {
                let name = s.get_name(ids[i]).to_owned();
                s.ident_line(&name)?;
                match &variant.kind {
                    ASTEnumVariantKind::Unit => {
                        s.line_sub(format_args!(
                            "{}kind{}: {}unit{}",
                            subtext, reset, blue, reset
                        ))?;
                    }
                    ASTEnumVariantKind::Tuple(fields) => {
                        s.line_sub(format_args!(
                            "{}kind{}: {}tupled{}",
                            subtext, reset, blue, reset
                        ))?;
                        s.label_sub("fields")?;
                        s.indented(|s| s.visit_tuple_struct_fields(fields))?;
                    }
                    ASTEnumVariantKind::Struct(fields) => {
                        s.line_sub(format_args!(
                            "{}kind{}: {}labeled{}",
                            subtext, reset, blue, reset
                        ))?;
                        s.label_sub("fields")?;
                        s.indented(|s| s.visit_struct_fields(fields))?;
                    }
                }
            }
            Ok(())
        })
    }

    fn visit_type_alias(&mut self, expr: &ASTTypeAliasDecExpr) -> Result<(), Self::Error> {
        let (subtext, red, reset) = (self.color.subtext, self.color.red, self.color.reset);
        let name = self.get_name(expr.ident.id).to_owned();
        self.label("type alias")?;
        self.indented(|s| {
            s.ident_line(&name)?;
            let public = s.publicity_str(&expr.public);
            s.line(format_args!(
                "{}pub{}: {}{}{}",
                subtext, reset, red, public, reset
            ))?;
            s.visit_generics(&expr.generics)?;
            s.visit_type(&expr.ty)
        })
    }

    fn visit_func_params(
        &mut self,
        params: &SmallVec<[ASTFuncParam; 4]>,
    ) -> Result<(), Self::Error> {
        let (subtext, red, reset) = (self.color.subtext, self.color.red, self.color.reset);
        self.label_sub("params")?;
        self.indented(|s| {
            for param in params.iter() {
                match param {
                    ASTFuncParam::Receiver { mutable, span: _ } => {
                        s.ident_line("inst")?;
                        let mutable = s.mutablility_str(mutable);
                        s.line_sub(format_args!(
                            "{}mut{}: {}{}{}",
                            subtext, reset, red, mutable, reset
                        ))?;
                    }
                    ASTFuncParam::Named {
                        ident,
                        mutable,
                        ty,
                        span: _,
                    } => {
                        let param_name = s.get_name(ident.id).to_owned();
                        s.ident_line(&param_name)?;
                        s.visit_type(ty)?;
                        let mutable = s.mutablility_str(mutable);
                        s.line_sub(format_args!(
                            "{}mut{}: {}{}{}",
                            subtext, reset, red, mutable, reset
                        ))?;
                    }
                }
            }
            Ok(())
        })
    }

    fn visit_func_dec(&mut self, expr: &ASTFuncDecExpr) -> Result<(), Self::Error> {
        let (subtext, red, reset) = (self.color.subtext, self.color.red, self.color.reset);
        let name = self.get_name(expr.ident.id).to_owned();
        self.label("func")?;
        self.indented(|s| {
            s.ident_line(&name)?;
            let public = s.publicity_str(&expr.public);
            s.line_sub(format_args!(
                "{}pub{}: {}{}{}",
                subtext, reset, red, public, reset
            ))?;
            s.visit_generics(&expr.generics)?;
            s.visit_func_params(&expr.params)?;
            if let Some(ty) = &expr.return_ty {
                s.visit_type(ty)?;
            }
            s.visit_func_body(&expr.body)
        })
    }

    fn visit_func_body(&mut self, expr: &ASTBlockExpr) -> Result<(), Self::Error> {
        self.label_sub("body")?;
        self.indented(|s| {
            for stmt in expr.stmts.iter() {
                s.visit_stmt(stmt)?;
            }
            if let Some(expr) = &expr.tail {
                s.label("tail")?;
                s.indented(|s| s.visit_expr(expr))?;
            }
            Ok(())
        })
    }

    fn visit_block_expr(&mut self, block: &ASTBlockExpr) -> Result<(), Self::Error> {
        self.label("block")?;
        self.indented(|s| {
            for stmt in block.stmts.iter() {
                s.visit_stmt(stmt)?;
            }
            if let Some(expr) = &block.tail {
                s.label("tail")?;
                s.indented(|s| s.visit_expr(expr))?;
            }
            Ok(())
        })
    }

    fn visit_field_access(&mut self, expr: &ASTFieldAccessExpr) -> Result<(), Self::Error> {
        let (subtext, lavendar, reset) =
            (self.color.subtext, self.color.lavender, self.color.reset);
        self.label("field access")?;
        self.indented(|s| {
            let name = s.get_name(expr.field.id).to_owned();
            s.line(format_args!(
                "{}field{}: {}{}{}",
                subtext, reset, lavendar, name, reset
            ))?;
            s.visit_expr(expr.expr.as_ref())
        })
    }

    fn visit_method_call(&mut self, expr: &ASTMethodCallExpr) -> Result<(), Self::Error> {
        let (subtext, lavendar, reset) =
            (self.color.subtext, self.color.lavender, self.color.reset);
        self.label("method call")?;
        self.indented(|s| {
            let name = s.get_name(expr.method.id).to_owned();
            s.line(format_args!(
                "{}method{}: {}{}{}",
                subtext, reset, lavendar, name, reset
            ))?;
            s.label("receiver")?;
            s.indented(|s| s.visit_expr(expr.expr.as_ref()))?;
            if !expr.args.is_empty() {
                s.label("args")?;
                s.indented(|s| {
                    for arg in &expr.args {
                        s.visit_expr(arg)?;
                    }
                    Ok(())
                })?;
            }
            Ok(())
        })
    }

    fn visit_call(&mut self, expr: &ASTCallExpr) -> Result<(), Self::Error> {
        self.label("call")?;
        self.indented(|s| {
            s.label("callee")?;
            s.indented(|s| s.visit_expr(expr.expr.as_ref()))?;
            if !expr.args.is_empty() {
                s.label("args")?;
                s.indented(|s| {
                    for arg in &expr.args {
                        s.visit_expr(arg)?;
                    }
                    Ok(())
                })?;
            }
            Ok(())
        })
    }

    fn visit_index(&mut self, expr: &ASTIndexExpr) -> Result<(), Self::Error> {
        self.label("index")?;
        self.indented(|s| {
            s.label("expr")?;
            s.indented(|s| s.visit_expr(expr.expr.as_ref()))?;
            s.label("idx")?;
            s.indented(|s| s.visit_expr(expr.index.as_ref()))
        })
    }

    fn visit_path_segment(&mut self, expr: &ASTPathSegmentExpr) -> Result<(), Self::Error> {
        let (subtext, lavendar, reset) =
            (self.color.subtext, self.color.lavender, self.color.reset);
        self.label("path segment")?;
        self.indented(|s| {
            s.visit_expr(expr.expr.as_ref())?;
            let name = s.get_name(expr.segment.id).to_owned();
            s.line(format_args!(
                "{}segment{}: {}{}{}",
                subtext, reset, lavendar, name, reset
            ))?;
            Ok(())
        })
    }

    fn visit_macro_call(&mut self, expr: &ASTMacroCallExpr) -> Result<(), Self::Error> {
        let (lavendar, blue, reset) = (self.color.lavender, self.color.blue, self.color.reset);
        self.label("macro call")?;
        self.indented(|s| {
            s.line(format_args!(
                "{}name{}: {}{}!{}",
                blue, reset, lavendar, &expr.name, reset
            ))?;
            if !expr.args.is_empty() {
                s.label("args")?;
                s.indented(|s| {
                    for arg in &expr.args {
                        s.visit_expr(arg)?;
                    }
                    Ok(())
                })?;
            }
            Ok(())
        })
    }

    fn visit_tuple(&mut self, expr: &ASTTupleExpr) -> Result<(), Self::Error> {
        self.label("tuple")?;
        self.indented(|s| {
            for elem in &expr.elems {
                s.visit_expr(elem)?;
            }
            Ok(())
        })
    }

    fn visit_unit(&mut self) -> Result<(), Self::Error> {
        let (blue, reset) = (self.color.blue, self.color.reset);
        self.line_sub(format_args!("{}unit{}", blue, reset))
    }

    fn visit_unary_expr(&mut self, expr: &ASTUnaryExpr) -> Result<(), Self::Error> {
        self.label("unary")?;
        let (subtext, blue, reset) = (self.color.subtext, self.color.blue, self.color.reset);
        self.indented(|s| {
            s.line(format_args!(
                "{}operator{}: {}{}{}",
                subtext, reset, blue, expr.op.kind, reset
            ))?;
            s.visit_expr(&expr.expr)
        })
    }

    fn visit_type(&mut self, ty: &ASTType) -> Result<(), Self::Error> {
        let (subtext, reset) = (self.color.subtext, self.color.reset);
        let s = format_type(ty, &self.color, self.string_pool);
        self.line_sub(format_args!("{}type{}: {}", subtext, reset, s))
    }

    fn visit_error(&mut self) -> Result<(), Self::Error> {
        let (bold, red, reset) = (self.color.bold, self.color.red, self.color.reset);
        self.line(format_args!("{}{}ERROR{}", bold, red, reset))
    }

    fn visit_valued(&mut self, kind: &ASTExprKind) -> Result<(), Self::Error> {
        let (bl, pc, gr, rd, lv, rs) = (
            self.color.blue,
            self.color.peach,
            self.color.green,
            self.color.red,
            self.color.lavender,
            self.color.reset,
        );
        match kind {
            ASTExprKind::Integer(v, _) => {
                self.line_sub(format_args!("{}int{}({}{}{})", bl, rs, pc, v, rs))
            }
            ASTExprKind::Float(v, _) => {
                self.line_sub(format_args!("{}float{}({}{}{})", bl, rs, pc, v, rs))
            }
            ASTExprKind::Byte(v) => {
                self.line_sub(format_args!("{}byte{}({}{:02X}{})", bl, rs, pc, v, rs))
            }
            ASTExprKind::Char(c) => {
                self.write_indent_sub()?;
                write!(self.out, "{}char{}({}", bl, rs, gr)?;
                escape_char(self.out, *c)?;
                writeln!(self.out, "{})", rs)
            }
            ASTExprKind::String(s) => {
                self.write_indent_sub()?;
                write!(self.out, "{}string{}({}", bl, rs, gr)?;
                escape_string(self.out, s)?;
                writeln!(self.out, "{})", rs)
            }
            ASTExprKind::ByteString(b) => {
                self.write_indent_sub()?;
                write!(self.out, "{}byte string{}[{}", bl, rs, pc)?;
                for (i, byte) in b.iter().enumerate() {
                    if i > 0 {
                        write!(self.out, ", ")?;
                    }
                    write!(self.out, "{:02X}", byte)?;
                }
                writeln!(self.out, "{}]", rs)
            }
            ASTExprKind::Bool(v) => {
                self.line_sub(format_args!("{}bool{}({}{}{})", bl, rs, rd, v, rs))
            }
            ASTExprKind::Variable(n) => {
                self.line_sub(format_args!("{}var{}({}{}{})", bl, rs, lv, n, rs))
            }
            _ => unreachable!(),
        }
    }
}

use std::fmt::Arguments;
use std::io::{self, Write};

use smallvec::SmallVec;

use crate::ast::strings::{StringId, StringPool};
use crate::ast::token::TokenKind;
use crate::ast::{
    ASTAssignmentExpr, ASTBinaryExpr, ASTBlockExpr, ASTCallExpr, ASTCastExpr, ASTConstDecExpr,
    ASTEnumDecExpr, ASTEnumVariant, ASTEnumVariantKind, ASTExpr, ASTExprKind, ASTFieldAccessExpr,
    ASTGenericParam, ASTIndexExpr, ASTMacroCallExpr, ASTMethodCallExpr, ASTParenExpr,
    ASTPathSegmentExpr, ASTStmt, ASTStmtKind, ASTStructDecExpr, ASTStructField, ASTTupleExpr,
    ASTTupleStructDecExpr, ASTTupleStructField, ASTType, ASTTypeAliasDecExpr, ASTUnaryExpr,
    ASTUnitStructDecExpr, ASTVarDecExpr, Mutability, Publicity,
};
use crate::color::{
    Color, BLUE_COLOR, GREEN_COLOR, LAVENDAR_COLOR, PEACH_COLOR, RED_COLOR, SUBTEXT_COLOR,
    YELLOW_COLOR,
};
use crate::debug;

pub fn walk_stmt<V: ASTVisitor + ?Sized>(v: &mut V, stmt: &ASTStmt) -> Result<(), V::Error> {
    match &stmt.kind {
        ASTStmtKind::Expr(expr) => v.visit_expr(expr),
        ASTStmtKind::Return(expr) => v.visit_expr(expr),
        ASTStmtKind::VarDec(expr) => v.visit_var_dec(expr),
        ASTStmtKind::ConstDec(expr) => v.visit_const_dec(expr),
        ASTStmtKind::StructDec(expr) => v.visit_struct_dec(expr),
        ASTStmtKind::TupleStructDec(expr) => v.visit_tuple_struct_dec(expr),
        ASTStmtKind::UnitStructDec(expr) => v.visit_unit_struct_dec(expr),
        ASTStmtKind::EnumDec(expr) => v.visit_enum_dec(expr),
        ASTStmtKind::TypeAliasDec(expr) => v.visit_type_alias(expr),
    }
}

pub fn walk_expr<V: ASTVisitor + ?Sized>(v: &mut V, expr: &ASTExpr) -> Result<(), V::Error> {
    match &expr.kind {
        ASTExprKind::Integer(_)
        | ASTExprKind::Float(_)
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
// Hilfsfunktionen
// ---------------------------------------------------------------------------

const SPACES: [u8; 256] = [b' '; 256];
const INDENT_SIZE: usize = 4;

fn write_spaces(out: &mut impl Write, count: usize) -> io::Result<()> {
    let mut remaining = count;
    while remaining > 0 {
        let chunk = remaining.min(SPACES.len());
        out.write_all(&SPACES[..chunk])?;
        remaining -= chunk;
    }
    Ok(())
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
    }
}

fn format_type(ty: &ASTType) -> String {
    match ty {
        ASTType::Path(n) => format!("{}{}{}", YELLOW_COLOR, n, Color::ResetAll),
        ASTType::QualifiedPath(ids) => {
            let mut s = String::new();
            for (i, id) in ids.iter().enumerate() {
                if i > 0 {
                    s.push_str(&format!("{}:{}", SUBTEXT_COLOR, Color::ResetAll));
                }

                if ["pkg", "super"].contains(&id.as_ref()) {
                    s.push_str(&format!("{}{}{}", LAVENDAR_COLOR, id, Color::ResetAll));
                } else {
                    s.push_str(&format!("{}{}{}", YELLOW_COLOR, id, Color::ResetAll));
                }
            }
            s
        }
        ASTType::Ref { mutable, inner } => {
            if *mutable == Mutability::Mutable {
                format!(
                    "{}&mut{} {}",
                    LAVENDAR_COLOR,
                    Color::ResetAll,
                    format_type(inner)
                )
            } else {
                format!(
                    "{}&{}{}",
                    LAVENDAR_COLOR,
                    Color::ResetAll,
                    format_type(inner)
                )
            }
        }
        ASTType::Tuple(elems) => {
            let inner = elems
                .iter()
                .map(format_type)
                .collect::<Vec<_>>()
                .join(&format!("{},{} ", SUBTEXT_COLOR, Color::ResetAll));
            format!(
                "{}({}{}{}){}",
                SUBTEXT_COLOR,
                Color::ResetAll,
                inner,
                SUBTEXT_COLOR,
                Color::ResetAll
            )
        }
        ASTType::Generic { base, args } => {
            let args_s = args
                .iter()
                .map(format_type)
                .collect::<Vec<_>>()
                .join(&format!("{},{} ", SUBTEXT_COLOR, Color::ResetAll));
            format!(
                "{}{}<{}{}{}>{}",
                format_type(base),
                SUBTEXT_COLOR,
                Color::ResetAll,
                args_s,
                SUBTEXT_COLOR,
                Color::ResetAll
            )
        }
        ASTType::Error => "<error>".to_string(),
    }
}

// ---------------------------------------------------------------------------
// Visitor-Trait
// ---------------------------------------------------------------------------

pub trait ASTVisitor {
    type Error;

    fn visit_stmt(&mut self, stmt: &ASTStmt) -> Result<(), Self::Error> {
        walk_stmt(self, stmt)
    }

    fn visit_expr(&mut self, expr: &ASTExpr) -> Result<(), Self::Error> {
        walk_expr(self, expr)
    }

    fn visit_publicity(&mut self, vis: &Publicity) -> Result<(), Self::Error>;
    fn visit_generics(
        &mut self,
        generics: &SmallVec<[ASTGenericParam; 2]>,
    ) -> Result<(), Self::Error>;
    fn visit_var_dec(&mut self, expr: &ASTVarDecExpr) -> Result<(), Self::Error>;
    fn visit_const_dec(&mut self, expr: &ASTConstDecExpr) -> Result<(), Self::Error>;
    fn visit_struct_fields(
        &mut self,
        fields: &SmallVec<[ASTStructField; 4]>,
    ) -> Result<(), Self::Error>;
    fn visit_tuple_struct_fields(
        &mut self,
        fields: &SmallVec<[ASTTupleStructField; 4]>,
    ) -> Result<(), Self::Error>;
    fn visit_struct_dec(&mut self, expr: &ASTStructDecExpr) -> Result<(), Self::Error>;
    fn visit_tuple_struct_dec(&mut self, expr: &ASTTupleStructDecExpr) -> Result<(), Self::Error>;
    fn visit_unit_struct_dec(&mut self, expr: &ASTUnitStructDecExpr) -> Result<(), Self::Error>;
    fn visit_enum_dec(&mut self, expr: &ASTEnumDecExpr) -> Result<(), Self::Error>;
    fn visit_enum_variants(
        &mut self,
        variants: &SmallVec<[ASTEnumVariant; 4]>,
    ) -> Result<(), Self::Error>;
    fn visit_type_alias(&mut self, expr: &ASTTypeAliasDecExpr) -> Result<(), Self::Error>;

    fn visit_binary_expr(&mut self, expr: &ASTBinaryExpr) -> Result<(), Self::Error> {
        self.visit_expr(&expr.left)?;
        self.visit_expr(&expr.right)
    }

    fn visit_paren_expr(&mut self, expr: &ASTParenExpr) -> Result<(), Self::Error>;
    fn visit_assignment_expr(&mut self, expr: &ASTAssignmentExpr) -> Result<(), Self::Error>;
    fn visit_unary_expr(&mut self, expr: &ASTUnaryExpr) -> Result<(), Self::Error>;
    fn visit_cast_expr(&mut self, expr: &ASTCastExpr) -> Result<(), Self::Error>;
    fn visit_block_expr(&mut self, expr: &ASTBlockExpr) -> Result<(), Self::Error>;
    fn visit_field_access(&mut self, expr: &ASTFieldAccessExpr) -> Result<(), Self::Error>;
    fn visit_method_call(&mut self, expr: &ASTMethodCallExpr) -> Result<(), Self::Error>;
    fn visit_call(&mut self, expr: &ASTCallExpr) -> Result<(), Self::Error>;
    fn visit_index(&mut self, expr: &ASTIndexExpr) -> Result<(), Self::Error>;
    fn visit_path_segment(&mut self, expr: &ASTPathSegmentExpr) -> Result<(), Self::Error>;
    fn visit_macro_call(&mut self, expr: &ASTMacroCallExpr) -> Result<(), Self::Error>;
    fn visit_tuple(&mut self, expr: &ASTTupleExpr) -> Result<(), Self::Error>;
    fn visit_unit(&mut self) -> Result<(), Self::Error>;
    fn visit_error(&mut self) -> Result<(), Self::Error>;
    fn visit_type(&mut self, ty: &ASTType) -> Result<(), Self::Error>;
    fn visit_valued(&mut self, kind: &ASTExprKind) -> Result<(), Self::Error>;
}

// ---------------------------------------------------------------------------
// ASTPrinter
// ---------------------------------------------------------------------------

pub struct ASTPrinter<'a, W: Write> {
    pub indent: usize,
    pub out: &'a mut W,
    pub string_pool: &'a StringPool,
}

impl<'a, W: Write> ASTPrinter<'a, W> {
    /// Erhöht den Indent, führt `f` aus, verringert ihn wieder – auch bei Fehler.
    fn indented<F>(&mut self, f: F) -> io::Result<()>
    where
        F: FnOnce(&mut Self) -> io::Result<()>,
    {
        self.indent += INDENT_SIZE;
        let result = f(self);
        self.indent -= INDENT_SIZE;
        result
    }

    fn write_indent(&mut self) -> io::Result<()> {
        if self.indent > 0 {
            write_spaces(self.out, self.indent - INDENT_SIZE)?;
            self.out.write_all(b"  - ")?;
        }
        Ok(())
    }

    fn write_indent_sub(&mut self) -> io::Result<()> {
        write_spaces(self.out, self.indent)
    }

    fn line(&mut self, args: Arguments) -> io::Result<()> {
        self.write_indent()?;
        self.out.write_fmt(args)?;
        self.out.write_all(b"\n")
    }

    fn line_sub(&mut self, args: Arguments) -> io::Result<()> {
        self.write_indent_sub()?;
        self.out.write_fmt(args)?;
        self.out.write_all(b"\n")
    }

    /// Gibt ein farbiges Label aus: `SubtextColor <label> ResetAll :`
    fn label(&mut self, label: &str) -> io::Result<()> {
        self.line(format_args!(
            "{}{}{}:",
            SUBTEXT_COLOR,
            label,
            Color::ResetAll
        ))
    }

    fn label_sub(&mut self, label: &str) -> io::Result<()> {
        self.line_sub(format_args!(
            "{}{}{}:",
            SUBTEXT_COLOR,
            label,
            Color::ResetAll
        ))
    }

    /// Gibt eine Identifier-Zeile aus.
    fn ident_line(&mut self, name: &str) -> io::Result<()> {
        self.line(format_args!(
            "{}ident{}: '{}'",
            SUBTEXT_COLOR,
            Color::ResetAll,
            name
        ))
    }

    fn get_name(&self, id: StringId) -> &str {
        self.string_pool.get(id).unwrap_or("<unknown>")
    }
}

impl<'a, W: Write> ASTVisitor for ASTPrinter<'a, W> {
    type Error = io::Error;

    fn visit_stmt(&mut self, stmt: &ASTStmt) -> Result<(), Self::Error> {
        self.label("statement")?;
        self.indented(|s| walk_stmt(s, stmt))?;
        writeln!(self.out) // vorher: println!() — jetzt korrekt auf self.out
    }

    fn visit_expr(&mut self, expr: &ASTExpr) -> Result<(), Self::Error> {
        self.label_sub("expression")?;
        self.indented(|s| walk_expr(s, expr))
    }

    fn visit_publicity(&mut self, vis: &Publicity) -> Result<(), Self::Error> {
        self.line_sub(format_args!(
            "{}public{}: {}{}{}",
            SUBTEXT_COLOR,
            Color::ResetAll,
            RED_COLOR,
            if *vis == Publicity::Public {
                "0x1 (true)"
            } else {
                "0x0 (false)"
            },
            Color::ResetAll
        ))
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

        // ← NACHHER: in der Closure nur names[i] nutzen
        self.indented(|s| {
            for (i, generic) in generics.iter().enumerate() {
                let name = s.get_name(ids[i]).to_owned();
                match &generic.default {
                    Some(default) => {
                        s.line(format_args!("{}", name))?; // ← names[i]!
                        s.line_sub(format_args!(
                            "{}def{}: {}{}{}",
                            SUBTEXT_COLOR,
                            Color::ResetAll,
                            BLUE_COLOR,
                            format_type(default),
                            Color::ResetAll
                        ))?;
                    }
                    None => s.line(format_args!("{}", name))?, // ← names[i]!
                }
            }
            Ok(())
        })
    }

    fn visit_binary_expr(&mut self, expr: &ASTBinaryExpr) -> Result<(), Self::Error> {
        self.label("binary")?;
        self.indented(|s| {
            s.line(format_args!(
                "{}operator{}: {}{}{}",
                SUBTEXT_COLOR,
                Color::ResetAll,
                BLUE_COLOR,
                expr.operator.kind,
                Color::ResetAll
            ))?;
            s.visit_expr(&expr.left)?;
            s.visit_expr(&expr.right)
        })
    }

    fn visit_paren_expr(&mut self, expr: &ASTParenExpr) -> Result<(), Self::Error> {
        self.label("parenthesized")?;
        self.indented(|s| s.visit_expr(&expr.expr))
    }

    fn visit_assignment_expr(&mut self, expr: &ASTAssignmentExpr) -> Result<(), Self::Error> {
        self.label("assignment")?;
        let name = self.get_name(expr.target.id).to_owned();
        self.indented(|s| {
            s.line(format_args!(
                "{}to{}: {}{}{}",
                SUBTEXT_COLOR,
                Color::ResetAll,
                LAVENDAR_COLOR,
                name,
                Color::ResetAll
            ))?;
            s.line(format_args!(
                "{}operator{}: {}{}{}",
                SUBTEXT_COLOR,
                Color::ResetAll,
                BLUE_COLOR,
                expr.op,
                Color::ResetAll
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
        self.label("var_dec")?;
        let name = self.get_name(expr.ident.id).to_owned();
        self.indented(|s| {
            s.ident_line(&name)?;
            s.line_sub(format_args!(
                "{}mutable{}: {}{}{}",
                SUBTEXT_COLOR,
                Color::ResetAll,
                RED_COLOR,
                if expr.mut_ == Mutability::Mutable {
                    "0x1 (true)"
                } else {
                    "0x0 (false)"
                },
                Color::ResetAll
            ))?;
            if let Some(ty) = &expr.ty {
                s.visit_type(ty)?;
            }
            s.visit_expr(&expr.initializer)
        })
    }

    fn visit_const_dec(&mut self, expr: &ASTConstDecExpr) -> Result<(), Self::Error> {
        debug!("const decl: \n{:#?}", expr);
        self.label("const_dec")?;
        let name = self.get_name(expr.ident.id).to_owned();
        self.indented(|s| {
            s.ident_line(&name)?;
            s.visit_publicity(&expr.pub_)?;
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
        for field in fields.iter() {
            let field_name = self.get_name(field.ident.id).to_owned();
            self.ident_line(&field_name)?;
            self.visit_publicity(&field.pub_)?;
            self.visit_type(&field.ty)?;
        }
        Ok(())
    }

    fn visit_tuple_struct_fields(
        &mut self,
        fields: &SmallVec<[ASTTupleStructField; 4]>,
    ) -> Result<(), Self::Error> {
        for field in fields.iter() {
            // Erstes Feld mit "-" einleiten
            self.line(format_args!(
                "{}public{}: {}{}{}",
                SUBTEXT_COLOR,
                Color::ResetAll,
                RED_COLOR,
                if field.pub_ == Publicity::Public {
                    "0x1 (true)"
                } else {
                    "0x0 (false)"
                },
                Color::ResetAll
            ))?;
            self.visit_type(&field.ty)?;
        }
        Ok(())
    }

    fn visit_struct_dec(&mut self, expr: &ASTStructDecExpr) -> Result<(), Self::Error> {
        debug!("struct decl: \n{:#?}", expr);
        self.label("struct_dec")?;
        let name = self.get_name(expr.ident.id).to_owned();
        self.indented(|s| {
            s.ident_line(&name)?;
            s.visit_publicity(&expr.pub_)?;
            s.visit_generics(&expr.generics)?;
            s.label_sub("fields")?;
            s.indented(|s| s.visit_struct_fields(&expr.fields))
        })
    }

    fn visit_tuple_struct_dec(&mut self, expr: &ASTTupleStructDecExpr) -> Result<(), Self::Error> {
        debug!("tuple struct decl: \n{:#?}", expr);
        self.label("tuple_struct_dec")?;
        let name = self.get_name(expr.ident.id).to_owned();
        self.indented(|s| {
            s.ident_line(&name)?;
            s.visit_publicity(&expr.pub_)?;
            s.visit_generics(&expr.generics)?;
            s.label_sub("fields")?;
            s.indented(|s| s.visit_tuple_struct_fields(&expr.fields))
        })
    }

    fn visit_unit_struct_dec(&mut self, expr: &ASTUnitStructDecExpr) -> Result<(), Self::Error> {
        debug!("unit struct decl: \n{:#?}", expr);
        self.label("unit_struct_dec")?;
        let name = self.get_name(expr.ident.id).to_owned();
        self.indented(|s| {
            s.ident_line(&name)?;
            s.visit_publicity(&expr.pub_)
        })
    }

    fn visit_enum_dec(&mut self, expr: &ASTEnumDecExpr) -> Result<(), Self::Error> {
        debug!("enum decl: \n{:#?}", expr);
        self.label("enum_dec")?;
        let name = self.get_name(expr.ident.id).to_owned();
        self.indented(|s| {
            s.ident_line(&name)?;
            s.visit_publicity(&expr.pub_)?;
            s.visit_generics(&expr.generics)?;
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
        self.label_sub("variants")?;
        self.indented(|s| {
            for (i, variant) in variants.iter().enumerate() {
                let name = s.get_name(ids[i]).to_owned();
                // ident auf aktueller Ebene mit "-"
                s.ident_line(&name)?;
                // Varianteninhalt auf gleicher Ebene (kein extra indented!)
                match &variant.kind {
                    ASTEnumVariantKind::Unit => {
                        s.line_sub(format_args!("{}unit{}", BLUE_COLOR, Color::ResetAll))?;
                    }
                    ASTEnumVariantKind::Tuple(fields) => {
                        s.label_sub("tuple")?;
                        s.indented(|s| s.visit_tuple_struct_fields(fields))?;
                    }
                    ASTEnumVariantKind::Struct(fields) => {
                        s.label_sub("struct")?;
                        s.indented(|s| s.visit_struct_fields(fields))?;
                    }
                }
            }
            Ok(())
        })
    }

    fn visit_type_alias(&mut self, expr: &ASTTypeAliasDecExpr) -> Result<(), Self::Error> {
        debug!("type alias decl: \n{:#?}", expr);
        self.label("type_alias_dec")?;
        let name = self.get_name(expr.ident.id).to_owned();
        self.indented(|s| {
            s.ident_line(&name)?;
            s.visit_publicity(&expr.pub_)?;
            s.visit_generics(&expr.generics)?;
            s.visit_type(&expr.ty)
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
        self.label("field_access")?;
        self.indented(|s| {
            let name = s.get_name(expr.field.id).to_owned();
            s.line(format_args!(
                "{}field{}: {}{}{}",
                SUBTEXT_COLOR,
                Color::ResetAll,
                LAVENDAR_COLOR,
                name,
                Color::ResetAll
            ))?;
            s.visit_expr(expr.expr.as_ref())
        })
    }

    fn visit_method_call(&mut self, expr: &ASTMethodCallExpr) -> Result<(), Self::Error> {
        self.label("method_call")?;
        self.indented(|s| {
            let name = s.get_name(expr.method.id).to_owned();
            s.line(format_args!(
                "{}method{}: {}{}{}",
                SUBTEXT_COLOR,
                Color::ResetAll,
                LAVENDAR_COLOR,
                name,
                Color::ResetAll
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
        self.label("undexed")?;
        self.indented(|s| {
            s.label("expr")?;
            s.indented(|s| s.visit_expr(expr.expr.as_ref()))?;
            s.label("index")?;
            s.indented(|s| s.visit_expr(expr.index.as_ref()))
        })
    }

    fn visit_path_segment(&mut self, expr: &ASTPathSegmentExpr) -> Result<(), Self::Error> {
        self.label("path_segment")?;
        self.indented(|s| {
            s.visit_expr(expr.expr.as_ref())?;
            let name = s.get_name(expr.segment.id).to_owned();
            s.line(format_args!(
                "{}segment{}: {}{}{}",
                SUBTEXT_COLOR,
                Color::ResetAll,
                LAVENDAR_COLOR,
                name,
                Color::ResetAll
            ))?;
            Ok(())
        })
    }

    fn visit_macro_call(&mut self, expr: &ASTMacroCallExpr) -> Result<(), Self::Error> {
        self.label("macro_call")?;
        self.indented(|s| {
            s.line(format_args!(
                "{}name{}: {}{}!{}",
                BLUE_COLOR,
                Color::ResetAll,
                LAVENDAR_COLOR,
                expr.name,
                Color::ResetAll
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
        self.line_sub(format_args!("{}unit{}", BLUE_COLOR, Color::ResetAll))
    }

    fn visit_unary_expr(&mut self, expr: &ASTUnaryExpr) -> Result<(), Self::Error> {
        self.label("unary")?;
        self.indented(|s| {
            s.line(format_args!(
                "{}operator{}: {}{}{}",
                SUBTEXT_COLOR,
                Color::ResetAll,
                BLUE_COLOR,
                expr.op.kind,
                Color::ResetAll
            ))?;
            s.visit_expr(&expr.expr)
        })
    }

    fn visit_type(&mut self, ty: &ASTType) -> Result<(), Self::Error> {
        let s = format_type(ty);
        self.line_sub(format_args!(
            "{}type{}: {}",
            SUBTEXT_COLOR,
            Color::ResetAll,
            s,
        ))
    }

    fn visit_error(&mut self) -> Result<(), Self::Error> {
        self.line(format_args!(
            "{}{}ERROR{}",
            Color::Bold,
            RED_COLOR,
            Color::ResetAll
        ))
    }

    fn visit_valued(&mut self, kind: &ASTExprKind) -> Result<(), Self::Error> {
        match kind {
            ASTExprKind::Integer(v) => self.line_sub(format_args!(
                "{}int{}({}{}{})",
                BLUE_COLOR,
                Color::ResetAll,
                PEACH_COLOR,
                v,
                Color::ResetAll
            )),
            ASTExprKind::Float(v) => self.line_sub(format_args!(
                "{}float{}({}{}{})",
                BLUE_COLOR,
                Color::ResetAll,
                PEACH_COLOR,
                v,
                Color::ResetAll
            )),
            ASTExprKind::Byte(v) => self.line_sub(format_args!(
                "{}byte{}({}{:02X}{})",
                BLUE_COLOR,
                Color::ResetAll,
                PEACH_COLOR,
                v,
                Color::ResetAll
            )),
            ASTExprKind::Char(c) => {
                self.write_indent_sub()?;
                write!(
                    self.out,
                    "{}char{}({}",
                    BLUE_COLOR,
                    Color::ResetAll,
                    GREEN_COLOR
                )?;
                escape_char(self.out, *c)?;
                writeln!(self.out, "{})", Color::ResetAll)
            }
            ASTExprKind::String(s) => {
                self.write_indent_sub()?;
                write!(
                    self.out,
                    "{}string{}({}",
                    BLUE_COLOR,
                    Color::ResetAll,
                    GREEN_COLOR
                )?;
                escape_string(self.out, s)?;
                writeln!(self.out, "{})", Color::ResetAll)
            }
            ASTExprKind::ByteString(b) => {
                self.write_indent_sub()?;
                write!(
                    self.out,
                    "{}byte_string{}[{}",
                    BLUE_COLOR,
                    Color::ResetAll,
                    PEACH_COLOR
                )?;
                for (i, byte) in b.iter().enumerate() {
                    if i > 0 {
                        write!(self.out, ", ")?;
                    }
                    write!(self.out, "{:02X}", byte)?;
                }
                writeln!(self.out, "{}]", Color::ResetAll)
            }
            ASTExprKind::Bool(v) => self.line_sub(format_args!(
                "{}bool{}({}{}{})",
                BLUE_COLOR,
                Color::ResetAll,
                RED_COLOR,
                v,
                Color::ResetAll
            )),
            ASTExprKind::Variable(n) => self.line_sub(format_args!(
                "{}var{}({}{}{})",
                BLUE_COLOR,
                Color::ResetAll,
                LAVENDAR_COLOR,
                n,
                Color::ResetAll
            )),
            _ => unreachable!(),
        }
    }
}

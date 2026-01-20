use smallvec::smallvec;

use crate::Compiler;
use crate::ast::scope::Symbol;
use crate::ast::traits::TraitKind;
use crate::ast::types::{ InferTy, Primitive::*, Ty, TyKind };
use crate::ast::{
    AST, ASTBinaryOperatorKind, ASTExpr, ASTExprKind, ASTStmt, ASTStmtKind, ASTVarDecExpr
};
use crate::diagnostics::{ DiagnosticBuilder, DiagnosticKind, Literal, DiagnosticBag };
use crate::TokenKind;

use super::scope::{NameInterner, ScopeCtx};
use super::token::Span;
use super::traits::TraitCtx;
use super::types::{InferCtx, TyInterner, SymbolInterner};

pub struct TypeCkCtx<'a> {
    pub infer_ctx: &'a mut InferCtx,
    pub ty_interner: &'a mut TyInterner,
    pub trait_ctx: &'a TraitCtx,
    pub scopes: &'a mut ScopeCtx,
    pub name_interner: &'a mut NameInterner,
    pub symbol_interner: &'a mut SymbolInterner,
    pub diagnostics: &'a mut DiagnosticBag,
}

pub struct TypeChecker {}

impl<'a> TypeChecker {
    pub fn new() -> Self {
        Self { }
    }

    fn type_mismatch(&self, ctx: &mut TypeCkCtx, expr: &ASTExpr, given: Ty, expected: Ty) {
        ctx.diagnostics.push(
            DiagnosticBuilder::error(
                DiagnosticKind::TypeMismatch { given, expected },
                expr.span.clone(),
            )
            .build(),
        );
    }

    pub fn check(&self, compiler: &mut Compiler, ast: &AST) {
        let mut ctx = TypeCkCtx {
            infer_ctx: &mut compiler.infer_ctx,
            ty_interner: &mut compiler.ty_interner,
            trait_ctx: &compiler.trait_ctx,
            scopes: &mut compiler.scopes,
            name_interner: &mut compiler.name_interner,
            symbol_interner: &mut compiler.symbol_interner,
            diagnostics: &mut compiler.diagnostics,
        };

        for stmt in &ast.stmts {
            self.check_stmt(&mut ctx, stmt);
        }
    }

    fn check_stmt(&self, ctx: &mut TypeCkCtx, stmt: &ASTStmt) {
        match &stmt.kind {
            ASTStmtKind::Expr(expr) => {
                let ty = self.check_expr(ctx, expr, None);

                if !self.is_valid_expr_stmt(ctx, expr, ty) {
                    ctx.diagnostics.push(
                        DiagnosticBuilder::error(
                            DiagnosticKind::UnusedExpressionResult,
                            expr.span.clone(),
                        )
                        .build(),
                    );
                }
            }

            ASTStmtKind::VarDec(dec) => {
                self.check_var_dec(ctx, dec);
            }

            ASTStmtKind::Return(expr) => {
                self.check_expr(ctx, expr, None);
            }

            ASTStmtKind::StructDec(_) | ASTStmtKind::TupleStructDec(_) => {
                // später
            }
        }
    }

    fn check_expr(&self, ctx: &mut TypeCkCtx, expr: &ASTExpr, expected: Option<InferTy>) -> InferTy {
        let err_ty = InferTy::Known(ctx.ty_interner.intern(TyKind::Error));
        let bool_ty = InferTy::Known(ctx.ty_interner.intern(TyKind::Primitive(Bool)));
        let u8_tid = ctx.ty_interner.intern(TyKind::Primitive(U8));

        match &expr.kind {
            ASTExprKind::Error => err_ty,

            ASTExprKind::Integer(value) => {
                let v = InferTy::Var(ctx.infer_ctx.fresh_int());

                if let Some(expected) = expected {
                    if let InferTy::Known(ty) = expected {
                        self.check_literal_fits_int(ctx, *value as i64, ty, &expr.span);
                    }

                    let _ = ctx.infer_ctx.unify(ctx.ty_interner, v.clone(), expected);
                }

                v
            }

            ASTExprKind::Float(value) => {
                let v = InferTy::Var(ctx.infer_ctx.fresh_float());

                if let Some(expected) = expected {
                    if let InferTy::Known(ty) = expected {
                        self.check_literal_fits_float(ctx, *value, ty, &expr.span);
                    }

                    let _ = ctx.infer_ctx.unify(ctx.ty_interner, v.clone(), expected);
                }

                v
            }

            ASTExprKind::Bool(_) => bool_ty,
            ASTExprKind::Byte(_) => InferTy::Known(u8_tid),
            ASTExprKind::Char(_) => InferTy::Known(ctx.ty_interner.intern(TyKind::Primitive(Char))),
            ASTExprKind::String(_) => InferTy::Known(ctx.ty_interner.intern(TyKind::Primitive(String))),
            ASTExprKind::ByteString(_) => InferTy::Known(ctx.ty_interner.intern(TyKind::Generic {
                base: ctx.symbol_interner.intern("Vec"),
                args: smallvec![u8_tid],
            })),
            ASTExprKind::Parenthesized(expr) => self.check_expr(ctx, &expr.expr, None),
            ASTExprKind::Variable(name) => {
                let sym_id_pre = ctx.name_interner.intern(&name);
                let Some(sym_id) = ctx.scopes.lookup(sym_id_pre) else {
                    ctx.diagnostics.push(
                        DiagnosticBuilder::error(
                            DiagnosticKind::UnknownIdentifier { identifier: name.to_string() },
                            expr.span.clone(),
                        )
                        .build(),
                    );
                    return err_ty;
                };

                InferTy::Known(ctx.scopes.symbol_type(sym_id))
            }
            ASTExprKind::Binary(bin) => {
                let lhs = self.check_expr(ctx, &bin.left, expected.clone());
                let rhs = self.check_expr(ctx, &bin.right, expected);

                use super::ASTBinaryOperatorKind::*;
                if lhs == err_ty || rhs == err_ty {
                    return err_ty;
                }

                match bin.operator.kind {
                    Add | Subtract | Multiply | Divide | Modulus => {
                        let lhs_ty = ctx.infer_ctx.resolve_to_ty(ctx.ty_interner, lhs);
                        let rhs_ty = ctx.infer_ctx.resolve_to_ty(ctx.ty_interner, rhs);

                        if let Some(result_ty) = ctx.trait_ctx.implements(
                            lhs_ty,
                            match bin.operator.kind {
                                Add => TraitKind::Add,
                                Subtract => TraitKind::Sub,
                                Multiply => TraitKind::Mul,
                                Divide => TraitKind::Div,
                                Modulus => TraitKind::Rem,
                                _ => unreachable!(),
                            },
                            &[rhs_ty],
                        ) {
                            InferTy::Known(result_ty)
                        } else {
                            let lhs_kind = ctx.ty_interner.kind(lhs_ty).clone();
                            let rhs_kind = ctx.ty_interner.kind(rhs_ty).clone();

                            ctx.diagnostics.push(
                                DiagnosticBuilder::error(
                                    DiagnosticKind::InvalidBinaryOperator {
                                        op: bin.operator.kind.clone(),
                                        left: lhs_ty,
                                        right: rhs_ty,
                                    },
                                    expr.span.clone(),
                                )
                                .label(
                                    expr.span.clone(),
                                    format!(
                                        "Operator `{}` not defined for types `{}` and `{}`",
                                        bin.operator.kind,
                                        lhs_kind,
                                        rhs_kind
                                    ),
                                )
                                .build(),
                            );
                            err_ty
                        }
                    }

                    Equal | NotEqual | Less | Greater | LessEqual | GreaterEqual => {
                        // if self.trait_ctx.borrow_mut().implements(ty, tr, params)
                        bool_ty
                    }

                    // Assign-BinOps wie +=, -=
                    AddAssign | SubtractAssign | MultiplyAssign | DivideAssign => {
                        let Some(lhs_sym_id) = (match &bin.left.kind {
                            ASTExprKind::Variable(name) => {
                                let sym_id_pre = ctx.name_interner.intern(&name);
                                ctx.scopes.lookup_current(sym_id_pre)
                            }
                            _ => None,
                        }) else {
                            ctx.diagnostics.push(
                                DiagnosticBuilder::error(
                                    DiagnosticKind::InvalidAssignmentTarget,
                                    bin.left.span.clone(),
                                )
                                .build(),
                            );
                            return err_ty;
                        };

                        // self.types.unify(lhs_sym.type_.clone(), rhs);
                        InferTy::Known(ctx.scopes.symbol_type(lhs_sym_id))
                    }

                    _ => err_ty,
                }
            }
            ASTExprKind::Assignment(assign) => {
                let sym_id_pre = ctx.name_interner.intern(&assign.name);
                let Some(sym_id) = ctx.scopes.lookup_current(sym_id_pre) else {
                    ctx.diagnostics.push(
                        DiagnosticBuilder::error(
                            DiagnosticKind::UnknownIdentifier { identifier: assign.name.to_string() },
                            expr.span.clone(),
                        )
                        .build(),
                    );
                    return err_ty;
                };

                let lhs = ctx.scopes.symbol_type(sym_id);

                match assign.op {
                    ASTBinaryOperatorKind::AddAssign
                    | ASTBinaryOperatorKind::SubtractAssign
                    | ASTBinaryOperatorKind::MultiplyAssign
                    | ASTBinaryOperatorKind::DivideAssign => {

                        // EXPECTED ist der Typ von `a`
                        let rhs = self.check_expr(ctx, &assign.value, Some(InferTy::Known(lhs)));

                        let _ = ctx.infer_ctx.unify(ctx.ty_interner, rhs, InferTy::Known(lhs));

                        InferTy::Known(lhs)
                    }

                    ASTBinaryOperatorKind::Assign => {
                        let rhs = self.check_expr(ctx, &assign.value, Some(InferTy::Known(lhs)));
                        let _ = ctx.infer_ctx.unify(ctx.ty_interner, rhs, InferTy::Known(lhs));
                        InferTy::Known(lhs)
                    }

                    _ => unreachable!(),
                }
            }
            ASTExprKind::Unary(unary) => {
                let inner = self.check_expr(ctx, &unary.expr, None);

                use super::ASTUnaryOperatorKind::*;

                match unary.op.kind {
                    Negate => {
                        if let Some(output) = ctx.trait_ctx
                            .implements(ctx.infer_ctx.resolve_to_ty(&mut ctx.ty_interner, inner.clone()), TraitKind::Neg, &[])
                        {
                            InferTy::Known(output) // <- hier wird UInt->Int korrekt
                        } else {
                            ctx.diagnostics.push(
                                DiagnosticBuilder::error(
                                    DiagnosticKind::InvalidUnaryOperator {
                                        op: Negate,
                                        ty: ctx.infer_ctx.resolve_to_ty(&mut ctx.ty_interner, inner),
                                    },
                                    unary.op.token.span.clone(),
                                )
                                .build(),
                            );
                            err_ty
                        }
                    }
                    Not => {
                        if let Some(output) =
                            ctx.trait_ctx
                                .implements(ctx.infer_ctx.resolve_to_ty(&mut ctx.ty_interner, inner.clone()), TraitKind::Not, &[])
                        {
                            InferTy::Known(output)
                        } else {
                            ctx.diagnostics.push(
                                DiagnosticBuilder::error(
                                    DiagnosticKind::InvalidUnaryOperator { op: Not, ty: ctx.infer_ctx.resolve_to_ty(&mut ctx.ty_interner, inner) },
                                    unary.op.token.span.clone(),
                                )
                                .build(),
                            );
                            err_ty
                        }
                    }

                    PreIncrement | PostIncrement | PreDecrement | PostDecrement => {
                        todo!();
                    }

                    Ref => {
                        let inner_ty = ctx.infer_ctx.resolve_to_ty(ctx.ty_interner, inner);
                        let ref_ty = ctx.ty_interner.intern(TyKind::Ref(inner_ty));
                        InferTy::Known(ref_ty)
                    }

                    RefMut => {
                        let inner_ty = ctx.infer_ctx.resolve_to_ty(ctx.ty_interner, inner);
                        let ref_ty = ctx.ty_interner.intern(TyKind::Ref(inner_ty));
                        InferTy::Known(ref_ty)
                    }

                    Deref => {
                        let resolved = ctx.infer_ctx.resolve_to_ty(ctx.ty_interner, inner);

                        let deref = match ctx.ty_interner.kind(resolved) {
                            TyKind::Ref(t) | TyKind::MutRef(t) => Some(*t),
                            _ => None,
                        };

                        let ty = deref.unwrap_or_else(|| {
                            ctx.infer_ctx.resolve_to_ty(ctx.ty_interner, err_ty)
                        });

                        InferTy::Known(ty)
                    },
                }
            }
            ASTExprKind::Cast(cast) => {
                let expr_ty = self.check_expr(ctx, &cast.expr, None);
                let from = ctx.infer_ctx.resolve_to_ty(ctx.ty_interner, expr_ty);
                let to = cast.target.clone();

                match ctx.ty_interner.kind(from.clone()) {
                    // Primitive → Primitive prüfen
                    TyKind::Primitive(_) => {
                        if ctx.trait_ctx.implements(from.clone(), TraitKind::Cast, &[to]).is_some() {
                            InferTy::Known(to)
                        } else {
                            self.type_mismatch(ctx, expr, from, to);
                            err_ty
                        }
                    }

                    _ => {
                        self.type_mismatch(ctx, expr, from, to);
                        err_ty
                    }
                }
            }
            ASTExprKind::Block(block) => {
                ctx.scopes.push();

                for stmt in &block.statements {
                    self.check_stmt(ctx, stmt);
                }

                let result = if let Some(expr) = &block.tail_expr {
                    self.check_expr(ctx, expr, None)
                } else {
                    InferTy::Known(ctx.ty_interner.intern(TyKind::Tuple(smallvec![]))) // unit
                };

                ctx.scopes.pop();
                result
            }
        }
    }

    fn check_var_dec(&self, ctx: &mut TypeCkCtx, dec: &ASTVarDecExpr) {
        let name = match &dec.identifier.kind {
            TokenKind::Identifier(s) => s,
            _ => unreachable!(),
        };

        let expected = dec.type_.map(|t| InferTy::Known(t));

        let init_ty = self.check_expr(ctx, &dec.initializer, expected.clone());

        if let Some(expected) = expected.clone() {
            if ctx.infer_ctx.unify(ctx.ty_interner, init_ty.clone(), expected.clone()).is_err() {
                let init_ty = ctx.infer_ctx.resolve_to_ty(ctx.ty_interner, init_ty.clone());
                let expected = ctx.infer_ctx.resolve_to_ty(ctx.ty_interner, expected.clone());
                self.type_mismatch(
                    ctx,
                    &dec.initializer,
                    init_ty,
                    expected,
                );
            }
        }

        let final_ty = ctx.infer_ctx.resolve_to_ty(ctx.ty_interner, init_ty);

        let _ = ctx.scopes.define(Symbol {
            name: ctx.name_interner.intern(name),
            type_: final_ty,
            mut_: dec.mut_,
            pub_: dec.pub_,
            span: dec.identifier.span.clone(),
        });
    }

    fn is_valid_expr_stmt(&self, ctx: &TypeCkCtx, expr: &ASTExpr, ty: InferTy) -> bool {
        use ASTExprKind::*;

        let ty = match ty {
            InferTy::Known(t) => t,
            _ => return false,
        };

        match &expr.kind {
            // explizite Side-Effects
            Assignment(_) => true,

            // +=, -=, etc (sind bei dir Binary)
            Binary(bin) => {
                 match bin.operator.kind {
                    ASTBinaryOperatorKind::Assign | ASTBinaryOperatorKind::AddAssign | ASTBinaryOperatorKind::SubtractAssign
                    | ASTBinaryOperatorKind::MultiplyAssign | ASTBinaryOperatorKind::DivideAssign => true,
                    _ => false,
                }
            }

            // Funktionsaufrufe (falls vorhanden)
            // Call(_) => true,

            // Block: prüfen, ob er Side-Effects enthält
            Block(block) => {
                block.statements.iter().any(|s| self.stmt_has_side_effect(s))
            }

            // unit-Typ (z. B. `{}` oder `()`)
            _ if *ctx.ty_interner.kind(ty) == TyKind::Tuple(smallvec![]) => true,

            // alles andere → wertlos
            _ => false,
        }
    }

    fn stmt_has_side_effect(&self, stmt: &ASTStmt) -> bool {
        match &stmt.kind {
            ASTStmtKind::Expr(expr) => self.expr_has_side_effect(expr),
            ASTStmtKind::VarDec(_) => true,
            ASTStmtKind::Return(_) => true,
            _ => false,
        }
    }

    fn expr_has_side_effect(&self, expr: &ASTExpr) -> bool {
        use ASTExprKind::*;

        match &expr.kind {
            Assignment(_) => true,

            // Call(_) => true,

            Unary(un) => self.expr_has_side_effect(&un.expr),

            Binary(bin) => {
                self.expr_has_side_effect(&bin.left)
                    || self.expr_has_side_effect(&bin.right)
            }

            Block(block) => block
                .statements
                .iter()
                .any(|s| self.stmt_has_side_effect(s)),

            _ => false,
        }
    }

    fn check_literal_fits_int(&self, ctx: &mut TypeCkCtx, value: i64, ty: Ty, span: &Span) {
        use super::types::Primitive::*;

        let (fits, rty) = match ctx.ty_interner.kind(ty) {
            TyKind::Primitive(I8) => (value >= i8::MIN as i64 && value <= i8::MAX as i64, Literal::Int(value as i128)),
            TyKind::Primitive(I16) => (value >= i16::MIN as i64 && value <= i16::MAX as i64, Literal::Int(value as i128)),
            TyKind::Primitive(I32) => (value >= i32::MIN as i64 && value <= i32::MAX as i64, Literal::Int(value as i128)),
            TyKind::Primitive(I64) => (true, Literal::Int(value as i128)),
            TyKind::Primitive(I128) => (true, Literal::Int(value as i128)),

            TyKind::Primitive(U8) => (value >= 0 && value <= u8::MAX as i64, Literal::UInt(value as u128)),
            TyKind::Primitive(U16) => (value >= 0 && value <= u16::MAX as i64, Literal::UInt(value as u128)),
            TyKind::Primitive(U32) => (value >= 0 && value <= u32::MAX as i64, Literal::UInt(value as u128)),
            TyKind::Primitive(U64) => (value >= 0, Literal::UInt(value as u128)),
            TyKind::Primitive(U128) => (value >= 0, Literal::UInt(value as u128)),

            _ => (false, Literal::Int(value as i128)),
        };

        if !fits {
            ctx.diagnostics.push(
                DiagnosticBuilder::error(
                    DiagnosticKind::TypeOverflow { value: rty, ty },
                    span.clone(),
                )
                .build(),
            );
        }
    }

    fn check_literal_fits_float(&self, ctx: &mut TypeCkCtx, value: f64, ty: Ty, span: &Span) {
        let fits = match ctx.ty_interner.kind(ty) {
            TyKind::Primitive(F32) => value < f32::MIN as f64 || value > f32::MAX as f64,
            TyKind::Primitive(F64) => value < f64::MIN || value > f64::MAX,
            _ => false
        };

        if !fits {
            ctx.diagnostics.push(
                DiagnosticBuilder::error(
                    DiagnosticKind::TypeOverflow { value: Literal::Float(value), ty },
                    span.clone(),
                )
                .build(),
            );
        }
    }
}

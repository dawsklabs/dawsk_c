use smallvec::smallvec;

use crate::ast::scope::Symbol;
use crate::ast::traits::TraitKind;
use crate::ast::{
    ASTBinaryOperatorKind, ASTExpr, ASTExprKind, ASTStmt, ASTStmtKind, ASTVarDecExpr, AST,
};
use crate::reports::{Label, Report, ReportBag, ReportKind};
use crate::Compiler;
use crate::TokenKind;

use super::scope::{NameInterner, ScopeCtx};
use super::traits::TraitCtx;
use super::ASTItem;
use crate::types::inference::{InferCtx, InferTy};
use crate::types::{Primitive::*, SymbolInterner, Ty, TyInterner, TyKind};

pub struct TypeCkCtx<'a> {
    pub infer_ctx: &'a mut InferCtx,
    pub ty_interner: &'a mut TyInterner,
    pub trait_ctx: &'a TraitCtx,
    pub scopes: &'a mut ScopeCtx,
    pub name_interner: &'a mut NameInterner,
    pub symbol_interner: &'a mut SymbolInterner,
    pub reports: &'a mut ReportBag,
}

pub struct TypeChecker {}

impl<'a> TypeChecker {
    pub fn new() -> Self {
        Self {}
    }

    fn is_error(&self, ctx: &TypeCkCtx, ty: &InferTy) -> bool {
        match ty {
            InferTy::Known(t) => *ctx.ty_interner.kind(*t) == TyKind::Error,
            _ => false,
        }
    }

    fn type_mismatch(&self, ctx: &mut TypeCkCtx, expr: &ASTExpr, given: Ty, expected: Ty) {
        ctx.reports.push(
            Report::build(ReportKind::Error, expr.span)
                .with_message("mismatched types")
                .with_label(Label::new(expr.span).with_message(format_args!(
                        "`{}` and `{}` cannot match",
                        ctx.ty_interner
                            .display_with_symbols(given, ctx.symbol_interner),
                        ctx.ty_interner
                            .display_with_symbols(expected, ctx.symbol_interner)
                    )))
                .finish(),
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
            reports: &mut compiler.reports,
        };

        for i in &ast.items {
            self.check_item(&mut ctx, i);
        }
    }

    fn check_item(&self, ctx: &mut TypeCkCtx, item: &ASTItem) {
        match item {
            ASTItem::Stmt(stmt) => self.check_stmt(ctx, stmt),

            ASTItem::Use(_) => {
                // use ist rein Namensauflösung → kein Typchecking
            }

            ASTItem::Mod(_) => {
                // Modul-Scope push/pop
            } // später:
              // ASTItem::Fn(f) => self.check_fn(ctx, f),
              // ASTItem::Struct(s) => self.check_struct(ctx, s),
        }
    }

    fn check_stmt(&self, ctx: &mut TypeCkCtx, stmt: &ASTStmt) {
        match &stmt.kind {
            ASTStmtKind::Expr(expr) => {
                let ty = self.check_expr(ctx, expr, None);

                if !self.is_valid_expr_stmt(ctx, expr, ty) {
                    ctx.reports.push(
                        Report::build(ReportKind::Warning, expr.span)
                            .with_message("unused expression result")
                            .with_label(
                                Label::new(expr.span).with_message("consider: `dec _ = ...;`"),
                            )
                            .finish(),
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

    fn check_expr(
        &self,
        ctx: &mut TypeCkCtx,
        expr: &ASTExpr,
        expected: Option<InferTy>,
    ) -> InferTy {
        let err_ty = InferTy::Known(ctx.ty_interner.intern(TyKind::Error));
        let bool_ty = InferTy::Known(ctx.ty_interner.intern(TyKind::Primitive(Bool)));
        let u8_tid = ctx.ty_interner.intern(TyKind::Primitive(U8));

        match expr.kind.as_ref() {
            ASTExprKind::Error => err_ty,

            ASTExprKind::Integer(value) => {
                let v = InferTy::Var(ctx.infer_ctx.fresh_int());

                if let Some(expected) = expected {
                    if let InferTy::Known(ty) = expected {
                        IntValue { value: *value }.fits(ty, ctx.ty_interner);
                    }

                    let _ = ctx.infer_ctx.unify(ctx.ty_interner, v.clone(), expected);
                }

                v
            }

            ASTExprKind::Float(value) => {
                let v = InferTy::Var(ctx.infer_ctx.fresh_float());

                if let Some(expected) = expected {
                    if let InferTy::Known(ty) = expected {
                        FloatValue { value: *value }.fits(ty, ctx.ty_interner);
                    }

                    let _ = ctx.infer_ctx.unify(ctx.ty_interner, v.clone(), expected);
                }

                v
            }

            ASTExprKind::Bool(_) => bool_ty,
            ASTExprKind::Byte(_) => InferTy::Known(u8_tid),
            ASTExprKind::Char(_) => InferTy::Known(ctx.ty_interner.intern(TyKind::Primitive(Char))),
            ASTExprKind::String(_) => {
                InferTy::Known(ctx.ty_interner.intern(TyKind::Primitive(String)))
            }
            ASTExprKind::ByteString(_) => InferTy::Known(ctx.ty_interner.intern(TyKind::Generic {
                base: ctx.symbol_interner.intern("Vec"),
                args: Box::new([u8_tid]),
            })),
            ASTExprKind::Parenthesized(expr) => self.check_expr(ctx, &expr.expr, None),
            ASTExprKind::Variable(name) => {
                let sym_id_pre = ctx.name_interner.intern(&name);
                let Some(sym_id) = ctx.scopes.lookup_symbol(sym_id_pre) else {
                    ctx.reports.push(
                        Report::build(ReportKind::Error, expr.span)
                            .with_message("unknown identifier")
                            .with_label(Label::new(expr.span).with_message("undeclared variable"))
                            .finish(),
                    );

                    return err_ty;
                };

                InferTy::Known(ctx.scopes.symbol_type(sym_id))
            }
            ASTExprKind::Binary(bin) => {
                let lhs = self.check_expr(ctx, &bin.left, expected.clone());
                let rhs = self.check_expr(ctx, &bin.right, expected);

                use super::ASTBinaryOperatorKind::*;
                if self.is_error(ctx, &lhs) || self.is_error(ctx, &rhs) {
                    return err_ty;
                }

                match bin.operator.kind {
                    Add | Subtract | Multiply | Divide | Remainder => {
                        let lhs_ty = ctx.infer_ctx.resolve_to_ty(ctx.ty_interner, lhs);
                        let rhs_ty = ctx.infer_ctx.resolve_to_ty(ctx.ty_interner, rhs);

                        let trait_kind = match bin.operator.kind {
                            Add => TraitKind::Add,
                            Subtract => TraitKind::Sub,
                            Multiply => TraitKind::Mul,
                            Divide => TraitKind::Div,
                            Remainder => TraitKind::Rem,
                            _ => unreachable!(),
                        };

                        if let Some(result_ty) =
                            ctx.trait_ctx.implements(lhs_ty, trait_kind, &[rhs_ty])
                        {
                            InferTy::Known(result_ty)
                        } else {
                            ctx.reports.push(
                                Report::build(ReportKind::Error, expr.span)
                                    .with_message("invalid binary operator")
                                    .with_label(
                                        Label::new(expr.span)
                                            .with_message(format_args!(
                                                "operator `{}` not appliable",
                                                bin.operator.kind
                                            ))
                                            .with_message(format_args!(
                                                "trait `{:?}` not defined for types `{}` and `{}`",
                                                trait_kind,
                                                ctx.ty_interner.display_with_symbols(
                                                    lhs_ty,
                                                    ctx.symbol_interner
                                                ),
                                                ctx.ty_interner.display_with_symbols(
                                                    rhs_ty,
                                                    ctx.symbol_interner
                                                )
                                            )),
                                    )
                                    .finish(),
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
                        let Some(lhs_sym_id) = (match bin.left.kind.as_ref() {
                            ASTExprKind::Variable(name) => {
                                let sym_id_pre = ctx.name_interner.intern(&name);
                                ctx.scopes.lookup_current(sym_id_pre)
                            }
                            _ => None,
                        }) else {
                            ctx.reports.push(
                                Report::build(ReportKind::Error, expr.span)
                                    .with_message("invalid assignment target")
                                    .with_label(
                                        Label::new(expr.span).with_message("cannot assign to here"),
                                    )
                                    .finish(),
                            );

                            return err_ty;
                        };

                        // check mutability
                        if !ctx.scopes.is_mutable(lhs_sym_id) {
                            ctx.reports.push(
                                Report::build(ReportKind::Error, expr.span)
                                    .with_message("assign to immutable variable")
                                    .with_label(Label::new(expr.span).with_message("immutable"))
                                    .finish(),
                            );

                            return err_ty;
                        }

                        // self.types.unify(lhs_sym.type_.clone(), rhs);
                        InferTy::Known(ctx.scopes.symbol_type(lhs_sym_id))
                    }

                    _ => err_ty,
                }
            }
            ASTExprKind::Assignment(assign) => {
                let Some(sym_id) = (match &assign.target.kind {
                    TokenKind::Identifier(name) => {
                        let sym_id_pre = ctx.name_interner.intern(&name);
                        ctx.scopes.lookup_current(sym_id_pre)
                    }
                    _ => None,
                }) else {
                    ctx.reports.push(
                        Report::build(ReportKind::Error, expr.span)
                            .with_message("invalid assignment target")
                            .with_label(Label::new(expr.span).with_message("cannot assign to here"))
                            .finish(),
                    );

                    return err_ty;
                };

                // check mutability
                if !ctx.scopes.is_mutable(sym_id) {
                    ctx.reports.push(
                        Report::build(ReportKind::Error, expr.span)
                            .with_message("assign to immutable variable")
                            .with_label(Label::new(expr.span).with_message("immutable"))
                            .finish(),
                    );

                    return err_ty;
                }

                let lhs = ctx.scopes.symbol_type(sym_id);

                match assign.op {
                    ASTBinaryOperatorKind::AddAssign
                    | ASTBinaryOperatorKind::SubtractAssign
                    | ASTBinaryOperatorKind::MultiplyAssign
                    | ASTBinaryOperatorKind::DivideAssign => {
                        // EXPECTED ist der Typ von `a`
                        let rhs = self.check_expr(ctx, &assign.value, Some(InferTy::Known(lhs)));

                        let _ = ctx
                            .infer_ctx
                            .unify(ctx.ty_interner, rhs, InferTy::Known(lhs));

                        InferTy::Known(lhs)
                    }

                    ASTBinaryOperatorKind::Assign => {
                        let rhs = self.check_expr(ctx, &assign.value, Some(InferTy::Known(lhs)));
                        let _ = ctx
                            .infer_ctx
                            .unify(ctx.ty_interner, rhs, InferTy::Known(lhs));
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
                        let inner_ty = ctx
                            .infer_ctx
                            .resolve_to_ty(&mut ctx.ty_interner, inner.clone());

                        if let Some(output) =
                            ctx.trait_ctx.implements(inner_ty, TraitKind::Neg, &[])
                        {
                            InferTy::Known(output)
                        } else {
                            ctx.reports.push(
                                Report::build(ReportKind::Error, expr.span)
                                    .with_message("invalid unary operator")
                                    .with_label(
                                        Label::new(expr.span)
                                            .with_message(format_args!(
                                                "operator `{}` not appliable",
                                                unary.op.kind
                                            ))
                                            .with_message(format_args!(
                                                "Trait `{:?}` not defined for types `{}`",
                                                TraitKind::Neg,
                                                ctx.ty_interner.display_with_symbols(
                                                    inner_ty,
                                                    ctx.symbol_interner
                                                ),
                                            )),
                                    )
                                    .finish(),
                            );

                            err_ty
                        }
                    }

                    Not => {
                        let inner_ty = ctx
                            .infer_ctx
                            .resolve_to_ty(&mut ctx.ty_interner, inner.clone());

                        if let Some(output) =
                            ctx.trait_ctx.implements(inner_ty, TraitKind::Not, &[])
                        {
                            InferTy::Known(output)
                        } else {
                            ctx.reports.push(
                                Report::build(ReportKind::Error, expr.span)
                                    .with_message("invalid unary operator")
                                    .with_label(
                                        Label::new(expr.span)
                                            .with_message(format_args!(
                                                "operator `{}` not appliable",
                                                unary.op.kind
                                            ))
                                            .with_message(format_args!(
                                                "Trait `{:?}` not defined for types `{}`",
                                                TraitKind::Neg,
                                                ctx.ty_interner.display_with_symbols(
                                                    inner_ty,
                                                    ctx.symbol_interner
                                                ),
                                            )),
                                    )
                                    .finish(),
                            );

                            err_ty
                        }
                    }

                    PreIncrement | PostIncrement | PreDecrement | PostDecrement => {
                        todo!();
                    }

                    Ref => {
                        let inner_ty = ctx.infer_ctx.resolve_to_ty(&mut ctx.ty_interner, inner);
                        let ref_ty = ctx.ty_interner.intern(TyKind::Ref(inner_ty));
                        InferTy::Known(ref_ty)
                    }

                    RefMut => {
                        let inner_ty = ctx.infer_ctx.resolve_to_ty(&mut ctx.ty_interner, inner);
                        let ref_ty = ctx.ty_interner.intern(TyKind::Ref(inner_ty));

                        if *ctx.ty_interner.kind(inner_ty) == TyKind::Error {
                            return InferTy::Known(ctx.ty_interner.intern(TyKind::Error));
                        }

                        InferTy::Known(ref_ty)
                    }

                    Deref => {
                        let resolved = ctx.infer_ctx.resolve_to_ty(&mut ctx.ty_interner, inner);

                        if *ctx.ty_interner.kind(resolved) == TyKind::Error {
                            return InferTy::Known(ctx.ty_interner.intern(TyKind::Error));
                        }

                        let deref = match *ctx.ty_interner.kind(resolved) {
                            TyKind::Ref(t) | TyKind::MutRef(t) => Some(t),
                            _ => {
                                ctx.reports.push(
                                    Report::build(ReportKind::Error, expr.span)
                                        .with_message("invalid dereference")
                                        .with_label(Label::new(expr.span).with_message(
                                            format_args!(
                                                "type `{}` cannot be dereferenced",
                                                ctx.ty_interner.display_with_symbols(
                                                    resolved,
                                                    ctx.symbol_interner
                                                )
                                            ),
                                        ))
                                        .finish(),
                                );

                                None
                            }
                        };

                        let ty = deref.unwrap_or_else(|| {
                            ctx.infer_ctx.resolve_to_ty(&mut ctx.ty_interner, err_ty)
                        });

                        InferTy::Known(ty)
                    }
                }
            }
            ASTExprKind::Cast(cast) => {
                let expr_ty = self.check_expr(ctx, &cast.expr, None);
                let from = ctx.infer_ctx.resolve_to_ty(ctx.ty_interner, expr_ty);
                let to = cast.target.clone();

                match ctx.ty_interner.kind(from.clone()) {
                    // Primitive → Primitive prüfen
                    TyKind::Primitive(_) => {
                        if ctx
                            .trait_ctx
                            .implements(from.clone(), TraitKind::Cast, &[to])
                            .is_some()
                        {
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
                    InferTy::Known(ctx.ty_interner.intern(TyKind::Tuple(smallvec![])))
                    // unit
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

        let init_ty = self.check_expr(ctx, &dec.initializer, None);

        if self.is_error(ctx, &init_ty) {
            return;
        }

        if let Some(expected) = expected.clone() {
            if ctx
                .infer_ctx
                .unify(ctx.ty_interner, init_ty.clone(), expected.clone())
                .is_err()
            {
                let init_ty = ctx
                    .infer_ctx
                    .resolve_to_ty(ctx.ty_interner, init_ty.clone());
                let expected = ctx
                    .infer_ctx
                    .resolve_to_ty(ctx.ty_interner, expected.clone());
                self.type_mismatch(ctx, &dec.initializer, init_ty, expected);
            }
        }

        let final_ty = ctx.infer_ctx.resolve_to_ty(ctx.ty_interner, init_ty);

        let _ = ctx.scopes.define_symbol(Symbol {
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

        match expr.kind.as_ref() {
            // explizite Side-Effects
            Assignment(_) => true,

            // +=, -=, etc (sind bei dir Binary)
            Binary(bin) => match bin.operator.kind {
                ASTBinaryOperatorKind::Assign
                | ASTBinaryOperatorKind::AddAssign
                | ASTBinaryOperatorKind::SubtractAssign
                | ASTBinaryOperatorKind::MultiplyAssign
                | ASTBinaryOperatorKind::DivideAssign => true,
                _ => false,
            },

            // Funktionsaufrufe (falls vorhanden)
            // Call(_) => true,

            // Block: prüfen, ob er Side-Effects enthält
            Block(block) => block
                .statements
                .iter()
                .any(|s| self.stmt_has_side_effect(s)),

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

        match expr.kind.as_ref() {
            Assignment(_) => true,

            // Call(_) => true,
            Unary(un) => self.expr_has_side_effect(&un.expr),

            Binary(bin) => {
                self.expr_has_side_effect(&bin.left) || self.expr_has_side_effect(&bin.right)
            }

            Block(block) => block
                .statements
                .iter()
                .any(|s| self.stmt_has_side_effect(s)),

            _ => false,
        }
    }

    // fn check_literal_fits_int(&self, ctx: &mut TypeCkCtx, value: i64, ty: Ty, span: &Span) {
    //     use crate::types::Primitive::*;

    //     let fits = match ctx.ty_interner.kind(ty) {
    //         TyKind::Primitive(I8) => value >= i8::MIN as i64 && value <= i8::MAX as i64,
    //         TyKind::Primitive(I16) => value >= i16::MIN as i64 && value <= i16::MAX as i64,
    //         TyKind::Primitive(I32) => value >= i32::MIN as i64 && value <= i32::MAX as i64,
    //         TyKind::Primitive(I64) => true,
    //         TyKind::Primitive(I128) => true,

    //         TyKind::Primitive(U8) => value >= 0 && value <= u8::MAX as i64,
    //         TyKind::Primitive(U16) => value >= 0 && value <= u16::MAX as i64,
    //         TyKind::Primitive(U32) => value >= 0 && value <= u32::MAX as i64,
    //         TyKind::Primitive(U64) => value >= 0, // alles <= i64::MAX passt in u64
    //         TyKind::Primitive(U128) => value >= 0,

    //         _ => false,
    //     };

    //     if !fits {
    //         ctx.reports.push(
    //             Report::build(ReportKind::Error, *span)
    //                 .with_message("type overflow")
    //                 .with_label(Label::new(*span).with_message(format_args!(
    //                 "value does not fit type `{}`",
    //                 ctx.ty_interner.display_with_symbols(ty, ctx.symbol_interner)
    //             )))
    //                 .finish(),
    //         );
    //     }
    // }

    // fn check_literal_fits_float(&self, ctx: &mut TypeCkCtx, value: f64, ty: Ty, span: &Span) {
    //     let fits = match ctx.ty_interner.kind(ty) {
    //         TyKind::Primitive(F32) => value >= f32::MIN as f64 && value <= f32::MAX as f64,
    //         TyKind::Primitive(F64) => value >= f64::MIN && value <= f64::MAX,
    //         _ => false,
    //     };

    //     if !fits {
    //         ctx.reports.push(
    //             Report::build(ReportKind::Error, *span)
    //                 .with_message("type overflow")
    //                 .with_label(Label::new(*span).with_message(format_args!(
    //                         "value does not fit type `{}`",
    //                         ctx.ty_interner
    //                             .display_with_symbols(ty, ctx.symbol_interner)
    //                     )))
    //                 .finish(),
    //         );
    //     }
    // }
}

#[derive(Copy, Clone, Debug)]
pub struct IntValue {
    pub value: u128,
}

impl IntValue {
    pub fn fits(&self, ty: Ty, ty_interner: &TyInterner) -> bool {
        use crate::types::Primitive::*;

        match ty_interner.kind(ty) {
            TyKind::Primitive(I8) => self.value <= i8::MAX as u128,
            TyKind::Primitive(I16) => self.value <= i16::MAX as u128,
            TyKind::Primitive(I32) => self.value <= i32::MAX as u128,
            TyKind::Primitive(I64) => self.value <= i64::MAX as u128,
            TyKind::Primitive(I128) => true,

            TyKind::Primitive(U8) => self.value <= u8::MAX as u128,
            TyKind::Primitive(U16) => self.value <= u16::MAX as u128,
            TyKind::Primitive(U32) => self.value <= u32::MAX as u128,
            TyKind::Primitive(U64) => self.value <= u64::MAX as u128,
            TyKind::Primitive(U128) => true,

            _ => false,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct FloatValue {
    pub value: f64,
}

impl FloatValue {
    pub fn fits(&self, ty: Ty, ty_interner: &TyInterner) -> bool {
        use crate::types::Primitive::*;

        match ty_interner.kind(ty) {
            TyKind::Primitive(F32) => {
                self.value >= f32::MIN as f64 && self.value <= f32::MAX as f64
            }
            TyKind::Primitive(F64) => true,
            _ => false,
        }
    }
}

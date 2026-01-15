use crate::ast::scope::{ScopeCtx, Symbol};
use crate::ast::traits::{TraitCtx, TraitKind};
use crate::ast::types::{LiteralType, Primitive::*, TypeCtx, TypeId, TypeKind};
use crate::ast::{
    AST, ASTBinaryOperatorKind, ASTExpr, ASTExprKind, ASTStmt, ASTStmtKind, ASTVarDecExpr
};
use crate::diagnostics::{DiagnosticBagCell, DiagnosticBuilder, DiagnosticKind};
use crate::TokenKind;

pub struct TypeChecker {
    scopes: ScopeCtx,
    types: TypeCtx,
    trait_ctx: TraitCtx,
    diagnostics_bag: DiagnosticBagCell,
    error_ty: TypeId,
    u8_ty: TypeId,
}

impl TypeChecker {
    pub fn new(diagnostics_bag: DiagnosticBagCell, types: TypeCtx, trait_ctx: TraitCtx) -> Self {
        let error_ty = types.intern(TypeKind::Error);
        let u8_ty = types.intern(TypeKind::Primitive(U8));

        Self {
            scopes: ScopeCtx::new(),
            types,
            trait_ctx,
            diagnostics_bag,
            error_ty,
            u8_ty,
        }
    }

    fn type_mismatch(&self, expr: &ASTExpr, given: TypeId, expected: TypeId) {
        self.diagnostics_bag.push(
            DiagnosticBuilder::error(
                DiagnosticKind::TypeMismatch { given, expected },
                expr.span.clone(),
            )
            .build(),
        );
    }

    pub fn check(&mut self, ast: &AST) {
        for stmt in &ast.stmts {
            self.check_stmt(stmt);
        }
    }

    fn check_stmt(&mut self, stmt: &ASTStmt) {
        match &stmt.kind {
            ASTStmtKind::Expr(expr) => {
                let ty = self.check_expr(expr);

                if !self.is_valid_expr_stmt(expr, ty) {
                    self.diagnostics_bag.push(
                        DiagnosticBuilder::error(
                            DiagnosticKind::UnusedExpressionResult,
                            expr.span.clone(),
                        )
                        .build(),
                    );
                }
            }

            ASTStmtKind::VarDec(dec) => {
                self.check_var_dec(dec);
            }

            ASTStmtKind::Return(expr) => {
                self.check_expr(expr);
            }

            ASTStmtKind::StructDec(_) | ASTStmtKind::TupleStructDec(_) => {
                // später
            }
        }
    }

    fn check_expr(&mut self, expr: &ASTExpr) -> TypeId {
        match &expr.kind {
            ASTExprKind::Error => self.error_ty,

            ASTExprKind::Integer(_) => self.types.intern(TypeKind::Literal(LiteralType::UInt)),
            ASTExprKind::Float(_) => self.types.intern(TypeKind::Literal(LiteralType::Float)),
            ASTExprKind::Bool(_) => self.types.intern(TypeKind::Primitive(Bool)),
            ASTExprKind::Byte(_) => self.u8_ty,
            ASTExprKind::Char(_) => self.types.intern(TypeKind::Primitive(Char)),
            ASTExprKind::String(_) => self.types.intern(TypeKind::Primitive(String)),
            ASTExprKind::ByteString(_) => self.types.intern(TypeKind::Generic {
                base: "Vec",
                args: vec![self.u8_ty],
            }),
            ASTExprKind::Parenthesized(expr) => self.check_expr(&expr.expr),
            ASTExprKind::Variable(name) => {
                let Some(sym_id) = self.scopes.lookup(name) else {
                    self.diagnostics_bag.push(
                        DiagnosticBuilder::error(
                            DiagnosticKind::UnknownIdentifier { identifier: name.to_string() },
                            expr.span.clone(),
                        )
                        .build(),
                    );
                    return self.error_ty;
                };

                self.scopes.symbol_type(sym_id)
            }
            ASTExprKind::Binary(bin) => {
                let lhs = self.check_expr(&bin.left);
                let rhs = self.check_expr(&bin.right);

                use super::ASTBinaryOperatorKind::*;

                if lhs == self.error_ty || rhs == self.error_ty {
                    return self.error_ty;
                }

                match bin.operator.kind {
                    Add | Subtract | Multiply | Divide | Modulus => {
                        if let Some(result_ty) = self.trait_ctx.implements(
                            lhs.clone(),
                            match bin.operator.kind {
                                Add => TraitKind::Add,
                                Subtract => TraitKind::Sub,
                                Multiply => TraitKind::Mul,
                                Divide => TraitKind::Div,
                                Modulus => TraitKind::Rem,
                                _ => unreachable!(),
                            },
                            &[rhs.clone()],
                        ) {
                            result_ty
                        } else {
                            self.diagnostics_bag.push(
                                DiagnosticBuilder::error(
                                    DiagnosticKind::InvalidBinaryOperator {
                                        op: bin.operator.kind.clone(),
                                        left: lhs.clone(),
                                        right: rhs.clone(),
                                    },
                                    expr.span.clone(),
                                )
                                .label(
                                    expr.span.clone(),
                                    format!(
                                        "Operator `{}` not defined for types `{}` and `{}`",
                                        bin.operator.kind,
                                        self.types.get(lhs),
                                        self.types.get(rhs)
                                    ),
                                )
                                .build(),
                            );
                            self.error_ty
                        }
                    }

                    Equal | NotEqual | Less | Greater | LessEqual | GreaterEqual => {
                        // if !self.types.unify(lhs.clone(), rhs.clone()) {
                        //     self.type_mismatch(expr, lhs, rhs);
                        // }
                        self.types.intern(TypeKind::Primitive(Bool))
                    }

                    // Assign-BinOps wie +=, -=
                    AddAssign | SubtractAssign | MultiplyAssign | DivideAssign => {
                        let Some(lhs_sym_id) = (match &bin.left.kind {
                            ASTExprKind::Variable(name) => self.scopes.lookup_current(name),
                            _ => None,
                        }) else {
                            self.diagnostics_bag.push(
                                DiagnosticBuilder::error(
                                    DiagnosticKind::InvalidAssignmentTarget,
                                    bin.left.span.clone(),
                                )
                                .build(),
                            );
                            return self.error_ty;
                        };

                        // self.types.unify(lhs_sym.type_.clone(), rhs);
                        self.scopes.symbol_type(lhs_sym_id)
                    }

                    _ => self.error_ty,
                }
            }
            ASTExprKind::Assignment(assign) => {
                let Some(sym_id) = self.scopes.lookup_current(&assign.name) else {
                    self.diagnostics_bag.push(
                        DiagnosticBuilder::error(
                            DiagnosticKind::UnknownIdentifier { identifier: assign.name.to_string() },
                            expr.span.clone(),
                        )
                        .build(),
                    );
                    return self.error_ty;
                };

                match assign.op {
                    ASTBinaryOperatorKind::AddAssign
                    | ASTBinaryOperatorKind::SubtractAssign
                    | ASTBinaryOperatorKind::MultiplyAssign
                    | ASTBinaryOperatorKind::DivideAssign => {
                        let lhs = self.scopes.symbol_type(sym_id);
                        let rhs = self.check_expr(&assign.value);
                        let _ = self.types.unify(lhs, rhs);
                        lhs
                    }

                    ASTBinaryOperatorKind::Assign => {
                        self.scopes.symbol_type(sym_id)
                    }

                    _ => unreachable!(),
                }
            }
            ASTExprKind::Unary(unary) => {
                let inner = self.check_expr(&unary.expr);

                use super::ASTUnaryOperatorKind::*;

                match unary.op.kind {
                    Negate => {
                        if let Some(output) =
                            self.trait_ctx
                                .implements(inner.clone(), TraitKind::Neg, &[])
                        {
                            output // <- hier wird UInt->Int korrekt
                        } else {
                            self.diagnostics_bag.push(
                                DiagnosticBuilder::error(
                                    DiagnosticKind::InvalidUnaryOperator {
                                        op: Negate,
                                        ty: inner.clone(),
                                    },
                                    unary.op.token.span.clone(),
                                )
                                .build(),
                            );
                            self.error_ty
                        }
                    }
                    Not => {
                        if let Some(output) =
                            self.trait_ctx
                                .implements(inner.clone(), TraitKind::Not, &[])
                        {
                            output
                        } else {
                            self.diagnostics_bag.push(
                                DiagnosticBuilder::error(
                                    DiagnosticKind::InvalidUnaryOperator { op: Not, ty: inner },
                                    unary.op.token.span.clone(),
                                )
                                .build(),
                            );
                            self.error_ty
                        }
                    }

                    PreIncrement | PostIncrement | PreDecrement | PostDecrement => {
                        todo!();
                    }

                    Ref => self.types.intern(TypeKind::Ref(inner)),
                    RefMut => self.types.intern(TypeKind::MutRef(inner)),

                    Deref => match self.types.get(inner) {
                        TypeKind::Ref(t) | TypeKind::MutRef(t) => t,
                        _ => self.error_ty,
                    },
                }
            }
            ASTExprKind::Cast(cast) => {
                let from = self.check_expr(&cast.expr);
                let to = cast.target.clone();

                match self.types.get(from) {
                    // Primitive → Primitive prüfen
                    TypeKind::Primitive(_) => {
                        // if !self.is_valid_cast(&self.types.get(from).clone()) {
                        //     self.type_mismatch(expr, from, self.types.intern(to));
                        //     self.error_ty
                        // } else {
                        to
                        // }
                    }

                    _ => {
                        self.type_mismatch(expr, from, to);
                        self.error_ty
                    }
                }
            }
            ASTExprKind::Block(block) => {
                self.scopes.push();

                for stmt in &block.statements {
                    self.check_stmt(stmt);
                }

                let result = if let Some(expr) = &block.tail_expr {
                    self.check_expr(expr)
                } else {
                    self.types.intern(TypeKind::Tuple(vec![])) // unit
                };

                self.scopes.pop();
                result
            }
        }
    }

    fn check_var_dec(&mut self, dec: &ASTVarDecExpr) {
        let name = match &dec.identifier.kind {
            TokenKind::Identifier(s) => s,
            _ => unreachable!(),
        };

        let init_ty = self.check_expr(&dec.initializer);

        let final_ty = if let Some(annotated) = &dec.type_ {
            // if !self.types.unify(init_ty.clone(), annotated.clone()) {
            //     self.type_mismatch(&dec.initializer.clone(), init_ty.clone(), annotated.clone());
            //     TypeKind::Error
            // } else {
            *annotated
            // }
        } else {
            init_ty
        };

        let _ = self.scopes.define(Symbol {
            name: name,
            type_: final_ty,
            mut_: dec.mut_,
            pub_: dec.pub_,
            span: dec.identifier.span.clone(),
        });
    }

    fn is_valid_expr_stmt(&self, expr: &ASTExpr, ty: TypeId) -> bool {
        use ASTExprKind::*;

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
            _ if self.types.types_eq(ty, TypeKind::Tuple(vec![])) => true,

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

}

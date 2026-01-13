use crate::ast::scope::{ScopeCtx, Symbol};
use crate::ast::traits::{TraitCtx, TraitKind};
use crate::ast::types::{LiteralType, Primitive::*, TypeCtx, TypeId, TypeKind};
use crate::ast::{
    ASTBinaryOperatorKind, ASTExpr, ASTExprKind, ASTStmt, ASTStmtKind, ASTVarDecExpr, AST,
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
                self.check_expr(expr);
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
                            ASTExprKind::Variable(name) => self.scopes.lookup(name),
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
                let Some(sym_id) = self.scopes.lookup(&assign.name) else {
                    self.diagnostics_bag.push(
                        DiagnosticBuilder::error(
                            DiagnosticKind::UnknownIdentifier { identifier: assign.name.to_string() },
                            expr.span.clone(),
                        )
                        .build(),
                    );
                    return self.error_ty;
                };

                // let rhs = self.check_expr(&assign.value);

                match assign.op {
                    ASTBinaryOperatorKind::AddAssign
                    | ASTBinaryOperatorKind::SubtractAssign
                    | ASTBinaryOperatorKind::MultiplyAssign
                    | ASTBinaryOperatorKind::DivideAssign => {
                        // let t = self.types.fresh_var();

                        // a + rhs
                        // self.types.unify(sym.clone().type_, t.clone());
                        // self.types.unify(rhs, t);

                        self.scopes.symbol_type(sym_id)
                    }

                    ASTBinaryOperatorKind::Assign => {
                        // self.types.unify(sym.clone().type_, rhs);
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

    // fn is_valid_cast(&self, from: &TypeKind, to: &TypeKind) -> bool {
    //     match (from, to) {
    //         (Primitive(a), Primitive(b)) => match (a, b) {
    //             // Integer <-> Integer (signed/unsigned erlaubt)
    //             // (I8 | I16 | I32 | I64 | U8 | U16 | U32 | U64 | Int,
    //             //     I8 | I16 | I32 | I64 | U8 | U16 | U32 | U64 | Int) => true,

    //             // // Float <-> Float
    //             // (F32 | F64 | Float, F32 | F64 | Float) => true,

    //             // // Integer <-> Float
    //             // (I8 | I16 | I32 | I64 | U8 | U16 | U32 | U64 | Int, F32 | F64 | Float) => true,
    //             // (F32 | F64 | Float, I8 | I16 | I32 | I64 | U8 | U16 | U32 | U64 | Int) => true,

    //             _ => false,
    //         },

    //         // Referenz-Casts explizit verbieten (erstmal)
    //         (Ref(_), _) | (MutRef(_), _) => false,
    //         (_, Ref(_)) | (_, MutRef(_)) => false,

    //         _ => false,
    //     }
    // }

    // fn finish(&self) {
    //     for id in &self.types.all_vars {
    //         if !self.types.subs.contains_key(id) {
    //             self.diagnostics_bag.push(
    //                 DiagnosticBuilder::error(
    //                     DiagnosticKind::UnresolvedTypeVariable { id: *id },
    //                     Span::new(0, 0),
    //                 )
    //                 .build(),
    //             );
    //         }
    //     }
    // }
}

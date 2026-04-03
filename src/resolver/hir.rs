use crate::ast::scope::{Mutability, ScopeCtx, Symbol};
use crate::ast::{ASTExpr, ASTItem, ASTStmt};
use crate::types::Ty;

// ---------- HIR-Strukturen ----------
pub struct HIRExpr {
    pub kind: HIRExprKind,
    pub ty: Ty,
    pub span: crate::source::Span,
}

pub enum HIRExprKind {
    Variable(crate::ast::scope::SymbolId),
    Literal(Literal),
    Binary(
        Box<HIRExpr>,
        crate::ast::ASTBinaryOperatorKind,
        Box<HIRExpr>,
    ),
    Unary(crate::ast::ASTUnaryOperatorKind, Box<HIRExpr>),
    Assignment(crate::ast::scope::SymbolId, Box<HIRExpr>),
    Block(Vec<HIRStmt>, Option<Box<HIRExpr>>),
    Cast(Box<HIRExpr>, Ty),
}

pub struct HIRStmt {
    pub kind: HIRStmtKind,
    pub span: crate::source::Span,
}

pub enum HIRStmtKind {
    Expr(HIRExpr),
    VarDec(crate::ast::scope::SymbolId),
    Return(HIRExpr),
}

pub enum Literal {
    Integer(u128),
    Float(f64),
    Bool(bool),
    Char(char),
    String(String),
}

// ---------- HIR-Resolver ----------
pub struct HIRResolver<'a> {
    pub scopes: &'a mut ScopeCtx,
}

impl<'a> HIRResolver<'a> {
    pub fn new(scopes: &'a mut ScopeCtx) -> Self {
        Self { scopes }
    }

    pub fn resolve_item(&mut self, item: &ASTItem) -> Option<HIRStmt> {
        match item {
            ASTItem::Stmt(stmt) => Some(self.resolve_stmt(stmt)),
            _ => None, // Use / Mod / Fn / Struct später
        }
    }

    pub fn resolve_stmt(&mut self, stmt: &crate::ast::ASTStmt) -> HIRStmt {
        match &stmt.kind {
            crate::ast::ASTStmtKind::Expr(expr) => HIRStmt {
                kind: HIRStmtKind::Expr(self.resolve_expr(expr)),
                span: stmt.span,
            },
            crate::ast::ASTStmtKind::VarDec(dec) => {
                let name = match &dec.identifier.kind {
                    crate::TokenKind::Identifier(s) => s,
                    _ => unreachable!(),
                };

                let init_ty = self.resolve_expr(&dec.initializer).ty;

                let sym_id = self
                    .scopes
                    .define_symbol(Symbol {
                        name: self.scopes.name_interner.intern(name),
                        type_: init_ty,
                        mutable: dec.mutable,
                        public: dec.public,
                        span: dec.identifier.span,
                    })
                    .expect("Variable already defined");

                HIRStmt {
                    kind: HIRStmtKind::VarDec(sym_id),
                    span: stmt.span,
                }
            }
            crate::ast::ASTStmtKind::Return(expr) => HIRStmt {
                kind: HIRStmtKind::Return(self.resolve_expr(expr)),
                span: stmt.span,
            },
            _ => unimplemented!(),
        }
    }

    pub fn resolve_expr(&mut self, expr: &ASTExpr) -> HIRExpr {
        match &expr.kind {
            crate::ast::ASTExprKind::Integer(v) => HIRExpr {
                kind: HIRExprKind::Literal(Literal::Integer(*v)),
                ty: crate::types::Ty::Primitive(crate::types::Primitive::U128),
                span: expr.span,
            },
            crate::ast::ASTExprKind::Variable(_, name) => {
                let name_id = self.scopes.name_interner.intern(name);
                let sym_id = self
                    .scopes
                    .lookup_symbol(name_id)
                    .expect("Variable not found");

                let ty = self.scopes.symbol_type(sym_id);
                HIRExpr {
                    kind: HIRExprKind::Variable(sym_id),
                    ty,
                    span: expr.span,
                }
            }
            crate::ast::ASTExprKind::Assignment(assign) => {
                let name = match &assign.target.kind {
                    crate::TokenKind::Identifier(s) => s,
                    _ => unreachable!(),
                };
                let name_id = self.scopes.name_interner.intern(name);
                let sym_id = self
                    .scopes
                    .lookup_current(name_id)
                    .expect("Assignment target not found");

                let value = Box::new(self.resolve_expr(&assign.value));

                HIRExpr {
                    kind: HIRExprKind::Assignment(sym_id, value),
                    ty: self.scopes.symbol_type(sym_id),
                    span: expr.span,
                }
            }
            crate::ast::ASTExprKind::Block(block) => {
                self.scopes.push();
                let mut stmts = vec![];
                for stmt in &block.statements {
                    stmts.push(self.resolve_stmt(stmt));
                }
                let tail_expr = block
                    .tail_expr
                    .as_ref()
                    .map(|e| Box::new(self.resolve_expr(e)));
                self.scopes.pop();

                HIRExpr {
                    kind: HIRExprKind::Block(stmts, tail_expr),
                    ty: tail_expr
                        .as_ref()
                        .map(|e| e.ty)
                        .unwrap_or(crate::types::Ty::Tuple(vec![])), // unit
                    span: expr.span,
                }
            }
            _ => unimplemented!(),
        }
    }
}

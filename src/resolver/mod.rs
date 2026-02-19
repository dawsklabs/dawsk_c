use crate::ast::{scope::{NameInterner, ScopeCtx}, structs::{StructArena, StructFields}, AST, ASTStmtKind, types::{InferCtx, TyInterner}, token::TokenKind};

pub struct Resolver<'a> {
    pub names: &'a mut NameInterner,
    pub structs: &'a mut StructArena,
    pub ty_interner: &'a mut TyInterner,
    pub infer: &'a mut InferCtx,
    pub scopes: &'a mut ScopeCtx,
}

impl<'a> Resolver<'a> {
    pub fn collect_structs(&mut self, ast: &AST) {
        for stmt in &ast.stmts {
            match &stmt.kind {
                ASTStmtKind::StructDec(s)
                | ASTStmtKind::TupleStructDec(s) => {
                    let name = self.names.intern(match &s.identifier.kind {
                        TokenKind::Identifier(name) => name,
                        _ => "_",
                    });
                    self.structs
                        .alloc_placeholder(name, s.identifier.span.clone())
                        .unwrap();
                }
                _ => {}
            }
        }
    }

    pub fn resolve_structs(&mut self, ast: &AST) {
        for stmt in &ast.stmts {
            match &stmt.kind {
                ASTStmtKind::StructDec(s) => {
                    let name = self.names.intern(match &s.identifier.kind {
                        TokenKind::Identifier(name) => name,
                        _ => "_",
                    });
                    let sid = self.structs.lookup(name).unwrap();

                    let fields = s.fields.iter()
                        .map(|f| (
                            self.names.intern(match &s.identifier.kind {
                                TokenKind::Identifier(name) => name,
                                _ => "_",
                            }),
                            self.resolve_ty(&f.type_),
                        ))
                        .collect();

                    self.structs
                        .set_fields(sid, StructFields::Named(fields))
                        .unwrap();
                }

                ASTStmtKind::TupleStructDec(s) => {
                    let name = self.names.intern(match &s.identifier.kind {
                        TokenKind::Identifier(name) => name,
                        _ => "_",
                    });
                    let sid = self.structs.lookup(name).unwrap();

                    let fields = s.fields.iter()
                        .map(|t| self.resolve_ty(t))
                        .collect();

                    self.structs
                        .set_fields(sid, StructFields::Tuple(fields))
                        .unwrap();
                }
                _ => {}
            }
        }
    }
}
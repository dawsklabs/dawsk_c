use std::collections::HashMap;

use crate::ast::types::{LiteralType, TypeCtx, TypeId, TypeKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TraitKind {
    Add,       // +, +=
    Sub,       // -, -=
    Mul,       // *, *=
    Div,       // /, /=
    Rem,       // %, &=
    Not,       // !
    Neg,       // -
    BitAnd,    // &
    Eq,        // ==
    PartialEq, // ==, !=
    Cast,      // .. as ..
    Custom(u32),
}

#[derive(Debug, Clone)]
pub struct TraitImpl {
    pub trait_kind: TraitKind,
    pub params: Vec<TypeId>,
    pub output: TypeId,
}

#[derive(Debug, Clone)]
pub struct TraitImplKind {
    pub trait_kind: TraitKind,
    pub params: Vec<TypeKind>,
    pub output: TypeKind,
}

pub struct TraitCtx {
    pub impls: HashMap<TypeId, Vec<TraitImpl>>,
}

impl TraitCtx {
    pub fn new(types: &TypeCtx) -> Self {
        let mut ctx = Self {
            impls: HashMap::new(),
        };

        ctx.gen_default_traits(types); // types weitergeben
        ctx
    }

    pub fn add_impl(
        &mut self,
        types: &TypeCtx,
        ty: TypeKind,
        tr: TraitImplKind, // TraitImplKind statt TraitImpl
    ) {
        let ty_id = types.intern(ty);

        let tr = TraitImpl {
            trait_kind: tr.trait_kind,
            params: tr.params.into_iter().map(|p| types.intern(p)).collect(),
            output: ty_id,
        };

        self.impls.entry(ty_id).or_default().push(tr);
    }

    pub fn implements(&self, ty: TypeId, tr: TraitKind, params: &[TypeId]) -> Option<TypeId> {
        self.impls.get(&ty)?.iter().find_map(|impl_| {
            (impl_.trait_kind == tr && impl_.params == params).then_some(impl_.output)
        })
    }

    pub fn gen_default_traits(&mut self, types: &TypeCtx) {
        use crate::ast::types::Primitive::*;

        // basic types
        self.add_impl(
            types,
            TypeKind::Literal(LiteralType::UInt),
            TraitImplKind {
                trait_kind: TraitKind::Add,
                params: vec![TypeKind::Literal(LiteralType::UInt)],
                output: TypeKind::Literal(LiteralType::UInt),
            },
        );

        for ty in &[I8, I16, I32, I64, U8, U16, U32, U64] {
            for op in &[TraitKind::Add, TraitKind::Sub, TraitKind::Mul] {
                self.add_impl(
                    types,
                    TypeKind::Primitive(*ty),
                    TraitImplKind {
                        trait_kind: *op,
                        params: vec![TypeKind::Literal(LiteralType::Int)],
                        output: TypeKind::Primitive(*ty),
                    },
                );

                self.add_impl(
                    types,
                    TypeKind::Literal(LiteralType::Int),
                    TraitImplKind {
                        trait_kind: *op,
                        params: vec![TypeKind::Primitive(*ty)],
                        output: TypeKind::Primitive(*ty),
                    },
                );

                self.add_impl(
                    types,
                    TypeKind::Primitive(*ty),
                    TraitImplKind {
                        trait_kind: *op,
                        params: vec![TypeKind::Primitive(*ty)],
                        output: TypeKind::Primitive(*ty),
                    },
                );
            }
        }

        for op in &[TraitKind::Add, TraitKind::Sub, TraitKind::Mul] {
            self.add_impl(
                types,
                TypeKind::Literal(LiteralType::Int),
                TraitImplKind {
                    trait_kind: *op,
                    params: vec![TypeKind::Literal(LiteralType::Int)],
                    output: TypeKind::Literal(LiteralType::Int),
                },
            );

            self.add_impl(
                types,
                TypeKind::Literal(LiteralType::UInt),
                TraitImplKind {
                    trait_kind: *op,
                    params: vec![TypeKind::Literal(LiteralType::Int)],
                    output: TypeKind::Literal(LiteralType::Int),
                },
            );
            self.add_impl(
                types,
                TypeKind::Literal(LiteralType::Int),
                TraitImplKind {
                    trait_kind: *op,
                    params: vec![TypeKind::Literal(LiteralType::UInt)],
                    output: TypeKind::Literal(LiteralType::Int),
                },
            );
        }

        for ty in &[F32, F64] {
            for op in &[
                TraitKind::Add,
                TraitKind::Sub,
                TraitKind::Mul,
                TraitKind::Div,
            ] {
                self.add_impl(
                    types,
                    TypeKind::Primitive(*ty),
                    TraitImplKind {
                        trait_kind: *op,
                        params: vec![TypeKind::Literal(LiteralType::Float)],
                        output: TypeKind::Primitive(*ty),
                    },
                );

                self.add_impl(
                    types,
                    TypeKind::Literal(LiteralType::Float),
                    TraitImplKind {
                        trait_kind: *op,
                        params: vec![TypeKind::Primitive(*ty)],
                        output: TypeKind::Primitive(*ty),
                    },
                );

                self.add_impl(
                    types,
                    TypeKind::Primitive(*ty),
                    TraitImplKind {
                        trait_kind: *op,
                        params: vec![TypeKind::Primitive(*ty)],
                        output: TypeKind::Primitive(*ty),
                    },
                );
            }
        }

        self.add_impl(
            types,
            TypeKind::Primitive(Bool),
            TraitImplKind {
                trait_kind: TraitKind::Not,
                params: vec![],
                output: TypeKind::Primitive(Bool),
            },
        );

        self.add_impl(
            types,
            TypeKind::Literal(LiteralType::UInt),
            TraitImplKind {
                trait_kind: TraitKind::Neg,
                params: vec![],
                output: TypeKind::Literal(LiteralType::Int),
            },
        );

        self.add_impl(
            types,
            TypeKind::Literal(LiteralType::Int),
            TraitImplKind {
                trait_kind: TraitKind::Neg,
                params: vec![],
                output: TypeKind::Literal(LiteralType::Int),
            },
        );

        // println!("Trait implementations added successfully!");
        // println!("{:#?}", self.impls);
    }
}

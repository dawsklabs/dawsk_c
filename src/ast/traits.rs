use smallvec::{smallvec, SmallVec};
use std::collections::HashMap;

use crate::types::inference::{InferCtx, InferTy};
use crate::types::{Primitive, Ty, TyInterner, TyKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TraitKind {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Not,
    Neg,
    BitAnd,
    Eq,
    PartialEq,
    Cast,
    Custom(u32),
}

#[derive(Debug, Clone)]
pub struct TraitImpl {
    pub trait_kind: TraitKind,
    pub params: SmallVec<[Ty; 2]>,
    pub output: Ty,
}

pub struct TraitCtx {
    pub impls: HashMap<Ty, Vec<TraitImpl>>,
}

impl<'a> TraitCtx {
    pub fn new() -> Self {
        Self {
            impls: HashMap::new(),
        }
    }

    pub fn add_impl(&mut self, output: Ty, tr: TraitKind, params: SmallVec<[Ty; 2]>) {
        self.impls.entry(output).or_default().push(TraitImpl {
            trait_kind: tr,
            params,
            output,
        });
    }

    pub fn implements(&self, ty: Ty, tr: TraitKind, params: &[Ty]) -> Option<Ty> {
        self.impls.get(&ty)?.iter().find_map(|impl_| {
            (impl_.trait_kind == tr && impl_.params.as_slice() == params).then_some(impl_.output)
        })
    }

    pub fn implements_infer(
        &self,
        infer_ctx: &mut InferCtx,
        ty_interner: &mut TyInterner,
        ty: InferTy,
        tr: TraitKind,
        params: &[InferTy],
    ) -> Option<Ty> {
        let ty = infer_ctx.resolve_to_ty(ty_interner, ty); // converts to concrete Ty
        let params: Vec<Ty> = params
            .iter()
            .map(|p| infer_ctx.resolve_to_ty(ty_interner, p.clone()))
            .collect();

        self.implements(ty, tr, &params)
    }

    pub fn gen_default_traits(&mut self, ty_interner: &mut TyInterner) {
        use Primitive::*;

        let (bool_t, int_prims, uint_prims) = {
            (
                ty_interner.intern(TyKind::Primitive(Bool)),
                [I8, I16, I32, I64],
                [U8, U16, U32, U64],
            )
        };

        for &p in int_prims.iter().chain(uint_prims.iter()) {
            let prim_t = { ty_interner.intern(TyKind::Primitive(p)) };

            for &op in &[
                TraitKind::Add,
                TraitKind::Sub,
                TraitKind::Mul,
                TraitKind::Div,
                TraitKind::Rem,
            ] {
                self.add_impl(prim_t, op, smallvec![prim_t]);
            }
        }

        for &p in &[F32, F64] {
            let prim_t = { ty_interner.intern(TyKind::Primitive(p)) };

            for &op in &[
                TraitKind::Add,
                TraitKind::Sub,
                TraitKind::Mul,
                TraitKind::Div,
            ] {
                self.add_impl(prim_t, op, smallvec![prim_t]);
            }
        }

        self.add_impl(bool_t, TraitKind::Not, smallvec![]);
    }
}

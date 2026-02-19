use crate::Compiler;

use super::{Primitive, Ty, TyInterner, TyKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InferVar(u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InferClass {
    Integer,
    Float,
}

#[derive(Debug, Clone)]
pub enum InferNode {
    Var {
        parent: u32,
        rank: u8,
        class: InferClass,
    }, // union-find
    Link(Ty), // bound to a known Ty
    Error,
}

pub struct InferCtx {
    nodes: Vec<InferNode>,
}

impl InferCtx {
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    pub fn fresh_int(&mut self) -> InferVar {
        let id = self.nodes.len() as u32;
        self.nodes.push(InferNode::Var {
            parent: id,
            rank: 0,
            class: InferClass::Integer,
        });
        InferVar(id)
    }

    pub fn fresh_float(&mut self) -> InferVar {
        let id = self.nodes.len() as u32;
        self.nodes.push(InferNode::Var {
            parent: id,
            rank: 0,
            class: InferClass::Float,
        });
        InferVar(id)
    }

    fn find_root(&mut self, v: u32) -> u32 {
        let mut x = v;
        while let InferNode::Var { parent, .. } = self.nodes[x as usize] {
            if parent == x {
                break;
            }
            x = parent;
        }
        let root = x;

        let mut x = v;
        while let InferNode::Var {
            parent,
            rank: _,
            class,
        } = self.nodes[x as usize]
        {
            if parent == root {
                break;
            }
            self.nodes[x as usize] = InferNode::Var {
                parent: root,
                rank: 0,
                class,
            };
            x = parent;
        }

        root
    }

    pub fn resolve(&mut self, ty: InferTy) -> InferTy {
        match ty {
            InferTy::Var(v) => {
                let root = self.find_root(v.0);
                match self.nodes[root as usize] {
                    InferNode::Link(t) => {
                        // optional: path compression
                        self.nodes[v.0 as usize] = InferNode::Link(t);
                        InferTy::Known(t)
                    }
                    InferNode::Error => InferTy::Error,
                    _ => InferTy::Var(InferVar(root)),
                }
            }
            _ => ty,
        }
    }

    pub fn resolve_to_ty<'a>(&mut self, ty_interner: &'a mut TyInterner, ty: InferTy) -> Ty {
        match self.resolve(ty) {
            InferTy::Known(t) => t,
            InferTy::Var(v) => {
                // If still unresolved, default it
                let class = match self.nodes[v.0 as usize] {
                    InferNode::Var { class, .. } => class,
                    _ => return ty_interner.intern(TyKind::Error),
                };

                let default = match class {
                    InferClass::Integer => Primitive::I32,
                    InferClass::Float => Primitive::F64,
                };

                ty_interner.intern(TyKind::Primitive(default))
            }
            InferTy::Error => ty_interner.intern(TyKind::Error),
        }
    }

    #[must_use]
    pub fn unify(
        &mut self,
        ty_interner: &mut TyInterner,
        a: InferTy,
        b: InferTy,
    ) -> Result<InferTy, ()> {
        let a = self.resolve(a);
        let b = self.resolve(b);

        match (a, b) {
            (InferTy::Error, _) | (_, InferTy::Error) => Ok(InferTy::Error),

            (InferTy::Known(x), InferTy::Known(y)) => {
                if x == y {
                    Ok(InferTy::Known(x))
                } else {
                    Err(())
                }
            }

            (InferTy::Var(v), InferTy::Known(t)) | (InferTy::Known(t), InferTy::Var(v)) => {
                let root = self.find_root(v.0);

                let class = match self.nodes[root as usize] {
                    InferNode::Var { class, .. } => class,
                    _ => return Ok(InferTy::Known(t)),
                };

                if !self.ty_fits_class(ty_interner, t, class) {
                    return Err(());
                }

                self.nodes[root as usize] = InferNode::Link(t);
                Ok(InferTy::Known(t))
            }

            (InferTy::Var(v1), InferTy::Var(v2)) => {
                let r1 = self.find_root(v1.0);
                let r2 = self.find_root(v2.0);

                if r1 == r2 {
                    return Ok(InferTy::Var(InferVar(r1)));
                }

                let (rank1, class1) = match self.nodes[r1 as usize] {
                    InferNode::Var { rank, class, .. } => (rank, class),
                    _ => unreachable!(),
                };

                let (rank2, class2) = match self.nodes[r2 as usize] {
                    InferNode::Var { rank, class, .. } => (rank, class),
                    _ => unreachable!(),
                };

                if class1 != class2 {
                    return Err(());
                }

                // union by rank
                if rank1 < rank2 {
                    self.nodes[r1 as usize] = InferNode::Var {
                        parent: r2,
                        rank: 0,
                        class: class1,
                    };
                    Ok(InferTy::Var(InferVar(r2)))
                } else {
                    self.nodes[r2 as usize] = InferNode::Var {
                        parent: r1,
                        rank: 0,
                        class: class2,
                    };
                    if rank1 == rank2 {
                        if let InferNode::Var { rank, .. } = &mut self.nodes[r1 as usize] {
                            *rank += 1;
                        }
                    }
                    Ok(InferTy::Var(InferVar(r1)))
                }
            }
        }
    }

    pub fn default_unresolved<'a>(&mut self, compiler: &'a mut Compiler) {
        for node in &mut self.nodes {
            if let InferNode::Var { class, .. } = *node {
                let default = match class {
                    InferClass::Integer => Primitive::I32,
                    InferClass::Float => Primitive::F32,
                };

                let ty = compiler.ty_interner.intern(TyKind::Primitive(default));
                *node = InferNode::Link(ty);
            }
        }
    }

    fn ty_fits_class<'a>(
        &self,
        ty_interner: &'a mut TyInterner,
        ty: Ty,
        class: InferClass,
    ) -> bool {
        match (class, ty_interner.kind(ty)) {
            (InferClass::Integer, TyKind::Primitive(p)) => matches!(
                p,
                Primitive::I8
                    | Primitive::I16
                    | Primitive::I32
                    | Primitive::I64
                    | Primitive::I128
                    | Primitive::ISize
                    | Primitive::U8
                    | Primitive::U16
                    | Primitive::U32
                    | Primitive::U64
                    | Primitive::U128
                    | Primitive::USize
            ),
            (InferClass::Float, TyKind::Primitive(p)) => {
                matches!(p, Primitive::F32 | Primitive::F64)
            }
            _ => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum InferTy {
    Known(Ty),
    Var(InferVar),
    Error,
}

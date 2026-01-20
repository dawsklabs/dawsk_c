use bumpalo::Bump;
use smallvec::SmallVec;

use std::fmt::{Display, Formatter, Result};
use std::hash::{Hash, Hasher};
use std::collections::HashMap;
use std::result::Result as StdResult;

use crate::Compiler;

// ---------------- SYMBOL INTERN ----------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Symbol(pub u32);

pub struct SymbolInterner {
    map: HashMap<Box<str>, Symbol>,
    arena: Vec<Box<str>>,
}

impl SymbolInterner {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
            arena: Vec::new(),
        }
    }

    pub fn intern(&mut self, s: &str) -> Symbol {
        if let Some(&sym) = self.map.get(s) {
            return sym;
        }

        let boxed: Box<str> = s.into();
        let sym = Symbol(self.arena.len() as u32);
        self.arena.push(boxed.clone());
        self.map.insert(boxed, sym);
        sym
    }

    pub fn lookup(&self, sym: Symbol) -> &str {
        &self.arena[sym.0 as usize]
    }
}

// ---------------- TYPES ----------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Ty(pub u32); // interned ID (pointer)

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TyKind {
    Primitive(Primitive),
    Tuple(SmallVec<[Ty; 2]>),
    Array(Ty, usize),
    Generic { base: Symbol, args: SmallVec<[Ty; 2]> },
    Ref(Ty),
    MutRef(Ty),
    Custom(Symbol),
    Error,
}

// ---------------- INTERNER (BUMP ARENA + POINTER KEY) ----------------

#[derive(Clone, Copy)]
struct TyKindKey(*const TyKind);

impl PartialEq for TyKindKey {
    fn eq(&self, other: &Self) -> bool {
        unsafe { std::ptr::eq(self.0, other.0) || *self.0 == *other.0 }
    }
}

impl Eq for TyKindKey {}

impl Hash for TyKindKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        unsafe { (*self.0).hash(state) }
    }
}

pub struct TyInterner {
    arena: Bump,
    index: HashMap<TyKindKey, Ty>,
    // for lookup by Ty
    kinds: Vec<*const TyKind>,
}

impl TyInterner {
    pub fn new() -> Self {
        let arena = Bump::new();
        let mut interner = Self {
            arena,
            index: HashMap::new(),
            kinds: Vec::new(),
        };

        // Ty::Error reserved at ID 0
        let err = interner.arena.alloc(TyKind::Error);
        interner.kinds.push(err);
        interner.index.insert(TyKindKey(err), Ty(0));

        interner
    }

    pub fn intern(&mut self, kind: TyKind) -> Ty {
        // allocate into arena first
        let allocated: &TyKind = self.arena.alloc(kind);

        // lookup by pointer-key
        let key = TyKindKey(allocated);

        if let Some(&ty) = self.index.get(&key) {
            return ty;
        }

        let id = Ty(self.kinds.len() as u32);
        self.kinds.push(allocated);
        self.index.insert(key, id);
        id
    }

    pub fn kind(&self, ty: Ty) -> &TyKind {
        unsafe { &*self.kinds[ty.0 as usize] }
    }
}

// ---------------- INFERENCE ----------------

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
        class: InferClass
    }, // union-find
    Link(Ty),                      // bound to a known Ty
    Error,
}

pub struct InferCtx
 {
    nodes: Vec<InferNode>,
}

impl InferCtx {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new()
        }
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
        while let InferNode::Var { parent, rank: _, class } = self.nodes[x as usize] {
            if parent == root {
                break;
            }
            self.nodes[x as usize] = InferNode::Var { parent: root, rank: 0, class };
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
    pub fn unify(&mut self, ty_interner: &mut TyInterner, a: InferTy, b: InferTy) -> StdResult<InferTy, ()> {
        let a = self.resolve(a);
        let b = self.resolve(b);

        match (a, b) {
            (InferTy::Error, _) | (_, InferTy::Error) => Ok(InferTy::Error),

            (InferTy::Known(x), InferTy::Known(y)) => {
                if x == y { Ok(InferTy::Known(x)) } else { Err(()) }
            }

            (InferTy::Var(v), InferTy::Known(t))
            | (InferTy::Known(t), InferTy::Var(v)) => {
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
                    InferClass::Float => Primitive::F64,
                };

                let ty = compiler.ty_interner.intern(TyKind::Primitive(default));
                *node = InferNode::Link(ty);
            }
        }
    }

    fn ty_fits_class<'a>(&self, ty_interner: &'a mut TyInterner, ty: Ty, class: InferClass) -> bool {
        match (class, ty_interner.kind(ty)) {
            (InferClass::Integer, TyKind::Primitive(p)) => matches!(
                p,
                Primitive::I8 | Primitive::I16 | Primitive::I32 |
                Primitive::I64 | Primitive::I128 | Primitive::ISize |
                Primitive::U8 | Primitive::U16 | Primitive::U32 |
                Primitive::U64 | Primitive::U128 | Primitive::USize
            ),
            (InferClass::Float, TyKind::Primitive(p)) => matches!(
                p,
                Primitive::F32 | Primitive::F64
            ),
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

// ---------------- PRIMITIVES ----------------

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum Primitive {
    I8, I16, I32, I64, I128, ISize,
    U8, U16, U32, U64, U128, USize,
    F32, F64,
    Bool, Char, String,
}

// ---------------- DISPLAY ----------------

impl Display for TyKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            TyKind::Primitive(p) => write!(f, "{}", p),
            TyKind::Tuple(ts) => {
                write!(f, "(")?;
                for (i, t) in ts.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", t.0)?;
                }
                write!(f, ")")
            }
            TyKind::Array(inner, size) => write!(f, "Array[{}]{{{size}}}", inner.0),
            TyKind::Ref(inner) => write!(f, "&{}", inner.0),
            TyKind::MutRef(inner) => write!(f, "&mut {}", inner.0),
            TyKind::Generic { base, args } => {
                write!(f, "{}<", base.0)?;
                for (i, t) in args.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", t.0)?;
                }
                write!(f, ">")
            }
            TyKind::Custom(s) => write!(f, "Custom({})", s.0),
            TyKind::Error => write!(f, "?T_E"),
        }
    }
}

impl Display for Primitive {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{:?}", self)
    }
}
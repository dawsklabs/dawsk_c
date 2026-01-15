use std::collections::HashMap;
use std::fmt::{Display, Formatter, Result};
use std::hash::Hash;
use std::result::Result as StdResult;

use crate::color::{Color, BLUE_COLOR, FLAMINGO_COLOR, GREEN_COLOR, MAUVE_COLOR};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TyVarId(pub u32);

impl Display for TyVarId {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone)]
pub enum TyKind {
    Literal(LiteralType),
    Primitive(Primitive),
    Tuple(Vec<Ty>),
    Array(Ty, usize),
    Generic { base: &'static str, args: Vec<Ty> },
    Ref(Ty),
    MutRef(Ty),
    Custom(&'static str),
    Never,
}

impl Display for TyKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            TyKind::Literal(lit) => write!(f, "{}", lit),
            TyKind::Primitive(p) => write!(f, "{}{}{}", MAUVE_COLOR, p, Color::ResetFg),
            TyKind::Tuple(t) => {
                write!(f, "(")?;
                for (i, ty) in t.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", ty.0)?;
                }
                write!(f, ")")
            }
            TyKind::Array(inner, size) => write!(f, "Array[{}]{{{}}}", inner.0, size),
            TyKind::Custom(s) => write!(f, "{}{}{}", BLUE_COLOR, s, Color::ResetFg),
            TyKind::Ref(inner) => write!(f, "{}&{}{}", FLAMINGO_COLOR, Color::ResetFg, inner.0),
            TyKind::MutRef(inner) => {
                write!(f, "{}&mut {}{}", FLAMINGO_COLOR, Color::ResetFg, inner.0)
            }
            TyKind::Generic { base, args } => {
                write!(f, "{}{}{}", GREEN_COLOR, base, Color::ResetFg)?;
                write!(f, "<")?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?
                    }
                    write!(f, "{}", arg.0)?;
                }
                write!(f, ">")
            }
            TyKind::Never => write!(f, "?T_N")
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Ty(u32); // INTERNED ID


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InferVar(u32);

#[derive(Debug, Clone)]
pub enum InferTy {
    Known(Ty),
    Var(InferVar),
    Error,
}

pub struct TyInterner {
    arena: Vec<TyKind>,
    index: HashMap<u64, Ty>,
}

impl TyInterner {
    pub fn intern(&mut self, kind: TyKind) -> Ty {
        let hash = self.semantic_hash(&kind);

        if let Some(&ty) = self.index.get(&hash) {
            return ty;
        }

        let id = Ty(self.arena.len() as u32);
        self.arena.push(kind);
        self.index.insert(hash, id);
        id
    }

    pub fn kind(&self, ty: Ty) -> &TyKind {
        &self.arena[ty.0 as usize]
    }

    fn hash_type(&self, ty: &TyKind) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::Hasher;

        let mut hasher = DefaultHasher::new();
        std::mem::discriminant(ty).hash(&mut hasher);

        match ty {
            TyKind::Literal(l) => l.hash(&mut hasher),
            TyKind::Primitive(p) => p.hash(&mut hasher),
            TyKind::Tuple(ts) => ts.iter().for_each(|t| t.0.hash(&mut hasher)),
            TyKind::Array(inner, size) => { inner.0.hash(&mut hasher); size.hash(&mut hasher); }
            TyKind::Ref(inner) | TyKind::MutRef(inner) => inner.0.hash(&mut hasher),
            TyKind::Generic { base, args } => { base.hash(&mut hasher); args.iter().for_each(|a| a.0.hash(&mut hasher)); }
            TyKind::Custom(s) => s.hash(&mut hasher),
            TyKind::Never => {},
        }

        hasher.finish()
    }

    fn semantic_hash(&self, ty: &TyKind) -> u64 {
        self.hash_type(ty)
    }
}

pub struct InferCtx {
    parents: Vec<InferTy>, // union-find
}

impl InferCtx {
    pub fn new() -> Self { Self { parents: Vec::new() } }

    pub fn fresh_var(&mut self) -> InferVar {
        let id = self.parents.len() as u32;
        self.parents.push(InferTy::Var(InferVar(id)));
        InferVar(id)
    }

    pub fn resolve(&mut self, ty: InferTy) -> InferTy {
        match ty {
            InferTy::Var(v) => {
                match self.parents[v.0 as usize].clone() {
                    InferTy::Var(_) => ty,
                    other => {
                        let resolved = self.resolve(other);
                        self.parents[v.0 as usize] = resolved.clone();
                        resolved
                    }
                }
            }
            _ => ty,
        }
    }

    #[must_use]
    pub fn unify(&mut self, a: InferTy, b: InferTy) -> StdResult<InferTy, ()> {
        let a = self.resolve(a);
        let b = self.resolve(b);

        match (a, b) {
            (InferTy::Error, _) | (_, InferTy::Error) => Ok(InferTy::Error),
            (InferTy::Known(x), InferTy::Known(y)) => {
                if x == y { Ok(InferTy::Known(x)) } else { Err(()) }
            }
            (InferTy::Var(v), ty) | (ty, InferTy::Var(v)) => {
                self.parents[v.0 as usize] = ty.clone();
                Ok(ty)
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum LiteralType {
    Int,
    UInt,
    Float,
}

impl Display for LiteralType {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        use LiteralType::*;
        match self {
            Int => write!(f, "Integer Literal"),
            UInt => write!(f, "Unsigned Integer Literal"),
            Float => write!(f, "Float Literal"),
        }
    }
}

#[derive(Debug, Clone, Copy, Hash)]
pub enum Primitive {
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
    F32,
    F64,
    Bool,
    Char,
    String,
}

impl Display for Primitive {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        use Primitive::*;
        match self {
            I8 => write!(f, "i8"),
            I16 => write!(f, "i16"),
            I32 => write!(f, "i32"),
            I64 => write!(f, "i64"),
            U8 => write!(f, "u8"),
            U16 => write!(f, "u16"),
            U32 => write!(f, "u32"),
            U64 => write!(f, "u64"),
            F32 => write!(f, "f32"),
            F64 => write!(f, "f64"),
            Bool => write!(f, "bool"),
            Char => write!(f, "char"),
            String => write!(f, "String"),
        }
    }
}

impl PartialEq for Primitive {
    fn eq(&self, other: &Self) -> bool {
        self.equals(other)
    }
}
impl Eq for Primitive {}

impl Primitive {
    pub fn equals(&self, other: &Primitive) -> bool {
        use Primitive::*;
        match (self, other) {
            (I8, I8)
            | (I16, I16)
            | (I32, I32)
            | (I64, I64)
            | (U8, U8)
            | (U16, U16)
            | (U32, U32)
            | (U64, U64)
            | (F32, F32)
            | (F64, F64)
            | (Bool, Bool)
            | (Char, Char)
            | (String, String) => true,
            _ => false,
        }
    }
}

// pub type TypeId = u64;

// #[derive(Debug, Clone)]
// pub struct TypeCtx(Rc<RefCell<TypeCtxInner>>);

// impl TypeCtx {
//     pub fn new() -> Self {
//         Self(Rc::new(RefCell::new(TypeCtxInner::new())))
//     }

//     pub fn intern(&self, kind: TypeKind) -> TypeId {
//         self.0.borrow_mut().intern(kind)
//     }

//     pub fn type_id(&self, kind: TypeKind) -> TypeId {
//         self.0.borrow_mut().type_id(kind)
//     }

//     pub fn get(&self, id: TypeId) -> TypeKind {
//         self.0.borrow().get(id).clone()
//     }

//     pub fn fresh_type_var(&self) -> TypeId {
//         self.0.borrow_mut().fresh_type_var()
//     }

//     pub fn unify(&self, a: TypeId, b: TypeId) -> Result {
//         self.0.borrow_mut().unify(a, b)
//     }

//     // pub fn len(&self) -> usize {
//     //     self.0.borrow().len()
//     // }

//     pub fn types_eq(&self, a: TypeId, b: TypeId) -> bool {
//         self.0.borrow().types_eq(a, b)
//     }
// }

// #[derive(Debug, Clone)]
// pub struct TypeCtxInner {
//     arena: Vec<TypeKind>,           // ← TYPE ARENA
//     index: HashMap<u64, TypeId>,    // ← hash → id
//     bindings: Vec<Option<TypeId>>,  // TypeVarId → gebundener Typ
// }

// impl TypeCtxInner {
//     pub fn new() -> Self {
//         Self {
//             arena: Vec::new(),
//             index: HashMap::new(),
//             bindings: Vec::new(),
//         }
//     }

//     pub fn types_eq(&self, a: TypeId, b: TypeId) -> bool {
//         if a == b {
//             return true;
//         }

//         use TyKind::*;
//         match (&self.arena[a as usize], &self.arena[b as usize]) {
//             (Literal(x), Literal(y)) => x == y,
//             (Primitive(x), Primitive(y)) => x == y,

//             (Tuple(xs), Tuple(ys)) => {
//                 xs.len() == ys.len() && xs.iter().zip(ys).all(|(&x, &y)| self.types_eq(x, y))
//             }

//             (Array(x, sx), Array(y, sy)) => sx == sy && self.types_eq(*x, *y),

//             (Ref(x), Ref(y)) => self.types_eq(*x, *y),
//             (MutRef(x), MutRef(y)) => self.types_eq(*x, *y),

//             (Generic { base: b1, args: a1 }, Generic { base: b2, args: a2 }) => {
//                 b1 == b2
//                     && a1.len() == a2.len()
//                     && a1.iter().zip(a2).all(|(&x, &y)| self.types_eq(x, y))
//             }

//             (Custom(a), Custom(b)) => a == b,
//             (Untyped, Untyped) => true,
//             (Error, Error) => true,

//             _ => false,
//         }
//     }

//     pub fn intern(&mut self, kind: TypeKind) -> TypeId {
//         let tmp_id = self.arena.len() as TypeId;
//         self.arena.push(kind);

//         let hash = self.semantic_hash(tmp_id);

//         if let Some(&existing) = self.index.get(&hash) {
//             if self.types_eq(existing, tmp_id) {
//                 self.arena.pop(); // ← WICHTIG
//                 return existing;
//             }
//         }

//         self.index.insert(hash, tmp_id);
//         tmp_id
//     }

//     pub fn get_id(&self, ty: TypeKind) -> Option<TypeId> {
//         todo!()
//     }

//     pub fn hash_type(&self, id: TypeId, state: &mut impl Hasher) {
//         use TypeKind::*;
//         let ty = &self.arena[id as usize];

//         std::mem::discriminant(ty).hash(state);

//         match ty {
//             Literal(l) => l.hash(state),
//             Primitive(p) => p.hash(state),

//             Tuple(ts) => ts.iter().for_each(|&t| self.hash_type(t, state)),

//             Array(inner, size) => {
//                 self.hash_type(*inner, state);
//                 size.hash(state);
//             }

//             Ref(inner) | MutRef(inner) => self.hash_type(*inner, state),

//             Generic { base, args } => {
//                 base.hash(state);
//                 args.iter().for_each(|&a| self.hash_type(a, state));
//             }
//             Custom(name) => name.hash(state),
//             TypeVar(id) => id.hash(state),
//             Untyped | Error => {}
//         }
//     }

//     pub fn semantic_hash(&self, id: TypeId) -> u64 {
//         let mut h = DefaultHasher::new();
//         self.hash_type(id, &mut h);
//         h.finish()
//     }

//     pub fn get(&self, id: TypeId) -> &TypeKind {
//         &self.arena[id as usize]
//     }

//     pub fn fresh_type_var(&mut self) -> TypeId {
//         let id = self.arena.len() as TypeId;
//         self.arena.push(TypeKind::TypeVar(TypeVarId(id as u32)));
//         self.bindings.push(None);
//         id
//     }

//     pub fn resolve(&mut self, id: TypeId) -> TypeId {
//         match self.arena[id as usize] {
//             TypeKind::TypeVar(_) => {
//                 if let Some(bound) = self.bindings[id as usize] {
//                     let root = self.resolve(bound);
//                     self.bindings[id as usize] = Some(root);
//                     root
//                 } else {
//                     id
//                 }
//             }
//             _ => id,
//         }
//     }

//     pub fn unify(&mut self, a: TypeId, b: TypeId) -> Result {
//         let a = self.resolve(a);
//         let b = self.resolve(b);

//         if a == b {
//             return Ok(());
//         }

//         use TypeKind::*;
//         let (ta, tb) = (self.arena[a as usize].clone(), self.arena[b as usize].clone());

//         match (ta, tb) {
//             (TypeVar(_), _) => {
//                 self.bindings[a as usize] = Some(b);
//                 Ok(())
//             }
//             (_, TypeVar(_)) => {
//                 self.bindings[b as usize] = Some(a);
//                 Ok(())
//             }

//             (Primitive(x), Primitive(y)) if x == y => Ok(()),

//             (Tuple(xs), Tuple(ys)) if xs.len() == ys.len() => {
//                 let xs = xs.clone();
//                 let ys = ys.clone();

//                 for (x, y) in xs.into_iter().zip(ys) {
//                     self.unify(x, y)?;
//                 }

//                 Ok(())
//             }

//             (Array(x, sx), Array(y, sy)) if sx == sy => self.unify(x, y),

//             (Ref(x), Ref(y)) | (MutRef(x), MutRef(y)) => self.unify(x, y),

//             (
//                 Generic { base: b1, args: a1 },
//                 Generic { base: b2, args: a2 }
//             ) => {
//                 if b1 != b2 || a1.len() != a2.len() {
//                     return Err(std::fmt::Error);
//                 }

//                 let args1 = a1.clone();
//                 let args2 = a2.clone();

//                 for (x, y) in args1.into_iter().zip(args2) {
//                     self.unify(x, y)?;
//                 }

//                 Ok(())
//             }

//             (Custom(a), Custom(b)) if a == b => Ok(()),

//             _ => Err(std::fmt::Error),
//         }
//     }

//     // pub fn len(&self) -> usize {
//     //     self.arena.len()
//     // }
// }
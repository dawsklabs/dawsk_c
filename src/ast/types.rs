use std::cell::RefCell;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::fmt::{Display, Formatter, Result};
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use crate::color::{Color, BLUE_COLOR, FLAMINGO_COLOR, GREEN_COLOR, MAUVE_COLOR};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TypeVarId(pub u32);

impl Display for TypeVarId {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone)]
pub enum TypeKind {
    Literal(LiteralType),
    Primitive(Primitive),
    Tuple(Vec<TypeId>),
    Array(TypeId, usize), // e.g. [i32; 5]
    Generic {
        base: &'static str,
        args: Vec<TypeId>,
    },
    Custom(&'static str), // e.g. structs, custom types, etc...
    Ref(TypeId),
    MutRef(TypeId),
    Untyped,
    Error,
}

impl Display for TypeKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            TypeKind::Literal(lit) => write!(f, "{}", lit),
            TypeKind::Primitive(p) => write!(f, "{}{}{}", MAUVE_COLOR, p, Color::ResetFg),
            TypeKind::Tuple(t) => {
                write!(f, "(")?;
                for (i, ty) in t.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", ty)?;
                }
                write!(f, ")")
            }
            TypeKind::Array(inner, size) => write!(f, "Array[{}]{{{}}}", inner, size),
            TypeKind::Custom(s) => write!(f, "{}{}{}", BLUE_COLOR, s, Color::ResetFg),
            TypeKind::Ref(inner) => write!(f, "{}&{}{}", FLAMINGO_COLOR, Color::ResetFg, inner),
            TypeKind::MutRef(inner) => {
                write!(f, "{}&mut {}{}", FLAMINGO_COLOR, Color::ResetFg, inner)
            }
            TypeKind::Generic { base, args } => {
                write!(f, "{}{}{}", GREEN_COLOR, base, Color::ResetFg)?;
                write!(f, "<")?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?
                    }
                    write!(f, "{}", arg)?;
                }
                write!(f, ">")
            }
            TypeKind::Untyped => write!(f, "__UNTYPED"),
            TypeKind::Error => write!(f, "__ERROR<Type>"),
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

pub type TypeId = u64;

#[derive(Debug, Clone)]
pub struct TypeCtx(Rc<RefCell<TypeCtxInner>>);

impl TypeCtx {
    pub fn new() -> Self {
        Self(Rc::new(RefCell::new(TypeCtxInner::new())))
    }

    pub fn intern(&self, kind: TypeKind) -> TypeId {
        self.0.borrow_mut().intern(kind)
    }

    pub fn get(&self, id: TypeId) -> TypeKind {
        self.0.borrow().get(id).clone()
    }

    // pub fn len(&self) -> usize {
    //     self.0.borrow().len()
    // }

    // pub fn types_eq(&self, a: TypeId, b: TypeId) -> bool {
    //     self.0.borrow().types_eq(a, b)
    // }
}

#[derive(Debug, Clone)]
pub struct TypeCtxInner {
    arena: Vec<TypeKind>,        // ← TYPE ARENA
    index: HashMap<u64, TypeId>, // ← hash → id
}

impl TypeCtxInner {
    pub fn new() -> Self {
        Self {
            arena: Vec::new(),
            index: HashMap::new(),
        }
    }

    pub fn types_eq(&self, a: TypeId, b: TypeId) -> bool {
        if a == b {
            return true;
        }

        use TypeKind::*;
        match (&self.arena[a as usize], &self.arena[b as usize]) {
            (Literal(x), Literal(y)) => x == y,
            (Primitive(x), Primitive(y)) => x == y,

            (Tuple(xs), Tuple(ys)) => {
                xs.len() == ys.len() && xs.iter().zip(ys).all(|(&x, &y)| self.types_eq(x, y))
            }

            (Array(x, sx), Array(y, sy)) => sx == sy && self.types_eq(*x, *y),

            (Ref(x), Ref(y)) => self.types_eq(*x, *y),
            (MutRef(x), MutRef(y)) => self.types_eq(*x, *y),

            (Generic { base: b1, args: a1 }, Generic { base: b2, args: a2 }) => {
                b1 == b2
                    && a1.len() == a2.len()
                    && a1.iter().zip(a2).all(|(&x, &y)| self.types_eq(x, y))
            }

            (Custom(a), Custom(b)) => a == b,
            (Untyped, Untyped) => true,
            (Error, Error) => true,

            _ => false,
        }
    }

    pub fn intern(&mut self, kind: TypeKind) -> TypeId {
        let tmp_id = self.arena.len() as TypeId;
        self.arena.push(kind);

        let hash = self.semantic_hash(tmp_id);

        if let Some(&existing) = self.index.get(&hash) {
            if self.types_eq(existing, tmp_id) {
                self.arena.pop(); // ← WICHTIG
                return existing;
            }
        }

        self.index.insert(hash, tmp_id);
        tmp_id
    }

    pub fn hash_type(&self, id: TypeId, state: &mut impl Hasher) {
        use TypeKind::*;
        let ty = &self.arena[id as usize];

        std::mem::discriminant(ty).hash(state);

        match ty {
            Literal(l) => l.hash(state),
            Primitive(p) => p.hash(state),

            Tuple(ts) => ts.iter().for_each(|&t| self.hash_type(t, state)),

            Array(inner, size) => {
                self.hash_type(*inner, state);
                size.hash(state);
            }

            Ref(inner) | MutRef(inner) => self.hash_type(*inner, state),

            Generic { base, args } => {
                base.hash(state);
                args.iter().for_each(|&a| self.hash_type(a, state));
            }

            Custom(name) => name.hash(state),
            Untyped | Error => {}
        }
    }

    pub fn semantic_hash(&self, id: TypeId) -> u64 {
        let mut h = DefaultHasher::new();
        self.hash_type(id, &mut h);
        h.finish()
    }

    pub fn get(&self, id: TypeId) -> &TypeKind {
        &self.arena[id as usize]
    }

    // pub fn len(&self) -> usize {
    //     self.arena.len()
    // }
}

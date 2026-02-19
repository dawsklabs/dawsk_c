pub mod inference;
pub mod structs;

use bumpalo::Bump;
use smallvec::SmallVec;

use std::collections::HashMap;
use std::fmt::{Display, Formatter, Result};
use std::hash::{Hash, Hasher};

use crate::types::structs::StructId;

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
    Generic { base: Symbol, args: Box<[Ty]> },
    Struct(StructId),
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

// ---------------- PRIMITIVES ----------------
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum Primitive {
    I8,
    I16,
    I32,
    I64,
    I128,
    ISize,
    U8,
    U16,
    U32,
    U64,
    U128,
    USize,
    F32,
    F64,
    Bool,
    Char,
    String,
}

// ---------------- DISPLAY ----------------
use std::fmt::{self, Write};

impl TyInterner {
    pub fn fmt_with_symbols<W: Write>(
        &self,
        w: &mut W,
        ty: Ty,
        symbols: &SymbolInterner,
    ) -> fmt::Result {
        match self.kind(ty) {
            TyKind::Primitive(p) => write!(w, "{p:?}"),

            TyKind::Tuple(elems) => {
                write!(w, "(")?;
                for (i, t) in elems.iter().enumerate() {
                    if i > 0 {
                        write!(w, ", ")?;
                    }
                    self.fmt_with_symbols(w, *t, symbols)?;
                }
                write!(w, ")")
            }

            TyKind::Array(inner, n) => {
                write!(w, "[")?;
                self.fmt_with_symbols(w, *inner, symbols)?;
                write!(w, "; {}]", n)
            }

            TyKind::Ref(inner) => {
                write!(w, "&")?;
                self.fmt_with_symbols(w, *inner, symbols)
            }

            TyKind::MutRef(inner) => {
                write!(w, "&mut ")?;
                self.fmt_with_symbols(w, *inner, symbols)
            }

            TyKind::Generic { base, args } => {
                let name = symbols.lookup(*base);
                write!(w, "{name}")?;
                if !args.is_empty() {
                    write!(w, "<")?;
                    for (i, t) in args.iter().enumerate() {
                        if i > 0 {
                            write!(w, ", ")?;
                        }
                        self.fmt_with_symbols(w, *t, symbols)?;
                    }
                    write!(w, ">")?;
                }
                Ok(())
            }

            TyKind::Struct(s) => write!(w, "Struct{{{}}}", s.0),
            TyKind::Custom(s) => write!(w, "Custom({})", s.0),
            TyKind::Error => write!(w, "<error>"),
        }
    }

    /// Legacy-Methode für String-Ausgabe, baut einfach auf fmt_with_symbols
    pub fn display_with_symbols(&self, ty: Ty, symbols: &SymbolInterner) -> String {
        let mut buf = String::new();
        self.fmt_with_symbols(&mut buf, ty, symbols).unwrap();
        buf
    }
}

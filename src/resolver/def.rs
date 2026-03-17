use smallvec::SmallVec;
use std::marker::PhantomData;

// use crate::ast::scope::TyId;
use crate::ast::Mutability;
use crate::source::Span;

use super::{Idx, IndexVec};

pub type NameId = Id<String>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Symbol {
    pub name: NameId,
    pub span: Span,
}

// ======================================
// Def IDs
// ======================================

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Id<T>(u32, PhantomData<fn() -> T>);

impl<T> Idx for Id<T> {
    fn new(idx: usize) -> Self {
        assert!(idx < u32::MAX as usize);
        Id(idx as u32, PhantomData)
    }

    fn index(self) -> usize {
        self.0 as usize
    }
}

pub type ScopeId = Id<Scope>;

pub type StructId = Id<StructDef>;
pub type EnumId = Id<EnumDef>;
pub type FunctionId = Id<FunctionDef>;
pub type BodyId = Id<Body>;
pub type ExprId = Id<Expr>;
pub type TraitId = Id<TraitDef>;
pub type TraitItemId = Id<TraitItem>;
pub type ImplId = Id<ImplDef>;
pub type TypeAliasId = Id<TypeAliasDef>;
pub type VarId = Id<Var>;

// ======================================
// DefTables (wie Rust intern HIR/DefMap)
// ======================================

pub struct DefTables {
    pub structs: IndexVec<StructId, StructDef>,
    pub enums: IndexVec<EnumId, EnumDef>,
    pub functions: IndexVec<FunctionId, FunctionDef>,
    pub traits: IndexVec<TraitId, TraitDef>,
    pub impls: IndexVec<ImplId, ImplDef>,
    pub type_aliases: IndexVec<TypeAliasId, TypeAliasDef>,
}

// ======================================
// Generics
// ======================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenericParam {
    pub name: NameId,
    pub bounds: SmallVec<[TraitId; 2]>, // Trait bounds
    pub default: Option<TyId>,
}

pub type Generics = SmallVec<[GenericParam; 4]>;

// ======================================
// Structs
// ======================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructDef {
    pub name: Symbol,
    pub generics: Generics,
    pub kind: StructKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StructKind {
    Named(Vec<Field>),
    Tuple(Vec<Field>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    pub name: Option<Symbol>, // None bei Tuple-Structs
    pub ty: TyId,
    pub span: Span,
}

// ======================================
// Enums
// ======================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumDef {
    pub name: Symbol,
    pub generics: Generics,
    pub variants: Vec<Variant>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variant {
    pub name: Symbol,
    pub kind: VariantKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VariantKind {
    Unit,
    Tuple(Vec<TyId>),
    Struct(Vec<Field>),
}

// ======================================
// Functions
// ======================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionDef {
    pub name: Symbol,
    pub generics: Generics,
    pub sig: FnSig,
    pub body: Option<BodyId>, // None = extern
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FnSig {
    pub generics: Generics,
    pub params: Vec<TyId>,
    pub ret: TyId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Body {
    pub root: ExprId,
    pub span: Span,
}

pub struct BodyArena {
    pub bodies: IndexVec<BodyId, Body>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    Xor,
    BitAnd,
    BitOr,
    BitXor,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    Literal(i64),
    Binary {
        op: BinOp,
        lhs: ExprId,
        rhs: ExprId,
        span: Span,
    },
}

pub struct ExprArena {
    exprs: IndexVec<ExprId, Expr>,
}

impl ExprArena {
    pub fn new() -> Self {
        Self {
            exprs: IndexVec::new(),
        }
    }

    pub fn alloc(&mut self, expr: Expr) -> ExprId {
        self.exprs.push(expr)
    }

    pub fn get(&self, id: ExprId) -> &Expr {
        &self.exprs[id]
    }
}

// ======================================
// Traits
// ======================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraitDef {
    pub name: Symbol,
    pub generics: Generics,
    pub items: Vec<TraitItemId>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraitItem {
    pub name: Symbol,
    pub kind: TraitItemKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TraitItemKind {
    Method {
        sig: FnSig,
        body: Option<BodyId>, // None = default impl
    },
    Type {
        generics: Generics,
        bounds: SmallVec<[TraitId; 2]>,
        default: Option<TyId>,
    },
    Const {
        ty: TyId,
        default: Option<ExprId>,
    },
}

// ======================================
// Impl blocks
// ======================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImplDef {
    pub generics: Generics,
    pub target: TyId,
    pub trait_id: Option<TraitId>, // None = inherent impl
    pub items: Vec<FunctionId>,
}

// ======================================
// Type Aliases
// ======================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeAliasDef {
    pub name: Symbol,
    pub generics: Generics,
    pub target: TyId,
    pub span: Span,
}

// ======================================
// Scopes für lokale Variablen
// ======================================

pub struct Scope {
    pub parent: Option<ScopeId>,
    pub variables: IndexVec<VarId, Var>, // localsd
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Var {
    pub name: Symbol,
    pub ty: TyId,
    pub mut_: Mutability,
    pub span: Span,
}

pub struct ScopeArena {
    scopes: IndexVec<ScopeId, Scope>,
}

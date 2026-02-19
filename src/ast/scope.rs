use smallvec::SmallVec;

use crate::source::Span;
use crate::types::Ty;

use std::{collections::HashMap, hash::Hash};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct ScopeId(pub u32);

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct SymbolId(pub u32);

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct TyId(pub u32);

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct FuncId(pub u32);

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct TraitId(pub u32);

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct NameId(pub u32);

// ---------- Name Interner (std::HashMap) ----------
pub struct NameInterner {
    map: HashMap<String, NameId>,
    arena: Vec<String>,
}

impl NameInterner {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
            arena: Vec::new(),
        }
    }

    pub fn intern(&mut self, s: &str) -> NameId {
        if let Some(&id) = self.map.get(s) {
            return id;
        }

        let id = NameId(self.arena.len() as u32);
        let owned = s.to_string();
        self.arena.push(owned.clone());
        self.map.insert(owned, id);
        id
    }

    pub fn get(&self, id: NameId) -> &str {
        &self.arena[id.0 as usize]
    }
}

// ---------- Symbol ----------
#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: NameId,
    pub type_: Ty,
    pub mut_: bool,
    pub pub_: bool,
    pub span: Span,
}

// ---------- Symbol Arena ----------
pub struct SymbolArena {
    symbols: Vec<Symbol>,
}

impl SymbolArena {
    pub fn new() -> Self {
        Self {
            symbols: Vec::new(),
        }
    }

    pub fn alloc(&mut self, sym: Symbol) -> SymbolId {
        let id = SymbolId(self.symbols.len() as u32);
        self.symbols.push(sym);
        id
    }

    pub fn get(&self, id: SymbolId) -> &Symbol {
        &self.symbols[id.0 as usize]
    }
}

// ---------- Type Arena ----------
pub struct TyArena {
    types: Vec<Ty>,
}

impl TyArena {
    pub fn new() -> Self {
        Self { types: Vec::new() }
    }

    pub fn alloc(&mut self, ty: Ty) -> TyId {
        let id = TyId(self.types.len() as u32);
        self.types.push(ty);
        id
    }

    pub fn get(&self, id: TyId) -> &Ty {
        &self.types[id.0 as usize]
    }
}

// ---------- Trait ----------
#[derive(Debug, Clone)]
pub struct Trait {
    pub name: NameId,
    pub methods: HashMap<NameId, FuncId>, // nur Signaturen hier
    pub generics: SmallVec<[Ty; 2]>,
}

// ---------- Trait Arena ----------
pub struct TraitArena {
    traits: Vec<Trait>,
}

impl TraitArena {
    pub fn new() -> Self {
        Self { traits: Vec::new() }
    }

    pub fn alloc(&mut self, t: Trait) -> TraitId {
        let id = TraitId(self.traits.len() as u32);
        self.traits.push(t);
        id
    }

    pub fn get(&self, id: TraitId) -> &Trait {
        &self.traits[id.0 as usize]
    }
}

// ---------- Scope ----------
pub struct Scope {
    pub parent: Option<ScopeId>,
    pub variables: HashMap<NameId, SymbolId>, // local vars
    pub types: HashMap<NameId, TyId>,         // structs, enums, types
    pub functions: HashMap<NameId, FuncId>,   // independent funcs
    pub traits: HashMap<NameId, TraitId>,     // user-defined traits
}

impl Scope {
    pub fn new(parent: Option<ScopeId>) -> Self {
        Self {
            parent,
            variables: HashMap::new(),
            types: HashMap::new(),
            functions: HashMap::new(),
            traits: HashMap::new(),
        }
    }
}

pub struct ScopeArena {
    scopes: Vec<Scope>,
}

impl ScopeArena {
    pub fn new() -> Self {
        Self { scopes: Vec::new() }
    }

    pub fn alloc(&mut self, parent: Option<ScopeId>) -> ScopeId {
        let id = ScopeId(self.scopes.len() as u32);
        self.scopes.push(Scope::new(parent));
        id
    }

    pub fn get(&self, id: ScopeId) -> &Scope {
        &self.scopes[id.0 as usize]
    }

    pub fn get_mut(&mut self, id: ScopeId) -> &mut Scope {
        &mut self.scopes[id.0 as usize]
    }
}

// ---------- Scope Context ----------
pub struct ScopeCtx {
    scopes: ScopeArena,
    symbols: SymbolArena, // für Variablen
    types: TyArena,       // Structs/Enums
    traits: TraitArena,   // user traits
    current: ScopeId,
}

impl ScopeCtx {
    pub fn new() -> Self {
        let mut scopes = ScopeArena::new();
        let root = scopes.alloc(None);

        Self {
            scopes,
            symbols: SymbolArena::new(),
            types: TyArena::new(),
            traits: TraitArena::new(),
            current: root,
        }
    }

    pub fn push(&mut self) {
        let parent = self.current;
        let new_scope = self.scopes.alloc(Some(parent));
        self.current = new_scope;
    }

    pub fn pop(&mut self) {
        if let Some(parent) = self.scopes.get(self.current).parent {
            self.current = parent;
        }
    }

    pub fn define_symbol(&mut self, mut sym: Symbol) -> Result<SymbolId, ()> {
        // name internieren (nur wenn sym.name ein String wäre)
        let name_id = sym.name;

        sym.name = name_id;
        let sym_id = self.symbols.alloc(sym);

        if self
            .scopes
            .get(self.current)
            .variables
            .contains_key(&name_id)
        {
            return Err(());
        }

        self.scopes
            .get_mut(self.current)
            .variables
            .insert(name_id, sym_id);
        Ok(sym_id)
    }

    pub fn define_type(&mut self, name: NameId, ty: Ty) -> Result<TyId, ()> {
        let ty_id = self.types.alloc(ty);
        let scope = self.scopes.get_mut(self.current);

        if scope.types.contains_key(&name) {
            return Err(());
        }

        scope.types.insert(name, ty_id);
        Ok(ty_id)
    }

    pub fn define_trait(&mut self, name: NameId, tr: Trait) -> Result<TraitId, ()> {
        let tr_id = self.traits.alloc(tr);
        let scope = self.scopes.get_mut(self.current);

        if scope.traits.contains_key(&name) {
            return Err(());
        }

        scope.traits.insert(name, tr_id);
        Ok(tr_id)
    }

    pub fn define_function(&mut self, name: NameId, func_id: FuncId) -> Result<(), ()> {
        let scope = self.scopes.get_mut(self.current);

        if scope.functions.contains_key(&name) {
            return Err(());
        }

        scope.functions.insert(name, func_id);
        Ok(())
    }

    pub fn lookup_symbol(&self, name: NameId) -> Option<SymbolId> {
        let mut cur = Some(self.current);

        while let Some(id) = cur {
            let scope = self.scopes.get(id);
            if let Some(&sym) = scope.variables.get(&name) {
                return Some(sym);
            }
            cur = scope.parent;
        }
        None
    }

    pub fn lookup_current(&self, name: NameId) -> Option<SymbolId> {
        self.scopes.get(self.current).variables.get(&name).copied()
    }

    pub fn lookup_type(&self, name: NameId) -> Option<TyId> {
        let mut cur = Some(self.current);
        while let Some(id) = cur {
            let scope = self.scopes.get(id);
            if let Some(&ty_id) = scope.types.get(&name) {
                return Some(ty_id);
            }
            cur = scope.parent;
        }
        None
    }

    pub fn lookup_trait(&self, name: NameId) -> Option<TraitId> {
        let mut cur = Some(self.current);
        while let Some(id) = cur {
            let scope = self.scopes.get(id);
            if let Some(&tr_id) = scope.traits.get(&name) {
                return Some(tr_id);
            }
            cur = scope.parent;
        }
        None
    }

    pub fn lookup_function(&self, name: NameId) -> Option<FuncId> {
        let mut cur = Some(self.current);
        while let Some(id) = cur {
            let scope = self.scopes.get(id);
            if let Some(&fn_id) = scope.functions.get(&name) {
                return Some(fn_id);
            }
            cur = scope.parent;
        }
        None
    }

    pub fn with_symbol<R>(&self, id: SymbolId, f: impl FnOnce(&Symbol) -> R) -> R {
        f(self.symbols.get(id))
    }

    pub fn symbol_type(&self, id: SymbolId) -> Ty {
        self.with_symbol(id, |s| s.type_)
    }

    pub fn symbol_span(&self, id: SymbolId) -> Span {
        self.with_symbol(id, |s| s.span)
    }

    pub fn is_mutable(&self, id: SymbolId) -> bool {
        self.with_symbol(id, |s| s.mut_)
    }
}

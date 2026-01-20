use crate::Compiler;

use super::token::Span;
use super::types::Ty;

use std::collections::HashMap;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct ScopeId(pub u32);

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct SymbolId(pub u32);

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
        Self { symbols: Vec::new() }
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

// ---------- Scope ----------
pub struct Scope {
    pub parent: Option<ScopeId>,
    pub symbols: HashMap<NameId, SymbolId>,
}

impl Scope {
    pub fn new(parent: Option<ScopeId>) -> Self {
        Self {
            parent,
            symbols: HashMap::new(),
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
    symbols: SymbolArena,
    current: ScopeId,
}

impl ScopeCtx {
    pub fn new() -> Self {
        let mut scopes = ScopeArena::new();
        let root = scopes.alloc(None);

        Self {
            scopes,
            symbols: SymbolArena::new(),
            current: root
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

    pub fn define(&mut self, mut sym: Symbol) -> Result<SymbolId, ()> {
        // name internieren (nur wenn sym.name ein String wäre)
        let name_id = sym.name;

        sym.name = name_id;
        let sym_id = self.symbols.alloc(sym);

        if self.scopes.get(self.current).symbols.contains_key(&name_id) {
            return Err(());
        }

        self.scopes.get_mut(self.current).symbols.insert(name_id, sym_id);
        Ok(sym_id)
    }

    pub fn lookup(&self, name: NameId) -> Option<SymbolId> {
        let mut cur = Some(self.current);

        while let Some(id) = cur {
            let scope = self.scopes.get(id);
            if let Some(&sym) = scope.symbols.get(&name) {
                return Some(sym);
            }
            cur = scope.parent;
        }
        None
    }

    pub fn lookup_current(&self, name: NameId) -> Option<SymbolId> {
        self.scopes
            .get(self.current)
            .symbols
            .get(&name)
            .copied()
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
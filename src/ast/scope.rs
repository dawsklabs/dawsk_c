use super::token::Span;
use super::types::TypeId;

use std::cell::{ Cell, RefCell };
use std::collections::HashMap;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct ScopeId(pub u32);

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct SymbolId(pub u32);

#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: &'static str,
    pub type_: TypeId,
    pub mut_: bool,
    pub pub_: bool,
    pub span: Span,
}

#[derive(Debug)]
pub struct Scope {
    pub parent: Option<ScopeId>,
    pub symbols: HashMap<&'static str, SymbolId>,
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

pub struct ScopeCtx {
    scopes: RefCell<ScopeArena>,
    symbols: RefCell<SymbolArena>,
    current: Cell<ScopeId>,
}

impl ScopeCtx {
    pub fn new() -> Self {
        let mut scopes = ScopeArena::new();
        let root = scopes.alloc(None);

        Self {
            scopes: RefCell::new(scopes),
            symbols: RefCell::new(SymbolArena::new()),
            current: Cell::new(root),
        }
    }

    pub fn push(&self) {
        let parent = self.current.get();
        let new_scope = self.scopes.borrow_mut().alloc(Some(parent));
        self.current.set(new_scope);
    }

    pub fn pop(&self) {
        let scopes = self.scopes.borrow();
        if let Some(parent) = scopes.get(self.current.get()).parent {
            self.current.set(parent);
        }
    }

    pub fn define(&self, sym: Symbol) -> Result<SymbolId, ()> {
        let mut symbols = self.symbols.borrow_mut();
        let sym_id = symbols.alloc(sym);

        let mut scopes = self.scopes.borrow_mut();
        let scope = scopes.get_mut(self.current.get());

        let name = symbols.get(sym_id).name;

        if scope.symbols.contains_key(name) {
            return Err(());
        }

        scope.symbols.insert(name, sym_id);
        Ok(sym_id)
    }

    pub fn lookup(&self, name: &str) -> Option<SymbolId> {
        let scopes = self.scopes.borrow();
        let mut cur = Some(self.current.get());

        while let Some(id) = cur {
            let scope = scopes.get(id);
            if let Some(&sym) = scope.symbols.get(name) {
                return Some(sym);
            }
            cur = scope.parent;
        }
        None
    }

    pub fn lookup_current(&self, name: &str) -> Option<SymbolId> {
        let scopes = self.scopes.borrow();
        let scope = scopes.get(self.current.get());
        scope.symbols.get(name).copied()
    }

    pub fn with_symbol<R>(&self, id: SymbolId, f: impl FnOnce(&Symbol) -> R) -> R {
        let symbols = self.symbols.borrow();
        f(symbols.get(id))
    }

    pub fn symbol_type(&self, id: SymbolId) -> TypeId {
        self.with_symbol(id, |s| s.type_)
    }

    pub fn symbol_span(&self, id: SymbolId) -> Span {
        self.with_symbol(id, |s| s.span)
    }

    pub fn is_mutable(&self, id: SymbolId) -> bool {
        self.with_symbol(id, |s| s.mut_)
    }
}

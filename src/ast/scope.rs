use super::types::TypeKind;

use fxhash::FxHashMap;

use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub type_: TypeKind,
    pub mut_: bool,
    pub vis: bool,
}

#[derive(Debug)]
pub struct Scope {
    pub symbols: RefCell<FxHashMap<String, Symbol>>,
    pub parent: Option<Rc<RefCell<Scope>>>,
}

impl Scope {
    pub fn new(parent: Option<Rc<RefCell<Scope>>>) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self {
            symbols: RefCell::new(FxHashMap::default()),
            parent,
        }))
    }
}


pub struct ScopeStack {
    current: RefCell<Rc<RefCell<Scope>>>,
}

impl ScopeStack {
    pub fn new() -> Self {
        Self {
            current: RefCell::new(Scope::new(None)),
        }
    }

    pub fn push(&self) {
        let parent = self.current.borrow().clone();
        let new_scope = Scope::new(Some(parent));
        *self.current.borrow_mut() = new_scope;
    }

    pub fn pop(&self) {
        let parent_opt = self.current.borrow().borrow().parent.clone();
        if let Some(parent) = parent_opt {
            *self.current.borrow_mut() = parent;
        }
    }

    pub fn define(&self, symbol: Symbol) -> Result<(), String> {
        let current_rc = self.current.borrow().clone();
        let scope = current_rc.borrow_mut();

        let name = symbol.name.clone();

        if scope.symbols.borrow().contains_key(&name) {
            return Err(format!("Variable '{}' already defined", name));
        }

        scope.symbols.borrow_mut().insert(name.clone(), symbol);

        Ok(())
    }

    pub fn lookup(&self, name: &str) -> Option<Symbol> {
        let mut scope = Some(self.current.borrow().clone());

        while let Some(rc) = scope {
            let inner = rc.borrow();

            if let Some(sym) = inner.symbols.borrow().get(name) {
                return Some(sym.clone());
            }

            scope = inner.parent.clone();
        }

        None
    }
}
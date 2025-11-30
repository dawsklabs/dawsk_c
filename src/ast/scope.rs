use super::types::TypeKind;
use super::eval::Value;

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
    pub values: RefCell<FxHashMap<String, Value>>, // TODO: this should be a trait
    pub parent: Option<Rc<Scope>>,
}

impl Scope {
    pub fn new(parent: Option<Rc<Scope>>) -> Rc<Self> {
        Rc::new(Self {
            symbols: RefCell::new(FxHashMap::default()),
            values: RefCell::new(FxHashMap::default()),
            parent,
        })
    }
}

pub struct ScopeStack {
    current: Rc<Scope>,
}

impl ScopeStack {
    pub fn new() -> Self {
        Self {
            current: Scope::new(None),
        }
    }

    pub fn push(&mut self) {
        self.current = Scope::new(Some(self.current.clone()));
    }

    pub fn pop(&mut self) {
        let parent = self.current.parent.clone();
        if let Some(p) = parent {
            self.current = p;
        } else {
            // cannot pop the root
        }
    }

    pub fn define(&self, symbol: Symbol, value: Value) -> Result<(), String> {
        let name = symbol.name.clone();

        let mut symbols = self.current.symbols.borrow_mut();
        let mut values  = self.current.values.borrow_mut();

        if symbols.contains_key(&name) {
            return Err(format!("Variable '{}' already defined", name));
        }

        symbols.insert(name.clone(), symbol);
        values.insert(name, value);

        Ok(())
    }

    pub fn lookup(&self, name: &str) -> Option<Symbol> {
        let mut scope = Some(self.current.clone());

        while let Some(s) = scope {
            if let Some(sym) = s.symbols.borrow().get(name) {
                return Some(sym.clone());
            }
            scope = s.parent.clone();
        }

        None
    }

    fn lookup_scope(&self, name: &str) -> Option<Rc<Scope>> {
        let mut scope = Some(self.current.clone());

        while let Some(s) = scope {
            if s.symbols.borrow().contains_key(name) {
                return Some(s);
            }
            scope = s.parent.clone();
        }

        None
    }

    pub fn lookup_value(&self, name: &str) -> Option<Value> {
        let mut scope = Some(self.current.clone());
        while let Some(s) = scope {
            if let Some(v) = s.values.borrow().get(name) {
                return Some(v.clone());
            }
            scope = s.parent.clone();
        }
        None
    }

    pub fn assign(&self, name: &str, new_value: Value) -> Result<(), String> {
        if let Some(scope) = self.lookup_scope(name) {
            let symbols_ref = scope.symbols.borrow();
            let sym = symbols_ref.get(name).unwrap();
            let mut values = scope.values.borrow_mut();

            if !sym.mut_ {
                return Err(format!("Cannot assign to immutable variable '{}'", name));
            }
            if let Some(v) = values.get_mut(name) {
                *v = new_value;
                return Ok(());
            }
        }
        Err(format!("Variable '{}' not found", name))
    }
}
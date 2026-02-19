use std::collections::HashMap;

use crate::ast::scope::NameId;
use crate::types::Ty;
use crate::source::Span;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct StructId(pub u32);

pub enum StructFields {
    Named(Vec<(NameId, Ty)>),
    Tuple(Vec<Ty>),
}

pub struct StructDecl {
    pub name: NameId,
    pub fields: Option<StructFields>,
    pub span: Span,
}

pub struct StructArena {
    structs: Vec<StructDecl>,
    lookup: HashMap<NameId, StructId>,
}

impl StructArena {
    pub fn new() -> Self {
        Self {
            structs: Vec::new(),
            lookup: HashMap::new(),
        }
    }

    pub fn alloc_placeholder(
        &mut self,
        name: NameId,
        span: Span,
    ) -> Result<StructId, ()> {
        if self.lookup.contains_key(&name) {
            return Err(()); // doppelt definiert
        }

        let id = StructId(self.structs.len() as u32);
        self.structs.push(StructDecl {
            name,
            fields: None,
            span,
        });
        self.lookup.insert(name, id);
        Ok(id)
    }

    pub fn set_fields(
        &mut self,
        id: StructId,
        fields: StructFields,
    ) {
        let decl = &mut self.structs[id.0 as usize];

        if decl.fields.is_some() {
            todo!() // doppelte Definition
        }

        decl.fields = Some(fields);
        todo!()
    }

    pub fn get(&self, id: StructId) -> &StructDecl {
        &self.structs[id.0 as usize]
    }

    pub fn lookup(&self, name: NameId) -> Option<StructId> {
        self.lookup.get(&name).copied()
    }
}

pub enum StructError {
    DuplicateDefinition { name: NameId, span: Span },
    AlreadyDefined { id: StructId },
}
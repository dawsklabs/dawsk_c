/// Eindeutige ID für eine Macro-Expansion
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExpansionId(pub u32);

/// Der syntaktische Kontext eines Tokens –
/// in welchen Macro-Expansionen wurde es erzeugt?
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyntaxContext(u32);  // Index in SyntaxContextTable

impl SyntaxContext {
    pub const ROOT: Self = Self(0);
}

pub struct SyntaxContextTable {
    contexts: Vec<SyntaxContextData>,
}

struct SyntaxContextData {
    parent: Option<SyntaxContext>,
    expansion: Option<ExpansionId>,
}

impl SyntaxContextTable {
    pub fn new() -> Self {
        // Index 0 = Root
        Self { contexts: vec![SyntaxContextData { parent: None, expansion: None }] }
    }

    pub fn push(&mut self, parent: SyntaxContext, id: ExpansionId) -> SyntaxContext {
        let idx = self.contexts.len();
        self.contexts.push(SyntaxContextData {
            parent: Some(parent),
            expansion: Some(id),
        });
        SyntaxContext(idx as u32)
    }
}

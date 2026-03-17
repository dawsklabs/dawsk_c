use std::collections::HashMap;
use std::fmt::{Formatter, Result};
use std::sync::Arc;

/// Eindeutige ID für einen interned String
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct StringId(u32);

impl StringId {
    pub const EMPTY: StringId = StringId(0);
}

impl std::fmt::Display for StringId {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "StringId({})", self.0)
    }
}

/// String-Pool: internt alle Strings, gibt nur IDs zurück
pub struct StringPool {
    /// Map von String → StringId (schneller Lookup)
    lookup: HashMap<Arc<str>, StringId>,
    /// Vec von StringId → String (reverse lookup für Debugging)
    arena: Vec<Arc<str>>,
}

impl StringPool {
    pub fn new() -> Self {
        let mut pool = Self {
            lookup: HashMap::new(),
            arena: Vec::new(),
        };
        // Reserve StringId(0) für empty string
        pool.arena.push(Arc::from(""));
        pool.lookup.insert(Arc::from(""), StringId::EMPTY);
        pool
    }

    /// Interned einen String oder gibt existierende ID zurück
    pub fn intern(&mut self, s: impl AsRef<str>) -> StringId {
        let s = s.as_ref();

        // Früher Return falls schon vorhanden
        if let Some(&id) = self.lookup.get(s) {
            return id;
        }

        // Neu hinzufügen
        let id = StringId(self.arena.len() as u32);
        let arc_str: Arc<str> = Arc::from(s);
        self.arena.push(arc_str.clone());
        self.lookup.insert(arc_str, id);

        id
    }

    /// Ruft den String für eine ID auf
    pub fn get(&self, id: StringId) -> Option<&str> {
        self.arena.get(id.0 as usize).map(|s| s.as_ref())
    }
}

impl Default for StringPool {
    fn default() -> Self {
        Self::new()
    }
}

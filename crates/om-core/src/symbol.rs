//! Compact shared symbol identifiers; names are kept for the process lifetime.

use crate::builtins::NAMES;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::sync::{OnceLock, RwLock};

/// Interned symbol identifier. ID ordering is only for containers, not printing.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Debug)]
pub struct Symbol(u32);

impl Symbol {
    /// Intern a symbol, sharing its identifier across threads and expressions.
    pub fn intern(name: &str) -> Self {
        let table = interner();
        if let Some(&symbol) = table
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .by_name
            .get(name)
        {
            return symbol;
        }
        let mut table = table.write().unwrap_or_else(|e| e.into_inner());
        // Another writer may have interned this name between the read and write locks.
        if let Some(&symbol) = table.by_name.get(name) {
            return symbol;
        }
        let id = u32::try_from(table.names.len()).expect("invariant: symbol table fits u32 IDs");
        let name = Box::leak(name.to_owned().into_boxed_str());
        let symbol = Self(id);
        table.names.push(name);
        table.by_name.insert(name, symbol);
        symbol
    }
    /// The process-lifetime symbol name.
    pub fn name(self) -> &'static str {
        interner().read().unwrap_or_else(|e| e.into_inner()).names[self.0 as usize]
    }
    pub(crate) const fn builtin(id: u32) -> Self {
        Self(id)
    }
}

struct Interner {
    names: Vec<&'static str>,
    by_name: FxHashMap<&'static str, Symbol>,
}
fn interner() -> &'static RwLock<Interner> {
    static INTERNER: OnceLock<RwLock<Interner>> = OnceLock::new();
    INTERNER.get_or_init(|| {
        let names = NAMES.to_vec();
        let by_name = NAMES
            .iter()
            .enumerate()
            .map(|(id, &name)| (name, Symbol::builtin(id as u32)))
            .collect();
        RwLock::new(Interner { names, by_name })
    })
}

impl Serialize for Symbol {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.name())
    }
}
impl<'de> Deserialize<'de> for Symbol {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer).map(|name| Self::intern(&name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BUILTIN, builtins::NAMES};
    #[test]
    fn repeated_names_share_ids_and_different_names_do_not() {
        assert_eq!(Symbol::intern("Plus"), Symbol::intern("Plus"));
        assert_ne!(Symbol::intern("x"), Symbol::intern("X"));
        assert_eq!(Symbol::intern("α").name(), "α");
        assert_eq!(Symbol::intern("").name(), "");
    }
    #[test]
    fn all_builtins_keep_declared_ids_names_and_order() {
        assert_eq!(BUILTIN::PLUS.name(), "Plus");
        for (id, &name) in NAMES.iter().enumerate() {
            let symbol = Symbol::builtin(id as u32);
            assert_eq!(symbol.name(), name);
            assert_eq!(Symbol::intern(name), symbol);
        }
        assert_eq!(Symbol::intern("List"), Symbol::builtin(0));
        assert_eq!(Symbol::intern("Plus"), Symbol::builtin(1));
        assert_eq!(
            Symbol::intern("Length"),
            Symbol::builtin((NAMES.len() - 1) as u32)
        );
    }
    #[test]
    fn symbol_serde_uses_names_instead_of_process_local_ids() {
        for name in ["Plus", "persisted_α", "quoted\"name"] {
            let symbol = Symbol::intern(name);
            let json = serde_json::to_string(&symbol).unwrap();
            assert_eq!(json, serde_json::to_string(name).unwrap());
            assert_eq!(symbol, serde_json::from_str::<Symbol>(&json).unwrap());
        }
    }
    #[test]
    fn concurrent_interning_has_no_duplicate_or_missing_symbols() {
        use std::sync::{Arc, Barrier};
        let barrier = Arc::new(Barrier::new(2));
        let threads: Vec<_> = (0..2)
            .map(|_| {
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    (0..100)
                        .map(|i| Symbol::intern(&format!("shared_concurrent_{i}")))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        let results: Vec<_> = threads.into_iter().map(|t| t.join().unwrap()).collect();
        assert_eq!(results[0], results[1]);
        let unique: std::collections::BTreeSet<_> = results[0].iter().collect();
        assert_eq!(unique.len(), 100);
        for (i, symbol) in results[0].iter().enumerate() {
            assert_eq!(symbol.name(), format!("shared_concurrent_{i}"));
        }
    }
}

use std::collections::HashMap;
use std::collections::hash_map::{Entry, Iter};
use std::sync::RwLock;

pub fn get<E: Clone + 'static>(
    singleton_fn: impl FnOnce() -> &'static RwLock<SymbolTable<E>>,
    name: &str,
) -> Option<E> {
    with_global_symbol_table(singleton_fn, |table| table.get_entry_cloned(name))
}

pub fn insert<E: Clone + 'static>(
    singleton_fn: impl FnOnce() -> &'static RwLock<SymbolTable<E>>,
    name: impl Into<String>,
    new_entry: E,
) -> Option<E> {
    with_global_symbol_table_mut(singleton_fn, |table| table.insert(name, new_entry))
}

pub fn clear<E: Clone + 'static>(singleton_fn: impl FnOnce() -> &'static RwLock<SymbolTable<E>>) {
    with_global_symbol_table_mut(singleton_fn, |table| table.clear());
}

pub fn with_global_symbol_table<T, E: Clone + 'static>(
    singleton_fn: impl FnOnce() -> &'static RwLock<SymbolTable<E>>,
    f: impl FnOnce(&SymbolTable<E>) -> T,
) -> T {
    let table = singleton_fn()
        .read()
        .expect("Global symbol table lock poisoned");
    f(&table)
}

pub fn with_global_symbol_table_mut<T, E: Clone + 'static>(
    singleton_fn: impl FnOnce() -> &'static RwLock<SymbolTable<E>>,
    f: impl FnOnce(&mut SymbolTable<E>) -> T,
) -> T {
    let mut table = singleton_fn()
        .write()
        .expect("Global symbol table lock poisoned");
    f(&mut table)
}

#[derive(Debug, Default)]
pub struct SymbolTable<E>
where
    E: Clone,
{
    entries: HashMap<String, E>,
}

impl<E: Clone> SymbolTable<E> {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub fn insert(&mut self, name: impl Into<String>, new_entry: E) -> Option<E> {
        self.entries.insert(name.into(), new_entry)
    }

    pub fn get_entry(&self, name: &str) -> Option<&E> {
        self.entries.get(name)
    }

    pub fn get_entry_cloned(&self, name: &str) -> Option<E>
    where
        E: Clone,
    {
        self.entries.get(name).cloned()
    }

    pub fn get_all_entries(&self) -> Iter<'_, String, E> {
        self.entries.iter()
    }

    pub fn modify(&mut self, name: &str) -> Entry<'_, String, E> {
        self.entries.entry(name.to_string())
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

use std::collections::hash_map::{Entry, Iter};
use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};
use crate::common::Type;

static SYMBOL_TABLE: OnceLock<RwLock<SymbolTable>> = OnceLock::new();

pub fn get(name: &str) -> Option<SymbolTableEntry> {
    with_global_symbol_table(|table| table.get_entry_cloned(name))
}

pub fn insert(name: impl Into<String>, new_entry: SymbolTableEntry) -> Option<SymbolTableEntry> {
    with_global_symbol_table_mut(|table| table.insert(name, new_entry))
}

pub fn clear() {
    with_global_symbol_table_mut(|table| table.clear());
}   

pub fn global_symbol_table() -> &'static RwLock<SymbolTable> {
    SYMBOL_TABLE.get_or_init(|| RwLock::new(SymbolTable::new()))
}

pub fn with_global_symbol_table<T>(f: impl FnOnce(&SymbolTable) -> T) -> T {
    let table = global_symbol_table()
        .read()
        .expect("Global symbol table lock poisoned");
    f(&table)
}

pub fn with_global_symbol_table_mut<T>(f: impl FnOnce(&mut SymbolTable) -> T) -> T {
    let mut table = global_symbol_table()
        .write()
        .expect("Global symbol table lock poisoned");
    f(&mut table)
}

#[derive(Debug, Default)]
pub struct SymbolTable {
    entries: HashMap<String, SymbolTableEntry>,
}

impl SymbolTable {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub fn insert(&mut self, name: impl Into<String>, new_entry: SymbolTableEntry) -> Option<SymbolTableEntry> {
        self.entries.insert(name.into(), new_entry)
    }

    pub fn get_entry(&self, name: &str) -> Option<&SymbolTableEntry> {
        self.entries.get(name)
    }

    pub fn get_entry_cloned(&self, name: &str) -> Option<SymbolTableEntry> {
        self.entries.get(name).cloned()
    }

    pub fn get_all_entries(&self) -> Iter<'_, String, SymbolTableEntry> {
        self.entries.iter()
    }

    pub fn modify(&mut self, name: &str) -> Entry<'_, String, SymbolTableEntry> {
        self.entries.entry(name.to_string())
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

#[derive(Debug, Clone)]
pub struct SymbolTableEntry {
    pub c_type: Type,
    pub attrs: IdentAttrs,
}

#[derive(Debug, Clone)]
pub enum IdentAttrs {
    Function {
        is_defined: bool,
        is_global: bool,
    },
    Static {
        init_value: Option<InitialValue>,
        is_global: bool,
    },
    Local,
}

#[derive(Debug, Clone)]
pub enum InitialValue {
    Tentative,
    Initialized(InitValue),
}

#[derive(Debug, Clone)]
pub enum InitValue {
    Int(i32),
    Long(i64),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_table_insert_and_get() {
        let mut table = SymbolTable::new();
        table.insert("main", SymbolTableEntry{
            c_type: Type::Function {
                return_type: Box::new(Type::Int),
                param_types: vec![],
            },
            attrs: IdentAttrs::Function { is_defined: true, is_global: true },
        });

        assert_eq!(
            Some(&Type::Function {
                return_type: Box::new(Type::Int),
                param_types: vec![],
            }),
            table.get_entry("main").map(|entry| &entry.c_type)
        );
    }

    #[test]
    fn global_table_is_singleton() {
        with_global_symbol_table_mut(|table| {
            table.clear();
            table.insert("x", SymbolTableEntry {
                c_type: Type::Int,
                attrs: IdentAttrs::Local,
            });
        });

        let loaded = with_global_symbol_table(|table| table.get_entry_cloned("x"));
        assert_eq!(Some(Type::Int), loaded.map(|entry| entry.c_type));
    }
}

use std::sync::{OnceLock, RwLock};
use crate::assembly::ast::AssemblyType;
use crate::common::symbol_table_generic;
use crate::common::symbol_table_generic::SymbolTable as GenericSymbolTable;

pub type SymbolTable = GenericSymbolTable<SymbolTableEntry>;

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
    symbol_table_generic::with_global_symbol_table(global_symbol_table, f)
}

pub fn with_global_symbol_table_mut<T>(f: impl FnOnce(&mut SymbolTable) -> T) -> T {
    symbol_table_generic::with_global_symbol_table_mut(global_symbol_table, f)
}

#[derive(Debug, Clone, PartialEq)]  // PartialEq für assert_eq! nötig
pub enum SymbolTableEntry {
    Object {
        assembly_type: AssemblyType,
        is_static: bool,
    },
    Function {
        is_defined: bool,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assembly::ast::AssemblyType;

    #[test]
    fn insert_and_get_returns_inserted_entry() {
        clear();

        let entry = SymbolTableEntry::Object {
            assembly_type: AssemblyType::Longword,
            is_static: false,
        };
        insert("tmp.0", entry.clone());

        assert_eq!(get("tmp.0"), Some(entry));
    }

    #[test]
    fn get_returns_none_for_unknown_key() {
        clear();

        assert_eq!(get("does_not_exist"), None);
    }

    #[test]
    fn clear_removes_all_entries() {
        clear();
        insert("a", SymbolTableEntry::Function { is_defined: true });
        insert("b", SymbolTableEntry::Object {
            assembly_type: AssemblyType::Quadword,
            is_static: true,
        });

        clear();

        assert_eq!(get("a"), None);
        assert_eq!(get("b"), None);
    }

    #[test]
    fn insert_overwrites_existing_entry_for_same_key() {
        clear();
        insert("tmp.1", SymbolTableEntry::Function { is_defined: false });

        let updated = SymbolTableEntry::Function { is_defined: true };
        insert("tmp.1", updated.clone());

        assert_eq!(get("tmp.1"), Some(updated));
    }
}

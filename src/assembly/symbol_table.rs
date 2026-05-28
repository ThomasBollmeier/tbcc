use crate::assembly::ast::AssemblyType;

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
    use crate::common::symbol_table_generic::{SymbolTable, SymbolTableRef};

    #[test]
    fn insert_and_get_returns_inserted_entry() {
        let symbol_table: SymbolTableRef<SymbolTableEntry> = SymbolTable::new_ref();

        let entry = SymbolTableEntry::Object {
            assembly_type: AssemblyType::Longword,
            is_static: false,
        };
        symbol_table.borrow_mut().insert("tmp.0", entry.clone());

        assert_eq!(symbol_table.borrow().get_entry("tmp.0"), Some(&entry));
    }

    #[test]
    fn get_returns_none_for_unknown_key() {
        let symbol_table: SymbolTableRef<SymbolTableEntry> = SymbolTable::new_ref();

        assert_eq!(symbol_table.borrow().get_entry("does_not_exist"), None);
    }

    #[test]
    fn clear_removes_all_entries() {
        let symbol_table: SymbolTableRef<SymbolTableEntry> = SymbolTable::new_ref();

        symbol_table.borrow_mut().insert("a", SymbolTableEntry::Function { is_defined: true });
        symbol_table.borrow_mut().insert("b", SymbolTableEntry::Object {
            assembly_type: AssemblyType::Quadword,
            is_static: true,
        });

        symbol_table.borrow_mut().clear();

        assert_eq!(symbol_table.borrow().get_entry("a"), None);
        assert_eq!(symbol_table.borrow().get_entry("b"), None);
    }

    #[test]
    fn insert_overwrites_existing_entry_for_same_key() {
        let symbol_table: SymbolTableRef<SymbolTableEntry> = SymbolTable::new_ref();

        symbol_table.borrow_mut().insert("tmp.1", SymbolTableEntry::Function { is_defined: false });

        let updated = SymbolTableEntry::Function { is_defined: true };
        symbol_table.borrow_mut().insert("tmp.1", updated.clone());

        assert_eq!(symbol_table.borrow().get_entry("tmp.1"), Some(&updated));
    }
}

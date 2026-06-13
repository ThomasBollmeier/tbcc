use crate::common::Type;

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
    UInt(u32),
    Long(i64),
    ULong(u64),
}

#[cfg(test)]
mod tests {
    use crate::common::symbol_table_generic::SymbolTable;
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
}

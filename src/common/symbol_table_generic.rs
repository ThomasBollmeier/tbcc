use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::hash_map::{Entry, Iter};
use std::rc::Rc;

pub type SymbolTableRef<E> = Rc<RefCell<SymbolTable<E>>>;

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

    pub fn new_ref() -> SymbolTableRef<E> {
        Rc::new(RefCell::new(SymbolTable::new()))
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

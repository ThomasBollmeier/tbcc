use crate::semantic::name_generator::NameGeneratorRef;
use anyhow::{Result, anyhow};
use std::cell::RefCell;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::rc::Rc;

pub struct Scope<T=NoAdditionalData> {
    parent: Option<ScopeRef<T>>,
    name_generator: NameGeneratorRef,
    names: HashMap<String, NamingData<T>>,
}

pub type ScopeRef<T=NoAdditionalData> = Rc<RefCell<Scope<T>>>;

impl<'a, T: Default> Scope<T> {
    pub fn new(parent: Option<ScopeRef<T>>, name_generator: NameGeneratorRef) -> Scope<T> {
        Scope {
            parent,
            name_generator,
            names: HashMap::new(),
        }
    }

    pub fn new_ref(parent: Option<ScopeRef<T>>, name_generator: NameGeneratorRef) -> ScopeRef<T> {
        Rc::new(RefCell::new(Scope::new(parent, name_generator)))
    }

    pub fn get_parent(&self) -> Option<ScopeRef<T>> {
        self.parent.clone()
    }

    pub fn get_unique_name(&self, name: &str) -> Option<String> {
        if let Some(naming) = self.names.get(name) {
            return Some(naming.unique_name.clone());
        }

        match &self.parent {
            Some(parent) => parent.borrow().get_unique_name(name),
            None => None,
        }
    }

    pub fn get_names_in_current_scope(&self) -> Vec<String> {
        self.names.keys().cloned().collect()
    }

    pub fn is_in_current_scope(&self, name: &str) -> bool {
        self.names.contains_key(name)
    }

    pub fn get_current_info_mut(&mut self, name: &str) -> Entry<'_, String, NamingData<T>> {
        self.names.entry(name.to_string())
    }

    pub fn get_current_info(&self, name: &str) -> Option<&NamingData<T>> {
        self.names.get(name)
    }

    pub fn add_full(&mut self, name: &str, additional_data: T) -> Result<String> {
        if self.is_in_current_scope(name) {
            return Err(anyhow!("'{name}' already exists in current scope"));
        }

        let unique_name = self.name_generator.borrow_mut().make_unique_name(name);
        self.names.insert(
            name.to_string(),
            NamingData {
                unique_name: unique_name.clone(),
                additional: additional_data,
            },
        );

        Ok(unique_name)
    }

    pub fn add(&mut self, name: &str) -> Result<String> {
        self.add_full(name, T::default())
    }
}

pub struct NamingData<T> {
    pub unique_name: String,
    pub additional: T,
}

#[derive(Debug, Default)]
pub struct NoAdditionalData;

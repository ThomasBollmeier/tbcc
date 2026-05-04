use crate::semantic::name_generator::NameGeneratorRef;
use anyhow::Result;
use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::rc::Rc;

pub struct Scope<T = NoAdditionalData>
where
    T: Clone,
{
    parent: Option<ScopeRef<T>>,
    name_generator: NameGeneratorRef,
    names: HashMap<String, NamingData<T>>,
    pub strategy: Rc<dyn ResolutionStrategy<T>>,
}

pub type ScopeRef<T = NoAdditionalData> = Rc<RefCell<Scope<T>>>;

impl<'a, T: Clone> Scope<T> {
    pub fn new(
        parent: Option<ScopeRef<T>>,
        name_generator: NameGeneratorRef,
        strategy: Rc<dyn ResolutionStrategy<T>>,
    ) -> Scope<T> {
        Scope {
            parent,
            name_generator,
            names: HashMap::new(),
            strategy,
        }
    }

    pub fn new_ref(
        parent: Option<ScopeRef<T>>,
        name_generator: NameGeneratorRef,
        strategy: Rc<dyn ResolutionStrategy<T>>,
    ) -> ScopeRef<T> {
        Rc::new(RefCell::new(Scope::new(parent, name_generator, strategy)))
    }

    pub fn new_child(parent: &ScopeRef<T>) -> ScopeRef<T> {
        let name_generator = parent.borrow().name_generator.clone();
        let strategy = parent.borrow().strategy.clone();
        Scope::new_ref(Some(parent.clone()), name_generator, strategy)
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

    fn get_naming_data(&self, name: &str) -> (Option<T>, bool) {
        if let Some(naming) = self.names.get(name) {
            return (Some(naming.clone().additional), true);
        }

        let mut scope = self.parent.clone();
        while let Some(current_scope) = scope {
            let current_scope = current_scope.borrow();
            if let Some(naming) = current_scope.names.get(name) {
                return (Some(naming.clone().additional), false);
            }
            scope = current_scope.parent.clone();
        }

        (None, false)
    }

    pub fn get_names_in_current_scope(&self) -> Vec<String> {
        self.names.keys().cloned().collect()
    }

    pub fn get_current_info_mut(&mut self, name: &str) -> Entry<'_, String, NamingData<T>> {
        self.names.entry(name.to_string())
    }

    pub fn get_current_info(&self, name: &str) -> Option<&NamingData<T>> {
        self.names.get(name)
    }

    pub fn add(&mut self, name: &str, additional_data: T) -> Result<String> {
        let (existing_data_opt, exists_in_current_scope) = self.get_naming_data(name);

        self.strategy.check_add_name_to_scope(
            name,
            &existing_data_opt,
            exists_in_current_scope,
            &additional_data,
        )?;

        let unique_name = self.strategy.create_unique_name(
            name,
            &existing_data_opt,
            exists_in_current_scope,
            &additional_data,
            self.name_generator.clone(),
        )?;

        let new_entry = NamingData {
            unique_name: unique_name.clone(),
            additional: additional_data,
        };

        self.names.insert(name.to_string(), new_entry);

        Ok(unique_name)
    }
}

#[derive(Clone)]
pub struct NamingData<T: Clone> {
    pub unique_name: String,
    pub additional: T,
}

#[derive(Debug, Default, Clone)]
pub struct NoAdditionalData;

pub trait ResolutionStrategy<T: Clone> {
    fn check_add_name_to_scope(
        &self,
        name: &str,
        existing_entry: &Option<T>,
        exists_in_current_scope: bool,
        new_additional_data: &T,
    ) -> Result<()>;

    fn create_unique_name(
        &self,
        name: &str,
        existing_entry: &Option<T>,
        exists_in_current_scope: bool,
        new_additional_data: &T,
        name_generator: NameGeneratorRef,
    ) -> Result<String>;
}

use crate::semantic::name_creator::NameCreatorRef;
use anyhow::{Result, anyhow};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub struct Scope {
    parent: Option<ScopeRef>,
    name_creator: NameCreatorRef,
    vars: HashMap<String, VarInfo>,
}

pub type ScopeRef = Rc<RefCell<Scope>>;

impl<'a> Scope {
    pub fn new(parent: Option<ScopeRef>, name_creator: NameCreatorRef) -> Scope {
        Scope {
            parent,
            name_creator,
            vars: HashMap::new(),
        }
    }

    pub fn new_ref(parent: Option<ScopeRef>, name_creator: NameCreatorRef) -> ScopeRef {
        Rc::new(RefCell::new(Scope::new(parent, name_creator)))
    }

    pub fn get_parent(&self) -> Option<ScopeRef> {
        self.parent.clone()
    }

    pub fn get_var_unique_name(&self, name: &str) -> Option<String> {
        if let Some(var) = self.vars.get(name) {
            return Some(var.unique_name.clone());
        }

        match &self.parent {
            Some(parent) => parent.borrow().get_var_unique_name(name),
            None => None,
        }
    }

    pub fn is_var_in_current_scope(&self, name: &str) -> bool {
        self.get_var_unique_name(name).is_some()
    }

    pub fn add_var(&mut self, name: &str) -> Result<String> {
        if self.is_var_in_current_scope(name) {
            return Err(anyhow!("Variable `{name}` already exists in current scope"));
        }

        let unique_name = self.name_creator.borrow_mut().make_var_name(name);
        self.vars.insert(
            name.to_string(),
            VarInfo {
                unique_name: unique_name.clone(),
            },
        );

        Ok(unique_name)
    }
}

struct VarInfo {
    unique_name: String,
}

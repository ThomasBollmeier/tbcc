use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub struct NameCreator {
    vars: HashMap<String, usize>,
    cnt_tmp_vars: usize,
}

pub type NameCreatorRef = Rc<RefCell<NameCreator>>;

impl NameCreator {
    pub fn new() -> Self {
        Self {
            vars: HashMap::new(),
            cnt_tmp_vars: 0,
        }
    }

    pub fn new_ref() -> NameCreatorRef {
        Rc::new(RefCell::new(NameCreator {
            vars: HashMap::new(),
            cnt_tmp_vars: 0,
        }))
    }

    pub fn make_var_name(&mut self, name: &str) -> String {
        let cnt = self.vars.entry(name.to_string()).or_insert(0);
        let ret = format!("var.{name}.{cnt}");
        *cnt += 1;
        ret
    }

    pub fn make_temp_var_name(&mut self) -> String {
        let ret = format!("tmp.{}", self.cnt_tmp_vars);
        self.cnt_tmp_vars += 1;
        ret
    }
}

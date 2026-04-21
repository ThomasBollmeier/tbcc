use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub type FormatterFn = dyn Fn(&str, usize) -> String;

pub struct NameGenerator {
    names: HashMap<String, usize>,
    formatter: Box<FormatterFn>,
}

pub type NameGeneratorRef = Rc<RefCell<NameGenerator>>;

impl NameGenerator {
    pub fn new(formatter: Box<FormatterFn>) -> Self {
        Self {
            names: HashMap::new(),
            formatter,
        }
    }

    pub fn new_ref(formatter: Box<FormatterFn>) -> NameGeneratorRef {
        Rc::new(RefCell::new(NameGenerator::new(formatter)))
    }

    pub fn make_unique_name(&mut self, name: &str) -> String {
        let cnt = self.names.entry(name.to_string()).or_insert(0);
        let ret = (self.formatter)(name, *cnt);
        *cnt += 1;
        ret
    }
}

pub fn make_var_name_generator() -> NameGeneratorRef {
    Rc::new(RefCell::new(NameGenerator::new(Box::new(|name, cnt| 
        format!("var.{name}.{cnt}") 
    ))))
}

pub fn make_temp_var_name_generator() -> NameGeneratorRef {
    Rc::new(RefCell::new(NameGenerator::new(Box::new(|_, cnt| 
        format!("tmp.{cnt}") 
    ))))
}

pub fn make_label_name_generator() -> NameGeneratorRef {
    Rc::new(RefCell::new(NameGenerator::new(Box::new(|name, cnt|
        format!("{name}_{cnt}")
    ))))
}

pub fn make_loop_id_generator() -> NameGeneratorRef {
    Rc::new(RefCell::new(NameGenerator::new(Box::new(|_, cnt|
        format!("loop.{cnt}")
    ))))
}

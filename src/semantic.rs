mod label_resolver;
mod name_generator;
mod scope;
mod variable_resolver;

pub use label_resolver::LabelResolver;
pub use name_generator::{
    NameGeneratorRef, make_label_name_generator, make_temp_var_name_generator,
    make_var_name_generator,
};
pub use variable_resolver::VariableResolver;

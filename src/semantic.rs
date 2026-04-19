use crate::ast::Program;
use anyhow::Result;

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

pub fn validate(
    var_name_generator: &NameGeneratorRef,
    label_name_generator: &NameGeneratorRef,
    program: &mut Program,
) -> Result<()> {
    let mut variable_resolver = VariableResolver::new(var_name_generator.clone());
    variable_resolver.resolve(program)?;

    let mut label_resolver = LabelResolver::new(label_name_generator.clone());
    label_resolver.resolve(program)?;

    Ok(())
}

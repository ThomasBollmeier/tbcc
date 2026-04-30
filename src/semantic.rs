use crate::ast::Program;
use anyhow::Result;

mod walker;
mod label_resolver;
mod name_generator;
mod scope;
mod identifier_resolver;
mod loop_labeler;
pub mod symbol_table;
mod type_checker;

pub use label_resolver::LabelResolver;
pub use name_generator::{
    NameGeneratorRef, make_label_name_generator, make_temp_var_name_generator,
    make_var_name_generator,
};
pub use identifier_resolver::IdentifierResolver;
use crate::semantic::loop_labeler::LoopLabeler;

pub fn validate(
    var_name_generator: &NameGeneratorRef,
    label_name_generator: &NameGeneratorRef,
    program: &mut Program,
) -> Result<()> {
    symbol_table::clear();

    let mut variable_resolver = IdentifierResolver::new(var_name_generator.clone());
    variable_resolver.resolve(program)?;
    
    let mut type_checker = type_checker::TypeChecker::new();
    type_checker.check(program)?;

    let mut label_resolver = LabelResolver::new(label_name_generator.clone());
    label_resolver.resolve(program)?;

    let mut loop_labeler = LoopLabeler::new();
    loop_labeler.label_loops(program)?;

    Ok(())
}

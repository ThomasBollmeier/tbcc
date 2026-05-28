use crate::ast::Program;
use anyhow::Result;

mod walker;
pub(crate) mod visitor;
mod label_resolver;
mod name_generator;
mod scope;
mod identifier_resolver;
mod loop_labeler;
//mod type_checker_old;
mod type_checker;

use crate::common::symbol_table::SymbolTableEntry;
use crate::common::symbol_table_generic::SymbolTableRef;
use crate::semantic::loop_labeler::LoopLabeler;
pub use identifier_resolver::IdentifierResolver;
pub use label_resolver::LabelResolver;
pub use name_generator::{
    make_label_name_generator, make_temp_var_name_generator, make_var_name_generator,
    NameGeneratorRef,
};

pub fn validate(
    var_name_generator: &NameGeneratorRef,
    label_name_generator: &NameGeneratorRef,
    symbol_table: SymbolTableRef<SymbolTableEntry>,
    program: &mut Program,
) -> Result<()> {

    let mut variable_resolver = IdentifierResolver::new(var_name_generator.clone());
    variable_resolver.resolve(program)?;
    
    let mut type_checker = type_checker::TypeChecker::new(symbol_table.clone());
    type_checker.check(program)?;

    let mut label_resolver = LabelResolver::new(label_name_generator.clone());
    label_resolver.resolve(program)?;

    let mut loop_labeler = LoopLabeler::new();
    loop_labeler.label_loops(program)?;

    Ok(())
}

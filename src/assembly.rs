use crate::assembly::symbol_table::SymbolTableEntry as AsmSymbolTableEntry;
use crate::assembly::assembly_creator::AssemblyCreator;
use crate::assembly::instruction_fixer::InstructionFixer;
use crate::common::symbol_table::SymbolTableEntry;
use crate::common::symbol_table_generic::SymbolTableRef;
use anyhow::Result;
use crate::semantic::NameGeneratorRef;

mod assembly_creator;
pub mod ast;
pub mod codegen;
mod instruction_fixer;
mod pseudo_reg_replacer;
mod symbol_table;

pub fn create_program(
    tacky_program: &crate::tacky::ast::Program,
    symbol_table: SymbolTableRef<SymbolTableEntry>,
    label_name_generator: NameGeneratorRef,
) -> Result<(ast::Program, SymbolTableRef<AsmSymbolTableEntry>)> {
    let mut assembly_creator = AssemblyCreator::new(symbol_table.clone(), label_name_generator);
    let (mut asm_program, asm_symbol_table) = assembly_creator.create_program(tacky_program)?;

    let mut pseudo_reg_replacer = pseudo_reg_replacer::PseudoRegReplacer::new(asm_symbol_table.clone());
    asm_program.walk_mut(&mut pseudo_reg_replacer);

    let mut instruction_fixer = InstructionFixer::new();
    asm_program.walk_mut(&mut instruction_fixer);

    Ok((asm_program, asm_symbol_table))
}

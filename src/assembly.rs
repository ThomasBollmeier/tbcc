use crate::assembly::assembly_creator::AssemblyCreator;
use crate::assembly::instruction_fixer::InstructionFixer;
use anyhow::Result;

mod assembly_creator;
pub mod ast;
mod instruction_fixer;
mod pseudo_reg_replacer;

pub fn create_program(tacky_program: &crate::tacky::Program) -> Result<ast::Program> {
    let mut assembly_creator = AssemblyCreator::new();
    let mut asm_program = assembly_creator.create_program(tacky_program)?;

    let mut pseudo_reg_replacer = pseudo_reg_replacer::PseudoRegReplacer::new();
    asm_program.walk_mut(&mut pseudo_reg_replacer);

    let mut instruction_fixer = InstructionFixer::new(pseudo_reg_replacer.get_frame_size());
    asm_program.walk_mut(&mut instruction_fixer);

    Ok(asm_program)
}

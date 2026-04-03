use crate::assembly::ast::VisitorMut;

pub struct InstructionFixer {
    stack_frame_size: usize,
}

impl InstructionFixer {
    pub fn new(stack_frame_size: usize) -> InstructionFixer {
        InstructionFixer { stack_frame_size }
    }
}

impl VisitorMut for InstructionFixer {
    fn enter_func_def(&mut self, func_def: &mut crate::assembly::ast::FuncDef) {
        use crate::assembly::ast::{Instruction::*, Operand::*, Register::*};

        let mut new_instructions = vec![];

        // Allocate stack space at the beginning of the function
        new_instructions.push(AllocateStack(self.stack_frame_size as i32));

        for instruction in &func_def.instructions {
            match instruction {
                Mov { src, dst } => match (src, dst) {
                    (Stack(_), Stack(_)) => {
                        new_instructions.push(Mov {
                            src: src.clone(),
                            dst: Register(R10),
                        });
                        new_instructions.push(Mov {
                            src: Register(R10),
                            dst: dst.clone(),
                        });
                    }
                    _ => new_instructions.push(instruction.clone()),
                },
                _ => new_instructions.push(instruction.clone()),
            }
        }

        func_def.instructions = new_instructions;
    }
}

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

#[cfg(test)]
mod tests {
    use super::InstructionFixer;
    use crate::assembly::ast::{FuncDef, Instruction, Operand, Program, Register, UnaryOp};

    fn apply_fixer(instructions: Vec<Instruction>, stack_frame_size: usize) -> Vec<Instruction> {
        let mut program = Program::new(FuncDef::new("main".to_string(), instructions));
        let mut fixer = InstructionFixer::new(stack_frame_size);
        program.walk(&mut fixer);
        program.0.instructions
    }

    #[test]
    fn inserts_allocate_stack_as_first_instruction() {
        let instructions = vec![Instruction::Ret];

        let fixed = apply_fixer(instructions, 16);

        assert!(matches!(fixed[0], Instruction::AllocateStack(16)));
        assert!(matches!(fixed[1], Instruction::Ret));
    }

    #[test]
    fn rewrites_stack_to_stack_mov_into_two_movs_via_r10() {
        let instructions = vec![Instruction::Mov {
            src: Operand::Stack(-4),
            dst: Operand::Stack(-8),
        }];

        let fixed = apply_fixer(instructions, 8);

        assert!(matches!(fixed[0], Instruction::AllocateStack(8)));

        match &fixed[1] {
            Instruction::Mov { src, dst } => {
                assert!(matches!(src, Operand::Stack(-4)));
                assert!(matches!(dst, Operand::Register(Register::R10)));
            }
            _ => panic!("expected first rewritten mov"),
        }

        match &fixed[2] {
            Instruction::Mov { src, dst } => {
                assert!(matches!(src, Operand::Register(Register::R10)));
                assert!(matches!(dst, Operand::Stack(-8)));
            }
            _ => panic!("expected second rewritten mov"),
        }
    }

    #[test]
    fn keeps_non_stack_to_stack_instructions_in_order() {
        let instructions = vec![
            Instruction::Mov {
                src: Operand::Immediate(1),
                dst: Operand::Register(Register::AX),
            },
            Instruction::Unary {
                op: UnaryOp::Neg,
                operand: Operand::Register(Register::AX),
            },
            Instruction::Ret,
        ];

        let fixed = apply_fixer(instructions, 0);

        assert_eq!(fixed.len(), 4);
        assert!(matches!(fixed[0], Instruction::AllocateStack(0)));

        match &fixed[1] {
            Instruction::Mov { src, dst } => {
                assert!(matches!(src, Operand::Immediate(1)));
                assert!(matches!(dst, Operand::Register(Register::AX)));
            }
            _ => panic!("expected mov"),
        }

        match &fixed[2] {
            Instruction::Unary { op, operand } => {
                assert_eq!(*op, UnaryOp::Neg);
                assert!(matches!(operand, Operand::Register(Register::AX)));
            }
            _ => panic!("expected unary"),
        }

        assert!(matches!(fixed[3], Instruction::Ret));
    }
}

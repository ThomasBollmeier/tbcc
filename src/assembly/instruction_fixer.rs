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
        use crate::assembly::ast::{BinaryOp::*, Instruction::*, Operand::*, Register::*};

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
                Binary { op, left, right } => match op {
                    Mul => {
                        if let Stack(_) = right {
                            new_instructions.push(Mov {
                                src: right.clone(),
                                dst: Register(R11),
                            });
                            new_instructions.push(Binary {
                                op: Mul,
                                left: left.clone(),
                                right: Register(R11),
                            });
                            new_instructions.push(Mov {
                                src: Register(R11),
                                dst: right.clone(),
                            });
                        } else {
                            new_instructions.push(instruction.clone());
                        }
                    }
                    ShiftLeft | ShiftRight => {
                        if let Stack(_) = left {
                            new_instructions.push(Mov {
                                src: left.clone(),
                                dst: Register(CX),
                            });
                            new_instructions.push(Binary {
                                op: op.clone(),
                                left: Register(CX),
                                right: right.clone(),
                            });
                        } else {
                            new_instructions.push(instruction.clone());
                        }
                    }
                    Add | Sub | BitAnd | BitOr | BitXor =>  {
                        match (left, right) {
                            (Stack(_), Stack(_)) => {
                                new_instructions.push(Mov {
                                    src: left.clone(),
                                    dst: Register(R10),
                                });
                                new_instructions.push(Binary {
                                    op: op.clone(),
                                    left: Register(R10),
                                    right: right.clone(),
                                });
                            }
                            _ => new_instructions.push(instruction.clone()),
                        }
                    }
                },
                Idiv(op) => match op {
                    Immediate(_) => {
                        new_instructions.push(Mov {
                            src: op.clone(),
                            dst: Register(R10),
                        });
                        new_instructions.push(Idiv(Register(R10)));
                    }
                    _ => new_instructions.push(instruction.clone()),
                },
                Cmp {
                    op1,
                    op2
                } => match (op1, op2) {
                    (Stack(_), Stack(_)) => {
                        new_instructions.push(Mov {
                            src: op1.clone(),
                            dst: Register(R10),
                        });
                        new_instructions.push(Cmp {
                            op1: Register(R10),
                            op2: op2.clone(),
                        });
                    }
                    (_, Immediate(_)) => {
                        new_instructions.push(Mov {
                            src: op2.clone(),
                            dst: Register(R11),
                        });
                        new_instructions.push(Cmp {
                            op1: op1.clone(),
                            op2: Register(R11),
                        });
                    }
                    _ => new_instructions.push(instruction.clone()),
                }
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
        let mut program = Program::new(vec![FuncDef::new("main".to_string(), instructions)]);
        let mut fixer = InstructionFixer::new(stack_frame_size);
        program.walk_mut(&mut fixer);
        program.functions[0].instructions.clone()
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

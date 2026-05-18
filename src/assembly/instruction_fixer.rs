use crate::assembly::assembly_creator::AssemblyCreator;
use crate::assembly::ast::{Operand, VisitorMut};

pub struct InstructionFixer;

impl InstructionFixer {
    pub fn new() -> InstructionFixer {
        InstructionFixer
    }

    fn is_memory(operand: &Operand) -> bool {
        match operand {
            Operand::Stack(_) => true,
            Operand::Data(_) => true,
            _ => false,
        }
    }

    fn all_memory(operands: &[&Operand]) -> bool {
        operands.iter().all(|op| Self::is_memory(*op))
    }
}

impl VisitorMut for InstructionFixer {
    fn enter_func_def(&mut self, func_def: &mut crate::assembly::ast::FuncDef) {
        use crate::assembly::ast::{BinaryOp::*, Instruction::*, Operand::*, Register::*};

        let mut new_instructions = vec![];

        // Allocate stack space at the beginning of the function
        new_instructions.push(AssemblyCreator::allocate_stack(func_def.stack_frame_size as i32));

        for instruction in &func_def.instructions {
            match instruction {
                Mov { assembly_type, src, dst, .. } => {
                    if Self::all_memory(&[src, dst]) {
                        new_instructions.push(Mov {
                            assembly_type: assembly_type.clone(),
                            src: src.clone(),
                            dst: Register(R10),
                        });
                        new_instructions.push(Mov {
                            assembly_type: assembly_type.clone(),
                            src: Register(R10),
                            dst: dst.clone(),
                        });
                    } else {
                        new_instructions.push(instruction.clone())
                    }
                }
                Binary { assembly_type, op, left, right } => match op {
                    Mul => {
                        if Self::is_memory(right) {
                            new_instructions.push(Mov {
                                assembly_type: assembly_type.clone(),
                                src: right.clone(),
                                dst: Register(R11),
                            });
                            new_instructions.push(Binary {
                                assembly_type: assembly_type.clone(),
                                op: Mul,
                                left: left.clone(),
                                right: Register(R11),
                            });
                            new_instructions.push(Mov {
                                assembly_type: assembly_type.clone(),
                                src: Register(R11),
                                dst: right.clone(),
                            });
                        } else {
                            new_instructions.push(instruction.clone());
                        }
                    }
                    ShiftLeft | ShiftRight => {
                        if Self::is_memory(left) {
                            new_instructions.push(Mov {
                                assembly_type: assembly_type.clone(),
                                src: left.clone(),
                                dst: Register(CX),
                            });
                            new_instructions.push(Binary {
                                assembly_type: assembly_type.clone(),
                                op: op.clone(),
                                left: Register(CX),
                                right: right.clone(),
                            });
                        } else {
                            new_instructions.push(instruction.clone());
                        }
                    }
                    Add | Sub | BitAnd | BitOr | BitXor => {
                        if Self::all_memory(&[left, right]) {
                            new_instructions.push(Mov {
                                assembly_type: assembly_type.clone(),
                                src: left.clone(),
                                dst: Register(R10),
                            });
                            new_instructions.push(Binary {
                                assembly_type: assembly_type.clone(),
                                op: op.clone(),
                                left: Register(R10),
                                right: right.clone(),
                            });
                        } else {
                            new_instructions.push(instruction.clone());
                        }
                    }
                },
                Idiv { assembly_type, operand: op} => match op {
                    Immediate(_) => {
                        new_instructions.push(Mov {
                            assembly_type: assembly_type.clone(),
                            src: op.clone(),
                            dst: Register(R10),
                        });
                        new_instructions.push(Idiv {
                            assembly_type: assembly_type.clone(),
                            operand: Register(R10)
                        });
                    }
                    _ => new_instructions.push(instruction.clone()),
                },
                Cmp { assembly_type, op1, op2 } => match (op1, op2) {
                    (_, Immediate(_)) => {
                        new_instructions.push(Mov {
                            assembly_type: assembly_type.clone(),
                            src: op2.clone(),
                            dst: Register(R11),
                        });
                        new_instructions.push(Cmp {
                            assembly_type: assembly_type.clone(),
                            op1: op1.clone(),
                            op2: Register(R11),
                        });
                    }
                    _ => {
                        if Self::all_memory(&[op1, op2]) {
                            new_instructions.push(Mov {
                                assembly_type: assembly_type.clone(),
                                src: op1.clone(),
                                dst: Register(R10),
                            });
                            new_instructions.push(Cmp {
                                assembly_type: assembly_type.clone(),
                                op1: Register(R10),
                                op2: op2.clone(),
                            });
                        } else {
                            new_instructions.push(instruction.clone());
                        }
                    }
                },
                _ => new_instructions.push(instruction.clone()),
            }
        }

        func_def.instructions = new_instructions;
    }
}

#[cfg(test)]
mod tests {
    use crate::assembly::assembly_creator::AssemblyCreator;
    use super::InstructionFixer;
    use crate::assembly::ast::TopLevel::Function;
    use crate::assembly::ast::{AssemblyType, FuncDef, Instruction, Operand, Program, Register, UnaryOp};
    use crate::assembly::pseudo_reg_replacer::PseudoRegReplacer;

    fn apply_fixer(instructions: Vec<Instruction>) -> Vec<Instruction> {
        let func_def = FuncDef::new("main".to_string(), true, instructions);
        let mut program = Program::new(vec![Function(func_def)]);
        let mut pseudo_reg_replacer = PseudoRegReplacer::new();
        program.walk_mut(&mut pseudo_reg_replacer);
        let mut fixer = InstructionFixer::new();
        program.walk_mut(&mut fixer);

        let function = &program.top_levels[0];
        if let Function(func) = function {
            func.instructions.clone()
        } else {
            unreachable!()
        }
    }

    #[test]
    fn inserts_allocate_stack_as_first_instruction() {
        let instructions = vec![Instruction::Ret];

        let fixed = apply_fixer(instructions);

        assert_eq!(fixed[0], AssemblyCreator::allocate_stack(0));
        assert_eq!(fixed[1], Instruction::Ret);
    }

    #[test]
    fn rewrites_stack_to_stack_mov_into_two_movs_via_r10() {
        let instructions = vec![Instruction::Mov {
            assembly_type: AssemblyType::Longword,
            src: Operand::Stack(-4),
            dst: Operand::Stack(-8),
        }];

        let fixed = apply_fixer(instructions);

        assert_eq!(fixed[0], AssemblyCreator::allocate_stack(0));

        match &fixed[1] {
            Instruction::Mov { src, dst, .. } => {
                assert!(matches!(src, Operand::Stack(-4)));
                assert!(matches!(dst, Operand::Register(Register::R10)));
            }
            _ => panic!("expected first rewritten mov"),
        }

        match &fixed[2] {
            Instruction::Mov { src, dst, .. } => {
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
                assembly_type: AssemblyType::Longword,
                src: Operand::Immediate(1),
                dst: Operand::Register(Register::AX),
            },
            Instruction::Unary {
                assembly_type: AssemblyType::Longword,
                op: UnaryOp::Neg,
                operand: Operand::Register(Register::AX),
            },
            Instruction::Ret,
        ];

        let fixed = apply_fixer(instructions);

        assert_eq!(fixed.len(), 4);
        assert_eq!(fixed[0], AssemblyCreator::allocate_stack(0));

        match &fixed[1] {
            Instruction::Mov { src, dst, .. } => {
                assert_eq!(*src, Operand::Immediate(1));
                assert_eq!(*dst, Operand::Register(Register::AX));
            }
            _ => panic!("expected mov"),
        }

        match &fixed[2] {
            Instruction::Unary { op, operand, .. } => {
                assert_eq!(*op, UnaryOp::Neg);
                assert!(matches!(operand, Operand::Register(Register::AX)));
            }
            _ => panic!("expected unary"),
        }

        assert!(matches!(fixed[3], Instruction::Ret));
    }
}

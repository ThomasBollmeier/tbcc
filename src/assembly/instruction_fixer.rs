use crate::assembly::assembly_creator::AssemblyCreator;
use crate::assembly::ast::AssemblyType::{Longword, Quadword};
use crate::assembly::ast::Instruction::{Mov, MovSx};
use crate::assembly::ast::Operand::Register;
use crate::assembly::ast::Register::{R10, R11};
use crate::assembly::ast::{AssemblyType, BinaryOp, ImmValue, Instruction, Operand, VisitorMut};

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

    fn handle_mov(
        &self,
        instruction: &Instruction,
        assembly_type: &AssemblyType,
        src: &Operand,
        dst: &Operand,
        new_instructions: &mut Vec<Instruction>,
    ) {
        use crate::assembly::ast::{Instruction::Mov, Operand::Register, Register::R10};

        let src_is_long = if let Operand::Immediate(ImmValue::Long(_)) = src {
            true
        } else {
            false
        };

        if Self::all_memory(&[src, dst]) || src_is_long {
            let src = if *assembly_type == Longword && src_is_long {
                match src {
                    Operand::Immediate(ImmValue::Long(l)) => {
                        Operand::Immediate(ImmValue::Int(*l as i32))
                    },
                    _ => unreachable!()
                }
            } else {
                src.clone()
            };
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
            new_instructions.push(instruction.clone());
        }
    }
    fn handle_movsx(
        &self,
        instruction: &Instruction,
        src: &Operand,
        dst: &Operand,
        new_instructions: &mut Vec<Instruction>,
    ) {
        if let Operand::Immediate(_) = src {
            new_instructions.push(Mov {
                assembly_type: Longword,
                src: src.clone(),
                dst: Register(R10),
            });
            if Self::is_memory(dst) {
                new_instructions.push(MovSx {
                    src: Register(R10),
                    dst: Register(R11),
                });
                new_instructions.push(Mov {
                    assembly_type: Quadword,
                    src: Register(R11),
                    dst: dst.clone(),
                });
            } else {
                new_instructions.push(MovSx {
                    src: Register(R10),
                    dst: dst.clone(),
                });
            }
        } else if Self::is_memory(dst) {
            new_instructions.push(MovSx {
                src: src.clone(),
                dst: Register(R10),
            });
            new_instructions.push(Mov {
                assembly_type: Quadword,
                src: Register(R10),
                dst: dst.clone(),
            })
        } else {
            new_instructions.push(instruction.clone());
        }
    }

    fn handle_binary(
        &self,
        instruction: &Instruction,
        assembly_type: &AssemblyType,
        op: &BinaryOp,
        left: &Operand,
        right: &Operand,
        new_instructions: &mut Vec<Instruction>,
    ) {
        use crate::assembly::ast::{
            BinaryOp::{Add, BitAnd, BitOr, BitXor, Mul, ShiftLeft, ShiftRight, Sub},
            Instruction::{Binary, Mov},
            Operand::Register,
            Register::{CX, R10, R11},
        };

        let left = match op {
            Add | Sub | Mul => {
                &Self::replace_long_src_operand(assembly_type, left, new_instructions)
            }
            _ => left,
        };

        match op {
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
        }
    }

    fn replace_long_src_operand(
        assembly_type: &AssemblyType,
        left: &Operand,
        instructions: &mut Vec<Instruction>,
    ) -> Operand {
        use crate::assembly::ast::{Instruction::Mov, Operand::Register, Register::R10};
        if let Operand::Immediate(ImmValue::Long(_)) = left {
            instructions.push(Mov {
                assembly_type: assembly_type.clone(),
                src: left.clone(),
                dst: Register(R10),
            });
            Register(R10)
        } else {
            left.clone()
        }
    }

    fn handle_idiv(
        &self,
        instruction: &Instruction,
        assembly_type: &AssemblyType,
        operand: &Operand,
        new_instructions: &mut Vec<Instruction>,
    ) {
        use crate::assembly::ast::{
            Instruction::Idiv, Instruction::Mov, Operand::Immediate, Operand::Register,
            Register::R10,
        };

        match operand {
            Immediate(_) => {
                new_instructions.push(Mov {
                    assembly_type: assembly_type.clone(),
                    src: operand.clone(),
                    dst: Register(R10),
                });
                new_instructions.push(Idiv {
                    assembly_type: assembly_type.clone(),
                    operand: Register(R10),
                });
            }
            _ => new_instructions.push(instruction.clone()),
        }
    }

    fn handle_cmp(
        &self,
        instruction: &Instruction,
        assembly_type: &AssemblyType,
        op1: &Operand,
        op2: &Operand,
        new_instructions: &mut Vec<Instruction>,
    ) {
        use crate::assembly::ast::{
            Instruction::Cmp,
            Instruction::Mov,
            Operand::Immediate,
            Operand::Register,
            Register::{R10, R11},
        };

        let op1 = &Self::replace_long_src_operand(assembly_type, op1, new_instructions);

        match (op1, op2) {
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
        }
    }
}

impl VisitorMut for InstructionFixer {
    fn enter_func_def(&mut self, func_def: &mut crate::assembly::ast::FuncDef) {
        use crate::assembly::ast::Instruction::*;

        let mut new_instructions = vec![];

        // Allocate stack space at the beginning of the function
        new_instructions.push(AssemblyCreator::allocate_stack(
            func_def.stack_frame_size as i32,
        ));

        for instruction in &func_def.instructions {
            match instruction {
                Mov {
                    assembly_type,
                    src,
                    dst,
                    ..
                } => {
                    self.handle_mov(instruction, assembly_type, src, dst, &mut new_instructions);
                }
                MovSx { src, dst } => {
                    self.handle_movsx(instruction, src, dst, &mut new_instructions);
                }
                Binary {
                    assembly_type,
                    op,
                    left,
                    right,
                } => self.handle_binary(
                    instruction,
                    assembly_type,
                    op,
                    left,
                    right,
                    &mut new_instructions,
                ),
                Idiv {
                    assembly_type,
                    operand,
                } => {
                    self.handle_idiv(instruction, assembly_type, operand, &mut new_instructions);
                }
                Cmp {
                    assembly_type,
                    op1,
                    op2,
                } => self.handle_cmp(instruction, assembly_type, op1, op2, &mut new_instructions),
                _ => new_instructions.push(instruction.clone()),
            }
        }

        func_def.instructions = new_instructions;
    }
}

#[cfg(test)]
mod tests {
    use super::InstructionFixer;
    use crate::assembly::assembly_creator::AssemblyCreator;
    use crate::assembly::ast::TopLevel::Function;
    use crate::assembly::ast::{
        AssemblyType, FuncDef, ImmValue, Instruction, Operand, Program, Register, UnaryOp,
    };
    use crate::assembly::pseudo_reg_replacer::PseudoRegReplacer;
    use crate::common::symbol_table_generic::SymbolTable;

    fn apply_fixer(instructions: Vec<Instruction>) -> Vec<Instruction> {
        let func_def = FuncDef::new("main".to_string(), true, instructions);
        let mut program = Program::new(vec![Function(func_def)]);
        let asm_symbol_table = SymbolTable::new_ref();
        let mut pseudo_reg_replacer = PseudoRegReplacer::new(asm_symbol_table.clone());
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
                src: Operand::Immediate(ImmValue::Int(1)),
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
                assert_eq!(*src, Operand::Immediate(ImmValue::Int(1)));
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

use std::collections::HashMap;
use crate::assembly::ast::{Instruction, Operand, VisitorMut};

pub struct PseudoRegReplacer {
    var_map: HashMap<String, i32>,
    last_offset: i32,
}

impl PseudoRegReplacer {
    pub fn new() -> Self {
        Self {
            var_map: HashMap::new(),
            last_offset: 0,
        }
    }
    
    pub fn get_frame_size(&self) -> usize {
        self.last_offset.abs() as usize
    }

    fn replace_operand(&mut self, operand: &Operand) -> Option<Operand> {
        match operand {
            Operand::PseudoReg(name) => {
                if let Some(offset) = self.var_map.get(name) {
                    Some(Operand::Stack(*offset))
                } else {
                    self.last_offset -= 4; // Assuming 4 bytes per variable
                    self.var_map.insert(name.clone(), self.last_offset);
                    Some(Operand::Stack(self.last_offset))
                }
            }
            _ => None,
        }
    }
}

impl VisitorMut for PseudoRegReplacer {
    fn visit_instruction(&mut self, instruction: &mut Instruction) {
        use crate::assembly::ast::Instruction::*;
        match instruction {
            Mov { src, dst } => {
                if let Some(new_src) = self.replace_operand(src) {
                    *src = new_src;
                }
                if let Some(new_dst) = self.replace_operand(dst) {
                    *dst = new_dst;
                }
            }
            Unary { operand, .. } => {
                if let Some(new_operand) = self.replace_operand(operand) {
                    *operand = new_operand;
                }
            }
            AllocateStack(_) | Ret => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PseudoRegReplacer;
    use crate::assembly::ast::{FuncDef, Instruction, Operand, Program, Register, UnaryOp};

    fn apply_replacer(instructions: Vec<Instruction>) -> (Vec<Instruction>, usize) {
        let mut program = Program::new(FuncDef::new("main".to_string(), instructions));
        let mut replacer = PseudoRegReplacer::new();
        program.walk(&mut replacer);

        (program.0.instructions, replacer.get_frame_size())
    }

    #[test]
    fn replaces_distinct_pseudo_registers_with_unique_stack_offsets() {
        let instructions = vec![
            Instruction::Mov {
                src: Operand::PseudoReg("a".to_string()),
                dst: Operand::PseudoReg("b".to_string()),
            },
            Instruction::Unary {
                op: UnaryOp::Neg,
                operand: Operand::PseudoReg("c".to_string()),
            },
        ];

        let (instructions, frame_size) = apply_replacer(instructions);

        match &instructions[0] {
            Instruction::Mov { src, dst } => {
                assert!(matches!(src, Operand::Stack(-4)));
                assert!(matches!(dst, Operand::Stack(-8)));
            }
            _ => panic!("expected mov instruction"),
        }

        match &instructions[1] {
            Instruction::Unary { operand, .. } => {
                assert!(matches!(operand, Operand::Stack(-12)));
            }
            _ => panic!("expected unary instruction"),
        }

        assert_eq!(frame_size, 12);
    }

    #[test]
    fn reuses_stack_offset_for_same_pseudo_register() {
        let instructions = vec![
            Instruction::Mov {
                src: Operand::PseudoReg("tmp".to_string()),
                dst: Operand::PseudoReg("tmp".to_string()),
            },
            Instruction::Unary {
                op: UnaryOp::Not,
                operand: Operand::PseudoReg("tmp".to_string()),
            },
        ];

        let (instructions, frame_size) = apply_replacer(instructions);

        match &instructions[0] {
            Instruction::Mov { src, dst } => {
                assert!(matches!(src, Operand::Stack(-4)));
                assert!(matches!(dst, Operand::Stack(-4)));
            }
            _ => panic!("expected mov instruction"),
        }

        match &instructions[1] {
            Instruction::Unary { operand, .. } => {
                assert!(matches!(operand, Operand::Stack(-4)));
            }
            _ => panic!("expected unary instruction"),
        }

        assert_eq!(frame_size, 4);
    }

    #[test]
    fn leaves_non_pseudo_operands_unchanged() {
        let instructions = vec![
            Instruction::Mov {
                src: Operand::Immediate(7),
                dst: Operand::Register(Register::AX),
            },
            Instruction::Ret,
        ];

        let (instructions, frame_size) = apply_replacer(instructions);

        match &instructions[0] {
            Instruction::Mov { src, dst } => {
                assert!(matches!(src, Operand::Immediate(7)));
                assert!(matches!(dst, Operand::Register(Register::AX)));
            }
            _ => panic!("expected mov instruction"),
        }

        assert!(matches!(instructions[1], Instruction::Ret));
        assert_eq!(frame_size, 0);
    }
}

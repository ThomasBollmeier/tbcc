use crate::assembly::ast::{FuncDef, Instruction, Operand, VisitorMut};
use crate::semantic::symbol_table;
use crate::semantic::symbol_table::IdentAttrs;
use std::collections::HashMap;

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

    fn replace_operand(&mut self, operand: &Operand) -> Option<Operand> {
        match operand {
            Operand::PseudoReg(name) => {
                if let Some(offset) = self.var_map.get(name) {
                    Some(Operand::Stack(*offset))
                } else if Self::is_static_var(name) {
                    Some(Operand::Data(name.clone()))
                } else {
                    self.last_offset -= 4; // Assuming 4 bytes per variable
                    self.var_map.insert(name.clone(), self.last_offset);
                    Some(Operand::Stack(self.last_offset))
                }
            }
            _ => None,
        }
    }

    fn is_static_var(name: &str) -> bool {
        let entry = symbol_table::get(name);
        match entry {
            Some(entry) => {
                match entry.attrs {
                    IdentAttrs::Static {..} => true,
                    _ => false,
                }
            },
            _ => false,
        }
    }
}

impl VisitorMut for PseudoRegReplacer {
    fn enter_func_def(&mut self, _func_def: &mut FuncDef) {
        self.last_offset = 0;
    }

    fn exit_func_def(&mut self, func_def: &mut FuncDef) {
        let mut stack_frame_size = self.last_offset.abs() as usize;
        // Align stack frame size to 16 bytes
        if stack_frame_size % 16 != 0 {
            stack_frame_size += 16 - (stack_frame_size % 16);
        }

        func_def.stack_frame_size = stack_frame_size;
    }

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
            Binary { left, right, .. } => {
                if let Some(new_left) = self.replace_operand(left) {
                    *left = new_left;
                }
                if let Some(new_right) = self.replace_operand(right) {
                    *right = new_right;
                }
            }
            Idiv(operand) => {
                if let Some(new_operand) = self.replace_operand(operand) {
                    *operand = new_operand;
                }
            }
            Cdq | AllocateStack(_) | Ret => {}
            Cmp { op1, op2 } => {
                if let Some(new_op1) = self.replace_operand(op1) {
                    *op1 = new_op1;
                }
                if let Some(new_op2) = self.replace_operand(op2) {
                    *op2 = new_op2;
                }
            }
            Jmp(_) | JmpCC(_, _) => {}
            SetCC(_, operand) => {
                if let Some(new_operand) = self.replace_operand(operand) {
                    *operand = new_operand;
                }
            }
            Label(_) => {}
            Push(operand) => {
                if let Some(new_operand) = self.replace_operand(operand) {
                    *operand = new_operand;
                }
            }
            Call(_) => {}
            DeAllocateStack(_) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PseudoRegReplacer;
    use crate::assembly::ast::TopLevel::Function;
    use crate::assembly::ast::{FuncDef, Instruction, Operand, Program, Register, UnaryOp};

    fn apply_replacer(instructions: Vec<Instruction>) -> Vec<Instruction> {
        let func_def = FuncDef::new("main".to_string(), true, instructions);
        let mut program = Program::new(vec![Function(func_def)]);
        let mut replacer = PseudoRegReplacer::new();
        program.walk_mut(&mut replacer);

        let function = &program.top_levels[0];
        let instructions = if let Function(func) = function {
            func.instructions.clone()
        } else {
            unreachable!()
        };

        instructions
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

        let instructions = apply_replacer(instructions);

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

        let instructions = apply_replacer(instructions);

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

        let instructions = apply_replacer(instructions);

        match &instructions[0] {
            Instruction::Mov { src, dst } => {
                assert!(matches!(src, Operand::Immediate(7)));
                assert!(matches!(dst, Operand::Register(Register::AX)));
            }
            _ => panic!("expected mov instruction"),
        }

        assert!(matches!(instructions[1], Instruction::Ret));
    }
}

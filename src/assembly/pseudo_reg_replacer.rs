use crate::assembly::ast::{AssemblyType, FuncDef, Instruction, Operand, VisitorMut};
use crate::assembly::symbol_table;
use crate::assembly::symbol_table::SymbolTableEntry;
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
                    self.update_last_offset(name);
                    self.var_map.insert(name.clone(), self.last_offset);
                    Some(Operand::Stack(self.last_offset))
                }
            }
            _ => None,
        }
    }

    fn update_last_offset(&mut self, var_name: &str) {
        let entry = symbol_table::get(var_name)
            .expect(&format!("variable {var_name} not found in symbol table"));
        match entry {
            SymbolTableEntry::Object { assembly_type, .. } => {
                match assembly_type {
                    AssemblyType::Longword => self.last_offset -= 4,
                    AssemblyType::Quadword => {
                        self.last_offset -= 8;
                        // Round last offset down to next multiple of 8:
                        let remainder = self.last_offset % 8;
                        if remainder != 0 {
                            self.last_offset -= 8 - remainder.abs();
                        }
                    },
                }

            }
            _ => panic!("expected object entry for variable {var_name}"),
        }
    }

    fn is_static_var(name: &str) -> bool {
        let entry = symbol_table::get(name);
        match entry {
            Some(entry) => match entry {
                SymbolTableEntry::Object { is_static, .. } => is_static,
                _ => false,
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
            Mov { src, dst, .. } => {
                if let Some(new_src) = self.replace_operand(src) {
                    *src = new_src;
                }
                if let Some(new_dst) = self.replace_operand(dst) {
                    *dst = new_dst;
                }
            }
            MovSx { src, dst, .. } => {
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
            Idiv { operand, .. } => {
                if let Some(new_operand) = self.replace_operand(operand) {
                    *operand = new_operand;
                }
            }
            Cmp { op1, op2, .. } => {
                if let Some(new_op1) = self.replace_operand(op1) {
                    *op1 = new_op1;
                }
                if let Some(new_op2) = self.replace_operand(op2) {
                    *op2 = new_op2;
                }
            }
            SetCC(_, operand) => {
                if let Some(new_operand) = self.replace_operand(operand) {
                    *operand = new_operand;
                }
            }
            Push(operand) => {
                if let Some(new_operand) = self.replace_operand(operand) {
                    *operand = new_operand;
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use serial_test::serial;
    use super::PseudoRegReplacer;
    use crate::assembly::ast::TopLevel::Function;
    use crate::assembly::ast::{
        AssemblyType, FuncDef, Instruction, Operand, Program, Register, UnaryOp,
    };
    use crate::assembly::symbol_table;

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

    fn fill_symbol_table(entries: &[(&str, AssemblyType)]) {
        symbol_table::clear();
        for (name, asm_type) in entries {
            symbol_table::insert(
                name.to_string(),
                symbol_table::SymbolTableEntry::Object {
                    assembly_type: asm_type.clone(),
                    is_static: false,
                },
            );
        }
    }

    #[test]
    #[serial]
    fn replaces_distinct_pseudo_registers_with_unique_stack_offsets() {
        fill_symbol_table(&[
            ("a", AssemblyType::Longword),
            ("b", AssemblyType::Longword),
            ("c", AssemblyType::Longword),
        ]);

        let instructions = vec![
            Instruction::Mov {
                assembly_type: AssemblyType::Longword,
                src: Operand::PseudoReg("a".to_string()),
                dst: Operand::PseudoReg("b".to_string()),
            },
            Instruction::Unary {
                assembly_type: AssemblyType::Longword,
                op: UnaryOp::Neg,
                operand: Operand::PseudoReg("c".to_string()),
            },
        ];

        let instructions = apply_replacer(instructions);

        match &instructions[0] {
            Instruction::Mov { src, dst, .. } => {
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
    #[serial]
    fn reuses_stack_offset_for_same_pseudo_register() {
        fill_symbol_table(&[("tmp", AssemblyType::Longword)]);

        let instructions = vec![
            Instruction::Mov {
                assembly_type: AssemblyType::Longword,
                src: Operand::PseudoReg("tmp".to_string()),
                dst: Operand::PseudoReg("tmp".to_string()),
            },
            Instruction::Unary {
                assembly_type: AssemblyType::Longword,
                op: UnaryOp::Not,
                operand: Operand::PseudoReg("tmp".to_string()),
            },
        ];

        let instructions = apply_replacer(instructions);

        match &instructions[0] {
            Instruction::Mov { src, dst, .. } => {
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
                assembly_type: AssemblyType::Longword,
                src: Operand::Immediate(7),
                dst: Operand::Register(Register::AX),
            },
            Instruction::Ret,
        ];

        let instructions = apply_replacer(instructions);

        match &instructions[0] {
            Instruction::Mov { src, dst, .. } => {
                assert!(matches!(src, Operand::Immediate(7)));
                assert!(matches!(dst, Operand::Register(Register::AX)));
            }
            _ => panic!("expected mov instruction"),
        }

        assert!(matches!(instructions[1], Instruction::Ret));
    }
}

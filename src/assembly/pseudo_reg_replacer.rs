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
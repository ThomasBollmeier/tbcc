use crate::assembly::ast::{FuncDef, Instruction, Operand, Program};

pub struct CodeGenerator {
    code: String,
}

impl CodeGenerator {
    pub fn new() -> Self {
        CodeGenerator {
            code: String::new(),
        }
    }

    pub fn generate_assembly(mut self, asm_program: &Program) -> String {
        self.code = String::new();
        self.write_program(asm_program);
        // If running in Linux, we need to add the .note.GNU-stack section to indicate that the stack is not executable
        if cfg!(target_os = "linux") {
            self.code
                .push_str(".section .note.GNU-stack,\"\",@progbits\n");
        }
        self.code.clone()
    }

    fn write_program(&mut self, asm_program: &Program) {
        self.write_comment("===== Program =====");
        self.write_function_def(&asm_program.0);
    }

    fn write_function_def(&mut self, func_def: &FuncDef) {
        self.write_instruction(&format!(".globl {}", func_def.name));
        self.write_label(&func_def.name);
        for inst in &func_def.instructions {
            self.write_inst(&inst);
        }
    }

    fn write_inst(&mut self, inst: &Instruction) {
        match inst {
            Instruction::Mov { src, dst } => {
                let src = self.operand_to_string(src);
                let dst = self.operand_to_string(dst);
                self.write_instruction(&format!("movl {src}, {dst}"));
            }
            Instruction::Ret => {
                self.write_instruction("ret");
            }
            _ => todo!("Unsupported instruction: {:?}", inst),
        }
    }

    fn write_instruction(&mut self, inst: &str) {
        self.code.push_str(&format!("\t{}\n", inst));
    }

    fn write_label(&mut self, label: &str) {
        self.code.push_str(&format!(" {}:\n", label));
    }

    fn write_comment(&mut self, comment: &str) {
        self.code.push_str(&format!("# {}\n", comment));
    }

    fn operand_to_string(&self, operand: &Operand) -> String {
        match operand {
            Operand::Immediate(imm) => format!("${}", imm),
            Operand::Register(_) => "%eax".to_string(),
            _ => todo!("Unsupported operand type: {:?}", operand),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assembly;
    use crate::lexer::Lexer;
    use crate::parser::Parser;
    use crate::tacky::TackyEmitter;

    #[test]
    fn creates_asm_program_ok() {
        let parser = Parser::new();
        let lexer = Lexer::new();

        let code = "int main(void) { return -(~42); }";

        let tokens = lexer.scan_tokens(code).expect("Failed to scan tokens");
        let program = parser.parse(tokens).expect("Failed to parse program");

        let mut tacky_emitter = TackyEmitter::new();
        let tacky_program = tacky_emitter
            .emit_program(&program)
            .expect("Failed to emit tacky program");

        let assembly_program =
            assembly::create_program(&tacky_program).expect("Failed to create assembly program");

        let code_generator = CodeGenerator::new();
        let asm_code = code_generator.generate_assembly(&assembly_program);

        assert!(!asm_code.is_empty());

        print!("{asm_code}");
    }
}

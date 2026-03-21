use crate::assembly_ast::{FuncDef, Instruction, Operand, Program, Visitor};

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
        asm_program.accept(&mut self);
        // If running in Linux, we need to add the .note.GNU-stack section to indicate that the stack is not executable
        if cfg!(target_os = "linux") {
            self.code
                .push_str(".section .note.GNU-stack,\"\",@progbits\n");
        }
        self.code.clone()
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
            Operand::Register => "%eax".to_string(),
        }
    }
}

impl Visitor<()> for CodeGenerator {
    fn visit_program(&mut self, program: &Program) {
        self.write_comment("===== Program =====");
        program.0.accept(self);
    }

    fn visit_function_def(&mut self, func_def: &FuncDef) -> () {
        self.write_instruction(&format!(".globl {}", func_def.name));
        self.write_label(&func_def.name);
        for inst in &func_def.instructions {
            inst.accept(self);
        }
    }

    fn visit_instruction(&mut self, instruction: &Instruction) -> () {
        match instruction {
            Instruction::Mov { src, dst } => {
                let src = self.operand_to_string(src);
                let dst = self.operand_to_string(dst);
                self.write_instruction(&format!("movl {}, {}", src, dst));
            }
            Instruction::Ret => {
                self.write_instruction("ret");
            }
        }
    }

    fn visit_operand(&mut self, _operand: &Operand) {
        // nothing to do here
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assembly_ast::AssemblyCreator;
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    #[test]
    fn creates_asm_program_ok() {
        let parser = Parser::new();
        let lexer = Lexer::new();

        let code = "int main(void) { return 42; }";

        let tokens = lexer.scan_tokens(code).expect("Failed to scan tokens");
        let program = parser.parse(tokens).expect("Failed to parse program");

        let mut assembly_creator = AssemblyCreator::new();
        let assembly_program = assembly_creator
            .create_assembly_program(&program)
            .expect("Failed to create assembly program");

        let code_generator = CodeGenerator::new();
        let asm_code = code_generator.generate_assembly(&assembly_program);

        assert!(!asm_code.is_empty());

        dbg!(&asm_code);
    }
}

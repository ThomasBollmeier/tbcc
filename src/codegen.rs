use crate::assembly::ast::{
    BinaryOp, FuncDef, Instruction, Operand, Program, Register, UnaryOp, Visitor,
};

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
        asm_program.walk(&mut self);
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
            Operand::Register(Register::AX) => "%eax".to_string(),
            Operand::Register(Register::CX) => "%ecx".to_string(),
            Operand::Register(Register::DX) => "%edx".to_string(),
            Operand::Register(Register::R10) => "%r10d".to_string(),
            Operand::Register(Register::R11) => "%r11d".to_string(),
            Operand::Stack(offset) => format!("{}(%rbp)", offset),
            _ => panic!("Unsupported operand type: {:?}", operand),
        }
    }

    fn unary_op_to_string(&self, unary_op: &UnaryOp) -> String {
        match unary_op {
            UnaryOp::Neg => "negl".to_string(),
            UnaryOp::Not => "notl".to_string(),
        }
    }

    fn binary_op_to_string(&self, binary_op: &BinaryOp) -> String {
        match binary_op {
            BinaryOp::Add => "addl".to_string(),
            BinaryOp::Sub => "subl".to_string(),
            BinaryOp::Mul => "imull".to_string(),
            BinaryOp::BitAnd => "andl".to_string(),
            BinaryOp::BitOr => "orl".to_string(),
            BinaryOp::BitXor => "xorl".to_string(),
            BinaryOp::ShiftLeft => "shll".to_string(),
            BinaryOp::ShiftRight => "sarl".to_string(),
        }
    }
}

impl Visitor for CodeGenerator {
    fn enter_program(&mut self, _program: &Program) {
        self.code = String::new();
        self.write_comment("===== Program =====");
    }

    fn exit_program(&mut self, _program: &Program) {
        // If running in Linux, we need to add the .note.GNU-stack section to indicate that the stack is not executable
        if cfg!(target_os = "linux") {
            self.code
                .push_str(".section .note.GNU-stack,\"\",@progbits\n");
        }
    }

    fn enter_func_def(&mut self, func_def: &FuncDef) {
        self.write_instruction(&format!(".globl {}", func_def.name));
        self.write_label(&func_def.name);
        self.write_instruction("pushq \t%rbp");
        self.write_instruction("movq \t%rsp, %rbp");
    }
    fn exit_func_def(&mut self, _func_def: &FuncDef) {}

    fn visit_instruction(&mut self, instruction: &Instruction) {
        match instruction {
            Instruction::Mov { src, dst } => {
                let src = self.operand_to_string(src);
                let dst = self.operand_to_string(dst);
                self.write_instruction(&format!("movl \t{src}, {dst}"));
            }
            Instruction::Ret => {
                self.write_instruction("movq \t%rbp, %rsp");
                self.write_instruction("popq \t%rbp");
                self.write_instruction("ret");
            }
            Instruction::Unary { op, operand } => {
                let op_str = self.unary_op_to_string(op);
                let operand_str = self.operand_to_string(operand);
                self.write_instruction(&format!("{op_str} \t{operand_str}"));
            }
            Instruction::Binary { op, left, right } => {
                let op_str = self.binary_op_to_string(op);
                let left_str = self.operand_to_string(left);
                let right_str = self.operand_to_string(right);
                self.write_instruction(&format!("{op_str} \t{left_str}, {right_str}"));
            }
            Instruction::Idiv(operand) => {
                let operand_str = self.operand_to_string(operand);
                self.write_instruction(&format!("idivl \t{operand_str}"));
            }
            Instruction::Cdq => self.write_instruction("cdq"),
            Instruction::AllocateStack(size) => {
                self.write_instruction(&format!("subq \t${size}, %rsp"));
            }
            _ => todo!("handle other instruction types"),
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

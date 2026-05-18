use crate::assembly::ast::{BinaryOp, ConditionCode, FuncDef, Instruction, Operand, Program, Register, StaticVar, UnaryOp, Visitor};
use crate::common::{symbol_table, InitValue};

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

    fn operand_8byte_to_string(&mut self, operand: &Operand) -> String {
        match operand {
            Operand::Immediate(imm) => format!("${}", imm),
            Operand::Register(Register::AX) => "%rax".to_string(),
            Operand::Register(Register::CX) => "%rcx".to_string(),
            Operand::Register(Register::DX) => "%rdx".to_string(),
            Operand::Register(Register::DI) => "%rdi".to_string(),
            Operand::Register(Register::SI) => "%rsi".to_string(),
            Operand::Register(Register::R8) => "%r8".to_string(),
            Operand::Register(Register::R9) => "%r9".to_string(),
            Operand::Register(Register::R10) => "%r10".to_string(),
            Operand::Register(Register::R11) => "%r11".to_string(),
            Operand::Register(Register::SP) => "%rsp".to_string(),
            Operand::Stack(offset) => format!("{}(%rbp)", offset),
            Operand::Data(label) => self.get_variable_op_name(label),
            _ => panic!("Unsupported operand type: {:?}", operand),
        }
    }

    fn operand_4byte_to_string(&self, operand: &Operand) -> String {
        match operand {
            Operand::Immediate(imm) => format!("${}", imm),
            Operand::Register(Register::AX) => "%eax".to_string(),
            Operand::Register(Register::CX) => "%ecx".to_string(),
            Operand::Register(Register::DX) => "%edx".to_string(),
            Operand::Register(Register::DI) => "%edi".to_string(),
            Operand::Register(Register::SI) => "%esi".to_string(),
            Operand::Register(Register::R8) => "%r8d".to_string(),
            Operand::Register(Register::R9) => "%r9d".to_string(),
            Operand::Register(Register::R10) => "%r10d".to_string(),
            Operand::Register(Register::R11) => "%r11d".to_string(),
            Operand::Register(Register::SP) => "%rsp".to_string(),
            Operand::Stack(offset) => format!("{}(%rbp)", offset),
            Operand::Data(label) => self.get_variable_op_name(label),
            _ => panic!("Unsupported operand type: {:?}", operand),
        }
    }

    fn operand_1byte_to_string(&self, operand: &Operand) -> String {
        match operand {
            Operand::Immediate(imm) => format!("${}", imm),
            Operand::Register(Register::AX) => "%al".to_string(),
            Operand::Register(Register::CX) => "%cl".to_string(),
            Operand::Register(Register::DX) => "%dl".to_string(),
            Operand::Register(Register::DI) => "%dil".to_string(),
            Operand::Register(Register::SI) => "%sil".to_string(),
            Operand::Register(Register::R8) => "%r8b".to_string(),
            Operand::Register(Register::R9) => "%r9b".to_string(),
            Operand::Register(Register::R10) => "%r10b".to_string(),
            Operand::Register(Register::R11) => "%r11b".to_string(),
            Operand::Register(Register::SP) => "%rsp".to_string(),
            Operand::Stack(offset) => format!("{}(%rbp)", offset),
            Operand::Data(label) => self.get_variable_op_name(label),
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

    fn condition_code_to_suffix(&self, condition_code: &ConditionCode) -> String {
        match condition_code {
            ConditionCode::Eq => "e".to_string(),
            ConditionCode::NotEq => "ne".to_string(),
            ConditionCode::Gt => "g".to_string(),
            ConditionCode::GtEq => "ge".to_string(),
            ConditionCode::Lt => "l".to_string(),
            ConditionCode::LtEq => "le".to_string(),
        }
    }

    fn local_label(&self, label: &str) -> String {
        if cfg!(target_os = "linux") {
            format!(".L{}", label)
        } else {
            format!("L{}", label)
        }
    }

    fn get_function_name(&self, original_function_name: &str) -> String {
        if cfg!(target_os = "linux") {
            match symbol_table::get(&original_function_name) {
                Some(entry) => match &entry.attrs {
                    symbol_table::IdentAttrs::Function { is_defined, .. } => {
                        if *is_defined {
                            original_function_name.to_string()
                        } else {
                            format!("{}@PLT", original_function_name)
                        }
                    }
                    _ => panic!("Expected '{}' to be a function", original_function_name),
                },
                None => panic!("Function '{}' not found in symbol table", original_function_name),
            }
        } else if cfg!(target_os = "macos") {
            format!("_{}", original_function_name)
        } else {
            original_function_name.to_string()
        }
    }

    fn get_variable_name(&self, original_name: &str) -> String {
        if cfg!(target_os = "linux") {
            format!("{}", original_name)
        } else if cfg!(target_os = "macos") {
            format!("_{}", original_name)
        } else {
            unreachable!()
        }
    }

    fn get_variable_op_name(&self, original_name: &str) -> String {
        if cfg!(target_os = "linux") {
            format!("{}(%rip)", original_name)
        } else if cfg!(target_os = "macos") {
            format!("_{}(%rip)", original_name)
        } else {
            unreachable!()
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
        if func_def.is_global {
            self.write_instruction(&format!(".globl {}", func_def.name));
        }
        self.write_instruction(".text");
        self.write_label(&self.get_function_name(&func_def.name));
        self.write_instruction("pushq \t%rbp");
        self.write_instruction("movq \t%rsp, %rbp");
    }

    fn exit_func_def(&mut self, _func_def: &FuncDef) {}

    fn visit_static_var(&mut self, static_var: &StaticVar) {
        if static_var.is_global {
            self.write_instruction(&format!(".globl {}", static_var.name));
        }

        let value = match static_var.value {
            InitValue::Int(i) => i as i64,
            InitValue::Long(l) => l,
        };

        if value != 0 {
            self.write_instruction(".data");
        } else {
            self.write_instruction(".bss");
        }

        if cfg!(target_os = "linux") {
            self.write_instruction(".align 4");
        } else if cfg!(target_os = "macos") {
            self.write_instruction(".balign 4");
        } else {
            unreachable!()
        }

        self.write_label(&self.get_variable_name(&static_var.name));

        if value != 0 {
            self.write_instruction(&format!(".long {}", value));
        } else {
            self.write_instruction(".zero 4");
        }
    }

    fn visit_instruction(&mut self, instruction: &Instruction) {
        match instruction {
            Instruction::Mov { src, dst, .. } => {
                let src = self.operand_4byte_to_string(src);
                let dst = self.operand_4byte_to_string(dst);
                self.write_instruction(&format!("movl \t{src}, {dst}"));
            }
            Instruction::MovSx { .. } => todo!("implement"),
            Instruction::Ret => {
                self.write_instruction("movq \t%rbp, %rsp");
                self.write_instruction("popq \t%rbp");
                self.write_instruction("ret");
            }
            Instruction::Unary { op, operand, .. } => {
                let op_str = self.unary_op_to_string(op);
                let operand_str = self.operand_4byte_to_string(operand);
                self.write_instruction(&format!("{op_str} \t{operand_str}"));
            }
            Instruction::Binary { op, left, right, .. } => {
                let op_str = self.binary_op_to_string(op);
                let left_str = self.operand_4byte_to_string(left);
                let right_str = self.operand_4byte_to_string(right);
                self.write_instruction(&format!("{op_str} \t{left_str}, {right_str}"));
            }
            Instruction::Idiv { operand, .. } => {
                let operand_str = self.operand_4byte_to_string(operand);
                self.write_instruction(&format!("idivl \t{operand_str}"));
            }
            Instruction::Cdq(_) => self.write_instruction("cdq"),
            Instruction::Cmp { op1, op2, .. } => {
                let op1_str = self.operand_4byte_to_string(op1);
                let op2_str = self.operand_4byte_to_string(op2);
                self.write_instruction(&format!("cmpl \t{op1_str}, {op2_str}"));
            }
            Instruction::Jmp(label) => {
                let label = self.local_label(label);
                self.write_instruction(&format!("jmp \t{label}"));
            }
            Instruction::JmpCC(condition_code, label) => {
                let suffix = self.condition_code_to_suffix(condition_code);
                let label = self.local_label(label);
                self.write_instruction(&format!("j{suffix} \t{label}"));
            }
            Instruction::SetCC(condition_code, operand) => {
                let suffix = self.condition_code_to_suffix(condition_code);
                let operand_str = self.operand_1byte_to_string(operand);
                self.write_instruction(&format!("set{suffix} \t{operand_str}"));
            }
            Instruction::Label(label) => {
                let label = self.local_label(label);
                self.write_label(&label);
            }
            Instruction::Push(operand) => {
                let operand_str = self.operand_8byte_to_string(operand);
                self.write_instruction(&format!("pushq \t{operand_str}"));
            }
            Instruction::Call(func_name) => {
                let name = self.get_function_name(func_name);
                self.write_instruction(&format!("call \t{name}"));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;
    use crate::semantic;
    use crate::tacky::TackyEmitter;
    use crate::assembly;

    #[test]
    fn creates_asm_program_ok() {
        let code = "int main(void) { return -(~42); }";
        let assembly_program = create_assembly(code);
        let code_generator = CodeGenerator::new();
        let asm_code = code_generator.generate_assembly(&assembly_program);

        assert!(!asm_code.is_empty());

        print!("{asm_code}");
    }

    #[test]
    fn generates_asm_with_comparisons_and_jumps() {
        let code = "int main(void) { return (1 < 2) && (3 != 4); }";
        let assembly_program = create_assembly(code);
        let code_generator = CodeGenerator::new();
        let asm_code = code_generator.generate_assembly(&assembly_program);

        assert!(
            asm_code.contains("cmpl"),
            "Expected a compare instruction in ASM"
        );
        assert!(
            asm_code.contains("je \t"),
            "Expected a conditional jump (je) in ASM"
        );
        assert!(
            asm_code.contains("jmp \t"),
            "Expected an unconditional jump in ASM"
        );
        assert!(
            asm_code.contains("setl") || asm_code.contains("setne"),
            "Expected setcc for comparisons in ASM"
        );
    }

    #[test]
    fn generate_asm_multiple_static_locals() {
        let code = r#"
            /* Multiple functions may declare static local variables
             * with the same name; these variables have no linkage,
             * and are distinct from each other.
             */

            int foo(void) {
                /* 'a' is a static local variable.
                 * its value doubles each time we call foo()
                 */
                static int a = 3;
                a = a * 2;
                return a;
            }

            int bar(void) {
                /* 'a' is a static local variable, distinct from the
                 * 'a' variable declared in foo.
                 * its value increases by one each time we call bar()
                 */
                static int a = 4;
                a = a + 1;
                return a;
            }

            int main(void) {
                return foo() + bar() + foo() + bar();
            }
        "#;

        let assembly_program = create_assembly(code);
        let code_generator = CodeGenerator::new();
        let asm_code = code_generator.generate_assembly(&assembly_program);

        print!("{asm_code}");
    }

    fn create_assembly(code: &str) -> Program {
        let parser = Parser::new();
        let lexer = Lexer::new();

        let tokens = lexer.scan_tokens(code).expect("Failed to scan tokens");
        let mut program = parser.parse(tokens).expect("Failed to parse program");

        let var_name_gen = semantic::make_var_name_generator();
        let label_name_gen = semantic::make_label_name_generator();
        let tmp_var_name_gen = semantic::make_temp_var_name_generator();

        semantic::validate(&var_name_gen, &label_name_gen, &mut program)
            .expect("Semantic validation failed");

        let mut tacky_emitter = TackyEmitter::new(label_name_gen, tmp_var_name_gen);
        let tacky_program = tacky_emitter
            .emit_program(&program)
            .expect("Failed to emit tacky program");

        assembly::create_program(&tacky_program).expect("Failed to create assembly program")
    }
}

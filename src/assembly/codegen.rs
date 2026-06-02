use crate::assembly::ast::{AssemblyType, BinaryOp, ConditionCode, FuncDef, Instruction, Operand, Program, Register, StaticVar, UnaryOp, Visitor};
use crate::assembly::symbol_table::SymbolTableEntry;
use crate::common::InitValue;
use crate::common::symbol_table_generic::SymbolTableRef;

pub struct CodeGenerator {
    code: String,
    asm_symbol_table: SymbolTableRef<SymbolTableEntry>,
}

impl CodeGenerator {
    pub fn new(asm_symbol_table: SymbolTableRef<SymbolTableEntry>) -> Self {
        CodeGenerator {
            code: String::new(),
            asm_symbol_table,
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

    fn operand_8byte_to_string(&self, operand: &Operand) -> String {
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
            Operand::Register(Register::SP) => "%esp".to_string(),
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
            Operand::Register(Register::SP) => "%spl".to_string(),
            Operand::Stack(offset) => format!("{}(%rbp)", offset),
            Operand::Data(label) => self.get_variable_op_name(label),
            _ => panic!("Unsupported operand type: {:?}", operand),
        }
    }

    fn operand_to_string(&self, operand: &Operand, assembly_type: &AssemblyType) -> String {
        match assembly_type {
            AssemblyType::Longword => self.operand_4byte_to_string(&operand),
            AssemblyType::Quadword => self.operand_8byte_to_string(&operand),
        }
    }

    fn unary_op_to_string(&self, unary_op: &UnaryOp, assembly_type: &AssemblyType) -> String {
        let instruction = match unary_op {
            UnaryOp::Neg => "neg".to_string(),
            UnaryOp::Not => "not".to_string(),
        };
        let suffix = self.get_instruction_suffix(assembly_type);
        format!("{}{}", instruction, suffix)
    }

    fn binary_op_to_string(&self, binary_op: &BinaryOp, assembly_type: &AssemblyType) -> String {
        let instruction = match binary_op {
            BinaryOp::Add => "add".to_string(),
            BinaryOp::Sub => "sub".to_string(),
            BinaryOp::Mul => "imul".to_string(),
            BinaryOp::BitAnd => "and".to_string(),
            BinaryOp::BitOr => "or".to_string(),
            BinaryOp::BitXor => "xor".to_string(),
            BinaryOp::ShiftLeft => "shl".to_string(),
            BinaryOp::ShiftRight => "sar".to_string(),
        };
        let suffix = self.get_instruction_suffix(assembly_type);
        format!("{}{}", instruction, suffix)
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
            match self
                .asm_symbol_table
                .borrow()
                .get_entry_cloned(&original_function_name)
            {
                Some(entry) => match &entry {
                    SymbolTableEntry::Function { is_defined, .. } => {
                        if *is_defined {
                            original_function_name.to_string()
                        } else {
                            format!("{}@PLT", original_function_name)
                        }
                    }
                    _ => panic!("Expected '{}' to be a function", original_function_name),
                },
                None => panic!(
                    "Function '{}' not found in symbol table",
                    original_function_name
                ),
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

    fn get_instruction_suffix(&self, assembly_type: &AssemblyType) -> String {
        match assembly_type {
            AssemblyType::Longword => String::from("l"),
            AssemblyType::Quadword => String::from("q"),
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

        let (value, is_quadword) = match static_var.value {
            InitValue::Int(i) => (i as i64, false),
            InitValue::Long(l) => (l, true),
        };

        if value != 0 {
            self.write_instruction(".data");
        } else {
            self.write_instruction(".bss");
        }

        if cfg!(target_os = "linux") {
            self.write_instruction(&format!(".align {}", static_var.alignment));
        } else if cfg!(target_os = "macos") {
            self.write_instruction(&format!(".balign {}", static_var.alignment));
        } else {
            unreachable!()
        }

        self.write_label(&self.get_variable_name(&static_var.name));

        if value != 0 {
            if !is_quadword {
                self.write_instruction(&format!(".long {}", value));
            } else {
                self.write_instruction(&format!(".quad {}", value));
            }
        } else {
            let size = if !is_quadword { 4 } else { 8 };
            self.write_instruction(&format!(".zero {size}"));
        }
    }

    fn visit_instruction(&mut self, instruction: &Instruction) {
        match instruction {
            Instruction::Mov { assembly_type, src  , dst} => {
                let suffix = self.get_instruction_suffix(assembly_type);
                let src = self.operand_to_string(src, assembly_type);
                let dst = self.operand_to_string(dst, assembly_type);
                self.write_instruction(&format!("mov{suffix} \t{src}, {dst}"));
            }
            Instruction::MovSx { src, dst, .. } => {
                let src_str = self.operand_4byte_to_string(src);
                let dst_str = self.operand_8byte_to_string(dst);
                self.write_instruction(&format!("movslq \t{src_str}, {dst_str}"));
            }
            Instruction::Ret => {
                self.write_instruction("movq \t%rbp, %rsp");
                self.write_instruction("popq \t%rbp");
                self.write_instruction("ret");
            }
            Instruction::Unary { assembly_type, op, operand} => {
                let op_str = self.unary_op_to_string(op, assembly_type);
                let operand_str = self.operand_to_string(operand, assembly_type);
                self.write_instruction(&format!("{op_str} \t{operand_str}"));
            }
            Instruction::Binary {
                op, left, right, assembly_type,
            } => {
                let op_str = self.binary_op_to_string(op, assembly_type);
                let left_str = self.operand_to_string(left, assembly_type);
                let right_str = self.operand_to_string(right, assembly_type);
                self.write_instruction(&format!("{op_str} \t{left_str}, {right_str}"));
            }
            Instruction::Idiv { operand, assembly_type } => {
                let suffix = self.get_instruction_suffix(assembly_type);
                let operand_str = self.operand_to_string(operand, assembly_type);
                self.write_instruction(&format!("idiv{suffix} \t{operand_str}"));
            }
            Instruction::Cdq(assembly_type ) => match assembly_type {
                AssemblyType::Longword => self.write_instruction("cdq"),
                AssemblyType::Quadword => self.write_instruction("cqo"),
            },
            Instruction::Cmp { op1, op2, assembly_type } => {
                let suffix = self.get_instruction_suffix(assembly_type);
                let op1_str = self.operand_to_string(op1, assembly_type);
                let op2_str = self.operand_to_string(op2, assembly_type);
                self.write_instruction(&format!("cmp{suffix} \t{op1_str}, {op2_str}"));
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
    use crate::assembly;
    use crate::common::symbol_table_generic::SymbolTable;
    use crate::lexer::Lexer;
    use crate::parser::Parser;
    use crate::semantic;
    use crate::tacky::TackyEmitter;

    #[test]
    fn creates_asm_program_ok() {
        let code = "int main(void) { return -(~42); }";
        let (assembly_program, asm_symbol_table) = create_assembly(code);
        let code_generator = CodeGenerator::new(asm_symbol_table);
        let asm_code = code_generator.generate_assembly(&assembly_program);

        assert!(!asm_code.is_empty());

        print!("{asm_code}");
    }

    #[test]
    fn generates_asm_with_comparisons_and_jumps() {
        let code = "int main(void) { return (1 < 2) && (3 != 4); }";
        let (assembly_program, asm_symbol_table) = create_assembly(code);
        let code_generator = CodeGenerator::new(asm_symbol_table);
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

        let (assembly_program, asm_symbol_table) = create_assembly(code);
        let code_generator = CodeGenerator::new(asm_symbol_table);
        let asm_code = code_generator.generate_assembly(&assembly_program);

        print!("{asm_code}");
    }

    fn create_assembly(code: &str) -> (Program, SymbolTableRef<SymbolTableEntry>) {
        let parser = Parser::new();
        let lexer = Lexer::new();

        let tokens = lexer.scan_tokens(code).expect("Failed to scan tokens");
        let mut program = parser.parse(tokens).expect("Failed to parse program");

        let var_name_gen = semantic::make_var_name_generator();
        let label_name_gen = semantic::make_label_name_generator();
        let tmp_var_name_gen = semantic::make_temp_var_name_generator();
        let symbol_table = SymbolTable::new_ref();

        semantic::validate(
            &var_name_gen,
            &label_name_gen,
            symbol_table.clone(),
            &mut program,
        )
        .expect("Semantic validation failed");

        let mut tacky_emitter =
            TackyEmitter::new(label_name_gen, tmp_var_name_gen, symbol_table.clone());
        let tacky_program = tacky_emitter
            .emit_program(&program)
            .expect("Failed to emit tacky program");

        assembly::create_program(&tacky_program, symbol_table)
            .expect("Failed to create assembly program")
    }
}

use crate::assembly::ast::Operand::Stack;
use crate::assembly::ast::Register::{CX, DI, DX, R8, R9, SI};
use crate::assembly::ast::{AssemblyType, StaticVar, TopLevel as TopLevelAsm};
use crate::assembly::ast::{
    ConditionCode, FuncDef, Instruction, Operand, Program, Register, UnaryOp,
};
use crate::common::{InitValue, Type, symbol_table};
use crate::tacky::ast::{
    BinaryOperator as TackyBinOp, BinaryOperator, Function, Instruction as TackyInstruction,
    StaticVariable, TopLevel, UnaryOperator, Value,
};

#[derive(Debug)]
pub struct AssemblyCreator {
    arg_registers: [Register; 6],
}

impl AssemblyCreator {
    pub fn new() -> AssemblyCreator {
        AssemblyCreator {
            arg_registers: [DI, SI, DX, CX, R8, R9],
        }
    }

    pub fn create_program(
        &mut self,
        tacky_program: &crate::tacky::ast::Program,
    ) -> anyhow::Result<Program> {
        let mut top_levels_asm = vec![];
        for top_level in &tacky_program.0 {
            match top_level {
                TopLevel::Function(f) => {
                    let func_def_asm = self.create_func_def(f)?;
                    top_levels_asm.push(TopLevelAsm::Function(func_def_asm));
                }
                TopLevel::StaticVariable(static_var) => {
                    let static_var_asm = self.create_static_var(static_var)?;
                    top_levels_asm.push(TopLevelAsm::StaticVariable(static_var_asm));
                }
            }
        }

        Ok(Program::new(top_levels_asm))
    }

    fn create_static_var(&mut self, static_var: &StaticVariable) -> anyhow::Result<StaticVar> {
        let (value, alignment) = match static_var.initial_value {
            Value::IntegerConstant(i) => (InitValue::Int(i), 4),
            Value::LongConstant(l) => (InitValue::Long(l), 8),
            _ => return Err(anyhow::anyhow!("Not a valid constant.")),
        };

        Ok(StaticVar {
            name: static_var.name.clone(),
            is_global: static_var.is_global,
            value,
            alignment,
        })
    }

    fn create_func_def(&mut self, func_def: &Function) -> anyhow::Result<FuncDef> {
        let name = func_def.name.clone();
        let num_arg_regs = self.arg_registers.len();
        let mut instructions = vec![];

        // Copy arguments into pseudo-registers:
        for (idx, param) in func_def.parameters.iter().enumerate() {
            let src = if idx < num_arg_regs {
                Operand::Register(self.arg_registers[idx].clone())
            } else {
                let offset = (idx - num_arg_regs) * 8 + 16;
                Stack(offset as i32)
            };

            let assembly_type = Self::lookup_asm_type(param);
            let dst = Operand::PseudoReg(param.clone());
            instructions.push(Instruction::Mov {
                assembly_type,
                src,
                dst,
            });
        }

        instructions.extend(self.create_instructions(&func_def.body)?);

        Ok(FuncDef::new(name, func_def.is_global, instructions))
    }

    fn create_instructions(
        &mut self,
        instructions: &Vec<TackyInstruction>,
    ) -> anyhow::Result<Vec<Instruction>> {
        let mut ret = vec![];

        for instruction in instructions {
            match instruction {
                TackyInstruction::Return(value) => self.push_return(&mut ret, value),
                TackyInstruction::Unary {
                    op: UnaryOperator::Not,
                    src,
                    dst,
                } => self.push_unary_not(&mut ret, src, dst),
                TackyInstruction::Unary { op, src, dst } => self.push_unary(&mut ret, op, src, dst),
                TackyInstruction::Binary {
                    op: TackyBinOp::Divide,
                    src1,
                    src2,
                    dst,
                } => self.push_binary_divide(&mut ret, src1, src2, dst),
                TackyInstruction::Binary {
                    op: TackyBinOp::Remainder,
                    src1,
                    src2,
                    dst,
                } => self.push_binary_remainder(&mut ret, src1, src2, dst),
                TackyInstruction::Binary {
                    op,
                    src1,
                    src2,
                    dst,
                } => self.push_binary(&mut ret, op, src1, src2, dst),
                TackyInstruction::Jump { target } => self.push_jump(&mut ret, target),
                TackyInstruction::JumpIfZero { condition, target } => {
                    self.push_jump_if_zero(&mut ret, condition, target)
                }
                TackyInstruction::JumpIfNotZero { condition, target } => {
                    self.push_jump_if_not_zero(&mut ret, condition, target)
                }
                TackyInstruction::Copy { src, dst } => self.push_copy(&mut ret, src, dst),
                TackyInstruction::Label(name) => self.push_label(&mut ret, name),
                TackyInstruction::FunctionCall {
                    name,
                    arguments,
                    dst,
                } => self.push_function_call(&mut ret, name, arguments, dst),
                _ => return Err(anyhow::anyhow!("Not a valid instruction.")),
            }
        }

        Ok(ret)
    }

    fn push_function_call(
        &mut self,
        instructions: &mut Vec<Instruction>,
        name: &str,
        arguments: &Vec<Value>,
        dst: &Value,
    ) {
        use Register::*;

        const ARG_SIZE: usize = 8;
        let num_arg_registers = self.arg_registers.len();

        let (register_args, stack_args) = if arguments.len() <= num_arg_registers {
            (arguments.clone(), vec![])
        } else {
            let register_args = arguments
                .iter()
                .take(num_arg_registers)
                .cloned()
                .collect::<Vec<_>>();
            let stack_args = arguments
                .iter()
                .skip(num_arg_registers)
                .cloned()
                .collect::<Vec<_>>();
            (register_args, stack_args)
        };

        let stack_padding = if stack_args.len() % 2 == 0 { 0 } else { 8 };

        if stack_padding > 0 {
            instructions.push(Instruction::AllocateStack(stack_padding));
        }

        // System V calling convention:
        // First 6 arguments into registers
        for (reg_index, arg) in register_args.iter().enumerate() {
            let assembly_type = Self::get_asm_type(arg);
            let src = self.create_operand(arg);
            let dst = Operand::Register(self.arg_registers[reg_index].clone());
            instructions.push(Instruction::Mov {
                assembly_type,
                src,
                dst,
            });
        }

        // Remaining arguments pushed onto stack
        for arg in stack_args.iter().rev() {
            let assembly_type = Self::get_asm_type(arg);
            let op = self.create_operand(arg);
            match op {
                Operand::Register(_) | Operand::Immediate(_) => {
                    instructions.push(Instruction::Push(op));
                }
                _ => {
                    instructions.push(Instruction::Mov {
                        assembly_type,
                        src: op,
                        dst: Operand::Register(AX),
                    });
                    instructions.push(Instruction::Push(Operand::Register(AX)));
                }
            }
        }

        instructions.push(Instruction::Call(name.to_string()));

        // Adjust stack pointer
        let bytes_to_remove = ARG_SIZE * stack_args.len() + stack_padding as usize;
        if bytes_to_remove > 0 {
            instructions.push(Instruction::DeAllocateStack(bytes_to_remove as i32));
        }

        // Set return value:
        let assembly_type = Self::get_asm_type(dst);
        instructions.push(Instruction::Mov {
            assembly_type,
            src: Operand::Register(AX),
            dst: self.create_operand(dst),
        });
    }

    fn push_return(&mut self, instructions: &mut Vec<Instruction>, value: &Value) {
        use crate::assembly::ast::Instruction::*;

        let assembly_type = Self::get_asm_type(value);
        let src = self.create_operand(value);
        instructions.push(Mov {
            assembly_type,
            src,
            dst: Operand::Register(Register::AX),
        });
        instructions.push(Ret);
    }

    fn push_unary_not(&mut self, instructions: &mut Vec<Instruction>, src: &Value, dst: &Value) {
        use crate::assembly::ast::Instruction::*;

        let src_op = self.create_operand(src);
        let dst_op = self.create_operand(dst);
        instructions.push(Cmp {
            assembly_type: Self::get_asm_type(src),
            op1: Operand::Immediate(0),
            op2: src_op,
        });
        instructions.push(Mov {
            assembly_type: Self::get_asm_type(dst),
            src: Operand::Immediate(0),
            dst: dst_op.clone(),
        });
        instructions.push(SetCC(ConditionCode::Eq, dst_op));
    }

    fn push_unary(
        &mut self,
        instructions: &mut Vec<Instruction>,
        op: &UnaryOperator,
        src: &Value,
        dst: &Value,
    ) {
        use crate::assembly::ast::Instruction::*;

        let assembly_type = Self::get_asm_type(src);
        let src_op = self.create_operand(src);
        let dst_op = self.create_operand(dst);
        let unary_op = self.map_unary_operator(op);

        instructions.push(Mov {
            assembly_type: assembly_type.clone(),
            src: src_op,
            dst: dst_op.clone(),
        });
        instructions.push(Unary {
            assembly_type,
            op: unary_op,
            operand: dst_op,
        });
    }

    fn push_binary_divide(
        &mut self,
        instructions: &mut Vec<Instruction>,
        src1: &Value,
        src2: &Value,
        dst: &Value,
    ) {
        use crate::assembly::ast::Instruction::*;

        let assembly_type = Self::get_asm_type(src1);
        let src1_op = self.create_operand(src1);
        let src2_op = self.create_operand(src2);
        let dst_op = self.create_operand(dst);

        instructions.push(Mov {
            assembly_type: assembly_type.clone(),
            src: src1_op,
            dst: Operand::Register(Register::AX),
        });
        instructions.push(Cdq(assembly_type.clone()));
        instructions.push(Idiv {
            assembly_type: assembly_type.clone(),
            operand: src2_op,
        });
        instructions.push(Mov {
            assembly_type,
            src: Operand::Register(Register::AX),
            dst: dst_op,
        });
    }

    fn push_binary_remainder(
        &mut self,
        instructions: &mut Vec<Instruction>,
        src1: &Value,
        src2: &Value,
        dst: &Value,
    ) {
        use crate::assembly::ast::Instruction::*;

        let assembly_type = Self::get_asm_type(src1);
        let src1_op = self.create_operand(src1);
        let src2_op = self.create_operand(src2);
        let dst_op = self.create_operand(dst);

        instructions.push(Mov {
            assembly_type: assembly_type.clone(),
            src: src1_op,
            dst: Operand::Register(Register::AX),
        });
        instructions.push(Cdq(assembly_type.clone()));
        instructions.push(Idiv {
            assembly_type: assembly_type.clone(),
            operand: src2_op,
        });
        instructions.push(Mov {
            assembly_type,
            src: Operand::Register(DX),
            dst: dst_op,
        });
    }

    fn push_binary(
        &mut self,
        instructions: &mut Vec<Instruction>,
        op: &TackyBinOp,
        src1: &Value,
        src2: &Value,
        dst: &Value,
    ) {
        match op {
            BinaryOperator::Equal
            | BinaryOperator::NotEqual
            | BinaryOperator::Greater
            | BinaryOperator::GreaterEqual
            | BinaryOperator::Less
            | BinaryOperator::LessEqual => {
                self.push_binary_relational(instructions, op, src1, src2, dst)
            }
            _ => self.push_binary_arithmetic(instructions, op, src1, src2, dst),
        }
    }

    fn push_binary_relational(
        &mut self,
        instructions: &mut Vec<Instruction>,
        op: &TackyBinOp,
        src1: &Value,
        src2: &Value,
        dst: &Value,
    ) {
        use crate::assembly::ast::Instruction::*;

        let src1_op = self.create_operand(src1);
        let src2_op = self.create_operand(src2);
        let dst_op = self.create_operand(dst);

        instructions.push(Cmp {
            assembly_type: Self::get_asm_type(src1),
            op1: src2_op,
            op2: src1_op,
        });
        let condition_code = self.map_relational_operator(op);
        instructions.push(Mov {
            assembly_type: Self::get_asm_type(dst),
            src: Operand::Immediate(0),
            dst: dst_op.clone(),
        });
        instructions.push(SetCC(condition_code, dst_op));
    }

    fn push_binary_arithmetic(
        &mut self,
        instructions: &mut Vec<Instruction>,
        op: &TackyBinOp,
        src1: &Value,
        src2: &Value,
        dst: &Value,
    ) {
        use crate::assembly::ast::Instruction::*;

        let assembly_type = Self::get_asm_type(src1);
        let src1_op = self.create_operand(src1);
        let src2_op = self.create_operand(src2);
        let dst_op = self.create_operand(dst);

        let binary_op = self.map_binary_operator(op);
        instructions.push(Mov {
            assembly_type: assembly_type.clone(),
            src: src1_op,
            dst: dst_op.clone(),
        });
        instructions.push(Binary {
            assembly_type,
            op: binary_op,
            left: src2_op,
            right: dst_op,
        });
    }

    fn push_jump(&mut self, instructions: &mut Vec<Instruction>, target: &str) {
        instructions.push(Instruction::Jmp(target.to_string()));
    }

    fn push_jump_if_zero(
        &mut self,
        instructions: &mut Vec<Instruction>,
        condition: &Value,
        target: &str,
    ) {
        use crate::assembly::ast::Instruction::*;

        let condition_op = self.create_operand(condition);
        instructions.push(Cmp {
            assembly_type: Self::get_asm_type(condition),
            op1: Operand::Immediate(0),
            op2: condition_op,
        });
        instructions.push(JmpCC(ConditionCode::Eq, target.to_string()));
    }

    fn push_jump_if_not_zero(
        &mut self,
        instructions: &mut Vec<Instruction>,
        condition: &Value,
        target: &str,
    ) {
        use crate::assembly::ast::Instruction::*;

        let condition_op = self.create_operand(condition);
        instructions.push(Cmp {
            assembly_type: Self::get_asm_type(condition),
            op1: Operand::Immediate(0),
            op2: condition_op,
        });
        instructions.push(JmpCC(ConditionCode::NotEq, target.to_string()));
    }

    fn push_copy(&mut self, instructions: &mut Vec<Instruction>, src: &Value, dst: &Value) {
        use crate::assembly::ast::Instruction::*;

        let src_op = self.create_operand(src);
        let dst_op = self.create_operand(dst);
        instructions.push(Mov {
            assembly_type: Self::get_asm_type(src),
            src: src_op,
            dst: dst_op,
        });
    }

    fn push_label(&mut self, instructions: &mut Vec<Instruction>, name: &str) {
        instructions.push(Instruction::Label(name.to_string()));
    }

    fn create_operand(&mut self, value: &Value) -> Operand {
        match value {
            Value::IntegerConstant(i) => Operand::Immediate(*i),
            Value::LongConstant(_l) => todo!("long constants not supported yet"),
            Value::Variable(name) => Operand::PseudoReg(name.clone()),
        }
    }

    fn map_unary_operator(&self, unary_op: &UnaryOperator) -> UnaryOp {
        use crate::tacky::ast::UnaryOperator::*;
        match unary_op {
            Negate => UnaryOp::Neg,
            Complement => UnaryOp::Not,
            _ => todo!("unsupported unary operator {:?}", unary_op),
        }
    }

    fn map_binary_operator(&self, binary_op: &TackyBinOp) -> crate::assembly::ast::BinaryOp {
        use crate::tacky::ast::BinaryOperator::*;
        match binary_op {
            Add => crate::assembly::ast::BinaryOp::Add,
            Subtract => crate::assembly::ast::BinaryOp::Sub,
            Multiply => crate::assembly::ast::BinaryOp::Mul,
            BitAnd => crate::assembly::ast::BinaryOp::BitAnd,
            BitOr => crate::assembly::ast::BinaryOp::BitOr,
            BitXor => crate::assembly::ast::BinaryOp::BitXor,
            ShiftLeft => crate::assembly::ast::BinaryOp::ShiftLeft,
            ShiftRight => crate::assembly::ast::BinaryOp::ShiftRight,
            Divide => unreachable!(),
            Remainder => unreachable!(),
            _ => unimplemented!("unsupported binary operator {:?}", binary_op),
        }
    }

    fn map_relational_operator(&self, relational_op: &TackyBinOp) -> ConditionCode {
        use crate::tacky::ast::BinaryOperator::*;
        match relational_op {
            Equal => ConditionCode::Eq,
            NotEqual => ConditionCode::NotEq,
            Greater => ConditionCode::Gt,
            GreaterEqual => ConditionCode::GtEq,
            Less => ConditionCode::Lt,
            LessEqual => ConditionCode::LtEq,
            _ => unimplemented!("unsupported relational operator {:?}", relational_op),
        }
    }

    fn map_type_to_asm_type(c_type: Type) -> AssemblyType {
        use crate::common::Type::*;
        match c_type {
            Int => AssemblyType::Longword,
            Long => AssemblyType::Quadword,
            _ => unimplemented!("unsupported type {:?}", c_type),
        }
    }

    fn lookup_asm_type(name: &str) -> AssemblyType {
        match symbol_table::get(name) {
            Some(entry) => Self::map_type_to_asm_type(entry.c_type),
            None => panic!("Symbol not found: {}", name),
        }
    }

    fn get_asm_type(value: &Value) -> AssemblyType {
        match value {
            Value::IntegerConstant(_) => AssemblyType::Longword,
            Value::LongConstant(_) => AssemblyType::Quadword,
            Value::Variable(name) => Self::lookup_asm_type(name),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assembly::ast::{
        BinaryOp as AsmBinaryOp, Instruction as AsmInstruction, Operand as AsmOperand,
        Register as AsmRegister,
    };
    use crate::lexer::Lexer;
    use crate::parser::Parser;
    use crate::semantic;
    use crate::semantic::NameGeneratorRef;
    use crate::tacky::TackyEmitter;
    use crate::tacky::ast::{
        BinaryOperator, Function as TackyFunctionDef, Instruction as TackyInstruction,
        Program as TackyProgram, Value,
    };
    use anyhow::Result;
    use crate::common::symbol_table::IdentAttrs;

    fn make_emitter(
        label_name_gen: &NameGeneratorRef,
        tmp_var_name_gen: &NameGeneratorRef,
    ) -> TackyEmitter {
        TackyEmitter::new(label_name_gen.clone(), tmp_var_name_gen.clone())
    }

    fn validate(
        var_name_gen: &NameGeneratorRef,
        label_name_gen: &NameGeneratorRef,
        program: &mut crate::ast::Program,
    ) -> Result<()> {
        semantic::validate(var_name_gen, label_name_gen, program)
    }

    #[test]
    fn creates_asm_program_ok() {
        let code = "int main(void) { return 42 >> 1; }";

        let lexer = Lexer::new();
        let tokens = lexer.scan_tokens(code).expect("Failed to scan tokens");

        let parser = Parser::new();
        let mut program = parser.parse(tokens).expect("Failed to parse program");

        let var_name_gen = semantic::make_var_name_generator();
        let label_name_gen = semantic::make_label_name_generator();
        let tmp_var_name_gen = semantic::make_temp_var_name_generator();

        symbol_table::clear();

        validate(&var_name_gen, &label_name_gen, &mut program)
            .expect("Failed to validate program");

        let mut tacky_emitter = make_emitter(&label_name_gen, &tmp_var_name_gen);
        let tacky_program = tacky_emitter
            .emit_program(&program)
            .expect("Failed to emit");

        let mut assembly_creator = AssemblyCreator::new();
        let assembly_program = assembly_creator
            .create_program(&tacky_program)
            .expect("Failed to create assembly program");

        dbg!(&assembly_program);
    }

    #[test]
    fn creates_asm_program_with_binary_ops() {

        symbol_table::clear();
        for i in 0..=4 {
            let var_name = format!("tmp.{}", i);
            symbol_table::insert(&var_name, symbol_table::SymbolTableEntry {
                c_type: Type::Int,
                attrs: IdentAttrs::Local,
            });
        }

        let tacky_program = TackyProgram(vec![TopLevel::Function(TackyFunctionDef {
            name: "main".to_string(),
            is_global: true,
            parameters: vec![],
            body: vec![
                TackyInstruction::Binary {
                    op: BinaryOperator::Add,
                    src1: Value::IntegerConstant(1),
                    src2: Value::IntegerConstant(2),
                    dst: Value::Variable("tmp.0".to_string()),
                },
                TackyInstruction::Binary {
                    op: BinaryOperator::Subtract,
                    src1: Value::Variable("tmp.0".to_string()),
                    src2: Value::IntegerConstant(3),
                    dst: Value::Variable("tmp.1".to_string()),
                },
                TackyInstruction::Binary {
                    op: BinaryOperator::Multiply,
                    src1: Value::Variable("tmp.1".to_string()),
                    src2: Value::IntegerConstant(4),
                    dst: Value::Variable("tmp.2".to_string()),
                },
                TackyInstruction::Binary {
                    op: BinaryOperator::Divide,
                    src1: Value::Variable("tmp.2".to_string()),
                    src2: Value::IntegerConstant(5),
                    dst: Value::Variable("tmp.3".to_string()),
                },
                TackyInstruction::Binary {
                    op: BinaryOperator::Remainder,
                    src1: Value::Variable("tmp.3".to_string()),
                    src2: Value::IntegerConstant(2),
                    dst: Value::Variable("tmp.4".to_string()),
                },
                TackyInstruction::Return(Value::Variable("tmp.4".to_string())),
            ],
        })]);

        let mut assembly_creator = AssemblyCreator::new();
        let assembly_program = assembly_creator
            .create_program(&tacky_program)
            .expect("Failed to create assembly program");

        let main_func = if let TopLevelAsm::Function(func) = &assembly_program.top_levels[0] {
            func
        } else {
            panic!("Expected function");
        };

        let instructions = &main_func.instructions;
        assert_eq!(instructions.len(), 16);

        assert!(matches!(
            &instructions[0],
            AsmInstruction::Mov {
                assembly_type: AssemblyType::Longword,
                src: AsmOperand::Immediate(1),
                dst: AsmOperand::PseudoReg(name)
            } if name == "tmp.0"
        ));
        assert!(matches!(
            &instructions[1],
            AsmInstruction::Binary {
                assembly_type: AssemblyType::Longword,
                op: AsmBinaryOp::Add,
                left: AsmOperand::Immediate(2),
                right: AsmOperand::PseudoReg(name)
            } if name == "tmp.0"
        ));

        assert!(matches!(
            &instructions[2],
            AsmInstruction::Mov {
                assembly_type: AssemblyType::Longword,
                src: AsmOperand::PseudoReg(src),
                dst: AsmOperand::PseudoReg(dst)
            } if src == "tmp.0" && dst == "tmp.1"
        ));
        assert!(matches!(
            &instructions[3],
            AsmInstruction::Binary {
                assembly_type: AssemblyType::Longword,
                op: AsmBinaryOp::Sub,
                left: AsmOperand::Immediate(3),
                right: AsmOperand::PseudoReg(name)
            } if name == "tmp.1"
        ));

        assert!(matches!(
            &instructions[4],
            AsmInstruction::Mov {
                assembly_type: AssemblyType::Longword,
                src: AsmOperand::PseudoReg(src),
                dst: AsmOperand::PseudoReg(dst)
            } if src == "tmp.1" && dst == "tmp.2"
        ));
        assert!(matches!(
            &instructions[5],
            AsmInstruction::Binary {
                assembly_type: AssemblyType::Longword,
                op: AsmBinaryOp::Mul,
                left: AsmOperand::Immediate(4),
                right: AsmOperand::PseudoReg(name)
            } if name == "tmp.2"
        ));

        assert!(matches!(
            &instructions[6],
            AsmInstruction::Mov {
                assembly_type: AssemblyType::Longword,
                src: AsmOperand::PseudoReg(name),
                dst: AsmOperand::Register(AsmRegister::AX)
            } if name == "tmp.2"
        ));
        assert!(matches!(
            &instructions[7],
            AsmInstruction::Cdq(AssemblyType::Longword)
        ));
        assert!(matches!(
            &instructions[8],
            AsmInstruction::Idiv {
                assembly_type: AssemblyType::Longword,
                operand: AsmOperand::Immediate(5),
            },
        ));
        assert!(matches!(
            &instructions[9],
            AsmInstruction::Mov {
                assembly_type: AssemblyType::Longword,
                src: AsmOperand::Register(AsmRegister::AX),
                dst: AsmOperand::PseudoReg(name)
            } if name == "tmp.3"
        ));

        assert!(matches!(
            &instructions[10],
            AsmInstruction::Mov {
                assembly_type: AssemblyType::Longword,
                src: AsmOperand::PseudoReg(name),
                dst: AsmOperand::Register(AsmRegister::AX)
            } if name == "tmp.3"
        ));
        assert!(matches!(
            &instructions[11],
            AsmInstruction::Cdq(AssemblyType::Longword)
        ));
        assert!(matches!(
            &instructions[12],
            AsmInstruction::Idiv {
                assembly_type: AssemblyType::Longword,
                operand: AsmOperand::Immediate(2),
            },
        ));
        assert!(matches!(
            &instructions[13],
            AsmInstruction::Mov {
                assembly_type: AssemblyType::Longword,
                src: AsmOperand::Register(DX),
                dst: AsmOperand::PseudoReg(name)
            } if name == "tmp.4"
        ));

        assert!(matches!(
            &instructions[14],
            AsmInstruction::Mov {
                assembly_type: AssemblyType::Longword,
                src: AsmOperand::PseudoReg(name),
                dst: AsmOperand::Register(AsmRegister::AX)
            } if name == "tmp.4"
        ));
        assert!(matches!(&instructions[15], AsmInstruction::Ret));
    }
}

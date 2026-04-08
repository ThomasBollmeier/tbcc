use crate::assembly::ast::{FuncDef, Instruction, Operand, Program, Register, UnaryOp};
use crate::tacky::{
    BinaryOperator as TackyBinOp, FunctionDef, Instruction as TackyInstruction, UnaryOperator,
    Value,
};

#[derive(Debug)]
pub struct AssemblyCreator;

impl AssemblyCreator {
    pub fn new() -> AssemblyCreator {
        AssemblyCreator
    }

    pub fn create_program(
        &mut self,
        tacky_program: &crate::tacky::Program,
    ) -> anyhow::Result<Program> {
        let func_def = self.create_func_def(&tacky_program.func_def)?;

        Ok(Program(func_def))
    }

    fn create_func_def(&mut self, func_def: &FunctionDef) -> anyhow::Result<FuncDef> {
        let name = func_def.name.clone();
        let instructions = self.create_instructions(&func_def.body)?;

        Ok(FuncDef::new(name, instructions))
    }

    fn create_instructions(
        &mut self,
        instructions: &Vec<TackyInstruction>,
    ) -> anyhow::Result<Vec<Instruction>> {
        use crate::assembly::ast::Instruction::*;
        let mut ret = vec![];

        for instruction in instructions {
            match instruction {
                TackyInstruction::Return(value) => {
                    let src = self.create_operand(value);
                    ret.push(Mov {
                        src,
                        dst: Operand::Register(Register::AX),
                    });
                    ret.push(Ret);
                }
                TackyInstruction::Unary { op, src, dst } => {
                    let src_op = self.create_operand(src);
                    let dst_op = self.create_operand(dst);
                    let unary_op = self.map_unary_operator(op);

                    ret.push(Mov {
                        src: src_op,
                        dst: dst_op.clone(),
                    });
                    ret.push(Unary {
                        op: unary_op,
                        operand: dst_op,
                    });
                }
                TackyInstruction::Binary {
                    op: TackyBinOp::Divide,
                    src1,
                    src2,
                    dst,
                } => {
                    let src1_op = self.create_operand(src1);
                    let src2_op = self.create_operand(src2);
                    let dst_op = self.create_operand(dst);

                    ret.push(Mov {
                        src: src1_op,
                        dst: Operand::Register(Register::AX),
                    });
                    ret.push(Cdq);
                    ret.push(Idiv(src2_op));
                    ret.push(Mov {
                        src: Operand::Register(Register::AX),
                        dst: dst_op,
                    });
                }
                TackyInstruction::Binary {
                    op: TackyBinOp::Remainder,
                    src1,
                    src2,
                    dst,
                } => {
                    let src1_op = self.create_operand(src1);
                    let src2_op = self.create_operand(src2);
                    let dst_op = self.create_operand(dst);

                    ret.push(Mov {
                        src: src1_op,
                        dst: Operand::Register(Register::AX),
                    });
                    ret.push(Cdq);
                    ret.push(Idiv(src2_op));
                    ret.push(Mov {
                        src: Operand::Register(Register::DX),
                        dst: dst_op,
                    });
                }
                TackyInstruction::Binary {
                    op,
                    src1,
                    src2,
                    dst,
                } => {
                    let src1_op = self.create_operand(src1);
                    let src2_op = self.create_operand(src2);
                    let dst_op = self.create_operand(dst);

                    let binary_op = self.map_binary_operator(op);
                    ret.push(Mov {
                        src: src1_op,
                        dst: dst_op.clone(),
                    });
                    ret.push(Binary {
                        op: binary_op,
                        left: src2_op,
                        right: dst_op,
                    });
                }
                _ => todo!("unsupported instruction {:?}", instruction),
            }
        }

        Ok(ret)
    }

    fn create_operand(&mut self, value: &Value) -> Operand {
        match value {
            Value::IntegerConstant(i) => Operand::Immediate(*i),
            Value::Variable(name) => Operand::PseudoReg(name.clone()),
        }
    }

    fn map_unary_operator(&self, unary_op: &UnaryOperator) -> UnaryOp {
        use crate::tacky::UnaryOperator::*;
        match unary_op {
            Negate => UnaryOp::Neg,
            Complement => UnaryOp::Not,
            _ => todo!("unsupported unary operator {:?}", unary_op),
        }
    }

    fn map_binary_operator(&self, binary_op: &TackyBinOp) -> crate::assembly::ast::BinaryOp {
        use crate::tacky::BinaryOperator::*;
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
            _ => todo!("unsupported binary operator {:?}", binary_op),
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
    use crate::tacky::{
        BinaryOperator, FunctionDef as TackyFunctionDef, Instruction as TackyInstruction,
        Program as TackyProgram, TackyEmitter, Value,
    };

    #[test]
    fn creates_asm_program_ok() {
        let code = "int main(void) { return 42 >> 1; }";

        let lexer = Lexer::new();
        let tokens = lexer.scan_tokens(code).expect("Failed to scan tokens");

        let parser = Parser::new();
        let program = parser.parse(tokens).expect("Failed to parse program");

        let mut tacky_emitter = TackyEmitter::new();
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
        let tacky_program = TackyProgram {
            func_def: TackyFunctionDef {
                name: "main".to_string(),
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
            },
        };

        let mut assembly_creator = AssemblyCreator::new();
        let assembly_program = assembly_creator
            .create_program(&tacky_program)
            .expect("Failed to create assembly program");

        let instructions = &assembly_program.0.instructions;
        assert_eq!(instructions.len(), 16);

        assert!(matches!(
            &instructions[0],
            AsmInstruction::Mov {
                src: AsmOperand::Immediate(1),
                dst: AsmOperand::PseudoReg(name)
            } if name == "tmp.0"
        ));
        assert!(matches!(
            &instructions[1],
            AsmInstruction::Binary {
                op: AsmBinaryOp::Add,
                left: AsmOperand::Immediate(2),
                right: AsmOperand::PseudoReg(name)
            } if name == "tmp.0"
        ));

        assert!(matches!(
            &instructions[2],
            AsmInstruction::Mov {
                src: AsmOperand::PseudoReg(src),
                dst: AsmOperand::PseudoReg(dst)
            } if src == "tmp.0" && dst == "tmp.1"
        ));
        assert!(matches!(
            &instructions[3],
            AsmInstruction::Binary {
                op: AsmBinaryOp::Sub,
                left: AsmOperand::Immediate(3),
                right: AsmOperand::PseudoReg(name)
            } if name == "tmp.1"
        ));

        assert!(matches!(
            &instructions[4],
            AsmInstruction::Mov {
                src: AsmOperand::PseudoReg(src),
                dst: AsmOperand::PseudoReg(dst)
            } if src == "tmp.1" && dst == "tmp.2"
        ));
        assert!(matches!(
            &instructions[5],
            AsmInstruction::Binary {
                op: AsmBinaryOp::Mul,
                left: AsmOperand::Immediate(4),
                right: AsmOperand::PseudoReg(name)
            } if name == "tmp.2"
        ));

        assert!(matches!(
            &instructions[6],
            AsmInstruction::Mov {
                src: AsmOperand::PseudoReg(name),
                dst: AsmOperand::Register(AsmRegister::AX)
            } if name == "tmp.2"
        ));
        assert!(matches!(&instructions[7], AsmInstruction::Cdq));
        assert!(matches!(
            &instructions[8],
            AsmInstruction::Idiv(AsmOperand::Immediate(5))
        ));
        assert!(matches!(
            &instructions[9],
            AsmInstruction::Mov {
                src: AsmOperand::Register(AsmRegister::AX),
                dst: AsmOperand::PseudoReg(name)
            } if name == "tmp.3"
        ));

        assert!(matches!(
            &instructions[10],
            AsmInstruction::Mov {
                src: AsmOperand::PseudoReg(name),
                dst: AsmOperand::Register(AsmRegister::AX)
            } if name == "tmp.3"
        ));
        assert!(matches!(&instructions[11], AsmInstruction::Cdq));
        assert!(matches!(
            &instructions[12],
            AsmInstruction::Idiv(AsmOperand::Immediate(2))
        ));
        assert!(matches!(
            &instructions[13],
            AsmInstruction::Mov {
                src: AsmOperand::Register(AsmRegister::DX),
                dst: AsmOperand::PseudoReg(name)
            } if name == "tmp.4"
        ));

        assert!(matches!(
            &instructions[14],
            AsmInstruction::Mov {
                src: AsmOperand::PseudoReg(name),
                dst: AsmOperand::Register(AsmRegister::AX)
            } if name == "tmp.4"
        ));
        assert!(matches!(&instructions[15], AsmInstruction::Ret));
    }
}

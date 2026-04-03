use crate::assembly::ast::{FuncDef, Instruction, Operand, Program, Register, UnaryOp};
use crate::tacky::{FunctionDef, UnaryOperator, Value};

#[derive(Debug)]
pub struct AssemblyCreator;

impl AssemblyCreator {
    pub fn new() -> AssemblyCreator {
        AssemblyCreator
    }

    pub fn create_program(&mut self, tacky_program: &crate::tacky::Program) -> anyhow::Result<Program> {
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
        instructions: &Vec<crate::tacky::Instruction>,
    ) -> anyhow::Result<Vec<Instruction>> {
        use crate::assembly::ast::Instruction::*;
        let mut ret = vec![];

        for instruction in instructions {
            match instruction {
                crate::tacky::Instruction::Return(value) => {
                    let src = self.create_operand(value);
                    ret.push(Mov {
                        src,
                        dst: Operand::Register(Register::AX),
                    });
                    ret.push(Ret);
                }
                crate::tacky::Instruction::Unary { op, src, dst } => {
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

    fn map_unary_operator(&mut self, unary_op: &UnaryOperator) -> UnaryOp {
        use crate::tacky::UnaryOperator::*;
        match unary_op {
            Negate => UnaryOp::Neg,
            Complement => UnaryOp::Not,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;
    use crate::tacky::TackyEmitter;

    #[test]
    fn creates_asm_program_ok() {

        let code = "int main(void) { return 42; }";

        let lexer = Lexer::new();
        let tokens = lexer.scan_tokens(code).expect("Failed to scan tokens");

        let parser = Parser::new();
        let program = parser.parse(tokens).expect("Failed to parse program");

        let mut tacky_emitter = TackyEmitter::new();
        let tacky_program = tacky_emitter.emit_program(&program).expect("Failed to emit");

        let mut assembly_creator = AssemblyCreator::new();
        let assembly_program = assembly_creator
            .create_program(&tacky_program)
            .expect("Failed to create assembly program");

        dbg!(&assembly_program);
    }
}

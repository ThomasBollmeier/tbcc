use crate::ast::Program as AstProgram;
use crate::ast::{Expression, FunctionDefinition, Statement, Visitor};
use anyhow::{Result, anyhow};

#[derive(Debug)]
pub enum ASTNode {
    Program(Program),
    FuncDef(FuncDef),
    Instruction(Instruction),
    Operand(Operand),
}

#[derive(Debug)]
pub struct Program(FuncDef);

#[derive(Debug)]
pub struct FuncDef {
    pub name: String,
    pub instructions: Vec<Instruction>,
}

#[derive(Debug)]
pub enum Instruction {
    Mov { src: Operand, dst: Operand },
    Ret,
}

#[derive(Debug)]
pub enum Operand {
    Immediate(i32),
    Register,
}

#[derive(Debug)]
pub struct AssemblyCreator;

impl AssemblyCreator {
    pub fn new() -> AssemblyCreator {
        AssemblyCreator
    }

    pub fn create_assembly_program(&mut self, ast_program: &AstProgram) -> Result<Program> {
        if let ASTNode::Program(program) = ast_program.accept(self)? {
            Ok(program)
        } else {
            Err(anyhow!("Expected Program node"))
        }
    }

    fn get_instructions(&mut self, stmt: &Statement) -> Result<Vec<Instruction>> {
        let mut instructions: Vec<Instruction> = Vec::new();

        match stmt {
            Statement::Return(expr) => {
                if let Ok(ASTNode::Operand(operand)) = expr.accept(self) {
                    instructions.push(Instruction::Mov {
                        src: operand,
                        dst: Operand::Register,
                    });
                    instructions.push(Instruction::Ret);
                } else {
                    return Err(anyhow!("Expected Operand node"));
                }
            }
        }

        Ok(instructions)
    }
}

impl Visitor<Result<ASTNode>> for AssemblyCreator {
    fn visit_program(&mut self, program: &AstProgram) -> Result<ASTNode> {
        let func_def = program.function_definition.accept(self);
        if let Ok(ASTNode::FuncDef(func_def)) = func_def {
            Ok(ASTNode::Program(Program(func_def)))
        } else {
            Err(anyhow!("Expected FuncDef node"))
        }
    }

    fn visit_function_definition(&mut self, func_def: &FunctionDefinition) -> Result<ASTNode> {
        let name = func_def.name.clone();
        let instructions = self.get_instructions(&func_def.body)?;
        Ok(ASTNode::FuncDef(FuncDef { name, instructions }))
    }

    fn visit_statement(&mut self, _stmt: &Statement) -> Result<ASTNode> {
        unimplemented!()
    }

    fn visit_expression(&mut self, expr: &Expression) -> Result<ASTNode> {
        match expr {
            Expression::IntegerConstant(value) => Ok(ASTNode::Operand(Operand::Immediate(*value))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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

        dbg!(&assembly_program);
    }
}

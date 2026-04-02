use crate::ast::{Expression, FunctionDefinition, Statement, UnaryOp, Visitor as AstVisitor};
use crate::tacky::Instruction::Unary;
use anyhow::Result;

#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Program(Program),
    FunctionDef(FunctionDef),
    Instruction(Instruction),
    Value(Value),
    UnaryOperator(UnaryOperator),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub func_def: FunctionDef,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDef {
    pub name: String,
    pub body: Vec<Instruction>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Instruction {
    Return(Value),
    Unary {
        op: UnaryOperator,
        src: Value,
        dst: Value,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    IntegerConstant(i32),
    Variable(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOperator {
    Complement,
    Negate,
}

#[derive(Debug, Clone)]
pub struct TackyEmitter {
    cnt_tmp_vars: usize,
}

impl TackyEmitter {
    pub fn new() -> TackyEmitter {
        TackyEmitter { cnt_tmp_vars: 0 }
    }

    pub fn emit_program(&mut self, program: &crate::ast::Program) -> Result<Node> {
        program.accept(self)
    }

    fn emit_statement(&mut self, stmt: &Statement) -> Vec<Instruction> {
        match stmt {
            Statement::Return(expr) => {
                let mut instructions = vec![];
                let value = self.emit_expression(expr, &mut instructions);
                instructions.push(Instruction::Return(value));
                instructions
            }
        }
    }

    fn emit_expression(&mut self, expr: &Expression, instructions: &mut Vec<Instruction>) -> Value {
        match expr {
            Expression::IntegerConstant(value) => Value::IntegerConstant(*value),
            Expression::UnaryExpr(op, expr) => {
                let src = self.emit_expression(expr, instructions);
                let dst = Value::Variable(self.make_temp_var());
                let op = match op {
                    UnaryOp::Complement => UnaryOperator::Complement,
                    UnaryOp::Negate => UnaryOperator::Negate,
                };
                instructions.push(Unary {
                    op,
                    src,
                    dst: dst.clone(),
                });
                dst
            }
        }
    }

    fn make_temp_var(&mut self) -> String {
        let ret = format!("tmp.{}", self.cnt_tmp_vars);
        self.cnt_tmp_vars += 1;
        ret
    }
}

impl AstVisitor<Result<Node>> for TackyEmitter {
    fn visit_program(&mut self, program: &crate::ast::Program) -> Result<Node> {
        let func_def = program.function_definition.accept(self)?;

        if let Node::FunctionDef(func_def) = func_def {
            Ok(Node::Program(Program { func_def }))
        } else {
            Err(anyhow::anyhow!("Not a function"))
        }
    }

    fn visit_function_definition(&mut self, func_def: &FunctionDefinition) -> Result<Node> {
        let name = func_def.name.clone();
        let instructions = self.emit_statement(&func_def.body);

        Ok(Node::FunctionDef(FunctionDef {
            name,
            body: instructions,
        }))
    }

    fn visit_statement(&mut self, _stmt: &Statement) -> Result<Node> {
        unimplemented!()
    }

    fn visit_expression(&mut self, _expr: &Expression) -> Result<Node> {
        unimplemented!()
    }
}

#[cfg(test)]
mod tests {
    use crate::tacky::UnaryOperator::{Complement, Negate};
    use super::*;

    #[test]
    fn emit_return() {
        let mut emitter = TackyEmitter::new();
        let stmt = Statement::Return(Expression::IntegerConstant(42));
        let expected = vec![Instruction::Return(
            Value::IntegerConstant(42),
        )];
        let actual = emitter.emit_statement(&stmt);

        assert_eq!(expected, actual);
    }

    #[test]
    fn emit_complex_return() {
        use Expression::*;
        use Instruction::*;
        use Value::*;

        let mut emitter = TackyEmitter::new();
        let stmt = Statement::Return(UnaryExpr(
            UnaryOp::Negate,
            Box::new(UnaryExpr(
                UnaryOp::Complement,
                Box::new(UnaryExpr(
                    UnaryOp::Negate,
                    Box::new(Expression::IntegerConstant(42)),
                )),
            )),
        ));
        let expected = vec![
            Unary { op: Negate, src: Value::IntegerConstant(42), dst: Variable("tmp.0".to_string()) },
            Unary { op: Complement, src: Variable("tmp.0".to_string()), dst: Variable("tmp.1".to_string()) },
            Unary { op: Negate, src: Variable("tmp.1".to_string()), dst: Variable("tmp.2".to_string()) },
            Return(Variable("tmp.2".to_string())),
        ];
        let actual = emitter.emit_statement(&stmt);

        assert_eq!(expected, actual);
    }
}

use crate::ast::{BinaryOp, Expression, FunctionDefinition, Statement, UnaryOp};
use crate::tacky::Instruction::Unary;
use anyhow::Result;

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
    Binary {
        op: BinaryOperator,
        src1: Value,
        src2: Value,
        dst: Value,
    }
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

#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
    BitAnd,
    BitOr,
    BitXor,
    ShiftLeft,
    ShiftRight,
}

#[derive(Debug, Clone)]
pub struct TackyEmitter {
    cnt_tmp_vars: usize,
}

impl TackyEmitter {
    pub fn new() -> TackyEmitter {
        TackyEmitter { cnt_tmp_vars: 0 }
    }

    pub fn emit_program(&mut self, program: &crate::ast::Program) -> Result<Program> {
        let func_def = self.emit_function_def(&program.function_definition)?;

        Ok(Program { func_def })
    }

    fn emit_function_def(&mut self, function_definition: &FunctionDefinition) -> Result<FunctionDef> {
        let name = function_definition.name.clone();
        let instructions = self.emit_statement(&function_definition.body);

        Ok(FunctionDef {
            name,
            body: instructions,
        })
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
            Expression::BinaryExpr(op, left, right) => {
                let src1 = self.emit_expression(left, instructions);
                let src2 = self.emit_expression(right, instructions);
                let dst = Value::Variable(self.make_temp_var());
                let op = match op {
                    BinaryOp::Add => BinaryOperator::Add,
                    BinaryOp::Subtract => BinaryOperator::Subtract,
                    BinaryOp::Multiply => BinaryOperator::Multiply,
                    BinaryOp::Divide => BinaryOperator::Divide,
                    BinaryOp::Remainder => BinaryOperator::Remainder,
                    BinaryOp::BitAnd => BinaryOperator::BitAnd,
                    BinaryOp::BitOr => BinaryOperator::BitOr,
                    BinaryOp::BitXor => BinaryOperator::BitXor,
                    BinaryOp::ShiftLeft => BinaryOperator::ShiftLeft,
                    BinaryOp::ShiftRight => BinaryOperator::ShiftRight,
                };
                instructions.push(Instruction::Binary {
                    op,
                    src1,
                    src2,
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

    #[test]
    fn emit_binary_expr() {
        use BinaryOperator::*;
        use Expression::*;
        use Instruction::*;
        use Value::*;

        let mut emitter = TackyEmitter::new();
        let stmt = Statement::Return(BinaryExpr(
            BinaryOp::Add,
            Box::new(Expression::IntegerConstant(1)),
            Box::new(BinaryExpr(
                BinaryOp::Multiply,
                Box::new(Expression::IntegerConstant(2)),
                Box::new(Expression::IntegerConstant(3)),
            )),
        ));

        let expected = vec![
            Binary {
                op: Multiply,
                src1: Value::IntegerConstant(2),
                src2: Value::IntegerConstant(3),
                dst: Variable("tmp.0".to_string()),
            },
            Binary {
                op: Add,
                src1: Value::IntegerConstant(1),
                src2: Variable("tmp.0".to_string()),
                dst: Variable("tmp.1".to_string()),
            },
            Return(Variable("tmp.1".to_string())),
        ];

        let actual = emitter.emit_statement(&stmt);
        assert_eq!(expected, actual);
    }

    #[test]
    fn emit_shift_left_expr() {
        use BinaryOperator::*;
        use Expression::*;
        use Instruction::*;
        use Value::*;

        let mut emitter = TackyEmitter::new();
        let stmt = Statement::Return(BinaryExpr(
            BinaryOp::ShiftLeft,
            Box::new(Expression::IntegerConstant(8)),
            Box::new(Expression::IntegerConstant(2)),
        ));

        let expected = vec![
            Binary {
                op: ShiftLeft,
                src1: Value::IntegerConstant(8),
                src2: Value::IntegerConstant(2),
                dst: Variable("tmp.0".to_string()),
            },
            Return(Variable("tmp.0".to_string())),
        ];

        let actual = emitter.emit_statement(&stmt);
        assert_eq!(expected, actual);
    }

    #[test]
    fn emit_shift_right_expr() {
        use BinaryOperator::*;
        use Expression::*;
        use Instruction::*;
        use Value::*;

        let mut emitter = TackyEmitter::new();
        let stmt = Statement::Return(BinaryExpr(
            BinaryOp::ShiftRight,
            Box::new(Expression::IntegerConstant(16)),
            Box::new(Expression::IntegerConstant(1)),
        ));

        let expected = vec![
            Binary {
                op: ShiftRight,
                src1: Value::IntegerConstant(16),
                src2: Value::IntegerConstant(1),
                dst: Variable("tmp.0".to_string()),
            },
            Return(Variable("tmp.0".to_string())),
        ];

        let actual = emitter.emit_statement(&stmt);
        assert_eq!(expected, actual);
    }
}

use crate::ast::{
    BinaryOp, BlockItem, Declaration, Expression, FunctionDefinition, Statement, UnaryOp,
};
use crate::semantic::NameCreatorRef;
use crate::tacky::Instruction::Unary;
use crate::tacky::Value::IntegerConstant;
use anyhow::Result;
use std::collections::HashMap;

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
    },
    Copy {
        src: Value,
        dst: Value,
    },
    Jump {
        target: String,
    },
    JumpIfZero {
        condition: Value,
        target: String,
    },
    JumpIfNotZero {
        condition: Value,
        target: String,
    },
    Label(String),
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
    Not,
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
    LogicalAnd,
    LogicalOr,
    Equal,
    NotEqual,
    Greater,
    Less,
    GreaterEqual,
    LessEqual,
}

#[derive(Debug, Clone)]
pub struct TackyEmitter {
    name_creator: NameCreatorRef,
    label_counters: HashMap<String, usize>,
}

impl TackyEmitter {
    pub fn new(name_creator: NameCreatorRef) -> TackyEmitter {
        TackyEmitter {
            name_creator,
            label_counters: HashMap::new(),
        }
    }

    pub fn emit_program(&mut self, program: &crate::ast::Program) -> Result<Program> {
        let func_def = self.emit_function_def(&program.function_definition)?;

        Ok(Program { func_def })
    }

    fn emit_function_def(
        &mut self,
        function_definition: &FunctionDefinition,
    ) -> Result<FunctionDef> {
        let name = function_definition.name.clone();
        let mut instructions = self.emit_block(&function_definition.body);
        instructions.push(Instruction::Return(IntegerConstant(0))); // Ensure function ends with a return

        Ok(FunctionDef {
            name,
            body: instructions,
        })
    }

    fn emit_block(&mut self, items: &Vec<BlockItem>) -> Vec<Instruction> {
        let mut instructions = vec![];
        for item in items {
            match item {
                BlockItem::Declaration(decl) => {
                    instructions.extend(self.emit_declaration(decl));
                }
                BlockItem::Statement(stmt) => {
                    instructions.extend(self.emit_statement(stmt));
                }
            }
        }
        instructions
    }

    fn emit_declaration(&mut self, declaration: &Declaration) -> Vec<Instruction> {
        let mut instructions = vec![];

        if let Some(expr) = &declaration.init_expr {
            let init_value = self.emit_expression(expr, &mut instructions);
            let var_name = declaration.name.clone();
            instructions.push(Instruction::Copy {
                src: init_value,
                dst: Value::Variable(var_name),
            });
        }

        instructions
    }

    fn emit_statement(&mut self, stmt: &Statement) -> Vec<Instruction> {
        match stmt {
            Statement::Return(expr) => {
                let mut instructions = vec![];
                let value = self.emit_expression(expr, &mut instructions);
                instructions.push(Instruction::Return(value));
                instructions
            }
            Statement::Expression(expr) => {
                let mut instructions = vec![];
                self.emit_expression(expr, &mut instructions);
                instructions
            }
            Statement::Null => vec![],
            _ => todo!("Unsupported statement type {:?}", stmt),
        }
    }

    fn emit_expression(&mut self, expr: &Expression, instructions: &mut Vec<Instruction>) -> Value {
        match expr {
            Expression::IntegerConstant(value) => IntegerConstant(*value),
            Expression::UnaryExpr(op, expr) => {
                let op = self.unary_op(op);
                let src = self.emit_expression(expr, instructions);
                let dst = Value::Variable(self.make_temp_var());
                instructions.push(Unary {
                    op,
                    src,
                    dst: dst.clone(),
                });
                dst
            }
            Expression::BinaryExpr(BinaryOp::LogicalAnd, left, right) => {
                let end_label = self.make_label("and_end");
                let false_label = self.make_label("and_false");
                let result = Value::Variable(self.make_temp_var());

                let val1 = self.emit_expression(left, instructions);
                instructions.push(Instruction::JumpIfZero {
                    condition: val1.clone(),
                    target: false_label.clone(),
                });

                let val2 = self.emit_expression(right, instructions);
                instructions.push(Instruction::JumpIfZero {
                    condition: val2.clone(),
                    target: false_label.clone(),
                });

                instructions.push(Instruction::Copy {
                    src: IntegerConstant(1),
                    dst: result.clone(),
                });
                instructions.push(Instruction::Jump {
                    target: end_label.clone(),
                });

                instructions.push(Instruction::Label(false_label));
                instructions.push(Instruction::Copy {
                    src: IntegerConstant(0),
                    dst: result.clone(),
                });

                instructions.push(Instruction::Label(end_label));

                result
            }
            Expression::BinaryExpr(BinaryOp::LogicalOr, left, right) => {
                let end_label = self.make_label("or_end");
                let true_label = self.make_label("or_true");
                let result = Value::Variable(self.make_temp_var());

                let val1 = self.emit_expression(left, instructions);
                instructions.push(Instruction::JumpIfNotZero {
                    condition: val1.clone(),
                    target: true_label.clone(),
                });

                let val2 = self.emit_expression(right, instructions);
                instructions.push(Instruction::JumpIfNotZero {
                    condition: val2.clone(),
                    target: true_label.clone(),
                });

                instructions.push(Instruction::Copy {
                    src: IntegerConstant(0),
                    dst: result.clone(),
                });
                instructions.push(Instruction::Jump {
                    target: end_label.clone(),
                });

                instructions.push(Instruction::Label(true_label));
                instructions.push(Instruction::Copy {
                    src: IntegerConstant(1),
                    dst: result.clone(),
                });

                instructions.push(Instruction::Label(end_label));

                result
            }
            Expression::BinaryExpr(op, left, right) => {
                let src1 = self.emit_expression(left, instructions);
                let src2 = self.emit_expression(right, instructions);
                let dst = Value::Variable(self.make_temp_var());
                let op = self.binary_op(op);
                instructions.push(Instruction::Binary {
                    op,
                    src1,
                    src2,
                    dst: dst.clone(),
                });
                dst
            }
            Expression::Assignment {
                left,
                right,
                is_postfix,
            } => {
                let src = self.emit_expression(right, instructions);
                let dst = self.emit_expression(left, instructions);
                if !is_postfix {
                    instructions.push(Instruction::Copy {
                        src,
                        dst: dst.clone(),
                    });
                    dst
                } else {
                    let original_value = Value::Variable(self.make_temp_var());
                    instructions.push(Instruction::Copy {
                        src: dst.clone(),
                        dst: original_value.clone(),
                    });
                    instructions.push(Instruction::Copy {
                        src,
                        dst,
                    });
                    original_value
                }
            }
            Expression::Var(name) => Value::Variable(name.clone()),
            _ => todo!("Unsupported expression type {:?}", expr),
        }
    }

    fn unary_op(&mut self, op: &UnaryOp) -> UnaryOperator {
        match op {
            UnaryOp::Negate => UnaryOperator::Negate,
            UnaryOp::Complement => UnaryOperator::Complement,
            UnaryOp::Not => UnaryOperator::Not,
        }
    }

    fn binary_op(&mut self, op: &BinaryOp) -> BinaryOperator {
        match op {
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
            BinaryOp::LogicalAnd => BinaryOperator::LogicalAnd,
            BinaryOp::LogicalOr => BinaryOperator::LogicalOr,
            BinaryOp::Equal => BinaryOperator::Equal,
            BinaryOp::NotEqual => BinaryOperator::NotEqual,
            BinaryOp::Greater => BinaryOperator::Greater,
            BinaryOp::Less => BinaryOperator::Less,
            BinaryOp::GreaterEqual => BinaryOperator::GreaterEqual,
            BinaryOp::LessEqual => BinaryOperator::LessEqual,
            _ => todo!("Unsupported binary operator {:?}", op),
        }
    }

    fn make_temp_var(&mut self) -> String {
        self.name_creator.borrow_mut().make_temp_var_name()
    }

    fn make_label(&mut self, prefix: &str) -> String {
        let counter = self.label_counters.entry(prefix.to_string()).or_insert(0);
        let label = format!("{}_{}", prefix, counter);
        *counter += 1;
        label
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic::NameCreator;
    use crate::tacky::UnaryOperator::{Complement, Negate};

    #[test]
    fn emit_return() {
        let mut emitter = TackyEmitter::new(NameCreator::new_ref());
        let stmt = Statement::Return(Expression::IntegerConstant(42));
        let expected = vec![Instruction::Return(IntegerConstant(42))];
        let actual = emitter.emit_statement(&stmt);

        assert_eq!(expected, actual);
    }

    #[test]
    fn emit_complex_return() {
        use Expression::*;
        use Instruction::*;
        use Value::*;

        let mut emitter = TackyEmitter::new(NameCreator::new_ref());
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
            Unary {
                op: Negate,
                src: Value::IntegerConstant(42),
                dst: Variable("tmp.0".to_string()),
            },
            Unary {
                op: Complement,
                src: Variable("tmp.0".to_string()),
                dst: Variable("tmp.1".to_string()),
            },
            Unary {
                op: Negate,
                src: Variable("tmp.1".to_string()),
                dst: Variable("tmp.2".to_string()),
            },
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

        let mut emitter = TackyEmitter::new(NameCreator::new_ref());
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

        let mut emitter = TackyEmitter::new(NameCreator::new_ref());
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

        let mut emitter = TackyEmitter::new(NameCreator::new_ref());
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

    #[test]
    fn emit_logical_and_expr() {
        use Expression::*;
        use Instruction::*;
        use Value::*;

        let mut emitter = TackyEmitter::new(NameCreator::new_ref());
        let stmt = Statement::Return(BinaryExpr(
            BinaryOp::LogicalAnd,
            Box::new(Expression::IntegerConstant(1)),
            Box::new(Expression::IntegerConstant(0)),
        ));

        let expected = vec![
            JumpIfZero {
                condition: Value::IntegerConstant(1),
                target: "and_false_0".to_string(),
            },
            JumpIfZero {
                condition: Value::IntegerConstant(0),
                target: "and_false_0".to_string(),
            },
            Copy {
                src: Value::IntegerConstant(1),
                dst: Variable("tmp.0".to_string()),
            },
            Jump {
                target: "and_end_0".to_string(),
            },
            Label("and_false_0".to_string()),
            Copy {
                src: Value::IntegerConstant(0),
                dst: Variable("tmp.0".to_string()),
            },
            Label("and_end_0".to_string()),
            Return(Variable("tmp.0".to_string())),
        ];

        let actual = emitter.emit_statement(&stmt);
        assert_eq!(expected, actual);
    }

    #[test]
    fn emit_logical_or_expr() {
        use Expression::*;
        use Instruction::*;
        use Value::*;

        let mut emitter = TackyEmitter::new(NameCreator::new_ref());
        let stmt = Statement::Return(BinaryExpr(
            BinaryOp::LogicalOr,
            Box::new(Expression::IntegerConstant(0)),
            Box::new(Expression::IntegerConstant(1)),
        ));

        let expected = vec![
            JumpIfNotZero {
                condition: Value::IntegerConstant(0),
                target: "or_true_0".to_string(),
            },
            JumpIfNotZero {
                condition: Value::IntegerConstant(1),
                target: "or_true_0".to_string(),
            },
            Copy {
                src: Value::IntegerConstant(0),
                dst: Variable("tmp.0".to_string()),
            },
            Jump {
                target: "or_end_0".to_string(),
            },
            Label("or_true_0".to_string()),
            Copy {
                src: Value::IntegerConstant(1),
                dst: Variable("tmp.0".to_string()),
            },
            Label("or_end_0".to_string()),
            Return(Variable("tmp.0".to_string())),
        ];

        let actual = emitter.emit_statement(&stmt);
        assert_eq!(expected, actual);
    }

    #[test]
    fn emit_prefix_increment_expr() {
        use BinaryOperator::*;
        use Expression::*;
        use Instruction::*;
        use Value::*;

        let mut emitter = TackyEmitter::new(NameCreator::new_ref());
        let stmt = Statement::Return(Assignment {
            left: Box::new(Var("a".to_string())),
            right: Box::new(BinaryExpr(
                BinaryOp::Add,
                Box::new(Var("a".to_string())),
                Box::new(Expression::IntegerConstant(1)),
            )),
            is_postfix: false,
        });

        let expected = vec![
            Binary {
                op: Add,
                src1: Variable("a".to_string()),
                src2: Value::IntegerConstant(1),
                dst: Variable("tmp.0".to_string()),
            },
            Copy {
                src: Variable("tmp.0".to_string()),
                dst: Variable("a".to_string()),
            },
            Return(Variable("a".to_string())),
        ];

        let actual = emitter.emit_statement(&stmt);
        assert_eq!(expected, actual);
    }

    #[test]
    fn emit_postfix_increment_expr() {
        use BinaryOperator::*;
        use Expression::*;
        use Instruction::*;
        use Value::*;

        let mut emitter = TackyEmitter::new(NameCreator::new_ref());
        let stmt = Statement::Return(Assignment {
            left: Box::new(Var("a".to_string())),
            right: Box::new(BinaryExpr(
                BinaryOp::Add,
                Box::new(Var("a".to_string())),
                Box::new(Expression::IntegerConstant(1)),
            )),
            is_postfix: true,
        });

        let expected = vec![
            Binary {
                op: Add,
                src1: Variable("a".to_string()),
                src2: Value::IntegerConstant(1),
                dst: Variable("tmp.0".to_string()),
            },
            Copy {
                src: Variable("a".to_string()),
                dst: Variable("tmp.1".to_string()),
            },
            Copy {
                src: Variable("tmp.0".to_string()),
                dst: Variable("a".to_string()),
            },
            Return(Variable("tmp.1".to_string())),
        ];

        let actual = emitter.emit_statement(&stmt);
        assert_eq!(expected, actual);
    }

    #[test]
    fn emit_prefix_decrement_expr() {
        use BinaryOperator::*;
        use Expression::*;
        use Instruction::*;
        use Value::*;

        let mut emitter = TackyEmitter::new(NameCreator::new_ref());
        let stmt = Statement::Return(Assignment {
            left: Box::new(Var("a".to_string())),
            right: Box::new(BinaryExpr(
                BinaryOp::Subtract,
                Box::new(Var("a".to_string())),
                Box::new(Expression::IntegerConstant(1)),
            )),
            is_postfix: false,
        });

        let expected = vec![
            Binary {
                op: Subtract,
                src1: Variable("a".to_string()),
                src2: Value::IntegerConstant(1),
                dst: Variable("tmp.0".to_string()),
            },
            Copy {
                src: Variable("tmp.0".to_string()),
                dst: Variable("a".to_string()),
            },
            Return(Variable("a".to_string())),
        ];

        let actual = emitter.emit_statement(&stmt);
        assert_eq!(expected, actual);
    }

    #[test]
    fn emit_postfix_decrement_expr() {
        use BinaryOperator::*;
        use Expression::*;
        use Instruction::*;
        use Value::*;

        let mut emitter = TackyEmitter::new(NameCreator::new_ref());
        let stmt = Statement::Return(Assignment {
            left: Box::new(Var("a".to_string())),
            right: Box::new(BinaryExpr(
                BinaryOp::Subtract,
                Box::new(Var("a".to_string())),
                Box::new(Expression::IntegerConstant(1)),
            )),
            is_postfix: true,
        });

        let expected = vec![
            Binary {
                op: Subtract,
                src1: Variable("a".to_string()),
                src2: Value::IntegerConstant(1),
                dst: Variable("tmp.0".to_string()),
            },
            Copy {
                src: Variable("a".to_string()),
                dst: Variable("tmp.1".to_string()),
            },
            Copy {
                src: Variable("tmp.0".to_string()),
                dst: Variable("a".to_string()),
            },
            Return(Variable("tmp.1".to_string())),
        ];

        let actual = emitter.emit_statement(&stmt);
        assert_eq!(expected, actual);
    }


}

use crate::ast::{
    BinaryOp, Block, BlockItem, Declaration, Expression, ForInit, FunctionDefinition, Label,
    Statement, UnaryOp,
};
use crate::semantic::NameGeneratorRef;
use anyhow::Result;

use super::ast::Instruction::Unary;
use super::ast::Value::IntegerConstant;
use super::ast::{BinaryOperator, FunctionDef, Instruction, Program, UnaryOperator, Value};

#[derive(Clone)]
pub struct TackyEmitter {
    label_name_generator: NameGeneratorRef,
    tmp_var_name_generator: NameGeneratorRef,
}

impl TackyEmitter {
    pub fn new(
        label_name_generator: NameGeneratorRef,
        tmp_var_name_generator: NameGeneratorRef,
    ) -> TackyEmitter {
        TackyEmitter {
            label_name_generator,
            tmp_var_name_generator,
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

    fn emit_block(&mut self, block: &Block) -> Vec<Instruction> {
        let mut instructions = vec![];
        for item in &block.items {
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
            Statement::Return(expr) => self.emit_return_statement(expr),
            Statement::Expression(expr) => self.emit_expression_statement(expr),
            Statement::Null => self.emit_null_statement(),
            Statement::CompoundStatement(block) => self.emit_block(block),
            Statement::IfStatement {
                condition,
                then_branch,
                else_branch,
            } => self.emit_if_statement(condition, then_branch, else_branch),
            Statement::GotoStatement(label) => self.emit_goto_statement(label),
            Statement::LabeledStatement { label, statement } => {
                self.emit_labeled_statement(label, statement)
            }
            Statement::Break { loop_id } => self.emit_break_statement(loop_id),
            Statement::Continue { loop_id } => self.emit_continue_statement(loop_id),
            Statement::DoWhile {
                loop_id,
                body,
                condition,
            } => self.emit_do_while_statement(loop_id, body, condition),
            Statement::While {
                loop_id,
                condition,
                body,
            } => self.emit_while_statement(loop_id, condition, body),
            Statement::For {
                loop_id,
                init,
                condition,
                post,
                body,
            } => self.emit_for_statement(loop_id, init, condition, post, body),
            _ => todo!("Unsupported statement type: {:?}", stmt),
        }
    }

    fn emit_for_statement(
        &mut self,
        loop_id: &str,
        init: &ForInit,
        condition: &Option<Expression>,
        post: &Option<Expression>,
        body: &Box<Statement>,
    ) -> Vec<Instruction> {
        let mut instructions = vec![];

        let start_label = self.make_start_label(loop_id);
        let break_label = self.make_break_label(loop_id);
        let continue_label = self.make_continue_label(loop_id);

        instructions.extend(self.emit_for_init(init));

        instructions.push(Instruction::Label(start_label.clone()));

        if let Some(condition) = condition {
            let condition_value = self.emit_expression(condition, &mut instructions);
            instructions.push(Instruction::JumpIfZero {
                condition: condition_value,
                target: break_label.clone(),
            });
        }

        instructions.extend(self.emit_statement(body));

        instructions.push(Instruction::Label(continue_label));
        if let Some(post) = post {
            self.emit_expression(post, &mut instructions);
        }
        instructions.push(Instruction::Jump {
            target: start_label,
        });
        instructions.push(Instruction::Label(break_label));

        instructions
    }

    fn emit_for_init(&mut self, init: &ForInit) -> Vec<Instruction> {
        let mut instructions = vec![];

        match init {
            ForInit::InitDeclaration(decl) => {
                instructions.extend(self.emit_declaration(decl));
            }
            ForInit::InitExpression(expr) => {
                if let Some(expr) = expr {
                    self.emit_expression(expr, &mut instructions);
                }
            }
        }

        instructions
    }

    fn emit_while_statement(
        &mut self,
        loop_id: &str,
        condition: &Expression,
        body: &Statement,
    ) -> Vec<Instruction> {
        let mut instructions = vec![];

        let break_label = self.make_break_label(loop_id);
        let continue_label = self.make_continue_label(loop_id);

        instructions.push(Instruction::Label(continue_label.clone()));
        let condition_value = self.emit_expression(condition, &mut instructions);
        instructions.push(Instruction::JumpIfZero {
            condition: condition_value,
            target: break_label.clone(),
        });
        instructions.extend(self.emit_statement(body));
        instructions.push(Instruction::Jump {
            target: continue_label.clone(),
        });
        instructions.push(Instruction::Label(break_label));

        instructions
    }

    fn emit_do_while_statement(
        &mut self,
        loop_id: &str,
        body: &Statement,
        condition: &Expression,
    ) -> Vec<Instruction> {
        let mut instructions = vec![];

        let start_label = self.make_start_label(loop_id);
        instructions.push(Instruction::Label(start_label.clone()));
        instructions.extend(self.emit_statement(body));
        instructions.push(Instruction::Label(self.make_continue_label(loop_id)));
        let condition_value = self.emit_expression(condition, &mut instructions);
        instructions.push(Instruction::JumpIfNotZero {
            condition: condition_value,
            target: start_label,
        });
        instructions.push(Instruction::Label(self.make_break_label(loop_id)));

        instructions
    }

    fn emit_return_statement(&mut self, expr: &Expression) -> Vec<Instruction> {
        let mut instructions = vec![];
        let value = self.emit_expression(expr, &mut instructions);
        instructions.push(Instruction::Return(value));
        instructions
    }

    fn emit_expression_statement(&mut self, expr: &Expression) -> Vec<Instruction> {
        let mut instructions = vec![];
        self.emit_expression(expr, &mut instructions);
        instructions
    }

    fn emit_null_statement(&mut self) -> Vec<Instruction> {
        vec![]
    }

    fn emit_goto_statement(&mut self, label: &str) -> Vec<Instruction> {
        vec![Instruction::Jump {
            target: label.to_string(),
        }]
    }

    fn emit_labeled_statement(
        &mut self,
        label: &Label,
        statement: &Box<Statement>,
    ) -> Vec<Instruction> {
        let name = label.get_name();
        let mut instructions = vec![Instruction::Label(name)];
        instructions.extend(self.emit_statement(statement));
        instructions
    }

    fn emit_break_statement(&mut self, loop_id: &str) -> Vec<Instruction> {
        vec![Instruction::Jump {
            target: self.make_break_label(loop_id),
        }]
    }

    fn emit_continue_statement(&mut self, loop_id: &str) -> Vec<Instruction> {
        vec![Instruction::Jump {
            target: self.make_continue_label(loop_id),
        }]
    }

    fn emit_if_statement(
        &mut self,
        condition: &Expression,
        then_branch: &Box<Statement>,
        else_branch: &Option<Box<Statement>>,
    ) -> Vec<Instruction> {
        let mut instructions = vec![];
        let end_label = self.make_label("if_end");
        let condition_value = self.emit_expression(condition, &mut instructions);

        if let Some(else_branch) = else_branch {
            let else_label = self.make_label("if_else");
            instructions.push(Instruction::JumpIfZero {
                condition: condition_value,
                target: else_label.clone(),
            });

            instructions.extend(self.emit_statement(then_branch));
            instructions.push(Instruction::Jump {
                target: end_label.clone(),
            });
            instructions.push(Instruction::Label(else_label));
            instructions.extend(self.emit_statement(else_branch));
        } else {
            instructions.push(Instruction::JumpIfZero {
                condition: condition_value,
                target: end_label.clone(),
            });

            instructions.extend(self.emit_statement(then_branch));
        }

        instructions.push(Instruction::Label(end_label));

        instructions
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
                    instructions.push(Instruction::Copy { src, dst });
                    original_value
                }
            }
            Expression::Var(name) => Value::Variable(name.clone()),
            Expression::ConditionalExpr {
                condition,
                then_expr,
                else_expr,
            } => {
                let end_label = self.make_label("cond_end");
                let else_label = self.make_label("cond_else");
                let result = Value::Variable(self.make_temp_var());

                let condition_value = self.emit_expression(condition, instructions);
                instructions.push(Instruction::JumpIfZero {
                    condition: condition_value,
                    target: else_label.clone(),
                });

                let then_value = self.emit_expression(then_expr, instructions);
                instructions.push(Instruction::Copy {
                    src: then_value,
                    dst: result.clone(),
                });
                instructions.push(Instruction::Jump {
                    target: end_label.clone(),
                });

                instructions.push(Instruction::Label(else_label));
                let else_value = self.emit_expression(else_expr, instructions);
                instructions.push(Instruction::Copy {
                    src: else_value,
                    dst: result.clone(),
                });

                instructions.push(Instruction::Label(end_label));

                result
            }
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
        self.tmp_var_name_generator
            .borrow_mut()
            .make_unique_name("")
    }

    fn make_label(&mut self, prefix: &str) -> String {
        self.label_name_generator
            .borrow_mut()
            .make_unique_name(prefix)
    }

    fn make_start_label(&mut self, loop_id: &str) -> String {
        format!("start.{loop_id}")
    }

    fn make_break_label(&mut self, loop_id: &str) -> String {
        format!("break.{loop_id}")
    }

    fn make_continue_label(&mut self, loop_id: &str) -> String {
        format!("continue.{loop_id}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic;
    use crate::tacky::ast::UnaryOperator::{Complement, Negate};

    fn make_emitter() -> TackyEmitter {
        let label_name_generator = semantic::make_label_name_generator();
        let tmp_var_name_generator = semantic::make_temp_var_name_generator();
        TackyEmitter::new(label_name_generator, tmp_var_name_generator)
    }

    #[test]
    fn emit_return() {
        let mut emitter = make_emitter();
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

        let mut emitter = make_emitter();
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

        let mut emitter = make_emitter();
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

        let mut emitter = make_emitter();
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

        let mut emitter = make_emitter();
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

        let mut emitter = make_emitter();
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

        let mut emitter = make_emitter();
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

        let mut emitter = make_emitter();
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

        let mut emitter = make_emitter();
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

        let mut emitter = make_emitter();
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

        let mut emitter = make_emitter();
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

    #[test]
    fn emit_if_statement_without_else() {
        use Instruction::*;

        let mut emitter = make_emitter();
        let stmt = Statement::IfStatement {
            condition: Expression::IntegerConstant(1),
            then_branch: Box::new(Statement::Return(Expression::IntegerConstant(42))),
            else_branch: None,
        };

        let expected = vec![
            JumpIfZero {
                condition: IntegerConstant(1),
                target: "if_end_0".to_string(),
            },
            Return(IntegerConstant(42)),
            Label("if_end_0".to_string()),
        ];

        let actual = emitter.emit_statement(&stmt);
        assert_eq!(expected, actual);
    }

    #[test]
    fn emit_if_statement_with_else() {
        use Expression::*;
        use Instruction::*;

        let mut emitter = make_emitter();
        let stmt = Statement::IfStatement {
            condition: IntegerConstant(1),
            then_branch: Box::new(Statement::Return(IntegerConstant(42))),
            else_branch: Some(Box::new(Statement::Return(IntegerConstant(0)))),
        };

        let expected = vec![
            JumpIfZero {
                condition: Value::IntegerConstant(1),
                target: "if_else_0".to_string(),
            },
            Return(Value::IntegerConstant(42)),
            Jump {
                target: "if_end_0".to_string(),
            },
            Label("if_else_0".to_string()),
            Return(Value::IntegerConstant(0)),
            Label("if_end_0".to_string()),
        ];

        let actual = emitter.emit_statement(&stmt);
        assert_eq!(expected, actual);
    }

    #[test]
    fn emit_conditional_expression_simple() {
        use Instruction::*;
        use Value::*;

        let mut emitter = make_emitter();
        let stmt = Statement::Return(Expression::ConditionalExpr {
            condition: Box::new(Expression::IntegerConstant(1)),
            then_expr: Box::new(Expression::IntegerConstant(42)),
            else_expr: Box::new(Expression::IntegerConstant(0)),
        });

        let expected = vec![
            JumpIfZero {
                condition: IntegerConstant(1),
                target: "cond_else_0".to_string(),
            },
            Copy {
                src: IntegerConstant(42),
                dst: Variable("tmp.0".to_string()),
            },
            Jump {
                target: "cond_end_0".to_string(),
            },
            Label("cond_else_0".to_string()),
            Copy {
                src: IntegerConstant(0),
                dst: Variable("tmp.0".to_string()),
            },
            Label("cond_end_0".to_string()),
            Return(Variable("tmp.0".to_string())),
        ];

        let actual = emitter.emit_statement(&stmt);
        assert_eq!(expected, actual);
    }

    #[test]
    fn emit_nested_if_dangling_else_binds_to_inner_if() {
        use Instruction::*;
        use Value::*;

        let mut emitter = make_emitter();
        let stmt = Statement::IfStatement {
            condition: Expression::Var("a".to_string()),
            then_branch: Box::new(Statement::IfStatement {
                condition: Expression::Var("b".to_string()),
                then_branch: Box::new(Statement::Return(Expression::IntegerConstant(1))),
                else_branch: Some(Box::new(Statement::Return(Expression::IntegerConstant(2)))),
            }),
            else_branch: None,
        };

        let expected = vec![
            JumpIfZero {
                condition: Variable("a".to_string()),
                target: "if_end_0".to_string(),
            },
            JumpIfZero {
                condition: Variable("b".to_string()),
                target: "if_else_0".to_string(),
            },
            Return(IntegerConstant(1)),
            Jump {
                target: "if_end_1".to_string(),
            },
            Label("if_else_0".to_string()),
            Return(IntegerConstant(2)),
            Label("if_end_1".to_string()),
            Label("if_end_0".to_string()),
        ];

        let actual = emitter.emit_statement(&stmt);
        assert_eq!(expected, actual);
    }

    #[test]
    fn emit_while_with_break_and_continue() {
        use Instruction::*;
        use Value::*;

        let mut emitter = make_emitter();
        let stmt = Statement::While {
            loop_id: "loop.0".to_string(),
            condition: Expression::IntegerConstant(1),
            body: Box::new(Statement::CompoundStatement(Block::new(vec![
                BlockItem::Statement(Statement::Break {
                    loop_id: "loop.0".to_string(),
                }),
                BlockItem::Statement(Statement::Continue {
                    loop_id: "loop.0".to_string(),
                }),
            ]))),
        };

        let expected = vec![
            Label("continue.loop.0".to_string()),
            JumpIfZero {
                condition: IntegerConstant(1),
                target: "break.loop.0".to_string(),
            },
            Jump {
                target: "break.loop.0".to_string(),
            },
            Jump {
                target: "continue.loop.0".to_string(),
            },
            Jump {
                target: "continue.loop.0".to_string(),
            },
            Label("break.loop.0".to_string()),
        ];

        let actual = emitter.emit_statement(&stmt);
        assert_eq!(expected, actual);
    }

    #[test]
    fn emit_do_while_with_break_and_continue() {
        use Instruction::*;
        use Value::*;

        let mut emitter = make_emitter();
        let stmt = Statement::DoWhile {
            loop_id: "loop.1".to_string(),
            condition: Expression::IntegerConstant(1),
            body: Box::new(Statement::CompoundStatement(Block::new(vec![
                BlockItem::Statement(Statement::Break {
                    loop_id: "loop.1".to_string(),
                }),
                BlockItem::Statement(Statement::Continue {
                    loop_id: "loop.1".to_string(),
                }),
            ]))),
        };

        let expected = vec![
            Label("start.loop.1".to_string()),
            Jump {
                target: "break.loop.1".to_string(),
            },
            Jump {
                target: "continue.loop.1".to_string(),
            },
            Label("continue.loop.1".to_string()),
            JumpIfNotZero {
                condition: IntegerConstant(1),
                target: "start.loop.1".to_string(),
            },
            Label("break.loop.1".to_string()),
        ];

        let actual = emitter.emit_statement(&stmt);
        assert_eq!(expected, actual);
    }

    #[test]
    fn emit_for_with_break_and_continue() {
        use Instruction::*;
        use Value::*;

        let mut emitter = make_emitter();
        let stmt = Statement::For {
            loop_id: "loop.2".to_string(),
            init: ForInit::InitExpression(None),
            condition: Some(Expression::IntegerConstant(1)),
            post: None,
            body: Box::new(Statement::CompoundStatement(Block::new(vec![
                BlockItem::Statement(Statement::Break {
                    loop_id: "loop.2".to_string(),
                }),
                BlockItem::Statement(Statement::Continue {
                    loop_id: "loop.2".to_string(),
                }),
            ]))),
        };

        let expected = vec![
            Label("start.loop.2".to_string()),
            JumpIfZero {
                condition: IntegerConstant(1),
                target: "break.loop.2".to_string(),
            },
            Jump {
                target: "break.loop.2".to_string(),
            },
            Jump {
                target: "continue.loop.2".to_string(),
            },
            Label("continue.loop.2".to_string()),
            Jump {
                target: "start.loop.2".to_string(),
            },
            Label("break.loop.2".to_string()),
        ];

        let actual = emitter.emit_statement(&stmt);
        assert_eq!(expected, actual);
    }
}


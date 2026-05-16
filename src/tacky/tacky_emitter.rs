use super::ast::Instruction::Unary;
use super::ast::Value::{IntegerConstant, LongConstant};
use super::ast::{BinaryOperator, Function, Instruction, Program, TopLevel, UnaryOperator, Value};
use crate::ast::{
    BinaryOp, Block, BlockItem, Expression, ForInit, FunctionDeclaration, Label, Statement,
    StorageClass, TypedExpression, UnaryOp, VarDeclaration,
};
use crate::common::Type;
use crate::semantic::symbol_table::{IdentAttrs, InitValue, InitialValue, SymbolTableEntry};
use crate::semantic::{NameGeneratorRef, symbol_table};
use anyhow::{Result, anyhow};

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
        let mut top_levels = program
            .decls
            .iter()
            .filter_map(|decl| match decl {
                crate::ast::Declaration::FunctionDecl(func_decl) => Some(func_decl),
                _ => None,
            })
            .filter(|func_decl| func_decl.body.is_some())
            .map(|func_decl| self.emit_function_decl(func_decl))
            .filter_map(Result::ok)
            .map(TopLevel::Function)
            .collect::<Vec<TopLevel>>();

        top_levels.extend(Self::read_var_decls_from_symbol_table());

        Ok(Program(top_levels))
    }

    fn read_var_decls_from_symbol_table() -> Vec<TopLevel> {
        symbol_table::with_global_symbol_table(|table| {
            table
                .get_all_entries()
                .filter_map(|(name, entry)| match &entry.attrs {
                    IdentAttrs::Static {
                        is_global,
                        init_value,
                    } => {
                        let initial_value = match init_value {
                            Some(InitialValue::Initialized(InitValue::Int(ival))) => {
                                IntegerConstant(*ival)
                            }
                            Some(InitialValue::Initialized(InitValue::Long(lval))) => {
                                LongConstant(*lval)
                            }
                            Some(InitialValue::Tentative) => IntegerConstant(0),
                            None => return None,
                        };
                        Some(TopLevel::StaticVariable(
                            crate::tacky::ast::StaticVariable {
                                name: name.clone(),
                                is_global: *is_global,
                                initial_value,
                                c_type: entry.c_type.clone(),
                            },
                        ))
                    }
                    _ => None,
                })
                .collect()
        })
    }

    fn emit_function_decl(
        &mut self,
        function_declaration: &FunctionDeclaration,
    ) -> Result<Function> {
        let name = function_declaration.name.clone();
        let instructions = if let Some(body) = &function_declaration.body {
            let mut instructions = self.emit_block(body)?;
            instructions.push(Instruction::Return(IntegerConstant(0))); // Ensure function ends with a return
            instructions
        } else {
            vec![]
        };

        let entry = symbol_table::get(&name)
            .ok_or_else(|| anyhow!("Function {} not found in symbol table", name))?;
        let is_global = match entry.attrs {
            IdentAttrs::Function { is_global, .. } => is_global,
            _ => return Err(anyhow!("Function {} not found in symbol table", name)),
        };

        Ok(Function {
            name,
            parameters: function_declaration.parameters.clone(),
            body: instructions,
            is_global,
        })
    }

    fn emit_block(&mut self, block: &Block) -> Result<Vec<Instruction>> {
        let mut instructions = vec![];
        for item in &block.items {
            match item {
                BlockItem::FunctionDeclaration(func_decl) => {
                    let func = self.emit_function_decl(func_decl)?;
                    instructions.extend(func.body.clone());
                }
                BlockItem::VarDeclaration(decl) => {
                    instructions.extend(self.emit_declaration(decl));
                }
                BlockItem::Statement(stmt) => {
                    instructions.extend(self.emit_statement(stmt)?);
                }
            }
        }
        Ok(instructions)
    }

    fn emit_declaration(&mut self, declaration: &VarDeclaration) -> Vec<Instruction> {
        let mut instructions = vec![];

        if let Some(StorageClass::Static) = declaration.storage_class {
            return instructions;
        }

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

    fn emit_statement(&mut self, stmt: &Statement) -> Result<Vec<Instruction>> {
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
            Statement::SwitchStatement {
                switch_id,
                condition,
                body,
                arms,
            } => self.emit_switch_statement(switch_id, condition, body, arms),
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
        }
    }

    fn emit_for_statement(
        &mut self,
        loop_id: &str,
        init: &ForInit,
        condition: &Option<TypedExpression>,
        post: &Option<TypedExpression>,
        body: &Box<Statement>,
    ) -> Result<Vec<Instruction>> {
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

        instructions.extend(self.emit_statement(body)?);

        instructions.push(Instruction::Label(continue_label));
        if let Some(post) = post {
            self.emit_expression(post, &mut instructions);
        }
        instructions.push(Instruction::Jump {
            target: start_label,
        });
        instructions.push(Instruction::Label(break_label));

        Ok(instructions)
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
        condition: &TypedExpression,
        body: &Statement,
    ) -> Result<Vec<Instruction>> {
        let mut instructions = vec![];

        let break_label = self.make_break_label(loop_id);
        let continue_label = self.make_continue_label(loop_id);

        instructions.push(Instruction::Label(continue_label.clone()));
        let condition_value = self.emit_expression(condition, &mut instructions);
        instructions.push(Instruction::JumpIfZero {
            condition: condition_value,
            target: break_label.clone(),
        });
        instructions.extend(self.emit_statement(body)?);
        instructions.push(Instruction::Jump {
            target: continue_label.clone(),
        });
        instructions.push(Instruction::Label(break_label));

        Ok(instructions)
    }

    fn emit_do_while_statement(
        &mut self,
        loop_id: &str,
        body: &Statement,
        condition: &TypedExpression,
    ) -> Result<Vec<Instruction>> {
        let mut instructions = vec![];

        let start_label = self.make_start_label(loop_id);
        instructions.push(Instruction::Label(start_label.clone()));
        instructions.extend(self.emit_statement(body)?);
        instructions.push(Instruction::Label(self.make_continue_label(loop_id)));
        let condition_value = self.emit_expression(condition, &mut instructions);
        instructions.push(Instruction::JumpIfNotZero {
            condition: condition_value,
            target: start_label,
        });
        instructions.push(Instruction::Label(self.make_break_label(loop_id)));

        Ok(instructions)
    }

    fn emit_return_statement(&mut self, expr: &TypedExpression) -> Result<Vec<Instruction>> {
        let mut instructions = vec![];
        let value = self.emit_expression(expr, &mut instructions);
        instructions.push(Instruction::Return(value));
        Ok(instructions)
    }

    fn emit_expression_statement(&mut self, expr: &TypedExpression) -> Result<Vec<Instruction>> {
        let mut instructions = vec![];
        self.emit_expression(expr, &mut instructions);
        Ok(instructions)
    }

    fn emit_null_statement(&mut self) -> Result<Vec<Instruction>> {
        Ok(vec![])
    }

    fn emit_goto_statement(&mut self, label: &str) -> Result<Vec<Instruction>> {
        Ok(vec![Instruction::Jump {
            target: label.to_string(),
        }])
    }

    fn emit_labeled_statement(
        &mut self,
        label: &Label,
        statement: &Box<Statement>,
    ) -> Result<Vec<Instruction>> {
        let name = label.get_name();
        let mut instructions = vec![Instruction::Label(name)];
        instructions.extend(self.emit_statement(statement)?);
        Ok(instructions)
    }

    fn emit_break_statement(&mut self, loop_id: &str) -> Result<Vec<Instruction>> {
        Ok(vec![Instruction::Jump {
            target: self.make_break_label(loop_id),
        }])
    }

    fn emit_continue_statement(&mut self, loop_id: &str) -> Result<Vec<Instruction>> {
        Ok(vec![Instruction::Jump {
            target: self.make_continue_label(loop_id),
        }])
    }

    fn emit_if_statement(
        &mut self,
        condition: &TypedExpression,
        then_branch: &Box<Statement>,
        else_branch: &Option<Box<Statement>>,
    ) -> Result<Vec<Instruction>> {
        let mut instructions = vec![];
        let end_label = self.make_label("if_end");
        let condition_value = self.emit_expression(condition, &mut instructions);

        if let Some(else_branch) = else_branch {
            let else_label = self.make_label("if_else");
            instructions.push(Instruction::JumpIfZero {
                condition: condition_value,
                target: else_label.clone(),
            });

            instructions.extend(self.emit_statement(then_branch)?);
            instructions.push(Instruction::Jump {
                target: end_label.clone(),
            });
            instructions.push(Instruction::Label(else_label));
            instructions.extend(self.emit_statement(else_branch)?);
        } else {
            instructions.push(Instruction::JumpIfZero {
                condition: condition_value,
                target: end_label.clone(),
            });

            instructions.extend(self.emit_statement(then_branch)?);
        }

        instructions.push(Instruction::Label(end_label));

        Ok(instructions)
    }

    fn emit_switch_statement(
        &mut self,
        switch_id: &str,
        condition: &TypedExpression,
        body: &Statement,
        arms: &Vec<(String, Option<TypedExpression>)>,
    ) -> Result<Vec<Instruction>> {
        let mut instructions = vec![];

        let condition_value = self.emit_expression(condition, &mut instructions);
        let break_label = self.make_break_label(switch_id);

        for (label, expression) in arms {
            match expression {
                Some(expression) => {
                    let case_value = self.emit_expression(expression, &mut instructions);
                    let dst = self.make_temp_var(&Type::Int);
                    instructions.push(Instruction::Binary {
                        op: BinaryOperator::Equal,
                        src1: case_value,
                        src2: condition_value.clone(),
                        dst: dst.clone(),
                    });
                    instructions.push(Instruction::JumpIfNotZero {
                        condition: dst,
                        target: label.clone(),
                    })
                }
                None => instructions.push(Instruction::Jump {
                    target: label.clone(),
                }),
            }
        }

        instructions.push(Instruction::Jump {
            target: break_label.clone(),
        });

        instructions.extend(self.emit_statement(body)?);

        instructions.push(Instruction::Label(break_label));

        Ok(instructions)
    }

    fn emit_expression(
        &mut self,
        expr: &TypedExpression,
        instructions: &mut Vec<Instruction>,
    ) -> Value {
        let expr_type = expr.get_type();
        match &expr.0 {
            Expression::IntegerConstant(value) => self.emit_integer_constant(*value),
            Expression::LongConstant(value) => self.emit_long_constant(*value),
            Expression::UnaryExpr(op, expr) => {
                self.emit_unary_expr(op, expr, &expr_type, instructions)
            }
            Expression::BinaryExpr(BinaryOp::LogicalAnd, left, right) => {
                self.emit_logical_and_expr(left, right, &expr_type, instructions)
            }
            Expression::BinaryExpr(BinaryOp::LogicalOr, left, right) => {
                self.emit_logical_or_expr(left, right, &expr_type, instructions)
            }
            Expression::BinaryExpr(op, left, right) => {
                self.emit_binary_expr(op, left, right, &expr_type, instructions)
            }
            Expression::Assignment {
                left,
                right,
                is_postfix,
            } => self.emit_assignment_expr(left, right, *is_postfix, &expr_type, instructions),
            Expression::Var(name) => self.emit_var_expr(name),
            Expression::ConditionalExpr {
                condition,
                then_expr,
                else_expr,
            } => self.emit_conditional_expr(condition, then_expr, else_expr, &expr_type, instructions),
            Expression::FuncCall { name, args } => {
                self.emit_func_call_expr(name, args, &expr_type, instructions)
            }
            Expression::Cast { expr, target_type } => {
                self.emit_cast_expr(expr, target_type, instructions)
            }
        }
    }

    fn emit_cast_expr(
        &mut self,
        expr: &TypedExpression,
        target_type: &Type,
        instructions: &mut Vec<Instruction>,
    ) -> Value {
        let expr_value = self.emit_expression(expr, instructions);
        if expr.get_type() == *target_type {
            return expr_value;
        }

        let result = self.make_temp_var(target_type);

        if *target_type == Type::Long {
            instructions.push(Instruction::SignExtend {
                src: expr_value,
                dst: result.clone(),
            });
        } else {
            instructions.push(Instruction::Truncate {
                src: expr_value,
                dst: result.clone(),
            });
        }

        result
    }

    fn emit_integer_constant(&self, value: i32) -> Value {
        IntegerConstant(value)
    }

    fn emit_long_constant(&self, value: i64) -> Value {
        LongConstant(value)
    }

    fn emit_var_expr(&self, name: &str) -> Value {
        Value::Variable(name.to_string())
    }

    fn emit_unary_expr(
        &mut self,
        op: &UnaryOp,
        expr: &TypedExpression,
        c_type: &Type,
        instructions: &mut Vec<Instruction>,
    ) -> Value {
        let op = self.unary_op(op);
        let src = self.emit_expression(expr, instructions);
        let dst = self.make_temp_var(c_type);
        instructions.push(Unary {
            op,
            src,
            dst: dst.clone(),
        });
        dst
    }

    fn emit_logical_and_expr(
        &mut self,
        left: &TypedExpression,
        right: &TypedExpression,
        c_type: &Type,
        instructions: &mut Vec<Instruction>,
    ) -> Value {
        let end_label = self.make_label("and_end");
        let false_label = self.make_label("and_false");
        let result = self.make_temp_var(c_type);

        let val1 = self.emit_expression(left, instructions);
        instructions.push(Instruction::JumpIfZero {
            condition: val1,
            target: false_label.clone(),
        });

        let val2 = self.emit_expression(right, instructions);
        instructions.push(Instruction::JumpIfZero {
            condition: val2,
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

    fn emit_logical_or_expr(
        &mut self,
        left: &TypedExpression,
        right: &TypedExpression,
        c_type: &Type,
        instructions: &mut Vec<Instruction>,
    ) -> Value {
        let end_label = self.make_label("or_end");
        let true_label = self.make_label("or_true");
        let result = self.make_temp_var(c_type);

        let val1 = self.emit_expression(left, instructions);
        instructions.push(Instruction::JumpIfNotZero {
            condition: val1,
            target: true_label.clone(),
        });

        let val2 = self.emit_expression(right, instructions);
        instructions.push(Instruction::JumpIfNotZero {
            condition: val2,
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

    fn emit_binary_expr(
        &mut self,
        op: &BinaryOp,
        left: &TypedExpression,
        right: &TypedExpression,
        c_type: &Type,
        instructions: &mut Vec<Instruction>,
    ) -> Value {
        let src1 = self.emit_expression(left, instructions);
        let src2 = self.emit_expression(right, instructions);
        let dst = self.make_temp_var(c_type);
        let op = self.binary_op(op);
        instructions.push(Instruction::Binary {
            op,
            src1,
            src2,
            dst: dst.clone(),
        });
        dst
    }

    fn emit_assignment_expr(
        &mut self,
        left: &TypedExpression,
        right: &TypedExpression,
        is_postfix: bool,
        c_type: &Type,
        instructions: &mut Vec<Instruction>,
    ) -> Value {
        let src = self.emit_expression(right, instructions);
        let dst = self.emit_expression(left, instructions);
        if !is_postfix {
            instructions.push(Instruction::Copy {
                src,
                dst: dst.clone(),
            });
            dst
        } else {
            let original_value = self.make_temp_var(c_type);
            instructions.push(Instruction::Copy {
                src: dst.clone(),
                dst: original_value.clone(),
            });
            instructions.push(Instruction::Copy { src, dst });
            original_value
        }
    }

    fn emit_conditional_expr(
        &mut self,
        condition: &TypedExpression,
        then_expr: &TypedExpression,
        else_expr: &TypedExpression,
        c_type: &Type,
        instructions: &mut Vec<Instruction>,
    ) -> Value {
        let end_label = self.make_label("cond_end");
        let else_label = self.make_label("cond_else");
        let result = self.make_temp_var(c_type);

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

    fn emit_func_call_expr(
        &mut self,
        name: &str,
        args: &Vec<TypedExpression>,
        return_type: &Type,
        instructions: &mut Vec<Instruction>,
    ) -> Value {
        let arguments: Vec<Value> = args
            .iter()
            .map(|arg| self.emit_expression(arg, instructions))
            .collect();

        let dst = self.make_temp_var(return_type);

        instructions.push(Instruction::FunctionCall {
            name: name.to_string(),
            arguments,
            dst: dst.clone(),
        });

        dst
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

    fn make_temp_var_name(&mut self) -> String {
        self.tmp_var_name_generator
            .borrow_mut()
            .make_unique_name("")
    }

    fn make_temp_var(&mut self, c_type: &Type) -> Value {
        let name = self.make_temp_var_name();

        symbol_table::insert(
            name.clone(),
            SymbolTableEntry {
                c_type: c_type.clone(),
                attrs: IdentAttrs::Local,
            },
        );

        Value::Variable(name)
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
    use crate::ast::typed;
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
        let stmt = Statement::Return(typed(Expression::IntegerConstant(42)));
        let expected = vec![Instruction::Return(IntegerConstant(42))];
        let actual = emitter.emit_statement(&stmt).unwrap();

        assert_eq!(expected, actual);
    }

    #[test]
    fn emit_complex_return() {
        use Expression::*;
        use Instruction::*;
        use Value::*;

        let mut emitter = make_emitter();
        let stmt = Statement::Return(typed(UnaryExpr(
            UnaryOp::Negate,
            Box::new(typed(UnaryExpr(
                UnaryOp::Complement,
                Box::new(typed(UnaryExpr(
                    UnaryOp::Negate,
                    Box::new(typed(Expression::IntegerConstant(42))),
                ))),
            ))),
        )));
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
        let actual = emitter.emit_statement(&stmt).unwrap();

        assert_eq!(expected, actual);
    }

    #[test]
    fn emit_binary_expr() {
        use BinaryOperator::*;
        use Expression::*;
        use Instruction::*;
        use Value::*;

        let mut emitter = make_emitter();
        let stmt = Statement::Return(typed(BinaryExpr(
            BinaryOp::Add,
            Box::new(typed(Expression::IntegerConstant(1))),
            Box::new(typed(BinaryExpr(
                BinaryOp::Multiply,
                Box::new(typed(Expression::IntegerConstant(2))),
                Box::new(typed(Expression::IntegerConstant(3))),
            ))),
        )));

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

        let actual = emitter.emit_statement(&stmt).unwrap();
        assert_eq!(expected, actual);
    }

    #[test]
    fn emit_shift_left_expr() {
        use BinaryOperator::*;
        use Expression::*;
        use Instruction::*;
        use Value::*;

        let mut emitter = make_emitter();
        let stmt = Statement::Return(typed(BinaryExpr(
            BinaryOp::ShiftLeft,
            Box::new(typed(Expression::IntegerConstant(8))),
            Box::new(typed(Expression::IntegerConstant(2))),
        )));

        let expected = vec![
            Binary {
                op: ShiftLeft,
                src1: Value::IntegerConstant(8),
                src2: Value::IntegerConstant(2),
                dst: Variable("tmp.0".to_string()),
            },
            Return(Variable("tmp.0".to_string())),
        ];

        let actual = emitter.emit_statement(&stmt).unwrap();
        assert_eq!(expected, actual);
    }

    #[test]
    fn emit_shift_right_expr() {
        use BinaryOperator::*;
        use Expression::*;
        use Instruction::*;
        use Value::*;

        let mut emitter = make_emitter();
        let stmt = Statement::Return(typed(BinaryExpr(
            BinaryOp::ShiftRight,
            Box::new(typed(Expression::IntegerConstant(16))),
            Box::new(typed(Expression::IntegerConstant(1))),
        )));

        let expected = vec![
            Binary {
                op: ShiftRight,
                src1: Value::IntegerConstant(16),
                src2: Value::IntegerConstant(1),
                dst: Variable("tmp.0".to_string()),
            },
            Return(Variable("tmp.0".to_string())),
        ];

        let actual = emitter.emit_statement(&stmt).unwrap();
        assert_eq!(expected, actual);
    }

    #[test]
    fn emit_logical_and_expr() {
        use Expression::*;
        use Instruction::*;
        use Value::*;

        let mut emitter = make_emitter();
        let stmt = Statement::Return(typed(BinaryExpr(
            BinaryOp::LogicalAnd,
            Box::new(typed(Expression::IntegerConstant(1))),
            Box::new(typed(Expression::IntegerConstant(0))),
        )));

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

        let actual = emitter.emit_statement(&stmt).unwrap();
        assert_eq!(expected, actual);
    }

    #[test]
    fn emit_logical_or_expr() {
        use Expression::*;
        use Instruction::*;
        use Value::*;

        let mut emitter = make_emitter();
        let stmt = Statement::Return(typed(BinaryExpr(
            BinaryOp::LogicalOr,
            Box::new(typed(Expression::IntegerConstant(0))),
            Box::new(typed(Expression::IntegerConstant(1))),
        )));

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

        let actual = emitter.emit_statement(&stmt).unwrap();
        assert_eq!(expected, actual);
    }

    #[test]
    fn emit_prefix_increment_expr() {
        use BinaryOperator::*;
        use Expression::*;
        use Instruction::*;
        use Value::*;

        let mut emitter = make_emitter();
        let stmt = Statement::Return(typed(Assignment {
            left: Box::new(typed(Var("a".to_string()))),
            right: Box::new(typed(BinaryExpr(
                BinaryOp::Add,
                Box::new(typed(Var("a".to_string()))),
                Box::new(typed(Expression::IntegerConstant(1))),
            ))),
            is_postfix: false,
        }));

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

        let actual = emitter.emit_statement(&stmt).unwrap();
        assert_eq!(expected, actual);
    }

    #[test]
    fn emit_postfix_increment_expr() {
        use BinaryOperator::*;
        use Expression::*;
        use Instruction::*;
        use Value::*;

        let mut emitter = make_emitter();
        let stmt = Statement::Return(typed(Assignment {
            left: Box::new(typed(Var("a".to_string()))),
            right: Box::new(typed(BinaryExpr(
                BinaryOp::Add,
                Box::new(typed(Var("a".to_string()))),
                Box::new(typed(Expression::IntegerConstant(1))),
            ))),
            is_postfix: true,
        }));

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

        let actual = emitter.emit_statement(&stmt).unwrap();
        assert_eq!(expected, actual);
    }

    #[test]
    fn emit_prefix_decrement_expr() {
        use BinaryOperator::*;
        use Expression::*;
        use Instruction::*;
        use Value::*;

        let mut emitter = make_emitter();
        let stmt = Statement::Return(typed(Assignment {
            left: Box::new(typed(Var("a".to_string()))),
            right: Box::new(typed(BinaryExpr(
                BinaryOp::Subtract,
                Box::new(typed(Var("a".to_string()))),
                Box::new(typed(Expression::IntegerConstant(1))),
            ))),
            is_postfix: false,
        }));

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

        let actual = emitter.emit_statement(&stmt).unwrap();
        assert_eq!(expected, actual);
    }

    #[test]
    fn emit_postfix_decrement_expr() {
        use BinaryOperator::*;
        use Expression::*;
        use Instruction::*;
        use Value::*;

        let mut emitter = make_emitter();
        let stmt = Statement::Return(typed(Assignment {
            left: Box::new(typed(Var("a".to_string()))),
            right: Box::new(typed(BinaryExpr(
                BinaryOp::Subtract,
                Box::new(typed(Var("a".to_string()))),
                Box::new(typed(Expression::IntegerConstant(1))),
            ))),
            is_postfix: true,
        }));

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

        let actual = emitter.emit_statement(&stmt).unwrap();
        assert_eq!(expected, actual);
    }

    #[test]
    fn emit_if_statement_without_else() {
        use Instruction::*;

        let mut emitter = make_emitter();
        let stmt = Statement::IfStatement {
            condition: typed(Expression::IntegerConstant(1)),
            then_branch: Box::new(Statement::Return(typed(Expression::IntegerConstant(42)))),
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

        let actual = emitter.emit_statement(&stmt).unwrap();
        assert_eq!(expected, actual);
    }

    #[test]
    fn emit_if_statement_with_else() {
        use Expression::*;
        use Instruction::*;

        let mut emitter = make_emitter();
        let stmt = Statement::IfStatement {
            condition: typed(IntegerConstant(1)),
            then_branch: Box::new(Statement::Return(typed(IntegerConstant(42)))),
            else_branch: Some(Box::new(Statement::Return(typed(IntegerConstant(0))))),
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

        let actual = emitter.emit_statement(&stmt).unwrap();
        assert_eq!(expected, actual);
    }

    #[test]
    fn emit_conditional_expression_simple() {
        use Instruction::*;
        use Value::*;

        let mut emitter = make_emitter();
        let stmt = Statement::Return(typed(Expression::ConditionalExpr {
            condition: Box::new(typed(Expression::IntegerConstant(1))),
            then_expr: Box::new(typed(Expression::IntegerConstant(42))),
            else_expr: Box::new(typed(Expression::IntegerConstant(0))),
        }));

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

        let actual = emitter.emit_statement(&stmt).unwrap();
        assert_eq!(expected, actual);
    }

    #[test]
    fn emit_nested_if_dangling_else_binds_to_inner_if() {
        use Instruction::*;
        use Value::*;

        let mut emitter = make_emitter();
        let stmt = Statement::IfStatement {
            condition: typed(Expression::Var("a".to_string())),
            then_branch: Box::new(Statement::IfStatement {
                condition: typed(Expression::Var("b".to_string())),
                then_branch: Box::new(Statement::Return(typed(Expression::IntegerConstant(1)))),
                else_branch: Some(Box::new(Statement::Return(typed(
                    Expression::IntegerConstant(2),
                )))),
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

        let actual = emitter.emit_statement(&stmt).unwrap();
        assert_eq!(expected, actual);
    }

    #[test]
    fn emit_while_with_break_and_continue() {
        use Instruction::*;
        use Value::*;

        let mut emitter = make_emitter();
        let stmt = Statement::While {
            loop_id: "loop.0".to_string(),
            condition: typed(Expression::IntegerConstant(1)),
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

        let actual = emitter.emit_statement(&stmt).unwrap();
        assert_eq!(expected, actual);
    }

    #[test]
    fn emit_do_while_with_break_and_continue() {
        use Instruction::*;
        use Value::*;

        let mut emitter = make_emitter();
        let stmt = Statement::DoWhile {
            loop_id: "loop.1".to_string(),
            condition: typed(Expression::IntegerConstant(1)),
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

        let actual = emitter.emit_statement(&stmt).unwrap();
        assert_eq!(expected, actual);
    }

    #[test]
    fn emit_func_call_no_args() {
        use Instruction::*;
        use Value::*;

        let mut emitter = make_emitter();
        let stmt = Statement::Return(typed(Expression::FuncCall {
            name: "foo".to_string(),
            args: vec![],
        }));

        let expected = vec![
            FunctionCall {
                name: "foo".to_string(),
                arguments: vec![],
                dst: Variable("tmp.0".to_string()),
            },
            Return(Variable("tmp.0".to_string())),
        ];

        let actual = emitter.emit_statement(&stmt).unwrap();
        assert_eq!(expected, actual);
    }

    #[test]
    fn emit_func_call_with_args() {
        use BinaryOperator::*;
        use Instruction::*;
        use Value::*;

        let mut emitter = make_emitter();
        // bar(1, 2 + 3)
        let stmt = Statement::Return(typed(Expression::FuncCall {
            name: "bar".to_string(),
            args: vec![
                typed(Expression::IntegerConstant(1)),
                typed(Expression::BinaryExpr(
                    BinaryOp::Add,
                    Box::new(typed(Expression::IntegerConstant(2))),
                    Box::new(typed(Expression::IntegerConstant(3))),
                )),
            ],
        }));

        let expected = vec![
            // evaluate 2 + 3 first
            Binary {
                op: Add,
                src1: IntegerConstant(2),
                src2: IntegerConstant(3),
                dst: Variable("tmp.0".to_string()),
            },
            // function call with evaluated args
            FunctionCall {
                name: "bar".to_string(),
                arguments: vec![IntegerConstant(1), Variable("tmp.0".to_string())],
                dst: Variable("tmp.1".to_string()),
            },
            Return(Variable("tmp.1".to_string())),
        ];

        let actual = emitter.emit_statement(&stmt).unwrap();
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
            condition: Some(typed(Expression::IntegerConstant(1))),
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

        let actual = emitter.emit_statement(&stmt).unwrap();
        assert_eq!(expected, actual);
    }
}

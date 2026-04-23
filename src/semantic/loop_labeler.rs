use crate::ast::{Label, Statement};
use crate::semantic::walker;
use crate::semantic::walker::WalkerMut;
use crate::semantic::{NameGeneratorRef, name_generator};
use anyhow::Result;

pub struct LoopLabeler {
    loop_id_generator: NameGeneratorRef,
    switch_id_generator: NameGeneratorRef,
    target_ids: Vec<TargetId>,
    cnt_switch_arms: usize,
}

impl LoopLabeler {
    pub fn new() -> Self {
        Self {
            loop_id_generator: name_generator::make_loop_id_generator(),
            switch_id_generator: name_generator::make_switch_id_generator(),
            target_ids: Vec::new(),
            cnt_switch_arms: 0,
        }
    }

    pub fn label_loops(&mut self, program: &mut crate::ast::Program) -> Result<()> {
        walker::walk(program, self)
    }

    fn get_break_target(&self) -> Result<String> {
        match self.target_ids.last() {
            Some(TargetId::Loop(id)) | Some(TargetId::Switch(id)) => Ok(id.clone()),
            None => Err(anyhow::anyhow!(
                "break statement not inside a loop or a switch statement"
            )),
        }
    }

    fn get_innermost_loop_id(&self) -> Result<String> {
        for target_id in self.target_ids.iter().rev() {
            match target_id {
                TargetId::Loop(id) => return Ok(id.clone()),
                TargetId::Switch(_) => continue,
            }
        }
        Err(anyhow::anyhow!("not inside a loop"))
    }

    fn get_innermost_switch_id(&self) -> Result<String> {
        for target_id in self.target_ids.iter().rev() {
            match target_id {
                TargetId::Loop(_) => continue,
                TargetId::Switch(switch_id) => return Ok(switch_id.clone()),
            }
        }
        Err(anyhow::anyhow!("not inside a switch statement"))
    }
}

impl WalkerMut for LoopLabeler {
    fn enter_statement(&mut self, stmt: &mut Statement) -> Result<()> {
        match stmt {
            Statement::While { loop_id, .. }
            | Statement::For { loop_id, .. }
            | Statement::DoWhile { loop_id, .. } => {
                let unique_loop_id = self.loop_id_generator.borrow_mut().make_unique_name("");
                self.target_ids.push(TargetId::Loop(unique_loop_id.clone()));
                *loop_id = unique_loop_id;
            }
            Statement::SwitchStatement { switch_id, .. } => {
                let unique_switch_id = self.switch_id_generator.borrow_mut().make_unique_name("");
                self.target_ids
                    .push(TargetId::Switch(unique_switch_id.clone()));
                *switch_id = unique_switch_id;
                self.cnt_switch_arms = 0;
            }
            Statement::Break { loop_id } => {
                *loop_id = self.get_break_target()?;
            }
            Statement::Continue { loop_id } => {
                *loop_id = self.get_innermost_loop_id()?;
            }
            Statement::LabeledStatement { label, .. } => match label {
                Label::Case { case_id: id, .. } | Label::Default { default_id: id } => {
                    let switch_id = self.get_innermost_switch_id()?;
                    self.cnt_switch_arms += 1;
                    *id = format!("case.{switch_id}.{}", self.cnt_switch_arms);
                }
                _ => {}
            },
            _ => {}
        }
        Ok(())
    }

    fn leave_statement(&mut self, stmt: &mut Statement) -> Result<()> {
        match stmt {
            Statement::While { .. }
            | Statement::For { .. }
            | Statement::DoWhile { .. }
            | Statement::SwitchStatement { .. } => {
                self.target_ids.pop();
            }
            _ => {}
        }
        Ok(())
    }
}

enum TargetId {
    Loop(String),
    Switch(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Block, BlockItem, Expression, FunctionDefinition, Label, Program, Statement};

    #[test]
    fn rejects_break_outside_loop() {
        let mut program = program_with_statement(Statement::Break {
            loop_id: String::new(),
        });

        let mut labeler = LoopLabeler::new();
        let result = labeler.label_loops(&mut program);

        assert_outside_loop_error(result);
    }

    #[test]
    fn rejects_continue_outside_loop() {
        let mut program = program_with_statement(Statement::Continue {
            loop_id: String::new(),
        });

        let mut labeler = LoopLabeler::new();
        let result = labeler.label_loops(&mut program);

        assert_outside_loop_error(result);
    }

    #[test]
    fn labels_break_and_continue_in_single_loop() {
        let mut program = program_with_statement(Statement::While {
            loop_id: String::new(),
            condition: Expression::IntegerConstant(1),
            body: Box::new(Statement::CompoundStatement(Block::new(vec![
                BlockItem::Statement(Statement::Break {
                    loop_id: String::new(),
                }),
                BlockItem::Statement(Statement::Continue {
                    loop_id: String::new(),
                }),
            ]))),
        });

        let mut labeler = LoopLabeler::new();
        labeler
            .label_loops(&mut program)
            .expect("Expected loop labeler to succeed");

        match &program.function_definition.body.items[0] {
            BlockItem::Statement(Statement::While { loop_id, body, .. }) => {
                assert_eq!(loop_id, "loop.0");

                match body.as_ref() {
                    Statement::CompoundStatement(block) => {
                        match &block.items[0] {
                            BlockItem::Statement(Statement::Break { loop_id }) => {
                                assert_eq!(loop_id, "loop.0")
                            }
                            _ => panic!("Expected first loop body item to be break"),
                        }

                        match &block.items[1] {
                            BlockItem::Statement(Statement::Continue { loop_id }) => {
                                assert_eq!(loop_id, "loop.0")
                            }
                            _ => panic!("Expected second loop body item to be continue"),
                        }
                    }
                    _ => panic!("Expected while body to be a compound statement"),
                }
            }
            _ => panic!("Expected top-level statement to be while"),
        }
    }

    #[test]
    fn labels_break_and_continue_in_nested_loops() {
        let mut program = program_with_statement(Statement::While {
            loop_id: String::new(),
            condition: Expression::IntegerConstant(1),
            body: Box::new(Statement::CompoundStatement(Block::new(vec![
                BlockItem::Statement(Statement::Continue {
                    loop_id: String::new(),
                }),
                BlockItem::Statement(Statement::While {
                    loop_id: String::new(),
                    condition: Expression::IntegerConstant(1),
                    body: Box::new(Statement::CompoundStatement(Block::new(vec![
                        BlockItem::Statement(Statement::Break {
                            loop_id: String::new(),
                        }),
                        BlockItem::Statement(Statement::Continue {
                            loop_id: String::new(),
                        }),
                    ]))),
                }),
                BlockItem::Statement(Statement::Break {
                    loop_id: String::new(),
                }),
            ]))),
        });

        let mut labeler = LoopLabeler::new();
        labeler
            .label_loops(&mut program)
            .expect("Expected loop labeler to succeed");

        match &program.function_definition.body.items[0] {
            BlockItem::Statement(Statement::While { loop_id, body, .. }) => {
                assert_eq!(loop_id, "loop.0");

                match body.as_ref() {
                    Statement::CompoundStatement(outer_body) => {
                        match &outer_body.items[0] {
                            BlockItem::Statement(Statement::Continue { loop_id }) => {
                                assert_eq!(loop_id, "loop.0")
                            }
                            _ => panic!("Expected first outer item to be continue"),
                        }

                        match &outer_body.items[1] {
                            BlockItem::Statement(Statement::While { loop_id, body, .. }) => {
                                assert_eq!(loop_id, "loop.1");
                                match body.as_ref() {
                                    Statement::CompoundStatement(inner_body) => {
                                        match &inner_body.items[0] {
                                            BlockItem::Statement(Statement::Break { loop_id }) => {
                                                assert_eq!(loop_id, "loop.1")
                                            }
                                            _ => panic!("Expected first inner item to be break"),
                                        }
                                        match &inner_body.items[1] {
                                            BlockItem::Statement(Statement::Continue {
                                                loop_id,
                                            }) => {
                                                assert_eq!(loop_id, "loop.1")
                                            }
                                            _ => {
                                                panic!("Expected second inner item to be continue")
                                            }
                                        }
                                    }
                                    _ => panic!("Expected inner while body to be compound"),
                                }
                            }
                            _ => panic!("Expected second outer item to be inner while"),
                        }

                        match &outer_body.items[2] {
                            BlockItem::Statement(Statement::Break { loop_id }) => {
                                assert_eq!(loop_id, "loop.0")
                            }
                            _ => panic!("Expected third outer item to be break"),
                        }
                    }
                    _ => panic!("Expected outer while body to be a compound statement"),
                }
            }
            _ => panic!("Expected top-level statement to be while"),
        }
    }

    #[test]
    fn labels_switch_statement_targets_and_arms() {
        let mut program = program_with_statement(Statement::SwitchStatement {
            switch_id: String::new(),
            condition: Expression::Var("x".to_string()),
            body: Box::new(Statement::CompoundStatement(Block::new(vec![
                BlockItem::Statement(Statement::LabeledStatement {
                    label: Label::Case {
                        case_id: String::new(),
                        value: Expression::IntegerConstant(1),
                    },
                    statement: Box::new(Statement::Break {
                        loop_id: String::new(),
                    }),
                }),
                BlockItem::Statement(Statement::LabeledStatement {
                    label: Label::Case {
                        case_id: String::new(),
                        value: Expression::IntegerConstant(2),
                    },
                    statement: Box::new(Statement::Null),
                }),
                BlockItem::Statement(Statement::LabeledStatement {
                    label: Label::Default {
                        default_id: String::new(),
                    },
                    statement: Box::new(Statement::Break {
                        loop_id: String::new(),
                    }),
                }),
            ]))),
        });

        let mut labeler = LoopLabeler::new();
        labeler
            .label_loops(&mut program)
            .expect("Expected loop labeler to succeed");

        match &program.function_definition.body.items[0] {
            BlockItem::Statement(Statement::SwitchStatement {
                switch_id, body, ..
            }) => {
                assert_eq!(switch_id, "switch.0");

                match body.as_ref() {
                    Statement::CompoundStatement(block) => {
                        match &block.items[0] {
                            BlockItem::Statement(Statement::LabeledStatement { label, statement }) => {
                                match label {
                                    Label::Case { case_id, .. } => {
                                        assert_eq!(case_id, "case.switch.0.1")
                                    }
                                    _ => panic!("Expected first switch arm to be case"),
                                }
                                match statement.as_ref() {
                                    Statement::Break { loop_id } => assert_eq!(loop_id, "switch.0"),
                                    _ => panic!("Expected first case statement to be break"),
                                }
                            }
                            _ => panic!("Expected first switch body item to be labeled statement"),
                        }

                        match &block.items[1] {
                            BlockItem::Statement(Statement::LabeledStatement { label, .. }) => {
                                match label {
                                    Label::Case { case_id, .. } => {
                                        assert_eq!(case_id, "case.switch.0.2")
                                    }
                                    _ => panic!("Expected second switch arm to be case"),
                                }
                            }
                            _ => panic!("Expected second switch body item to be labeled statement"),
                        }

                        match &block.items[2] {
                            BlockItem::Statement(Statement::LabeledStatement { label, statement }) => {
                                match label {
                                    Label::Default { default_id } => {
                                        assert_eq!(default_id, "case.switch.0.3")
                                    }
                                    _ => panic!("Expected third switch arm to be default"),
                                }
                                match statement.as_ref() {
                                    Statement::Break { loop_id } => assert_eq!(loop_id, "switch.0"),
                                    _ => panic!("Expected default statement to be break"),
                                }
                            }
                            _ => panic!("Expected third switch body item to be labeled statement"),
                        }
                    }
                    _ => panic!("Expected switch body to be a compound statement"),
                }
            }
            _ => panic!("Expected top-level statement to be switch"),
        }
    }

    fn program_with_statement(statement: Statement) -> Program {
        Program::new(FunctionDefinition::new(
            "main".to_string(),
            Block::new(vec![BlockItem::Statement(statement)]),
        ))
    }

    fn assert_outside_loop_error(result: Result<()>) {
        let err = result.expect_err("Expected loop labeler to reject statement outside loops");
        assert!(
            err.to_string()
                .contains("not inside")
        );
    }
}

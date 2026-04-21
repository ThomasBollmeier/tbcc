use crate::ast::Statement;
use crate::semantic::walker;
use crate::semantic::walker::WalkerMut;
use crate::semantic::{NameGeneratorRef, name_generator};
use anyhow::Result;

pub struct LoopLabeler {
    loop_id_generator: NameGeneratorRef,
    loop_ids: Vec<String>,
}

impl LoopLabeler {
    pub fn new() -> Self {
        Self {
            loop_id_generator: name_generator::make_loop_id_generator(),
            loop_ids: Vec::new(),
        }
    }

    pub fn label_loops(&mut self, program: &mut crate::ast::Program) -> Result<()> {
        walker::walk(program, self)
    }
}

impl WalkerMut for LoopLabeler {
    fn enter_statement(&mut self, stmt: &mut Statement) -> Result<()> {
        match stmt {
            Statement::While { loop_id, .. }
            | Statement::For { loop_id, .. }
            | Statement::DoWhile { loop_id, .. } => {
                let unique_loop_id = self.loop_id_generator.borrow_mut().make_unique_name("");
                self.loop_ids.push(unique_loop_id.clone());
                *loop_id = unique_loop_id;
            }
            Statement::Break { loop_id } | Statement::Continue { loop_id } => {
                if let Some(current_loop_id) = self.loop_ids.last() {
                    *loop_id = current_loop_id.clone();
                } else {
                    return Err(anyhow::anyhow!(
                        "break and continue statements must be inside a loop"
                    ));
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn leave_statement(&mut self, stmt: &mut Statement) -> Result<()> {
        match stmt {
            Statement::While { .. }
            | Statement::For { .. }
            | Statement::DoWhile { .. } => {
                self.loop_ids.pop();
            }
            _ => {}
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Block, BlockItem, Expression, FunctionDefinition, Program, Statement};

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
                            BlockItem::Statement(Statement::While {
                                loop_id,
                                body,
                                ..
                            }) => {
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
                                            BlockItem::Statement(Statement::Continue { loop_id }) => {
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
                .contains("break and continue statements must be inside a loop")
        );
    }
}


use crate::ast::{Expression, Label, Statement};
use crate::semantic::walker;
use crate::semantic::walker::WalkerMut;
use crate::semantic::{NameGeneratorRef, name_generator};
use anyhow::Result;
use std::collections::HashSet;

pub struct LoopLabeler {
    loop_id_generator: NameGeneratorRef,
    switch_id_generator: NameGeneratorRef,
    target_ids: Vec<TargetId>,
    arms: Vec<Vec<(String, Option<Expression>)>>,
}

impl LoopLabeler {
    pub fn new() -> Self {
        Self {
            loop_id_generator: name_generator::make_loop_id_generator(),
            switch_id_generator: name_generator::make_switch_id_generator(),
            target_ids: Vec::new(),
            arms: Vec::new(),
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

    fn validate_arms(&mut self, arms: &Vec<(String, Option<Expression>)>) -> Result<()> {
        let mut cnt_default = 0;
        let mut case_values: HashSet<i32> = HashSet::new();

        for (_, expr) in arms {
            match expr {
                Some(expr) => match expr {
                    Expression::IntegerConstant(value) => {
                        if case_values.contains(&value) {
                            return Err(anyhow::anyhow!("case arm value is already used"));
                        }
                        case_values.insert(*value);
                    }
                    _ => return Err(anyhow::anyhow!("case arm expression is not an integer")),
                },
                None => {
                    cnt_default += 1;
                    if cnt_default > 1 {
                        return Err(anyhow::anyhow!("not more than one default arm allowed"));
                    }
                }
            }
        }

        Ok(())
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
                self.arms.push(Vec::new());
            }
            Statement::Break { loop_id } => {
                *loop_id = self.get_break_target()?;
            }
            Statement::Continue { loop_id } => {
                *loop_id = self.get_innermost_loop_id()?;
            }
            Statement::LabeledStatement { label, .. } => match label {
                Label::Case { case_id, value } => {
                    let switch_id = self.get_innermost_switch_id()?;
                    let current_arms = self.arms.last_mut().unwrap();
                    let cnt = current_arms.len() + 1;
                    *case_id = format!("{switch_id}.case.{cnt}");
                    current_arms.push((case_id.clone(), Some(value.clone())));
                }
                Label::Default { default_id } => {
                    let switch_id = self.get_innermost_switch_id()?;
                    *default_id = format!("{switch_id}.default");
                    let current_arms = self.arms.last_mut().unwrap();
                    current_arms.push((default_id.clone(), None));
                }
                _ => {}
            },
            _ => {}
        }
        Ok(())
    }

    fn leave_statement(&mut self, stmt: &mut Statement) -> Result<()> {
        match stmt {
            Statement::While { .. } | Statement::For { .. } | Statement::DoWhile { .. } => {
                self.target_ids.pop();
            }
            Statement::SwitchStatement { arms, .. } => {
                let current_arms = self.arms.last().unwrap().clone();
                self.validate_arms(&current_arms)?;
                *arms = current_arms;
                self.arms.pop();
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
    use crate::ast::{
        Block, BlockItem, Expression, FunctionDeclaration, Label, Program, Statement,
    };
    use crate::lexer::Lexer;
    use crate::parser::Parser;

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

        let body = program
            .function_decls
            .get(0)
            .expect("Expected main function to be present")
            .body
            .as_ref()
            .expect("Expected main function body to be present");

        match &body.items[0] {
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

        let body = program
            .function_decls
            .get(0)
            .expect("Expected main function to be present")
            .body
            .as_ref()
            .expect("Expected main function body to be present");

        match &body.items[0] {
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
            arms: Vec::new(),
        });

        let mut labeler = LoopLabeler::new();
        labeler
            .label_loops(&mut program)
            .expect("Expected loop labeler to succeed");

        let body = program
            .function_decls
            .get(0)
            .expect("Expected main function to be present")
            .body
            .as_ref()
            .expect("Expected main function body to be present");

        match &body.items[0] {
            BlockItem::Statement(Statement::SwitchStatement {
                switch_id, body, ..
            }) => {
                assert_eq!(switch_id, "switch.0");

                match body.as_ref() {
                    Statement::CompoundStatement(block) => {
                        match &block.items[0] {
                            BlockItem::Statement(Statement::LabeledStatement {
                                label,
                                statement,
                            }) => {
                                match label {
                                    Label::Case { case_id, .. } => {
                                        assert_eq!(case_id, "switch.0.case.1")
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
                                        assert_eq!(case_id, "switch.0.case.2")
                                    }
                                    _ => panic!("Expected second switch arm to be case"),
                                }
                            }
                            _ => panic!("Expected second switch body item to be labeled statement"),
                        }

                        match &block.items[2] {
                            BlockItem::Statement(Statement::LabeledStatement {
                                label,
                                statement,
                            }) => {
                                match label {
                                    Label::Default { default_id } => {
                                        assert_eq!(default_id, "switch.0.default")
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

    #[test]
    fn switch_nested_switch() {
        let code = r#"
        int main(void){
            switch(3) {
                case 0:
                    return 0;
                case 3: {
                    switch(4) {
                        case 3: return 0;
                        case 4: return 1;
                        default: return 0;
                    }
                }
                case 4: return 0;
                default: return 0;
            }
        }
        "#;

        let mut program = parse_code(code).expect("Expected code to parse");
        let mut labeler = LoopLabeler::new();
        labeler
            .label_loops(&mut program)
            .expect("Expected loop labeling to succeed");
    }

    #[test]
    fn finds_duplicate_label() {
        let code = r#"
        int main(void) {
            switch(4) {
                case 5: return 0;
                case 4: return 1;
                case 5: return 0; // duplicate of previous case 5
                default: return 2;
            }
        }
        "#;

        let mut program = parse_code(code).expect("Expected code to parse");
        let mut labeler = LoopLabeler::new();
        match labeler.label_loops(&mut program) {
            Ok(_) => panic!("Expected label loops to fail"),
            Err(_) => {}
        }
    }

    fn parse_code(code: &str) -> Result<Program> {
        let parser = Parser::new();
        let lexer = Lexer::new();

        let tokens = lexer.scan_tokens(code).expect("Failed to scan tokens");
        parser.parse(tokens)
    }

    fn program_with_statement(statement: Statement) -> Program {
        let mut program = Program::new();
        program.add_function_decl(FunctionDeclaration::new(
            "main".to_string(),
            vec![],
            Some(Block::new(vec![BlockItem::Statement(statement)])),
        ));
        program
    }

    fn assert_outside_loop_error(result: Result<()>) {
        let err = result.expect_err("Expected loop labeler to reject statement outside loops");
        assert!(err.to_string().contains("not inside"));
    }
}

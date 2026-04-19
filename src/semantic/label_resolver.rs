use crate::ast::{Block, Declaration, Expression, FunctionDefinition, Program, Statement, VisitorMut};
use crate::semantic::name_generator::NameGeneratorRef;
use crate::semantic::scope::{NamingData, Scope, ScopeRef};
use anyhow::{Result, anyhow};

pub struct LabelResolver {
    label_name_generator: NameGeneratorRef,
    current_scope: Option<ScopeRef<LabelAdditionalData>>,
}

impl LabelResolver {
    pub fn new(label_name_generator: NameGeneratorRef) -> Self {
        Self {
            label_name_generator: label_name_generator.clone(),
            current_scope: None,
        }
    }

    pub fn resolve(&mut self, program: &mut Program) -> Result<()> {
        program.accept_mut(self)
    }

    fn check_for_unique_name(&self, label: &str) -> Result<Option<String>> {
        match &self.current_scope {
            Some(current_scope) => {
                let current_scope = current_scope.borrow();
                let info = current_scope.get_current_info(label);
                match info {
                    Some(info) => {
                        if info.additional.is_declared {
                            return Err(anyhow!("Duplicate declaration for label {}", label));
                        }
                        Ok(Some(info.unique_name.clone()))
                    }
                    None => Ok(None),
                }
            }
            None => Err(anyhow!("labels cannot be used outside of a function")),
        }
    }

    fn get_existing_unique_name(&self, label: &str) -> Result<Option<String>> {
        match &self.current_scope {
            Some(current_scope) => {
                let current_scope = current_scope.borrow();
                Ok(current_scope
                    .get_current_info(label)
                    .map(|info| info.unique_name.clone()))
            }
            None => Err(anyhow!("labels cannot be used outside of a function")),
        }
    }

    fn check_for_undeclared_labels(&self) -> Result<()> {
        let current_scope = self
            .current_scope
            .as_ref()
            .ok_or_else(|| anyhow!("Labels cannot be used outside of a function"))?;
        let current_scope = current_scope.borrow();
        let names = current_scope.get_names_in_current_scope();
        for name in names {
            let info = current_scope.get_current_info(&name).unwrap();
            if !info.additional.is_declared {
                return Err(anyhow!("Label '{}' is used but not declared", name));
            }
        }
        Ok(())
    }

    fn visit_labeled_statement(
        &mut self,
        label: &mut String,
        statement: &mut Box<Statement>,
    ) -> Result<()> {
        match self.check_for_unique_name(label)? {
            Some(unique_name) => {
                let current_scope = self.current_scope.as_mut().unwrap();
                current_scope
                    .borrow_mut()
                    .get_current_info_mut(label)
                    .and_modify(|entry| {
                        entry.additional.is_declared = true;
                    });
                *label = unique_name.clone();
            }
            None => {
                let unique_name = self
                    .label_name_generator
                    .borrow_mut()
                    .make_unique_name(label);
                let current_scope = self.current_scope.as_mut().unwrap();
                let mut current_scope = current_scope.borrow_mut();
                let entry = current_scope.get_current_info_mut(label);
                entry.or_insert(NamingData {
                    unique_name: unique_name.clone(),
                    additional: LabelAdditionalData { is_declared: true },
                });
                *label = unique_name;
            }
        }
        statement.accept_mut(self)
    }

    fn visit_goto_statement(&mut self, label: &mut String) -> Result<()> {
        if let Some(unique_name) = self.get_existing_unique_name(label)? {
            *label = unique_name;
        } else {
            let unique_name = self
                .label_name_generator
                .borrow_mut()
                .make_unique_name(label);
            let current_scope = self.current_scope.as_mut().unwrap();
            let mut current_scope = current_scope.borrow_mut();
            let entry = current_scope.get_current_info_mut(label);
            entry.or_insert(NamingData {
                unique_name: unique_name.clone(),
                additional: LabelAdditionalData { is_declared: false },
            });
            *label = unique_name;
        }
        Ok(())
    }

    fn visit_if_statement(
        &mut self,
        then_branch: &mut Box<Statement>,
        else_branch: &mut Option<Box<Statement>>,
    ) -> Result<()> {
        then_branch.accept_mut(self)?;
        if let Some(else_branch) = else_branch {
            else_branch.accept_mut(self)?;
        }
        Ok(())
    }
}

impl VisitorMut<Result<()>> for LabelResolver {
    fn visit_program(&mut self, program: &mut Program) -> Result<()> {
        program.function_definition.accept_mut(self)
    }

    fn visit_function_definition(&mut self, func_def: &mut FunctionDefinition) -> Result<()> {
        self.current_scope = Some(Scope::new_ref(None, self.label_name_generator.clone()));
        func_def.body.accept_mut(self)?;
        self.check_for_undeclared_labels()?;
        self.current_scope = None;

        Ok(())
    }

    fn visit_block(&mut self, block: &mut Block) -> Result<()> {
        for item in &mut block.items {
            match item {
                crate::ast::BlockItem::Declaration(decl) => decl.accept_mut(self)?,
                crate::ast::BlockItem::Statement(stmt) => stmt.accept_mut(self)?,
            }
        }
        Ok(())
    }

    fn visit_declaration(&mut self, _decl: &mut Declaration) -> Result<()> {
        Ok(())
    }

    fn visit_statement(&mut self, stmt: &mut Statement) -> Result<()> {
        match stmt {
            Statement::LabeledStatement { label, statement } => {
                self.visit_labeled_statement(label, statement)?
            }
            Statement::GotoStatement(label) => {
                self.visit_goto_statement(label)?
            }
            Statement::IfStatement {
                condition: _,
                then_branch,
                else_branch,
            } => {
                self.visit_if_statement(then_branch, else_branch)?;
            }
            _ => {}
        };
        Ok(())
    }

    fn visit_expression(&mut self, _expr: &mut Expression) -> Result<()> {
        Ok(())
    }
}

#[derive(Debug, Default)]
struct LabelAdditionalData {
    is_declared: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{BlockItem, Expression, Statement};
    use crate::lexer::Lexer;
    use crate::parser::Parser;
    use crate::semantic::name_generator::make_label_name_generator;

    #[test]
    fn resolves_label_used_after_if_branch_declaration() {
        let mut program = resolve_code(
            r#"
            int main(void) {
                if (0)
                label:
                    return 5;
                goto label;
                return 0;
            }
            "#,
        )
        .expect("Expected label resolver to succeed for label declared in if branch");

        let body = &mut program.function_definition.body.items;

        let resolved_label = match &body[0] {
            BlockItem::Statement(Statement::IfStatement {
                condition: _,
                then_branch,
                else_branch,
            }) => {
                assert!(else_branch.is_none(), "Expected if statement without else branch");
                match then_branch.as_ref() {
                    Statement::LabeledStatement { label, statement } => {
                        match statement.as_ref() {
                            Statement::Return(Expression::IntegerConstant(5)) => {}
                            _ => panic!("Expected labeled statement to wrap 'return 5'")
                        }
                        label.clone()
                    }
                    _ => panic!("Expected then-branch to be a labeled statement"),
                }
            }
            _ => panic!("Expected first body item to be an if statement"),
        };

        assert_eq!(resolved_label, "label_0");

        match &body[1] {
            BlockItem::Statement(Statement::GotoStatement(label)) => {
                assert_eq!(label, &resolved_label);
            }
            _ => panic!("Expected second body item to be a goto statement"),
        }

        match &body[2] {
            BlockItem::Statement(Statement::Return(Expression::IntegerConstant(0))) => {}
            _ => panic!("Expected third body item to be 'return 0'")
        }
    }

    fn resolve_code(code: &str) -> Result<Program> {
        let lexer = Lexer::new();
        let parser = Parser::new();
        let tokens = lexer.scan_tokens(code)?;
        let mut program = parser.parse(tokens)?;

        let label_name_generator = make_label_name_generator();
        let mut resolver = LabelResolver::new(label_name_generator);
        resolver.resolve(&mut program)?;

        Ok(program)
    }
}

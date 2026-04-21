use crate::ast::{Block, Declaration, Expression, Program, Statement};
use crate::semantic::name_generator::NameGeneratorRef;
use crate::semantic::scope::{Scope, ScopeRef};
use crate::semantic::walker;
use crate::semantic::walker::WalkerMut;
use anyhow::{Result, anyhow};

pub struct VariableResolver {
    var_name_generator: NameGeneratorRef,
    current_scope: ScopeRef,
}

impl VariableResolver {
    pub fn new(var_name_generator: NameGeneratorRef) -> Self {
        Self {
            var_name_generator: var_name_generator.clone(),
            current_scope: Scope::new_ref(None, var_name_generator),
        }
    }

    pub fn resolve(&mut self, program: &mut Program) -> Result<()> {
        walker::walk(program, self)
    }

    fn open_scope(&mut self) {
        self.current_scope = Scope::new_ref(
            Some(self.current_scope.clone()),
            self.var_name_generator.clone(),
        );
    }

    fn close_scope(&mut self) {
        let parent = self.current_scope.borrow().get_parent().unwrap();
        self.current_scope = parent;
    }
}

impl WalkerMut for VariableResolver {
    fn enter_block(&mut self, _: &mut Block) -> Result<()> {
        self.open_scope();
        Ok(())
    }

    fn leave_block(&mut self, _: &mut Block) -> Result<()> {
        self.close_scope();
        Ok(())
    }

    fn enter_declaration(&mut self, decl: &mut Declaration) -> Result<()> {
        let unique_name = self.current_scope.borrow_mut().add(&decl.name)?;
        decl.name = unique_name;
        Ok(())
    }

    fn enter_statement(&mut self, stmt: &mut Statement) -> Result<()> {
        match stmt {
            Statement::For { .. } => {
                self.open_scope();
            }
            _ => {}
        }

        Ok(())
    }

    fn leave_statement(&mut self, stmt: &mut Statement) -> Result<()> {
        match stmt {
            Statement::For { .. } => {
                self.close_scope();
            }
            _ => {}
        }

        Ok(())
    }

    fn enter_expression(&mut self, expr: &mut Expression) -> Result<()> {
        use Expression::*;
        match expr {
            Assignment {
                left,
                right: _,
                is_postfix: _,
            } => match **left {
                Var(_) => {}
                _ => return Err(anyhow!("Left-hand side of assignment must be a variable")),
            },
            Var(name) => {
                if let Some(unique_name) = self.current_scope.borrow().get_unique_name(name) {
                    *name = unique_name;
                } else {
                    return Err(anyhow!("Variable `{name}` is not defined"));
                }
            }
            _ => {}
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{BlockItem, Expression, Statement};
    use crate::lexer::Lexer;
    use crate::parser::Parser;
    use crate::semantic::name_generator::make_var_name_generator;

    #[test]
    fn resolves_variable_names_in_declarations_and_usages() {
        let mut program = resolve_code(
            r#"
            int main(void) {
                int x = 1;
                int y = x;
                x = y;
                return x;
            }
            "#,
        )
        .expect("Expected variable resolver to succeed");

        let body = &mut program.function_definition.body.items;

        match &body[0] {
            BlockItem::Declaration(decl) => assert_eq!(decl.name, "var.x.0"),
            _ => panic!("Expected first item to be declaration"),
        }

        match &body[1] {
            BlockItem::Declaration(decl) => {
                assert_eq!(decl.name, "var.y.0");
                match decl.init_expr.as_ref() {
                    Some(Expression::Var(name)) => assert_eq!(name, "var.x.0"),
                    _ => panic!("Expected initializer to be variable expression"),
                }
            }
            _ => panic!("Expected second item to be declaration"),
        }

        match &body[2] {
            BlockItem::Statement(Statement::Expression(Expression::Assignment {
                left,
                right,
                is_postfix: _,
            })) => {
                match left.as_ref() {
                    Expression::Var(name) => assert_eq!(name, "var.x.0"),
                    _ => panic!("Expected assignment left side to be variable"),
                }
                match right.as_ref() {
                    Expression::Var(name) => assert_eq!(name, "var.y.0"),
                    _ => panic!("Expected assignment right side to be variable"),
                }
            }
            _ => panic!("Expected third item to be assignment expression statement"),
        }

        match &body[3] {
            BlockItem::Statement(Statement::Return(Expression::Var(name))) => {
                assert_eq!(name, "var.x.0")
            }
            _ => panic!("Expected fourth item to be return variable statement"),
        }
    }

    #[test]
    fn fails_on_undefined_variable() {
        let result = resolve_code(
            r#"
            int main(void) {
                return 0 && x;
            }
            "#,
        );

        let err = result.expect_err("Expected undefined variable error");
        assert!(err.to_string().contains("Variable `x` is not defined"));
    }

    #[test]
    fn fails_on_invalid_assignment_lhs() {
        let result = resolve_code(
            r#"
            int main(void) {
                int x = 1;
                1 = x;
                return x;
            }
            "#,
        );

        let err = result.expect_err("Expected invalid assignment error");
        assert!(
            err.to_string()
                .contains("Left-hand side of assignment must be a variable")
        );
    }

    #[test]
    fn fails_on_duplicate_variable_declaration() {
        let result = resolve_code(
            r#"
            int main(void) {
                {
                    int x = 1;
                    int x = 2;
                }
                return 1;
            }
            "#,
        );

        let err = result.expect_err("Expected duplicate declaration error");
        assert!(
            err.to_string()
                .contains("'x' already exists in current scope")
        );
    }

    #[test]
    fn fails_on_postfix_inc_of_non_lvalue() {
        let result = resolve_code(
            r#"
            int main(void) {
                int a = 0;
                (a = 4)++;
            }
            "#,
        );

        let err = result.expect_err("Expected non-lvalue expression error");
        assert!(
            err.to_string()
                .contains("Left-hand side of assignment must be a variable")
        );
    }

    fn resolve_code(code: &str) -> Result<Program> {
        let lexer = Lexer::new();
        let parser = Parser::new();
        let tokens = lexer.scan_tokens(code)?;
        let mut program = parser.parse(tokens)?;

        let var_name_generator = make_var_name_generator();
        let mut resolver = VariableResolver::new(var_name_generator);
        resolver.resolve(&mut program)?;

        Ok(program)
    }
}

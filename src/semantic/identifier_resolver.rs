use crate::ast::{Block, Declaration, Expression, FunctionDeclaration, Program, Statement};
use crate::semantic::name_generator::NameGeneratorRef;
use crate::semantic::scope::{ResolutionStrategy, Scope, ScopeRef};
use crate::semantic::walker;
use crate::semantic::walker::WalkerMut;
use anyhow::{Result, anyhow};
use std::rc::Rc;

pub struct IdentifierResolver {
    var_name_generator: NameGeneratorRef,
    current_scope: ScopeRef<IdentifierAdditional>,
    function_nesting_level: usize,
    block_nesting_levels: Vec<usize>,
}

impl IdentifierResolver {
    pub fn new(var_name_generator: NameGeneratorRef) -> Self {
        Self {
            var_name_generator: var_name_generator.clone(),
            current_scope: Scope::new_ref(None, var_name_generator, IdentifierStrategy::new_ref()),
            function_nesting_level: 0,
            block_nesting_levels: vec![],
        }
    }

    pub fn resolve(&mut self, program: &mut Program) -> Result<()> {
        walker::walk(program, self)
    }

    fn open_scope(&mut self) {
        let strategy = self.current_scope.borrow().strategy.clone();
        self.current_scope = Scope::new_ref(
            Some(self.current_scope.clone()),
            self.var_name_generator.clone(),
            strategy,
        );
    }

    fn close_scope(&mut self) {
        let parent = self.current_scope.borrow().get_parent().unwrap();
        self.current_scope = parent;
    }
}

impl WalkerMut for IdentifierResolver {
    fn enter_func_decl(&mut self, func_decl: &mut FunctionDeclaration) -> Result<()> {
        if func_decl.body.is_some() && self.function_nesting_level > 0 {
            return Err(anyhow!("Nested function definitions are not allowed"));
        }

        let additional_data = IdentifierAdditional {
            linkage: Linkage::External,
        };
        let unique_name = self
            .current_scope
            .borrow_mut()
            .add(&func_decl.name, additional_data)?;
        func_decl.name = unique_name;

        self.open_scope();

        for param in &mut func_decl.parameters {
            let additional_data = IdentifierAdditional {
                linkage: Linkage::None,
            };
            let unique_name = self
                .current_scope
                .borrow_mut()
                .add(param, additional_data)?;
            *param = unique_name;
        }

        self.function_nesting_level += 1;
        self.block_nesting_levels.push(0);

        Ok(())
    }

    fn leave_func_decl(&mut self, _: &mut FunctionDeclaration) -> Result<()> {
        self.function_nesting_level -= 1;
        self.block_nesting_levels.pop();
        self.close_scope();
        Ok(())
    }

    fn enter_block(&mut self, _: &mut Block) -> Result<()> {
        if self.block_nesting_levels.last().unwrap() > &0 {
            self.open_scope();
        }
        self.block_nesting_levels.last_mut().map(|level| *level += 1);

        Ok(())
    }

    fn leave_block(&mut self, _: &mut Block) -> Result<()> {
        self.block_nesting_levels.last_mut().map(|level| *level -= 1);
        if self.block_nesting_levels.last().unwrap() > &0 {
            self.close_scope();
        }

        Ok(())
    }

    fn enter_declaration(&mut self, decl: &mut Declaration) -> Result<()> {
        let additional_data = IdentifierAdditional {
            linkage: Linkage::None,
        };
        let unique_name = self
            .current_scope
            .borrow_mut()
            .add(&decl.name, additional_data)?;
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
            FuncCall { name, args: _ } => {
                if let Some(unique_name) = self.current_scope.borrow().get_unique_name(name) {
                    *name = unique_name;
                } else {
                    return Err(anyhow!("Function `{name}` is not defined"));
                }
            }
            _ => {}
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
struct IdentifierAdditional {
    linkage: Linkage,
}

#[derive(Debug, Clone, PartialEq)]
enum Linkage {
    External,
    None,
}

struct IdentifierStrategy;

impl IdentifierStrategy {
    fn new_ref() -> Rc<Self> {
        Rc::new(Self)
    }
}

impl ResolutionStrategy<IdentifierAdditional> for IdentifierStrategy {
    fn check_add_name_to_scope(
        &self,
        name: &str,
        existing_entry: &Option<IdentifierAdditional>,
        exists_in_current_scope: bool,
        new_additional_data: &IdentifierAdditional,
    ) -> Result<()> {
        if exists_in_current_scope {
            let existing_data = existing_entry.as_ref().unwrap();
            if existing_data.linkage != Linkage::External
                || existing_data.linkage != new_additional_data.linkage
            {
                return Err(anyhow!("'{name}' already exists in current scope"));
            }
        }

/*        match (existing_entry, new_additional_data) {
            (Some(IdentifierAdditional { kind: IdentifierKind::Function { is_definition: true}, .. }),
                IdentifierAdditional { kind: IdentifierKind::Function { is_definition: true}, .. }) => {
                    return Err(anyhow!("Redefinition of function `{name}`"));
                }
            _ => {}
        }
*/
        Ok(())
    }

    fn create_unique_name(
        &self,
        name: &str,
        _: &Option<IdentifierAdditional>,
        _: bool,
        new_additional_data: &IdentifierAdditional,
        name_generator: NameGeneratorRef,
    ) -> Result<String> {
        let unique_name = if new_additional_data.linkage == Linkage::None {
            name_generator.borrow_mut().make_unique_name(name)
        } else {
            name.to_string()
        };

        Ok(unique_name)
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
        let program = resolve_code(
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

        let body = program
            .function_decls
            .get(0)
            .expect("Expected main function to be present")
            .body
            .as_ref()
            .expect("Expected main function body to be present");

        let body = &body.items;

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

    #[test]
    fn fails_on_duplicate_parameters_in_function_declaration() {
        let result = resolve_code(
            r#"
            int foo(int a, int b, int a);
            "#,
        );

        result.expect_err("Expected duplicate parameter error");
    }

    #[test]
    fn fails_on_redefinition_of_parameters_in_body() {
        let result = resolve_code(
            r#"
            int foo(int a) {
                int a = 5;
                return a;
            }
            "#,
        );

        result.expect_err("Expected duplicate parameter error");
    }

    #[test]
    fn variable_shadows_function_ok() {
        let result = resolve_code(
            r#"
            int main(void) {
                int doit();
                if (1) {
                    int doit = 5;
                    return doit;
                }
            }

            int doit(void) {
                return 42;
            }
            "#,
        );

        result.expect("Expected variable shadowing function to be allowed");
    }

    #[test]
    fn parameter_shadows_var_ok() {
        let result = resolve_code(
            r#"
            int main(void) {
                int x = 5;
                int foo(int x);
                return foo(x);
            }

            int foo(int i) {
                return i + 1;
            }
            "#,
        );

        result.expect("Expected parameter shadowing variable to be allowed");
    }

    fn resolve_code(code: &str) -> Result<Program> {
        let lexer = Lexer::new();
        let parser = Parser::new();
        let tokens = lexer.scan_tokens(code)?;
        let mut program = parser.parse(tokens)?;

        let var_name_generator = make_var_name_generator();
        let mut resolver = IdentifierResolver::new(var_name_generator);
        resolver.resolve(&mut program)?;

        Ok(program)
    }
}

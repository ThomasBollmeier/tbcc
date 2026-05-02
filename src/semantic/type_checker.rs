use crate::ast::{Expression, Program};
use crate::semantic::symbol_table;
use crate::semantic::symbol_table::{CType, SymbolTableEntry};
use crate::semantic::walker::{WalkerMut, walk};
use anyhow::anyhow;

pub struct TypeChecker;

impl TypeChecker {
    pub fn new() -> Self {
        Self {}
    }

    pub fn check(&mut self, program: &mut Program) -> anyhow::Result<()> {
        walk(program, self)
    }
}

impl WalkerMut for TypeChecker {
    fn enter_func_decl(
        &mut self,
        func_decl: &mut crate::ast::FunctionDeclaration,
    ) -> anyhow::Result<()> {
        let is_func_defined = func_decl.body.is_some();
        let num_params = func_decl.parameters.len();

        match symbol_table::get(&func_decl.name) {
            Some(SymbolTableEntry {
                c_type,
            }) => {
                match c_type {
                    CType::Function {
                        num_params: num_params_other,
                        is_defined: is_func_defined_other,
                    } => {
                        if num_params_other != num_params {
                            return Err(anyhow!(
                                "Function '{}' declared with {} parameters, but previous declaration has {} parameters",
                                func_decl.name,
                                num_params,
                                num_params_other
                            ));
                        }
                        if is_func_defined && is_func_defined_other {
                            return Err(anyhow!(
                                "Function '{}' has already been defined",
                                func_decl.name
                            ));
                        }
                        if is_func_defined {
                            // Update the symbol table entry to mark the function as defined
                            symbol_table::with_global_symbol_table_mut(|table| {
                                table.modify(&func_decl.name).and_modify(|entry| {
                                    if let CType::Function { num_params, is_defined: _ } = &mut entry.c_type {
                                        *entry = SymbolTableEntry {
                                            c_type: CType::Function { num_params: *num_params, is_defined: true },
                                        };
                                    }
                                });
                            });

                            for param in &func_decl.parameters {
                                symbol_table::insert(
                                    param.clone(),
                                    SymbolTableEntry {
                                        c_type: CType::Int,
                                    },
                                );
                            }
                        }
                    }
                    _ => {
                        return Err(anyhow!(
                            "Name '{}' has already been declared as a non-function",
                            func_decl.name
                        ));
                    }
                }
            }
            None => {
                symbol_table::insert(
                    func_decl.name.clone(),
                    SymbolTableEntry {
                        c_type: CType::Function { num_params, is_defined: is_func_defined },
                    },
                );

                if is_func_defined {
                    for param in &func_decl.parameters {
                        symbol_table::insert(param.clone(), SymbolTableEntry {
                            c_type: CType::Int,
                        });
                    }
                }
            }
        }

        Ok(())
    }

    fn enter_declaration(&mut self, decl: &mut crate::ast::VarDeclaration) -> anyhow::Result<()> {
        if let Some(_) = symbol_table::get(&decl.name) {
            return Err(anyhow!("Name '{}' has already been defined", decl.name));
        }

        symbol_table::insert(
            decl.name.clone(),
            SymbolTableEntry {
                c_type: CType::Int,
            },
        );

        Ok(())
    }

    fn enter_expression(&mut self, expr: &mut Expression) -> anyhow::Result<()> {
        match expr {
            Expression::Var(name) => {
                if let Some(entry) = symbol_table::get(name) {
                    match entry.c_type {
                        CType::Int => {}
                        CType::Function { .. } => {
                            return Err(anyhow!(
                                "Identifier '{}' is a function, but used as a variable",
                                name
                            ));
                        }
                    }
                } else {
                    return Err(anyhow!("Undefined variable '{}'", name));
                }
            }
            Expression::FuncCall { name, args } => {
                if let Some(entry) = symbol_table::get(name) {
                    match entry.c_type {
                        CType::Function { num_params, is_defined: _ } => {
                            if args.len() != num_params {
                                return Err(anyhow!(
                                    "Function '{}' called with {} arguments, but expects {}",
                                    name,
                                    args.len(),
                                    num_params
                                ));
                            }
                        }
                        CType::Int => {
                            return Err(anyhow!(
                                "Identifier '{}' is a variable, but used as a function",
                                name
                            ));
                        }
                    }
                } else {
                    return Err(anyhow!("Undefined function '{}'", name));
                }
            }
            _ => {}
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::ast::Program;
    use crate::lexer::Lexer;
    use crate::parser::Parser;
    use crate::semantic::type_checker::TypeChecker;
    use crate::semantic::{IdentifierResolver, make_var_name_generator};
    use anyhow::Result;

    #[test]
    fn check_ok() {
        let code = r#"
        int add(int a, int b) {
            return a + b;
        }
        "#;

        check_code(code).expect("Expected code to type check successfully");
    }

    fn check_code(code: &str) -> Result<Program> {
        let lexer = Lexer::new();
        let parser = Parser::new();
        let tokens = lexer.scan_tokens(code)?;
        let mut program = parser.parse(tokens)?;

        let var_name_generator = make_var_name_generator();
        let mut resolver = IdentifierResolver::new(var_name_generator);
        resolver.resolve(&mut program)?;

        let mut type_checker = TypeChecker::new();
        type_checker.check(&mut program)?;

        Ok(program)
    }
}

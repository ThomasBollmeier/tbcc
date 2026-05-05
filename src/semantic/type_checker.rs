use crate::ast::{Expression, Program, StorageClass};
use crate::semantic::symbol_table;
use crate::semantic::symbol_table::{CType, IdentAttrs, InitialValue, SymbolTableEntry};
use crate::semantic::walker::{WalkerMut, walk};
use anyhow::anyhow;

pub struct TypeChecker {
    function_nesting: usize,
}

impl TypeChecker {
    pub fn new() -> Self {
        Self {
            function_nesting: 0,
        }
    }

    pub fn check(&mut self, program: &mut Program) -> anyhow::Result<()> {
        walk(program, self)
    }

    fn in_file_scope(&self) -> bool {
        self.function_nesting == 0
    }

    fn type_check_var_decl_file_scope(
        &mut self,
        decl: &mut crate::ast::VarDeclaration,
    ) -> anyhow::Result<()> {
        let init_value = match &decl.init_expr {
            Some(init_expr) => match init_expr {
                Expression::IntegerConstant(i) => Some(InitialValue::Initialized(*i)),
                _ => {
                    return Err(anyhow!(
                        "Only integer constants are allowed as initializers for file-scope variables"
                    ));
                }
            },
            None => {
                if let Some(StorageClass::Extern) = decl.storage_class {
                    None
                } else {
                    Some(InitialValue::Tentative)
                }
            }
        };

        let is_global = decl.storage_class != Some(StorageClass::Static);

        match symbol_table::get(&decl.name) {
            Some(entry) => {
                self.type_check_var_decl_file_scope_w_existing(decl, init_value, is_global, entry)
            }
            None => {
                symbol_table::insert(
                    decl.name.clone(),
                    SymbolTableEntry {
                        c_type: CType::Int,
                        attrs: IdentAttrs::Static {
                            init_value,
                            is_global,
                        },
                    },
                );
                Ok(())
            }
        }
    }

    fn type_check_var_decl_file_scope_w_existing(
        &mut self,
        decl: &mut crate::ast::VarDeclaration,
        init_value: Option<InitialValue>,
        is_global: bool,
        entry: SymbolTableEntry,
    ) -> anyhow::Result<()> {
        let mut init_value = init_value;
        let mut is_global = is_global;

        match entry.c_type {
            CType::Function { .. } => {
                return Err(anyhow!(
                    "Name '{}' has already been declared as a function",
                    decl.name
                ));
            }
            _ => {}
        }

        if let Some(IdentAttrs::Static {
            is_global: is_global_other,
            init_value: init_value_other,
        }) = Some(&entry.attrs)
        {
            if let Some(StorageClass::Extern) = decl.storage_class {
                is_global = *is_global_other;
            } else if is_global != *is_global_other {
                return Err(anyhow!("Conflicting variable linkage"));
            }

            match init_value_other {
                Some(InitialValue::Initialized(_)) => match init_value {
                    Some(InitialValue::Initialized(_)) => {
                        return Err(anyhow!(
                            "Conflicting initializers for variable '{}'",
                            decl.name
                        ));
                    }
                    _ => {
                        init_value = init_value_other.clone();
                    }
                },
                Some(InitialValue::Tentative) => match init_value {
                    Some(InitialValue::Initialized(_)) => {}
                    _ => {
                        init_value = Some(InitialValue::Tentative);
                    }
                },
                _ => {}
            }

            symbol_table::with_global_symbol_table_mut(|table| {
                table.modify(&decl.name).and_modify(|entry| {
                    entry.attrs = IdentAttrs::Static {
                        init_value,
                        is_global,
                    };
                });
            });

            Ok(())
        } else {
            Err(anyhow!(
                "Conflicting variable definitions for '{}'",
                decl.name
            ))
        }
    }

    fn type_check_var_decl_block_scope(
        &self,
        decl: &mut crate::ast::VarDeclaration,
    ) -> anyhow::Result<()> {
        match decl.storage_class {
            Some(StorageClass::Extern) => self.type_check_var_decl_block_scope_extern(decl),
            Some(StorageClass::Static) => self.type_check_var_decl_block_scope_static(decl),
            None => self.type_check_var_decl_block_scope_no_storage_class(decl),
        }
    }

    fn type_check_var_decl_block_scope_extern(
        &self,
        decl: &mut crate::ast::VarDeclaration,
    ) -> anyhow::Result<()> {
        if decl.init_expr.is_some() {
            return Err(anyhow!(
                "Block-scope extern variable '{}' cannot have an initializer",
                decl.name
            ));
        }

        match symbol_table::get(&decl.name) {
            Some(entry) => match entry.c_type {
                CType::Int { .. } => {}
                _ => {
                    return Err(anyhow!(
                        "Variable '{}' has already been declared as a non integer",
                        decl.name
                    ));
                }
            },
            None => {
                symbol_table::insert(
                    decl.name.clone(),
                    SymbolTableEntry {
                        c_type: CType::Int,
                        attrs: IdentAttrs::Static {
                            init_value: None,
                            is_global: true,
                        },
                    },
                );
            }
        }

        Ok(())
    }

    fn type_check_var_decl_block_scope_static(
        &self,
        decl: &mut crate::ast::VarDeclaration,
    ) -> anyhow::Result<()> {
        let init_value = match &decl.init_expr {
            Some(Expression::IntegerConstant(i)) => Some(InitialValue::Initialized(*i)),
            Some(_) => return Err(anyhow!("Non-constant initializer for local static declaration of {}", decl.name)),
            None => Some(InitialValue::Initialized(0)),
        };

        symbol_table::insert(
            decl.name.clone(),
            SymbolTableEntry {
                c_type: CType::Int,
                attrs: IdentAttrs::Static {
                    init_value,
                    is_global: false,
                },
            },
        );

        Ok(())
    }

    fn type_check_var_decl_block_scope_no_storage_class(
        &self,
        decl: &mut crate::ast::VarDeclaration,
    ) -> anyhow::Result<()> {
        if let Some(_) = symbol_table::get(&decl.name) {
            return Err(anyhow!("Name '{}' has already been defined", decl.name));
        }

        symbol_table::insert(
            decl.name.clone(),
            SymbolTableEntry {
                c_type: CType::Int,
                attrs: IdentAttrs::Local,
            },
        );

        Ok(())
    }
}

impl WalkerMut for TypeChecker {
    fn enter_func_decl(
        &mut self,
        func_decl: &mut crate::ast::FunctionDeclaration,
    ) -> anyhow::Result<()> {
        self.function_nesting += 1;

        let is_func_defined = func_decl.body.is_some();
        let is_global = func_decl.storage_class != Some(StorageClass::Static);
        let num_params = func_decl.parameters.len();

        match symbol_table::get(&func_decl.name) {
            Some(SymbolTableEntry {
                c_type,
                attrs:
                    IdentAttrs::Function {
                        is_defined: is_func_defined_other,
                        is_global: is_global_other,
                    },
            }) => {
                if is_global_other && func_decl.storage_class == Some(StorageClass::Static) {
                    return Err(anyhow!(
                        "Static function declaration for '{}' conflicts with previous global declaration",
                        func_decl.name
                    ));
                }
                match c_type {
                    CType::Function {
                        num_params: num_params_other,
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
                                    if let CType::Function { num_params } = &mut entry.c_type {
                                        *entry = SymbolTableEntry {
                                            c_type: CType::Function {
                                                num_params: *num_params,
                                            },
                                            attrs: IdentAttrs::Function {
                                                is_defined: true,
                                                is_global: is_global_other,
                                            },
                                        };
                                    }
                                });
                            });

                            for param in &func_decl.parameters {
                                symbol_table::insert(
                                    param.clone(),
                                    SymbolTableEntry {
                                        c_type: CType::Int,
                                        attrs: IdentAttrs::Local,
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
                        c_type: CType::Function { num_params },
                        attrs: IdentAttrs::Function {
                            is_defined: is_func_defined,
                            is_global,
                        },
                    },
                );

                if is_func_defined {
                    for param in &func_decl.parameters {
                        symbol_table::insert(
                            param.clone(),
                            SymbolTableEntry {
                                c_type: CType::Int,
                                attrs: IdentAttrs::Local,
                            },
                        );
                    }
                }
            }
            _ => {
                return Err(anyhow!(
                    "Name '{}' has been defined inconsistently",
                    func_decl.name
                ));
            }
        }

        Ok(())
    }

    fn leave_func_decl(&mut self, _: &mut crate::ast::FunctionDeclaration) -> anyhow::Result<()> {
        self.function_nesting -= 1;
        Ok(())
    }

    fn enter_declaration(&mut self, decl: &mut crate::ast::VarDeclaration) -> anyhow::Result<()> {
        if self.in_file_scope() {
            self.type_check_var_decl_file_scope(decl)
        } else {
            self.type_check_var_decl_block_scope(decl)
        }
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
                        CType::Function { num_params } => {
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

    #[test]
    fn check_extern_block_scope_ok() {
        let code = r#"
        int main(void) {
            int outer = 1;
            int foo = 0;
            if (outer) {
                /* You can declare a variable with linkage
                * multiple times in the same block;
                * these both refer to the 'foo' variable defined below
                */
                extern int foo;
                extern int foo;
                return foo;
            }
            return 0;
        }

        int foo = 3;
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

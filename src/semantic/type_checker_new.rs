use crate::ast::{
    FunctionDeclaration, Program, Statement, StorageClass, Type, TypedExpression, VarDeclaration,
};
use crate::semantic::symbol_table;
use crate::semantic::symbol_table::{IdentAttrs, SymbolTableEntry};
use crate::semantic::visitor::VisitorMut;
use anyhow::{Result, anyhow};

pub struct TypeChecker {
    function_nesting: usize,
}

impl TypeChecker {
    pub fn new() -> Self {
        Self {
            function_nesting: 0,
        }
    }

    pub fn check(&mut self, program: &mut Program) -> Result<()> {
        program.accept_mut(self)
    }

    fn handle_new_func_decl(&mut self, func_decl: &mut FunctionDeclaration) -> Result<()> {
        let is_func_defined = func_decl.body.is_some();
        let is_global = func_decl.storage_class != Some(StorageClass::Static);

        symbol_table::insert(
            func_decl.name.clone(),
            SymbolTableEntry {
                c_type: func_decl.func_type.clone(),
                attrs: IdentAttrs::Function {
                    is_defined: is_func_defined,
                    is_global,
                },
            },
        );

        if let Some(body) = func_decl.body.as_mut() {
            let param_types = match func_decl.func_type {
                Type::Function {
                    ref param_types, ..
                } => param_types,
                _ => {
                    return Err(anyhow!(
                        "Function '{}' has an invalid type declaration",
                        func_decl.name
                    ));
                }
            };

            let params_and_types = func_decl.parameters.iter().zip(param_types);

            for (param, param_type) in params_and_types {
                symbol_table::insert(
                    param.clone(),
                    SymbolTableEntry {
                        c_type: param_type.clone(),
                        attrs: IdentAttrs::Local,
                    },
                );
            }

            body.accept_mut(self)?;
        }

        Ok(())
    }

    fn check_func_decl_w_existing_func_decl(
        &mut self,
        func_decl: &mut FunctionDeclaration,
        param_types_other: &[Type],
        return_type_other: &Type,
        is_func_defined_other: bool,
        is_global_other: bool,
    ) -> Result<()> {
        let is_func_defined = func_decl.body.is_some();

        if is_func_defined && is_func_defined_other {
            return Err(anyhow!(
                "Function '{}' has already been defined",
                func_decl.name
            ));
        }

        if is_global_other && func_decl.storage_class == Some(StorageClass::Static) {
            return Err(anyhow!(
                "Static function declaration for '{}' conflicts with previous global declaration",
                func_decl.name
            ));
        }

        Self::check_function_types(func_decl, param_types_other, return_type_other)?;

        if let Some(body) = &mut func_decl.body {
            // Update the symbol table entry to mark the function as defined
            symbol_table::with_global_symbol_table_mut(|table| {
                table.modify(&func_decl.name).and_modify(|entry| {
                    if let Type::Function { .. } = &mut entry.c_type {
                        entry.attrs = IdentAttrs::Function {
                            is_defined: true,
                            is_global: is_global_other,
                        };
                    }
                });
            });

            let params_and_types = func_decl.parameters.iter().zip(param_types_other.iter());

            for (param, param_type) in params_and_types {
                symbol_table::insert(
                    param.clone(),
                    SymbolTableEntry {
                        c_type: param_type.clone(),
                        attrs: IdentAttrs::Local,
                    },
                );
            }

            body.accept_mut(self)?;
        }

        Ok(())
    }

    fn check_function_types(
        func_decl: &FunctionDeclaration,
        param_types_other: &[Type],
        return_type_other: &Type,
    ) -> Result<()> {
        let (param_types, return_type) = match func_decl.func_type {
            Type::Function {
                ref param_types,
                ref return_type,
            } => (param_types, return_type),
            _ => {
                return Err(anyhow!(
                    "Function '{}' has an invalid type declaration",
                    func_decl.name
                ));
            }
        };

        if **return_type != *return_type_other {
            return Err(anyhow!(
                "Function '{}' has a return type that conflicts with previous declaration",
                func_decl.name
            ));
        }

        if param_types.len() != param_types_other.len() {
            return Err(anyhow!(
                "Function '{}' has a different number of parameters than previous declaration",
                func_decl.name
            ));
        }

        Ok(())
    }
}

impl VisitorMut for TypeChecker {
    fn visit_program(&mut self, program: &mut Program) -> Result<()> {
        for decl in &mut program.decls {
            decl.accept_mut(self)?;
        }
        Ok(())
    }

    fn visit_function_declaration(&mut self, func_decl: &mut FunctionDeclaration) -> Result<()> {
        self.function_nesting += 1;

        match symbol_table::get(&func_decl.name) {
            Some(SymbolTableEntry {
                c_type:
                    Type::Function {
                        param_types: param_types_other,
                        return_type: return_type_other,
                    },
                attrs:
                    IdentAttrs::Function {
                        is_defined: is_func_defined_other,
                        is_global: is_global_other,
                    },
            }) => {
                self.check_func_decl_w_existing_func_decl(
                    func_decl,
                    &param_types_other,
                    &return_type_other,
                    is_func_defined_other,
                    is_global_other,
                )?;
            }
            None => {
                self.handle_new_func_decl(func_decl)?;
            }
            _ => {
                return Err(anyhow!(
                    "Name '{}' has been defined inconsistently",
                    func_decl.name
                ));
            }
        }

        self.function_nesting -= 1;

        Ok(())
    }

    fn visit_var_declaration(&mut self, var_decl: &mut VarDeclaration) -> Result<()> {
        todo!()
    }

    fn visit_statement(&mut self, stmt: &mut Statement) -> Result<()> {
        todo!()
    }

    fn visit_typed_expression(&mut self, typed_expr: &mut TypedExpression) -> Result<()> {
        todo!()
    }
}

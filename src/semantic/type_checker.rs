use crate::ast::Expression::{Cast, FuncCall};
use crate::ast::{
    Expression, FunctionDeclaration, Program, Statement, StorageClass, Type, TypedExpression,
    UnaryOp, VarDeclaration,
};
use crate::semantic::symbol_table;
use crate::semantic::symbol_table::{IdentAttrs, InitValue, InitialValue, SymbolTableEntry};
use crate::semantic::visitor::VisitorMut;
use anyhow::{Result, anyhow};

pub struct TypeChecker {
    current_function: Option<FunctionDeclaration>,
}

impl TypeChecker {
    pub fn new() -> Self {
        Self {
            current_function: None,
        }
    }

    pub fn check(&mut self, program: &mut Program) -> Result<()> {
        program.accept_mut(self)
    }

    fn in_file_scope(&self) -> bool {
        self.current_function.is_none()
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

    fn check_var_decl_file_scope(&mut self, decl: &mut VarDeclaration) -> Result<()> {
        let init_value = match &decl.init_expr {
            Some(TypedExpression(init_expr, _)) => match init_expr {
                Expression::IntegerConstant(i) => {
                    Some(InitialValue::Initialized(InitValue::Int(*i)))
                }
                Expression::LongConstant(l) => Some(InitialValue::Initialized(InitValue::Long(*l))),
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
                self.check_var_decl_file_scope_w_existing(decl, init_value, is_global, entry)
            }
            None => {
                symbol_table::insert(
                    decl.name.clone(),
                    SymbolTableEntry {
                        c_type: decl.var_type.clone(),
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

    fn check_var_decl_file_scope_w_existing(
        &mut self,
        decl: &mut VarDeclaration,
        init_value: Option<InitialValue>,
        is_global: bool,
        entry: SymbolTableEntry,
    ) -> Result<()> {
        let mut init_value = init_value;
        let mut is_global = is_global;

        match entry.c_type {
            Type::Function { .. } => {
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

    fn check_var_decl_block_scope(&self, decl: &mut VarDeclaration) -> Result<()> {
        match decl.storage_class {
            Some(StorageClass::Extern) => self.check_var_decl_block_scope_extern(decl),
            Some(StorageClass::Static) => self.check_var_decl_block_scope_static(decl),
            None => self.check_var_decl_block_scope_no_storage_class(decl),
        }
    }

    fn check_var_decl_block_scope_extern(&self, decl: &mut VarDeclaration) -> Result<()> {
        if decl.init_expr.is_some() {
            return Err(anyhow!(
                "Block-scope extern variable '{}' cannot have an initializer",
                decl.name
            ));
        }

        match symbol_table::get(&decl.name) {
            Some(entry) => {
                if entry.c_type != decl.var_type {
                    return Err(anyhow!(
                        "variable {} has already been defined with a different type",
                        decl.name
                    ));
                }
            }
            None => {
                symbol_table::insert(
                    decl.name.clone(),
                    SymbolTableEntry {
                        c_type: decl.var_type.clone(),
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

    fn check_var_decl_block_scope_static(&self, decl: &mut VarDeclaration) -> Result<()> {
        let init_value = match &decl.init_expr {
            Some(TypedExpression(Expression::IntegerConstant(i), _)) => {
                Some(InitialValue::Initialized(InitValue::Int(*i)))
            }
            Some(TypedExpression(Expression::LongConstant(l), _)) => {
                Some(InitialValue::Initialized(InitValue::Long(*l)))
            }
            Some(_) => {
                return Err(anyhow!(
                    "Non-constant initializer for local static declaration of {}",
                    decl.name
                ));
            }
            None => Some(InitialValue::Initialized(InitValue::Int(0))),
        };

        symbol_table::insert(
            decl.name.clone(),
            SymbolTableEntry {
                c_type: decl.var_type.clone(),
                attrs: IdentAttrs::Static {
                    init_value,
                    is_global: false,
                },
            },
        );

        Ok(())
    }

    fn check_var_decl_block_scope_no_storage_class(&self, decl: &mut VarDeclaration) -> Result<()> {
        if let Some(_) = symbol_table::get(&decl.name) {
            return Err(anyhow!("Name '{}' has already been defined", decl.name));
        }

        symbol_table::insert(
            decl.name.clone(),
            SymbolTableEntry {
                c_type: decl.var_type.clone(),
                attrs: IdentAttrs::Local,
            },
        );

        Ok(())
    }

    fn convert_to(typed_expr: &TypedExpression, target_type: &Type) -> TypedExpression {
        if typed_expr.1 == *target_type {
            return typed_expr.clone();
        }

        TypedExpression::with_type(
            Cast {
                expr: Box::new(typed_expr.clone()),
                target_type: target_type.clone(),
            },
            target_type.clone(),
        )
    }

    fn set_type(&mut self, typed_expr: &TypedExpression) -> Result<TypedExpression> {
        use Expression::*;

        match &typed_expr.0 {
            IntegerConstant(_) => self.set_type_integer_constant(typed_expr),
            LongConstant(_) => self.set_type_long_constant(typed_expr),
            Cast { expr, target_type } => self.set_type_cast(expr, target_type),
            Var(name) => self.set_type_var(name, typed_expr),
            FuncCall { name, args } => self.set_type_function_call(name, args),
            UnaryExpr(unary_op, operand) => self.set_type_unary(unary_op, operand),
            _ => Ok(typed_expr.clone()),
        }
    }

    fn set_type_unary(
        &mut self,
        unary_op: &UnaryOp,
        operand: &TypedExpression,
    ) -> Result<TypedExpression> {
        let operand = self.set_type(operand)?;
        match unary_op {
            UnaryOp::Not => Ok(TypedExpression::with_type(
                Expression::UnaryExpr(UnaryOp::Not, Box::new(operand)),
                Type::Int,
            )),
            _ => Ok(TypedExpression::with_type(
                Expression::UnaryExpr(UnaryOp::Not, Box::new(operand.clone())),
                operand.get_type(),
            )),
        }
    }

    fn set_type_function_call(
        &self,
        name: &str,
        args: &[TypedExpression],
    ) -> Result<TypedExpression> {
        let entry = symbol_table::get(name);
        if entry.is_none() {
            return Err(anyhow!("function '{}' has not been defined", name));
        }
        let entry = entry.unwrap();

        if let Type::Function {
            param_types,
            return_type,
        } = entry.c_type
        {
            if param_types.len() != args.len() {
                return Err(anyhow!(
                    "Function '{}' expects {} arguments, but {} were provided",
                    name,
                    param_types.len(),
                    args.len()
                ));
            }

            let mut converted_args = Vec::new();

            for (arg, param_type) in args.iter().zip(param_types.iter()) {
                converted_args.push(Self::convert_to(arg, param_type));
            }

            Ok(TypedExpression::with_type(
                FuncCall {
                    name: name.to_string(),
                    args: converted_args,
                },
                *return_type.clone(),
            ))
        } else {
            Err(anyhow!("Identifier '{}' is not a function", name))
        }
    }

    fn set_type_var(&self, name: &str, typed_expr: &TypedExpression) -> Result<TypedExpression> {
        match symbol_table::get(name) {
            Some(entry) => match entry.c_type {
                Type::Function { .. } => Err(anyhow!(
                    "Identifier '{}' is a function, but used as a variable",
                    name
                )),
                Type::Undefined => Err(anyhow!("variable '{}' has an undefined type", name)),
                _ => {
                    let mut result = typed_expr.clone();
                    result.set_type(entry.c_type.clone());
                    Ok(result)
                }
            },
            None => Err(anyhow!("Undefined variable '{}'", name)),
        }
    }

    fn set_type_integer_constant(&self, typed_expr: &TypedExpression) -> Result<TypedExpression> {
        let mut result = typed_expr.clone();
        result.set_type(Type::Int);
        Ok(result)
    }

    fn set_type_long_constant(&self, typed_expr: &TypedExpression) -> Result<TypedExpression> {
        let mut result = typed_expr.clone();
        result.set_type(Type::Long);
        Ok(result)
    }

    fn set_type_cast(
        &mut self,
        expr: &TypedExpression,
        target_type: &Type,
    ) -> Result<TypedExpression> {
        use Expression::Cast;
        let new_expr = self.set_type(expr)?;
        Ok(TypedExpression::with_type(
            Cast {
                expr: Box::new(new_expr),
                target_type: target_type.clone(),
            },
            target_type.clone(),
        ))
    }

    fn get_return_type_of_current_function(&self) -> Result<Type> {
        match &self.current_function {
            Some(func_decl) => match &func_decl.func_type {
                Type::Function { return_type, .. } => Ok(*return_type.clone()),
                _ => Err(anyhow!("Return type is not a function")),
            },
            None => Err(anyhow!("return statement outside of function")),
        }
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
        if func_decl.body.is_some() {
            self.current_function = Some(func_decl.clone());
        }

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

        if func_decl.body.is_some() {
            self.current_function = None;
        }

        Ok(())
    }

    fn visit_var_declaration(&mut self, var_decl: &mut VarDeclaration) -> Result<()> {
        if self.in_file_scope() {
            self.check_var_decl_file_scope(var_decl)
        } else {
            self.check_var_decl_block_scope(var_decl)
        }
    }

    fn visit_statement(&mut self, stmt: &mut Statement) -> Result<()> {
        use Statement::*;
        match stmt {
            Return(typed_expr) => {
                let ret_type = self.get_return_type_of_current_function()?;
                *typed_expr = Self::convert_to(&self.set_type(typed_expr)?, &ret_type);
            }
            Expression(typed_expr) => {
                *typed_expr = self.set_type(typed_expr)?;
            }
            Null => {}
            Break { .. } => {}
            Continue { .. } => {}
            While {
                condition, body, ..
            } => {
                *condition = self.set_type(condition)?;
                body.accept_mut(self)?;
            }
            DoWhile {
                condition, body, ..
            } => {
                body.accept_mut(self)?;
                *condition = self.set_type(condition)?;
            }
            For {
                init: _init,
                condition,
                post,
                body,
                ..
            } => {
                if let Some(condition) = condition {
                    *condition = self.set_type(condition)?;
                }
                if let Some(post) = post {
                    *post = self.set_type(post)?;
                }
                body.accept_mut(self)?;
            }
            CompoundStatement(block) => {
                block.accept_mut(self)?;
            }
            IfStatement {
                condition,
                then_branch,
                else_branch,
            } => {
                *condition = self.set_type(condition)?;
                then_branch.accept_mut(self)?;
                if let Some(else_branch) = else_branch {
                    else_branch.accept_mut(self)?;
                }
            }
            SwitchStatement {
                condition,
                body,
                arms,
                ..
            } => {
                *condition = self.set_type(condition)?;
                body.accept_mut(self)?;
                for (_, typed_expr) in arms {
                    if let Some(typed_expr) = typed_expr {
                        *typed_expr = self.set_type(typed_expr)?;
                    }
                }
            }
            GotoStatement(..) => {}
            LabeledStatement { statement, .. } => {
                statement.accept_mut(self)?;
            }
        }
        Ok(())
    }

    fn visit_typed_expression(&mut self, _typed_expr: &mut TypedExpression) -> Result<()> {
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

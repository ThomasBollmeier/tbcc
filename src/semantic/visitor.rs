use anyhow::Result;
use crate::ast::{FunctionDeclaration, Program, Statement, TypedExpression, VarDeclaration};
// ...existing code...

#[allow(unused_variables)]
pub trait VisitorMut {
    fn visit_program(&mut self, program: &mut Program) -> Result<()>;
    fn visit_function_declaration(&mut self, func_decl: &mut FunctionDeclaration) -> Result<()>;
    fn visit_var_declaration(&mut self, var_decl: &mut VarDeclaration) -> Result<()>;
    fn visit_statement(&mut self, stmt: &mut Statement) -> Result<()>;
    fn visit_typed_expression(&mut self, typed_expr: &mut TypedExpression) -> Result<()>;

}
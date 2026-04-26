use crate::ast::{Block, Declaration, Expression, ForInit, FunctionDeclaration, Program, Statement};
use anyhow::Result;

#[allow(unused_variables)]
pub trait WalkerMut {
    fn enter_program(&mut self, program: &mut Program) -> Result<()> {
        Ok(())
    }
    fn leave_program(&mut self, program: &mut Program) -> Result<()> {
        Ok(())
    }

    fn enter_func_decl(&mut self, func_decl: &mut FunctionDeclaration) -> Result<()> {
        Ok(())
    }
    fn leave_func_decl(&mut self, func_decl: &mut FunctionDeclaration) -> Result<()> {
        Ok(())
    }

    fn enter_block(&mut self, block: &mut Block) -> Result<()> {
        Ok(())
    }
    fn leave_block(&mut self, block: &mut Block) -> Result<()> {
        Ok(())
    }

    fn enter_declaration(&mut self, decl: &mut Declaration) -> Result<()> {
        Ok(())
    }
    fn leave_declaration(&mut self, decl: &mut Declaration) -> Result<()> {
        Ok(())
    }

    fn enter_statement(&mut self, stmt: &mut Statement) -> Result<()> {
        Ok(())
    }
    fn leave_statement(&mut self, stmt: &mut Statement) -> Result<()> {
        Ok(())
    }

    fn enter_expression(&mut self, expr: &mut Expression) -> Result<()> {
        Ok(())
    }
    fn leave_expression(&mut self, expr: &mut Expression) -> Result<()> {
        Ok(())
    }
}

pub fn walk(program: &mut Program, walker: &mut impl WalkerMut) -> Result<()> {
    walker.enter_program(program)?;
    for function_decl in &mut program.function_decls {
        walk_function_decl(function_decl, walker)?;
    }
    walker.leave_program(program)?;
    Ok(())
}

fn walk_function_decl(func_decl: &mut FunctionDeclaration, walker: &mut impl WalkerMut) -> Result<()> {
    walker.enter_func_decl(func_decl)?;
    if let Some(body) = &mut func_decl.body {
        walk_block(body, walker)?;
    }
    walker.leave_func_decl(func_decl)?;
    Ok(())
}

fn walk_block(block: &mut Block, walker: &mut impl WalkerMut) -> Result<()> {
    walker.enter_block(block)?;
    for item in &mut block.items {
        match item {
            crate::ast::BlockItem::FunctionDeclaration(func_decl) => {
                walk_function_decl(func_decl, walker)?;
            }
            crate::ast::BlockItem::Declaration(decl) => {
                walk_declaration(decl, walker)?;
            }
            crate::ast::BlockItem::Statement(stmt) => {
                walk_statement(stmt, walker)?;
            }
        }
    }
    walker.leave_block(block)?;
    Ok(())
}

fn walk_declaration(decl: &mut Declaration, walker: &mut impl WalkerMut) -> Result<()> {
    walker.enter_declaration(decl)?;
    if let Some(init_expr) = &mut decl.init_expr {
        walk_expression(init_expr, walker)?;
    }
    walker.leave_declaration(decl)?;
    Ok(())
}

fn walk_statement(stmt: &mut Statement, walker: &mut impl WalkerMut) -> Result<()> {
    use Statement::*;

    walker.enter_statement(stmt)?;

    match stmt {
        Return(expr) => {
            walk_expression(expr, walker)?;
        }
        Expression(expr) => {
            walk_expression(expr, walker)?;
        }
        Null => {}
        Break { loop_id: _ } => {}
        Continue { loop_id: _ } => {}
        While {
            loop_id: _,
            condition,
            body,
        } => {
            walk_expression(condition, walker)?;
            walk_statement(body, walker)?;
        }
        DoWhile {
            loop_id: _,
            condition,
            body,
        } => {
            walk_statement(body, walker)?;
            walk_expression(condition, walker)?;
        }
        For {
            loop_id: _,
            init,
            condition,
            post,
            body,
        } => {
            walk_for_init(init, walker)?;
            if let Some(condition) = condition {
                walk_expression(condition, walker)?;
            }
            if let Some(post) = post {
                walk_expression(post, walker)?;
            }
            walk_statement(body, walker)?;
        }
        CompoundStatement(block) => {
            walk_block(block, walker)?;
        }
        IfStatement {
            condition,
            then_branch,
            else_branch,
        } => {
            walk_expression(condition, walker)?;
            walk_statement(then_branch, walker)?;
            if let Some(else_branch) = else_branch {
                walk_statement(else_branch, walker)?;
            }
        }
        SwitchStatement {
            condition,
            body,
            ..
        } => {
            walk_expression(condition, walker)?;
            walk_statement(body, walker)?;
        }
        GotoStatement(_) => {}
        LabeledStatement { statement, .. } => {
            walk_statement(statement, walker)?;
        }
    }

    walker.leave_statement(stmt)?;

    Ok(())
}

fn walk_expression(expr: &mut Expression, walker: &mut impl WalkerMut) -> Result<()> {
    use Expression::*;
    walker.enter_expression(expr)?;

    match expr {
        IntegerConstant(_) => {}
        Var(_) => {}
        FuncCall {args, ..} => {
            for arg in args {
                walk_expression(arg, walker)?;
            }
        }
        UnaryExpr(_, expr) => {
            walk_expression(expr, walker)?;
        }
        BinaryExpr(_, left, right) => {
            walk_expression(left, walker)?;
            walk_expression(right, walker)?;
        }
        Assignment {
            left,
            right,
            is_postfix: _,
        } => {
            walk_expression(left, walker)?;
            walk_expression(right, walker)?;
        }
        ConditionalExpr {
            condition,
            then_expr,
            else_expr,
        } => {
            walk_expression(condition, walker)?;
            walk_expression(then_expr, walker)?;
            walk_expression(else_expr, walker)?;
        }
    }

    walker.leave_expression(expr)?;

    Ok(())
}

fn walk_for_init(init: &mut ForInit, walker: &mut impl WalkerMut) -> Result<()> {
    match init {
        ForInit::InitDeclaration(decl) => {
            walk_declaration(decl, walker)?;
        }
        ForInit::InitExpression(expr) => {
            if let Some(expr) = expr {
                walk_expression(expr, walker)?;
            }
        }
    }

    Ok(())
}

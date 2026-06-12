use crate::ast::{
    Block, Declaration, Expression, ForInit, FunctionDeclaration, Program, Statement,
    TypedExpression, VarDeclaration,
};
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

    fn enter_declaration(&mut self, decl: &mut VarDeclaration) -> Result<()> {
        Ok(())
    }
    fn leave_declaration(&mut self, decl: &mut VarDeclaration) -> Result<()> {
        Ok(())
    }

    fn enter_statement(&mut self, stmt: &mut Statement) -> Result<()> {
        Ok(())
    }
    fn leave_statement(&mut self, stmt: &mut Statement) -> Result<()> {
        Ok(())
    }

    fn enter_typed_expression(&mut self, expr: &mut TypedExpression) -> Result<()> {
        Ok(())
    }

    fn leave_typed_expression(&mut self, expr: &mut TypedExpression) -> Result<()> {
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
    for decl in &mut program.decls {
        match decl {
            Declaration::FunctionDecl(func_decl) => {
                walk_function_decl(func_decl, walker)?;
            }
            Declaration::VarDecl(var_decl) => {
                walk_declaration(var_decl, walker)?;
            }
        }
    }
    walker.leave_program(program)?;
    Ok(())
}

fn walk_function_decl(
    func_decl: &mut FunctionDeclaration,
    walker: &mut impl WalkerMut,
) -> Result<()> {
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
            crate::ast::BlockItem::VarDeclaration(decl) => {
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

fn walk_declaration(decl: &mut VarDeclaration, walker: &mut impl WalkerMut) -> Result<()> {
    walker.enter_declaration(decl)?;
    if let Some(init_expr) = &mut decl.init_expr {
        walk_typed_expression(init_expr, walker)?;
    }
    walker.leave_declaration(decl)?;
    Ok(())
}

fn walk_statement(stmt: &mut Statement, walker: &mut impl WalkerMut) -> Result<()> {
    use Statement::*;

    walker.enter_statement(stmt)?;

    match stmt {
        Return(expr) => {
            walk_typed_expression(expr, walker)?;
        }
        Expression(expr) => {
            walk_typed_expression(expr, walker)?;
        }
        Null => {}
        Break { loop_id: _ } => {}
        Continue { loop_id: _ } => {}
        While {
            loop_id: _,
            condition,
            body,
        } => {
            walk_typed_expression(condition, walker)?;
            walk_statement(body, walker)?;
        }
        DoWhile {
            loop_id: _,
            condition,
            body,
        } => {
            walk_statement(body, walker)?;
            walk_typed_expression(condition, walker)?;
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
                walk_typed_expression(condition, walker)?;
            }
            if let Some(post) = post {
                walk_typed_expression(post, walker)?;
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
            walk_typed_expression(condition, walker)?;
            walk_statement(then_branch, walker)?;
            if let Some(else_branch) = else_branch {
                walk_statement(else_branch, walker)?;
            }
        }
        SwitchStatement {
            condition, body, ..
        } => {
            walk_typed_expression(condition, walker)?;
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

fn walk_typed_expression(expr: &mut TypedExpression, walker: &mut impl WalkerMut) -> Result<()> {
    walker.enter_typed_expression(expr)?;

    walk_expression(&mut expr.0, walker)?;

    walker.leave_typed_expression(expr)?;

    Ok(())
}

fn walk_expression(expr: &mut Expression, walker: &mut impl WalkerMut) -> Result<()> {
    use Expression::*;

    walker.enter_expression(expr)?;

    match expr {
        IntegerConstant(_) => {}
        UnsignedIntegerConstant(_) => {}
        LongConstant(_) => {}
        UnsignedLongConstant(_) => {}
        Cast {
            target_type: _,
            expr,
        } => {
            walk_typed_expression(expr, walker)?;
        }
        Var(_) => {}
        FuncCall { args, .. } => {
            for arg in args {
                walk_typed_expression(arg, walker)?;
            }
        }
        UnaryExpr(_, expr) => {
            walk_typed_expression(expr, walker)?;
        }
        BinaryExpr(_, left, right) => {
            walk_typed_expression(left, walker)?;
            walk_typed_expression(right, walker)?;
        }
        Assignment {
            left,
            right,
            is_postfix: _,
        } => {
            walk_typed_expression(left, walker)?;
            walk_typed_expression(right, walker)?;
        }
        ConditionalExpr {
            condition,
            then_expr,
            else_expr,
        } => {
            walk_typed_expression(condition, walker)?;
            walk_typed_expression(then_expr, walker)?;
            walk_typed_expression(else_expr, walker)?;
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
                walk_typed_expression(expr, walker)?;
            }
        }
    }

    Ok(())
}

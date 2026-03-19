#[derive(Debug, Clone)]
pub struct Program {
    pub function_definition: FunctionDefinition,
}

impl Program {
    pub fn new(function_definition: FunctionDefinition) -> Self {
        Program { function_definition }
    }

    pub fn accept<R>(&self, visitor: &mut impl Visitor<R>) -> R {
        visitor.visit_program(self)
    }
}

#[derive(Debug, Clone)]
pub struct FunctionDefinition {
    pub name: String,
    pub body: Statement,
}

impl FunctionDefinition {

    pub fn new(name: String, body: Statement) -> Self {
        FunctionDefinition { name, body }
    }

    pub fn accept<R>(&self, visitor: &mut impl Visitor<R>) -> R {
        visitor.visit_function_definition(self)
    }
}

#[derive(Debug, Clone)]
pub enum Statement {
    Return(Expression),
}

impl Statement {
    pub fn accept<R>(&self, visitor: &mut impl Visitor<R>) -> R {
        visitor.visit_statement(self)
    }
}

#[derive(Debug, Clone)]
pub enum Expression {
    IntegerConstant(i64),
}

impl Expression {
    pub fn accept<R>(&self, visitor: &mut impl Visitor<R>) -> R {
        visitor.visit_expression(self)
    }
}

pub trait Visitor<A> {
    fn visit_program(&mut self, program: &Program) -> A;
    fn visit_function_definition(&mut self, func_def: &FunctionDefinition) -> A;
    fn visit_statement(&mut self, stmt: &Statement) -> A;
    fn visit_expression(&mut self, expr: &Expression) -> A;
}
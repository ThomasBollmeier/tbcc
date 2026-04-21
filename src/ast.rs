#[derive(Debug, Clone)]
pub struct Program {
    pub function_definition: FunctionDefinition,
}

impl Program {
    pub fn new(function_definition: FunctionDefinition) -> Self {
        Program {
            function_definition,
        }
    }

    pub fn accept<R>(&self, visitor: &mut impl Visitor<R>) -> R {
        visitor.visit_program(self)
    }
    pub fn accept_mut<R>(&mut self, visitor: &mut impl VisitorMut<R>) -> R {
        visitor.visit_program(self)
    }
}

#[derive(Debug, Clone)]
pub struct FunctionDefinition {
    pub name: String,
    pub body: Block,
}

impl FunctionDefinition {
    pub fn new(name: String, body: Block) -> Self {
        FunctionDefinition { name, body }
    }

    pub fn accept<R>(&self, visitor: &mut impl Visitor<R>) -> R {
        visitor.visit_function_definition(self)
    }

    pub fn accept_mut<R>(&mut self, visitor: &mut impl VisitorMut<R>) -> R {
        visitor.visit_function_definition(self)
    }
}

#[derive(Debug, Clone)]
pub struct Block {
    pub items: Vec<BlockItem>,
}

impl Block {
    pub fn new(items: Vec<BlockItem>) -> Self {
        Block { items }
    }
    pub fn accept<R>(&self, visitor: &mut impl Visitor<R>) -> R {
        visitor.visit_block(self)
    }

    pub fn accept_mut<R>(&mut self, visitor: &mut impl VisitorMut<R>) -> R {
        visitor.visit_block(self)
    }
}

#[derive(Debug, Clone)]
pub enum BlockItem {
    Declaration(Declaration),
    Statement(Statement),
}

impl BlockItem {
    pub fn accept<R>(&self, visitor: &mut impl Visitor<R>) -> R {
        match self {
            BlockItem::Declaration(d) => d.accept(visitor),
            BlockItem::Statement(s) => s.accept(visitor),
        }
    }

    pub fn accept_mut<R>(&mut self, visitor: &mut impl VisitorMut<R>) -> R {
        match self {
            BlockItem::Declaration(d) => d.accept_mut(visitor),
            BlockItem::Statement(s) => s.accept_mut(visitor),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Declaration {
    pub name: String,
    pub init_expr: Option<Expression>,
}

impl Declaration {
    pub fn new(name: String, init_expr: Option<Expression>) -> Self {
        Declaration { name, init_expr }
    }

    pub fn accept<R>(&self, visitor: &mut impl Visitor<R>) -> R {
        visitor.visit_declaration(self)
    }

    pub fn accept_mut<R>(&mut self, visitor: &mut impl VisitorMut<R>) -> R {
        visitor.visit_declaration(self)
    }
}

#[derive(Debug, Clone)]
pub enum Statement {
    Return(Expression),
    Expression(Expression),
    Null,
    Break {
        loop_id: String,
    },
    Continue {
        loop_id: String,
    },
    While {
        loop_id: String,
        condition: Expression,
        body: Box<Statement>,
    },
    DoWhile {
        loop_id: String,
        condition: Expression,
        body: Box<Statement>,
    },
    For {
        loop_id: String,
        init: ForInit,
        condition: Option<Expression>,
        post: Option<Expression>,
        body: Box<Statement>,
    },
    CompoundStatement(Block),
    IfStatement {
        condition: Expression,
        then_branch: Box<Statement>,
        else_branch: Option<Box<Statement>>,
    },
    GotoStatement(String),
    LabeledStatement {
        label: String,
        statement: Box<Statement>,
    },
}

impl Statement {
    pub fn accept<R>(&self, visitor: &mut impl Visitor<R>) -> R {
        visitor.visit_statement(self)
    }

    pub fn accept_mut<R>(&mut self, visitor: &mut impl VisitorMut<R>) -> R {
        visitor.visit_statement(self)
    }
}

#[derive(Debug, Clone)]
pub enum ForInit {
    InitDeclaration(Declaration),
    InitExpression(Option<Expression>),
}

#[derive(Debug, Clone)]
pub enum Expression {
    IntegerConstant(i32),
    Var(String),
    UnaryExpr(UnaryOp, Box<Expression>),
    BinaryExpr(BinaryOp, Box<Expression>, Box<Expression>),
    Assignment {
        left: Box<Expression>,
        right: Box<Expression>,
        is_postfix: bool,
    },
    ConditionalExpr {
        condition: Box<Expression>,
        then_expr: Box<Expression>,
        else_expr: Box<Expression>,
    },
}

impl Expression {
    pub fn accept<R>(&self, visitor: &mut impl Visitor<R>) -> R {
        visitor.visit_expression(self)
    }

    pub fn accept_mut<R>(&mut self, visitor: &mut impl VisitorMut<R>) -> R {
        visitor.visit_expression(self)
    }
}

#[derive(Debug, Clone)]
pub enum UnaryOp {
    Negate,
    Complement,
    Not,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
    BitAnd,
    BitOr,
    BitXor,
    ShiftLeft,
    ShiftRight,
    LogicalAnd,
    LogicalOr,
    Equal,
    NotEqual,
    Greater,
    Less,
    GreaterEqual,
    LessEqual,
    Assign,
    AssignAdd,
    AssignSubtract,
    AssignMultiply,
    AssignDivide,
    AssignRemainder,
    AssignBitAnd,
    AssignBitOr,
    AssignBitXor,
    AssignShiftLeft,
    AssignShiftRight,
    Conditional, // ?:
}

#[derive(Debug, Clone)]
pub enum Associativity {
    Left,
    Right,
}

impl From<&BinaryOp> for Associativity {
    fn from(value: &BinaryOp) -> Self {
        use BinaryOp::*;
        match value {
            Assign | AssignAdd | AssignSubtract | AssignMultiply | AssignDivide
            | AssignRemainder | AssignBitAnd | AssignBitOr | AssignBitXor | AssignShiftLeft
            | AssignShiftRight | Conditional => Associativity::Right,
            _ => Associativity::Left,
        }
    }
}

pub trait Visitor<A> {
    fn visit_program(&mut self, program: &Program) -> A;
    fn visit_function_definition(&mut self, func_def: &FunctionDefinition) -> A;
    fn visit_block(&mut self, block: &Block) -> A;
    fn visit_declaration(&mut self, decl: &Declaration) -> A;
    fn visit_statement(&mut self, stmt: &Statement) -> A;
    fn visit_expression(&mut self, expr: &Expression) -> A;
}

pub trait VisitorMut<A> {
    fn visit_program(&mut self, program: &mut Program) -> A;
    fn visit_function_definition(&mut self, func_def: &mut FunctionDefinition) -> A;
    fn visit_block(&mut self, block: &mut Block) -> A;
    fn visit_declaration(&mut self, decl: &mut Declaration) -> A;
    fn visit_statement(&mut self, stmt: &mut Statement) -> A;
    fn visit_expression(&mut self, expr: &mut Expression) -> A;
}

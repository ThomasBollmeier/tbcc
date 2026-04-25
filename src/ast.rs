#[derive(Debug, Clone)]
pub struct Program {
    pub function_decls: Vec<FunctionDeclaration>,
}

impl Program {
    pub fn new() -> Self {
        Program {
            function_decls: vec![],
        }
    }

    pub fn add_function_decl(&mut self, function_decl: FunctionDeclaration) {
        self.function_decls.push(function_decl);
    }
}

#[derive(Debug, Clone)]
pub struct FunctionDeclaration {
    pub name: String,
    pub parameters: Vec<String>,
    pub body: Option<Block>,
}

impl FunctionDeclaration {
    pub fn new(name: String, parameters: Vec<String>, body: Option<Block>) -> Self {
        FunctionDeclaration {
            name,
            parameters,
            body,
        }
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
}

#[derive(Debug, Clone)]
pub enum BlockItem {
    Declaration(Declaration),
    FunctionDeclaration(FunctionDeclaration),
    Statement(Statement),
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
    SwitchStatement {
        switch_id: String,
        condition: Expression,
        body: Box<Statement>,
        arms: Vec<(String, Option<Expression>)>,
    },
    GotoStatement(String),
    LabeledStatement {
        label: Label,
        statement: Box<Statement>,
    },
}

#[derive(Debug, Clone)]
pub enum Label {
    Label(String),
    Case{
        case_id: String,
        value: Expression,
    }, // <-- to be used in switch statement
    Default {
        default_id: String,
    }, // <-- to be used in switch statement
}

impl Label {
    pub fn get_name(&self) -> String {
        match self {
            Label::Label(name) => name.clone(),
            Label::Case { case_id, ..} => case_id.clone(),
            Label::Default { default_id } => default_id.clone(),
        }
    }

    pub fn get_case_value(&self) -> Option<&Expression> {
        match self {
            Label::Case{ value, ..} => Some(value),
            _ => None,
        }
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
    FuncCall{
        name: String,
        args: Vec<Expression>,
    },
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

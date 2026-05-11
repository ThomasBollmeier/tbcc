#[derive(Debug, Clone)]
pub struct Program {
    pub decls: Vec<Declaration>,
}

impl Program {
    pub fn new() -> Self {
        Program { decls: vec![] }
    }
}

#[derive(Debug, Clone)]
pub enum Declaration {
    FunctionDecl(FunctionDeclaration),
    VarDecl(VarDeclaration),
}

#[derive(Debug, Clone)]
pub struct FunctionDeclaration {
    pub name: String,
    pub parameters: Vec<String>,
    pub body: Option<Block>,
    pub storage_class: Option<StorageClass>,
    pub func_type: Type,
}

impl FunctionDeclaration {
    pub fn new(
        name: String,
        parameters: Vec<String>,
        body: Option<Block>,
        storage_class: Option<StorageClass>,
        func_type: Type,
    ) -> Self {
        FunctionDeclaration {
            name,
            parameters,
            body,
            storage_class,
            func_type,
        }
    }
}

#[derive(Debug, Clone)]
pub struct VarDeclaration {
    pub name: String,
    pub init_expr: Option<Expression>,
    pub storage_class: Option<StorageClass>,
    pub var_type: Type,
}

impl VarDeclaration {
    pub fn new(
        name: String,
        init_expr: Option<Expression>,
        storage_class: Option<StorageClass>,
        var_type: Type,
    ) -> Self {
        VarDeclaration {
            name,
            init_expr,
            storage_class,
            var_type,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Int,
    Long,
    Function {
        return_type: Box<Type>,
        param_types: Vec<Type>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum StorageClass {
    Static,
    Extern,
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
    VarDeclaration(VarDeclaration),
    FunctionDeclaration(FunctionDeclaration),
    Statement(Statement),
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
    Case { case_id: String, value: Expression }, // <-- to be used in switch statement
    Default { default_id: String },              // <-- to be used in switch statement
}

impl Label {
    pub fn get_name(&self) -> String {
        match self {
            Label::Label(name) => name.clone(),
            Label::Case { case_id, .. } => case_id.clone(),
            Label::Default { default_id } => default_id.clone(),
        }
    }

    pub fn get_case_value(&self) -> Option<&Expression> {
        match self {
            Label::Case { value, .. } => Some(value),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ForInit {
    InitDeclaration(VarDeclaration),
    InitExpression(Option<Expression>),
}

#[derive(Debug, Clone)]
pub enum Expression {
    IntegerConstant(i32),
    LongConstant(i64),
    Cast {
        expr: Box<Expression>,
        target_type: Type,
    },
    Var(String),
    FuncCall {
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

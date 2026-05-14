use crate::semantic::visitor::VisitorMut;
use anyhow::Result;

#[derive(Debug, Clone)]
pub struct Program {
    pub decls: Vec<Declaration>,
}

impl Program {
    pub fn new() -> Self {
        Program { decls: vec![] }
    }

    pub fn accept_mut(&mut self, visitor: &mut impl VisitorMut) -> Result<()> {
        visitor.visit_program(self)
    }
}

#[derive(Debug, Clone)]
pub enum Declaration {
    FunctionDecl(FunctionDeclaration),
    VarDecl(VarDeclaration),
}

impl Declaration {
    pub fn accept_mut(&mut self, visitor: &mut impl VisitorMut) -> Result<()> {
        match self {
            Declaration::FunctionDecl(func_decl) => func_decl.accept_mut(visitor),
            Declaration::VarDecl(var_decl) => var_decl.accept_mut(visitor),
        }
    }
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

    pub fn accept_mut(&mut self, visitor: &mut impl VisitorMut) -> Result<()> {
        visitor.visit_function_declaration(self)
    }
}

#[derive(Debug, Clone)]
pub struct VarDeclaration {
    pub name: String,
    pub init_expr: Option<TypedExpression>,
    pub storage_class: Option<StorageClass>,
    pub var_type: Type,
}

impl VarDeclaration {
    pub fn accept_mut(&mut self, visitor: &mut impl VisitorMut) -> Result<()> {
        visitor.visit_var_declaration(self)
    }
}

impl VarDeclaration {
    pub fn new(
        name: String,
        init_expr: Option<TypedExpression>,
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
    Undefined,
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

    pub fn accept_mut(&mut self, visitor: &mut impl VisitorMut) -> Result<()> {
        for item in &mut self.items {
            item.accept_mut(visitor)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum BlockItem {
    VarDeclaration(VarDeclaration),
    FunctionDeclaration(FunctionDeclaration),
    Statement(Statement),
}

impl BlockItem {
    pub fn accept_mut(&mut self, visitor: &mut impl VisitorMut) -> Result<()> {
        match self {
            BlockItem::VarDeclaration(var_decl) => var_decl.accept_mut(visitor),
            BlockItem::FunctionDeclaration(function_decl) => function_decl.accept_mut(visitor),
            BlockItem::Statement(stmt) => stmt.accept_mut(visitor),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Statement {
    Return(TypedExpression),
    Expression(TypedExpression),
    Null,
    Break {
        loop_id: String,
    },
    Continue {
        loop_id: String,
    },
    While {
        loop_id: String,
        condition: TypedExpression,
        body: Box<Statement>,
    },
    DoWhile {
        loop_id: String,
        condition: TypedExpression,
        body: Box<Statement>,
    },
    For {
        loop_id: String,
        init: ForInit,
        condition: Option<TypedExpression>,
        post: Option<TypedExpression>,
        body: Box<Statement>,
    },
    CompoundStatement(Block),
    IfStatement {
        condition: TypedExpression,
        then_branch: Box<Statement>,
        else_branch: Option<Box<Statement>>,
    },
    SwitchStatement {
        switch_id: String,
        condition: TypedExpression,
        body: Box<Statement>,
        arms: Vec<(String, Option<TypedExpression>)>,
    },
    GotoStatement(String),
    LabeledStatement {
        label: Label,
        statement: Box<Statement>,
    },
}

impl Statement {
    pub fn accept_mut(&mut self, visitor: &mut impl VisitorMut) -> Result<()> {
        visitor.visit_statement(self)
    }
}

#[derive(Debug, Clone)]
pub enum Label {
    Label(String),
    Case {
        case_id: String,
        value: TypedExpression,
    }, // <-- to be used in switch statement
    Default {
        default_id: String,
    }, // <-- to be used in switch statement
}

impl Label {
    pub fn accept_mut(&mut self, visitor: &mut impl VisitorMut) -> Result<()> {
        match self {
            Label::Case { value, .. } => value.accept_mut(visitor),
            _ => Ok(()),
        }
    }

    pub fn get_name(&self) -> String {
        match self {
            Label::Label(name) => name.clone(),
            Label::Case { case_id, .. } => case_id.clone(),
            Label::Default { default_id } => default_id.clone(),
        }
    }

    pub fn get_case_value(&self) -> Option<&TypedExpression> {
        match self {
            Label::Case { value, .. } => Some(value),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ForInit {
    InitDeclaration(VarDeclaration),
    InitExpression(Option<TypedExpression>),
}

impl ForInit {
    pub fn accept_mut(&mut self, visitor: &mut impl VisitorMut) -> Result<()> {
        match self {
            ForInit::InitDeclaration(var_decl) => var_decl.accept_mut(visitor),
            ForInit::InitExpression(expr_opt) => {
                if let Some(expr) = expr_opt {
                    expr.accept_mut(visitor)
                } else {
                    Ok(())
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct TypedExpression(pub Expression, pub Type);

impl TypedExpression {
    pub fn new(expr: Expression) -> Self {
        TypedExpression(expr, Type::Undefined)
    }

    pub fn with_type(expr: Expression, c_type: Type) -> Self {
        TypedExpression(expr, c_type)
    }

    pub fn accept_mut(&mut self, visitor: &mut impl VisitorMut) -> Result<()> {
        visitor.visit_typed_expression(self)
    }

    pub fn get_type(&self) -> Type {
        self.1.clone()
    }

    pub fn set_type(&mut self, c_type: Type) {
        self.1 = c_type;
    }
}

pub fn typed(expression: Expression) -> TypedExpression {
    TypedExpression::new(expression)
}

#[derive(Debug, Clone)]
pub enum Expression {
    IntegerConstant(i32),
    LongConstant(i64),
    Cast {
        expr: Box<TypedExpression>,
        target_type: Type,
    },
    Var(String),
    FuncCall {
        name: String,
        args: Vec<TypedExpression>,
    },
    UnaryExpr(UnaryOp, Box<TypedExpression>),
    BinaryExpr(BinaryOp, Box<TypedExpression>, Box<TypedExpression>),
    Assignment {
        left: Box<TypedExpression>,
        right: Box<TypedExpression>,
        is_postfix: bool,
    },
    ConditionalExpr {
        condition: Box<TypedExpression>,
        then_expr: Box<TypedExpression>,
        else_expr: Box<TypedExpression>,
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

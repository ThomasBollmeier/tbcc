use crate::common::Type;

#[derive(Debug, Clone, PartialEq)]
pub struct Program (pub Vec<TopLevel>);

#[derive(Debug, Clone, PartialEq)]
pub enum TopLevel {
    Function(Function),
    StaticVariable(StaticVariable),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub name: String,
    pub parameters: Vec<String>,
    pub body: Vec<Instruction>,
    pub is_global: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StaticVariable {
    pub name: String,
    pub is_global: bool,
    pub initial_value: Value,
    pub c_type: Type,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Instruction {
    Return(Value),
    SignExtend {
        src: Value,
        dst: Value,
    },
    ZeroExtend {
        src: Value,
        dst: Value,
    },
    Truncate {
        src: Value,
        dst: Value,
    },
    Unary {
        op: UnaryOperator,
        src: Value,
        dst: Value,
    },
    Binary {
        op: BinaryOperator,
        src1: Value,
        src2: Value,
        dst: Value,
    },
    Copy {
        src: Value,
        dst: Value,
    },
    Jump {
        target: String,
    },
    JumpIfZero {
        condition: Value,
        target: String,
    },
    JumpIfNotZero {
        condition: Value,
        target: String,
    },
    Label(String),
    FunctionCall {
        name: String,
        arguments: Vec<Value>,
        dst: Value,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    IntegerConstant(i32),
    UnsignedIntegerConstant(u32),
    LongConstant(i64),
    UnsignedLongConstant(u64),
    Variable(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOperator {
    Complement,
    Negate,
    Not,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOperator {
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
}


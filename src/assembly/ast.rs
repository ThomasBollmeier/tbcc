#[derive(Debug)]
pub struct Program {
    pub top_levels: Vec<TopLevel>,
}

impl Program {
    pub fn new(top_levels: Vec<TopLevel>) -> Self {
        Self { top_levels }
    }

    pub fn walk(&self, visitor: &mut impl Visitor) {
        visitor.enter_program(self);
        for top_level in &self.top_levels {
            match top_level {
                TopLevel::Function(func_def) => Self::walk_func_def(func_def, visitor),
                TopLevel::StaticVariable(static_var) => Self::walk_static_var(static_var, visitor),
            }
        }
        visitor.exit_program(self);
    }

    pub fn walk_mut(&mut self, visitor: &mut impl VisitorMut) {
        visitor.enter_program(self);
        for top_level in &mut self.top_levels {
            match top_level {
                TopLevel::Function(func_def) => Self::walk_func_def_mut(func_def, visitor),
                TopLevel::StaticVariable(static_var) => Self::walk_static_var_mut(static_var, visitor),
            }
        }
        visitor.exit_program(self);
    }

    fn walk_func_def_mut(func_def: &mut FuncDef, visitor: &mut impl VisitorMut) {
        visitor.enter_func_def(func_def);

        for instruction in &mut func_def.instructions {
            Self::walk_instruction_mut(instruction, visitor);
        }

        visitor.exit_func_def(func_def);
    }

    fn walk_static_var_mut(static_var: &mut StaticVar, visitor: &mut impl VisitorMut) {
        visitor.visit_static_var(static_var);
    }

    fn walk_instruction_mut(instruction: &mut Instruction, visitor: &mut impl VisitorMut) {
        visitor.visit_instruction(instruction);
    }

    fn walk_func_def(func_def: &FuncDef, visitor: &mut impl Visitor) {
        visitor.enter_func_def(func_def);

        for instruction in &func_def.instructions {
            Self::walk_instruction(instruction, visitor);
        }

        visitor.exit_func_def(func_def);
    }

    fn walk_static_var(static_var: &StaticVar, visitor: &mut impl Visitor) {
        visitor.visit_static_var(static_var);
    }

    fn walk_instruction(instruction: &Instruction, visitor: &mut impl Visitor) {
        visitor.visit_instruction(instruction);
    }
}

#[derive(Debug)]
pub enum TopLevel {
    Function(FuncDef),
    StaticVariable(StaticVar),
}

#[derive(Debug)]
pub struct FuncDef {
    pub name: String,
    pub is_global: bool,
    pub instructions: Vec<Instruction>,
    pub stack_frame_size: usize,
}

impl FuncDef {
    pub fn new(name: String, is_global: bool, instructions: Vec<Instruction>) -> Self {
        Self {
            name,
            is_global,
            instructions,
            stack_frame_size: 0, // filled by pseudo_reg_replacer
        }
    }
}

#[derive(Debug)]
pub struct StaticVar {
    pub name: String,
    pub is_global: bool,
    pub value: i32,
}

#[derive(Debug, Clone)]
pub enum Instruction {
    Mov {
        src: Operand,
        dst: Operand,
    },
    Unary {
        op: UnaryOp,
        operand: Operand,
    },
    Binary {
        op: BinaryOp,
        left: Operand,
        right: Operand,
    },
    Cmp {
        op1: Operand,
        op2: Operand,
    },
    Idiv(Operand),
    Cdq,
    Jmp(String),
    JmpCC(ConditionCode, String),
    SetCC(ConditionCode, Operand),
    Label(String),
    AllocateStack(i32),
    DeAllocateStack(i32),
    Push(Operand),
    Call(String),
    Ret,
}

#[derive(Debug, Clone)]
pub enum ConditionCode {
    Eq,
    NotEq,
    Gt,
    GtEq,
    Lt,
    LtEq,
}

#[derive(Debug, Clone)]
pub enum Operand {
    Immediate(i32),
    Register(Register),
    PseudoReg(String),
    Data(String),
    Stack(i32),
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    BitAnd,
    BitOr,
    BitXor,
    ShiftLeft,
    ShiftRight,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOp {
    Neg,
    Not,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Register {
    AX,
    CX,
    DX,
    DI,
    SI,
    R8,
    R9,
    R10,
    R11,
}

#[allow(unused_variables)]
pub trait Visitor {
    fn enter_program(&mut self, program: &Program) {}
    fn exit_program(&mut self, program: &Program) {}
    fn enter_func_def(&mut self, func_def: &FuncDef) {}
    fn exit_func_def(&mut self, func_def: &FuncDef) {}
    fn visit_static_var(&mut self, static_var: &StaticVar) {}
    fn visit_instruction(&mut self, instruction: &Instruction) {}
}

#[allow(unused_variables)]
pub trait VisitorMut {
    fn enter_program(&mut self, program: &mut Program) {}
    fn exit_program(&mut self, program: &mut Program) {}
    fn enter_func_def(&mut self, func_def: &mut FuncDef) {}
    fn exit_func_def(&mut self, func_def: &mut FuncDef) {}
    fn visit_static_var(&mut self, static_var: &mut StaticVar) {}
    fn visit_instruction(&mut self, instruction: &mut Instruction) {}
}

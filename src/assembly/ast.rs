#[derive(Debug)]
pub struct Program(pub FuncDef);

impl Program {
    pub fn new(def: FuncDef) -> Self {
        Self(def)
    }

    pub fn walk(&self, visitor: &mut impl Visitor) {
        visitor.enter_program(self);
        Self::walk_func_def(&self.0, visitor);
        visitor.exit_program(self);
    }

    pub fn walk_mut(&mut self, visitor: &mut impl VisitorMut) {
        visitor.enter_program(self);
        Self::walk_func_def_mut(&mut self.0, visitor);
        visitor.exit_program(self);
    }

    fn walk_func_def_mut(func_def: &mut FuncDef, visitor: &mut impl VisitorMut) {
        visitor.enter_func_def(func_def);

        for instruction in &mut func_def.instructions {
            Self::walk_instruction_mut(instruction, visitor);
        }

        visitor.exit_func_def(func_def);
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

    fn walk_instruction(instruction: &Instruction, visitor: &mut impl Visitor) {
        visitor.visit_instruction(instruction);
    }
}

#[derive(Debug)]
pub struct FuncDef {
    pub name: String,
    pub instructions: Vec<Instruction>,
}

impl FuncDef {
    pub fn new(name: String, instructions: Vec<Instruction>) -> Self {
        Self { name, instructions }
    }
}

#[derive(Debug, Clone)]
pub enum Instruction {
    Mov { src: Operand, dst: Operand },
    Unary { op: UnaryOp, operand: Operand },
    Binary { op: BinaryOp, left: Operand, right: Operand },
    Idiv(Operand),
    Cdq,
    AllocateStack(i32),
    Ret,
}

#[derive(Debug, Clone)]
pub enum Operand {
    Immediate(i32),
    Register(Register),
    PseudoReg(String),
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
    R10,
    R11,
}

#[allow(unused_variables)]
pub trait Visitor {
    fn enter_program(&mut self, program: &Program) {}
    fn exit_program(&mut self, program: &Program) {}
    fn enter_func_def(&mut self, func_def: &FuncDef) {}
    fn exit_func_def(&mut self, func_def: &FuncDef) {}
    fn visit_instruction(&mut self, instruction: &Instruction) {}
}

#[allow(unused_variables)]
pub trait VisitorMut {
    fn enter_program(&mut self, program: &mut Program) {}
    fn exit_program(&mut self, program: &mut Program) {}
    fn enter_func_def(&mut self, func_def: &mut FuncDef) {}
    fn exit_func_def(&mut self, func_def: &mut FuncDef) {}
    fn visit_instruction(&mut self, instruction: &mut Instruction) {}
}

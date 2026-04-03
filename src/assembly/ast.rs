#[derive(Debug)]
pub struct Program(pub FuncDef);

impl Program {
    pub fn new(def: FuncDef) -> Self {
        Self(def)
    }

    pub fn walk(&mut self, visitor: &mut impl VisitorMut) {
        visitor.enter_program(self);
        Self::walk_func_def(&mut self.0, visitor);
        visitor.exit_program(self);
    }

    fn walk_func_def(func_def: &mut FuncDef, visitor: &mut impl VisitorMut) {
        visitor.enter_func_def(func_def);

        for instruction in &mut func_def.instructions {
            Self::walk_instruction(instruction, visitor);
        }

        visitor.exit_func_def(func_def);
    }

    fn walk_instruction(instruction: &mut Instruction, visitor: &mut impl VisitorMut) {
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
pub enum UnaryOp {
    Neg,
    Not,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Register {
    AX,
    R10,
}


#[allow(unused_variables)]
pub trait VisitorMut {
    fn enter_program(&mut self, program: &mut Program) {}
    fn exit_program(&mut self, program: &mut Program) {}
    fn enter_func_def(&mut self, func_def: &mut FuncDef) {}
    fn exit_func_def(&mut self, func_def: &mut FuncDef) {}
    fn visit_instruction(&mut self, instruction: &mut Instruction) {}
}

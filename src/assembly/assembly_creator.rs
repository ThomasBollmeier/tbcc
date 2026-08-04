use crate::assembly::ast::AssemblyType::{Double, Longword, Quadword};
use crate::assembly::ast::BinaryOp::{BitXor, DivDouble};
use crate::assembly::ast::Instruction::{
    Binary, Cdq, Cmp, ConvertDoubleToInt, ConvertIntToDouble, Idiv, Jmp, JmpCC, Label, Mov,
    MovZeroExtend, SetCC, Unary,
};
use crate::assembly::ast::Operand::{Immediate, Stack};
use crate::assembly::ast::Register::{
    AX, CX, DI, DX, R8, R9, SI, XMM0, XMM1, XMM2, XMM3, XMM4, XMM5, XMM6, XMM7,
};
use crate::assembly::ast::{
    AssemblyType, BinaryOp, ImmValue, StaticConst, StaticVar, TopLevel as TopLevelAsm,
};
use crate::assembly::ast::{
    ConditionCode, FuncDef, Instruction, Operand, Program, Register, UnaryOp,
};
use crate::assembly::symbol_table::SymbolTableEntry as AsmSymbolTableEntry;
use crate::ast::Type::{Int, Long, UInt, ULong};
use crate::common::name_generator::make_static_const_label_generator;
use crate::common::symbol_table::SymbolTableEntry;
use crate::common::symbol_table_generic::{SymbolTable, SymbolTableRef};
use crate::common::{InitValue, Type, symbol_table};
use crate::semantic::NameGeneratorRef;
use crate::tacky::ast::{
    BinaryOperator as TackyBinOp, BinaryOperator, Function, Instruction as TackyInstruction,
    StaticVariable, TopLevel, UnaryOperator, Value,
};
use anyhow::{Result, anyhow};
use std::collections::HashMap;

pub struct AssemblyCreator {
    int_arg_registers: [Register; 6],
    double_arg_registers: [Register; 8],
    symbol_table: SymbolTableRef<SymbolTableEntry>,
    static_const_label_gen: NameGeneratorRef,
    static_consts: HashMap<(Type, String), StaticConst>,
    label_name_generator: NameGeneratorRef,
}

impl AssemblyCreator {
    pub fn new(
        symbol_table: SymbolTableRef<SymbolTableEntry>,
        label_name_generator: NameGeneratorRef,
    ) -> AssemblyCreator {
        AssemblyCreator {
            int_arg_registers: [DI, SI, DX, CX, R8, R9],
            double_arg_registers: [XMM0, XMM1, XMM2, XMM3, XMM4, XMM5, XMM6, XMM7],
            symbol_table: symbol_table.clone(),
            static_const_label_gen: make_static_const_label_generator(),
            static_consts: HashMap::new(),
            label_name_generator,
        }
    }

    pub fn create_program(
        &mut self,
        tacky_program: &crate::tacky::ast::Program,
    ) -> Result<(Program, SymbolTableRef<AsmSymbolTableEntry>)> {
        let mut top_levels_asm = vec![];
        for top_level in &tacky_program.0 {
            match top_level {
                TopLevel::Function(f) => {
                    let func_def_asm = self.create_func_def(f)?;
                    top_levels_asm.push(TopLevelAsm::Function(func_def_asm));
                }
                TopLevel::StaticVariable(static_var) => {
                    let static_var_asm = self.create_static_var(static_var)?;
                    top_levels_asm.push(TopLevelAsm::StaticVariable(static_var_asm));
                }
            }
        }

        let mut static_constants: Vec<TopLevelAsm> = self
            .static_consts
            .values()
            .map(|static_const| TopLevelAsm::StaticConstant(static_const.clone()))
            .collect();

        static_constants.extend(top_levels_asm);
        top_levels_asm = static_constants;

        let asm_symbol_table = self.fill_asm_symbol_table();

        Ok((Program::new(top_levels_asm), asm_symbol_table))
    }

    fn fill_asm_symbol_table(&self) -> SymbolTableRef<AsmSymbolTableEntry> {
        let asm_symbol_table: SymbolTableRef<AsmSymbolTableEntry> = SymbolTable::new_ref();

        for (name, entry) in self.symbol_table.borrow().get_all_entries() {
            let asm_entry = match &entry.attrs {
                symbol_table::IdentAttrs::Function { is_defined, .. } => {
                    AsmSymbolTableEntry::Function {
                        is_defined: *is_defined,
                    }
                }
                symbol_table::IdentAttrs::Static { .. } => AsmSymbolTableEntry::Object {
                    assembly_type: Self::map_type_to_asm_type(&entry.c_type),
                    is_static: true,
                    is_constant: false,
                },
                symbol_table::IdentAttrs::Local => AsmSymbolTableEntry::Object {
                    assembly_type: Self::map_type_to_asm_type(&entry.c_type),
                    is_static: false,
                    is_constant: false,
                },
            };

            asm_symbol_table
                .borrow_mut()
                .insert(name.clone(), asm_entry);
        }

        for static_const in self.static_consts.values() {
            let asm_entry = AsmSymbolTableEntry::Object {
                assembly_type: Self::map_init_value_to_asm_type(&static_const.value),
                is_static: true,
                is_constant: true,
            };
            asm_symbol_table
                .borrow_mut()
                .insert(static_const.name.clone(), asm_entry);
        }

        asm_symbol_table
    }

    fn create_static_var(&mut self, static_var: &StaticVariable) -> Result<StaticVar> {
        let value = self.determine_static_var_value(static_var)?;
        let alignment = match value {
            InitValue::Int(_) | InitValue::UInt(_) => 4,
            InitValue::Long(_) | InitValue::ULong(_) => 8,
            InitValue::Double(_) => 8,
        };

        Ok(StaticVar {
            name: static_var.name.clone(),
            is_global: static_var.is_global,
            value,
            alignment,
        })
    }

    fn determine_static_var_value(&self, static_var: &StaticVariable) -> Result<InitValue> {
        match static_var.initial_value {
            Value::IntegerConstant(i) => Self::cast_int_to_ctype(i, &static_var.c_type),
            Value::UnsignedIntegerConstant(u) => Self::cast_uint_to_ctype(u, &static_var.c_type),
            Value::LongConstant(l) => Self::cast_long_to_ctype(l, &static_var.c_type),
            Value::UnsignedLongConstant(ul) => Self::cast_ulong_to_ctype(ul, &static_var.c_type),
            Value::DoubleConstant(d) => Self::cast_double_to_ctype(d, &static_var.c_type),
            _ => Err(anyhow!("invalid initial value of static variable")),
        }
    }

    fn cast_int_to_ctype(value: i32, c_type: &Type) -> Result<InitValue> {
        match c_type {
            Int => Ok(InitValue::Int(value)),
            UInt => Ok(InitValue::UInt(value as u32)),
            Long => Ok(InitValue::Long(value as i64)),
            ULong => Ok(InitValue::ULong(value as u64)),
            Type::Double => Ok(InitValue::Double(value as f64)),
            _ => Err(anyhow!("invalid target type of cast: {:#?}", c_type)),
        }
    }

    fn cast_uint_to_ctype(value: u32, c_type: &Type) -> Result<InitValue> {
        match c_type {
            Int => Ok(InitValue::Int(value as i32)),
            UInt => Ok(InitValue::UInt(value)),
            Long => Ok(InitValue::Long(value as i64)),
            ULong => Ok(InitValue::ULong(value as u64)),
            Type::Double => Ok(InitValue::Double(value as f64)),
            _ => Err(anyhow!("invalid target type of cast: {:#?}", c_type)),
        }
    }

    fn cast_long_to_ctype(value: i64, c_type: &Type) -> Result<InitValue> {
        match c_type {
            Int => Ok(InitValue::Int(value as i32)),
            UInt => Ok(InitValue::UInt(value as u32)),
            Long => Ok(InitValue::Long(value)),
            ULong => Ok(InitValue::ULong(value as u64)),
            Type::Double => Ok(InitValue::Double(value as f64)),
            _ => Err(anyhow!("invalid target type of cast: {:#?}", c_type)),
        }
    }

    fn cast_ulong_to_ctype(value: u64, c_type: &Type) -> Result<InitValue> {
        match c_type {
            Int => Ok(InitValue::Int(value as i32)),
            UInt => Ok(InitValue::UInt(value as u32)),
            Long => Ok(InitValue::Long(value as i64)),
            ULong => Ok(InitValue::ULong(value)),
            Type::Double => Ok(InitValue::Double(value as f64)),
            _ => Err(anyhow!("invalid target type of cast: {:#?}", c_type)),
        }
    }

    fn cast_double_to_ctype(value: f64, c_type: &Type) -> Result<InitValue> {
        match c_type {
            Int => Ok(InitValue::Int(value as i32)),
            UInt => Ok(InitValue::UInt(value as u32)),
            Long => Ok(InitValue::Long(value as i64)),
            ULong => Ok(InitValue::ULong(value as u64)),
            Type::Double => Ok(InitValue::Double(value)),
            _ => Err(anyhow!("invalid target type of cast: {:#?}", c_type)),
        }
    }

    fn create_func_def(&mut self, func_def: &Function) -> Result<FuncDef> {
        let name = func_def.name.clone();
        let func_type = self.lookup_function_type(&name)?;
        let param_types = func_type.0;

        let (int_reg_params, double_reg_params, stack_params) =
            self.split_items(&param_types, &func_def.parameters)?;

        let mut moves = vec![];

        for (idx, param) in int_reg_params.iter().enumerate() {
            let assembly_type = self.lookup_asm_type(param);
            let src = Operand::Register(self.int_arg_registers[idx].clone());
            let dst = Operand::PseudoReg(param.clone());
            moves.push((assembly_type, src, dst));
        }

        for (idx, param) in double_reg_params.iter().enumerate() {
            let assembly_type = self.lookup_asm_type(param);
            let src = Operand::Register(self.double_arg_registers[idx].clone());
            let dst = Operand::PseudoReg(param.clone());
            moves.push((assembly_type, src, dst));
        }

        let mut offset = 16;
        for param in &stack_params {
            let assembly_type = self.lookup_asm_type(param);
            let src = Stack(offset);
            offset += 8;
            let dst = Operand::PseudoReg(param.clone());
            moves.push((assembly_type, src, dst));
        }

        let mut instructions: Vec<Instruction> = moves
            .iter()
            .map(|(assembly_type, src, dst)| Mov {
                assembly_type: assembly_type.clone(),
                src: src.clone(),
                dst: dst.clone(),
            })
            .collect();

        instructions.extend(self.create_instructions(&func_def.body)?);

        Ok(FuncDef::new(name, func_def.is_global, instructions))
    }

    fn create_instructions(
        &mut self,
        instructions: &Vec<TackyInstruction>,
    ) -> Result<Vec<Instruction>> {
        let mut ret = vec![];

        for instruction in instructions {
            match instruction {
                TackyInstruction::Return(value) => self.push_return(&mut ret, value),
                TackyInstruction::SignExtend { src, dst } => {
                    self.push_sign_extend(&mut ret, src, dst);
                }
                TackyInstruction::ZeroExtend { src, dst } => {
                    self.push_zero_extend(&mut ret, src, dst)
                }
                TackyInstruction::Truncate { src, dst } => {
                    self.push_truncate(&mut ret, src, dst);
                }
                TackyInstruction::Unary {
                    op: UnaryOperator::Not,
                    src,
                    dst,
                } => self.push_unary_not(&mut ret, src, dst),
                TackyInstruction::Unary { op, src, dst } => self.push_unary(&mut ret, op, src, dst),
                TackyInstruction::Binary {
                    op: TackyBinOp::Divide,
                    src1,
                    src2,
                    dst,
                } => self.push_binary_divide(&mut ret, src1, src2, dst),
                TackyInstruction::Binary {
                    op: TackyBinOp::Remainder,
                    src1,
                    src2,
                    dst,
                } => self.push_binary_remainder(&mut ret, src1, src2, dst),
                TackyInstruction::Binary {
                    op,
                    src1,
                    src2,
                    dst,
                } => self.push_binary(&mut ret, op, src1, src2, dst),
                TackyInstruction::Jump { target } => self.push_jump(&mut ret, target),
                TackyInstruction::JumpIfZero { condition, target } => {
                    self.push_jump_if_zero(&mut ret, condition, target)
                }
                TackyInstruction::JumpIfNotZero { condition, target } => {
                    self.push_jump_if_not_zero(&mut ret, condition, target)
                }
                TackyInstruction::Copy { src, dst } => self.push_copy(&mut ret, src, dst),
                TackyInstruction::Label(name) => self.push_label(&mut ret, name),
                TackyInstruction::FunctionCall {
                    name,
                    arguments,
                    dst,
                } => self.push_function_call(&mut ret, name, arguments, dst),
                TackyInstruction::IntToDouble { src, dst } => {
                    self.push_int_to_double(&mut ret, src, dst)
                }
                TackyInstruction::DoubleToInt { src, dst } => {
                    self.push_double_to_int(&mut ret, src, dst)
                }
                TackyInstruction::UintToDouble { src, dst } => {
                    self.push_uint_to_double(&mut ret, src, dst)
                }
                TackyInstruction::DoubleToUint { src, dst } => {
                    self.push_double_to_uint(&mut ret, src, dst)
                }
            }
        }

        Ok(ret)
    }

    fn push_function_call(
        &mut self,
        instructions: &mut Vec<Instruction>,
        name: &str,
        arguments: &Vec<Value>,
        dst: &Value,
    ) {
        use Register::*;

        const ARG_SIZE: usize = 8;

        let (param_types, _) = self
            .lookup_function_type(name)
            .expect("function type not found");

        let (int_reg_args, double_reg_args, stack_args) = self
            .split_items(&param_types, arguments)
            .expect("invalid arguments");

        let stack_padding = if stack_args.len() % 2 == 0 { 0 } else { 8 };

        if stack_padding > 0 {
            instructions.push(AssemblyCreator::allocate_stack(stack_padding));
        }

        // System V calling convention:

        // First 6 integer arguments into registers
        for (reg_index, arg) in int_reg_args.iter().enumerate() {
            let assembly_type = self.get_asm_type(arg);
            let src = self.create_operand(arg);
            let dst = Operand::Register(self.int_arg_registers[reg_index].clone());
            instructions.push(Mov {
                assembly_type,
                src,
                dst,
            });
        }

        // First 8 double arguments into registers
        for (reg_index, arg) in double_reg_args.iter().enumerate() {
            let src = self.create_operand(arg);
            let dst = Operand::Register(self.double_arg_registers[reg_index].clone());
            instructions.push(Mov {
                assembly_type: Double,
                src,
                dst,
            });
        }

        // Remaining arguments pushed onto stack
        for arg in stack_args.iter().rev() {
            let assembly_type = self.get_asm_type(arg);
            let op = self.create_operand(arg);
            match op {
                Operand::Register(_) | Immediate(_) => {
                    instructions.push(Instruction::Push(op));
                }
                _ => match assembly_type {
                    Quadword | Double => {
                        instructions.push(Instruction::Push(op));
                    }
                    _ => {
                        instructions.push(Mov {
                            assembly_type,
                            src: op,
                            dst: Operand::Register(AX),
                        });
                        instructions.push(Instruction::Push(Operand::Register(AX)));
                    }
                },
            }
        }

        instructions.push(Instruction::Call(name.to_string()));

        // Adjust stack pointer
        let bytes_to_remove = ARG_SIZE * stack_args.len() + stack_padding as usize;
        if bytes_to_remove > 0 {
            instructions.push(AssemblyCreator::deallocate_stack(bytes_to_remove as i32));
        }

        // Set return value:
        let assembly_type = self.get_asm_type(dst);
        let src = Self::get_return_register(&assembly_type);
        let dst = self.create_operand(dst);
        instructions.push(Mov {
            assembly_type,
            src,
            dst,
        });
    }

    fn get_return_register(assembly_type: &AssemblyType) -> Operand {
        if *assembly_type == Double {
            Operand::Register(XMM0)
        } else {
            Operand::Register(AX)
        }
    }

    fn split_items<T: Clone>(
        &self,
        param_types: &[Type],
        items: &Vec<T>,
    ) -> Result<(Vec<T>, Vec<T>, Vec<T>)> {
        if param_types.len() != items.len() {
            return Err(anyhow!("number of parameters and items do not match"));
        }

        let num_int_regs = self.int_arg_registers.len();
        let num_double_regs = self.double_arg_registers.len();
        let mut int_reg_items = Vec::new();
        let mut double_reg_items = Vec::new();
        let mut stack_items = Vec::new();

        for (param_type, item) in param_types.iter().zip(items) {
            match param_type {
                Int | UInt | Long | ULong => {
                    if int_reg_items.len() < num_int_regs {
                        int_reg_items.push(item.clone());
                    } else {
                        stack_items.push(item.clone());
                    }
                }
                Type::Double => {
                    if double_reg_items.len() < num_double_regs {
                        double_reg_items.push(item.clone());
                    } else {
                        stack_items.push(item.clone());
                    }
                }
                _ => {
                    return Err(anyhow!("unsupported parameter type"));
                }
            }
        }

        Ok((int_reg_items, double_reg_items, stack_items))
    }

    fn lookup_type(&self, name: &str) -> Result<Type> {
        self.symbol_table
            .borrow()
            .get_entry(name)
            .map(|entry| entry.c_type.clone())
            .ok_or_else(|| anyhow!("symbol not found: {}", name))
    }

    fn lookup_function_type(&self, func_name: &str) -> Result<(Vec<Type>, Box<Type>)> {
        if let Type::Function {
            param_types,
            return_type,
        } = self.lookup_type(func_name)?
        {
            Ok((param_types, return_type))
        } else {
            Err(anyhow!("{func_name} is not a function"))
        }
    }

    fn push_return(&mut self, instructions: &mut Vec<Instruction>, value: &Value) {
        use crate::assembly::ast::Instruction::*;

        let assembly_type = self.get_asm_type(value);
        let src = self.create_operand(value);
        let dst = Self::get_return_register(&assembly_type);
        instructions.push(Mov {
            assembly_type,
            src,
            dst,
        });
        instructions.push(Ret);
    }

    fn push_sign_extend(&mut self, instructions: &mut Vec<Instruction>, src: &Value, dst: &Value) {
        let src_op = self.create_operand(src);
        let dst_op = self.create_operand(dst);
        instructions.push(Instruction::MovSx {
            src: src_op,
            dst: dst_op,
        });
    }

    fn push_zero_extend(&mut self, instructions: &mut Vec<Instruction>, src: &Value, dst: &Value) {
        let src_op = self.create_operand(src);
        let dst_op = self.create_operand(dst);
        instructions.push(MovZeroExtend {
            src: src_op,
            dst: dst_op,
        });
    }

    fn push_truncate(&mut self, instructions: &mut Vec<Instruction>, src: &Value, dst: &Value) {
        let src_op = self.create_operand(src);
        let dst_op = self.create_operand(dst);
        instructions.push(Mov {
            assembly_type: Longword,
            src: src_op,
            dst: dst_op,
        });
    }

    fn push_unary_not(&mut self, instructions: &mut Vec<Instruction>, src: &Value, dst: &Value) {
        use crate::assembly::ast::Instruction::*;

        if self.get_asm_type(src) == Double {
            self.push_unary_double_not(instructions, src, dst);
            return;
        }

        let assembly_type = self.get_asm_type(dst);
        let src_op = self.create_operand(src);
        let dst_op = self.create_operand(dst);
        instructions.push(Cmp {
            assembly_type: assembly_type.clone(),
            op1: Immediate(ImmValue::Int(0)),
            op2: src_op,
        });
        instructions.push(Mov {
            assembly_type,
            src: Immediate(ImmValue::Int(0)),
            dst: dst_op.clone(),
        });
        instructions.push(SetCC(ConditionCode::Eq, dst_op));
    }

    fn push_unary(
        &mut self,
        instructions: &mut Vec<Instruction>,
        op: &UnaryOperator,
        src: &Value,
        dst: &Value,
    ) {
        use crate::assembly::ast::Instruction::*;

        let assembly_type = self.get_asm_type(src);

        if assembly_type == Double {
            match op {
                UnaryOperator::Negate => {
                    self.push_unary_double_negate(instructions, src, dst);
                    return;
                }
                _ => {}
            }
        }

        let src_op = self.create_operand(src);
        let dst_op = self.create_operand(dst);
        let unary_op = self.map_unary_operator(op);

        instructions.push(Mov {
            assembly_type: assembly_type.clone(),
            src: src_op,
            dst: dst_op.clone(),
        });
        instructions.push(Unary {
            assembly_type,
            op: unary_op,
            operand: dst_op,
        });
    }

    fn push_unary_double_not(
        &mut self,
        instructions: &mut Vec<Instruction>,
        src: &Value,
        dst: &Value,
    ) {
        let reg = Operand::Register(Register::XMM14);
        let src_op = self.create_operand(src);
        let dst_op = self.create_operand(dst);
        let dst_type = self.get_asm_type(dst);

        instructions.push(Binary {
            assembly_type: Double,
            op: BitXor,
            left: reg.clone(),
            right: reg.clone(),
        });
        instructions.push(Cmp {
            assembly_type: Double,
            op1: src_op,
            op2: reg,
        });
        instructions.push(Mov {
            assembly_type: dst_type,
            src: Immediate(ImmValue::Int(0)),
            dst: dst_op.clone(),
        });
        instructions.push(SetCC(ConditionCode::Eq, dst_op));
    }

    fn push_unary_double_negate(
        &mut self,
        instructions: &mut Vec<Instruction>,
        src: &Value,
        dst: &Value,
    ) {
        let src_op = self.create_operand(src);
        let dst_op = self.create_operand(dst);

        instructions.push(Mov {
            assembly_type: Double,
            src: src_op,
            dst: dst_op.clone(),
        });

        let negative_zero = self.create_double_constant_operand_with_key("negative_zero", -0.0, 16);

        instructions.push(Binary {
            assembly_type: Double,
            op: BitXor,
            left: negative_zero,
            right: dst_op.clone(),
        });
    }

    fn push_binary_divide(
        &mut self,
        instructions: &mut Vec<Instruction>,
        src1: &Value,
        src2: &Value,
        dst: &Value,
    ) {
        use crate::assembly::ast::Instruction::*;

        let assembly_type = self.get_asm_type(src1);
        let src1_op = self.create_operand(src1);
        let src2_op = self.create_operand(src2);
        let dst_op = self.create_operand(dst);

        if assembly_type == Double {
            instructions.push(Mov {
                assembly_type: assembly_type.clone(),
                src: src1_op,
                dst: dst_op.clone(),
            });
            instructions.push(Binary {
                assembly_type,
                op: DivDouble,
                left: src2_op,
                right: dst_op,
            });
            return;
        }

        instructions.push(Mov {
            assembly_type: assembly_type.clone(),
            src: src1_op,
            dst: Operand::Register(AX),
        });

        self.push_div(instructions, &assembly_type, src1, &src2_op);

        instructions.push(Mov {
            assembly_type,
            src: Operand::Register(AX),
            dst: dst_op,
        });
    }

    fn push_div(
        &mut self,
        instructions: &mut Vec<Instruction>,
        assembly_type: &AssemblyType,
        src1: &Value,
        src2_op: &Operand,
    ) {
        if self.is_value_unsigned(src1) {
            let zero_op = if *assembly_type == Longword {
                Immediate(ImmValue::UInt(0))
            } else {
                Immediate(ImmValue::ULong(0))
            };
            instructions.push(Mov {
                assembly_type: assembly_type.clone(),
                src: zero_op,
                dst: Operand::Register(DX),
            });
            instructions.push(Instruction::Div {
                assembly_type: assembly_type.clone(),
                operand: src2_op.clone(),
            });
        } else {
            instructions.push(Cdq(assembly_type.clone()));
            instructions.push(Idiv {
                assembly_type: assembly_type.clone(),
                operand: src2_op.clone(),
            });
        }
    }

    fn push_binary_remainder(
        &mut self,
        instructions: &mut Vec<Instruction>,
        src1: &Value,
        src2: &Value,
        dst: &Value,
    ) {
        use crate::assembly::ast::Instruction::*;

        let assembly_type = self.get_asm_type(src1);
        let src1_op = self.create_operand(src1);
        let src2_op = self.create_operand(src2);
        let dst_op = self.create_operand(dst);

        instructions.push(Mov {
            assembly_type: assembly_type.clone(),
            src: src1_op,
            dst: Operand::Register(AX),
        });

        self.push_div(instructions, &assembly_type, src1, &src2_op);

        instructions.push(Mov {
            assembly_type,
            src: Operand::Register(DX),
            dst: dst_op,
        });
    }

    fn push_binary(
        &mut self,
        instructions: &mut Vec<Instruction>,
        op: &TackyBinOp,
        src1: &Value,
        src2: &Value,
        dst: &Value,
    ) {
        match op {
            BinaryOperator::Equal
            | BinaryOperator::NotEqual
            | BinaryOperator::Greater
            | BinaryOperator::GreaterEqual
            | BinaryOperator::Less
            | BinaryOperator::LessEqual => {
                self.push_binary_relational(instructions, op, src1, src2, dst)
            }
            _ => self.push_binary_arithmetic(instructions, op, src1, src2, dst),
        }
    }

    fn push_binary_relational(
        &mut self,
        instructions: &mut Vec<Instruction>,
        op: &TackyBinOp,
        src1: &Value,
        src2: &Value,
        dst: &Value,
    ) {
        use crate::assembly::ast::Instruction::*;

        let src1_op = self.create_operand(src1);
        let src1_asm_type = self.get_asm_type(src1);
        let src2_op = self.create_operand(src2);
        let dst_op = self.create_operand(dst);

        instructions.push(Cmp {
            assembly_type: src1_asm_type.clone(),
            op1: src2_op,
            op2: src1_op,
        });
        let is_unsigned = self.is_value_unsigned(src1);
        let is_double = src1_asm_type == Double;
        let condition_code = self.map_relational_operator(op, is_unsigned || is_double);
        instructions.push(Mov {
            assembly_type: self.get_asm_type(dst),
            src: Immediate(ImmValue::Int(0)),
            dst: dst_op.clone(),
        });
        instructions.push(SetCC(condition_code, dst_op));
    }

    fn push_binary_arithmetic(
        &mut self,
        instructions: &mut Vec<Instruction>,
        op: &TackyBinOp,
        src1: &Value,
        src2: &Value,
        dst: &Value,
    ) {
        use crate::assembly::ast::Instruction::*;

        let assembly_type = self.get_asm_type(src1);
        let src1_op = self.create_operand(src1);
        let src2_op = self.create_operand(src2);
        let dst_op = self.create_operand(dst);

        let is_src1_unsigned = self.is_value_unsigned(src1);

        let binary_op = self.map_binary_operator(op, is_src1_unsigned);
        instructions.push(Mov {
            assembly_type: assembly_type.clone(),
            src: src1_op,
            dst: dst_op.clone(),
        });
        instructions.push(Binary {
            assembly_type,
            op: binary_op,
            left: src2_op,
            right: dst_op,
        });
    }

    fn push_jump(&mut self, instructions: &mut Vec<Instruction>, target: &str) {
        instructions.push(Jmp(target.to_string()));
    }

    fn push_jump_if_zero(
        &mut self,
        instructions: &mut Vec<Instruction>,
        condition: &Value,
        target: &str,
    ) {
        use crate::assembly::ast::Instruction::*;

        let assembly_type = self.get_asm_type(condition);

        if assembly_type == Double {
            self.push_jump_if_zero_double(instructions, condition, target);
            return;
        }

        let condition_op = self.create_operand(condition);
        instructions.push(Cmp {
            assembly_type,
            op1: Immediate(ImmValue::Int(0)),
            op2: condition_op,
        });
        instructions.push(JmpCC(ConditionCode::Eq, target.to_string()));
    }

    fn push_jump_if_zero_double(
        &mut self,
        instructions: &mut Vec<Instruction>,
        condition: &Value,
        target: &str,
    ) {
        let reg = Operand::Register(Register::XMM14);

        instructions.push(Binary {
            op: BitXor,
            assembly_type: Double,
            left: reg.clone(),
            right: reg.clone(),
        });
        let condition_op = self.create_operand(condition);
        instructions.push(Cmp {
            assembly_type: Double,
            op1: condition_op,
            op2: reg,
        });
        instructions.push(JmpCC(ConditionCode::Eq, target.to_string()));
    }

    fn push_jump_if_not_zero(
        &mut self,
        instructions: &mut Vec<Instruction>,
        condition: &Value,
        target: &str,
    ) {
        use crate::assembly::ast::Instruction::*;

        let assembly_type = self.get_asm_type(condition);

        if assembly_type == Double {
            self.push_jump_if_not_zero_double(instructions, condition, target);
            return;
        }

        let condition_op = self.create_operand(condition);
        instructions.push(Cmp {
            assembly_type,
            op1: Immediate(ImmValue::Int(0)),
            op2: condition_op,
        });
        instructions.push(JmpCC(ConditionCode::NotEq, target.to_string()));
    }

    fn push_jump_if_not_zero_double(
        &mut self,
        instructions: &mut Vec<Instruction>,
        condition: &Value,
        target: &str,
    ) {
        let reg = Operand::Register(Register::XMM14);

        instructions.push(Binary {
            op: BitXor,
            assembly_type: Double,
            left: reg.clone(),
            right: reg.clone(),
        });
        let condition_op = self.create_operand(condition);
        instructions.push(Cmp {
            assembly_type: Double,
            op1: condition_op,
            op2: reg,
        });
        instructions.push(JmpCC(ConditionCode::NotEq, target.to_string()));
    }

    fn push_int_to_double(
        &mut self,
        instructions: &mut Vec<Instruction>,
        src: &Value,
        dst: &Value,
    ) {
        instructions.push(ConvertIntToDouble {
            src: self.create_operand(src),
            dst: self.create_operand(dst),
            src_type: self.get_asm_type(src),
        });
    }

    fn push_double_to_int(
        &mut self,
        instructions: &mut Vec<Instruction>,
        src: &Value,
        dst: &Value,
    ) {
        instructions.push(ConvertDoubleToInt {
            src: self.create_operand(src),
            dst: self.create_operand(dst),
            dst_type: self.get_asm_type(dst),
        });
    }

    fn push_uint_to_double(
        &mut self,
        instructions: &mut Vec<Instruction>,
        src: &Value,
        dst: &Value,
    ) {
        let is_long = self.get_asm_type(src) == Quadword;
        let src = self.create_operand(src);
        let dst = self.create_operand(dst);
        let reg1 = Operand::Register(Register::R10);
        let reg2 = Operand::Register(Register::R11);

        if !is_long {
            instructions.push(MovZeroExtend {
                src,
                dst: reg1.clone(),
            });
            instructions.push(ConvertIntToDouble {
                src_type: Quadword,
                src: reg1,
                dst,
            });
        } else {
            let label1 = self
                .label_name_generator
                .borrow_mut()
                .make_unique_name("uint_to_double");
            let label2 = self
                .label_name_generator
                .borrow_mut()
                .make_unique_name("uint_to_double");

            instructions.extend(vec![
                Cmp {
                    assembly_type: Quadword,
                    op1: Immediate(ImmValue::ULong(0)),
                    op2: src.clone(),
                },
                JmpCC(ConditionCode::Lt, label1.clone()),
                ConvertIntToDouble {
                    src_type: Quadword,
                    src: src.clone(),
                    dst: dst.clone(),
                },
                Jmp(label2.clone()),
                Label(label1.clone()),
                Mov {
                    assembly_type: Quadword,
                    src: src.clone(),
                    dst: reg1.clone(),
                },
                Mov {
                    assembly_type: Quadword,
                    src: reg1.clone(),
                    dst: reg2.clone(),
                },
                Unary {
                    op: UnaryOp::Shr,
                    assembly_type: Quadword,
                    operand: reg2.clone(),
                },
                Binary {
                    op: BinaryOp::BitAnd,
                    assembly_type: Quadword,
                    left: Immediate(ImmValue::ULong(1)),
                    right: reg1.clone(),
                },
                Binary {
                    op: BinaryOp::BitOr,
                    assembly_type: Quadword,
                    left: reg1.clone(),
                    right: reg2.clone(),
                },
                ConvertIntToDouble {
                    src_type: Quadword,
                    src: reg2.clone(),
                    dst: dst.clone(),
                },
                Binary {
                    op: BinaryOp::Add,
                    assembly_type: Double,
                    left: dst.clone(),
                    right: dst.clone(),
                },
                Label(label2.clone()),
            ]);
        }
    }

    fn push_double_to_uint(
        &mut self,
        instructions: &mut Vec<Instruction>,
        src: &Value,
        dst: &Value,
    ) {
        let is_long = self.get_asm_type(dst) == Quadword;
        let src = self.create_operand(src);
        let dst = self.create_operand(dst);
        let reg = Operand::Register(Register::R10);

        if !is_long {
            instructions.extend(vec![
                ConvertDoubleToInt {
                    src: src.clone(),
                    dst: reg.clone(),
                    dst_type: Quadword,
                },
                Mov {
                    assembly_type: Longword,
                    src: reg,
                    dst,
                },
            ]);
        } else {
            let label1 = self
                .label_name_generator
                .borrow_mut()
                .make_unique_name("double_to_uint");
            let label2 = self
                .label_name_generator
                .borrow_mut()
                .make_unique_name("double_to_uint");
            const MAX_VALUE: u64 = i64::MAX as u64 + 1;
            let upper_bound =
                self.create_double_constant_operand_with_key("upper_bound", MAX_VALUE as f64, 8);

            let x_reg = Operand::Register(Register::XMM14);

            instructions.extend(vec![
                Cmp {
                    assembly_type: Double,
                    op1: upper_bound.clone(),
                    op2: src.clone(),
                },
                JmpCC(ConditionCode::AE, label1.clone()),
                ConvertDoubleToInt {
                    src: src.clone(),
                    dst: dst.clone(),
                    dst_type: Quadword,
                },
                Jmp(label2.clone()),
                Label(label1.clone()),
                Mov {
                    assembly_type: Double,
                    src: src.clone(),
                    dst: x_reg.clone(),
                },
                Binary {
                    op: BinaryOp::Sub,
                    assembly_type: Double,
                    left: upper_bound.clone(),
                    right: x_reg.clone(),
                },
                ConvertDoubleToInt {
                    src: x_reg.clone(),
                    dst: dst.clone(),
                    dst_type: Quadword,
                },
                Mov {
                    assembly_type: Quadword,
                    src: Immediate(ImmValue::ULong(MAX_VALUE)),
                    dst: reg.clone(),
                },
                Binary {
                    op: BinaryOp::Add,
                    assembly_type: Quadword,
                    left: reg.clone(),
                    right: dst.clone(),
                },
                Label(label2.clone()),
            ]);
        }
    }

    fn push_copy(&mut self, instructions: &mut Vec<Instruction>, src: &Value, dst: &Value) {
        use crate::assembly::ast::Instruction::*;

        let src_op = self.create_operand(src);
        let dst_op = self.create_operand(dst);
        instructions.push(Mov {
            assembly_type: self.get_asm_type(src),
            src: src_op,
            dst: dst_op,
        });
    }

    fn push_label(&mut self, instructions: &mut Vec<Instruction>, name: &str) {
        instructions.push(Label(name.to_string()));
    }

    fn create_operand(&mut self, value: &Value) -> Operand {
        match value {
            Value::IntegerConstant(i) => Immediate(ImmValue::Int(*i)),
            Value::UnsignedIntegerConstant(u) => Immediate(ImmValue::UInt(*u)),
            Value::LongConstant(l) => Immediate(ImmValue::Long(*l)),
            Value::UnsignedLongConstant(ul) => Immediate(ImmValue::ULong(*ul)),
            Value::DoubleConstant(dbl) => self.create_double_constant_operand(*dbl),
            Value::Variable(name) => Operand::PseudoReg(name.clone()),
        }
    }

    fn create_double_constant_operand(&mut self, value: f64) -> Operand {
        let key = format!("{value}");
        self.create_double_constant_operand_with_key(&key, value, 8)
    }

    fn create_double_constant_operand_with_key(
        &mut self,
        key: &str,
        value: f64,
        alignment: i32,
    ) -> Operand {
        let key = (Type::Double, key.to_string());

        let name = if let Some(static_const) = self.static_consts.get(&key) {
            static_const.name.clone()
        } else {
            let name = self
                .static_const_label_gen
                .borrow_mut()
                .make_unique_name("");
            let static_const = StaticConst {
                name: name.clone(),
                value: InitValue::Double(value),
                alignment,
            };
            self.static_consts.insert(key, static_const);
            name
        };
        Operand::Data(name)
    }

    fn map_unary_operator(&self, unary_op: &UnaryOperator) -> UnaryOp {
        use crate::tacky::ast::UnaryOperator::*;
        match unary_op {
            Negate => UnaryOp::Neg,
            Complement => UnaryOp::Not,
            _ => todo!("unsupported unary operator {:?}", unary_op),
        }
    }

    fn map_binary_operator(&self, binary_op: &TackyBinOp, is_unsigned: bool) -> BinaryOp {
        use crate::tacky::ast::BinaryOperator::*;
        match binary_op {
            Add => BinaryOp::Add,
            Subtract => BinaryOp::Sub,
            Multiply => BinaryOp::Mul,
            BitAnd => BinaryOp::BitAnd,
            BitOr => BinaryOp::BitOr,
            BitXor => BinaryOp::BitXor,
            ShiftLeft => BinaryOp::ShiftLeft,
            ShiftRight => {
                if is_unsigned {
                    BinaryOp::ShiftRightLogical
                } else {
                    BinaryOp::ShiftRightArithmetic
                }
            }
            Divide => unreachable!(),
            Remainder => unreachable!(),
            _ => unimplemented!("unsupported binary operator {:?}", binary_op),
        }
    }

    fn map_relational_operator(
        &self,
        relational_op: &TackyBinOp,
        is_unsigned_or_double: bool,
    ) -> ConditionCode {
        use crate::tacky::ast::BinaryOperator::*;
        if is_unsigned_or_double {
            match relational_op {
                Equal => ConditionCode::Eq,
                NotEqual => ConditionCode::NotEq,
                Greater => ConditionCode::A,
                GreaterEqual => ConditionCode::AE,
                Less => ConditionCode::B,
                LessEqual => ConditionCode::BE,
                _ => unimplemented!("unsupported relational operator {:?}", relational_op),
            }
        } else {
            match relational_op {
                Equal => ConditionCode::Eq,
                NotEqual => ConditionCode::NotEq,
                Greater => ConditionCode::Gt,
                GreaterEqual => ConditionCode::GtEq,
                Less => ConditionCode::Lt,
                LessEqual => ConditionCode::LtEq,
                _ => unimplemented!("unsupported relational operator {:?}", relational_op),
            }
        }
    }

    fn map_type_to_asm_type(c_type: &Type) -> AssemblyType {
        use crate::common::Type::*;
        match c_type {
            Int | UInt => Longword,
            Long | ULong => Quadword,
            Double => AssemblyType::Double,
            _ => unimplemented!("unsupported type {:?}", c_type),
        }
    }

    fn map_init_value_to_asm_type(value: &InitValue) -> AssemblyType {
        match value {
            InitValue::Int(_) | InitValue::UInt(_) => Longword,
            InitValue::Long(_) | InitValue::ULong(_) => Quadword,
            InitValue::Double(_) => Double,
        }
    }

    fn lookup_asm_type(&self, name: &str) -> AssemblyType {
        match self.symbol_table.borrow().get_entry(name) {
            Some(entry) => Self::map_type_to_asm_type(&entry.c_type),
            None => panic!("Symbol not found: {}", name),
        }
    }

    fn get_asm_type(&self, value: &Value) -> AssemblyType {
        match value {
            Value::IntegerConstant(_) => Longword,
            Value::UnsignedIntegerConstant(_) => Longword,
            Value::LongConstant(_) => Quadword,
            Value::UnsignedLongConstant(_) => Quadword,
            Value::DoubleConstant(_) => Double,
            Value::Variable(name) => self.lookup_asm_type(name),
        }
    }

    pub fn allocate_stack(bytes: i32) -> Instruction {
        Binary {
            assembly_type: Quadword,
            op: BinaryOp::Sub,
            left: Immediate(ImmValue::Int(bytes)),
            right: Operand::Register(Register::SP),
        }
    }

    pub fn deallocate_stack(bytes: i32) -> Instruction {
        Binary {
            assembly_type: Quadword,
            op: BinaryOp::Add,
            left: Immediate(ImmValue::Int(bytes)),
            right: Operand::Register(Register::SP),
        }
    }

    fn is_value_unsigned(&self, value: &Value) -> bool {
        match value {
            Value::UnsignedIntegerConstant(_) | Value::UnsignedLongConstant(_) => true,
            Value::IntegerConstant(_) | Value::LongConstant(_) => false,
            Value::DoubleConstant(_) => false,
            Value::Variable(name) => match self.symbol_table.borrow().get_entry(name) {
                Some(entry) => matches!(entry.c_type, UInt | ULong),
                None => panic!("Symbol not found: {}", name),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assembly::ast::{
        BinaryOp as AsmBinaryOp, Instruction as AsmInstruction, Operand as AsmOperand,
    };
    use crate::common::Type::Int;
    use crate::common::symbol_table::IdentAttrs;
    use crate::lexer::Lexer;
    use crate::parser::Parser;
    use crate::semantic;
    use crate::semantic::NameGeneratorRef;
    use crate::tacky::TackyEmitter;
    use crate::tacky::ast::{
        BinaryOperator, Function as TackyFunctionDef, Instruction as TackyInstruction,
        Program as TackyProgram, Value,
    };
    use anyhow::Result;

    fn make_emitter(
        label_name_gen: &NameGeneratorRef,
        tmp_var_name_gen: &NameGeneratorRef,
        symbol_table: SymbolTableRef<SymbolTableEntry>,
    ) -> TackyEmitter {
        TackyEmitter::new(
            label_name_gen.clone(),
            tmp_var_name_gen.clone(),
            symbol_table,
        )
    }

    fn validate(
        var_name_gen: &NameGeneratorRef,
        label_name_gen: &NameGeneratorRef,
        symbol_table: SymbolTableRef<SymbolTableEntry>,
        program: &mut crate::ast::Program,
    ) -> Result<()> {
        semantic::validate(var_name_gen, label_name_gen, symbol_table, program)
    }

    fn run_code(code: &str) {
        let lexer = Lexer::new();
        let tokens = lexer.scan_tokens(code).expect("Failed to scan tokens");

        let parser = Parser::new();
        let mut program = parser.parse(tokens).expect("Failed to parse program");

        let var_name_gen = semantic::make_var_name_generator();
        let label_name_gen = semantic::make_label_name_generator();
        let tmp_var_name_gen = semantic::make_temp_var_name_generator();
        let symbol_table = SymbolTable::new_ref();

        validate(
            &var_name_gen,
            &label_name_gen,
            symbol_table.clone(),
            &mut program,
        )
        .expect("Failed to validate program");

        dbg!(&program);

        let mut tacky_emitter =
            make_emitter(&label_name_gen, &tmp_var_name_gen, symbol_table.clone());
        let tacky_program = tacky_emitter
            .emit_program(&program)
            .expect("Failed to emit");

        let mut assembly_creator = AssemblyCreator::new(symbol_table, label_name_gen);
        let (assembly_program, asm_symbol_table) = assembly_creator
            .create_program(&tacky_program)
            .expect("Failed to create assembly program");

        dbg!(&assembly_program);
        dbg!(&asm_symbol_table);
    }

    #[test]
    fn creates_asm_program_ok() {
        let code = "int main(void) { return 42 >> 1; }";

        run_code(code);
    }

    #[test]
    fn creates_asm_program_2_ok() {
        let code = r#"
        int return_truncated_long(long l) {
            return l;
        }

        long return_extended_int(int i) {
            return i;
        }

        int truncate_on_assignment(long l, int expected) {
            int result = l; // implicit conversion truncates l
            return result == expected;
        }

        int main(void) {

            // return statements

            /* return_truncated_long will truncate 2^32 + 2 to 2
             * assigning it to result converts this to a long
             * but preserves its value.
             */
            long result = return_truncated_long(4294967298l);
            if (result != 2l) {
                return 1;
            }

            /* return_extended_int sign-extends its argument, preserving its value */
            result = return_extended_int(-10);
            if (result != -10) {
                return 2;
            }

            // initializer

            /* This is 2^32 + 2,
             * it will be truncated to 2 by assignment
             */
            int i = 4294967298l;
            if (i != 2) {
                return 3;
            }

            // assignment expression

            // 2^34 will be truncated to 0 when assigned to an int
            if (!truncate_on_assignment(17179869184l, 0)) {
                return 4;
            }

            return 0;
        }
        "#;

        run_code(code);
    }

    #[test]
    fn creates_asm_program_3_ok() {
        let code = r#"
        int main(void) {
            int b = -1;
            b = b - 1;
        }
        "#;

        run_code(code);
    }

    #[test]
    fn creates_asm_program_4_ok() {
        let code = r#"
        long a;
        long b;

        int equal(void) {
            return (a == b);
        }

        int main(void) {
            a = 1152921504606846976l; // 2^60
            b = a;
            if (equal())
                return 0;

            return 99;
        }
        "#;

        run_code(code);
    }

    #[test]
    fn creates_asm_program_5_ok() {
        let code = r#"
        int switch_on_long(long l) {
            switch (l) {
                case 0: return 0;
                case 100: return 1;
                case 8589934592l: // 2^33
                    return 2;
                default:
                    return -1;
            }
        }

        int main(void) {
            if (switch_on_long(8589934592) != 2)
                return 1;
            if (switch_on_long(100) != 1)
                return 2;
            return 0; // success
        }
        "#;

        run_code(code);
    }

    #[test]
    fn creates_asm_program_6_ok() {
        let code = r#"
        long main(void) {
          // bitwise compound operations on long integers
            long l1 = 71777214294589695l;  // 0x00ff_00ff_00ff_00ff
            long l2 = -4294967296;  // -2^32; upper 32 bits are 1, lower 32 bits are 0

            l1 &= l2; // should zero out the lower 32 bits of l1

            if (l1 != 71777214277877760l) {
              return l1;
            }

            return 42;

        }
        "#;

        run_code(code);
    }

    #[test]
    fn creates_asm_program_7_ok() {
        let code = r#"
        int main(void) {
            int x = 1;
            if (x << 3l != 8) {
                return 1;
            }

            return 0;
        }

        "#;

        run_code(code);
    }

    #[test]
    fn creates_asm_program_with_double_addition() {
        let code = r#"
        double increment(double x) {
            return x + 1.0;
        }
        "#;

        run_code(code);
    }

    #[test]
    fn creates_asm_program_with_double_addition_2() {
        let code = r#"
        int main(void) {
            double x = 40.0;
            double y = 2.0;
            return x + y;
        }
        "#;

        run_code(code);
    }

    #[test]
    fn creates_asm_program_with_global_double_vars() {
        let code = r#"
        double a = 4294967295u;

        int main(void) {
          if (a != 4294967295.) {
            return 99;
          }
          return 0;
        }
        "#;

        run_code(code);
    }

    #[test]
    fn creates_asm_program_w_double_not() {
        let code = r#"
         int non_zero(double x) {
            return !x;
        }
        "#;

        run_code(code);
    }

    #[test]
    fn creates_asm_program_with_binary_ops() {
        let symbol_table: SymbolTableRef<SymbolTableEntry> = SymbolTable::new_ref();

        for i in 0..=4 {
            let var_name = format!("tmp.{}", i);
            symbol_table.borrow_mut().insert(
                &var_name,
                SymbolTableEntry {
                    c_type: Int,
                    attrs: IdentAttrs::Local,
                },
            );
        }

        symbol_table.borrow_mut().insert(
            "main",
            SymbolTableEntry {
                c_type: Type::Function {
                    return_type: Box::new(Int),
                    param_types: vec![],
                },
                attrs: IdentAttrs::Function {
                    is_defined: true,
                    is_global: true,
                },
            },
        );

        let tacky_program = TackyProgram(vec![TopLevel::Function(TackyFunctionDef {
            name: "main".to_string(),
            is_global: true,
            parameters: vec![],
            body: vec![
                TackyInstruction::Binary {
                    op: BinaryOperator::Add,
                    src1: Value::IntegerConstant(1),
                    src2: Value::IntegerConstant(2),
                    dst: Value::Variable("tmp.0".to_string()),
                },
                TackyInstruction::Binary {
                    op: BinaryOperator::Subtract,
                    src1: Value::Variable("tmp.0".to_string()),
                    src2: Value::IntegerConstant(3),
                    dst: Value::Variable("tmp.1".to_string()),
                },
                TackyInstruction::Binary {
                    op: BinaryOperator::Multiply,
                    src1: Value::Variable("tmp.1".to_string()),
                    src2: Value::IntegerConstant(4),
                    dst: Value::Variable("tmp.2".to_string()),
                },
                TackyInstruction::Binary {
                    op: BinaryOperator::Divide,
                    src1: Value::Variable("tmp.2".to_string()),
                    src2: Value::IntegerConstant(5),
                    dst: Value::Variable("tmp.3".to_string()),
                },
                TackyInstruction::Binary {
                    op: BinaryOperator::Remainder,
                    src1: Value::Variable("tmp.3".to_string()),
                    src2: Value::IntegerConstant(2),
                    dst: Value::Variable("tmp.4".to_string()),
                },
                TackyInstruction::Return(Value::Variable("tmp.4".to_string())),
            ],
        })]);

        let label_name_generator = semantic::make_label_name_generator();
        let mut assembly_creator = AssemblyCreator::new(symbol_table, label_name_generator);
        let (assembly_program, _) = assembly_creator
            .create_program(&tacky_program)
            .expect("Failed to create assembly program");

        let main_func = if let TopLevelAsm::Function(func) = &assembly_program.top_levels[0] {
            func
        } else {
            panic!("Expected function");
        };

        let instructions = &main_func.instructions;
        assert_eq!(instructions.len(), 16);

        assert!(matches!(
            &instructions[0],
            Mov {
                assembly_type: Longword,
                src: Immediate(ImmValue::Int(1)),
                dst: AsmOperand::PseudoReg(name)
            } if name == "tmp.0"
        ));
        assert!(matches!(
            &instructions[1],
            Binary {
                assembly_type: Longword,
                op: AsmBinaryOp::Add,
                left: Immediate(ImmValue::Int(2)),
                right: AsmOperand::PseudoReg(name)
            } if name == "tmp.0"
        ));

        assert!(matches!(
            &instructions[2],
            Mov {
                assembly_type: Longword,
                src: AsmOperand::PseudoReg(src),
                dst: AsmOperand::PseudoReg(dst)
            } if src == "tmp.0" && dst == "tmp.1"
        ));
        assert!(matches!(
            &instructions[3],
            Binary {
                assembly_type: Longword,
                op: AsmBinaryOp::Sub,
                left: Immediate(ImmValue::Int(3)),
                right: AsmOperand::PseudoReg(name),
            } if name == "tmp.1"
        ));

        assert!(matches!(
            &instructions[4],
            Mov {
                assembly_type: Longword,
                src: AsmOperand::PseudoReg(src),
                dst: AsmOperand::PseudoReg(dst)
            } if src == "tmp.1" && dst == "tmp.2"
        ));
        assert!(matches!(
            &instructions[5],
            Binary {
                assembly_type: Longword,
                op: AsmBinaryOp::Mul,
                left: Immediate(ImmValue::Int(4)),
                right: AsmOperand::PseudoReg(name)
            } if name == "tmp.2"
        ));

        assert!(matches!(
            &instructions[6],
            Mov {
                assembly_type: Longword,
                src: AsmOperand::PseudoReg(name),
                dst: AsmOperand::Register(AX)
            } if name == "tmp.2"
        ));
        assert!(matches!(&instructions[7], Cdq(Longword)));
        assert!(matches!(
            &instructions[8],
            Idiv {
                assembly_type: Longword,
                operand: Immediate(ImmValue::Int(5)),
            },
        ));
        assert!(matches!(
            &instructions[9],
            Mov {
                assembly_type: Longword,
                src: AsmOperand::Register(AX),
                dst: AsmOperand::PseudoReg(name)
            } if name == "tmp.3"
        ));

        assert!(matches!(
            &instructions[10],
            Mov {
                assembly_type: Longword,
                src: AsmOperand::PseudoReg(name),
                dst: AsmOperand::Register(AX)
            } if name == "tmp.3"
        ));
        assert!(matches!(&instructions[11], Cdq(Longword)));
        assert!(matches!(
            &instructions[12],
            Idiv {
                assembly_type: Longword,
                operand: Immediate(ImmValue::Int(2)),
            },
        ));
        assert!(matches!(
            &instructions[13],
            Mov {
                assembly_type: Longword,
                src: AsmOperand::Register(DX),
                dst: AsmOperand::PseudoReg(name)
            } if name == "tmp.4"
        ));

        assert!(matches!(
            &instructions[14],
            Mov {
                assembly_type: Longword,
                src: AsmOperand::PseudoReg(name),
                dst: AsmOperand::Register(AX)
            } if name == "tmp.4"
        ));
        assert!(matches!(&instructions[15], AsmInstruction::Ret));
    }
}

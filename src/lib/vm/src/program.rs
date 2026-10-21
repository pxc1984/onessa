//! An object file prepared for execution: the bytecode is validated and decoded once into a
//! flat array of [`Op`]s, so the interpreter never decodes bytes or looks names up.

use crate::error::LoadError;
use crate::native::Native;
use crate::value::Value;
use onessa_core::bytecode::{Constant, Instruction};
use onessa_core::object::ObjectFile;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

/// Instruction of the interpreter. Jump targets are indices into [`Program::ops`], constants
/// are indices into [`Program::constants`], natives are resolved.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Op {
    PushC(u32),
    PushV(u16),
    PopV(u16),
    PushG(u16),
    PopG(u16),
    Pop,
    Dup,
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Neg,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Not,
    Jump(u32),
    JumpIfFalse(u32),
    Call { function: u32, argc: u8 },
    CallNative { native: Native, argc: u8 },
    PushOut(u8),
    Return,
    TryBegin(u32),
    TryEnd,
    Raise,
    ReRaise,
    New { type_name: u32, argc: u8 },
    IndexGet,
    IndexSet,
    CallMethod { name: u32, argc: u8 },
}

pub struct FunctionInfo {
    /// Index of the first op.
    pub entry: usize,
    pub param_count: usize,
    pub local_count: usize,
}

pub struct Program {
    pub ops: Vec<Op>,
    /// Code section offset of every op, for source lines in error messages.
    pub op_offsets: Vec<u32>,
    pub constants: Vec<Value>,
    pub functions: Vec<FunctionInfo>,
    pub entry: u32,
    pub global_count: usize,
    pub strings: Vec<Rc<str>>,
    /// Lowercased strings, for case-insensitive type and method names.
    pub lowercase: Vec<Box<str>>,
}

impl Program {
    /// Checks everything the interpreter relies on without re-checking at run time:
    /// instructions decode, operands are in range, jumps stay inside their function, natives
    /// exist. Then translates the bytecode into ops.
    pub fn load(object: &ObjectFile) -> Result<Self, LoadError> {
        let strings: Vec<Rc<str>> = (0..object.string_table.len() as u32)
            .map(|idx| Rc::from(&*object.string_table.get(idx).unwrap_or_default()))
            .collect();
        let lowercase = strings.iter().map(|s| s.to_lowercase().into()).collect();
        let table = &object.function_table;
        let code = &object.code_section;
        let fail = |message: String| Err(LoadError(message));

        if table.entry as usize >= table.functions.len() {
            return fail(format!("entry function #{} does not exist", table.entry));
        }

        let mut program = Program {
            ops: Vec::new(),
            op_offsets: Vec::new(),
            constants: Vec::new(),
            functions: Vec::with_capacity(table.functions.len()),
            entry: table.entry,
            global_count: table.global_count as usize,
            strings,
            lowercase,
        };

        for (idx, function) in table.functions.iter().enumerate() {
            let start = function.code_offset as usize;
            let end = start + function.code_len as usize;
            if end > code.code.len() || u16::from(function.param_count) > function.local_count {
                return fail(format!("function #{idx} has an invalid layout"));
            }
            let instructions = code
                .instructions(start, end)
                .map_err(|e| LoadError(format!("function #{idx}: {e}")))?;
            if !matches!(
                instructions.last(),
                Some((
                    _,
                    Instruction::Return
                        | Instruction::Raise
                        | Instruction::ReRaise
                        | Instruction::Jump(_)
                ))
            ) {
                return fail(format!("function #{idx} does not end with a return"));
            }

            // Ops of this function start right after the previous ones.
            let first_op = program.ops.len();
            let op_index: HashMap<usize, u32> = instructions
                .iter()
                .enumerate()
                .map(|(i, (offset, _))| (*offset, (first_op + i) as u32))
                .collect();
            let starts: HashSet<usize> = op_index.keys().copied().collect();

            program.functions.push(FunctionInfo {
                entry: first_op,
                param_count: function.param_count as usize,
                local_count: function.local_count as usize,
            });
            for (offset, instruction) in instructions {
                let bad = |what: &str| LoadError(format!("function #{idx} at {offset}: {what}"));
                let target = |target: u32| {
                    if starts.contains(&(target as usize)) {
                        Ok(op_index[&(target as usize)])
                    } else {
                        Err(bad("jump target is not an instruction of this function"))
                    }
                };
                let string = |s: u32| {
                    if (s as usize) < program.strings.len() {
                        Ok(s)
                    } else {
                        Err(bad("string index out of range"))
                    }
                };
                let local = |i: u16| {
                    if i < function.local_count {
                        Ok(i)
                    } else {
                        Err(bad("local index out of range"))
                    }
                };
                let global = |i: u16| {
                    if i < table.global_count {
                        Ok(i)
                    } else {
                        Err(bad("global index out of range"))
                    }
                };

                let op = match instruction {
                    Instruction::PushC(constant) => {
                        let value = match constant {
                            Constant::Undefined => Value::Undefined,
                            Constant::Null => Value::Null,
                            Constant::Boolean(b) => Value::Boolean(b),
                            Constant::Number(n) => Value::number(n),
                            Constant::String(s) => {
                                Value::String(program.strings[string(s)? as usize].clone())
                            }
                            Constant::Date(secs) => Value::date_from_timestamp(secs)
                                .unwrap_or(Value::Date(Value::empty_date())),
                        };
                        program.constants.push(value);
                        Op::PushC(program.constants.len() as u32 - 1)
                    }
                    Instruction::PushV(i) => Op::PushV(local(i)?),
                    Instruction::PopV(i) => Op::PopV(local(i)?),
                    Instruction::PushG(i) => Op::PushG(global(i)?),
                    Instruction::PopG(i) => Op::PopG(global(i)?),
                    Instruction::Pop => Op::Pop,
                    Instruction::Dup => Op::Dup,
                    Instruction::Add => Op::Add,
                    Instruction::Sub => Op::Sub,
                    Instruction::Mul => Op::Mul,
                    Instruction::Div => Op::Div,
                    Instruction::Mod => Op::Mod,
                    Instruction::Neg => Op::Neg,
                    Instruction::Eq => Op::Eq,
                    Instruction::Ne => Op::Ne,
                    Instruction::Lt => Op::Lt,
                    Instruction::Le => Op::Le,
                    Instruction::Gt => Op::Gt,
                    Instruction::Ge => Op::Ge,
                    Instruction::Not => Op::Not,
                    Instruction::Jump(t) => Op::Jump(target(t)?),
                    Instruction::JumpIfFalse(t) => Op::JumpIfFalse(target(t)?),
                    Instruction::Call {
                        function: callee,
                        argc,
                    } => match table.functions.get(callee as usize) {
                        Some(f) if f.param_count == argc => Op::Call {
                            function: callee,
                            argc,
                        },
                        _ => return Err(bad("call to a missing function or with wrong arity")),
                    },
                    Instruction::CallNative { name, argc } => {
                        let native = Native::resolve(&program.strings[string(name)? as usize])
                            .ok_or_else(|| bad("unknown native function"))?;
                        Op::CallNative { native, argc }
                    }
                    Instruction::PushOut(n) => Op::PushOut(n),
                    Instruction::Return => Op::Return,
                    Instruction::TryBegin(t) => Op::TryBegin(target(t)?),
                    Instruction::TryEnd => Op::TryEnd,
                    Instruction::Raise => Op::Raise,
                    Instruction::ReRaise => Op::ReRaise,
                    Instruction::New { type_name, argc } => Op::New {
                        type_name: string(type_name)?,
                        argc,
                    },
                    Instruction::IndexGet => Op::IndexGet,
                    Instruction::IndexSet => Op::IndexSet,
                    Instruction::CallMethod { name, argc } => Op::CallMethod {
                        name: string(name)?,
                        argc,
                    },
                };
                program.ops.push(op);
                program.op_offsets.push(offset as u32);
            }
        }
        Ok(program)
    }
}

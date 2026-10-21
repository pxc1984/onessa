use crate::bytecode::{Constant, ValueType};
use rust_decimal::Decimal;
use std::fmt::{Display, Formatter};

/// A single VM instruction.
///
/// `L` is the type of jump targets: byte offsets inside the code section (`u32`) in the final
/// bytecode, basic block ids while the code is still a control flow graph.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Instruction<L = u32> {
    // Category 1: Stack management
    PushC(Constant),
    PushV(u16),
    PopV(u16),
    PushG(u16),
    PopG(u16),
    Pop,
    Dup,
    /// Raises an exception unless the value on top of the stack has the type (the value stays
    /// on the stack). Emitted in strict mode where a value of an unknown type is stored into
    /// a typed variable.
    CheckType(ValueType),

    // Category 2: Arithmetic, comparison and logic
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

    // Category 3: Control flow
    Jump(L),
    JumpIfFalse(L),
    /// Before the first iteration of `Для`: jumps to the target (the loop exit) unless
    /// local `counter <= limit`.
    ForCheck {
        counter: u16,
        limit: u16,
        target: L,
    },
    /// After an iteration of `Для`: increments local `counter` by one and jumps to the target
    /// (the loop body) if `counter <= limit`.
    ForStep {
        counter: u16,
        limit: u16,
        target: L,
    },
    Call {
        function: u32,
        argc: u8,
    },
    CallNative {
        name: u32,
        argc: u8,
    },
    /// Pushes the final value of by-reference parameter `n` of the call that just returned.
    PushOut(u8),
    Return,

    // Category 4: Exceptions
    TryBegin(L),
    TryEnd,
    Raise,
    ReRaise,

    // Category 5: Objects
    /// Creates an object of the type named by string `type_name` from `argc` constructor
    /// arguments.
    New {
        type_name: u32,
        argc: u8,
    },
    /// Pops the index and the collection, pushes the element.
    IndexGet,
    /// Pops the value, the index and the collection, stores the element.
    IndexSet,
    /// Pops `argc` arguments and the object, calls the method named by string `name` and
    /// pushes its result (`Неопределено` for procedures).
    CallMethod {
        name: u32,
        argc: u8,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    UnexpectedEnd(usize),
    UnknownOpcode { offset: usize, opcode: u8 },
    UnknownConstant { offset: usize, tag: u8 },
    UnknownValueType { offset: usize, tag: u8 },
}

impl Display for DecodeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            DecodeError::UnexpectedEnd(offset) => {
                write!(f, "unexpected end of code at offset {offset}")
            }
            DecodeError::UnknownOpcode { offset, opcode } => {
                write!(f, "unknown opcode {opcode:#04x} at offset {offset}")
            }
            DecodeError::UnknownConstant { offset, tag } => {
                write!(f, "unknown constant tag {tag} at offset {offset}")
            }
            DecodeError::UnknownValueType { offset, tag } => {
                write!(f, "unknown value type {tag} at offset {offset}")
            }
        }
    }
}

impl std::error::Error for DecodeError {}

mod opcode {
    pub const PUSH_C: u8 = 0x01;
    pub const PUSH_V: u8 = 0x02;
    pub const POP_V: u8 = 0x03;
    pub const PUSH_G: u8 = 0x04;
    pub const POP_G: u8 = 0x05;
    pub const POP: u8 = 0x06;
    pub const DUP: u8 = 0x07;
    pub const CHECK_TYPE: u8 = 0x08;

    pub const ADD: u8 = 0x10;
    pub const SUB: u8 = 0x11;
    pub const MUL: u8 = 0x12;
    pub const DIV: u8 = 0x13;
    pub const MOD: u8 = 0x14;
    pub const NEG: u8 = 0x15;
    pub const EQ: u8 = 0x16;
    pub const NE: u8 = 0x17;
    pub const LT: u8 = 0x18;
    pub const LE: u8 = 0x19;
    pub const GT: u8 = 0x1A;
    pub const GE: u8 = 0x1B;
    pub const NOT: u8 = 0x1C;

    pub const JUMP: u8 = 0x20;
    pub const JUMP_IF_FALSE: u8 = 0x21;
    pub const FOR_CHECK: u8 = 0x26;
    pub const FOR_STEP: u8 = 0x27;
    pub const CALL: u8 = 0x22;
    pub const CALL_NATIVE: u8 = 0x23;
    pub const RETURN: u8 = 0x24;
    pub const PUSH_OUT: u8 = 0x25;

    pub const TRY_BEGIN: u8 = 0x30;
    pub const TRY_END: u8 = 0x31;
    pub const RAISE: u8 = 0x32;
    pub const RERAISE: u8 = 0x33;

    pub const NEW: u8 = 0x40;
    pub const INDEX_GET: u8 = 0x41;
    pub const INDEX_SET: u8 = 0x42;
    pub const CALL_METHOD: u8 = 0x43;
}

impl<L> Instruction<L> {
    /// Converts jump targets with `f`, leaving every other instruction untouched.
    pub fn map_target<M>(self, mut f: impl FnMut(L) -> M) -> Instruction<M> {
        use Instruction::*;
        match self {
            PushC(c) => PushC(c),
            PushV(i) => PushV(i),
            PopV(i) => PopV(i),
            PushG(i) => PushG(i),
            PopG(i) => PopG(i),
            Pop => Pop,
            Dup => Dup,
            CheckType(t) => CheckType(t),
            Add => Add,
            Sub => Sub,
            Mul => Mul,
            Div => Div,
            Mod => Mod,
            Neg => Neg,
            Eq => Eq,
            Ne => Ne,
            Lt => Lt,
            Le => Le,
            Gt => Gt,
            Ge => Ge,
            Not => Not,
            Jump(l) => Jump(f(l)),
            JumpIfFalse(l) => JumpIfFalse(f(l)),
            ForCheck {
                counter,
                limit,
                target,
            } => ForCheck {
                counter,
                limit,
                target: f(target),
            },
            ForStep {
                counter,
                limit,
                target,
            } => ForStep {
                counter,
                limit,
                target: f(target),
            },
            Call { function, argc } => Call { function, argc },
            CallNative { name, argc } => CallNative { name, argc },
            PushOut(n) => PushOut(n),
            Return => Return,
            TryBegin(l) => TryBegin(f(l)),
            TryEnd => TryEnd,
            Raise => Raise,
            ReRaise => ReRaise,
            New { type_name, argc } => New { type_name, argc },
            IndexGet => IndexGet,
            IndexSet => IndexSet,
            CallMethod { name, argc } => CallMethod { name, argc },
        }
    }
}

impl Instruction<u32> {
    /// Appends the binary encoding of the instruction. All operands are little endian.
    pub fn encode(&self, out: &mut Vec<u8>) {
        use Instruction::*;
        match *self {
            PushC(constant) => {
                out.push(opcode::PUSH_C);
                match constant {
                    Constant::Undefined => out.push(Constant::TAG_UNDEFINED),
                    Constant::Null => out.push(Constant::TAG_NULL),
                    Constant::Boolean(false) => out.push(Constant::TAG_FALSE),
                    Constant::Boolean(true) => out.push(Constant::TAG_TRUE),
                    Constant::Number(n) => {
                        out.push(Constant::TAG_NUMBER);
                        // Decimal is 128 bits: lo (32) + mid (32) + hi (32) + scale (32)
                        let unpacked = n.unpack();
                        out.extend_from_slice(&unpacked.lo.to_le_bytes());
                        out.extend_from_slice(&unpacked.mid.to_le_bytes());
                        out.extend_from_slice(&unpacked.hi.to_le_bytes());
                        out.extend_from_slice(&unpacked.scale.to_le_bytes());
                        out.push(if unpacked.negative { 1 } else { 0 });
                    }
                    Constant::String(idx) => {
                        out.push(Constant::TAG_STRING);
                        out.extend_from_slice(&idx.to_le_bytes());
                    }
                    Constant::Date(secs) => {
                        out.push(Constant::TAG_DATE);
                        out.extend_from_slice(&secs.to_le_bytes());
                    }
                }
            }
            PushV(i) => encode_u16(out, opcode::PUSH_V, i),
            PopV(i) => encode_u16(out, opcode::POP_V, i),
            PushG(i) => encode_u16(out, opcode::PUSH_G, i),
            PopG(i) => encode_u16(out, opcode::POP_G, i),
            Pop => out.push(opcode::POP),
            Dup => out.push(opcode::DUP),
            CheckType(t) => {
                out.push(opcode::CHECK_TYPE);
                out.push(t.tag());
            }
            Add => out.push(opcode::ADD),
            Sub => out.push(opcode::SUB),
            Mul => out.push(opcode::MUL),
            Div => out.push(opcode::DIV),
            Mod => out.push(opcode::MOD),
            Neg => out.push(opcode::NEG),
            Eq => out.push(opcode::EQ),
            Ne => out.push(opcode::NE),
            Lt => out.push(opcode::LT),
            Le => out.push(opcode::LE),
            Gt => out.push(opcode::GT),
            Ge => out.push(opcode::GE),
            Not => out.push(opcode::NOT),
            Jump(target) => encode_u32(out, opcode::JUMP, target),
            JumpIfFalse(target) => encode_u32(out, opcode::JUMP_IF_FALSE, target),
            ForCheck {
                counter,
                limit,
                target,
            } => encode_for(out, opcode::FOR_CHECK, counter, limit, target),
            ForStep {
                counter,
                limit,
                target,
            } => encode_for(out, opcode::FOR_STEP, counter, limit, target),
            Call { function, argc } => {
                encode_u32(out, opcode::CALL, function);
                out.push(argc);
            }
            CallNative { name, argc } => {
                encode_u32(out, opcode::CALL_NATIVE, name);
                out.push(argc);
            }
            PushOut(n) => {
                out.push(opcode::PUSH_OUT);
                out.push(n);
            }
            Return => out.push(opcode::RETURN),
            TryBegin(target) => encode_u32(out, opcode::TRY_BEGIN, target),
            TryEnd => out.push(opcode::TRY_END),
            Raise => out.push(opcode::RAISE),
            ReRaise => out.push(opcode::RERAISE),
            New { type_name, argc } => {
                encode_u32(out, opcode::NEW, type_name);
                out.push(argc);
            }
            IndexGet => out.push(opcode::INDEX_GET),
            IndexSet => out.push(opcode::INDEX_SET),
            CallMethod { name, argc } => {
                encode_u32(out, opcode::CALL_METHOD, name);
                out.push(argc);
            }
        }
    }

    /// Size of the encoded instruction in bytes.
    pub fn encoded_len(&self) -> usize {
        use Instruction::*;
        match self {
            PushC(Constant::Number(_)) => 19, // 1 opcode + 1 tag + 17 bytes for Decimal (16 + 1 sign)
            PushC(Constant::Date(_)) => 10,
            PushC(Constant::String(_)) => 6,
            PushC(_) | PushOut(_) | CheckType(_) => 2,
            PushV(_) | PopV(_) | PushG(_) | PopG(_) => 3,
            Jump(_) | JumpIfFalse(_) | TryBegin(_) => 5,
            ForCheck { .. } | ForStep { .. } => 9,
            Call { .. } | CallNative { .. } | New { .. } | CallMethod { .. } => 6,
            _ => 1,
        }
    }

    /// Decodes the instruction at `*pc` and advances `pc` past it.
    pub fn decode(code: &[u8], pc: &mut usize) -> Result<Self, DecodeError> {
        use Instruction::*;
        let offset = *pc;
        let op = read_u8(code, pc)?;
        let instruction = match op {
            opcode::PUSH_C => {
                let tag = read_u8(code, pc)?;
                PushC(match tag {
                    Constant::TAG_UNDEFINED => Constant::Undefined,
                    Constant::TAG_NULL => Constant::Null,
                    Constant::TAG_FALSE => Constant::Boolean(false),
                    Constant::TAG_TRUE => Constant::Boolean(true),
                    Constant::TAG_NUMBER => {
                        // Decimal is 128 bits: lo (32) + mid (32) + hi (32) + scale (32) + sign (8)
                        let lo = u32::from_le_bytes(read_array(code, pc)?);
                        let mid = u32::from_le_bytes(read_array(code, pc)?);
                        let hi = u32::from_le_bytes(read_array(code, pc)?);
                        let scale = u32::from_le_bytes(read_array(code, pc)?);
                        let negative = read_u8(code, pc)? != 0;
                        Constant::Number(Decimal::from_parts(lo, mid, hi, negative, scale))
                    }
                    Constant::TAG_STRING => Constant::String(read_u32(code, pc)?),
                    Constant::TAG_DATE => Constant::Date(i64::from_le_bytes(read_array(code, pc)?)),
                    tag => return Err(DecodeError::UnknownConstant { offset, tag }),
                })
            }
            opcode::PUSH_V => PushV(read_u16(code, pc)?),
            opcode::POP_V => PopV(read_u16(code, pc)?),
            opcode::PUSH_G => PushG(read_u16(code, pc)?),
            opcode::POP_G => PopG(read_u16(code, pc)?),
            opcode::POP => Pop,
            opcode::DUP => Dup,
            opcode::CHECK_TYPE => {
                let tag = read_u8(code, pc)?;
                CheckType(
                    ValueType::from_tag(tag)
                        .ok_or(DecodeError::UnknownValueType { offset, tag })?,
                )
            }
            opcode::ADD => Add,
            opcode::SUB => Sub,
            opcode::MUL => Mul,
            opcode::DIV => Div,
            opcode::MOD => Mod,
            opcode::NEG => Neg,
            opcode::EQ => Eq,
            opcode::NE => Ne,
            opcode::LT => Lt,
            opcode::LE => Le,
            opcode::GT => Gt,
            opcode::GE => Ge,
            opcode::NOT => Not,
            opcode::JUMP => Jump(read_u32(code, pc)?),
            opcode::JUMP_IF_FALSE => JumpIfFalse(read_u32(code, pc)?),
            opcode::FOR_CHECK => ForCheck {
                counter: read_u16(code, pc)?,
                limit: read_u16(code, pc)?,
                target: read_u32(code, pc)?,
            },
            opcode::FOR_STEP => ForStep {
                counter: read_u16(code, pc)?,
                limit: read_u16(code, pc)?,
                target: read_u32(code, pc)?,
            },
            opcode::CALL => Call {
                function: read_u32(code, pc)?,
                argc: read_u8(code, pc)?,
            },
            opcode::CALL_NATIVE => CallNative {
                name: read_u32(code, pc)?,
                argc: read_u8(code, pc)?,
            },
            opcode::PUSH_OUT => PushOut(read_u8(code, pc)?),
            opcode::RETURN => Return,
            opcode::TRY_BEGIN => TryBegin(read_u32(code, pc)?),
            opcode::TRY_END => TryEnd,
            opcode::RAISE => Raise,
            opcode::RERAISE => ReRaise,
            opcode::NEW => New {
                type_name: read_u32(code, pc)?,
                argc: read_u8(code, pc)?,
            },
            opcode::INDEX_GET => IndexGet,
            opcode::INDEX_SET => IndexSet,
            opcode::CALL_METHOD => CallMethod {
                name: read_u32(code, pc)?,
                argc: read_u8(code, pc)?,
            },
            opcode => return Err(DecodeError::UnknownOpcode { offset, opcode }),
        };
        Ok(instruction)
    }
}

impl<L: Display> Display for Instruction<L> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use Instruction::*;
        match self {
            PushC(c) => write!(f, "PUSH_C {c}"),
            PushV(i) => write!(f, "PUSH_V {i}"),
            PopV(i) => write!(f, "POP_V {i}"),
            PushG(i) => write!(f, "PUSH_G {i}"),
            PopG(i) => write!(f, "POP_G {i}"),
            Pop => write!(f, "POP"),
            Dup => write!(f, "DUP"),
            CheckType(t) => write!(f, "CHECK_TYPE {t}"),
            Add => write!(f, "ADD"),
            Sub => write!(f, "SUB"),
            Mul => write!(f, "MUL"),
            Div => write!(f, "DIV"),
            Mod => write!(f, "MOD"),
            Neg => write!(f, "NEG"),
            Eq => write!(f, "EQ"),
            Ne => write!(f, "NE"),
            Lt => write!(f, "LT"),
            Le => write!(f, "LE"),
            Gt => write!(f, "GT"),
            Ge => write!(f, "GE"),
            Not => write!(f, "NOT"),
            Jump(l) => write!(f, "JUMP {l}"),
            JumpIfFalse(l) => write!(f, "JUMP_IF_FALSE {l}"),
            ForCheck {
                counter,
                limit,
                target,
            } => write!(f, "FOR_CHECK {counter} {limit} {target}"),
            ForStep {
                counter,
                limit,
                target,
            } => write!(f, "FOR_STEP {counter} {limit} {target}"),
            Call { function, argc } => write!(f, "CALL fn#{function} {argc}"),
            CallNative { name, argc } => write!(f, "CALL_NATIVE str#{name} {argc}"),
            PushOut(n) => write!(f, "PUSH_OUT {n}"),
            Return => write!(f, "RETURN"),
            TryBegin(l) => write!(f, "TRY_BEGIN {l}"),
            TryEnd => write!(f, "TRY_END"),
            Raise => write!(f, "RAISE"),
            ReRaise => write!(f, "RERAISE"),
            New { type_name, argc } => write!(f, "NEW str#{type_name} {argc}"),
            IndexGet => write!(f, "INDEX_GET"),
            IndexSet => write!(f, "INDEX_SET"),
            CallMethod { name, argc } => write!(f, "CALL_METHOD str#{name} {argc}"),
        }
    }
}

fn encode_u16(out: &mut Vec<u8>, op: u8, value: u16) {
    out.push(op);
    out.extend_from_slice(&value.to_le_bytes());
}

fn encode_for(out: &mut Vec<u8>, op: u8, counter: u16, limit: u16, target: u32) {
    out.push(op);
    out.extend_from_slice(&counter.to_le_bytes());
    out.extend_from_slice(&limit.to_le_bytes());
    out.extend_from_slice(&target.to_le_bytes());
}

fn encode_u32(out: &mut Vec<u8>, op: u8, value: u32) {
    out.push(op);
    out.extend_from_slice(&value.to_le_bytes());
}

fn read_array<const N: usize>(code: &[u8], pc: &mut usize) -> Result<[u8; N], DecodeError> {
    let bytes = code
        .get(*pc..*pc + N)
        .ok_or(DecodeError::UnexpectedEnd(*pc))?;
    *pc += N;
    Ok(bytes.try_into().expect("slice has exactly N bytes"))
}

fn read_u8(code: &[u8], pc: &mut usize) -> Result<u8, DecodeError> {
    Ok(read_array::<1>(code, pc)?[0])
}

fn read_u16(code: &[u8], pc: &mut usize) -> Result<u16, DecodeError> {
    Ok(u16::from_le_bytes(read_array(code, pc)?))
}

fn read_u32(code: &[u8], pc: &mut usize) -> Result<u32, DecodeError> {
    Ok(u32::from_le_bytes(read_array(code, pc)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_all_instructions() {
        use Instruction::*;
        let instructions: Vec<Instruction> = vec![
            PushC(Constant::Undefined),
            PushC(Constant::Null),
            PushC(Constant::Boolean(true)),
            PushC(Constant::Boolean(false)),
            PushC(Constant::Number(Decimal::new(-125, 1))), // -12.5
            PushC(Constant::String(7)),
            PushC(Constant::Date(1_700_000_000)),
            PushV(3),
            PopV(4),
            PushG(65535),
            PopG(1),
            Pop,
            Dup,
            CheckType(ValueType::Number),
            CheckType(ValueType::Array),
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
            Jump(42),
            JumpIfFalse(u32::MAX),
            ForCheck {
                counter: 1,
                limit: 2,
                target: 30,
            },
            ForStep {
                counter: 3,
                limit: 4,
                target: 7,
            },
            Call {
                function: 2,
                argc: 3,
            },
            CallNative { name: 9, argc: 255 },
            PushOut(1),
            Return,
            TryBegin(100),
            TryEnd,
            Raise,
            ReRaise,
            New {
                type_name: 4,
                argc: 1,
            },
            IndexGet,
            IndexSet,
            CallMethod { name: 5, argc: 2 },
        ];

        let mut code = Vec::new();
        for instruction in &instructions {
            let before = code.len();
            instruction.encode(&mut code);
            assert_eq!(
                code.len() - before,
                instruction.encoded_len(),
                "{instruction}"
            );
        }

        let mut pc = 0;
        let mut decoded = Vec::new();
        while pc < code.len() {
            decoded.push(Instruction::decode(&code, &mut pc).unwrap());
        }
        assert_eq!(decoded, instructions);
    }

    #[test]
    fn decode_errors() {
        let mut pc = 0;
        assert_eq!(
            Instruction::decode(&[0xFF], &mut pc),
            Err(DecodeError::UnknownOpcode {
                offset: 0,
                opcode: 0xFF
            })
        );
        let mut pc = 0;
        assert_eq!(
            Instruction::decode(&[opcode::JUMP, 1, 2], &mut pc),
            Err(DecodeError::UnexpectedEnd(1))
        );
    }
}

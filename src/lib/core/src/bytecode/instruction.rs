use crate::bytecode::Constant;
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
    Call { function: u32, argc: u8 },
    CallNative { name: u32, argc: u8 },
    Return,

    // Category 4: Exceptions
    TryBegin(L),
    TryEnd,
    Raise,
    ReRaise,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    UnexpectedEnd(usize),
    UnknownOpcode { offset: usize, opcode: u8 },
    UnknownConstant { offset: usize, tag: u8 },
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
    pub const CALL: u8 = 0x22;
    pub const CALL_NATIVE: u8 = 0x23;
    pub const RETURN: u8 = 0x24;

    pub const TRY_BEGIN: u8 = 0x30;
    pub const TRY_END: u8 = 0x31;
    pub const RAISE: u8 = 0x32;
    pub const RERAISE: u8 = 0x33;
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
            Call { function, argc } => Call { function, argc },
            CallNative { name, argc } => CallNative { name, argc },
            Return => Return,
            TryBegin(l) => TryBegin(f(l)),
            TryEnd => TryEnd,
            Raise => Raise,
            ReRaise => ReRaise,
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
                        out.extend_from_slice(&n.to_le_bytes());
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
            Call { function, argc } => {
                encode_u32(out, opcode::CALL, function);
                out.push(argc);
            }
            CallNative { name, argc } => {
                encode_u32(out, opcode::CALL_NATIVE, name);
                out.push(argc);
            }
            Return => out.push(opcode::RETURN),
            TryBegin(target) => encode_u32(out, opcode::TRY_BEGIN, target),
            TryEnd => out.push(opcode::TRY_END),
            Raise => out.push(opcode::RAISE),
            ReRaise => out.push(opcode::RERAISE),
        }
    }

    /// Size of the encoded instruction in bytes.
    pub fn encoded_len(&self) -> usize {
        use Instruction::*;
        match self {
            PushC(Constant::Number(_)) | PushC(Constant::Date(_)) => 10,
            PushC(Constant::String(_)) => 6,
            PushC(_) => 2,
            PushV(_) | PopV(_) | PushG(_) | PopG(_) => 3,
            Jump(_) | JumpIfFalse(_) | TryBegin(_) => 5,
            Call { .. } | CallNative { .. } => 6,
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
                        Constant::Number(f64::from_le_bytes(read_array(code, pc)?))
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
            opcode::CALL => Call {
                function: read_u32(code, pc)?,
                argc: read_u8(code, pc)?,
            },
            opcode::CALL_NATIVE => CallNative {
                name: read_u32(code, pc)?,
                argc: read_u8(code, pc)?,
            },
            opcode::RETURN => Return,
            opcode::TRY_BEGIN => TryBegin(read_u32(code, pc)?),
            opcode::TRY_END => TryEnd,
            opcode::RAISE => Raise,
            opcode::RERAISE => ReRaise,
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
            Call { function, argc } => write!(f, "CALL fn#{function} {argc}"),
            CallNative { name, argc } => write!(f, "CALL_NATIVE str#{name} {argc}"),
            Return => write!(f, "RETURN"),
            TryBegin(l) => write!(f, "TRY_BEGIN {l}"),
            TryEnd => write!(f, "TRY_END"),
            Raise => write!(f, "RAISE"),
            ReRaise => write!(f, "RERAISE"),
        }
    }
}

fn encode_u16(out: &mut Vec<u8>, op: u8, value: u16) {
    out.push(op);
    out.extend_from_slice(&value.to_le_bytes());
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
            PushC(Constant::Number(-12.5)),
            PushC(Constant::String(7)),
            PushC(Constant::Date(1_700_000_000)),
            PushV(3),
            PopV(4),
            PushG(65535),
            PopG(1),
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
            Jump(42),
            JumpIfFalse(u32::MAX),
            Call { function: 2, argc: 3 },
            CallNative { name: 9, argc: 255 },
            Return,
            TryBegin(100),
            TryEnd,
            Raise,
            ReRaise,
        ];

        let mut code = Vec::new();
        for instruction in &instructions {
            let before = code.len();
            instruction.encode(&mut code);
            assert_eq!(code.len() - before, instruction.encoded_len(), "{instruction}");
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
            Err(DecodeError::UnknownOpcode { offset: 0, opcode: 0xFF })
        );
        let mut pc = 0;
        assert_eq!(
            Instruction::decode(&[opcode::JUMP, 1, 2], &mut pc),
            Err(DecodeError::UnexpectedEnd(1))
        );
    }
}

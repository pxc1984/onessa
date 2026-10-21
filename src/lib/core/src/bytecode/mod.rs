//! Instruction set of the Onessa stack virtual machine.
//!
//! See `BYTECODE.md` next to this file for the description of every instruction.

mod constant;
mod instruction;
mod value_type;

pub use constant::Constant;
pub use instruction::{DecodeError, Instruction};
pub use value_type::ValueType;

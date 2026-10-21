//! Instruction set of the Onessa stack virtual machine.
//!
//! See `BYTECODE.md` next to this file for the description of every instruction.

mod constant;
mod instruction;

pub use constant::Constant;
pub use instruction::{DecodeError, Instruction};

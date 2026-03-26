use chrono::{DateTime, Utc};
use onessa_core::object::ObjectFile;
use crate::cli::RunArgs;

#[repr(u16)]
pub enum Instruction {
    // Category 1: Stack management
    PushC,
    PushV,
    PopV,
    Pop,
    Dup,
}

pub enum Variable {
    Int(f64),
    String(String),
    Date(DateTime<Utc>),
}

pub struct VirtualMachine {
    pub stack: Vec<Variable>,
    pub variables: Vec<Variable>,
}

pub fn run(args: RunArgs) -> Option<i32> {
    let mut obj = ObjectFile::from_file(args.input).ok()?;
    todo!()
}

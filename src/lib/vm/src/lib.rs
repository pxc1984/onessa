//! Stack virtual machine executing Onessa object files.

mod error;
mod native;
mod value;
mod vm;

pub use error::{LoadError, RuntimeError};
pub use value::Value;
pub use vm::Vm;

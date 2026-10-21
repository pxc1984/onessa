//! Stack virtual machine executing Onessa object files.

mod error;
mod format;
mod native;
mod objects;
mod value;
mod vm;

pub use error::{LoadError, RuntimeError};
pub use value::Value;
pub use vm::Vm;

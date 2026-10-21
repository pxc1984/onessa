use crate::cli::RunArgs;
use log::error;
use onessa_core::object::ObjectFile;

pub fn run(args: RunArgs) -> i32 {
    let _obj = match ObjectFile::from_file(&args.input) {
        Ok(obj) => obj,
        Err(err) => {
            error!("Failed to load {}: {err}", args.input.display());
            return 1;
        }
    };
    todo!("virtual machine")
}

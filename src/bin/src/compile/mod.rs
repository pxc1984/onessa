use crate::cli::CompileArgs;
use onessa_core::io::Source;
use std::process::exit;
use tracing::{debug, error};

pub fn compile(args: CompileArgs) {
    let source = Source::open(args.input).unwrap_or_else(|err| {
        error!("Failed to open source file: {}", err);
        exit(1);
    });
    debug!("Opened source file {:?}", source.path_buf);
    todo!()
}
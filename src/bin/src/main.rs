extern crate core;

use std::process::exit;
use clap::Parser;
use log::debug;
use onessa_logging::init_logging;

use crate::cli::{CLIArgs, Command};
use crate::compile::compile;
use crate::run::{disasm, run};

pub mod cli;
pub mod compile;

mod opt;
mod run;

fn main() {
    let cli_args = CLIArgs::parse();

    init_logging(cli_args.log.into());

    debug!("CLI Args: {cli_args:?}");

    match cli_args.command {
        Command::Compile(args) => compile(args),
        Command::Build(args) => compile(args),
        Command::Run(args) => exit(run(args)),
        Command::Disasm(args) => exit(disasm(args)),
    }
}

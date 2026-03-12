use clap::Parser;
use onessa_logging::init_logging;
use tracing::debug;

use crate::cli::CLIArgs;

mod cli;

fn main() {
    let cli_args = CLIArgs::parse();

    init_logging(cli_args.log.into());

    debug!("CLI Args: {cli_args:?}");
}

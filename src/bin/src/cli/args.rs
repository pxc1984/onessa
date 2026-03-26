use clap::builder::PossibleValue;
use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;
use tracing::Level;

#[derive(Parser, Debug)]
pub struct CLIArgs {
    #[command(subcommand)]
    pub command: Command,

    #[clap(long)]
    #[arg(value_enum)]
    #[cfg_attr(debug_assertions, arg(default_value_t = LogLevel(Level::DEBUG)))]
    #[cfg_attr(not(debug_assertions), arg(default_value_t = LogLevel(Level::INFO)))]
    pub log: LogLevel,
}

#[derive(Subcommand, Clone, Debug)]
pub enum Command {
    Compile(CompileArgs),
    Run(RunArgs),
}

#[derive(Debug, Clone, Parser)]
pub struct RunArgs {
    // path to bytecode file
    pub input: PathBuf,

    // TODO: add ability to pass cli arguments to the command by placing -- after onessa run
    //      example: onessa run -- --help
    //                             ------ arguments for a program
}

#[derive(Debug, Clone, Parser)]
pub struct CompileArgs {
    pub input: Option<PathBuf>,

    #[clap(long, short)]
    pub output: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy)]
pub struct LogLevel(pub Level);

impl ValueEnum for LogLevel {
    fn value_variants<'a>() -> &'a [Self] {
        static VARIANTS: &[LogLevel] = &[
            LogLevel(Level::TRACE),
            LogLevel(Level::DEBUG),
            LogLevel(Level::INFO),
            LogLevel(Level::WARN),
            LogLevel(Level::ERROR),
        ];
        VARIANTS
    }

    fn to_possible_value(&self) -> Option<PossibleValue> {
        match self.0 {
            Level::TRACE => Some(PossibleValue::new("trace")),
            Level::DEBUG => Some(PossibleValue::new("debug")),
            Level::INFO => Some(PossibleValue::new("info")),
            Level::WARN => Some(PossibleValue::new("warn")),
            Level::ERROR => Some(PossibleValue::new("error")),
        }
    }
}

impl From<LogLevel> for Level {
    fn from(value: LogLevel) -> Self {
        value.0
    }
}

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
    /// Compile a module into an object file (`.osc`)
    Compile(CompileArgs),
    /// Run an object file, or compile and run a source file
    Run(RunArgs),
    /// Print the bytecode of an object or source file
    Disasm(DisasmArgs),
}

#[derive(Debug, Clone, Parser)]
pub struct DisasmArgs {
    pub input: PathBuf,

    /// Compile a source file with static typing
    #[clap(long)]
    pub strict: bool,
}

#[derive(Debug, Clone, Parser)]
pub struct RunArgs {
    /// Object file (or source file to compile first)
    pub input: PathBuf,

    /// Compile a source file with static typing: a variable keeps the type of the value
    /// first assigned to it
    #[clap(long)]
    pub strict: bool,

    // TODO: add ability to pass cli arguments to the command by placing -- after onessa run
    //      example: onessa run -- --help
    //                             ------ arguments for a program
}

#[derive(Debug, Clone, Parser)]
pub struct CompileArgs {
    /// Source file; stdin if omitted
    pub input: Option<PathBuf>,

    /// Output object file; defaults to the input path with the `.osc` extension
    #[clap(long, short)]
    pub output: Option<PathBuf>,

    /// Static typing: a variable keeps the type of the value first assigned to it
    #[clap(long)]
    pub strict: bool,
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

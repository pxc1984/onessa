use crate::cli::CompileArgs;
use log::{error, info};
use onessa_core::object::ObjectFile;
use std::path::{Path, PathBuf};
use std::process::exit;

mod build_ast;
mod build_cfg;
mod codegen;
mod lexer;
mod source;

pub fn compile(args: CompileArgs) {
    let output = match (&args.output, &args.input) {
        (Some(output), _) => output.clone(),
        (None, Some(input)) => input.with_extension("osc"),
        (None, None) => {
            error!("--output is required when compiling from stdin");
            exit(1);
        }
    };

    let object = compile_file(args.input, args.strict);
    if let Err(err) = object.write_file(&output) {
        error!("Failed to write {}: {err}", output.display());
        exit(1);
    }
    info!("Wrote {}", output.display());
}

/// Runs the whole pipeline for a source file (stdin if `None`), exiting on errors.
pub fn compile_file(input: Option<PathBuf>, strict: bool) -> ObjectFile {
    let source = source::open_source(input);
    let source_name = source
        .path_buf
        .as_deref()
        .map(Path::to_string_lossy)
        .unwrap_or("<stdin>".into())
        .to_string();

    let mut lexer = lexer::init_lexer(&source.content);
    let syntax_tree = build_ast::build_ast(&mut lexer, &source_name);
    let module = build_cfg::build_cfg(&syntax_tree, &source_name, strict);
    codegen::codegen(module, &source_name)
}

pub mod ast;
pub mod tokens;

use crate::cli::CompileArgs;
use crate::compile::ast::Lang;
use crate::compile::tokens::Token;

use onessa_core::io::Source;

use log::{debug, error, info};
use logos::{Lexer, Logos};
use rowan::SyntaxNode;
use std::path::PathBuf;
use std::process::exit;
use std::time::Instant;

pub fn init_lexer(content: &str) -> Lexer<'_, Token> {
    let start = Instant::now();
    let lexer = Token::lexer(content);
    debug!(
        "Lexer initialized successfully in {}ms",
        start.elapsed().as_millis()
    );
    #[cfg(feature = "lexer_dump")]
    crate::opt::lexer_dump(&lexer);
    lexer
}

pub fn open_source(path_buf: Option<PathBuf>) -> Source {
    let source = Source::open(path_buf).unwrap_or_else(|err| {
        error!("Failed to open source file: {}", err);
        exit(1);
    });
    debug!("Opened source file {:?}", source.path_buf);
    source
}

pub fn build_ast<'a>(lexer: &'a mut Lexer<'a, Token>) -> SyntaxNode<Lang> {
    info!("Building AST tree");
    let start = Instant::now();
    let parser = ast::build_ast(lexer);
    let syntax = parser.syntax();
    #[cfg(feature = "ast_dump")]
    crate::opt::ast_dump(&syntax);
    debug!(
        "AST built successfully in {}ms",
        start.elapsed().as_millis()
    );

    parser.consume_errors();
    syntax
}

pub fn compile(args: CompileArgs) {
    let source = open_source(args.input);
    let mut lexer = init_lexer(&source.content);
    let _syntax_tree = build_ast(&mut lexer);
}

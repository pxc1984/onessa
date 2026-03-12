mod tokens;

use crate::cli::CompileArgs;
use crate::compile::tokens::Token;
use logos::Logos;
use onessa_core::io::Source;
use std::process::exit;
use tracing::{debug, error, info};

pub fn compile(args: CompileArgs) {
    let source = Source::open(args.input).unwrap_or_else(|err| {
        error!("Failed to open source file: {}", err);
        exit(1);
    });
    debug!("Opened source file {:?}", source.path_buf);
    let _lexer = Token::lexer(&source.content);
    debug!("Lexer initialized successfully");
    #[cfg(feature = "lexer_dump")]
    lexer_dump(&_lexer);

    info!("Building AST tree");
    todo!()
}

#[cfg(feature = "lexer_dump")]
use logos::Lexer;
#[cfg(feature = "lexer_dump")]
fn lexer_dump(lexer: &Lexer<Token>) {
    use tracing::{debug, error, info};
    let mut lexer = lexer.clone();

    info!("Starting debug lexer token dumping");
    while let Some(token) = lexer.next() {
        match token {
            Ok(token) => {
                debug!("{:#?}\t{:#?}\t{:#?}", token, lexer.span(), lexer.slice());
            }
            Err(_) => {
                error!("Failed to read next token");
            }
        }
    }
    info!("Finished debug lexer token dumping");
}

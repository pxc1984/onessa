use crate::compile::ast::{Lang, Parser};
use crate::compile::tokens::Token;
use logos::Lexer;
use rowan::SyntaxNode;

pub fn lexer_dump(lexer: &Lexer<Token>) {
    use log::{debug, error, info};
    let lexer = lexer.clone();

    info!("Starting debug lexer token dumping");
    for token in lexer {
        match token {
            Ok(token) => {
                debug!("{:#?}", token);
            }
            Err(_) => {
                error!("Failed to read next token");
            }
        }
    }
    info!("Finished debug lexer token dumping");
}

use log::{debug, error, info};
use logos::Lexer;
use onessa_lexer::Token;

pub fn lexer_dump(lexer: &Lexer<Token>) {
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

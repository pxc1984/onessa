use log::debug;
use logos::{Lexer, Logos};
use onessa_lexer::Token;
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

use log::{debug, info};
use logos::Lexer;
use onessa_ast::Lang;
use onessa_lexer::Token;
use rowan::SyntaxNode;
use std::time::Instant;

pub fn build_ast<'a>(lexer: &'a mut Lexer<'a, Token>) -> SyntaxNode<Lang> {
    info!("Building AST tree");
    let start = Instant::now();
    let parser = onessa_ast::build_ast(lexer);
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

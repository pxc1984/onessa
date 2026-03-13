pub mod tokens;
pub mod ast;

use crate::cli::CompileArgs;
use crate::compile::tokens::Token;

use onessa_core::io::Source;
use std::process::exit;
use std::time::Instant;
use log::{debug, error, info};
use logos::Logos;

pub fn compile(args: CompileArgs) {
    let source = Source::open(args.input).unwrap_or_else(|err| {
        error!("Failed to open source file: {}", err);
        exit(1);
    });
    debug!("Opened source file {:?}", source.path_buf);
    let start = Instant::now();
    let mut _lexer = Token::lexer(&source.content);
    debug!("Lexer initialized successfully in {}ms", start.elapsed().as_millis());
    #[cfg(feature = "lexer_dump")]
    lexer_dump(&_lexer);

    info!("Building AST tree");
    let start = Instant::now();
    let parser = ast::build_ast(&mut _lexer);

    if !parser.errors().is_empty() {
        for err in parser.errors() {
            error!("{}", err);
        }
        exit(1);
    }

    debug!("AST built successfully in {}ms", start.elapsed().as_millis());

    let syntax_tree = parser.syntax();

    #[cfg(feature = "ast_dump")]
    ast_dump(&syntax_tree);

    let _syntax_tree = syntax_tree;
}

#[cfg(feature = "ast_dump")]
use rowan::{NodeOrToken, SyntaxNode};

#[cfg(feature = "ast_dump")]
fn ast_dump(root: &SyntaxNode<ast::Lang>) {
    use log::{debug, info};

    fn dump(node: &SyntaxNode<ast::Lang>, indent: usize) {
        let prefix = " ".repeat(indent);
        debug!("{}Node: {:?}", prefix, node.kind());

        for child in node.children_with_tokens() {
            match child {
                NodeOrToken::Node(n) => dump(&n, indent + 2),
                NodeOrToken::Token(t) => {
                    let prefix = " ".repeat(indent + 2);
                    debug!("{}Token: {:?} {:?}", prefix, t.kind(), t.text());
                }
            }
        }
    }

    info!("Starting debug AST dump");
    dump(root, 0);
    info!("Finished debug AST dump");
}

#[cfg(feature = "lexer_dump")]
use logos::Lexer;
#[cfg(feature = "lexer_dump")]
fn lexer_dump(lexer: &Lexer<Token>) {
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

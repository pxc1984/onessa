mod lang;
mod parse;
mod syntax_kind;
mod syntax_tree;
pub(crate) mod token_kind;

pub use lang::Lang;
use onessa_lexer::Token;
use parse::ParseSession;
pub use syntax_kind::SyntaxKind;
pub use syntax_tree::SyntaxTree;
use token_kind::token_kind;

pub fn build_ast<'a>(lexer: &'a mut logos::Lexer<'a, Token>) -> SyntaxTree {
    let mut session = ParseSession::new(lexer);
    session.parse();

    SyntaxTree {
        tree: session.builder.finish(),
        errors: session.errors,
    }
}

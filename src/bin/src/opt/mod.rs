#[cfg(feature = "ast_dump")]
pub mod ast_dump;
#[cfg(feature = "ast_dump")]
pub use ast_dump::ast_dump;

#[cfg(feature = "lexer_dump")]
pub mod lexer_dump;
#[cfg(feature = "lexer_dump")]
pub use lexer_dump::lexer_dump;

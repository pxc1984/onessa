use std::error::Error;
use onessa_ast::{SyntaxKind, SyntaxTree};

pub fn build_cfg(_syntax_tree: SyntaxTree) -> Result<(), Box<dyn Error>> {
    for child in _syntax_tree.tree.children() {

    }
    todo!()
}

use crate::compile::ast;
use rowan::{NodeOrToken, SyntaxNode};

pub fn ast_dump(root: &SyntaxNode<ast::Lang>) {
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

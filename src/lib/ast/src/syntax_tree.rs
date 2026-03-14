use crate::Lang;
use log::error;
use rowan::GreenNode;
use std::process::exit;

pub struct SyntaxTree {
    pub tree: GreenNode,
    pub errors: Vec<String>,
}

impl SyntaxTree {
    pub fn consume_errors(self) {
        if self.errors.is_empty() {
            return;
        }

        for err in self.errors {
            error!("{}", err);
        }

        exit(1);
    }

    pub fn syntax(&self) -> rowan::SyntaxNode<Lang> {
        rowan::SyntaxNode::new_root(self.tree.clone())
    }
}

//! Small helpers for navigating the untyped rowan tree.

use onessa_ast::{Lang, SyntaxKind};
use rowan::{SyntaxNode, SyntaxToken};

pub fn child_nodes(node: &SyntaxNode<Lang>) -> impl Iterator<Item = SyntaxNode<Lang>> {
    node.children()
}

/// Tokens directly inside `node`, trivia excluded.
pub fn child_tokens(node: &SyntaxNode<Lang>) -> impl Iterator<Item = SyntaxToken<Lang>> {
    node.children_with_tokens()
        .filter_map(|element| element.into_token())
        .filter(|token| token.kind() != SyntaxKind::Trivia)
}

pub fn has_token(node: &SyntaxNode<Lang>, kind: SyntaxKind) -> bool {
    child_tokens(node).any(|t| t.kind() == kind)
}

/// Text of an `Identifier` node.
pub fn identifier_text(node: &SyntaxNode<Lang>) -> String {
    child_tokens(node)
        .next()
        .map(|t| t.text().to_string())
        .unwrap_or_default()
}

/// Kind of the first token directly inside `node`.
pub fn first_token_kind(node: &SyntaxNode<Lang>) -> Option<SyntaxKind> {
    child_tokens(node).next().map(|t| t.kind())
}

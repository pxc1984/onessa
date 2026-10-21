use crate::lower::syntax::{child_nodes, child_tokens, first_token_kind};
use crate::lower::Context;
use logos::Logos;
use onessa_ast::{Lang, SyntaxKind};
use onessa_core::bytecode::Constant;
use onessa_lexer::Token;
use rowan::SyntaxNode;

/// Value of a `Literal` node.
pub fn literal(ctx: &mut Context, node: &SyntaxNode<Lang>) -> Option<Constant> {
    let token = child_tokens(node).next()?;
    let constant = match token.kind() {
        SyntaxKind::True => Constant::Boolean(true),
        SyntaxKind::False => Constant::Boolean(false),
        SyntaxKind::Undefined => Constant::Undefined,
        SyntaxKind::Null => Constant::Null,
        SyntaxKind::NumberLiteral
        | SyntaxKind::StringLiteral
        | SyntaxKind::DateLiteral => match Token::lexer(token.text()).next() {
            Some(Ok(Token::NumberLiteral(n))) => Constant::Number(n),
            Some(Ok(Token::StringLiteral(s))) => Constant::String(ctx.strings.push(&s)),
            Some(Ok(Token::DateLiteral(d))) => Constant::Date(d.timestamp()),
            _ => {
                ctx.error(node, format!("Invalid literal {}", token.text()));
                return None;
            }
        },
        _ => return None,
    };
    Some(constant)
}

/// Evaluates expressions allowed as default parameter values: literals, optionally negated.
pub fn constant_expression(ctx: &mut Context, node: &SyntaxNode<Lang>) -> Option<Constant> {
    match node.kind() {
        SyntaxKind::Expression | SyntaxKind::ParenExpression => {
            let inner = child_nodes(node).next()?;
            constant_expression(ctx, &inner)
        }
        SyntaxKind::Literal => literal(ctx, node),
        SyntaxKind::UnaryExpression => {
            let operand = child_nodes(node).next()?;
            match (first_token_kind(node)?, constant_expression(ctx, &operand)?) {
                (SyntaxKind::Minus, Constant::Number(n)) => Some(Constant::Number(-n)),
                (SyntaxKind::Plus, Constant::Number(n)) => Some(Constant::Number(n)),
                _ => None,
            }
        }
        _ => None,
    }
}

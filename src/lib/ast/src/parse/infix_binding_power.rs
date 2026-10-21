use crate::parse::ParseSession;
use onessa_lexer::Token;

impl<'a> ParseSession<'a> {
    pub fn infix_binding_power(token: &Token) -> Option<(u8, u8)> {
        match token {
            Token::Or => Some((1, 2)),
            Token::And => Some((3, 4)),
            Token::Eq | Token::Ne | Token::Lt | Token::Gt | Token::Le | Token::Ge => Some((5, 6)),
            Token::Plus | Token::Minus => Some((7, 8)),
            Token::Star | Token::Slash | Token::Percent => Some((9, 10)),
            _ => None,
        }
    }
}

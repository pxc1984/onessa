use crate::parse::ParseSession;
use onessa_lexer::Token;

impl<'a> ParseSession<'a> {
    pub fn parse_primary(&mut self) {
        match &self.current {
            Some(Token::Identifier(_)) | Some(Token::Select) => self.parse_identifier(),
            Some(Token::NumberLiteral(_))
            | Some(Token::StringLiteral(_))
            | Some(Token::DateLiteral(_))
            | Some(Token::True)
            | Some(Token::False)
            | Some(Token::Undefined)
            | Some(Token::Null) => self.parse_literal(),
            Some(Token::LParen) => {
                self.token_current();
                self.parse_expression();
                self.expect_token(&Token::RParen);
            }
            Some(token) => {
                self.error_at_current(format!("Unexpected token in expression: {:?}", token));
                self.token_current();
            }
            None => {}
        }
    }
}

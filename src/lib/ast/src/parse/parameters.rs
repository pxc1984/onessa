use crate::parse::ParseSession;
use crate::SyntaxKind;
use onessa_lexer::Token;

impl<'a> ParseSession<'a> {
    /// Parses `Имя(Знач А, Б = 1) Экспорт` shared by procedures and functions.
    pub fn parse_signature(&mut self) {
        if let Some(Token::Identifier(_)) = self.current {
            self.parse_identifier();
        } else {
            self.error_at_current(format!("Expected method name, got {:?}", self.current));
        }

        self.expect_token(&Token::LParen);
        while !self.peek_is(&Token::RParen) && self.current.is_some() {
            if self.peek_in(&[&Token::Val, &Token::Identifier(String::new())]) {
                self.parse_parameter();
            } else {
                self.error_at_current(format!(
                    "Unexpected token in parameter list: {:?}",
                    self.current
                ));
                self.token_current();
                continue;
            }

            if self.peek_is(&Token::Comma) {
                self.token_current();
            } else if !self.peek_is(&Token::RParen) {
                self.error_at_current(format!(
                    "Expected ',' or ')' in parameter list, got {:?}",
                    self.current
                ));
                self.token_current();
            }
        }
        self.expect_token(&Token::RParen);

        if self.peek_is(&Token::Export) {
            self.token_current();
        }
    }

    fn parse_parameter(&mut self) {
        self.builder.start_node(SyntaxKind::Parameter.into());
        if self.peek_is(&Token::Val) {
            self.token_current();
        }
        if let Some(Token::Identifier(_)) = self.current {
            self.parse_identifier();
        } else {
            self.error_at_current(format!("Expected parameter name, got {:?}", self.current));
        }
        if self.peek_is(&Token::Eq) {
            self.token_current();
            self.parse_expression();
        }
        self.builder.finish_node();
    }
}

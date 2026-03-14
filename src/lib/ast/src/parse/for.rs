use crate::parse::ParseSession;
use crate::SyntaxKind;
use onessa_lexer::Token;

impl<'a> ParseSession<'a> {
    pub fn parse_for(&mut self) {
        self.builder.start_node(SyntaxKind::ForEachStatement.into());
        self.expect_token(&Token::For);

        if self.peek_is(&Token::Each) {
            self.expect_token(&Token::Each);
            self.parse_expression();
            self.expect_token(&Token::In);
            self.parse_expression();
        } else {
            // Loop variable
            if let Some(Token::Identifier(_)) = self.current.clone() {
                self.parse_identifier();
            } else {
                self.error_at_current(format!(
                    "Expected loop variable after For, got {:?}",
                    self.current
                ));
            }

            if self.peek_is(&Token::Eq) {
                self.token_current();
                self.parse_expression();
            }

            if let Some(Token::Identifier(_)) = self.current.clone() {
                if self.current_text.eq_ignore_ascii_case("to") {
                    self.token_current();
                } else {
                    self.error_at_current(format!(
                        "Expected 'To' in for-loop, got {:?}",
                        self.current
                    ));
                }
            } else {
                self.error_at_current(format!("Expected 'To' in for-loop, got {:?}", self.current));
            }

            self.parse_expression();
        }

        if self.peek_is(&Token::Do) {
            self.expect_token(&Token::Do);
        }

        self.parse_block(&[&Token::EndDo]);
        self.expect_token(&Token::EndDo);
        self.builder.finish_node();
    }
}

use crate::parse::ParseSession;
use crate::SyntaxKind;
use onessa_lexer::Token;

impl<'a> ParseSession<'a> {
    pub fn parse_procedure(&mut self) {
        self.builder
            .start_node(SyntaxKind::ProcedureDeclaration.into());
        self.expect_token(&Token::Procedure);
        if let Some(Token::Identifier(_)) = self.current.clone() {
            self.parse_identifier();
        }

        if self.peek_is(&Token::LParen) {
            self.expect_token(&Token::LParen);
            while !self.peek_is(&Token::RParen) && self.current.is_some() {
                let mut progressed = false;

                if self.peek_is(&Token::Val) {
                    self.expect_token(&Token::Val);
                    progressed = true;
                }

                if let Some(Token::Identifier(_)) = self.current.clone() {
                    self.parse_identifier();
                    progressed = true;

                    if self.peek_is(&Token::Eq) {
                        self.token_current();
                        self.parse_expression();
                    }
                }

                if self.peek_is(&Token::Comma) {
                    self.token_current();
                    progressed = true;
                }

                if !progressed {
                    self.error_at_current(format!(
                        "Unexpected token in parameter list: {:?}",
                        self.current
                    ));
                    self.token_current();
                }
            }
            self.expect_token(&Token::RParen);
        }

        if self.peek_is(&Token::Export) {
            self.expect_token(&Token::Export);
        }

        self.parse_block(&[&Token::EndProcedure]);
        self.expect_token(&Token::EndProcedure);
        self.builder.finish_node();
    }
}

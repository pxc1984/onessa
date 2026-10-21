use crate::parse::ParseSession;
use crate::SyntaxKind;
use onessa_lexer::Token;

impl<'a> ParseSession<'a> {
    /// `Перем А, Б Экспорт;`
    pub fn parse_var_decl(&mut self) {
        self.builder
            .start_node(SyntaxKind::VariableDeclaration.into());
        self.expect_token(&Token::Var);

        loop {
            if let Some(Token::Identifier(_)) = self.current {
                self.parse_identifier();
                if self.peek_is(&Token::Export) {
                    self.token_current();
                }
            } else {
                self.error_at_current(format!(
                    "Expected identifier in variable declaration, got {:?}",
                    self.current
                ));
                self.token_current();
            }

            if self.peek_is(&Token::Comma) {
                self.token_current();
                continue;
            }
            break;
        }

        if self.peek_is(&Token::Semicolon) {
            self.token_current();
        }
        self.builder.finish_node();
    }
}

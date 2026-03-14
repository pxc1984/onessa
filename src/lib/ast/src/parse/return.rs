use crate::parse::ParseSession;
use crate::SyntaxKind;
use onessa_lexer::Token;

impl<'a> ParseSession<'a> {
    pub fn parse_return(&mut self) {
        self.builder.start_node(SyntaxKind::ReturnStatement.into());
        self.expect_token(&Token::Return);
        if self.current.is_some() && !self.peek_is(&Token::Semicolon) {
            self.parse_expression();
        }
        if self.peek_is(&Token::Semicolon) {
            self.token_current();
        }
        self.builder.finish_node();
    }
}

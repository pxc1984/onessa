use crate::parse::ParseSession;
use crate::SyntaxKind;
use onessa_lexer::Token;

impl<'a> ParseSession<'a> {
    pub fn parse_continue(&mut self) {
        self.builder
            .start_node(SyntaxKind::ContinueStatement.into());
        self.expect_token(&Token::Continue);
        if self.peek_is(&Token::Semicolon) {
            self.token_current();
        }
        self.builder.finish_node();
    }
}

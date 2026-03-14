use crate::parse::ParseSession;
use crate::SyntaxKind;
use onessa_lexer::Token;

impl<'a> ParseSession<'a> {
    pub fn parse_try(&mut self) {
        self.builder.start_node(SyntaxKind::TryStatement.into());
        self.expect_token(&Token::Try);
        self.parse_block(&[&Token::Except, &Token::EndTry]);

        if self.peek_is(&Token::Except) {
            self.expect_token(&Token::Except);
            self.parse_block(&[&Token::EndTry]);
        }

        self.expect_token(&Token::EndTry);
        self.builder.finish_node();
    }
}

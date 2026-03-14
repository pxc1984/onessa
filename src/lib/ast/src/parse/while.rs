use crate::parse::ParseSession;
use crate::SyntaxKind;
use onessa_lexer::Token;

impl<'a> ParseSession<'a> {
    pub fn parse_while(&mut self) {
        self.builder.start_node(SyntaxKind::WhileStatement.into());
        self.expect_token(&Token::While);
        self.parse_expression();
        self.expect_token(&Token::Do);
        self.parse_block(&[&Token::EndDo]);
        self.expect_token(&Token::EndDo);
        self.builder.finish_node();
    }
}

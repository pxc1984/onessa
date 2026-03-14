use crate::parse::ParseSession;
use crate::SyntaxKind;
use onessa_lexer::Token;

impl<'a> ParseSession<'a> {
    pub fn parse_if(&mut self) {
        self.builder.start_node(SyntaxKind::IfStatement.into());
        self.expect_token(&Token::If);
        self.parse_expression();
        self.expect_token(&Token::Then);
        self.parse_block(&[&Token::Else, &Token::ElseIf, &Token::EndIf]);

        while self.peek_is(&Token::ElseIf) {
            self.builder.start_node(SyntaxKind::IfStatement.into());
            self.expect_token(&Token::ElseIf);
            self.parse_expression();
            self.expect_token(&Token::Then);
            self.parse_block(&[&Token::Else, &Token::ElseIf, &Token::EndIf]);
            self.builder.finish_node();
        }

        if self.peek_is(&Token::Else) {
            self.expect_token(&Token::Else);
            self.parse_block(&[&Token::EndIf]);
        }

        self.expect_token(&Token::EndIf);
        self.builder.finish_node();
    }
}

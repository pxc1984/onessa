use crate::parse::ParseSession;
use crate::SyntaxKind;
use onessa_lexer::Token;

impl<'a> ParseSession<'a> {
    pub fn parse_if(&mut self) {
        const BRANCH_END: &[&Token] = &[&Token::Else, &Token::ElseIf, &Token::EndIf];

        self.builder.start_node(SyntaxKind::IfStatement.into());
        self.expect_token(&Token::If);
        self.parse_expression();
        self.expect_token(&Token::Then);
        self.parse_block(BRANCH_END);

        while self.peek_is(&Token::ElseIf) {
            self.builder.start_node(SyntaxKind::ElseIfClause.into());
            self.expect_token(&Token::ElseIf);
            self.parse_expression();
            self.expect_token(&Token::Then);
            self.parse_block(BRANCH_END);
            self.builder.finish_node();
        }

        if self.peek_is(&Token::Else) {
            self.builder.start_node(SyntaxKind::ElseClause.into());
            self.expect_token(&Token::Else);
            self.parse_block(&[&Token::EndIf]);
            self.builder.finish_node();
        }

        self.expect_token(&Token::EndIf);
        self.builder.finish_node();
    }
}

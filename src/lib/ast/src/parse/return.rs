use crate::parse::ParseSession;
use crate::SyntaxKind;
use onessa_lexer::Token;

impl<'a> ParseSession<'a> {
    pub fn parse_return(&mut self) {
        self.builder.start_node(SyntaxKind::ReturnStatement.into());
        self.expect_token(&Token::Return);
        if self.current.is_some() && !self.at_statement_end() {
            self.parse_expression();
        }
        if self.peek_is(&Token::Semicolon) {
            self.token_current();
        }
        self.builder.finish_node();
    }

    /// `ВызватьИсключение [Выражение];`. Without an expression the current error is re-raised.
    pub fn parse_raise(&mut self) {
        self.builder.start_node(SyntaxKind::RaiseStatement.into());
        self.expect_token(&Token::Raise);
        if self.current.is_some() && !self.at_statement_end() {
            self.parse_expression();
        }
        if self.peek_is(&Token::Semicolon) {
            self.token_current();
        }
        self.builder.finish_node();
    }

    /// True if the current token cannot start an expression continuing the statement.
    fn at_statement_end(&self) -> bool {
        self.peek_in(&[
            &Token::Semicolon,
            &Token::Else,
            &Token::ElseIf,
            &Token::EndIf,
            &Token::EndDo,
            &Token::Except,
            &Token::EndTry,
            &Token::EndProcedure,
            &Token::EndFunction,
        ])
    }
}

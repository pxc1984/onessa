use crate::parse::ParseSession;
use crate::SyntaxKind;
use onessa_lexer::Token;

impl<'a> ParseSession<'a> {
    /// `Для Сч = Начало По Конец Цикл ... КонецЦикла` or
    /// `Для Каждого Элемент Из Коллекция Цикл ... КонецЦикла`
    pub fn parse_for(&mut self) {
        let checkpoint = self.builder.checkpoint();
        self.expect_token(&Token::For);

        if self.peek_is(&Token::Each) {
            self.builder
                .start_node_at(checkpoint, SyntaxKind::ForEachStatement.into());
            self.expect_token(&Token::Each);
            self.parse_loop_variable();
            self.expect_token(&Token::In);
            self.parse_expression();
        } else {
            self.builder
                .start_node_at(checkpoint, SyntaxKind::ForStatement.into());
            self.parse_loop_variable();
            self.expect_token(&Token::Eq);
            self.parse_expression();
            self.expect_token(&Token::To);
            self.parse_expression();
        }

        self.expect_token(&Token::Do);
        self.parse_block(&[&Token::EndDo]);
        self.expect_token(&Token::EndDo);
        self.builder.finish_node();
    }

    fn parse_loop_variable(&mut self) {
        if let Some(Token::Identifier(_)) = self.current {
            self.parse_identifier();
        } else {
            self.error_at_current(format!(
                "Expected loop variable after For, got {:?}",
                self.current
            ));
        }
    }
}

use crate::parse::ParseSession;
use crate::SyntaxKind;
use onessa_lexer::Token;

impl<'a> ParseSession<'a> {
    pub fn parse_primary(&mut self) {
        match &self.current {
            Some(Token::Identifier(_)) | Some(Token::Select) => self.parse_identifier(),
            Some(Token::NumberLiteral(_))
            | Some(Token::StringLiteral(_))
            | Some(Token::DateLiteral(_))
            | Some(Token::True)
            | Some(Token::False)
            | Some(Token::Undefined)
            | Some(Token::Null) => self.parse_literal(),
            Some(Token::LParen) => {
                self.builder.start_node(SyntaxKind::ParenExpression.into());
                self.token_current();
                self.parse_expression();
                self.expect_token(&Token::RParen);
                self.builder.finish_node();
            }
            Some(Token::QuestionMark) => self.parse_ternary(),
            Some(token) => {
                self.error_at_current(format!("Unexpected token in expression: {:?}", token));
                self.token_current();
            }
            None => {
                self.error_at_current("Unexpected end of file in expression".to_string());
            }
        }
    }

    /// `?(Условие, ЕслиИстина, ЕслиЛожь)`
    fn parse_ternary(&mut self) {
        self.builder
            .start_node(SyntaxKind::TernaryExpression.into());
        self.expect_token(&Token::QuestionMark);
        self.expect_token(&Token::LParen);
        self.parse_expression();
        self.expect_token(&Token::Comma);
        self.parse_expression();
        self.expect_token(&Token::Comma);
        self.parse_expression();
        self.expect_token(&Token::RParen);
        self.builder.finish_node();
    }
}

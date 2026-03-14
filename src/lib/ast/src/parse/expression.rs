use crate::parse::ParseSession;
use crate::SyntaxKind;
use onessa_lexer::Token;

impl<'a> ParseSession<'a> {
    pub fn parse_expression_statement(&mut self) {
        self.builder
            .start_node(SyntaxKind::ExpressionStatement.into());
        self.parse_expression();
        if self.peek_is(&Token::Semicolon) {
            self.token_current();
        }
        self.builder.finish_node();
    }

    pub fn parse_expression(&mut self) {
        self.builder.start_node(SyntaxKind::Expression.into());
        self.parse_binary_expression(0);
        self.builder.finish_node();
    }

    pub fn parse_binary_expression(&mut self, min_bp: u8) {
        let checkpoint = self.builder.checkpoint();
        self.parse_unary();

        while let Some(op) = self.current.clone() {
            if let Some((l_bp, r_bp)) = Self::infix_binding_power(&op) {
                if l_bp < min_bp {
                    break;
                }

                self.builder
                    .start_node_at(checkpoint, SyntaxKind::BinaryExpression.into());
                self.token_current();
                self.parse_binary_expression(r_bp);
                self.builder.finish_node();
                continue;
            }

            break;
        }
    }

    pub fn parse_unary(&mut self) {
        if self.peek_is(&Token::Not) || self.peek_is(&Token::Minus) || self.peek_is(&Token::New) {
            let checkpoint = self.builder.checkpoint();
            let kind = if self.peek_is(&Token::New) {
                SyntaxKind::NewExpression
            } else {
                SyntaxKind::UnaryExpression
            };
            self.builder.start_node_at(checkpoint, kind.into());
            self.token_current();
            self.parse_unary();
            self.builder.finish_node();
            return;
        }
        self.parse_postfix();
    }

    pub fn parse_postfix(&mut self) {
        let checkpoint = self.builder.checkpoint();
        self.parse_primary();

        loop {
            if self.peek_is(&Token::LParen) {
                self.builder
                    .start_node_at(checkpoint, SyntaxKind::CallExpression.into());
                self.token_current();

                if !self.peek_is(&Token::RParen) {
                    self.parse_expression();
                    while self.peek_is(&Token::Comma) {
                        self.token_current();
                        self.parse_expression();
                    }
                }

                self.expect_token(&Token::RParen);
                self.builder.finish_node();
                continue;
            }

            if self.peek_is(&Token::Period) {
                self.builder
                    .start_node_at(checkpoint, SyntaxKind::MemberExpression.into());
                self.token_current();
                if self.is_identifier_like() {
                    self.parse_identifier();
                } else {
                    self.error_at_current(format!(
                        "Expected member name after '.', got {:?}",
                        self.current
                    ));
                }
                self.builder.finish_node();
                continue;
            }

            break;
        }
    }
}

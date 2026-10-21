use crate::parse::ParseSession;
use crate::SyntaxKind;
use onessa_lexer::Token;

/// Binding power of the `Не`/`Not` operand: tighter than `И`, looser than comparisons.
const NOT_OPERAND_BP: u8 = 5;

impl<'a> ParseSession<'a> {
    /// Parses either an assignment (`Приемник = Выражение;`) or an expression statement.
    ///
    /// In statement position `=` is always an assignment, so the left-hand side is parsed as a
    /// postfix expression and the operator is decided by the next token.
    pub fn parse_expression_statement(&mut self) {
        let checkpoint = self.builder.checkpoint();
        self.parse_postfix();

        if self.peek_is(&Token::Eq) {
            self.builder
                .start_node_at(checkpoint, SyntaxKind::AssignmentStatement.into());
            self.token_current();
            self.parse_expression();
        } else {
            self.builder
                .start_node_at(checkpoint, SyntaxKind::ExpressionStatement.into());
        }

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
        if self.peek_is(&Token::Not) {
            self.builder.start_node(SyntaxKind::UnaryExpression.into());
            self.token_current();
            self.parse_binary_expression(NOT_OPERAND_BP);
            self.builder.finish_node();
            return;
        }
        if self.peek_is(&Token::Minus) || self.peek_is(&Token::Plus) {
            self.builder.start_node(SyntaxKind::UnaryExpression.into());
            self.token_current();
            self.parse_unary();
            self.builder.finish_node();
            return;
        }
        if self.peek_is(&Token::New) {
            self.builder.start_node(SyntaxKind::NewExpression.into());
            self.token_current();
            self.parse_postfix();
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
                self.parse_arguments();
                self.builder.finish_node();
                continue;
            }

            if self.peek_is(&Token::LSqParen) {
                self.builder
                    .start_node_at(checkpoint, SyntaxKind::IndexExpression.into());
                self.token_current();
                self.parse_expression();
                self.expect_token(&Token::RSqParen);
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

    /// Parses `(arg, , arg)`. A skipped argument produces an empty `Expression` node so that
    /// argument positions are preserved.
    pub fn parse_arguments(&mut self) {
        self.expect_token(&Token::LParen);
        if self.peek_is(&Token::RParen) {
            self.token_current();
            return;
        }

        loop {
            if self.peek_in(&[&Token::Comma, &Token::RParen]) {
                self.builder.start_node(SyntaxKind::Expression.into());
                self.builder.finish_node();
            } else {
                self.parse_expression();
            }

            if self.peek_is(&Token::Comma) {
                self.token_current();
                continue;
            }
            break;
        }
        self.expect_token(&Token::RParen);
    }
}

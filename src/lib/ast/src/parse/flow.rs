use crate::parse::ParseSession;
use onessa_lexer::Token;

impl<'a> ParseSession<'a> {
    pub fn bump(&mut self) {
        match self.lexer.next() {
            Some(Ok(token)) => {
                self.current_text = self.lexer.slice().to_string();
                self.current = Some(token);
            }
            Some(Err(err)) => {
                self.current_text = self.lexer.slice().to_string();
                self.error_at_current(format!("Lexer error: {err:?}"));
                self.current = None;
            }
            None => {
                self.current_text.clear();
                self.current = None;
            }
        }
    }

    pub fn peek_is(&self, expected: &Token) -> bool {
        self.current
            .as_ref()
            .map(|t| std::mem::discriminant(t) == std::mem::discriminant(expected))
            .unwrap_or(false)
    }

    pub fn peek_in(&self, expected: &[&Token]) -> bool {
        self.current.as_ref().is_some_and(|t| {
            let disc = std::mem::discriminant(t);
            expected.iter().any(|e| std::mem::discriminant(*e) == disc)
        })
    }

    pub fn token_current(&mut self) {
        if let Some(token) = &self.current {
            let kind = crate::token_kind(token);
            self.builder.token(kind.into(), &self.current_text);
            self.bump();
        }
    }

    pub fn expect_token(&mut self, expected: &Token) {
        if self.peek_is(expected) {
            self.token_current();
        } else {
            self.error_at_current(format!(
                "Expected token {:?}, got {:?}",
                expected, self.current
            ));
            self.token_current();
        }
    }
}

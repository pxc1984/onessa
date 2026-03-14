use crate::parse::ParseSession;
use crate::SyntaxKind;

use onessa_lexer::Token;

impl<'a> ParseSession<'a> {
    pub fn is_identifier_like(&self) -> bool {
        matches!(
            self.current,
            Some(Token::Identifier(_))
                | Some(Token::Select)
                | Some(Token::New)
                | Some(Token::StringLiteral(_))
                | Some(Token::NumberLiteral(_))
        )
    }

    pub fn parse_identifier(&mut self) {
        self.builder.start_node(SyntaxKind::Identifier.into());
        self.token_current();
        self.builder.finish_node();
    }
}

use crate::parse::ParseSession;
use crate::SyntaxKind;
use onessa_lexer::Token;

impl<'a> ParseSession<'a> {
    pub fn parse_function(&mut self) {
        self.builder
            .start_node(SyntaxKind::FunctionDeclaration.into());
        self.expect_token(&Token::Function);
        self.parse_signature();
        self.parse_block(&[&Token::EndFunction]);
        self.expect_token(&Token::EndFunction);
        self.builder.finish_node();
    }
}

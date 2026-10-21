use crate::parse::ParseSession;
use crate::SyntaxKind;
use onessa_lexer::Token;

impl<'a> ParseSession<'a> {
    pub fn parse_procedure(&mut self) {
        self.builder
            .start_node(SyntaxKind::ProcedureDeclaration.into());
        self.expect_token(&Token::Procedure);
        self.parse_signature();
        self.parse_block(&[&Token::EndProcedure]);
        self.expect_token(&Token::EndProcedure);
        self.builder.finish_node();
    }
}

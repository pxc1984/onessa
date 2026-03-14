use crate::parse::ParseSession;
use crate::SyntaxKind;
use onessa_lexer::Token;

impl<'a> ParseSession<'a> {
    pub fn parse_block(&mut self, terminators: &[&Token]) {
        self.builder.start_node(SyntaxKind::Block.into());
        while self.current.is_some() && !self.peek_in(terminators) {
            self.parse_statement();
        }
        self.builder.finish_node();
    }
}

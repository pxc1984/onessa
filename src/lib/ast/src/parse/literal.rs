use crate::parse::ParseSession;
use crate::SyntaxKind;

impl<'a> ParseSession<'a> {
    pub fn parse_literal(&mut self) {
        self.builder.start_node(SyntaxKind::Literal.into());
        self.token_current();
        self.builder.finish_node();
    }
}

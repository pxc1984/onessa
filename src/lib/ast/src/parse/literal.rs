use crate::SyntaxKind;
use crate::parse::ParseSession;

impl<'a> ParseSession<'a> {
    pub fn parse_literal(&mut self) {
        self.builder.start_node(SyntaxKind::Literal.into());
        self.token_current();
        self.builder.finish_node();
    }
}

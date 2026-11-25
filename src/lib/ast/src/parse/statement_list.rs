use crate::SyntaxKind;
use crate::parse::ParseSession;

impl<'a> ParseSession<'a> {
    pub fn parse_statement_list(&mut self) {
        self.builder.start_node(SyntaxKind::StatementList.into());
        while self.current.is_some() {
            self.parse_statement();
        }
        self.builder.finish_node();
    }
}

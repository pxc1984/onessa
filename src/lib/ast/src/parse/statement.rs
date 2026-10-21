use crate::parse::ParseSession;
use onessa_lexer::Token;

impl<'a> ParseSession<'a> {
    pub fn parse_statement(&mut self) {
        if self.peek_is(&Token::If) {
            self.parse_if();
        } else if self.peek_is(&Token::While) {
            self.parse_while();
        } else if self.peek_is(&Token::For) {
            self.parse_for();
        } else if self.peek_is(&Token::Try) {
            self.parse_try();
        } else if self.peek_is(&Token::Procedure) {
            self.parse_procedure();
        } else if self.peek_is(&Token::Function) {
            self.parse_function();
        } else if self.peek_is(&Token::Var) {
            self.parse_var_decl();
        } else if self.peek_is(&Token::Return) {
            self.parse_return();
        } else if self.peek_is(&Token::Break) {
            self.parse_break();
        } else if self.peek_is(&Token::Raise) {
            self.parse_raise();
        } else if self.peek_is(&Token::Continue) {
            self.parse_continue();
        } else if self.peek_is(&Token::Semicolon) {
            // Empty statement/statement separator
            self.token_current();
        } else {
            self.parse_expression_statement();
        }
    }
}

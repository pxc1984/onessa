use crate::parse::ParseSession;

impl<'a> ParseSession<'a> {
    fn position(&self) -> (usize, usize) {
        let span = self.lexer.span();
        let idx = span.start.min(self.source.len());
        let mut line = 1;
        let mut col = 1;
        for ch in self.source[..idx].chars() {
            if ch == '\n' {
                line += 1;
                col = 1;
            } else {
                col += 1;
            }
        }
        (line, col)
    }

    pub fn error_at_current(&mut self, message: String) {
        let (line, col) = self.position();
        self.errors.push(format!("{} at {}:{}", message, line, col));
    }
}

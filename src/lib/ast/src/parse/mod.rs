use crate::SyntaxKind;
use onessa_lexer::Token;
use rowan::GreenNodeBuilder;

mod block;
mod r#break;
mod r#continue;
mod expression;
mod flow;
mod r#for;
mod function;
mod identifier;
mod r#if;
mod infix_binding_power;
mod literal;
mod parameters;
mod position;
mod primary;
mod procedure;
mod r#return;
mod statement;
mod statement_list;
mod r#try;
mod var_declaration;
mod r#while;

pub struct ParseSession<'a> {
    pub(crate) builder: GreenNodeBuilder<'static>,
    lexer: &'a mut logos::Lexer<'a, Token>,
    source: &'a str,
    current: Option<Token>,
    current_text: String,
    current_span: std::ops::Range<usize>,
    /// End of the last token added to the tree; the gap up to the next token is trivia.
    last_end: usize,
    pub(crate) errors: Vec<String>,
}

impl<'a> ParseSession<'a> {
    pub(crate) fn new(lexer: &'a mut logos::Lexer<'a, Token>) -> Self {
        let mut session = ParseSession {
            builder: GreenNodeBuilder::new(),
            source: lexer.source(),
            lexer,
            current: None,
            current_text: String::new(),
            current_span: 0..0,
            last_end: 0,
            errors: Vec::new(),
        };
        session.bump();
        session
    }

    pub(crate) fn parse(&mut self) {
        self.builder.start_node(SyntaxKind::Root.into());
        self.builder.start_node(SyntaxKind::SourceFile.into());
        self.parse_statement_list();
        self.trivia_until(self.source.len());
        self.builder.finish_node();
        self.builder.finish_node();
    }
}

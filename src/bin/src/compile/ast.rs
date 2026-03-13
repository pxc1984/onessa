use rowan::{GreenNode, GreenNodeBuilder};

use crate::compile::tokens::Token;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u16)]
pub enum SyntaxKind {
    // Nodes
    Root = 0,
    SourceFile,
    StatementList,
    Statement,
    Block,
    IfStatement,
    WhileStatement,
    ForEachStatement,
    TryStatement,
    ProcedureDeclaration,
    FunctionDeclaration,
    VariableDeclaration,
    ReturnStatement,
    BreakStatement,
    ContinueStatement,
    ExpressionStatement,
    Expression,
    BinaryExpression,
    UnaryExpression,
    Literal,
    Identifier,

    // Tokens
    LineComment,

    // 2. Многосимвольные операторы
    Ne,
    Le,
    Ge,

    // 3. Односимвольные операторы
    Eq,
    Lt,
    Gt,
    Plus,
    Minus,
    Star,
    Slash,
    Semicolon,
    Period,
    QuestionMark,
    Comma,
    LParen,
    RParen,
    LSqParen,
    RSqParen,

    // 4. Ключевые слова
    // Операторы цикла и условия
    If,
    Then,
    Else,
    ElseIf,
    EndIf,
    For,
    Each,
    In,
    While,
    Do,
    EndDo,
    Break,
    Continue,

    // Объявление переменных и процедур/функций
    Var,
    Procedure,
    Function,
    EndProcedure,
    EndFunction,

    // Экспорт/импорт
    Export,
    Val,

    // Логические операторы
    And,
    Or,
    Not,

    // Литералы
    True,
    False,
    Undefined,
    Null,

    // Работа с данными
    Select,
    Where,
    As,
    Union,
    All,
    Distinct,
    OrderBy,
    TotalBy,
    IntoTable,
    AutoSort,
    Asc,
    Desc,

    // Исключения
    Try,
    Except,
    EndTry,
    Raise,

    // Возврат значения
    Return,

    // Новый объект
    New,

    // 5. Литералы
    NumberLiteral,
    StringLiteral,
    DateLiteral,

    // Special
    Error,
}

impl From<SyntaxKind> for rowan::SyntaxKind {
    fn from(value: SyntaxKind) -> Self {
        Self(value as u16)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Lang {}
impl rowan::Language for Lang {
    type Kind = SyntaxKind;

    fn kind_from_raw(raw: rowan::SyntaxKind) -> Self::Kind {
        // Safety: we only ever create SyntaxKind values inside this crate.
        unsafe { std::mem::transmute::<u16, SyntaxKind>(raw.0) }
    }

    fn kind_to_raw(kind: Self::Kind) -> rowan::SyntaxKind {
        kind.into()
    }
}

pub struct Parser {
    tree: GreenNode,
    errors: Vec<String>,
}

impl Parser {
    pub fn errors(&self) -> &[String] {
        &self.errors
    }

    pub fn syntax(&self) -> rowan::SyntaxNode<Lang> {
        rowan::SyntaxNode::new_root(self.tree.clone())
    }
}

fn token_kind(token: &Token) -> SyntaxKind {
    match token {
        Token::LineComment => SyntaxKind::LineComment,
        Token::Ne => SyntaxKind::Ne,
        Token::Le => SyntaxKind::Le,
        Token::Ge => SyntaxKind::Ge,
        Token::Eq => SyntaxKind::Eq,
        Token::Lt => SyntaxKind::Lt,
        Token::Gt => SyntaxKind::Gt,
        Token::Plus => SyntaxKind::Plus,
        Token::Minus => SyntaxKind::Minus,
        Token::Star => SyntaxKind::Star,
        Token::Slash => SyntaxKind::Slash,
        Token::Semicolon => SyntaxKind::Semicolon,
        Token::Period => SyntaxKind::Period,
        Token::QuestionMark => SyntaxKind::QuestionMark,
        Token::Comma => SyntaxKind::Comma,
        Token::LParen => SyntaxKind::LParen,
        Token::RParen => SyntaxKind::RParen,
        Token::LSqParen => SyntaxKind::LSqParen,
        Token::RSqParen => SyntaxKind::RSqParen,

        Token::If => SyntaxKind::If,
        Token::Then => SyntaxKind::Then,
        Token::Else => SyntaxKind::Else,
        Token::ElseIf => SyntaxKind::ElseIf,
        Token::EndIf => SyntaxKind::EndIf,
        Token::For => SyntaxKind::For,
        Token::Each => SyntaxKind::Each,
        Token::In => SyntaxKind::In,
        Token::While => SyntaxKind::While,
        Token::Do => SyntaxKind::Do,
        Token::EndDo => SyntaxKind::EndDo,
        Token::Break => SyntaxKind::Break,
        Token::Continue => SyntaxKind::Continue,

        Token::Var => SyntaxKind::Var,
        Token::Procedure => SyntaxKind::Procedure,
        Token::Function => SyntaxKind::Function,
        Token::EndProcedure => SyntaxKind::EndProcedure,
        Token::EndFunction => SyntaxKind::EndFunction,

        Token::Export => SyntaxKind::Export,
        Token::Val => SyntaxKind::Val,

        Token::And => SyntaxKind::And,
        Token::Or => SyntaxKind::Or,
        Token::Not => SyntaxKind::Not,

        Token::True => SyntaxKind::True,
        Token::False => SyntaxKind::False,
        Token::Undefined => SyntaxKind::Undefined,
        Token::Null => SyntaxKind::Null,

        Token::Select => SyntaxKind::Select,
        Token::Where => SyntaxKind::Where,
        Token::As => SyntaxKind::As,
        Token::Union => SyntaxKind::Union,
        Token::All => SyntaxKind::All,
        Token::Distinct => SyntaxKind::Distinct,
        Token::OrderBy => SyntaxKind::OrderBy,
        Token::TotalBy => SyntaxKind::TotalBy,
        Token::IntoTable => SyntaxKind::IntoTable,
        Token::AutoSort => SyntaxKind::AutoSort,
        Token::Asc => SyntaxKind::Asc,
        Token::Desc => SyntaxKind::Desc,

        Token::Try => SyntaxKind::Try,
        Token::Except => SyntaxKind::Except,
        Token::EndTry => SyntaxKind::EndTry,
        Token::Raise => SyntaxKind::Raise,

        Token::Return => SyntaxKind::Return,
        Token::New => SyntaxKind::New,

        Token::NumberLiteral(_) => SyntaxKind::NumberLiteral,
        Token::StringLiteral(_) => SyntaxKind::StringLiteral,
        Token::DateLiteral(_) => SyntaxKind::DateLiteral,

        Token::Identifier(_) => SyntaxKind::Identifier,
    }
}

struct ParseSession<'a> {
    builder: GreenNodeBuilder<'static>,
    lexer: &'a mut logos::Lexer<'a, Token>,
    current: Option<Token>,
    current_text: String,
    errors: Vec<String>,
}

impl<'a> ParseSession<'a> {
    fn new(lexer: &'a mut logos::Lexer<'a, Token>) -> Self {
        let mut session = ParseSession {
            builder: GreenNodeBuilder::new(),
            lexer,
            current: None,
            current_text: String::new(),
            errors: Vec::new(),
        };
        session.bump();
        session
    }

    fn bump(&mut self) {
        match self.lexer.next() {
            Some(Ok(token)) => {
                self.current_text = self.lexer.slice().to_string();
                self.current = Some(token);
            }
            Some(Err(err)) => {
                self.current_text = self.lexer.slice().to_string();
                self.errors.push(format!("Lexer error: {err:?}"));
                self.current = None;
            }
            None => {
                self.current_text.clear();
                self.current = None;
            }
        }
    }

    fn peek_is(&self, expected: &Token) -> bool {
        self.current
            .as_ref()
            .map(|t| std::mem::discriminant(t) == std::mem::discriminant(expected))
            .unwrap_or(false)
    }

    fn peek_in(&self, expected: &[&Token]) -> bool {
        self.current.as_ref().is_some_and(|t| {
            let disc = std::mem::discriminant(t);
            expected.iter().any(|e| std::mem::discriminant(*e) == disc)
        })
    }

    fn token_current(&mut self) {
        if let Some(token) = &self.current {
            let kind = token_kind(token);
            self.builder.token(kind.into(), &self.current_text);
            self.bump();
        }
    }

    fn expect_token(&mut self, expected: &Token) {
        if self.peek_is(expected) {
            self.token_current();
        } else {
            self.errors.push(format!(
                "Expected token {:?}, got {:?}",
                expected, self.current
            ));
            // Consume something to avoid infinite loops when parser gets out of sync.
            self.token_current();
        }
    }

    fn parse(&mut self) {
        self.builder.start_node(SyntaxKind::Root.into());
        self.builder.start_node(SyntaxKind::SourceFile.into());
        self.parse_statement_list();
        self.builder.finish_node(); // SourceFile
        self.builder.finish_node(); // Root
    }

    fn parse_statement_list(&mut self) {
        self.builder.start_node(SyntaxKind::StatementList.into());
        while self.current.is_some() {
            self.parse_statement();
        }
        self.builder.finish_node();
    }

    fn parse_statement(&mut self) {
        if self.peek_is(&Token::If) {
            self.parse_if();
        } else if self.peek_is(&Token::While) {
            self.parse_while();
        } else if self.peek_is(&Token::For) {
            self.parse_for_each();
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
        } else if self.peek_is(&Token::Continue) {
            self.parse_continue();
        } else {
            self.parse_expression_statement();
        }
    }

    fn parse_block(&mut self, terminators: &[&Token]) {
        self.builder.start_node(SyntaxKind::Block.into());
        while self.current.is_some() && !self.peek_in(terminators) {
            self.parse_statement();
        }
        self.builder.finish_node();
    }

    fn parse_if(&mut self) {
        self.builder.start_node(SyntaxKind::IfStatement.into());
        self.expect_token(&Token::If);
        self.parse_expression();
        self.expect_token(&Token::Then);
        self.parse_block(&[&Token::Else, &Token::ElseIf, &Token::EndIf]);

        while self.peek_is(&Token::ElseIf) {
            self.builder.start_node(SyntaxKind::IfStatement.into());
            self.expect_token(&Token::ElseIf);
            self.parse_expression();
            self.expect_token(&Token::Then);
            self.parse_block(&[&Token::Else, &Token::ElseIf, &Token::EndIf]);
            self.builder.finish_node();
        }

        if self.peek_is(&Token::Else) {
            self.expect_token(&Token::Else);
            self.parse_block(&[&Token::EndIf]);
        }

        self.expect_token(&Token::EndIf);
        self.builder.finish_node();
    }

    fn parse_while(&mut self) {
        self.builder.start_node(SyntaxKind::WhileStatement.into());
        self.expect_token(&Token::While);
        self.parse_expression();
        self.expect_token(&Token::Do);
        self.parse_block(&[&Token::EndDo]);
        self.expect_token(&Token::EndDo);
        self.builder.finish_node();
    }

    fn parse_for_each(&mut self) {
        self.builder.start_node(SyntaxKind::ForEachStatement.into());
        self.expect_token(&Token::For);
        self.expect_token(&Token::Each);
        self.parse_expression();
        self.expect_token(&Token::In);
        self.parse_expression();
        if self.peek_is(&Token::Do) {
            self.expect_token(&Token::Do);
        }
        self.parse_block(&[&Token::EndDo]);
        self.expect_token(&Token::EndDo);
        self.builder.finish_node();
    }

    fn parse_try(&mut self) {
        self.builder.start_node(SyntaxKind::TryStatement.into());
        self.expect_token(&Token::Try);
        self.parse_block(&[&Token::Except, &Token::EndTry]);

        if self.peek_is(&Token::Except) {
            self.expect_token(&Token::Except);
            self.parse_block(&[&Token::EndTry]);
        }

        self.expect_token(&Token::EndTry);
        self.builder.finish_node();
    }

    fn parse_procedure(&mut self) {
        self.builder.start_node(SyntaxKind::ProcedureDeclaration.into());
        self.expect_token(&Token::Procedure);
        if let Some(Token::Identifier(_)) = self.current.clone() {
            self.parse_identifier();
        }

        if self.peek_is(&Token::LParen) {
            self.expect_token(&Token::LParen);
            while !self.peek_is(&Token::RParen) && self.current.is_some() {
                let mut progressed = false;

                if self.peek_is(&Token::Val) {
                    self.expect_token(&Token::Val);
                    progressed = true;
                }

                if let Some(Token::Identifier(_)) = self.current.clone() {
                    self.parse_identifier();
                    progressed = true;

                    // Optional default value assignment for parameters
                    if self.peek_is(&Token::Eq) {
                        self.token_current();
                        self.parse_expression();
                    }
                }

                if self.peek_is(&Token::Comma) {
                    self.token_current();
                    progressed = true;
                }

                if !progressed {
                    self.errors.push(format!(
                        "Unexpected token in parameter list: {:?}",
                        self.current
                    ));
                    self.token_current();
                }
            }
            self.expect_token(&Token::RParen);
        }

        if self.peek_is(&Token::Export) {
            self.expect_token(&Token::Export);
        }

        self.parse_block(&[&Token::EndProcedure]);
        self.expect_token(&Token::EndProcedure);
        self.builder.finish_node();
    }

    fn parse_function(&mut self) {
        self.builder.start_node(SyntaxKind::FunctionDeclaration.into());
        self.expect_token(&Token::Function);
        if let Some(Token::Identifier(_)) = self.current.clone() {
            self.parse_identifier();
        }

        if self.peek_is(&Token::LParen) {
            self.expect_token(&Token::LParen);
            while !self.peek_is(&Token::RParen) && self.current.is_some() {
                let mut progressed = false;

                if self.peek_is(&Token::Val) {
                    self.expect_token(&Token::Val);
                    progressed = true;
                }

                if let Some(Token::Identifier(_)) = self.current.clone() {
                    self.parse_identifier();
                    progressed = true;

                    // Optional default value assignment for parameters
                    if self.peek_is(&Token::Eq) {
                        self.token_current();
                        self.parse_expression();
                    }
                }

                if self.peek_is(&Token::Comma) {
                    self.token_current();
                    progressed = true;
                }

                if !progressed {
                    self.errors.push(format!(
                        "Unexpected token in parameter list: {:?}",
                        self.current
                    ));
                    self.token_current();
                }
            }
            self.expect_token(&Token::RParen);
        }

        if self.peek_is(&Token::Export) {
            self.expect_token(&Token::Export);
        }

        self.parse_block(&[&Token::EndFunction]);
        self.expect_token(&Token::EndFunction);
        self.builder.finish_node();
    }

    fn parse_var_decl(&mut self) {
        self.builder.start_node(SyntaxKind::VariableDeclaration.into());
        self.expect_token(&Token::Var);
        if let Some(Token::Identifier(_)) = self.current.clone() {
            self.parse_identifier();
        }
        if self.peek_is(&Token::Eq) {
            self.token_current();
            self.parse_expression();
        }
        if self.peek_is(&Token::Semicolon) {
            self.token_current();
        }
        self.builder.finish_node();
    }

    fn parse_return(&mut self) {
        self.builder.start_node(SyntaxKind::ReturnStatement.into());
        self.expect_token(&Token::Return);
        if self.current.is_some() && !self.peek_is(&Token::Semicolon) {
            self.parse_expression();
        }
        if self.peek_is(&Token::Semicolon) {
            self.token_current();
        }
        self.builder.finish_node();
    }

    fn parse_break(&mut self) {
        self.builder.start_node(SyntaxKind::BreakStatement.into());
        self.expect_token(&Token::Break);
        if self.peek_is(&Token::Semicolon) {
            self.token_current();
        }
        self.builder.finish_node();
    }

    fn parse_continue(&mut self) {
        self.builder.start_node(SyntaxKind::ContinueStatement.into());
        self.expect_token(&Token::Continue);
        if self.peek_is(&Token::Semicolon) {
            self.token_current();
        }
        self.builder.finish_node();
    }

    fn parse_expression_statement(&mut self) {
        self.builder.start_node(SyntaxKind::ExpressionStatement.into());
        self.parse_expression();
        if self.peek_is(&Token::Semicolon) {
            self.token_current();
        }
        self.builder.finish_node();
    }

    fn parse_expression(&mut self) {
        self.builder.start_node(SyntaxKind::Expression.into());
        self.parse_binary_expression(0);
        self.builder.finish_node();
    }

    fn parse_binary_expression(&mut self, min_bp: u8) {
        let checkpoint = self.builder.checkpoint();
        self.parse_unary();

        while let Some(op) = self.current.clone() {
            if let Some((l_bp, r_bp)) = Self::infix_binding_power(&op) {
                if l_bp < min_bp {
                    break;
                }

                self.builder.start_node_at(checkpoint, SyntaxKind::BinaryExpression.into());
                self.token_current();
                self.parse_binary_expression(r_bp);
                self.builder.finish_node();
                continue;
            }

            break;
        }
    }

    fn parse_unary(&mut self) {
        if self.peek_is(&Token::Not) || self.peek_is(&Token::Minus) {
            let checkpoint = self.builder.checkpoint();
            self.builder.start_node_at(checkpoint, SyntaxKind::UnaryExpression.into());
            self.token_current();
            self.parse_unary();
            self.builder.finish_node();
            return;
        }
        self.parse_primary();
    }

    fn parse_primary(&mut self) {
        match &self.current {
            Some(Token::Identifier(_)) => self.parse_identifier(),
            Some(Token::NumberLiteral(_))
            | Some(Token::StringLiteral(_))
            | Some(Token::DateLiteral(_))
            | Some(Token::True)
            | Some(Token::False)
            | Some(Token::Undefined)
            | Some(Token::Null) => self.parse_literal(),
            Some(Token::LParen) => {
                self.token_current();
                self.parse_expression();
                self.expect_token(&Token::RParen);
            }
            Some(token) => {
                self.errors.push(format!("Unexpected token in expression: {:?}", token));
                self.token_current();
            }
            None => {}
        }
    }

    fn parse_identifier(&mut self) {
        self.builder.start_node(SyntaxKind::Identifier.into());
        self.token_current();
        self.builder.finish_node();
    }

    fn parse_literal(&mut self) {
        self.builder.start_node(SyntaxKind::Literal.into());
        self.token_current();
        self.builder.finish_node();
    }

    fn infix_binding_power(token: &Token) -> Option<(u8, u8)> {
        match token {
            Token::Or => Some((1, 2)),
            Token::And => Some((3, 4)),
            Token::Eq | Token::Ne | Token::Lt | Token::Gt | Token::Le | Token::Ge => Some((5, 6)),
            Token::Plus | Token::Minus => Some((7, 8)),
            Token::Star | Token::Slash => Some((9, 10)),
            _ => None,
        }
    }
}

pub fn build_ast<'a>(lexer: &'a mut logos::Lexer<'a, Token>) -> Parser {
    let mut session = ParseSession::new(lexer);
    session.parse();

    Parser {
        tree: session.builder.finish(),
        errors: session.errors,
    }
}

use crate::SyntaxKind;
use onessa_lexer::Token;

pub fn token_kind(token: &Token) -> SyntaxKind {
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

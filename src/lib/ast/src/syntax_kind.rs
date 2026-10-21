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
    CallExpression,
    MemberExpression,
    NewExpression,
    ForStatement,
    AssignmentStatement,
    RaiseStatement,
    ElseIfClause,
    ElseClause,
    Parameter,
    ParenExpression,
    TernaryExpression,
    IndexExpression,

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
    Percent,
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
    To,
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
    /// Whitespace and comments between tokens, kept so that the tree text equals the source.
    Trivia,
    Error,
}

impl From<SyntaxKind> for rowan::SyntaxKind {
    fn from(value: SyntaxKind) -> Self {
        Self(value as u16)
    }
}

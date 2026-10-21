use crate::{build_ast, Lang, SyntaxKind};
use logos::Logos;
use onessa_lexer::Token;
use rowan::{NodeOrToken, SyntaxNode};
use std::fmt::Write;

fn parse(source: &str) -> (SyntaxNode<Lang>, Vec<String>) {
    let mut lexer = Token::lexer(source);
    let tree = build_ast(&mut lexer);
    (tree.syntax(), tree.errors)
}

/// Renders the node kinds of a tree (tokens omitted) as an S-expression.
fn shape(node: &SyntaxNode<Lang>) -> String {
    let mut out = String::new();
    write!(out, "({:?}", node.kind()).unwrap();
    for child in node.children_with_tokens() {
        if let NodeOrToken::Node(n) = child {
            write!(out, " {}", shape(&n)).unwrap();
        }
    }
    out.push(')');
    out
}

fn first_statement(source: &str) -> String {
    let (root, errors) = parse(source);
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
    let list = root
        .descendants()
        .find(|n| n.kind() == SyntaxKind::StatementList)
        .unwrap();
    shape(&list.first_child().unwrap())
}

#[test]
fn assets_parse_without_errors() {
    let assets = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../assets");
    for entry in std::fs::read_dir(assets).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "cf") {
            let source = std::fs::read_to_string(&path).unwrap();
            let (_, errors) = parse(&source);
            assert!(errors.is_empty(), "{}: {errors:?}", path.display());
        }
    }
}

#[test]
fn assignment_is_not_comparison() {
    assert_eq!(
        first_statement("А = Б = В;"),
        "(AssignmentStatement (Identifier) (Expression (BinaryExpression (Identifier) (Identifier))))"
    );
}

#[test]
fn call_statement() {
    assert_eq!(
        first_statement("Сообщить(1, , 2);"),
        "(ExpressionStatement (CallExpression (Identifier) (Expression (Literal)) (Expression) (Expression (Literal))))"
    );
}

#[test]
fn not_binds_looser_than_comparison() {
    assert_eq!(
        first_statement("А = Не Б = В И Г;"),
        "(AssignmentStatement (Identifier) (Expression (BinaryExpression (UnaryExpression (BinaryExpression (Identifier) (Identifier))) (Identifier))))"
    );
}

#[test]
fn precedence_and_parens() {
    assert_eq!(
        first_statement("А = -1 + 2 * (3 - 4) % 5;"),
        "(AssignmentStatement (Identifier) (Expression (BinaryExpression (UnaryExpression (Literal)) (BinaryExpression (BinaryExpression (Literal) (ParenExpression (Expression (BinaryExpression (Literal) (Literal))))) (Literal)))))"
    );
}

#[test]
fn ternary_and_index() {
    assert_eq!(
        first_statement("А = ?(Б, М[0], 2);"),
        "(AssignmentStatement (Identifier) (Expression (TernaryExpression (Expression (Identifier)) (Expression (IndexExpression (Identifier) (Expression (Literal)))) (Expression (Literal)))))"
    );
}

#[test]
fn for_loops() {
    assert_eq!(
        first_statement("Для Сч = 1 По 10 Цикл Прервать; КонецЦикла;"),
        "(ForStatement (Identifier) (Expression (Literal)) (Expression (Literal)) (Block (BreakStatement)))"
    );
    assert_eq!(
        first_statement("Для Каждого Эл Из Список Цикл КонецЦикла;"),
        "(ForEachStatement (Identifier) (Expression (Identifier)) (Block))"
    );
}

#[test]
fn if_chain() {
    assert_eq!(
        first_statement("Если А Тогда ИначеЕсли Б Тогда Иначе Возврат КонецЕсли;"),
        "(IfStatement (Expression (Identifier)) (Block) (ElseIfClause (Expression (Identifier)) (Block)) (ElseClause (Block (ReturnStatement))))"
    );
}

#[test]
fn function_signature() {
    assert_eq!(
        first_statement("Функция Ф(Знач А, Б = 1) Экспорт Возврат А; КонецФункции"),
        "(FunctionDeclaration (Identifier) (Parameter (Identifier)) (Parameter (Identifier) (Expression (Literal))) (Block (ReturnStatement (Expression (Identifier)))))"
    );
}

#[test]
fn raise_and_try() {
    assert_eq!(
        first_statement("Попытка ВызватьИсключение \"x\"; Исключение ВызватьИсключение; КонецПопытки;"),
        "(TryStatement (Block (RaiseStatement (Expression (Literal)))) (Block (RaiseStatement)))"
    );
}

#[test]
fn reports_errors() {
    let (_, errors) = parse("А = ;");
    assert!(!errors.is_empty());
}

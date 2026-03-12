/// Этот файл содержит лексер для языка и юнит-тесты, подтверждающие корректную работу.
use logos::Logos;

#[derive(Logos, Debug, PartialEq, Clone)]
#[logos(skip(r"[ \t\n\f]+"))]
pub enum Token {
    #[regex(r"//[^\n]*", allow_greedy=true, callback=logos::skip)]
    LineComment,

    // 2. Многосимвольные операторы (чтобы <> не распалось на < и >)
    #[token("<>")]
    Ne,

    #[token("<=")]
    Le,

    #[token(">=")]
    Ge,

    // 3. Односимвольные операторы
    #[token("=")]
    Eq,

    #[token("<")]
    Lt,

    #[token(">")]
    Gt,

    #[token("+")]
    Plus,

    #[token("-")]
    Minus,

    #[token("*")]
    Star,

    #[token("/")]
    Slash,

    #[token(";")]
    Semicolon,

    #[token(",")]
    Comma,

    #[token("(")]
    LParen,

    #[token(")")]
    RParen,

    // 4. Ключевые слова (с приоритетом выше, чем у идентификаторов)
    // Операторы цикла и условия
    #[regex(r"(?i)если|if", priority = 2)]
    If,

    #[regex(r"(?i)тогда|then", priority = 2)]
    Then,

    #[regex(r"(?i)иначе|else", priority = 2)]
    Else,

    #[regex(r"(?i)иначеесли|elseif|elsif", priority = 2)]
    ElseIf,

    #[regex(r"(?i)конецесли|endif", priority = 2)]
    EndIf,

    #[regex(r"(?i)для|for", priority = 2)]
    For,

    #[regex(r"(?i)каждого|each", priority = 2)]
    Each,

    #[regex(r"(?i)из|from|in", priority = 2)]
    In,

    #[regex(r"(?i)пока|while", priority = 2)]
    While,

    #[regex(r"(?i)цикл|do", priority = 2)]
    Do,

    #[regex(r"(?i)конеццикла|enddo|endwhile", priority = 2)]
    EndDo,

    #[regex(r"(?i)прервать|break", priority = 2)]
    Break,

    #[regex(r"(?i)продолжить|continue", priority = 2)]
    Continue,

    // Объявление переменных и процедур/функций
    #[regex(r"(?i)перем|var", priority = 2)]
    Var,

    #[regex(r"(?i)процедура|procedure", priority = 2)]
    Procedure,

    #[regex(r"(?i)функция|function", priority = 2)]
    Function,

    #[regex(r"(?i)конецпроцедуры|endprocedure", priority = 2)]
    EndProcedure,

    #[regex(r"(?i)конецфункции|endfunction", priority = 2)]
    EndFunction,

    // Экспорт/импорт
    #[regex(r"(?i)экспорт|export", priority = 2)]
    Export,

    #[regex(r"(?i)знач|val", priority = 2)]
    Val,

    // Логические операторы
    #[regex(r"(?i)и|and", priority = 2)]
    And,

    #[regex(r"(?i)или|or", priority = 2)]
    Or,

    #[regex(r"(?i)не|not", priority = 2)]
    Not,

    // Литералы
    #[regex(r"(?i)истина|true", priority = 2)]
    True,

    #[regex(r"(?i)ложь|false", priority = 2)]
    False,

    #[regex(r"(?i)неопределено|undefined", priority = 2)]
    Undefined,

    #[regex(r"(?i)null", priority = 2)]
    Null,

    // Работа с данными
    #[regex(r"(?i)выбрать|select", priority = 2)]
    Select,

    #[regex(r"(?i)где|where", priority = 2)]
    Where,

    #[regex(r"(?i)как|as", priority = 2)]
    As,

    #[regex(r"(?i)объединить|union", priority = 2)]
    Union,

    #[regex(r"(?i)все|all", priority = 2)]
    All,

    #[regex(r"(?i)различные|distinct", priority = 2)]
    Distinct,

    #[regex(r"(?i)упорядочитьпо|orderby", priority = 2)]
    OrderBy,

    #[regex(r"(?i)итогипо|totalby", priority = 2)]
    TotalBy,

    #[regex(r"(?i)помещениевтаблицу|intotable", priority = 2)]
    IntoTable,

    #[regex(r"(?i)автоупорядочивание|autosort", priority = 2)]
    AutoSort,

    #[regex(r"(?i)возрастание|asc", priority = 2)]
    Asc,

    #[regex(r"(?i)убывание|desc", priority = 2)]
    Desc,

    // Исключения
    #[regex(r"(?i)попытка|try", priority = 2)]
    Try,

    #[regex(r"(?i)исключение|except", priority = 2)]
    Except,

    #[regex(r"(?i)конецпопытки|endtry", priority = 2)]
    EndTry,

    #[regex(r"(?i)вызватьисключение|raise", priority = 2)]
    Raise,

    // Возврат значения
    #[regex(r"(?i)возврат|return", priority = 2)]
    Return,

    // Новый объект
    #[regex(r"(?i)новый|new", priority = 2)]
    New,

    // 5. Литералы
    #[regex(r"-?\d+(\.\d+)?", priority = 3)]
    NumberLiteral,

    #[regex(r#""([^"\\]|\\["\\nrt\t])*""#, priority = 3)]
    StringLiteral,

    #[regex(r"'\d{8,}'", priority = 3)]
    DateLiteral,

    // 6. Идентификаторы (В САМОМ КОНЦЕ, с наименьшим приоритетом)
    #[regex(r"[_\p{XID_Start}][\p{XID_Continue}]*", priority = 1)]
    Identifier,
}

#[cfg(test)]
mod tests {
    use super::*;
    use logos::Logos;

    fn assert_tokens(input: &str, expected: &[Token]) {
        let mut lex = Token::lexer(input);
        for token in expected {
            assert_eq!(lex.next(), Some(Ok(token.clone())));
        }
        assert_eq!(lex.next(), None);
    }

    #[test]
    fn test_if_keywords() {
        assert_tokens(
            "если if тогда then иначеесли elseif elsif иначе else",
            &[
                Token::If,
                Token::If,
                Token::Then,
                Token::Then,
                Token::ElseIf,
                Token::ElseIf,
                Token::ElseIf,
                Token::Else,
                Token::Else,
            ],
        );
    }
    #[test]
    fn test_if_keywords_upper() {
        assert_tokens(
            "ЕСЛИ IF ТОГДА THEN ИНАЧЕЕСЛИ ELSEIF ELSIF ИНАЧЕ ELSE",
            &[
                Token::If,
                Token::If,
                Token::Then,
                Token::Then,
                Token::ElseIf,
                Token::ElseIf,
                Token::ElseIf,
                Token::Else,
                Token::Else,
            ],
        );
    }

    #[test]
    fn test_comments() {
        assert_tokens(
            "Если Тогда\n\
            // Тут какой-то комментарий\n\
            ИначеЕсли Тогда\n\
            // Тут опять какой-то комментарий\n\
            Иначе \n\
            // Ну последний комментарий\n\
            КонецЕсли\n\n",
            &[
                Token::If,
                Token::Then,
                Token::ElseIf,
                Token::Then,
                Token::Else,
                Token::EndIf,
            ],
        );
    }

    #[test]
    fn test_identifiers() {
        assert_tokens(
            "Перем А = 5;",
            &[
                Token::Var,
                Token::Identifier,
                Token::Eq,
                Token::NumberLiteral,
                Token::Semicolon,
            ],
        );
    }

    #[test]
    fn test_operators_and_punctuation() {
        assert_tokens(
            "<> <= >= = < > + - * / ; , ( )",
            &[
                Token::Ne,
                Token::Le,
                Token::Ge,
                Token::Eq,
                Token::Lt,
                Token::Gt,
                Token::Plus,
                Token::Minus,
                Token::Star,
                Token::Slash,
                Token::Semicolon,
                Token::Comma,
                Token::LParen,
                Token::RParen,
            ],
        );
    }

    #[test]
    fn test_loop_keywords_ru_en() {
        assert_tokens(
            "Для каждого из Пока Цикл КонецЦикла для each in while do enddo endwhile",
            &[
                Token::For,
                Token::Each,
                Token::In,
                Token::While,
                Token::Do,
                Token::EndDo,
                Token::For,
                Token::Each,
                Token::In,
                Token::While,
                Token::Do,
                Token::EndDo,
                Token::EndDo,
            ],
        );
    }

    #[test]
    fn test_proc_func_and_export_val() {
        assert_tokens(
            "Процедура Тест Экспорт КонецПроцедуры Функция F Знач КонецФункции",
            &[
                Token::Procedure,
                Token::Identifier,
                Token::Export,
                Token::EndProcedure,
                Token::Function,
                Token::Identifier,
                Token::Val,
                Token::EndFunction,
            ],
        );
    }

    #[test]
    fn test_logical_and_literals() {
        assert_tokens(
            "И Или Не Истина Ложь Неопределено Null and or not true false undefined null",
            &[
                Token::And,
                Token::Or,
                Token::Not,
                Token::True,
                Token::False,
                Token::Undefined,
                Token::Null,
                Token::And,
                Token::Or,
                Token::Not,
                Token::True,
                Token::False,
                Token::Undefined,
                Token::Null,
            ],
        );
    }

    #[test]
    fn test_literals_number_string_date() {
        assert_tokens(
            "0 -1 12.5 \"строка\" \"строка\\n\\t\\\"ок\\\"\" '20240101' '19990101000000'",
            &[
                Token::NumberLiteral,
                Token::NumberLiteral,
                Token::NumberLiteral,
                Token::StringLiteral,
                Token::StringLiteral,
                Token::DateLiteral,
                Token::DateLiteral,
            ],
        );
    }

    #[test]
    fn test_select_query_keywords() {
        assert_tokens(
            "Выбрать Где Как Объединить Все Различные УпорядочитьПо ИтогиПо ПомещениеВТаблицу Автоупорядочивание Возрастание Убывание",
            &[
                Token::Select,
                Token::Where,
                Token::As,
                Token::Union,
                Token::All,
                Token::Distinct,
                Token::OrderBy,
                Token::TotalBy,
                Token::IntoTable,
                Token::AutoSort,
                Token::Asc,
                Token::Desc,
            ],
        );
    }

    #[test]
    fn test_exception_flow_and_return_new() {
        assert_tokens(
            "Попытка Исключение КонецПопытки ВызватьИсключение Возврат Новый Try Except EndTry Raise Return New",
            &[
                Token::Try,
                Token::Except,
                Token::EndTry,
                Token::Raise,
                Token::Return,
                Token::New,
                Token::Try,
                Token::Except,
                Token::EndTry,
                Token::Raise,
                Token::Return,
                Token::New,
            ],
        );
    }

    #[test]
    fn test_identifier_unicode_and_underscore() {
        assert_tokens(
            "_a _переменная ИмяПеременной имя_переменной",
            &[
                Token::Identifier,
                Token::Identifier,
                Token::Identifier,
                Token::Identifier,
            ],
        );
    }

    #[test]
    fn test_procedure_declaration_example() {
        assert_tokens(
            "Процедура СформироватьОтчет(Период, Знач Параметр) Экспорт\n\
            Перем Сумма;\n\
            Сумма = 0;\n\
            КонецПроцедуры",
            &[
                Token::Procedure,
                Token::Identifier,
                Token::LParen,
                Token::Identifier,
                Token::Comma,
                Token::Val,
                Token::Identifier,
                Token::RParen,
                Token::Export,
                Token::Var,
                Token::Identifier,
                Token::Semicolon,
                Token::Identifier,
                Token::Eq,
                Token::NumberLiteral,
                Token::Semicolon,
                Token::EndProcedure,
            ],
        );
    }

    #[test]
    fn test_function_with_return_example() {
        assert_tokens(
            "Функция Рассчитать(Знач A, Знач B)\n\
            Возврат A + B;\n\
            КонецФункции",
            &[
                Token::Function,
                Token::Identifier,
                Token::LParen,
                Token::Val,
                Token::Identifier,
                Token::Comma,
                Token::Val,
                Token::Identifier,
                Token::RParen,
                Token::Return,
                Token::Identifier,
                Token::Plus,
                Token::Identifier,
                Token::Semicolon,
                Token::EndFunction,
            ],
        );
    }

    #[test]
    fn test_if_else_with_compare_example() {
        assert_tokens(
            "Если A <> B Тогда\n\
            Возврат Истина;\n\
            Иначе\n\
            Возврат Ложь;\n\
            КонецЕсли",
            &[
                Token::If,
                Token::Identifier,
                Token::Ne,
                Token::Identifier,
                Token::Then,
                Token::Return,
                Token::True,
                Token::Semicolon,
                Token::Else,
                Token::Return,
                Token::False,
                Token::Semicolon,
                Token::EndIf,
            ],
        );
    }

    #[test]
    fn test_while_loop_example() {
        assert_tokens(
            "Пока Счетчик < 10 Цикл\n\
            Счетчик = Счетчик + 1;\n\
            КонецЦикла",
            &[
                Token::While,
                Token::Identifier,
                Token::Lt,
                Token::NumberLiteral,
                Token::Do,
                Token::Identifier,
                Token::Eq,
                Token::Identifier,
                Token::Plus,
                Token::NumberLiteral,
                Token::Semicolon,
                Token::EndDo,
            ],
        );
    }

    #[test]
    fn test_try_except_raise_example() {
        assert_tokens(
            "Попытка\n\
            ВызватьИсключение;\n\
            Исключение\n\
            Возврат Неопределено;\n\
            КонецПопытки",
            &[
                Token::Try,
                Token::Raise,
                Token::Semicolon,
                Token::Except,
                Token::Return,
                Token::Undefined,
                Token::Semicolon,
                Token::EndTry,
            ],
        );
    }
}

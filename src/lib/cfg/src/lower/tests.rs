use crate::{build_cfg, Diagnostic, Module};
use logos::Logos;
use onessa_lexer::Token;

fn lower(source: &str) -> Result<Module, Vec<Diagnostic>> {
    let mut lexer = Token::lexer(source);
    let tree = onessa_ast::build_ast(&mut lexer);
    assert!(tree.errors.is_empty(), "parse errors: {:?}", tree.errors);
    build_cfg(&tree.syntax())
}

fn dump(source: &str) -> String {
    match lower(source) {
        Ok(module) => module.to_string(),
        Err(errors) => panic!("unexpected errors: {errors:?}"),
    }
}

fn errors(source: &str) -> Vec<String> {
    match lower(source) {
        Ok(module) => panic!("expected errors, got:\n{module}"),
        Err(errors) => errors.iter().map(|e| e.to_string()).collect(),
    }
}

#[test]
fn module_body_assignments_and_calls() {
    assert_eq!(
        dump("А = 1 + 2;\nСообщить(А);"),
        "\
fn#0 (entry) <module>()
  locals: [А]
  b0:
    PUSH_C 1
    PUSH_C 2
    ADD
    POP_V 0
    PUSH_V 0
    CALL_NATIVE str#0 1
    POP
    PUSH_C undefined
    return
"
    );
}

#[test]
fn globals_and_methods() {
    let source = "\
Перем Счетчик;
Процедура Увеличить(Шаг = 1)
    Счетчик = Счетчик + Шаг;
КонецПроцедуры
Функция Удвоить(Знач Х) Экспорт
    Возврат Х * 2;
КонецФункции
Счетчик = 0;
Увеличить();
";
    assert_eq!(
        dump(source),
        "\
globals: Счетчик
fn#0 Увеличить(Шаг = 1)
  locals: [Шаг]
  b0:
    PUSH_G 0
    PUSH_V 0
    ADD
    POP_G 0
    PUSH_C undefined
    return
fn#1 Удвоить(val Х)
  locals: [Х]
  b0:
    PUSH_V 0
    PUSH_C 2
    MUL
    return
fn#2 (entry) <module>()
  locals: []
  b0:
    PUSH_C 0
    POP_G 0
    PUSH_C 1
    CALL fn#0 1
    POP
    PUSH_C undefined
    return
"
    );
}

#[test]
fn by_reference_arguments_are_written_back() {
    let source = "\
Процедура П(А, Знач Б)
    А = Б;
КонецПроцедуры
Процедура Читает(А)
    Сообщить(А);
КонецПроцедуры
Х = 1;
Y = 2;
П(Х, Y);
П(Х + 1, Y);
Читает(Х);
";
    let dump = dump(source);
    assert!(
        dump.contains("    CALL fn#0 2\n    PUSH_OUT 0\n    POP_V 0\n    POP\n"),
        "{dump}"
    );
    // Only the first call writes back: `Читает` never changes its parameter.
    assert_eq!(dump.matches("PUSH_OUT").count(), 1, "{dump}");
}

#[test]
fn if_and_loops() {
    let source = "\
Сч = 0;
Пока Сч < 3 Цикл
    Если Сч = 1 Тогда
        Продолжить;
    ИначеЕсли Сч = 2 Тогда
        Прервать;
    Иначе
        Сч = Сч + 1;
    КонецЕсли;
КонецЦикла;
";
    let dump = dump(source);
    assert!(dump.contains("branch"), "{dump}");
    assert!(!dump.contains("<unterminated>"), "{dump}");
}

#[test]
fn for_loop_evaluates_bound_once() {
    let dump = dump("Для Сч = 1 По 3 Цикл КонецЦикла;");
    assert_eq!(
        dump,
        "\
fn#0 (entry) <module>()
  locals: [Сч, $0]
  b0:
    PUSH_C 1
    POP_V 0
    PUSH_C 3
    POP_V 1
    jump b1
  b1:
    for_check 0 1 b2 b4
  b2:
    jump b3
  b3:
    for_step 0 1 b2 b4
  b4:
    PUSH_C undefined
    return
"
    );
}

#[test]
fn for_loop_with_module_variable_counter() {
    let dump = dump("Перем Сч;\nДля Сч = 1 По 3 Цикл КонецЦикла;");
    assert!(dump.contains("PUSH_G 0\n    PUSH_V 0\n    LE"), "{dump}");
    assert!(!dump.contains("for_step"), "{dump}");
}

#[test]
fn break_inside_try_uninstalls_handler() {
    let source = "\
Пока Истина Цикл
    Попытка
        Прервать;
    Исключение
        ВызватьИсключение;
    КонецПопытки;
КонецЦикла;
";
    let dump = dump(source);
    assert!(dump.contains("TRY_BEGIN"), "{dump}");
    assert!(dump.contains("    TRY_END\n    jump"), "{dump}");
    assert!(dump.contains("reraise"), "{dump}");
}

#[test]
fn short_circuit_and_ternary() {
    let dump = dump("А = Истина И Ложь ИЛИ ?(Истина, 1, 2) = 1;");
    assert_eq!(dump.matches("DUP").count(), 2, "{dump}");
    assert!(!dump.contains("<unterminated>"), "{dump}");
}

#[test]
fn unreachable_code_is_dropped() {
    let dump = dump("Функция Ф()\n Возврат 1;\n Сообщить(2);\nКонецФункции");
    assert!(!dump.contains("CALL_NATIVE"), "{dump}");
}

#[test]
fn semantic_errors() {
    assert_eq!(errors("А = Б;"), ["Variable is not defined (Б) at 1:5"]);
    assert_eq!(
        errors("Нет();"),
        ["Procedure or function is not defined (Нет) at 1:1"]
    );
    assert_eq!(
        errors("Процедура П()\nКонецПроцедуры\nА = П();"),
        ["Procedure cannot be used as a function (П) at 3:5"]
    );
    assert_eq!(
        errors("Функция Ф(А)\nВозврат;\nКонецФункции\nФ(1, 2);"),
        [
            "Function must return a value at 2:1",
            "Too many actual parameters (Ф) at 4:1"
        ]
    );
    assert_eq!(
        errors("Процедура П(А)\nКонецПроцедуры\nП();"),
        ["Not enough actual parameters (П) at 3:1"]
    );
    assert_eq!(
        errors("Прервать;"),
        ["'Прервать' can only be used inside a loop at 1:1"]
    );
    assert_eq!(
        errors("Возврат;"),
        ["'Возврат' is not allowed in the module body at 1:1"]
    );
    assert_eq!(
        errors("ВызватьИсключение;"),
        ["'ВызватьИсключение' without a message is only allowed inside 'Исключение' at 1:1"]
    );
    assert_eq!(
        errors("Перем А;\nПерем а;"),
        ["Variable is already defined (а) at 2:7"]
    );
    assert_eq!(
        errors("Процедура П()\nКонецПроцедуры\nПроцедура п()\nКонецПроцедуры"),
        ["Procedure or function is already defined (п) at 3:11"]
    );
    assert_eq!(
        errors("Процедура П(А = Б)\nКонецПроцедуры"),
        ["Default parameter value must be a constant at 1:17"]
    );
    assert_eq!(
        errors("А = Сообщить(1);"),
        ["Procedure cannot be used as a function (Сообщить) at 1:5"]
    );
    assert_eq!(
        errors("А;"),
        ["Expected a procedure call or an assignment at 1:1"]
    );
    assert_eq!(
        errors("А = Новый Массив;\nА.Б = 1;\nСообщить(А.Б);"),
        [
            "Only variables and indexed elements can be assigned to: object properties are not supported yet at 2:1",
            "Object properties are not supported yet at 3:10"
        ]
    );
}

#[test]
fn objects_indexing_and_methods() {
    assert_eq!(
        dump("М = Новый Массив(2);\nМ[0] = М.Количество();\nМ.Добавить(М[1]);"),
        "\
fn#0 (entry) <module>()
  locals: [М]
  b0:
    PUSH_C 2
    NEW str#0 1
    POP_V 0
    PUSH_V 0
    PUSH_C 0
    PUSH_V 0
    CALL_METHOD str#1 0
    INDEX_SET
    PUSH_V 0
    PUSH_V 0
    PUSH_C 1
    INDEX_GET
    CALL_METHOD str#2 1
    POP
    PUSH_C undefined
    return
"
    );
}

#[test]
fn names_are_case_insensitive() {
    dump("Перем Имя;\nимя = 1;\nИМЯ = ИмЯ + 1;\nсообщить(имя);");
}

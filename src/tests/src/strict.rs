//! Strict mode (`--strict`): static types of variables.

use crate::{compile_with, run_with};

fn errors(source: &str) -> Vec<String> {
    match compile_with(source, true) {
        Ok(_) => panic!("expected compile errors"),
        Err(errors) => errors,
    }
}

fn lines(source: &str) -> Vec<String> {
    match run_with(source, true) {
        Ok(output) => output.lines().map(str::to_string).collect(),
        Err((output, err)) => panic!("uncaught exception: {err}\noutput:\n{output}"),
    }
}

fn exception(source: &str) -> String {
    match run_with(source, true) {
        Ok(output) => panic!("expected an exception, got:\n{output}"),
        Err((_, err)) => err,
    }
}

#[test]
fn variables_keep_their_first_type() {
    assert_eq!(
        errors("А = 1;\nА = \"а\";\nБ = \"б\";\nБ = '20240101';"),
        [
            "Type mismatch: variable А is Число, cannot assign Строка at 2:1",
            "Type mismatch: variable Б is Строка, cannot assign Дата at 4:1",
        ]
    );
    assert_eq!(
        lines("А = 1;\nА = А * 2.5;\nБ = Новый Массив;\nБ = Новый Массив(2);\nСообщить(А);"),
        ["2,5"]
    );
    // The same rules apply to module variables and to locals declared with `Перем`.
    assert_eq!(
        errors(
            "Перем Г;\nПроцедура П()\n  Перем Л;\n  Г = 1;\n  Л = Истина;\n  Л = 0;\nКонецПроцедуры\nГ = \"г\";"
        ),
        [
            "Type mismatch: variable Л is Булево, cannot assign Число at 6:3",
            "Type mismatch: variable Г is Число, cannot assign Строка at 8:1",
        ]
    );
}

#[test]
fn unknown_types_are_dynamic_or_checked() {
    // `Неопределено`, parameters and array elements have no static type.
    assert_eq!(
        lines(
            "Функция Ф(П)\n  П = \"строка\";\n  Возврат П;\nКонецФункции
             Х = Неопределено; Х = 1; Х = \"а\";
             Сообщить(Ф(1) + Х);"
        ),
        ["строкаа"]
    );
    // A value of an unknown type stored into a typed variable is checked at run time.
    let source = "М = Новый Массив; М.Добавить(\"а\");\nЧ = 0;\nЧ = М[0];";
    assert_eq!(
        exception(source),
        "test.bsl:3: Несоответствие типов: ожидается Число, получено Строка"
    );
    assert!(run_with(&source.replace("\"а\"", "5"), true).is_ok());
    // Also when a by-reference parameter changes the type of the argument.
    assert_eq!(
        exception("Процедура П(А)\n  А = \"а\";\nКонецПроцедуры\nЧ = 1;\nП(Ч);"),
        "test.bsl:5: Несоответствие типов: ожидается Число, получено Строка"
    );
}

#[test]
fn function_results_are_inferred() {
    // `Имя` returns Строка. `Тот` returns its parameter, which has no static type, so its
    // result is Произвольный and the variable it initializes accepts anything.
    assert_eq!(
        errors(
            "Функция Имя()
               Возврат \"имя\";
             КонецФункции
             Функция Тот(З)
               Возврат З;
             КонецФункции
             Х = Имя();
             Х = 1;
             Ч = Тот(10);
             Ч = \"а\";"
        ),
        ["Type mismatch: variable Х is Строка, cannot assign Число at 8:14"]
    );
}

#[test]
fn recursive_results_are_inferred() {
    assert_eq!(
        errors(
            "Функция Факториал(Знач Н)
               Если Н <= 1 Тогда
                 Возврат 1;
               КонецЕсли;
               Возврат Н * Факториал(Н - 1);
             КонецФункции
             Х = \"\";
             Х = Факториал(5);"
        ),
        ["Type mismatch: variable Х is Строка, cannot assign Число at 8:14"]
    );
    assert_eq!(
        lines(
            "Функция Факториал(Знач Н)
               Если Н <= 1 Тогда
                 Возврат 1;
               КонецЕсли;
               Возврат Н * Факториал(Н - 1);
             КонецФункции
             Х = Факториал(20);
             Сообщить(Х);"
        ),
        ["2\u{a0}432\u{a0}902\u{a0}008\u{a0}176\u{a0}640\u{a0}000"]
    );
}

#[test]
fn operators_and_conditions_are_checked() {
    assert_eq!(
        errors(
            "С = \"с\"; Ч = 1; Д = '20240101'; Б = Истина;
             А = Ч + С;
             А = С * 2;
             А = Ч < С;
             А = -С;
             Если Ч Тогда КонецЕсли;
             Пока С Цикл КонецЦикла;
             А = Не Ч;
             А = Б И Ч;
             А = Д + Д;"
        ),
        [
            "Operator '+' cannot be applied to Число and Строка at 2:18",
            "Operator '*' cannot be applied to Строка and Число at 3:18",
            "Operator '<' cannot be applied to Число and Строка at 4:18",
            "Operator '-' cannot be applied to Строка at 5:18",
            "Condition must be Булево, not Число at 6:19",
            "Condition must be Булево, not Строка at 7:19",
            "Operator 'НЕ' cannot be applied to Число at 8:18",
            "Condition must be Булево, not Число at 9:22",
            "Operator '+' cannot be applied to Дата and Дата at 10:18",
        ]
    );
    // Operations that are valid for the types.
    assert_eq!(
        lines(
            "С = \"с\"; Ч = 1; Д = '20240101';
             Сообщить(С + Ч + Истина);
             Сообщить(Д + 86400 - Д);
             Сообщить(?(Ч > 0 И Не (Ч = 2), \"да\", \"нет\"));"
        ),
        ["с1Да", "86\u{a0}400", "да"]
    );
}

#[test]
fn objects_are_checked() {
    assert_eq!(
        errors(
            "М = Новый Массив;
             Ч = 1;
             Х = Новый Структура;
             М.Добавить();
             К = М.Добавить(1);
             М.Нет();
             Ч.Добавить(1);
             К = Ч[0];
             К = М[\"0\"];
             Для Сч = \"а\" По 2 Цикл КонецЦикла;"
        ),
        [
            "Type is not defined (Структура) at 3:24",
            "Not enough actual parameters (Добавить) at 4:16",
            "Procedure cannot be used as a function (Добавить) at 5:20",
            "Method is not defined for Массив (Нет) at 6:16",
            "Type Число has no methods (Добавить) at 7:16",
            "Indexing is not supported for Число at 8:18",
            "Index must be Число, not Строка at 9:20",
            "'Для' bounds must be Число, not Строка at 10:23",
        ]
    );
    assert_eq!(
        lines(
            "М = Новый Массив;
             М.Добавить(1);
             К = М.Количество() + М.ВГраница();
             Сообщить(К);"
        ),
        ["1"]
    );
}

#[test]
fn for_counters_are_numbers() {
    assert_eq!(
        errors("Сч = \"\";\nДля Сч = 1 По 2 Цикл КонецЦикла;"),
        ["Type mismatch: variable Сч is Строка, cannot assign Число at 2:5"]
    );
    // A bound of an unknown type is checked at run time.
    assert_eq!(
        exception(
            "Функция Тот(З)\n  Возврат З;\nКонецФункции\nДля Сч = 1 По Тот(\"а\") Цикл КонецЦикла;"
        ),
        "test.bsl:4: Несоответствие типов: ожидается Число, получено Строка"
    );
}

#[test]
fn programs_run_the_same_in_strict_mode() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/matrix_mult.cf");
    let source = std::fs::read_to_string(path)
        .unwrap()
        .replace("Размер = 400;", "Размер = 6;");
    assert_eq!(
        lines(&source),
        crate::run(&source).unwrap().lines().collect::<Vec<_>>()
    );
}

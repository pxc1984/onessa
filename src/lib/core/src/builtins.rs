//! Global context functions known to both the compiler and the virtual machine.
//!
//! The compiler emits `CALL_NATIVE` with the canonical (Russian) name; the VM resolves the name
//! when an object file is loaded.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Builtin {
    /// Canonical name written into the object file.
    pub name: &'static str,
    /// English alias accepted in source code.
    pub alias: &'static str,
    pub min_args: u8,
    pub max_args: u8,
    /// `false` for procedures, which cannot be used inside expressions.
    pub returns_value: bool,
}

const fn function(name: &'static str, alias: &'static str, min_args: u8, max_args: u8) -> Builtin {
    Builtin {
        name,
        alias,
        min_args,
        max_args,
        returns_value: true,
    }
}

const fn procedure(name: &'static str, alias: &'static str, min_args: u8, max_args: u8) -> Builtin {
    Builtin {
        name,
        alias,
        min_args,
        max_args,
        returns_value: false,
    }
}

pub const BUILTINS: &[Builtin] = &[
    procedure("Сообщить", "Message", 1, 2),
    function("Строка", "String", 1, 1),
    function("Число", "Number", 1, 1),
    function("Булево", "Boolean", 1, 1),
    function("Формат", "Format", 1, 2),
    function("СтрДлина", "StrLen", 1, 1),
    function("ВРег", "Upper", 1, 1),
    function("НРег", "Lower", 1, 1),
    function("Лев", "Left", 2, 2),
    function("Прав", "Right", 2, 2),
    function("Сред", "Mid", 2, 3),
    function("СокрЛ", "TrimL", 1, 1),
    function("СокрП", "TrimR", 1, 1),
    function("СокрЛП", "TrimAll", 1, 1),
    function("СтрНайти", "StrFind", 2, 2),
    function("СтрЗаменить", "StrReplace", 3, 3),
    function("Окр", "Round", 1, 2),
    function("Цел", "Int", 1, 1),
    function("Макс", "Max", 1, u8::MAX),
    function("Мин", "Min", 1, u8::MAX),
    function("Pow", "Pow", 2, 2),
    function("Sqrt", "Sqrt", 1, 1),
    function("ЗначениеЗаполнено", "ValueIsFilled", 1, 1),
    function("ТекущаяДата", "CurrentDate", 0, 0),
    function("ОписаниеОшибки", "ErrorDescription", 0, 0),
];

/// What a method of a built-in type returns, for the compiler's type checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Returns {
    /// A procedure: `Неопределено`, which cannot be used in an expression.
    Nothing,
    Number,
    /// A value of any type.
    Any,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrayMethod {
    Add,
    Insert,
    Count,
    UBound,
    Get,
    Set,
    Find,
    Delete,
    Clear,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Method {
    pub id: ArrayMethod,
    pub name: &'static str,
    pub alias: &'static str,
    pub min_args: u8,
    pub max_args: u8,
    pub returns: Returns,
}

const fn method(
    id: ArrayMethod,
    name: &'static str,
    alias: &'static str,
    args: u8,
    returns: Returns,
) -> Method {
    Method {
        id,
        name,
        alias,
        min_args: args,
        max_args: args,
        returns,
    }
}

/// Methods of `Массив`.
pub const ARRAY_METHODS: &[Method] = &[
    method(ArrayMethod::Add, "Добавить", "Add", 1, Returns::Nothing),
    method(ArrayMethod::Insert, "Вставить", "Insert", 2, Returns::Nothing),
    method(ArrayMethod::Count, "Количество", "Count", 0, Returns::Number),
    method(ArrayMethod::UBound, "ВГраница", "UBound", 0, Returns::Number),
    method(ArrayMethod::Get, "Получить", "Get", 1, Returns::Any),
    method(ArrayMethod::Set, "Установить", "Set", 2, Returns::Nothing),
    method(ArrayMethod::Find, "Найти", "Find", 1, Returns::Any),
    method(ArrayMethod::Delete, "Удалить", "Delete", 1, Returns::Nothing),
    method(ArrayMethod::Clear, "Очистить", "Clear", 0, Returns::Nothing),
];

/// Finds a method of `Массив` by its Russian or English name, ignoring case.
pub fn lookup_array_method(name: &str) -> Option<&'static Method> {
    let name = name.to_lowercase();
    ARRAY_METHODS
        .iter()
        .find(|m| m.name.to_lowercase() == name || m.alias.to_lowercase() == name)
}

/// Built-in types that `Новый` can create.
pub fn lookup_type(name: &str) -> Option<crate::bytecode::ValueType> {
    match name.to_lowercase().as_str() {
        "массив" | "array" => Some(crate::bytecode::ValueType::Array),
        _ => None,
    }
}

/// Finds a builtin by its Russian or English name, ignoring case.
pub fn lookup(name: &str) -> Option<&'static Builtin> {
    let name = name.to_lowercase();
    BUILTINS
        .iter()
        .find(|b| b.name.to_lowercase() == name || b.alias.to_lowercase() == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_is_case_insensitive_and_bilingual() {
        assert_eq!(lookup("сообщить").unwrap().name, "Сообщить");
        assert_eq!(lookup("MESSAGE").unwrap().name, "Сообщить");
        assert_eq!(lookup("strlen").unwrap().name, "СтрДлина");
        assert!(lookup("НеСуществует").is_none());
    }
}

//! Built-in object types: constructors (`Новый`), indexing and methods.

use crate::value::Value;
use rust_decimal::Decimal;
use std::cell::RefCell;

const NOT_INDEXABLE: &str = "Получение элемента по индексу для значения не поддерживается";
const NOT_INDEX_ASSIGNABLE: &str =
    "Установка значения элемента по индексу для значения не поддерживается";
const INDEX_OUT_OF_RANGE: &str = "Значение индекса выходит за пределы диапазона";
const INVALID_ARGUMENT: &str = "Неверное значение аргумента";

/// `Новый <type_name>(args)`. `type_name` must already be lowercased.
pub fn construct(type_name: &str, display_name: &str, args: &[Value]) -> Result<Value, String> {
    match type_name {
        "массив" | "array" => new_array(args),
        _ => Err(format!("Тип не определен ({display_name})")),
    }
}

/// `Новый Массив(n, m, ...)` creates nested arrays of `Неопределено` with the given sizes.
fn new_array(dimensions: &[Value]) -> Result<Value, String> {
    let Some((first, rest)) = dimensions.split_first() else {
        return Ok(Value::array(Vec::new()));
    };
    if matches!(first, Value::Undefined) && rest.is_empty() {
        return Ok(Value::array(Vec::new()));
    }
    let size = to_size(first)?;
    let items = if rest.is_empty() {
        vec![Value::Undefined; size]
    } else {
        (0..size)
            .map(|_| new_array(rest))
            .collect::<Result<_, _>>()?
    };
    Ok(Value::array(items))
}

/// `collection[index]`
#[inline(always)]
pub fn index_get(collection: &Value, index: &Value) -> Result<Value, String> {
    if let (Value::Array(items), Value::Int(i)) = (collection, index) {
        // A negative index wraps around to a huge one and takes the slow path.
        if let Some(item) = items.borrow().get(*i as usize) {
            return Ok(item.clone());
        }
    }
    index_get_slow(collection, index)
}

/// `collection[i][k]` without cloning `collection[i]` when it is an array.
#[inline(always)]
pub fn index_get2(collection: &Value, i: &Value, k: &Value) -> Result<Value, String> {
    if let (Value::Array(outer), Value::Int(i), Value::Int(k)) = (collection, i, k) {
        if let Some(Value::Array(inner)) = outer.borrow().get(*i as usize) {
            if let Some(item) = inner.borrow().get(*k as usize) {
                return Ok(item.clone());
            }
        }
    }
    index_get(&index_get(collection, i)?, k)
}

#[inline(never)]
fn index_get_slow(collection: &Value, index: &Value) -> Result<Value, String> {
    match collection {
        Value::Array(items) => index_get_items(&items.borrow(), index),
        _ => Err(NOT_INDEXABLE.to_string()),
    }
}

/// `collection[index] = value`
pub fn index_set(collection: &Value, index: &Value, value: Value) -> Result<(), String> {
    match collection {
        Value::Array(items) => {
            let mut items = items.borrow_mut();
            let i = to_index(index, items.len())?;
            items[i] = value;
            Ok(())
        }
        _ => Err(NOT_INDEX_ASSIGNABLE.to_string()),
    }
}

/// `object.method(args)`. `name` must already be lowercased; procedures return `Неопределено`.
pub fn call_method(
    object: &Value,
    name: &str,
    display_name: &str,
    args: &[Value],
) -> Result<Value, String> {
    let not_found = || format!("Метод объекта не обнаружен ({display_name})");
    match object {
        Value::Array(items) => array_method(items, name, args).unwrap_or_else(|| Err(not_found())),
        _ => Err(not_found()),
    }
}

/// `None` if the array has no such method.
fn array_method(
    items: &RefCell<Vec<Value>>,
    name: &str,
    args: &[Value],
) -> Option<Result<Value, String>> {
    let arg = |i: usize| args.get(i).cloned().unwrap_or(Value::Undefined);
    let count = |n: usize| Value::number(Decimal::from(n as u64));
    Some(match name {
        "добавить" | "add" => {
            items.borrow_mut().push(arg(0));
            Ok(Value::Undefined)
        }
        "вставить" | "insert" => to_size(&arg(0)).and_then(|i| {
            let mut items = items.borrow_mut();
            if i > items.len() {
                return Err(INDEX_OUT_OF_RANGE.to_string());
            }
            items.insert(i, arg(1));
            Ok(Value::Undefined)
        }),
        "количество" | "count" => Ok(count(items.borrow().len())),
        "вграница" | "ubound" => Ok(Value::number(Decimal::from(
            items.borrow().len() as i64 - 1,
        ))),
        "получить" | "get" => index_get_items(&items.borrow(), &arg(0)),
        "установить" | "set" => {
            let mut items = items.borrow_mut();
            to_index(&arg(0), items.len()).map(|i| {
                items[i] = arg(1);
                Value::Undefined
            })
        }
        "найти" | "find" => {
            let needle = arg(0);
            Ok(items
                .borrow()
                .iter()
                .position(|item| *item == needle)
                .map_or(Value::Undefined, count))
        }
        "удалить" | "delete" => {
            let mut items = items.borrow_mut();
            to_index(&arg(0), items.len()).map(|i| {
                items.remove(i);
                Value::Undefined
            })
        }
        "очистить" | "clear" => {
            items.borrow_mut().clear();
            Ok(Value::Undefined)
        }
        _ => return None,
    })
}

fn index_get_items(items: &[Value], index: &Value) -> Result<Value, String> {
    let i = to_index(index, items.len())?;
    Ok(items[i].clone())
}

/// A non-negative integer argument such as an array size.
fn to_size(value: &Value) -> Result<usize, String> {
    let n = value.to_number()?;
    if n.is_sign_negative() || n.fract() != Decimal::ZERO {
        return Err(INVALID_ARGUMENT.to_string());
    }
    n.try_into().map_err(|_| INVALID_ARGUMENT.to_string())
}

/// An index of an existing element of a collection of `len` elements.
fn to_index(value: &Value, len: usize) -> Result<usize, String> {
    let index = match value {
        Value::Int(i) => usize::try_from(*i).ok(),
        _ => usize::try_from(value.to_number()?.trunc()).ok(),
    };
    index
        .filter(|&i| i < len)
        .ok_or_else(|| INDEX_OUT_OF_RANGE.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn n(x: i64) -> Value {
        Value::Number(Decimal::from(x))
    }

    #[test]
    fn arrays() {
        let array = construct("массив", "Массив", &[n(2)]).unwrap();
        assert_eq!(index_get(&array, &n(1)), Ok(Value::Undefined));
        call_method(&array, "добавить", "Добавить", &[n(7)]).unwrap();
        assert_eq!(
            call_method(&array, "количество", "Количество", &[]),
            Ok(n(3))
        );
        index_set(&array, &n(0), n(5)).unwrap();
        assert_eq!(index_get(&array, &n(0)), Ok(n(5)));
        assert_eq!(call_method(&array, "найти", "Найти", &[n(7)]), Ok(n(2)));
        assert_eq!(
            call_method(&array, "найти", "Найти", &[n(8)]),
            Ok(Value::Undefined)
        );
        assert!(index_get(&array, &n(3)).is_err());
        assert!(index_get(&array, &n(-1)).is_err());
        assert!(call_method(&array, "нет", "Нет", &[]).is_err());
        assert!(construct("нет", "Нет", &[]).is_err());

        let matrix = construct("array", "Array", &[n(2), n(3)]).unwrap();
        let row = index_get(&matrix, &n(1)).unwrap();
        assert_eq!(call_method(&row, "ubound", "UBound", &[]), Ok(n(2)));
    }
}

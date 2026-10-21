use chrono::{DateTime, Datelike, NaiveDateTime, Timelike};
use onessa_core::bytecode::ValueType;
use rust_decimal::Decimal;
use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::rc::Rc;

/// Runtime value of the language. Strings are immutable and shared.
#[derive(Debug, Clone)]
pub enum Value {
    Undefined,
    Null,
    Boolean(bool),
    /// Число that is an integer fitting in `i64`: the fast representation of the common case.
    /// `Int` and `Number` are the same language type and compare by value.
    Int(i64),
    /// Any other Число.
    Number(Decimal),
    String(Rc<str>),
    Date(NaiveDateTime),
    /// Массив: a 0-based ordered collection shared by reference, like every 1C object.
    Array(Rc<RefCell<Vec<Value>>>),
    /// Structure (Структура) - key-value pairs with string keys
    Structure(Rc<RefCell<HashMap<String, Value>>>),
    /// Object - instance with properties and methods
    Object(Rc<RefCell<Object>>),
}

/// Objects live behind a pointer so that `Value` stays small (24 bytes): every stack and local
/// slot is a `Value`.
#[derive(Debug)]
pub struct Object {
    pub type_name: Rc<str>,
    pub properties: HashMap<String, Value>,
    /// Method name -> function index.
    pub methods: HashMap<String, u32>,
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Undefined, Value::Undefined) => true,
            (Value::Null, Value::Null) => true,
            (Value::Boolean(a), Value::Boolean(b)) => a == b,
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Number(a), Value::Number(b)) => a == b,
            (Value::Int(a), Value::Number(b)) | (Value::Number(b), Value::Int(a)) => {
                Decimal::from(*a) == *b
            }
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Date(a), Value::Date(b)) => a == b,
            (Value::Array(a), Value::Array(b)) => Rc::ptr_eq(a, b),
            (Value::Structure(a), Value::Structure(b)) => Rc::ptr_eq(a, b),
            (Value::Object(a), Value::Object(b)) => Rc::ptr_eq(a, b),
            _ => false,
        }
    }
}

pub const NOT_A_NUMBER: &str = "Преобразование значения к типу Число не может быть выполнено";
pub const NOT_A_BOOLEAN: &str = "Преобразование значения к типу Булево не может быть выполнено";

impl Value {
    pub fn string(s: impl Into<Rc<str>>) -> Self {
        Value::String(s.into())
    }

    /// A number in its canonical representation: `Int` whenever the value is an integer that
    /// fits.
    pub fn number(n: Decimal) -> Self {
        if n.scale() == 0 || n.fract().is_zero() {
            if let Ok(i) = i64::try_from(n) {
                return Value::Int(i);
            }
        }
        Value::Number(n)
    }

    /// Like [`Value::number`], but only integers written without a fractional part become
    /// `Int`: `2.0` stays a `Number`. Cheap enough for the results of arithmetic.
    #[inline]
    pub fn number_result(n: Decimal) -> Self {
        if n.scale() == 0 {
            if let Ok(i) = i64::try_from(n) {
                return Value::Int(i);
            }
        }
        Value::Number(n)
    }

    pub fn array(items: Vec<Value>) -> Self {
        Value::Array(Rc::new(RefCell::new(items)))
    }

    pub fn date_from_timestamp(secs: i64) -> Option<Self> {
        DateTime::from_timestamp(secs, 0).map(|d| Value::Date(d.naive_utc()))
    }

    /// The empty date `'00010101000000'`.
    pub fn empty_date() -> NaiveDateTime {
        chrono::NaiveDate::from_ymd_opt(1, 1, 1)
            .and_then(|d| d.and_hms_opt(0, 0, 0))
            .expect("valid date")
    }

    pub fn type_name(&self) -> Rc<str> {
        Rc::from(match self {
            Value::Undefined => "Неопределено",
            Value::Null => "Null",
            Value::Boolean(_) => "Булево",
            Value::Int(_) | Value::Number(_) => "Число",
            Value::String(_) => "Строка",
            Value::Date(_) => "Дата",
            Value::Array(_) => "Массив",
            Value::Structure(_) => "Структура",
            Value::Object(object) => return object.borrow().type_name.clone(),
        })
    }

    /// The type checked by `CHECK_TYPE`; `None` for types the compiler does not know.
    pub fn value_type(&self) -> Option<ValueType> {
        Some(match self {
            Value::Undefined => ValueType::Undefined,
            Value::Null => ValueType::Null,
            Value::Boolean(_) => ValueType::Boolean,
            Value::Int(_) | Value::Number(_) => ValueType::Number,
            Value::String(_) => ValueType::String,
            Value::Date(_) => ValueType::Date,
            Value::Array(_) => ValueType::Array,
            Value::Structure(_) | Value::Object(_) => return None,
        })
    }

    /// Conversion used by conditions, `Не` and `Булево()`.
    pub fn to_boolean(&self) -> Result<bool, String> {
        match self {
            Value::Boolean(b) => Ok(*b),
            Value::Int(n) => Ok(*n != 0),
            Value::Number(n) => Ok(*n != Decimal::ZERO),
            Value::String(s) => match s.trim().to_lowercase().as_str() {
                "истина" | "да" | "true" => Ok(true),
                "ложь" | "нет" | "false" => Ok(false),
                _ => Err(NOT_A_BOOLEAN.to_string()),
            },
            _ => Err(NOT_A_BOOLEAN.to_string()),
        }
    }

    /// Conversion used by arithmetic and `Число()`: numbers and numeric strings.
    pub fn to_number(&self) -> Result<Decimal, String> {
        match self {
            Value::Int(n) => Ok(Decimal::from(*n)),
            Value::Number(n) => Ok(*n),
            Value::String(s) => {
                let normalized: String = s
                    .trim()
                    .chars()
                    .filter(|c| !matches!(c, ' ' | '\u{00A0}'))
                    .map(|c| if c == ',' { '.' } else { c })
                    .collect();
                normalized
                    .parse::<Decimal>()
                    .ok()
                    .ok_or_else(|| NOT_A_NUMBER.to_string())
            }
            _ => Err(NOT_A_NUMBER.to_string()),
        }
    }

    /// Ordering for `<`, `<=`, `>`, `>=`: only between values of the same primitive types.
    pub fn compare(&self, other: &Value) -> Result<Ordering, String> {
        let ordering = match (self, other) {
            (Value::Int(a), Value::Int(b)) => Some(a.cmp(b)),
            (Value::Int(_) | Value::Number(_), Value::Int(_) | Value::Number(_)) => {
                Some(self.to_number()?.cmp(&other.to_number()?))
            }
            (Value::String(a), Value::String(b)) => Some(a.cmp(b)),
            (Value::Date(a), Value::Date(b)) => Some(a.cmp(b)),
            (Value::Boolean(a), Value::Boolean(b)) => Some(a.cmp(b)),
            _ => None,
        };
        ordering.ok_or_else(|| {
            "Сравнение на больше/меньше допускается только для значений совпадающих примитивных типов (Булево, Число, Строка, Дата)".to_string()
        })
    }

    /// `ЗначениеЗаполнено()`
    pub fn is_filled(&self) -> bool {
        match self {
            Value::Undefined | Value::Null => false,
            Value::Boolean(_) => true,
            Value::Int(n) => *n != 0,
            Value::Number(n) => *n != Decimal::ZERO,
            Value::String(s) => !s.trim().is_empty(),
            Value::Date(d) => *d != Self::empty_date(),
            Value::Array(arr) => !arr.borrow().is_empty(),
            Value::Structure(map) => !map.borrow().is_empty(),
            Value::Object(object) => !object.borrow().properties.is_empty(),
        }
    }
}

impl Display for Value {
    /// Formats like `Строка()` with the default (Russian) locale settings.
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Undefined | Value::Null => Ok(()),
            Value::Boolean(true) => write!(f, "Да"),
            Value::Boolean(false) => write!(f, "Нет"),
            Value::Int(n) => write!(f, "{}", format_decimal(Decimal::from(*n))),
            Value::Number(n) => write!(f, "{}", format_decimal(*n)),
            Value::String(s) => write!(f, "{s}"),
            Value::Date(d) => write!(
                f,
                "{:02}.{:02}.{:04} {}:{:02}:{:02}",
                d.day(),
                d.month(),
                d.year(),
                d.hour(),
                d.minute(),
                d.second()
            ),
            Value::Array(_) => write!(f, "Массив"),
            Value::Structure(_) => write!(f, "Структура"),
            Value::Object(object) => write!(f, "{}", object.borrow().type_name),
        }
    }
}

/// Format Decimal with Russian locale: `1 234 567,5` (groups separated by non-breaking space).
fn format_decimal(n: Decimal) -> String {
    crate::format::format_number(n, &crate::format::NumberFormat::default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    #[test]
    fn number_formatting() {
        assert_eq!(format_decimal(Decimal::ZERO), "0");
        assert_eq!(format_decimal(Decimal::new(5, 0)), "5");
        assert_eq!(format_decimal(Decimal::new(-125, 1)), "-12,5");
        assert_eq!(format_decimal(Decimal::new(1234, 0)), "1\u{00A0}234");
        assert_eq!(
            format_decimal(Decimal::new(123456725, 2)),
            "1\u{00A0}234\u{00A0}567,25"
        );
        // Test exact decimal arithmetic: 0.1 + 0.2 = 0.3
        let a = Decimal::new(1, 1); // 0.1
        let b = Decimal::new(2, 1); // 0.2
        assert_eq!(format_decimal(a + b), "0,3");
    }

    #[test]
    fn display() {
        assert_eq!(Value::Boolean(true).to_string(), "Да");
        assert_eq!(Value::Undefined.to_string(), "");
        let date = Value::date_from_timestamp(1_704_067_200).unwrap();
        assert_eq!(date.to_string(), "01.01.2024 0:00:00");
        let arr = Value::array(vec![Value::Number(Decimal::ONE)]);
        assert_eq!(arr.to_string(), "Массив");
        assert_eq!(arr, arr.clone());
        assert_ne!(arr, Value::array(vec![Value::Number(Decimal::ONE)]));
        let mut map = HashMap::new();
        map.insert("key".to_string(), Value::Number(Decimal::ONE));
        let structure = Value::Structure(Rc::new(RefCell::new(map)));
        assert_eq!(structure.to_string(), "Структура");
        assert_eq!(std::mem::size_of::<Value>(), 24);
    }

    #[test]
    fn conversions() {
        assert_eq!(
            Value::string(" 1 234,5 ").to_number(),
            Ok(Decimal::new(12345, 1))
        );
        assert!(Value::string("abc").to_number().is_err());
        assert_eq!(Value::Number(Decimal::new(2, 0)).to_boolean(), Ok(true));
        assert!(Value::Undefined.to_boolean().is_err());
        assert!(Value::Number(Decimal::ONE)
            .compare(&Value::string("1"))
            .is_err());
        assert_eq!(Value::number(Decimal::new(20, 1)), Value::Int(2));
        assert!(matches!(Value::number(Decimal::new(20, 1)), Value::Int(2)));
        assert!(matches!(
            Value::number(Decimal::new(25, 1)),
            Value::Number(_)
        ));
        assert_eq!(Value::Int(3), Value::Number(Decimal::from(3)));
        assert_eq!(
            Value::Int(3).compare(&Value::Number(Decimal::new(25, 1))),
            Ok(Ordering::Greater)
        );
    }

    #[test]
    fn decimal_arithmetic() {
        // Verify exact decimal arithmetic
        let a = Decimal::new(1, 1); // 0.1
        let b = Decimal::new(2, 1); // 0.2
        let c = Decimal::new(3, 1); // 0.3
        assert_eq!(a + b, c);
    }
}

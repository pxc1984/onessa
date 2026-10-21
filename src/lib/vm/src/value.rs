use chrono::{DateTime, Datelike, NaiveDateTime, Timelike};
use std::cmp::Ordering;
use std::fmt::{Display, Formatter};
use std::rc::Rc;

/// Runtime value of the language. Strings are immutable and shared.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Undefined,
    Null,
    Boolean(bool),
    Number(f64),
    String(Rc<str>),
    Date(NaiveDateTime),
}

/// Significant digits kept when printing numbers, hiding binary floating point noise
/// (`0.1 + 0.2` prints as `0,3`).
const PRINT_PRECISION: usize = 15;

pub const NOT_A_NUMBER: &str = "Преобразование значения к типу Число не может быть выполнено";
pub const NOT_A_BOOLEAN: &str = "Преобразование значения к типу Булево не может быть выполнено";

impl Value {
    pub fn string(s: impl Into<Rc<str>>) -> Self {
        Value::String(s.into())
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

    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Undefined => "Неопределено",
            Value::Null => "Null",
            Value::Boolean(_) => "Булево",
            Value::Number(_) => "Число",
            Value::String(_) => "Строка",
            Value::Date(_) => "Дата",
        }
    }

    /// Conversion used by conditions, `Не` and `Булево()`.
    pub fn to_boolean(&self) -> Result<bool, String> {
        match self {
            Value::Boolean(b) => Ok(*b),
            Value::Number(n) => Ok(*n != 0.0),
            Value::String(s) => match s.trim().to_lowercase().as_str() {
                "истина" | "да" | "true" => Ok(true),
                "ложь" | "нет" | "false" => Ok(false),
                _ => Err(NOT_A_BOOLEAN.to_string()),
            },
            _ => Err(NOT_A_BOOLEAN.to_string()),
        }
    }

    /// Conversion used by arithmetic and `Число()`: numbers and numeric strings.
    pub fn to_number(&self) -> Result<f64, String> {
        match self {
            Value::Number(n) => Ok(*n),
            Value::String(s) => {
                let normalized: String = s
                    .trim()
                    .chars()
                    .filter(|c| !matches!(c, ' ' | '\u{00A0}'))
                    .map(|c| if c == ',' { '.' } else { c })
                    .collect();
                normalized
                    .parse::<f64>()
                    .ok()
                    .filter(|n| n.is_finite())
                    .ok_or_else(|| NOT_A_NUMBER.to_string())
            }
            _ => Err(NOT_A_NUMBER.to_string()),
        }
    }

    /// Ordering for `<`, `<=`, `>`, `>=`: only between values of the same primitive type.
    pub fn compare(&self, other: &Value) -> Result<Ordering, String> {
        let ordering = match (self, other) {
            (Value::Number(a), Value::Number(b)) => a.partial_cmp(b),
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
            Value::Number(n) => *n != 0.0,
            Value::String(s) => !s.trim().is_empty(),
            Value::Date(d) => *d != Self::empty_date(),
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
            Value::Number(n) => write!(f, "{}", format_number(*n)),
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
        }
    }
}

/// `1234567.5` → `1 234 567,5` (groups separated by a non-breaking space).
fn format_number(n: f64) -> String {
    let int_digits = if n.abs() < 1.0 {
        1
    } else {
        n.abs().log10().floor() as usize + 1
    };
    let decimals = PRINT_PRECISION.saturating_sub(int_digits);
    let mut text = format!("{:.*}", decimals, n.abs());
    if text.contains('.') {
        text = text.trim_end_matches('0').trim_end_matches('.').to_string();
    }
    let (int_part, frac_part) = match text.split_once('.') {
        Some((i, f)) => (i.to_string(), Some(f.to_string())),
        None => (text, None),
    };

    let mut out = String::new();
    if n < 0.0 && (int_part != "0" || frac_part.is_some()) {
        out.push('-');
    }
    let len = int_part.len();
    for (i, digit) in int_part.chars().enumerate() {
        if i > 0 && (len - i) % 3 == 0 {
            out.push('\u{00A0}');
        }
        out.push(digit);
    }
    if let Some(frac) = frac_part {
        out.push(',');
        out.push_str(&frac);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn number_formatting() {
        assert_eq!(format_number(0.0), "0");
        assert_eq!(format_number(-0.0), "0");
        assert_eq!(format_number(5.0), "5");
        assert_eq!(format_number(-12.5), "-12,5");
        assert_eq!(format_number(1234.0), "1\u{00A0}234");
        assert_eq!(format_number(1234567.25), "1\u{00A0}234\u{00A0}567,25");
        assert_eq!(format_number(0.1 + 0.2), "0,3");
        assert_eq!(format_number(1.0 / 3.0), "0,33333333333333");
    }

    #[test]
    fn display() {
        assert_eq!(Value::Boolean(true).to_string(), "Да");
        assert_eq!(Value::Undefined.to_string(), "");
        let date = Value::date_from_timestamp(1_704_067_200).unwrap();
        assert_eq!(date.to_string(), "01.01.2024 0:00:00");
    }

    #[test]
    fn conversions() {
        assert_eq!(Value::string(" 1 234,5 ").to_number(), Ok(1234.5));
        assert!(Value::string("abc").to_number().is_err());
        assert_eq!(Value::Number(2.0).to_boolean(), Ok(true));
        assert!(Value::Undefined.to_boolean().is_err());
        assert!(Value::Number(1.0).compare(&Value::string("1")).is_err());
    }
}

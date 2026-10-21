//! Implementations of the global context functions listed in `onessa_core::builtins`.

use crate::value::Value;
use chrono::Local;
use rust_decimal::Decimal;
use std::cmp::Ordering;
use std::io::Write;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Native {
    Message,
    String,
    Number,
    Boolean,
    StrLen,
    Upper,
    Lower,
    Left,
    Right,
    Mid,
    TrimL,
    TrimR,
    TrimAll,
    StrFind,
    StrReplace,
    Round,
    Int,
    Max,
    Min,
    Pow,
    Sqrt,
    ValueIsFilled,
    CurrentDate,
    ErrorDescription,
}

impl Native {
    /// Resolves a canonical builtin name as written by the compiler.
    pub fn resolve(name: &str) -> Option<Self> {
        Some(match name {
            "Сообщить" => Native::Message,
            "Строка" => Native::String,
            "Число" => Native::Number,
            "Булево" => Native::Boolean,
            "СтрДлина" => Native::StrLen,
            "ВРег" => Native::Upper,
            "НРег" => Native::Lower,
            "Лев" => Native::Left,
            "Прав" => Native::Right,
            "Сред" => Native::Mid,
            "СокрЛ" => Native::TrimL,
            "СокрП" => Native::TrimR,
            "СокрЛП" => Native::TrimAll,
            "СтрНайти" => Native::StrFind,
            "СтрЗаменить" => Native::StrReplace,
            "Окр" => Native::Round,
            "Цел" => Native::Int,
            "Макс" => Native::Max,
            "Мин" => Native::Min,
            "Pow" => Native::Pow,
            "Sqrt" => Native::Sqrt,
            "ЗначениеЗаполнено" => Native::ValueIsFilled,
            "ТекущаяДата" => Native::CurrentDate,
            "ОписаниеОшибки" => Native::ErrorDescription,
            _ => return None,
        })
    }
}

/// VM state the builtins may access.
pub struct NativeContext<'a> {
    pub output: &'a mut dyn Write,
    pub error_description: Option<&'a str>,
}

pub fn call(native: Native, ctx: NativeContext<'_>, args: &[Value]) -> Result<Value, String> {
    let arg = |i: usize| args.get(i).cloned().unwrap_or(Value::Undefined);
    let text = |i: usize| arg(i).to_string();
    let number = |i: usize| arg(i).to_number();
    let chars = |i: usize| text(i).chars().collect::<Vec<_>>();
    // Converts a 1C character count/position argument, clamping negatives to zero.
    let count = |i: usize| -> usize {
        number(i)
            .map(|n| n.trunc().try_into().unwrap_or(0))
            .unwrap_or(0)
    };

    let value = match native {
        Native::Message => {
            writeln!(ctx.output, "{}", text(0)).map_err(|e| e.to_string())?;
            Value::Undefined
        }
        Native::String => Value::string(text(0)),
        Native::Number => Value::Number(number(0)?),
        Native::Boolean => Value::Boolean(arg(0).to_boolean()?),
        Native::StrLen => Value::Number(Decimal::from(chars(0).len() as u32)),
        Native::Upper => Value::string(text(0).to_uppercase()),
        Native::Lower => Value::string(text(0).to_lowercase()),
        Native::Left => {
            let s = chars(0);
            Value::string(s[..count(1).min(s.len())].iter().collect::<String>())
        }
        Native::Right => {
            let s = chars(0);
            let n = count(1).min(s.len());
            Value::string(s[s.len() - n..].iter().collect::<String>())
        }
        Native::Mid => {
            let s = chars(0);
            let start = count(1).max(1) - 1;
            let start = start.min(s.len());
            let len = match args.get(2) {
                Some(Value::Undefined) | None => s.len() - start,
                Some(_) => count(2).min(s.len() - start),
            };
            Value::string(s[start..start + len].iter().collect::<String>())
        }
        Native::TrimL => Value::string(text(0).trim_start()),
        Native::TrimR => Value::string(text(0).trim_end()),
        Native::TrimAll => Value::string(text(0).trim()),
        Native::StrFind => {
            let haystack = text(0);
            let needle = text(1);
            let position = haystack
                .find(&needle)
                .map(|byte| haystack[..byte].chars().count() + 1)
                .unwrap_or(0);
            Value::Number(Decimal::from(position as u32))
        }
        Native::StrReplace => {
            let needle = text(1);
            if needle.is_empty() {
                Value::string(text(0))
            } else {
                Value::string(text(0).replace(&needle, &text(2)))
            }
        }
        Native::Round => {
            let digits = match args.get(1) {
                Some(Value::Undefined) | None => 0,
                Some(_) => number(1)?.trunc().try_into().unwrap_or(0),
            };
            // Round using Decimal's built-in round with scale
            let rounded = number(0)?.round_dp(digits as u32);
            Value::Number(rounded)
        }
        Native::Int => Value::Number(number(0)?.trunc()),
        Native::Max | Native::Min => {
            let wanted = if native == Native::Max {
                Ordering::Greater
            } else {
                Ordering::Less
            };
            let mut best = arg(0);
            for candidate in &args[1..] {
                if candidate.compare(&best)? == wanted {
                    best = candidate.clone();
                }
            }
            best
        }
        Native::Pow => {
            let base = number(0)?;
            let exp: u32 = number(1)?.try_into().unwrap_or(0);
            // Decimal doesn't have powi, so we multiply manually
            let mut result = Decimal::ONE;
            for _ in 0..exp {
                result *= base;
            }
            Value::Number(result)
        }
        Native::Sqrt => {
            let n = number(0)?;
            if n.is_sign_negative() {
                return Err("Недопустимое значение параметра (Sqrt)".to_string());
            }
            // Newton's method for square root with Decimal
            if n == Decimal::ZERO {
                return Ok(Value::Number(Decimal::ZERO));
            }
            let mut guess = n / Decimal::TWO;
            for _ in 0..50 {
                let new_guess = (guess + n / guess) / Decimal::TWO;
                if new_guess == guess {
                    break;
                }
                guess = new_guess;
            }
            Value::Number(guess)
        }
        Native::ValueIsFilled => Value::Boolean(arg(0).is_filled()),
        Native::CurrentDate => {
            let now = Local::now().naive_local();
            Value::Date(now.with_nanosecond_zero())
        }
        Native::ErrorDescription => Value::string(ctx.error_description.unwrap_or_default()),
    };
    Ok(value)
}

trait WithNanosecondZero {
    fn with_nanosecond_zero(self) -> Self;
}

impl WithNanosecondZero for chrono::NaiveDateTime {
    fn with_nanosecond_zero(self) -> Self {
        use chrono::Timelike;
        self.with_nanosecond(0).unwrap_or(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use onessa_core::builtins::BUILTINS;

    #[test]
    fn every_builtin_has_an_implementation() {
        for builtin in BUILTINS {
            assert!(Native::resolve(builtin.name).is_some(), "{}", builtin.name);
        }
    }

    fn eval(native: Native, args: &[Value]) -> Result<Value, String> {
        let mut out = Vec::new();
        call(
            native,
            NativeContext {
                output: &mut out,
                error_description: None,
            },
            args,
        )
    }

    #[test]
    fn strings() {
        let s = Value::string("Привет, мир");
        let n = |x: i32| Value::Number(Decimal::from(x));
        assert_eq!(eval(Native::StrLen, std::slice::from_ref(&s)), Ok(n(11)));
        assert_eq!(
            eval(Native::Left, &[s.clone(), n(6)]),
            Ok(Value::string("Привет"))
        );
        assert_eq!(
            eval(Native::Right, &[s.clone(), n(3)]),
            Ok(Value::string("мир"))
        );
        assert_eq!(
            eval(Native::Mid, &[s.clone(), n(9)]),
            Ok(Value::string("мир"))
        );
        assert_eq!(
            eval(Native::Mid, &[s.clone(), n(1), n(3)]),
            Ok(Value::string("При"))
        );
        assert_eq!(
            eval(Native::Mid, &[s.clone(), n(50), n(3)]),
            Ok(Value::string(""))
        );
        assert_eq!(
            eval(Native::StrFind, &[s.clone(), Value::string("мир")]),
            Ok(n(9))
        );
        assert_eq!(
            eval(Native::StrFind, &[s.clone(), Value::string("x")]),
            Ok(n(0))
        );
        assert_eq!(
            eval(
                Native::StrReplace,
                &[s, Value::string("мир"), Value::string("всем")]
            ),
            Ok(Value::string("Привет, всем"))
        );
    }

    #[test]
    fn numbers() {
        let n = |x: i32| Value::Number(Decimal::from(x));
        assert_eq!(eval(Native::Round, &[n(2)]), Ok(n(2)));
        assert_eq!(eval(Native::Round, &[n(-2)]), Ok(n(-2)));
        assert_eq!(eval(Native::Int, &[n(-7)]), Ok(n(-7)));
        assert_eq!(eval(Native::Max, &[n(1), n(5), n(3)]), Ok(n(5)));
        assert_eq!(eval(Native::Min, &[n(1), n(5), n(-3)]), Ok(n(-3)));
        assert!(eval(Native::Max, &[n(1), Value::string("2")]).is_err());
        assert!(eval(Native::Sqrt, &[n(-1)]).is_err());
    }
}

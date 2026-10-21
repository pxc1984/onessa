//! `Формат()`: format strings such as `"ЧДЦ=2; ЧГ=0"` applied to numbers, booleans and dates.

use crate::value::Value;
use chrono::{Datelike, NaiveDateTime, Timelike};
use rust_decimal::{Decimal, RoundingStrategy};

/// Parsed `Имя=Значение; ...` pairs with lowercase names; values may be quoted.
struct Params(Vec<(String, String)>);

impl Params {
    fn parse(format: &str) -> Self {
        let mut params = Vec::new();
        let mut chars = format.chars().peekable();
        loop {
            let name: String = chars.by_ref().take_while(|&c| c != '=').collect();
            let name = name.trim().to_lowercase();
            while chars.peek().is_some_and(|c| c.is_whitespace()) {
                chars.next();
            }
            let value = match chars.peek() {
                Some(&quote @ ('\'' | '"')) => {
                    chars.next();
                    let value: String = chars.by_ref().take_while(|&c| c != quote).collect();
                    chars.by_ref().take_while(|&c| c != ';').for_each(drop);
                    value
                }
                _ => chars
                    .by_ref()
                    .take_while(|&c| c != ';')
                    .collect::<String>()
                    .trim()
                    .to_string(),
            };
            if !name.is_empty() {
                params.push((name, value));
            }
            if chars.peek().is_none() {
                return Self(params);
            }
        }
    }

    /// The value of the parameter with the Russian or English name (the last one wins).
    fn get(&self, russian: &str, english: &str) -> Option<&str> {
        self.0
            .iter()
            .rev()
            .find(|(name, _)| name == russian || name == english)
            .map(|(_, value)| value.as_str())
    }

    fn number(&self, russian: &str, english: &str) -> Option<usize> {
        self.get(russian, english).and_then(leading_number)
    }
}

fn leading_number(text: &str) -> Option<usize> {
    let digits: String = text
        .trim()
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    digits.parse().ok()
}

pub fn format(value: &Value, format: &str) -> String {
    let params = Params::parse(format);
    match value {
        Value::Int(n) => format_number_with(Decimal::from(*n), &params),
        Value::Number(n) => format_number_with(*n, &params),
        Value::Boolean(b) => {
            let (name, default) = if *b {
                ("би", "Да")
            } else {
                ("бл", "Нет")
            };
            let english = if *b { "bt" } else { "bf" };
            params.get(name, english).unwrap_or(default).to_string()
        }
        Value::Date(d) => format_date(*d, &params),
        other => other.to_string(),
    }
}

/// Number formatting options; the defaults are those of `Строка()`.
pub struct NumberFormat<'a> {
    /// `ЧЦ`: total number of digits.
    pub digits: Option<usize>,
    /// `ЧДЦ`: fractional digits, rounded half away from zero and padded with zeros.
    pub fraction_digits: Option<usize>,
    /// `ЧРД`
    pub decimal_separator: &'a str,
    /// `ЧРГ`
    pub group_separator: &'a str,
    /// `ЧГ`: group sizes from the right, the last one repeats; 0 stops grouping.
    pub groups: Vec<usize>,
    /// `ЧЛЗ`: pad the integer part with zeros up to `ЧЦ - ЧДЦ` digits.
    pub leading_zeros: bool,
}

impl Default for NumberFormat<'_> {
    fn default() -> Self {
        Self {
            digits: None,
            fraction_digits: None,
            decimal_separator: ",",
            group_separator: "\u{00A0}",
            groups: vec![3],
            leading_zeros: false,
        }
    }
}

fn format_number_with(n: Decimal, params: &Params) -> String {
    let mut options = NumberFormat {
        digits: params.number("чц", "nd"),
        fraction_digits: params.number("чдц", "nfd"),
        leading_zeros: params.get("члз", "nlz").is_some(),
        ..NumberFormat::default()
    };
    if let Some(separator) = params.get("чрд", "nds") {
        options.decimal_separator = separator;
    }
    if let Some(separator) = params.get("чрг", "ngs") {
        options.group_separator = separator;
    }
    if let Some(groups) = params.get("чг", "ng") {
        options.groups = groups.split(',').filter_map(leading_number).collect();
    }

    let formatted = format_number(n, &options);
    // Zero is shown as `ЧН`, or as nothing at all, unless leading zeros were requested.
    if n.is_zero() && !options.leading_zeros {
        return params.get("чн", "nz").unwrap_or_default().to_string();
    }
    formatted
}

pub fn format_number(n: Decimal, options: &NumberFormat) -> String {
    let mut n = n;
    if let Some(digits) = options.fraction_digits {
        n = n.round_dp_with_strategy(digits as u32, RoundingStrategy::MidpointAwayFromZero);
    }
    let text = match options.fraction_digits {
        Some(digits) => format!("{:.*}", digits, n.abs()),
        None => n.abs().normalize().to_string(),
    };
    let (int_part, frac_part) = text.split_once('.').unwrap_or((&text, ""));

    let mut int_part = int_part.to_string();
    if let Some(total) = options.digits {
        let int_digits = total.saturating_sub(options.fraction_digits.unwrap_or(0));
        if int_part.len() > int_digits {
            // The number does not fit: 1C shows the largest number that does.
            int_part = "9".repeat(int_digits);
        } else if options.leading_zeros {
            int_part = format!("{int_part:0>int_digits$}");
        }
    }

    let mut out = String::new();
    if n.is_sign_negative() && !n.is_zero() {
        out.push('-');
    }
    out.push_str(&group_digits(
        &int_part,
        &options.groups,
        options.group_separator,
    ));
    if !frac_part.is_empty() {
        out.push_str(options.decimal_separator);
        out.push_str(frac_part);
    }
    out
}

fn group_digits(digits: &str, groups: &[usize], separator: &str) -> String {
    let digits: Vec<char> = digits.chars().collect();
    let mut chunks = Vec::new();
    let mut end = digits.len();
    let mut group = 0;
    while end > 0 {
        let size = groups.get(group).or(groups.last()).copied().unwrap_or(0);
        if size == 0 || size >= end {
            chunks.push(&digits[..end]);
            break;
        }
        chunks.push(&digits[end - size..end]);
        end -= size;
        group += 1;
    }
    let chunks: Vec<String> = chunks.iter().rev().map(|c| c.iter().collect()).collect();
    chunks.join(separator)
}

fn format_date(date: NaiveDateTime, params: &Params) -> String {
    if date == Value::empty_date() {
        return params.get("дп", "de").unwrap_or_default().to_string();
    }
    let pattern = match params.get("дф", "df") {
        Some(pattern) => pattern.to_string(),
        None => match params.get("длф", "dlf").map(str::to_lowercase).as_deref() {
            Some("д" | "d") => "дд.ММ.гггг".to_string(),
            Some("в" | "t") => "Ч:мм:сс".to_string(),
            _ => return Value::Date(date).to_string(),
        },
    };
    format_date_pattern(date, &pattern)
}

/// Expands `дд.ММ.гггг ЧЧ:мм:сс` style patterns (Russian or English letters).
fn format_date_pattern(date: NaiveDateTime, pattern: &str) -> String {
    let chars: Vec<char> = pattern.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let run = chars[i..].iter().take_while(|&&x| x == c).count();
        let hour12 = (date.hour() + 11) % 12 + 1;
        let field = match c {
            'г' | 'y' => Some(if run <= 2 {
                format!("{:02}", date.year() % 100)
            } else {
                format!("{:04}", date.year())
            }),
            'М' | 'M' => Some(pad(date.month(), run)),
            'д' | 'd' => Some(pad(date.day(), run)),
            'Ч' | 'H' => Some(pad(date.hour(), run)),
            'ч' | 'h' => Some(pad(hour12, run)),
            'м' | 'm' => Some(pad(date.minute(), run)),
            'с' | 's' => Some(pad(date.second(), run)),
            _ => None,
        };
        match field {
            Some(field) => {
                out.push_str(&field);
                i += run;
            }
            None => {
                out.push(c);
                i += 1;
            }
        }
    }
    out
}

fn pad(value: u32, width: usize) -> String {
    if width >= 2 {
        format!("{value:02}")
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn number(n: Decimal, f: &str) -> String {
        format(&Value::Number(n), f)
    }

    #[test]
    fn numbers() {
        let n = Decimal::new(12345675, 3); // 12345.675
        assert_eq!(number(n, ""), "12\u{a0}345,675");
        assert_eq!(number(n, "ЧДЦ=2"), "12\u{a0}345,68");
        assert_eq!(number(n, "ЧДЦ=2; ЧГ=0"), "12345,68");
        assert_eq!(number(n, "ЧДЦ=0; ЧРГ=' '"), "12 346");
        assert_eq!(number(n, "NFD=1; NDS=.; NG=0"), "12345.7");
        assert_eq!(number(-n, "ЧДЦ=1"), "-12\u{a0}345,7");
        assert_eq!(number(Decimal::from(1234567), "ЧГ=3,0"), "1234\u{a0}567");
        assert_eq!(number(Decimal::from(21333400), "ЧГ=10.2"), "21333400");
        assert_eq!(number(Decimal::from(7), "ЧЦ=3; ЧЛЗ="), "007");
        assert_eq!(number(Decimal::from(1234), "ЧЦ=3"), "999");
        assert_eq!(number(Decimal::ZERO, "ЧДЦ=2"), "");
        assert_eq!(number(Decimal::ZERO, "ЧН=ноль"), "ноль");
    }

    #[test]
    fn booleans_and_dates() {
        assert_eq!(format(&Value::Boolean(true), ""), "Да");
        assert_eq!(format(&Value::Boolean(false), "БЛ=Выкл; БИ=Вкл"), "Выкл");
        let date = Value::date_from_timestamp(1_704_103_205).unwrap(); // 2024-01-01 10:00:05
        assert_eq!(format(&date, "ДФ=дд.ММ.гггг"), "01.01.2024");
        assert_eq!(
            format(&date, "ДФ='yyyy-MM-dd HH:mm:ss'"),
            "2024-01-01 10:00:05"
        );
        assert_eq!(format(&date, "ДЛФ=Д"), "01.01.2024");
        assert_eq!(
            format(&Value::Date(Value::empty_date()), "ДП=пусто"),
            "пусто"
        );
    }
}

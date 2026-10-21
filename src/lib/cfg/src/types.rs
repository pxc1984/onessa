//! Static types of strict mode (`--strict`).
//!
//! A variable gets the type of the value first assigned to it and keeps it: assigning a
//! value of another type is a compile error, and a value whose type is only known at run time
//! is checked with `CHECK_TYPE` before it is stored. Variables first assigned a value of an
//! unknown type (or `Неопределено`), and parameters, are `Произвольный`: they accept anything.
//!
//! Without `--strict` every variable is `Произвольный`, so the same rules only reject
//! operations on literals, and they are not reported.

use onessa_ast::SyntaxKind;
use onessa_core::bytecode::ValueType;
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type {
    /// The expression produces no value: a call of a function whose result type is not
    /// inferred yet. Operations on it are not checked.
    Never,
    Known(ValueType),
    /// Any type, checked at run time.
    Any,
}

impl Type {
    pub const UNDEFINED: Type = Type::Known(ValueType::Undefined);
    pub const BOOLEAN: Type = Type::Known(ValueType::Boolean);
    pub const NUMBER: Type = Type::Known(ValueType::Number);
    pub const STRING: Type = Type::Known(ValueType::String);
    pub const DATE: Type = Type::Known(ValueType::Date);
    pub const ARRAY: Type = Type::Known(ValueType::Array);

    /// The type of a value that may come from either expression.
    pub fn join(self, other: Type) -> Type {
        match (self, other) {
            (Type::Never, t) | (t, Type::Never) => t,
            (a, b) if a == b => a,
            _ => Type::Any,
        }
    }

    /// The type a new variable gets from the first value assigned to it, `None` while it
    /// is still unknown.
    pub fn of_new_variable(self) -> Option<Type> {
        match self {
            Type::Never => None,
            Type::Known(ValueType::Undefined | ValueType::Null) | Type::Any => Some(Type::Any),
            known => Some(known),
        }
    }

    fn is(self, value_type: ValueType) -> bool {
        self == Type::Known(value_type)
    }

    /// `true` unless the value certainly has another type than `value_type`.
    fn may_be(self, value_type: ValueType) -> bool {
        matches!(self, Type::Any | Type::Never) || self.is(value_type)
    }
}

impl Display for Type {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Never => f.write_str("<нет значения>"),
            Type::Known(t) => write!(f, "{t}"),
            Type::Any => f.write_str("Произвольный"),
        }
    }
}

fn operator_text(operator: SyntaxKind) -> &'static str {
    match operator {
        SyntaxKind::Plus => "+",
        SyntaxKind::Minus => "-",
        SyntaxKind::Star => "*",
        SyntaxKind::Slash => "/",
        SyntaxKind::Percent => "%",
        SyntaxKind::Lt => "<",
        SyntaxKind::Le => "<=",
        SyntaxKind::Gt => ">",
        SyntaxKind::Ge => ">=",
        SyntaxKind::Eq => "=",
        SyntaxKind::Ne => "<>",
        SyntaxKind::And => "И",
        SyntaxKind::Or => "ИЛИ",
        SyntaxKind::Not => "НЕ",
        _ => "?",
    }
}

fn not_applicable(operator: SyntaxKind, lhs: Type, rhs: Type) -> String {
    format!(
        "Operator '{}' cannot be applied to {lhs} and {rhs}",
        operator_text(operator)
    )
}

/// Type of `lhs <operator> rhs` for every binary operator except `И` and `ИЛИ`.
pub fn binary(operator: SyntaxKind, lhs: Type, rhs: Type) -> Result<Type, String> {
    use ValueType::{Date, Number, String as Str};
    let never = lhs == Type::Never || rhs == Type::Never;
    let result = match operator {
        SyntaxKind::Plus => match (lhs, rhs) {
            _ if lhs.is(Str) => Some(Type::STRING),
            _ if lhs.is(Number) && rhs.may_be(Number) => Some(Type::NUMBER),
            _ if lhs.is(Date) && rhs.may_be(Number) => Some(Type::DATE),
            (Type::Any | Type::Never, _) => Some(Type::Any),
            _ => None,
        },
        SyntaxKind::Minus => match (lhs, rhs) {
            _ if lhs.is(Number) && rhs.may_be(Number) => Some(Type::NUMBER),
            _ if lhs.is(Date) && rhs.is(Date) => Some(Type::NUMBER),
            _ if lhs.is(Date) && rhs.is(Number) => Some(Type::DATE),
            _ if lhs.is(Date) && rhs.may_be(Number) => Some(Type::Any),
            // Only `Дата - Дата` gives a number when the right operand is a date.
            (Type::Any | Type::Never, _) if rhs.is(Date) => Some(Type::NUMBER),
            (Type::Any | Type::Never, _) if rhs.may_be(Number) => Some(Type::Any),
            _ => None,
        },
        SyntaxKind::Star | SyntaxKind::Slash | SyntaxKind::Percent => {
            (lhs.may_be(Number) && rhs.may_be(Number)).then_some(Type::NUMBER)
        }
        SyntaxKind::Lt | SyntaxKind::Le | SyntaxKind::Gt | SyntaxKind::Ge => {
            let comparable = |t: Type| match t {
                Type::Known(t) => {
                    matches!(t, ValueType::Boolean | ValueType::Number | Str | Date)
                }
                _ => true,
            };
            let same = match (lhs, rhs) {
                (Type::Known(a), Type::Known(b)) => a == b,
                _ => true,
            };
            (comparable(lhs) && comparable(rhs) && same).then_some(Type::BOOLEAN)
        }
        SyntaxKind::Eq | SyntaxKind::Ne => Some(Type::BOOLEAN),
        _ => None,
    };
    match result {
        Some(_) if never => Ok(Type::Never),
        Some(result) => Ok(result),
        None if never => Ok(Type::Never),
        None => Err(not_applicable(operator, lhs, rhs)),
    }
}

/// Type of `-operand`, `+operand` and `НЕ operand`.
pub fn unary(operator: SyntaxKind, operand: Type) -> Result<Type, String> {
    let (expected, result) = match operator {
        SyntaxKind::Not => (ValueType::Boolean, Type::BOOLEAN),
        SyntaxKind::Minus => (ValueType::Number, Type::NUMBER),
        _ => (ValueType::Number, operand),
    };
    match operand {
        Type::Never => Ok(Type::Never),
        _ if operand.may_be(expected) => Ok(result),
        _ => Err(format!(
            "Operator '{}' cannot be applied to {operand}",
            operator_text(operator)
        )),
    }
}

/// Checks the operand of `И` or `ИЛИ`, or a condition.
pub fn condition(operand: Type) -> Result<(), String> {
    if operand.may_be(ValueType::Boolean) {
        Ok(())
    } else {
        Err(format!("Condition must be Булево, not {operand}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binary_operators() {
        use SyntaxKind::*;
        let (n, s, d, b, a) = (
            Type::NUMBER,
            Type::STRING,
            Type::DATE,
            Type::BOOLEAN,
            Type::Any,
        );
        assert_eq!(binary(Plus, n, n), Ok(n));
        assert_eq!(binary(Plus, s, n), Ok(s));
        assert_eq!(binary(Plus, n, a), Ok(n));
        assert_eq!(binary(Plus, a, n), Ok(a));
        assert_eq!(binary(Plus, d, n), Ok(d));
        assert!(binary(Plus, n, s).is_err());
        assert!(binary(Plus, b, b).is_err());
        assert_eq!(binary(Minus, d, d), Ok(n));
        assert_eq!(binary(Minus, a, d), Ok(n));
        assert_eq!(binary(Minus, d, a), Ok(a));
        assert!(binary(Minus, s, n).is_err());
        assert_eq!(binary(Star, a, n), Ok(n));
        assert!(binary(Star, s, n).is_err());
        assert_eq!(binary(Lt, n, n), Ok(b));
        assert_eq!(binary(Lt, a, s), Ok(b));
        assert!(binary(Lt, n, s).is_err());
        assert!(binary(Lt, Type::ARRAY, a).is_err());
        assert_eq!(binary(Eq, Type::ARRAY, n), Ok(b));
        assert_eq!(binary(Star, Type::Never, s), Ok(Type::Never));
    }

    #[test]
    fn variables_and_joins() {
        assert_eq!(Type::NUMBER.of_new_variable(), Some(Type::NUMBER));
        assert_eq!(Type::UNDEFINED.of_new_variable(), Some(Type::Any));
        assert_eq!(Type::Never.of_new_variable(), None);
        assert_eq!(Type::Never.join(Type::NUMBER), Type::NUMBER);
        assert_eq!(Type::NUMBER.join(Type::STRING), Type::Any);
        assert_eq!(unary(SyntaxKind::Not, Type::NUMBER).ok(), None);
        assert_eq!(unary(SyntaxKind::Minus, Type::Any), Ok(Type::NUMBER));
        assert!(condition(Type::STRING).is_err());
    }
}

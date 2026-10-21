use std::fmt::{Display, Formatter};

/// Operand of `PUSH_C`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Constant {
    Undefined,
    Null,
    Boolean(bool),
    Number(f64),
    /// Index into the string table.
    String(u32),
    /// Seconds since the Unix epoch (UTC).
    Date(i64),
}

impl Constant {
    pub(crate) const TAG_UNDEFINED: u8 = 0;
    pub(crate) const TAG_NULL: u8 = 1;
    pub(crate) const TAG_FALSE: u8 = 2;
    pub(crate) const TAG_TRUE: u8 = 3;
    pub(crate) const TAG_NUMBER: u8 = 4;
    pub(crate) const TAG_STRING: u8 = 5;
    pub(crate) const TAG_DATE: u8 = 6;
}

impl Display for Constant {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Constant::Undefined => write!(f, "undefined"),
            Constant::Null => write!(f, "null"),
            Constant::Boolean(b) => write!(f, "{b}"),
            Constant::Number(n) => write!(f, "{n}"),
            Constant::String(idx) => write!(f, "str#{idx}"),
            Constant::Date(secs) => write!(f, "date({secs})"),
        }
    }
}

use std::fmt::{Display, Formatter};

/// Runtime type of a value, as checked by `CHECK_TYPE`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValueType {
    Undefined,
    Null,
    Boolean,
    Number,
    String,
    Date,
    Array,
}

impl ValueType {
    pub fn tag(self) -> u8 {
        self as u8
    }

    pub fn from_tag(tag: u8) -> Option<Self> {
        use ValueType::*;
        [Undefined, Null, Boolean, Number, String, Date, Array]
            .get(tag as usize)
            .copied()
    }

    /// Name of the type in the language.
    pub fn name(self) -> &'static str {
        match self {
            ValueType::Undefined => "Неопределено",
            ValueType::Null => "Null",
            ValueType::Boolean => "Булево",
            ValueType::Number => "Число",
            ValueType::String => "Строка",
            ValueType::Date => "Дата",
            ValueType::Array => "Массив",
        }
    }
}

impl Display for ValueType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tags_roundtrip() {
        for tag in 0..=u8::MAX {
            if let Some(value_type) = ValueType::from_tag(tag) {
                assert_eq!(value_type.tag(), tag);
            }
        }
        assert_eq!(
            ValueType::from_tag(ValueType::Array.tag()),
            Some(ValueType::Array)
        );
        assert_eq!(ValueType::from_tag(7), None);
    }
}

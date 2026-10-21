use crate::object::table_trait::{
    invalid_data, read_section_header, write_section_header, Deserializable, Serializable,
};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use dashmap::mapref::entry::Entry;
use dashmap::DashMap;
use std::io::{Read, Write};
use std::sync::{Arc, RwLock};

const STRING_TABLE_MAGIC: &[u8; 4] = b"STR\0";
const STRING_TABLE_VERSION: u32 = 1;

/// Interned strings: identifiers, string literals and native function names.
#[derive(Default)]
pub struct StringTable {
    pub arena: RwLock<Vec<Arc<str>>>,
    pub lookup: DashMap<Arc<str>, u32, ahash::RandomState>, // string to its location in arena
}

impl StringTable {
    pub fn new() -> Self {
        Self {
            arena: RwLock::new(Vec::with_capacity(4096)),
            lookup: DashMap::with_capacity_and_hasher(4096, ahash::RandomState::new()),
        }
    }

    /// Interns `string` and returns its id. Pushing the same string twice returns the same id.
    pub fn push(&self, string: &str) -> u32 {
        if let Some(id) = self.lookup.get(string) {
            return *id;
        }

        let string: Arc<str> = string.into();
        match self.lookup.entry(string.clone()) {
            Entry::Occupied(entry) => *entry.get(),
            Entry::Vacant(entry) => {
                let mut arena = self.arena.write().expect("string table lock poisoned");
                let id = arena.len() as u32;
                arena.push(string);
                entry.insert(id);
                id
            }
        }
    }

    pub fn get(&self, id: u32) -> Option<Arc<str>> {
        let arena = self.arena.read().ok()?;
        arena.get(id as usize).cloned()
    }

    pub fn len(&self) -> usize {
        self.arena.read().map(|a| a.len()).unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Serializable for StringTable {
    fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        let strings = self
            .arena
            .read()
            .map_err(|_| invalid_data("Failed to acquire read lock on arena"))?;
        write_section_header(writer, STRING_TABLE_MAGIC, STRING_TABLE_VERSION)?;

        writer.write_u32::<LittleEndian>(strings.len() as u32)?;
        for s in strings.iter() {
            let bytes = s.as_bytes();
            writer.write_u32::<LittleEndian>(bytes.len() as u32)?;
            writer.write_all(bytes)?;
        }

        Ok(())
    }
}

impl Deserializable for StringTable {
    fn deserialize<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        read_section_header(reader, STRING_TABLE_MAGIC, STRING_TABLE_VERSION)?;
        let st = Self::new();

        let count = reader.read_u32::<LittleEndian>()? as usize;
        for index in 0..count {
            let len = reader.read_u32::<LittleEndian>()? as usize;
            let mut bytes = vec![0u8; len];
            reader.read_exact(&mut bytes)?;
            let string = String::from_utf8(bytes).map_err(invalid_data_from)?;
            if st.push(&string) as usize != index {
                return Err(invalid_data(format!("duplicate string {string:?} in string table")));
            }
        }

        Ok(st)
    }
}

fn invalid_data_from(err: impl std::error::Error) -> std::io::Error {
    invalid_data(err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn test_new_string_table() {
        let st = StringTable::new();
        assert!(st.is_empty());
    }

    #[test]
    pub fn test_push_get() {
        const TEST_STRING: &str = "Hello, World";

        let st = StringTable::new();
        let id = st.push(TEST_STRING);
        assert_eq!(id, 0);
        assert_eq!(st.len(), 1);
        assert_eq!(&*st.get(id).unwrap(), TEST_STRING);
    }

    #[test]
    pub fn test_push_deduplicates() {
        let st = StringTable::new();
        let a = st.push("a");
        let b = st.push("b");
        assert_eq!(st.push("a"), a);
        assert_ne!(a, b);
        assert_eq!(st.len(), 2);
    }

    #[test]
    pub fn test_roundtrip() {
        let st = StringTable::new();
        st.push("Привет");
        st.push("");
        st.push("мир");

        let mut bytes = Vec::new();
        st.serialize(&mut bytes).unwrap();
        let restored = StringTable::deserialize(&mut bytes.as_slice()).unwrap();
        assert_eq!(restored.len(), 3);
        assert_eq!(&*restored.get(0).unwrap(), "Привет");
        assert_eq!(&*restored.get(1).unwrap(), "");
        assert_eq!(&*restored.get(2).unwrap(), "мир");
    }

    #[test]
    pub fn test_bad_magic() {
        let bytes = b"XXX\0\x01\0\0\0\0\0\0\0";
        assert!(StringTable::deserialize(&mut bytes.as_slice()).is_err());
    }
}

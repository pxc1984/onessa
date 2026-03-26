use crate::object::table_trait::{Deserializable, Serializable};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use dashmap::DashMap;
use std::hash::{BuildHasher, Hasher};
use std::io::{Read, Write};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, RwLock};

const STRING_TABLE_MAGIC: &[u8; 4] = b"STR\0";
const STRING_TABLE_VERSION: u32 = 1;

#[derive(Default)]
pub struct StringTable {
    pub arena: RwLock<Vec<Arc<str>>>,
    pub next_id: AtomicU32,
    pub lookup: DashMap<u64, u32, ahash::RandomState>, // string hashe to string location in arena

    pub hasher: ahash::RandomState,
}

impl StringTable {
    pub fn new() -> Self {
        Self {
            arena: RwLock::new(Vec::with_capacity(4096)),
            next_id: AtomicU32::new(0),
            lookup: DashMap::with_capacity_and_hasher(4096, ahash::RandomState::new()),
            hasher: ahash::RandomState::new(),
        }
    }

    pub fn push(&mut self, string: String) -> Option<u32> {
        let hash = self.hash_string(&string);

        if let Some(id) = self.lookup.get(&hash) {
            if string
                == self
                    .arena
                    .read()
                    .ok()?
                    .get(*id.value() as usize)?
                    .to_string()
            {
                return Some(*id.value());
            } else {
                // we have hash collision... i don't really know what to do here honestly
                panic!("hash collision during pushing into StringTable");
            }
        }

        let mut arena = self.arena.write().ok()?;
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        arena.push(string.into_boxed_str().into());

        self.lookup.insert(hash, id);

        Some(id)
    }

    pub fn get(&self, id: u32) -> Option<String> {
        let arena = self.arena.read().ok()?;
        let value = arena.get(id as usize)?;
        Some(value.to_string())
    }

    fn hash_string(&self, string: &String) -> u64 {
        self.hasher.hash_one(string)
    }
}

impl Serializable for StringTable {
    fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        let strings = self.arena.read().map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Failed to acquire read lock on arena",
            )
        })?;
        // Magic: "STR\0"
        writer.write_all(STRING_TABLE_MAGIC)?;

        // Version
        writer.write_u32::<LittleEndian>(STRING_TABLE_VERSION)?;

        // String count
        writer.write_u32::<LittleEndian>(strings.len() as u32)?;

        // Write each string
        for s in strings.clone().into_iter() {
            let bytes = s.as_bytes();
            writer.write_u32::<LittleEndian>(bytes.len() as u32)?;
            writer.write_all(bytes)?;
        }

        Ok(())
    }
}

impl Deserializable for StringTable {
    fn deserialize<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        let mut st = Self::new();

        // Check magic
        let mut magic = [0u8; 4];
        reader.read_exact(&mut magic)?;
        if &magic != STRING_TABLE_MAGIC {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Invalid string table magic: {:?}", magic),
            ));
        }

        // Version
        let version = reader.read_u32::<LittleEndian>()?;
        if version != STRING_TABLE_VERSION {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Unsupported string table version: {}", version),
            ));
        }

        // String count
        let count = reader.read_u32::<LittleEndian>()? as usize;

        for _ in 0..count {
            let len = reader.read_u32::<LittleEndian>()? as usize;
            let mut bytes = vec![0u8; len];
            reader.read_exact(&mut bytes)?;
            st.push(
                String::from_utf8(bytes)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?,
            );
        }

        Ok(st)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::Ordering;

    #[test]
    pub fn test_new_string_table() {
        let st = StringTable::new();
        assert_eq!(st.next_id.load(Ordering::Relaxed), 0);
    }

    #[test]
    pub fn test_push_get() {
        const TEST_STRING: &str = "Hello, World";

        let mut st = StringTable::new();
        assert_eq!(st.next_id.load(Ordering::Relaxed), 0);
        let id = st.push(TEST_STRING.to_string()).unwrap();
        assert_eq!(id, 0);
        assert_eq!(st.next_id.load(Ordering::Relaxed), 1);
        let value = st.get(id).unwrap();
        assert_eq!(value, TEST_STRING.to_string());
    }

    #[test]
    pub fn test_multi() {
        const TEST_STRINGS: &[&str] = &[
            "Hello, World!",
            "Hello, Russia!",
            "Hello, Moscow!",
            "Hello, Yulia!",
        ];

        let mut st = StringTable::new();
        let mut ids = Vec::new();
        for &string in TEST_STRINGS {
            ids.push(st.push(string.to_string()).unwrap());
        }
        assert_eq!(ids, vec![0, 1, 2, 3]);
        ids.reverse();
        assert_eq!(ids, vec![3, 2, 1, 0]);
        for id in ids {
            assert_eq!(st.get(id).unwrap(), TEST_STRINGS[id as usize].to_string());
        }
    }
}

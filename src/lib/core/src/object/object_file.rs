use crate::object::code_section::CodeSection;
use crate::object::function_table::FunctionTable;
use crate::object::metadata_ref_table::MetadataRefTable;
use crate::object::string_table::StringTable;
use crate::object::table_trait::{Deserializable, Serializable};
use std::io::{Cursor, Read, Write};
use std::path::PathBuf;
use std::sync::Arc;

pub struct ObjectFile {
    pub magic_number: Arc<[u8; 4]>,
    pub version: Arc<[u8; 4]>,
    pub string_table: StringTable,
    pub metadata_ref_table: MetadataRefTable,
    pub function_table: FunctionTable,
    pub code_section: CodeSection,
}

impl ObjectFile {
    pub fn from_file(path: PathBuf) -> std::io::Result<Self> {
        let mut bytes = Cursor::new(std::fs::read(path)?);

        Ok(Self {
            magic_number: Arc::deserialize(&mut bytes)?,
            version: Arc::deserialize(&mut bytes)?,
            string_table: StringTable::deserialize(&mut bytes)?,
            metadata_ref_table: MetadataRefTable {},
            function_table: FunctionTable {},
            code_section: CodeSection {},
        })
    }
}

use crate::object::code_section::CodeSection;
use crate::object::debug_info::DebugInfo;
use crate::object::function_table::FunctionTable;
use crate::object::metadata_ref_table::MetadataRefTable;
use crate::object::string_table::StringTable;
use crate::object::table_trait::{invalid_data, Deserializable, Serializable};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::{Read, Write};
use std::path::Path;

pub const MAGIC_NUMBER: &[u8; 4] = b"1CV8";
pub const OBJECT_FILE_VERSION: u32 = 0;

#[derive(Default)]
pub struct ObjectFile {
    pub string_table: StringTable,
    pub metadata_ref_table: MetadataRefTable,
    pub function_table: FunctionTable,
    pub code_section: CodeSection,
    pub debug_info: Option<DebugInfo>,
}

impl ObjectFile {
    pub fn from_file(path: impl AsRef<Path>) -> std::io::Result<Self> {
        let bytes = std::fs::read(path)?;
        Self::deserialize(&mut bytes.as_slice())
    }

    pub fn write_file(&self, path: impl AsRef<Path>) -> std::io::Result<()> {
        let mut bytes = Vec::new();
        self.serialize(&mut bytes)?;
        std::fs::write(path, bytes)
    }
}

impl Serializable for ObjectFile {
    fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        writer.write_all(MAGIC_NUMBER)?;
        writer.write_u32::<LittleEndian>(OBJECT_FILE_VERSION)?;
        self.string_table.serialize(writer)?;
        self.metadata_ref_table.serialize(writer)?;
        self.function_table.serialize(writer)?;
        self.code_section.serialize(writer)?;
        match &self.debug_info {
            Some(debug_info) => {
                writer.write_u8(1)?;
                debug_info.serialize(writer)
            }
            None => writer.write_u8(0),
        }
    }
}

impl Deserializable for ObjectFile {
    fn deserialize<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        let mut magic = [0u8; 4];
        reader.read_exact(&mut magic)?;
        if &magic != MAGIC_NUMBER {
            return Err(invalid_data("not an onessa object file (bad magic number)"));
        }
        let version = reader.read_u32::<LittleEndian>()?;
        if version != OBJECT_FILE_VERSION {
            return Err(invalid_data(format!(
                "unsupported object file version {version}"
            )));
        }

        Ok(Self {
            string_table: StringTable::deserialize(reader)?,
            metadata_ref_table: MetadataRefTable::deserialize(reader)?,
            function_table: FunctionTable::deserialize(reader)?,
            code_section: CodeSection::deserialize(reader)?,
            debug_info: match reader.read_u8()? {
                0 => None,
                _ => Some(DebugInfo::deserialize(reader)?),
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode::{Constant, Instruction};
    use crate::object::FunctionEntry;

    #[test]
    fn roundtrip() {
        let object = ObjectFile::default();
        let name = object.string_table.push("<module>");
        let mut code = Vec::new();
        Instruction::PushC(Constant::Number(1.0)).encode(&mut code);
        Instruction::Return.encode(&mut code);
        let object = ObjectFile {
            function_table: FunctionTable {
                entry: 0,
                global_count: 2,
                functions: vec![FunctionEntry {
                    name,
                    returns_value: false,
                    export: false,
                    param_count: 0,
                    local_count: 1,
                    code_offset: 0,
                    code_len: code.len() as u32,
                }],
            },
            code_section: CodeSection { code },
            debug_info: Some(DebugInfo {
                source_name: name,
                lines: vec![(0, 1)],
            }),
            ..object
        };

        let mut bytes = Vec::new();
        object.serialize(&mut bytes).unwrap();
        assert_eq!(&bytes[..4], MAGIC_NUMBER);

        let restored = ObjectFile::deserialize(&mut bytes.as_slice()).unwrap();
        assert_eq!(restored.function_table, object.function_table);
        assert_eq!(restored.code_section, object.code_section);
        assert_eq!(restored.debug_info, object.debug_info);
        assert_eq!(&*restored.string_table.get(name).unwrap(), "<module>");
    }

    #[test]
    fn rejects_garbage() {
        assert!(ObjectFile::deserialize(&mut &b"nope"[..]).is_err());
        assert!(ObjectFile::deserialize(&mut &b""[..]).is_err());
    }
}

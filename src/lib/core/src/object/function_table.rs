use crate::object::table_trait::{
    read_section_header, write_section_header, Deserializable, Serializable,
};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::{Read, Write};

const FUNCTION_TABLE_MAGIC: &[u8; 4] = b"FUN\0";
const FUNCTION_TABLE_VERSION: u32 = 1;

const FLAG_RETURNS_VALUE: u8 = 1 << 0;
const FLAG_EXPORT: u8 = 1 << 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionEntry {
    /// Index of the name in the string table.
    pub name: u32,
    /// `Функция` (true) or `Процедура` (false).
    pub returns_value: bool,
    pub export: bool,
    pub param_count: u8,
    /// Total number of local slots, parameters included.
    pub local_count: u16,
    /// Byte offset of the first instruction inside the code section.
    pub code_offset: u32,
    pub code_len: u32,
}

/// Module methods plus module-level information.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct FunctionTable {
    /// Index of the module body (the code executed when the module is run).
    pub entry: u32,
    /// Number of module variables (`Перем` at module level).
    pub global_count: u16,
    pub functions: Vec<FunctionEntry>,
}

impl Serializable for FunctionTable {
    fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        write_section_header(writer, FUNCTION_TABLE_MAGIC, FUNCTION_TABLE_VERSION)?;
        writer.write_u32::<LittleEndian>(self.entry)?;
        writer.write_u16::<LittleEndian>(self.global_count)?;
        writer.write_u32::<LittleEndian>(self.functions.len() as u32)?;
        for f in &self.functions {
            let mut flags = 0;
            if f.returns_value {
                flags |= FLAG_RETURNS_VALUE;
            }
            if f.export {
                flags |= FLAG_EXPORT;
            }
            writer.write_u32::<LittleEndian>(f.name)?;
            writer.write_u8(flags)?;
            writer.write_u8(f.param_count)?;
            writer.write_u16::<LittleEndian>(f.local_count)?;
            writer.write_u32::<LittleEndian>(f.code_offset)?;
            writer.write_u32::<LittleEndian>(f.code_len)?;
        }
        Ok(())
    }
}

impl Deserializable for FunctionTable {
    fn deserialize<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        read_section_header(reader, FUNCTION_TABLE_MAGIC, FUNCTION_TABLE_VERSION)?;
        let entry = reader.read_u32::<LittleEndian>()?;
        let global_count = reader.read_u16::<LittleEndian>()?;
        let count = reader.read_u32::<LittleEndian>()? as usize;
        let mut functions = Vec::with_capacity(count.min(4096));
        for _ in 0..count {
            let name = reader.read_u32::<LittleEndian>()?;
            let flags = reader.read_u8()?;
            functions.push(FunctionEntry {
                name,
                returns_value: flags & FLAG_RETURNS_VALUE != 0,
                export: flags & FLAG_EXPORT != 0,
                param_count: reader.read_u8()?,
                local_count: reader.read_u16::<LittleEndian>()?,
                code_offset: reader.read_u32::<LittleEndian>()?,
                code_len: reader.read_u32::<LittleEndian>()?,
            });
        }
        Ok(Self {
            entry,
            global_count,
            functions,
        })
    }
}

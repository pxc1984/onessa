use crate::object::table_trait::{
    invalid_data, read_section_header, write_section_header, Deserializable, Serializable,
};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::{Read, Write};

const METADATA_REF_TABLE_MAGIC: &[u8; 4] = b"MDR\0";
const METADATA_REF_TABLE_VERSION: u32 = 1;

/// References to pre-compiled metadata objects (catalogs, documents, ...).
///
/// v0 has no metadata support, so the table is always empty.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct MetadataRefTable {}

impl Serializable for MetadataRefTable {
    fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        write_section_header(writer, METADATA_REF_TABLE_MAGIC, METADATA_REF_TABLE_VERSION)?;
        writer.write_u32::<LittleEndian>(0)
    }
}

impl Deserializable for MetadataRefTable {
    fn deserialize<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        read_section_header(reader, METADATA_REF_TABLE_MAGIC, METADATA_REF_TABLE_VERSION)?;
        match reader.read_u32::<LittleEndian>()? {
            0 => Ok(Self {}),
            count => Err(invalid_data(format!(
                "metadata references are not supported yet ({count} entries)"
            ))),
        }
    }
}

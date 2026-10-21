use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::{Read, Write};

pub trait Serializable {
    fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()>;
}

pub trait Deserializable: Sized {
    fn deserialize<R: Read>(reader: &mut R) -> std::io::Result<Self>;
}

pub(crate) fn invalid_data(message: impl Into<String>) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, message.into())
}

/// Writes the `magic` + `version` prefix every section starts with.
pub(crate) fn write_section_header<W: Write>(
    writer: &mut W,
    magic: &[u8; 4],
    version: u32,
) -> std::io::Result<()> {
    writer.write_all(magic)?;
    writer.write_u32::<LittleEndian>(version)
}

/// Reads and validates the `magic` + `version` prefix of a section.
pub(crate) fn read_section_header<R: Read>(
    reader: &mut R,
    magic: &[u8; 4],
    version: u32,
) -> std::io::Result<()> {
    let mut actual = [0u8; 4];
    reader.read_exact(&mut actual)?;
    if &actual != magic {
        return Err(invalid_data(format!(
            "invalid section magic: expected {:?}, got {:?}",
            String::from_utf8_lossy(magic),
            String::from_utf8_lossy(&actual)
        )));
    }
    let actual = reader.read_u32::<LittleEndian>()?;
    if actual != version {
        return Err(invalid_data(format!(
            "unsupported {} section version: {actual}",
            String::from_utf8_lossy(&magic[..3])
        )));
    }
    Ok(())
}

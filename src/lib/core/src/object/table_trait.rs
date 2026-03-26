use byteorder::ReadBytesExt;
use std::io::{Read, Write};
use std::sync::Arc;

pub trait Serializable {
    fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()>;
}
pub trait Deserializable {
    fn deserialize<R: Read>(reader: &mut R) -> std::io::Result<Self>;
}

impl Serializable for Arc<[u8; 4]> {
    fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        writer.write_all(self.as_ref())
    }
}

impl Deserializable for Arc<[u8; 4]> {
    fn deserialize<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        let mut array = [0u8; 4];
        reader.read_exact(&mut array)?;
        Ok(Arc::new(array))
    }
}

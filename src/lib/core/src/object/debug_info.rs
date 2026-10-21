use crate::object::table_trait::{
    read_section_header, write_section_header, Deserializable, Serializable,
};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::{Read, Write};

const DEBUG_INFO_MAGIC: &[u8; 4] = b"DBG\0";
const DEBUG_INFO_VERSION: u32 = 1;

/// Optional section mapping code offsets back to source lines.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct DebugInfo {
    /// Index of the source file name in the string table.
    pub source_name: u32,
    /// `(code offset, line)` pairs sorted by offset; a pair covers code up to the next one.
    pub lines: Vec<(u32, u32)>,
}

impl DebugInfo {
    /// Source line of the instruction at `offset`, if known.
    pub fn line_at(&self, offset: u32) -> Option<u32> {
        let idx = self.lines.partition_point(|(o, _)| *o <= offset);
        idx.checked_sub(1).map(|i| self.lines[i].1)
    }
}

impl Serializable for DebugInfo {
    fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        write_section_header(writer, DEBUG_INFO_MAGIC, DEBUG_INFO_VERSION)?;
        writer.write_u32::<LittleEndian>(self.source_name)?;
        writer.write_u32::<LittleEndian>(self.lines.len() as u32)?;
        for (offset, line) in &self.lines {
            writer.write_u32::<LittleEndian>(*offset)?;
            writer.write_u32::<LittleEndian>(*line)?;
        }
        Ok(())
    }
}

impl Deserializable for DebugInfo {
    fn deserialize<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        read_section_header(reader, DEBUG_INFO_MAGIC, DEBUG_INFO_VERSION)?;
        let source_name = reader.read_u32::<LittleEndian>()?;
        let count = reader.read_u32::<LittleEndian>()? as usize;
        let mut lines = Vec::with_capacity(count.min(1 << 16));
        for _ in 0..count {
            lines.push((
                reader.read_u32::<LittleEndian>()?,
                reader.read_u32::<LittleEndian>()?,
            ));
        }
        Ok(Self { source_name, lines })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_lookup() {
        let info = DebugInfo {
            source_name: 0,
            lines: vec![(0, 1), (10, 3), (20, 7)],
        };
        assert_eq!(info.line_at(0), Some(1));
        assert_eq!(info.line_at(9), Some(1));
        assert_eq!(info.line_at(10), Some(3));
        assert_eq!(info.line_at(25), Some(7));
        assert_eq!(DebugInfo::default().line_at(5), None);
    }
}

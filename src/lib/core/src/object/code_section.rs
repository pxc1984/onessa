use crate::bytecode::{DecodeError, Instruction};
use crate::object::table_trait::{
    read_section_header, write_section_header, Deserializable, Serializable,
};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::{Read, Write};

const CODE_SECTION_MAGIC: &[u8; 4] = b"COD\0";
const CODE_SECTION_VERSION: u32 = 1;

/// Linear bytecode of all module methods.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct CodeSection {
    pub code: Vec<u8>,
}

impl CodeSection {
    /// Decodes the instructions in `start..end` together with their offsets.
    pub fn instructions(
        &self,
        start: usize,
        end: usize,
    ) -> Result<Vec<(usize, Instruction)>, DecodeError> {
        let code = &self.code[..end.min(self.code.len())];
        let mut pc = start;
        let mut out = Vec::new();
        while pc < code.len() {
            let offset = pc;
            out.push((offset, Instruction::decode(code, &mut pc)?));
        }
        Ok(out)
    }
}

impl Serializable for CodeSection {
    fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        write_section_header(writer, CODE_SECTION_MAGIC, CODE_SECTION_VERSION)?;
        writer.write_u32::<LittleEndian>(self.code.len() as u32)?;
        writer.write_all(&self.code)
    }
}

impl Deserializable for CodeSection {
    fn deserialize<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        read_section_header(reader, CODE_SECTION_MAGIC, CODE_SECTION_VERSION)?;
        let len = reader.read_u32::<LittleEndian>()? as usize;
        let mut code = Vec::new();
        reader.take(len as u64).read_to_end(&mut code)?;
        if code.len() != len {
            return Err(std::io::ErrorKind::UnexpectedEof.into());
        }
        Ok(Self { code })
    }
}

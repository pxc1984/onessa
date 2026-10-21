use crate::bytecode::{Constant, Instruction};
use crate::object::ObjectFile;
use std::fmt::Write;

impl ObjectFile {
    /// Human readable listing of every method, annotated with strings and source lines.
    pub fn disassemble(&self) -> String {
        let mut out = String::new();
        let table = &self.function_table;
        writeln!(
            out,
            "globals: {}, entry: fn#{}",
            table.global_count, table.entry
        )
        .unwrap();

        for (idx, function) in table.functions.iter().enumerate() {
            let name = self.string_table.get(function.name).unwrap_or_default();
            writeln!(
                out,
                "fn#{idx} {name} (params: {}, locals: {})",
                function.param_count, function.local_count
            )
            .unwrap();

            let start = function.code_offset as usize;
            let end = start + function.code_len as usize;
            let instructions = match self.code_section.instructions(start, end) {
                Ok(instructions) => instructions,
                Err(err) => {
                    writeln!(out, "  <{err}>").unwrap();
                    continue;
                }
            };

            let mut last_line = None;
            for (offset, instruction) in instructions {
                write!(out, "  {offset:04}  {instruction}").unwrap();
                let string = match instruction {
                    Instruction::PushC(Constant::String(idx)) => Some(idx),
                    Instruction::CallNative { name, .. } => Some(name),
                    _ => None,
                };
                if let Some(text) = string.and_then(|idx| self.string_table.get(idx)) {
                    write!(out, "  ; {text:?}").unwrap();
                }
                let line = self
                    .debug_info
                    .as_ref()
                    .and_then(|d| d.line_at(offset as u32));
                if let Some(number) = line.filter(|_| line != last_line) {
                    write!(out, "  ; line {number}").unwrap();
                    last_line = line;
                }
                out.push('\n');
            }
        }
        out
    }
}

//! Bytecode generation: lays out the basic blocks of every method linearly and resolves jump
//! targets to code offsets.

use onessa_cfg::{BlockId, Function, Module, Terminator};
use onessa_core::bytecode::Instruction;
use onessa_core::object::{
    CodeSection, DebugInfo, FunctionEntry, FunctionTable, MetadataRefTable, ObjectFile,
};
use std::collections::HashMap;

/// Generates the object file for a lowered module. `source_name` is recorded in the debug info.
pub fn generate(module: Module, source_name: &str) -> ObjectFile {
    let mut code = Vec::new();
    let mut lines = Vec::new();
    let mut functions = Vec::with_capacity(module.functions.len());

    for function in &module.functions {
        let code_offset = code.len() as u32;
        emit_function(function, &mut code, &mut lines);
        functions.push(FunctionEntry {
            name: module.strings.push(&function.name),
            returns_value: function.returns_value,
            export: function.export,
            param_count: function.params.len() as u8,
            local_count: function.locals.len() as u16,
            code_offset,
            code_len: code.len() as u32 - code_offset,
        });
    }

    let source_name = module.strings.push(source_name);
    ObjectFile {
        string_table: module.strings,
        metadata_ref_table: MetadataRefTable::default(),
        function_table: FunctionTable {
            entry: module.entry as u32,
            global_count: module.globals.len() as u16,
            functions,
        },
        code_section: CodeSection { code },
        debug_info: Some(DebugInfo { source_name, lines }),
    }
}

/// Instructions of a laid out block, still jumping to block ids, with their source lines.
type BlockCode = Vec<(Instruction<BlockId>, u32)>;

/// Instructions a terminator turns into, given the block laid out right after it.
fn lower_terminator(terminator: Terminator, next: Option<BlockId>) -> Vec<Instruction<BlockId>> {
    match terminator {
        Terminator::Jump(target) if Some(target) == next => Vec::new(),
        Terminator::Jump(target) => vec![Instruction::Jump(target)],
        Terminator::Branch { then, otherwise } if Some(then) == next => {
            vec![Instruction::JumpIfFalse(otherwise)]
        }
        Terminator::Branch { then, otherwise } => {
            vec![Instruction::JumpIfFalse(otherwise), Instruction::Jump(then)]
        }
        Terminator::ForCheck {
            counter,
            limit,
            body,
            exit,
        } => {
            let mut code = vec![Instruction::ForCheck {
                counter,
                limit,
                target: exit,
            }];
            if Some(body) != next {
                code.push(Instruction::Jump(body));
            }
            code
        }
        Terminator::ForStep {
            counter,
            limit,
            body,
            exit,
        } => {
            let mut code = vec![Instruction::ForStep {
                counter,
                limit,
                target: body,
            }];
            if Some(exit) != next {
                code.push(Instruction::Jump(exit));
            }
            code
        }
        Terminator::Return => vec![Instruction::Return],
        Terminator::Raise => vec![Instruction::Raise],
        Terminator::ReRaise => vec![Instruction::ReRaise],
    }
}

fn emit_function(function: &Function, code: &mut Vec<u8>, lines: &mut Vec<(u32, u32)>) {
    let order = function.reachable_blocks();

    // Flatten blocks into instructions that still jump to block ids.
    let mut blocks: Vec<(BlockId, BlockCode)> = Vec::new();
    for (position, &id) in order.iter().enumerate() {
        let block = &function.blocks[id];
        let (terminator, line) = block
            .terminator
            .expect("reachable block must be terminated");
        let mut instructions = block.instructions.clone();
        instructions.extend(
            lower_terminator(terminator, order.get(position + 1).copied())
                .into_iter()
                .map(|instruction| (instruction, line)),
        );
        blocks.push((id, instructions));
    }

    // Every instruction has a fixed size, so block offsets are known before encoding.
    let mut offsets = HashMap::new();
    let mut offset = code.len() as u32;
    for (id, instructions) in &blocks {
        offsets.insert(*id, offset);
        offset += instructions
            .iter()
            .map(|(instruction, _)| instruction.map_target(|_| 0u32).encoded_len() as u32)
            .sum::<u32>();
    }

    for (_, instructions) in blocks {
        for (instruction, line) in instructions {
            if lines.last().is_none_or(|&(_, last)| last != line) {
                lines.push((code.len() as u32, line));
            }
            instruction.map_target(|block| offsets[&block]).encode(code);
        }
    }
}

#[cfg(test)]
mod tests;

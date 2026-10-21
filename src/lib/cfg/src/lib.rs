//! Control flow graph of a module, built from the syntax tree.
//!
//! Every method is lowered into basic blocks of stack machine instructions
//! ([`onessa_core::bytecode::Instruction`] with block ids as jump targets) that end with an
//! explicit [`Terminator`]. Name resolution and the other semantic checks of the language happen
//! during lowering, so a successfully built [`Module`] is ready for code generation.

mod diagnostic;
mod display;
mod lower;

pub use diagnostic::Diagnostic;
pub use lower::build_cfg;

use onessa_core::bytecode::Instruction;
use onessa_core::object::StringTable;

pub type BlockId = usize;

pub struct Module {
    /// Module variables declared with `Перем` at module level.
    pub globals: Vec<String>,
    pub functions: Vec<Function>,
    /// Index of the module body in `functions`.
    pub entry: usize,
    /// Strings referenced by `PUSH_C` and `CALL_NATIVE`, plus method names.
    pub strings: StringTable,
}

pub struct Function {
    pub name: String,
    pub returns_value: bool,
    pub export: bool,
    pub params: Vec<Parameter>,
    /// All local slots: parameters first, then declared and implicit locals.
    pub locals: Vec<String>,
    /// Block 0 is the entry block.
    pub blocks: Vec<BasicBlock>,
}

#[derive(Debug, Clone)]
pub struct Parameter {
    pub name: String,
    /// `Знач`: passed by value instead of by reference.
    pub by_value: bool,
    pub default: Option<onessa_core::bytecode::Constant>,
}

#[derive(Default)]
pub struct BasicBlock {
    /// Instructions with the source line they were generated from.
    pub instructions: Vec<(Instruction<BlockId>, u32)>,
    /// `None` only for blocks that are never reached.
    pub terminator: Option<(Terminator, u32)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Terminator {
    Jump(BlockId),
    /// Pops a boolean and continues in `then` if it is `Истина`, otherwise in `otherwise`.
    Branch {
        then: BlockId,
        otherwise: BlockId,
    },
    /// Returns the value on top of the stack.
    Return,
    /// Raises an exception with the message on top of the stack.
    Raise,
    /// Re-raises the exception being handled.
    ReRaise,
    /// Header of `Для`: continues in `body` if local `counter <= limit`, otherwise in `exit`.
    ForCheck {
        counter: u16,
        limit: u16,
        body: BlockId,
        exit: BlockId,
    },
    /// End of an iteration of `Для`: increments local `counter`, then continues like
    /// `ForCheck`.
    ForStep {
        counter: u16,
        limit: u16,
        body: BlockId,
        exit: BlockId,
    },
}

impl Terminator {
    pub fn successors(&self) -> Vec<BlockId> {
        match *self {
            Terminator::Jump(target) => vec![target],
            Terminator::Branch { then, otherwise } => vec![then, otherwise],
            Terminator::ForCheck { body, exit, .. } | Terminator::ForStep { body, exit, .. } => {
                vec![body, exit]
            }
            Terminator::Return | Terminator::Raise | Terminator::ReRaise => Vec::new(),
        }
    }
}

impl Function {
    /// Blocks reachable from the entry block in reverse postorder, which is also the layout
    /// order: a block comes before its successors (back edges aside), and the first successor
    /// of a block tends to directly follow it.
    ///
    /// Exception handlers count as reachable through the `TRY_BEGIN` that installs them.
    pub fn reachable_blocks(&self) -> Vec<BlockId> {
        let successors = |id: BlockId| {
            let block = &self.blocks[id];
            let mut next: Vec<BlockId> = block
                .terminator
                .map(|(terminator, _)| terminator.successors())
                .unwrap_or_default();
            next.extend(block.instructions.iter().filter_map(
                |(instruction, _)| match instruction {
                    Instruction::TryBegin(handler) => Some(*handler),
                    _ => None,
                },
            ));
            // `pop` visits the last successor first, so it finishes first and is placed last.
            next
        };

        let mut visited = vec![false; self.blocks.len()];
        let mut postorder = Vec::new();
        let mut stack = vec![(0, successors(0))];
        visited[0] = true;
        while let Some((id, pending)) = stack.last_mut() {
            match pending.pop() {
                Some(next) if !visited[next] => {
                    visited[next] = true;
                    let next_successors = successors(next);
                    stack.push((next, next_successors));
                }
                Some(_) => {}
                None => {
                    postorder.push(*id);
                    stack.pop();
                }
            }
        }
        postorder.reverse();
        postorder
    }
}

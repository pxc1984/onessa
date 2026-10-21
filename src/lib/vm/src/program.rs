//! An object file prepared for execution: the bytecode is validated and decoded once into a
//! flat array of [`Op`]s, so the interpreter never decodes bytes or looks names up.
//!
//! While loading, frequent instruction sequences are fused into superinstructions (see
//! [`fuse`]); they behave exactly like the sequences they replace.

use crate::error::LoadError;
use crate::native::Native;
use crate::value::Value;
use onessa_core::builtins::{self, ArrayMethod};
use onessa_core::bytecode::{Constant, Instruction, ValueType};
use onessa_core::object::ObjectFile;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arith {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cmp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

/// Instruction of the interpreter. Jump targets are indices into [`Program::ops`], constants
/// are indices into [`Program::constants`], natives are resolved.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Op {
    PushC(u32),
    PushV(u16),
    PopV(u16),
    PushG(u16),
    PopG(u16),
    Pop,
    Dup,
    CheckType(ValueType),
    Arith(Arith),
    Neg,
    Cmp(Cmp),
    Not,
    Jump(u32),
    JumpIfFalse(u32),
    ForCheck {
        counter: u16,
        limit: u16,
        target: u32,
    },
    ForStep {
        counter: u16,
        limit: u16,
        target: u32,
    },
    Call {
        function: u32,
        argc: u8,
    },
    CallNative {
        native: Native,
        argc: u8,
    },
    PushOut(u8),
    Return,
    TryBegin(u32),
    TryEnd,
    Raise,
    ReRaise,
    New {
        value_type: Option<ValueType>,
        type_name: u32,
        argc: u8,
    },
    IndexGet,
    IndexSet,
    CallMethod {
        method: Option<ArrayMethod>,
        name: u32,
        argc: u8,
    },

    // Superinstructions.
    /// `PUSH_V slot; INDEX_GET`
    IndexGetV(u16),
    /// `PUSH_C constant; INDEX_GET`
    IndexGetC(u32),
    /// `PUSH_V collection; PUSH_V index; INDEX_GET`
    IndexVV {
        collection: u16,
        index: u16,
    },
    /// `PUSH_V collection; PUSH_V i; INDEX_GET; PUSH_V k; INDEX_GET`
    IndexVVV {
        collection: u16,
        i: u16,
        k: u16,
    },
    /// `PUSH_V slot; <arith>`
    ArithV {
        op: Arith,
        slot: u16,
    },
    /// `PUSH_C constant; <arith>`
    ArithC {
        op: Arith,
        constant: u32,
    },
    /// `PUSH_V slot; <expression>; <arith>; POP_V slot` where the expression cannot change
    /// the slot: pops the value of the expression and updates the slot in place.
    ArithToV {
        op: Arith,
        slot: u16,
    },
    /// `PUSH_C constant; ARITH_TO_V slot`
    ArithToVC {
        op: Arith,
        slot: u16,
        constant: u32,
    },
    /// `PUSH_V collection; <index>; <value>; INDEX_SET` where the expressions cannot change
    /// the local: pops the value and the index.
    IndexSetV(u16),
    /// `<cmp>; JUMP_IF_FALSE target`
    CmpJump {
        op: Cmp,
        target: u32,
    },
    /// `PUSH_V slot; <cmp>; JUMP_IF_FALSE target`
    CmpVJump {
        op: Cmp,
        slot: u16,
        target: u32,
    },
    /// `PUSH_V lhs; PUSH_V rhs; <cmp>; JUMP_IF_FALSE target`
    CmpVVJump {
        op: Cmp,
        lhs: u16,
        rhs: u16,
        target: u32,
    },
    /// `PUSH_V slot; PUSH_C constant; <cmp>; JUMP_IF_FALSE target`
    CmpVCJump {
        op: Cmp,
        slot: u16,
        constant: u32,
        target: u32,
    },
    /// `PUSH_C constant; <cmp>; JUMP_IF_FALSE target`
    CmpCJump {
        op: Cmp,
        constant: u32,
        target: u32,
    },
}

impl Op {
    fn map_target(self, f: impl FnOnce(u32) -> u32) -> Op {
        match self {
            Op::Jump(t) => Op::Jump(f(t)),
            Op::JumpIfFalse(t) => Op::JumpIfFalse(f(t)),
            Op::TryBegin(t) => Op::TryBegin(f(t)),
            Op::ForCheck {
                counter,
                limit,
                target,
            } => Op::ForCheck {
                counter,
                limit,
                target: f(target),
            },
            Op::ForStep {
                counter,
                limit,
                target,
            } => Op::ForStep {
                counter,
                limit,
                target: f(target),
            },
            Op::CmpJump { op, target } => Op::CmpJump {
                op,
                target: f(target),
            },
            Op::CmpVJump { op, slot, target } => Op::CmpVJump {
                op,
                slot,
                target: f(target),
            },
            Op::CmpCJump {
                op,
                constant,
                target,
            } => Op::CmpCJump {
                op,
                constant,
                target: f(target),
            },
            Op::CmpVVJump {
                op,
                lhs,
                rhs,
                target,
            } => Op::CmpVVJump {
                op,
                lhs,
                rhs,
                target: f(target),
            },
            Op::CmpVCJump {
                op,
                slot,
                constant,
                target,
            } => Op::CmpVCJump {
                op,
                slot,
                constant,
                target: f(target),
            },
            op => op,
        }
    }

    fn target(self) -> Option<u32> {
        let mut target = None;
        self.map_target(|t| {
            target = Some(t);
            t
        });
        target
    }
}

pub struct FunctionInfo {
    /// Index of the first op.
    pub entry: usize,
    pub param_count: usize,
    pub local_count: usize,
}

pub struct Program {
    pub ops: Vec<Op>,
    /// Code section offset of every op, for source lines in error messages.
    pub op_offsets: Vec<u32>,
    pub constants: Vec<Value>,
    pub functions: Vec<FunctionInfo>,
    pub entry: u32,
    pub global_count: usize,
    pub strings: Vec<Rc<str>>,
}

/// An op of one function while it is being optimized. Jump targets are still code offsets.
#[derive(Debug, Clone, Copy)]
struct Entry {
    /// Code offset of the (first) instruction the op comes from.
    offset: u32,
    /// Some jump lands on this op, so it cannot be fused into the op before it.
    is_target: bool,
    op: Op,
}

impl Program {
    /// Checks everything the interpreter relies on without re-checking at run time:
    /// instructions decode, operands are in range, jumps stay inside their function, natives
    /// exist. Then translates the bytecode into ops.
    pub fn load(object: &ObjectFile) -> Result<Self, LoadError> {
        let strings: Vec<Rc<str>> = (0..object.string_table.len() as u32)
            .map(|idx| Rc::from(&*object.string_table.get(idx).unwrap_or_default()))
            .collect();
        let table = &object.function_table;
        let fail = |message: String| Err(LoadError(message));

        if table.entry as usize >= table.functions.len() {
            return fail(format!("entry function #{} does not exist", table.entry));
        }

        let mut program = Program {
            ops: Vec::new(),
            op_offsets: Vec::new(),
            constants: Vec::new(),
            functions: Vec::with_capacity(table.functions.len()),
            entry: table.entry,
            global_count: table.global_count as usize,
            strings,
        };

        for idx in 0..table.functions.len() {
            let entries = program.translate_function(object, idx)?;
            let entries = fuse(entries);

            // Ops of this function start right after the previous ones.
            let first_op = program.ops.len();
            let op_index: HashMap<u32, u32> = entries
                .iter()
                .enumerate()
                .filter(|(_, entry)| entry.is_target)
                .map(|(i, entry)| (entry.offset, (first_op + i) as u32))
                .collect();
            for entry in entries {
                program.ops.push(entry.op.map_target(|t| op_index[&t]));
                program.op_offsets.push(entry.offset);
            }

            let function = &table.functions[idx];
            program.functions.push(FunctionInfo {
                entry: first_op,
                param_count: function.param_count as usize,
                local_count: function.local_count as usize,
            });
        }
        Ok(program)
    }

    /// Validates the bytecode of a function and translates it one instruction per op.
    fn translate_function(
        &mut self,
        object: &ObjectFile,
        idx: usize,
    ) -> Result<Vec<Entry>, LoadError> {
        let table = &object.function_table;
        let code = &object.code_section;
        let function = &table.functions[idx];
        let fail = |message: String| Err(LoadError(message));

        let start = function.code_offset as usize;
        let end = start + function.code_len as usize;
        if end > code.code.len() || u16::from(function.param_count) > function.local_count {
            return fail(format!("function #{idx} has an invalid layout"));
        }
        let instructions = code
            .instructions(start, end)
            .map_err(|e| LoadError(format!("function #{idx}: {e}")))?;
        if !matches!(
            instructions.last(),
            Some((
                _,
                Instruction::Return
                    | Instruction::Raise
                    | Instruction::ReRaise
                    | Instruction::Jump(_)
            ))
        ) {
            return fail(format!("function #{idx} does not end with a return"));
        }
        let starts: HashSet<u32> = instructions.iter().map(|(o, _)| *o as u32).collect();

        let mut entries = Vec::with_capacity(instructions.len());
        for (offset, instruction) in instructions {
            let bad = |what: &str| LoadError(format!("function #{idx} at {offset}: {what}"));
            let target = |target: u32| {
                if starts.contains(&target) {
                    Ok(target)
                } else {
                    Err(bad("jump target is not an instruction of this function"))
                }
            };
            let string = |s: u32| {
                if (s as usize) < self.strings.len() {
                    Ok(s)
                } else {
                    Err(bad("string index out of range"))
                }
            };
            let local = |i: u16| {
                if i < function.local_count {
                    Ok(i)
                } else {
                    Err(bad("local index out of range"))
                }
            };
            let global = |i: u16| {
                if i < table.global_count {
                    Ok(i)
                } else {
                    Err(bad("global index out of range"))
                }
            };

            let op = match instruction {
                Instruction::PushC(constant) => {
                    let value = match constant {
                        Constant::Undefined => Value::Undefined,
                        Constant::Null => Value::Null,
                        Constant::Boolean(b) => Value::Boolean(b),
                        Constant::Number(n) => Value::number(n),
                        Constant::String(s) => {
                            Value::String(self.strings[string(s)? as usize].clone())
                        }
                        Constant::Date(secs) => Value::date_from_timestamp(secs)
                            .unwrap_or(Value::Date(Value::empty_date())),
                    };
                    self.constants.push(value);
                    Op::PushC(self.constants.len() as u32 - 1)
                }
                Instruction::PushV(i) => Op::PushV(local(i)?),
                Instruction::PopV(i) => Op::PopV(local(i)?),
                Instruction::PushG(i) => Op::PushG(global(i)?),
                Instruction::PopG(i) => Op::PopG(global(i)?),
                Instruction::Pop => Op::Pop,
                Instruction::Dup => Op::Dup,
                Instruction::CheckType(t) => Op::CheckType(t),
                Instruction::Add => Op::Arith(Arith::Add),
                Instruction::Sub => Op::Arith(Arith::Sub),
                Instruction::Mul => Op::Arith(Arith::Mul),
                Instruction::Div => Op::Arith(Arith::Div),
                Instruction::Mod => Op::Arith(Arith::Mod),
                Instruction::Neg => Op::Neg,
                Instruction::Eq => Op::Cmp(Cmp::Eq),
                Instruction::Ne => Op::Cmp(Cmp::Ne),
                Instruction::Lt => Op::Cmp(Cmp::Lt),
                Instruction::Le => Op::Cmp(Cmp::Le),
                Instruction::Gt => Op::Cmp(Cmp::Gt),
                Instruction::Ge => Op::Cmp(Cmp::Ge),
                Instruction::Not => Op::Not,
                Instruction::Jump(t) => Op::Jump(target(t)?),
                Instruction::JumpIfFalse(t) => Op::JumpIfFalse(target(t)?),
                Instruction::ForCheck {
                    counter,
                    limit,
                    target: t,
                } => Op::ForCheck {
                    counter: local(counter)?,
                    limit: local(limit)?,
                    target: target(t)?,
                },
                Instruction::ForStep {
                    counter,
                    limit,
                    target: t,
                } => Op::ForStep {
                    counter: local(counter)?,
                    limit: local(limit)?,
                    target: target(t)?,
                },
                Instruction::Call {
                    function: callee,
                    argc,
                } => match table.functions.get(callee as usize) {
                    Some(f) if f.param_count == argc => Op::Call {
                        function: callee,
                        argc,
                    },
                    _ => return Err(bad("call to a missing function or with wrong arity")),
                },
                Instruction::CallNative { name, argc } => {
                    let native = Native::resolve(&self.strings[string(name)? as usize])
                        .ok_or_else(|| bad("unknown native function"))?;
                    Op::CallNative { native, argc }
                }
                Instruction::PushOut(n) => Op::PushOut(n),
                Instruction::Return => Op::Return,
                Instruction::TryBegin(t) => Op::TryBegin(target(t)?),
                Instruction::TryEnd => Op::TryEnd,
                Instruction::Raise => Op::Raise,
                Instruction::ReRaise => Op::ReRaise,
                // Type and method names are resolved once here, not on every execution.
                Instruction::New { type_name, argc } => Op::New {
                    value_type: builtins::lookup_type(&self.strings[string(type_name)? as usize]),
                    type_name,
                    argc,
                },
                Instruction::IndexGet => Op::IndexGet,
                Instruction::IndexSet => Op::IndexSet,
                Instruction::CallMethod { name, argc } => Op::CallMethod {
                    method: builtins::lookup_array_method(&self.strings[string(name)? as usize])
                        .map(|m| m.id),
                    name,
                    argc,
                },
            };
            entries.push(Entry {
                offset: offset as u32,
                is_target: false,
                op,
            });
        }

        let targets: HashSet<u32> = entries.iter().filter_map(|e| e.op.target()).collect();
        for entry in &mut entries {
            entry.is_target = targets.contains(&entry.offset);
        }
        Ok(entries)
    }
}

/// Replaces instruction sequences with superinstructions. A sequence is only fused when no
/// jump lands inside it.
fn fuse(entries: Vec<Entry>) -> Vec<Entry> {
    let entries = fuse_updates(entries);

    let mut out: Vec<Entry> = Vec::with_capacity(entries.len());
    let mut i = 0;
    while i < entries.len() {
        let first = entries[i];
        let next = |k: usize| entries.get(i + k).filter(|e| !e.is_target).map(|e| e.op);
        let (op, len) = match (first.op, next(1), next(2)) {
            (Op::PushV(slot), Some(Op::Cmp(op)), Some(Op::JumpIfFalse(target))) => {
                (Op::CmpVJump { op, slot, target }, 3)
            }
            (Op::PushC(constant), Some(Op::Cmp(op)), Some(Op::JumpIfFalse(target))) => (
                Op::CmpCJump {
                    op,
                    constant,
                    target,
                },
                3,
            ),
            (Op::Cmp(op), Some(Op::JumpIfFalse(target)), _) => (Op::CmpJump { op, target }, 2),
            (Op::PushV(slot), Some(Op::IndexGet), _) => (Op::IndexGetV(slot), 2),
            (Op::PushC(constant), Some(Op::IndexGet), _) => (Op::IndexGetC(constant), 2),
            (Op::PushV(slot), Some(Op::Arith(op)), _) => (Op::ArithV { op, slot }, 2),
            (Op::PushC(constant), Some(Op::Arith(op)), _) => (Op::ArithC { op, constant }, 2),
            (op, _, _) => (op, 1),
        };
        out.push(Entry { op, ..first });
        i += len;
    }
    fuse_indexing(out)
}

/// Fusions on top of those of [`fuse`]: indexing of a local collection by locals, compare
/// and branch with locals, updates by a constant.
fn fuse_indexing(entries: Vec<Entry>) -> Vec<Entry> {
    let mut out: Vec<Entry> = Vec::with_capacity(entries.len());
    let mut i = 0;
    while i < entries.len() {
        let first = entries[i];
        let next = |k: usize| entries.get(i + k).filter(|e| !e.is_target).map(|e| e.op);
        let (op, len) = match (first.op, next(1), next(2)) {
            (Op::PushV(collection), Some(Op::IndexGetV(i)), Some(Op::IndexGetV(k))) => {
                (Op::IndexVVV { collection, i, k }, 3)
            }
            (Op::PushV(collection), Some(Op::IndexGetV(index)), _) => {
                (Op::IndexVV { collection, index }, 2)
            }
            (Op::PushV(lhs), Some(Op::CmpVJump { op, slot, target }), _) => (
                Op::CmpVVJump {
                    op,
                    lhs,
                    rhs: slot,
                    target,
                },
                2,
            ),
            (
                Op::PushV(slot),
                Some(Op::CmpCJump {
                    op,
                    constant,
                    target,
                }),
                _,
            ) => (
                Op::CmpVCJump {
                    op,
                    slot,
                    constant,
                    target,
                },
                2,
            ),
            (Op::PushC(constant), Some(Op::ArithToV { op, slot }), _) => {
                (Op::ArithToVC { op, slot, constant }, 2)
            }
            (op, _, _) => (op, 1),
        };
        out.push(Entry { op, ..first });
        i += len;
    }
    out
}

/// Longest expression considered by [`fuse_updates`].
const MAX_UPDATE_LEN: usize = 64;

/// Rewrites the code that pushes a local only to use it after a pure expression:
///
/// * `PUSH_V x; <expression>; <arith>; POP_V x` becomes `<expression>; ARITH_TO_V x`;
/// * `PUSH_V a; <index>; <value>; INDEX_SET` becomes `<index>; <value>; INDEX_SET_V a`.
///
/// The expressions may only push, read and compute values, so they cannot change the local
/// and reading it after them gives the same value as before them.
fn fuse_updates(entries: Vec<Entry>) -> Vec<Entry> {
    let mut out = Vec::with_capacity(entries.len());
    let mut i = 0;
    while i < entries.len() {
        let Op::PushV(slot) = entries[i].op else {
            out.push(entries[i]);
            i += 1;
            continue;
        };
        // The fused op and how many entries after the expression it replaces.
        let found = pure_expression(&entries[i + 1..], |k, op, depth| match op {
            Op::Arith(op) if depth == 1 && k > 0 => {
                let store = entries.get(i + 1 + k + 1)?;
                Some((store.op == Op::PopV(slot) && !store.is_target).then_some((
                    k,
                    Op::ArithToV { op, slot },
                    2,
                )))
            }
            Op::IndexSet if depth == 2 => Some(Some((k, Op::IndexSetV(slot), 1))),
            _ => None,
        });
        match found {
            Some((len, op, tail)) => {
                // `entries[i + 1..=i + len]` is the expression.
                let expression = &entries[i + 1..=i + len];
                out.push(Entry {
                    offset: entries[i].offset,
                    is_target: entries[i].is_target,
                    op: expression[0].op,
                });
                out.extend_from_slice(&expression[1..]);
                out.push(Entry {
                    offset: entries[i + len + 1].offset,
                    is_target: false,
                    op,
                });
                i += 1 + len + tail;
            }
            None => {
                out.push(entries[i]);
                i += 1;
            }
        }
    }
    out
}

/// Scans the pure expression code that follows a pushed local. `end` sees every op with the
/// number of values above the local and decides: `None` to go on, `Some(None)` to give up,
/// `Some(Some(result))` when the expression ends there.
fn pure_expression<T>(
    rest: &[Entry],
    mut end: impl FnMut(usize, Op, usize) -> Option<Option<T>>,
) -> Option<T> {
    let mut depth = 0usize;
    for (k, entry) in rest.iter().enumerate().take(MAX_UPDATE_LEN) {
        if entry.is_target {
            return None;
        }
        if let Some(result) = end(k, entry.op, depth) {
            return result;
        }
        match entry.op {
            Op::PushC(_) | Op::PushV(_) | Op::PushG(_) => depth += 1,
            Op::Neg | Op::Not if depth >= 1 => {}
            Op::IndexGet | Op::Cmp(_) | Op::Arith(_) if depth >= 2 => depth -= 1,
            _ => return None,
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entries(ops: &[Op], targets: &[usize]) -> Vec<Entry> {
        ops.iter()
            .enumerate()
            .map(|(i, &op)| Entry {
                offset: i as u32,
                is_target: targets.contains(&i),
                op,
            })
            .collect()
    }

    fn fused(ops: &[Op], targets: &[usize]) -> Vec<Op> {
        fuse(entries(ops, targets)).iter().map(|e| e.op).collect()
    }

    #[test]
    fn ops_stay_small() {
        assert!(std::mem::size_of::<Op>() <= 12);
    }

    #[test]
    fn updates_are_fused() {
        // Сумма = Сумма + А[i] * 2
        let ops = [
            Op::PushV(0),
            Op::PushV(1),
            Op::PushV(2),
            Op::IndexGet,
            Op::PushC(0),
            Op::Arith(Arith::Mul),
            Op::Arith(Arith::Add),
            Op::PopV(0),
            Op::Return,
        ];
        assert_eq!(
            fused(&ops, &[0]),
            [
                Op::IndexVV {
                    collection: 1,
                    index: 2
                },
                Op::ArithC {
                    op: Arith::Mul,
                    constant: 0
                },
                Op::ArithToV {
                    op: Arith::Add,
                    slot: 0
                },
                Op::Return,
            ]
        );
        let fused_entries = fuse(entries(&ops, &[0]));
        assert!(fused_entries[0].is_target);
        assert_eq!(fused_entries[0].offset, 0);

        // Another variable is stored: no update.
        let mut other = ops;
        other[7] = Op::PopV(3);
        assert_eq!(fused(&other, &[])[0], Op::PushV(0));
        // A jump into the expression: no update.
        assert_eq!(fused(&ops, &[4])[0], Op::PushV(0));
    }

    #[test]
    fn jump_targets_are_not_fused_away() {
        let ops = [
            Op::PushV(0),
            Op::PushV(1),
            Op::Cmp(Cmp::Lt),
            Op::JumpIfFalse(0),
        ];
        assert_eq!(
            fused(&ops, &[]),
            [Op::CmpVVJump {
                op: Cmp::Lt,
                lhs: 0,
                rhs: 1,
                target: 0
            }]
        );
        assert_eq!(
            fused(&ops, &[1]),
            [
                Op::PushV(0),
                Op::CmpVJump {
                    op: Cmp::Lt,
                    slot: 1,
                    target: 0
                }
            ]
        );
        assert_eq!(
            fused(&ops, &[3]),
            [
                Op::PushV(0),
                Op::PushV(1),
                Op::Cmp(Cmp::Lt),
                Op::JumpIfFalse(0)
            ]
        );
    }

    #[test]
    fn stores_are_fused() {
        // М[j + 1] = М[j]; j = j - 1;
        let ops = [
            Op::PushV(0),
            Op::PushV(1),
            Op::PushC(0),
            Op::Arith(Arith::Add),
            Op::PushV(0),
            Op::PushV(1),
            Op::IndexGet,
            Op::IndexSet,
            Op::PushV(1),
            Op::PushC(0),
            Op::Arith(Arith::Sub),
            Op::PopV(1),
        ];
        assert_eq!(
            fused(&ops, &[]),
            [
                Op::PushV(1),
                Op::ArithC {
                    op: Arith::Add,
                    constant: 0
                },
                Op::IndexVV {
                    collection: 0,
                    index: 1
                },
                Op::IndexSetV(0),
                Op::ArithToVC {
                    op: Arith::Sub,
                    slot: 1,
                    constant: 0
                },
            ]
        );
        // A call in the value may change the collection variable.
        let mut call = ops;
        call[6] = Op::Call {
            function: 0,
            argc: 2,
        };
        assert_eq!(fused(&call, &[])[0], Op::PushV(0));
    }
}

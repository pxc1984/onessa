use crate::lower::syntax::{child_nodes, identifier_text};
use crate::lower::{Context, MODULE_BODY_NAME};
use crate::types::Type;
use crate::{BasicBlock, BlockId, Function, Terminator};
use onessa_ast::{Lang, SyntaxKind};
use onessa_core::bytecode::{Constant, Instruction};
use rowan::SyntaxNode;
use std::collections::HashMap;

/// Where `Прервать` and `Продолжить` jump inside the innermost loop.
pub(crate) struct LoopTargets {
    pub break_to: BlockId,
    pub continue_to: BlockId,
    /// Number of active `Попытка` blocks outside the loop.
    pub try_depth: usize,
}

/// Resolved storage of a variable.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Slot {
    Local(u16),
    Global(u16),
}

pub(crate) struct FunctionBuilder<'c> {
    pub ctx: &'c mut Context,
    pub returns_value: bool,
    pub is_module_body: bool,
    locals: Vec<String>,
    /// Strict mode types of the locals; `None` until the first assignment.
    local_types: Vec<Option<Type>>,
    local_map: HashMap<String, u16>,
    blocks: Vec<BasicBlock>,
    current: BlockId,
    pub loops: Vec<LoopTargets>,
    /// Number of `Попытка` blocks the current code is nested in.
    pub try_depth: usize,
    /// Number of `Исключение` blocks the current code is nested in.
    pub handler_depth: usize,
    temp_count: usize,
    /// Join of the types of the returned values.
    pub returned: Type,
}

impl<'c> FunctionBuilder<'c> {
    fn new(ctx: &'c mut Context, returns_value: bool, is_module_body: bool) -> Self {
        Self {
            ctx,
            returns_value,
            is_module_body,
            locals: Vec::new(),
            local_types: Vec::new(),
            local_map: HashMap::new(),
            blocks: vec![BasicBlock::default()],
            current: 0,
            loops: Vec::new(),
            try_depth: 0,
            handler_depth: 0,
            temp_count: 0,
            returned: Type::Never,
        }
    }

    /// Lowers a method; also returns the type of its result.
    pub fn lower_method(
        ctx: &mut Context,
        index: usize,
        node: &SyntaxNode<Lang>,
    ) -> (Function, Type) {
        let signature = &ctx.signatures[index];
        let returns_value = signature.returns_value;
        let param_names: Vec<String> = signature.params.iter().map(|p| p.name.clone()).collect();

        let mut builder = FunctionBuilder::new(ctx, returns_value, false);
        for name in &param_names {
            if let Some(slot) = builder.declare_local(node, name) {
                // Callers may pass anything.
                builder.local_types[slot as usize] = Some(Type::Any);
            }
        }
        if let Some(block) = child_nodes(node).find(|n| n.kind() == SyntaxKind::Block) {
            builder.lower_block(&block);
        }
        let end_line = builder.ctx.position(node.text_range().end()).0;
        let (mut function, returned) = builder.finish(end_line);

        // `builder` borrowed the context until here.
        let signature = &ctx.signatures[index];
        function.name = signature.name.clone();
        function.export = signature.export;
        function.params = signature.params.clone();
        (function, returned)
    }

    pub fn lower_module_body(ctx: &'c mut Context, statements: &[SyntaxNode<Lang>]) -> Function {
        let mut builder = Self::new(ctx, false, true);
        for statement in statements {
            builder.lower_statement(statement);
        }
        let end_line = builder.ctx.position(rowan::TextSize::from(u32::MAX)).0;
        let (mut function, _) = builder.finish(end_line);
        function.name = MODULE_BODY_NAME.to_string();
        function
    }

    /// Adds the implicit `Возврат` at the end of the method body.
    fn finish(mut self, line: u32) -> (Function, Type) {
        let end = self.current;
        self.emit(Instruction::PushC(Constant::Undefined), line);
        self.terminate(Terminator::Return, line);
        let function = Function {
            name: String::new(),
            returns_value: self.returns_value,
            export: false,
            params: Vec::new(),
            locals: self.locals,
            blocks: self.blocks,
        };
        // A function that can reach its end returns `Неопределено` there.
        let mut returned = self.returned;
        if function.reachable_blocks().contains(&end) {
            returned = returned.join(Type::UNDEFINED);
        }
        (function, returned)
    }

    // ===== Blocks =====

    pub fn new_block(&mut self) -> BlockId {
        self.blocks.push(BasicBlock::default());
        self.blocks.len() - 1
    }

    pub fn switch_to(&mut self, block: BlockId) {
        self.current = block;
    }

    pub fn emit(&mut self, instruction: Instruction<BlockId>, line: u32) {
        self.blocks[self.current]
            .instructions
            .push((instruction, line));
    }

    /// Ends the current block. Code emitted afterwards lands in a fresh, unreachable block.
    pub fn terminate(&mut self, terminator: Terminator, line: u32) {
        let block = &mut self.blocks[self.current];
        debug_assert!(block.terminator.is_none(), "block terminated twice");
        block.terminator = Some((terminator, line));
        self.current = self.new_block();
    }

    // ===== Variables =====

    /// Declares a local variable, reporting duplicates.
    pub fn declare_local(&mut self, node: &SyntaxNode<Lang>, name: &str) -> Option<u16> {
        let key = name.to_lowercase();
        if self.local_map.contains_key(&key) {
            self.ctx
                .error(node, format!("Variable is already defined ({name})"));
            return None;
        }
        self.add_local(key, name.to_string(), node)
    }

    fn add_local(&mut self, key: String, name: String, node: &SyntaxNode<Lang>) -> Option<u16> {
        if self.locals.len() >= u16::MAX as usize {
            self.ctx.error(node, "Too many local variables");
            return None;
        }
        let slot = self.locals.len() as u16;
        self.locals.push(name);
        self.local_types.push(None);
        self.local_map.insert(key, slot);
        Some(slot)
    }

    /// Allocates an unnamed local, e.g. for the upper bound of a `Для` loop.
    pub fn temp_local(&mut self, node: &SyntaxNode<Lang>) -> Option<u16> {
        let name = format!("${}", self.temp_count);
        self.temp_count += 1;
        self.add_local(name.clone(), name, node)
    }

    pub fn resolve(&self, name: &str) -> Option<Slot> {
        let key = name.to_lowercase();
        if let Some(&slot) = self.local_map.get(&key) {
            return Some(Slot::Local(slot));
        }
        self.ctx
            .global_map
            .get(&key)
            .map(|&slot| Slot::Global(slot))
    }

    /// Resolves the target of an assignment, implicitly declaring a local if the name is new.
    pub fn resolve_or_declare(&mut self, identifier: &SyntaxNode<Lang>) -> Option<Slot> {
        let name = identifier_text(identifier);
        if let Some(slot) = self.resolve(&name) {
            return Some(slot);
        }
        self.add_local(name.to_lowercase(), name, identifier)
            .map(Slot::Local)
    }

    pub fn emit_load(&mut self, slot: Slot, line: u32) {
        match slot {
            Slot::Local(i) => self.emit(Instruction::PushV(i), line),
            Slot::Global(i) => self.emit(Instruction::PushG(i), line),
        }
    }

    /// Strict mode type of a variable; `Произвольный` outside strict mode.
    pub fn slot_type(&self, slot: Slot) -> Type {
        if !self.ctx.strict {
            return Type::Any;
        }
        let known = match slot {
            Slot::Local(i) => self.local_types[i as usize],
            Slot::Global(i) => self.ctx.global_types[i as usize],
        };
        known.unwrap_or(Type::Any)
    }

    /// Stores the value on top of the stack, of type `value`, into a variable. In strict mode
    /// the first assignment fixes the type of the variable and later ones must match it:
    /// a value of another known type is an error, one of an unknown type is checked at run
    /// time.
    pub fn store(&mut self, slot: Slot, value: Type, node: &SyntaxNode<Lang>, line: u32) {
        if self.ctx.strict {
            let variable = match slot {
                Slot::Local(i) => &mut self.local_types[i as usize],
                Slot::Global(i) => &mut self.ctx.global_types[i as usize],
            };
            match (*variable, value) {
                (None, value) => *variable = value.of_new_variable(),
                (Some(Type::Any), _) | (Some(_), Type::Never) => {}
                (Some(Type::Known(expected)), Type::Any) => {
                    self.emit(Instruction::CheckType(expected), line);
                }
                (Some(expected), value) if expected != value => {
                    let name = match slot {
                        Slot::Local(i) => self.locals[i as usize].clone(),
                        Slot::Global(i) => self.ctx.globals[i as usize].clone(),
                    };
                    self.ctx.error(
                        node,
                        format!(
                            "Type mismatch: variable {name} is {expected}, cannot assign {value}"
                        ),
                    );
                }
                _ => {}
            }
        }
        self.emit_store(slot, line);
    }

    pub fn emit_store(&mut self, slot: Slot, line: u32) {
        match slot {
            Slot::Local(i) => self.emit(Instruction::PopV(i), line),
            Slot::Global(i) => self.emit(Instruction::PopG(i), line),
        }
    }
}

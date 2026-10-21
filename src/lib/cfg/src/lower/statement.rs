use crate::lower::function::{FunctionBuilder, LoopTargets, Slot};
use crate::lower::syntax::{child_nodes, identifier_text};
use crate::types::Type;
use crate::Terminator;
use onessa_ast::{Lang, SyntaxKind};
use onessa_core::bytecode::{Constant, Instruction, ValueType};
use rowan::SyntaxNode;

impl FunctionBuilder<'_> {
    pub fn lower_block(&mut self, block: &SyntaxNode<Lang>) {
        for statement in child_nodes(block) {
            self.lower_statement(&statement);
        }
    }

    pub fn lower_statement(&mut self, node: &SyntaxNode<Lang>) {
        match node.kind() {
            SyntaxKind::AssignmentStatement => self.lower_assignment(node),
            SyntaxKind::ExpressionStatement => self.lower_expression_statement(node),
            SyntaxKind::VariableDeclaration => self.lower_local_declaration(node),
            SyntaxKind::IfStatement => self.lower_if(node),
            SyntaxKind::WhileStatement => self.lower_while(node),
            SyntaxKind::ForStatement => self.lower_for(node),
            SyntaxKind::TryStatement => self.lower_try(node),
            SyntaxKind::RaiseStatement => self.lower_raise(node),
            SyntaxKind::ReturnStatement => self.lower_return(node),
            SyntaxKind::BreakStatement => self.lower_loop_jump(node, true),
            SyntaxKind::ContinueStatement => self.lower_loop_jump(node, false),
            SyntaxKind::ForEachStatement => self.ctx.error(
                node,
                "'Для Каждого' is not supported yet: collections are not implemented",
            ),
            SyntaxKind::ProcedureDeclaration | SyntaxKind::FunctionDeclaration => self.ctx.error(
                node,
                "Procedures and functions must be declared at module level",
            ),
            kind => self
                .ctx
                .error(node, format!("Unexpected statement {kind:?}")),
        }
    }

    fn lower_assignment(&mut self, node: &SyntaxNode<Lang>) {
        let mut children = child_nodes(node);
        let (Some(target), Some(value)) = (children.next(), children.next()) else {
            return;
        };
        let line = self.ctx.line(node);
        match target.kind() {
            SyntaxKind::Identifier => {}
            SyntaxKind::IndexExpression => {
                if self.lower_index_operands(&target) {
                    self.lower_expression(&value);
                    self.emit(Instruction::IndexSet, line);
                }
                return;
            }
            _ => {
                self.ctx.error(
                    &target,
                    "Only variables and indexed elements can be assigned to: object properties are not supported yet",
                );
                return;
            }
        }

        let value_type = self.lower_expression(&value);
        // The target is declared after the value is computed, so `А = А + 1` with an
        // undefined `А` is still reported.
        if let Some(slot) = self.resolve_or_declare(&target) {
            self.store(slot, value_type, &target, line);
        }
    }

    fn lower_expression_statement(&mut self, node: &SyntaxNode<Lang>) {
        let Some(expression) = child_nodes(node).next() else {
            return;
        };
        if expression.kind() == SyntaxKind::CallExpression {
            self.lower_call(&expression, false);
        } else {
            self.ctx
                .error(&expression, "Expected a procedure call or an assignment");
        }
    }

    fn lower_local_declaration(&mut self, node: &SyntaxNode<Lang>) {
        for identifier in child_nodes(node).filter(|n| n.kind() == SyntaxKind::Identifier) {
            let name = identifier_text(&identifier);
            if let Some(slot) = self.declare_local(&identifier, &name) {
                // Re-entering the block (in a loop) must reset the variable to `Неопределено`.
                let line = self.ctx.line(&identifier);
                self.emit(Instruction::PushC(Constant::Undefined), line);
                self.emit(Instruction::PopV(slot), line);
            }
        }
    }

    fn lower_if(&mut self, node: &SyntaxNode<Lang>) {
        let end = self.new_block();
        let mut children = child_nodes(node).peekable();

        // `Если` condition and block, then every `ИначеЕсли` clause.
        let mut branches = Vec::new();
        if let (Some(condition), Some(block)) = (children.next(), children.next()) {
            branches.push((condition, block));
        }
        let mut else_block = None;
        for clause in children {
            match clause.kind() {
                SyntaxKind::ElseIfClause => {
                    let mut parts = child_nodes(&clause);
                    if let (Some(condition), Some(block)) = (parts.next(), parts.next()) {
                        branches.push((condition, block));
                    }
                }
                SyntaxKind::ElseClause => else_block = child_nodes(&clause).next(),
                _ => {}
            }
        }

        for (condition, block) in branches {
            let line = self.ctx.line(&condition);
            self.lower_condition(&condition);
            let then = self.new_block();
            let otherwise = self.new_block();
            self.terminate(Terminator::Branch { then, otherwise }, line);

            self.switch_to(then);
            self.lower_block(&block);
            let line = self.ctx.line(&block);
            self.terminate(Terminator::Jump(end), line);
            self.switch_to(otherwise);
        }

        if let Some(block) = else_block {
            self.lower_block(&block);
        }
        let line = self.ctx.line(node);
        self.terminate(Terminator::Jump(end), line);
        self.switch_to(end);
    }

    fn lower_while(&mut self, node: &SyntaxNode<Lang>) {
        let mut children = child_nodes(node);
        let (Some(condition), Some(block)) = (children.next(), children.next()) else {
            return;
        };
        let line = self.ctx.line(node);
        let header = self.new_block();
        let body = self.new_block();
        let exit = self.new_block();

        self.terminate(Terminator::Jump(header), line);
        self.switch_to(header);
        self.lower_condition(&condition);
        self.terminate(
            Terminator::Branch {
                then: body,
                otherwise: exit,
            },
            line,
        );

        self.switch_to(body);
        self.lower_loop_body(&block, exit, header);
        self.terminate(Terminator::Jump(header), line);
        self.switch_to(exit);
    }

    /// `Для Сч = Начало По Конец Цикл`: the bound is evaluated once, the counter is checked
    /// before every iteration and incremented by one after it.
    fn lower_for(&mut self, node: &SyntaxNode<Lang>) {
        let mut children = child_nodes(node);
        let (Some(variable), Some(start), Some(bound), Some(block)) = (
            children.next(),
            children.next(),
            children.next(),
            children.next(),
        ) else {
            return;
        };
        let line = self.ctx.line(node);

        let start_type = self.lower_expression(&start);
        self.check_loop_bound(&start, start_type, line);
        let Some(counter) = self.resolve_or_declare(&variable) else {
            return;
        };
        self.store(counter, Type::NUMBER, &variable, line);
        let bound_type = self.lower_expression(&bound);
        self.check_loop_bound(&bound, bound_type, line);
        let Some(limit) = self.temp_local(node) else {
            return;
        };
        self.emit(Instruction::PopV(limit), line);

        let header = self.new_block();
        let body = self.new_block();
        let increment = self.new_block();
        let exit = self.new_block();

        self.terminate(Terminator::Jump(header), line);
        self.switch_to(header);
        match counter {
            Slot::Local(counter) => self.terminate(
                Terminator::ForCheck {
                    counter,
                    limit,
                    body,
                    exit,
                },
                line,
            ),
            Slot::Global(_) => {
                self.emit_load(counter, line);
                self.emit(Instruction::PushV(limit), line);
                self.emit(Instruction::Le, line);
                self.terminate(
                    Terminator::Branch {
                        then: body,
                        otherwise: exit,
                    },
                    line,
                );
            }
        }

        self.switch_to(body);
        self.lower_loop_body(&block, exit, increment);
        self.terminate(Terminator::Jump(increment), line);

        self.switch_to(increment);
        match counter {
            Slot::Local(counter) => self.terminate(
                Terminator::ForStep {
                    counter,
                    limit,
                    body,
                    exit,
                },
                line,
            ),
            Slot::Global(_) => {
                self.emit_load(counter, line);
                self.emit(
                    Instruction::PushC(Constant::Number(rust_decimal::Decimal::ONE)),
                    line,
                );
                self.emit(Instruction::Add, line);
                self.emit_store(counter, line);
                self.terminate(Terminator::Jump(header), line);
            }
        }
        self.switch_to(exit);
    }

    /// In strict mode the start and the end of `Для` must be numbers.
    fn check_loop_bound(&mut self, node: &SyntaxNode<Lang>, bound: Type, line: u32) {
        if !self.ctx.strict {
            return;
        }
        match bound {
            Type::Known(ValueType::Number) | Type::Never => {}
            Type::Any => self.emit(Instruction::CheckType(ValueType::Number), line),
            other => self
                .ctx
                .error(node, format!("'Для' bounds must be Число, not {other}")),
        }
    }

    fn lower_loop_body(&mut self, block: &SyntaxNode<Lang>, break_to: usize, continue_to: usize) {
        self.loops.push(LoopTargets {
            break_to,
            continue_to,
            try_depth: self.try_depth,
        });
        self.lower_block(block);
        self.loops.pop();
    }

    fn lower_loop_jump(&mut self, node: &SyntaxNode<Lang>, is_break: bool) {
        let Some(targets) = self.loops.last() else {
            let keyword = if is_break {
                "Прервать"
            } else {
                "Продолжить"
            };
            self.ctx
                .error(node, format!("'{keyword}' can only be used inside a loop"));
            return;
        };
        let target = if is_break {
            targets.break_to
        } else {
            targets.continue_to
        };
        let line = self.ctx.line(node);
        // Leaving a `Попытка` block must uninstall its handler.
        for _ in targets.try_depth..self.try_depth {
            self.emit(Instruction::TryEnd, line);
        }
        self.terminate(Terminator::Jump(target), line);
    }

    fn lower_try(&mut self, node: &SyntaxNode<Lang>) {
        let mut blocks = child_nodes(node).filter(|n| n.kind() == SyntaxKind::Block);
        let (Some(body), handler_body) = (blocks.next(), blocks.next()) else {
            return;
        };
        let line = self.ctx.line(node);
        let handler = self.new_block();
        let after = self.new_block();

        self.emit(Instruction::TryBegin(handler), line);
        self.try_depth += 1;
        self.lower_block(&body);
        self.try_depth -= 1;
        self.emit(Instruction::TryEnd, line);
        self.terminate(Terminator::Jump(after), line);

        self.switch_to(handler);
        if let Some(handler_body) = handler_body {
            self.handler_depth += 1;
            self.lower_block(&handler_body);
            self.handler_depth -= 1;
        }
        self.terminate(Terminator::Jump(after), line);
        self.switch_to(after);
    }

    fn lower_raise(&mut self, node: &SyntaxNode<Lang>) {
        let line = self.ctx.line(node);
        match child_nodes(node).next() {
            Some(message) => {
                self.lower_expression(&message);
                self.terminate(Terminator::Raise, line);
            }
            None if self.handler_depth > 0 => self.terminate(Terminator::ReRaise, line),
            None => self.ctx.error(
                node,
                "'ВызватьИсключение' without a message is only allowed inside 'Исключение'",
            ),
        }
    }

    fn lower_return(&mut self, node: &SyntaxNode<Lang>) {
        let line = self.ctx.line(node);
        let value = child_nodes(node).next();
        if self.is_module_body {
            self.ctx
                .error(node, "'Возврат' is not allowed in the module body");
            return;
        }
        match (self.returns_value, value) {
            (true, Some(value)) => {
                let value_type = self.lower_expression(&value);
                self.returned = self.returned.join(value_type);
            }
            (true, None) => {
                self.ctx.error(node, "Function must return a value");
                return;
            }
            (false, Some(value)) => {
                self.ctx.error(&value, "Procedure cannot return a value");
                return;
            }
            (false, None) => self.emit(Instruction::PushC(Constant::Undefined), line),
        }
        self.terminate(Terminator::Return, line);
    }
}

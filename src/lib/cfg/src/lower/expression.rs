use crate::lower::function::{FunctionBuilder, Slot};
use crate::lower::literal;
use crate::lower::syntax::{child_nodes, child_tokens, first_token_kind, identifier_text};
use crate::Terminator;
use onessa_ast::{Lang, SyntaxKind};
use onessa_core::builtins;
use onessa_core::bytecode::{Constant, Instruction};
use rowan::SyntaxNode;

impl FunctionBuilder<'_> {
    /// Emits code that pushes the value of the expression onto the stack.
    pub fn lower_expression(&mut self, node: &SyntaxNode<Lang>) {
        let line = self.ctx.line(node);
        match node.kind() {
            SyntaxKind::Expression | SyntaxKind::ParenExpression => {
                match child_nodes(node).next() {
                    Some(inner) => self.lower_expression(&inner),
                    None => self.ctx.error(node, "Expected an expression"),
                }
            }
            SyntaxKind::Literal => {
                if let Some(constant) = literal::literal(self.ctx, node) {
                    self.emit(Instruction::PushC(constant), line);
                }
            }
            SyntaxKind::Identifier => {
                let name = identifier_text(node);
                match self.resolve(&name) {
                    Some(slot) => self.emit_load(slot, line),
                    None => self
                        .ctx
                        .error(node, format!("Variable is not defined ({name})")),
                }
            }
            SyntaxKind::UnaryExpression => self.lower_unary(node, line),
            SyntaxKind::BinaryExpression => self.lower_binary(node, line),
            SyntaxKind::TernaryExpression => self.lower_ternary(node, line),
            SyntaxKind::CallExpression => self.lower_call(node, true),
            SyntaxKind::MemberExpression => self
                .ctx
                .error(node, "Object properties are not supported yet"),
            SyntaxKind::IndexExpression => {
                if self.lower_index_operands(node) {
                    self.emit(Instruction::IndexGet, line);
                }
            }
            SyntaxKind::NewExpression => self.lower_new(node, line),
            kind => self
                .ctx
                .error(node, format!("Unexpected expression {kind:?}")),
        }
    }

    fn lower_unary(&mut self, node: &SyntaxNode<Lang>, line: u32) {
        let Some(operand) = child_nodes(node).next() else {
            return;
        };
        self.lower_expression(&operand);
        match first_token_kind(node) {
            Some(SyntaxKind::Minus) => self.emit(Instruction::Neg, line),
            Some(SyntaxKind::Not) => self.emit(Instruction::Not, line),
            // Unary plus converts nothing in v0: arithmetic operands must already be numbers.
            _ => {}
        }
    }

    fn lower_binary(&mut self, node: &SyntaxNode<Lang>, line: u32) {
        let mut operands = child_nodes(node);
        let (Some(lhs), Some(rhs)) = (operands.next(), operands.next()) else {
            return;
        };
        let Some(operator) = child_tokens(node)
            .map(|t| t.kind())
            .find(|k| is_operator(*k))
        else {
            return;
        };

        match operator {
            SyntaxKind::And | SyntaxKind::Or => {
                self.lower_short_circuit(&lhs, &rhs, operator == SyntaxKind::And, line);
                return;
            }
            _ => {}
        }

        self.lower_expression(&lhs);
        self.lower_expression(&rhs);
        let instruction = match operator {
            SyntaxKind::Plus => Instruction::Add,
            SyntaxKind::Minus => Instruction::Sub,
            SyntaxKind::Star => Instruction::Mul,
            SyntaxKind::Slash => Instruction::Div,
            SyntaxKind::Percent => Instruction::Mod,
            SyntaxKind::Eq => Instruction::Eq,
            SyntaxKind::Ne => Instruction::Ne,
            SyntaxKind::Lt => Instruction::Lt,
            SyntaxKind::Le => Instruction::Le,
            SyntaxKind::Gt => Instruction::Gt,
            SyntaxKind::Ge => Instruction::Ge,
            _ => unreachable!("is_operator covers every binary operator"),
        };
        self.emit(instruction, line);
    }

    /// `А И Б` evaluates `Б` only if `А` is `Истина`; `А ИЛИ Б` only if `А` is `Ложь`.
    /// The value of `А` is kept on the stack when `Б` is skipped.
    fn lower_short_circuit(
        &mut self,
        lhs: &SyntaxNode<Lang>,
        rhs: &SyntaxNode<Lang>,
        is_and: bool,
        line: u32,
    ) {
        let evaluate_rhs = self.new_block();
        let end = self.new_block();

        self.lower_expression(lhs);
        self.emit(Instruction::Dup, line);
        let terminator = if is_and {
            Terminator::Branch {
                then: evaluate_rhs,
                otherwise: end,
            }
        } else {
            Terminator::Branch {
                then: end,
                otherwise: evaluate_rhs,
            }
        };
        self.terminate(terminator, line);

        self.switch_to(evaluate_rhs);
        self.emit(Instruction::Pop, line);
        self.lower_expression(rhs);
        self.terminate(Terminator::Jump(end), line);
        self.switch_to(end);
    }

    fn lower_ternary(&mut self, node: &SyntaxNode<Lang>, line: u32) {
        let mut parts = child_nodes(node);
        let (Some(condition), Some(if_true), Some(if_false)) =
            (parts.next(), parts.next(), parts.next())
        else {
            return;
        };
        let then = self.new_block();
        let otherwise = self.new_block();
        let end = self.new_block();

        self.lower_expression(&condition);
        self.terminate(Terminator::Branch { then, otherwise }, line);
        self.switch_to(then);
        self.lower_expression(&if_true);
        self.terminate(Terminator::Jump(end), line);
        self.switch_to(otherwise);
        self.lower_expression(&if_false);
        self.terminate(Terminator::Jump(end), line);
        self.switch_to(end);
    }

    /// Pushes the collection and the index of `collection[index]`. Returns `false` if the
    /// node is incomplete.
    pub fn lower_index_operands(&mut self, node: &SyntaxNode<Lang>) -> bool {
        let mut children = child_nodes(node);
        let (Some(collection), Some(index)) = (children.next(), children.next()) else {
            return false;
        };
        self.lower_expression(&collection);
        self.lower_expression(&index);
        true
    }

    /// `Новый Тип` or `Новый Тип(аргументы)`.
    fn lower_new(&mut self, node: &SyntaxNode<Lang>, line: u32) {
        let Some(target) = child_nodes(node).next() else {
            return;
        };
        let (type_node, args) = match target.kind() {
            SyntaxKind::Identifier => (target, Vec::new()),
            SyntaxKind::CallExpression => {
                let mut children = child_nodes(&target);
                let Some(callee) = children.next() else {
                    return;
                };
                (callee, call_arguments(children))
            }
            _ => (target, Vec::new()),
        };
        if type_node.kind() != SyntaxKind::Identifier {
            self.ctx
                .error(&type_node, "Expected a type name after 'Новый'");
            return;
        }
        let Some(argc) = self.lower_arguments(node, &args, line) else {
            return;
        };
        let type_name = self.ctx.strings.push(&identifier_text(&type_node));
        self.emit(Instruction::New { type_name, argc }, line);
    }

    /// Pushes the arguments, skipped ones as `Неопределено`. Returns their count.
    fn lower_arguments(
        &mut self,
        node: &SyntaxNode<Lang>,
        args: &[Option<SyntaxNode<Lang>>],
        line: u32,
    ) -> Option<u8> {
        let Ok(argc) = u8::try_from(args.len()) else {
            self.ctx.error(node, "Too many actual parameters");
            return None;
        };
        for arg in args {
            match arg {
                Some(arg) => self.lower_expression(arg),
                None => self.emit(Instruction::PushC(Constant::Undefined), line),
            }
        }
        Some(argc)
    }

    /// Lowers a call. With `want_value` the result is left on the stack, otherwise it is
    /// discarded.
    pub fn lower_call(&mut self, node: &SyntaxNode<Lang>, want_value: bool) {
        let line = self.ctx.line(node);
        let mut children = child_nodes(node);
        let Some(callee) = children.next() else {
            return;
        };
        let args = call_arguments(children);
        if callee.kind() == SyntaxKind::MemberExpression {
            self.lower_method_call_on_object(node, &callee, &args, want_value, line);
            return;
        }
        if callee.kind() != SyntaxKind::Identifier {
            self.ctx
                .error(&callee, "Expected a procedure or function name");
            return;
        }
        let name = identifier_text(&callee);

        if let Some(&index) = self.ctx.signature_map.get(&name.to_lowercase()) {
            self.lower_method_call(node, index, &name, &args, want_value, line);
        } else if let Some(builtin) = builtins::lookup(&name) {
            self.lower_builtin_call(node, builtin, &args, want_value, line);
        } else {
            self.ctx.error(
                &callee,
                format!("Procedure or function is not defined ({name})"),
            );
        }
    }

    /// `Объект.Метод(аргументы)`: the method is looked up at run time, so procedures and
    /// functions cannot be told apart here.
    fn lower_method_call_on_object(
        &mut self,
        node: &SyntaxNode<Lang>,
        callee: &SyntaxNode<Lang>,
        args: &[Option<SyntaxNode<Lang>>],
        want_value: bool,
        line: u32,
    ) {
        let mut parts = child_nodes(callee);
        let (Some(object), Some(method)) = (parts.next(), parts.next()) else {
            return;
        };
        self.lower_expression(&object);
        let Some(argc) = self.lower_arguments(node, args, line) else {
            return;
        };
        let name = self.ctx.strings.push(&identifier_text(&method));
        self.emit(Instruction::CallMethod { name, argc }, line);
        if !want_value {
            self.emit(Instruction::Pop, line);
        }
    }

    fn lower_method_call(
        &mut self,
        node: &SyntaxNode<Lang>,
        index: usize,
        name: &str,
        args: &[Option<SyntaxNode<Lang>>],
        want_value: bool,
        line: u32,
    ) {
        let signature = &self.ctx.signatures[index];
        let returns_value = signature.returns_value;
        let params: Vec<_> = signature
            .params
            .iter()
            .map(|p| (p.by_value, p.default))
            .collect();

        if want_value && !returns_value {
            self.ctx.error(
                node,
                format!("Procedure cannot be used as a function ({name})"),
            );
            return;
        }
        if args.len() > params.len() {
            self.ctx
                .error(node, format!("Too many actual parameters ({name})"));
            return;
        }

        // Arguments that are plain variables passed to non-`Знач` parameters get the final
        // parameter value written back after the call.
        let mut write_back = Vec::new();
        for (position, &(by_value, default)) in params.iter().enumerate() {
            match args.get(position) {
                Some(Some(arg)) => {
                    self.lower_expression(arg);
                    if !by_value {
                        if let Some(slot) = self.variable_argument(arg) {
                            write_back.push((position as u8, slot));
                        }
                    }
                }
                Some(None) => {
                    let value = default.unwrap_or(Constant::Undefined);
                    self.emit(Instruction::PushC(value), line);
                }
                None => match default {
                    Some(value) => self.emit(Instruction::PushC(value), line),
                    None => {
                        self.ctx
                            .error(node, format!("Not enough actual parameters ({name})"));
                        return;
                    }
                },
            }
        }

        self.emit(
            Instruction::Call {
                function: index as u32,
                argc: params.len() as u8,
            },
            line,
        );
        for (position, slot) in write_back {
            self.emit(Instruction::PushOut(position), line);
            self.emit_store(slot, line);
        }
        if !want_value {
            self.emit(Instruction::Pop, line);
        }
    }

    /// The variable an argument expression names directly, if any.
    fn variable_argument(&self, arg: &SyntaxNode<Lang>) -> Option<Slot> {
        let inner = child_nodes(arg).next()?;
        if inner.kind() != SyntaxKind::Identifier {
            return None;
        }
        self.resolve(&identifier_text(&inner))
    }

    fn lower_builtin_call(
        &mut self,
        node: &SyntaxNode<Lang>,
        builtin: &builtins::Builtin,
        args: &[Option<SyntaxNode<Lang>>],
        want_value: bool,
        line: u32,
    ) {
        if want_value && !builtin.returns_value {
            self.ctx.error(
                node,
                format!("Procedure cannot be used as a function ({})", builtin.name),
            );
            return;
        }
        if args.len() < builtin.min_args as usize {
            self.ctx.error(
                node,
                format!("Not enough actual parameters ({})", builtin.name),
            );
            return;
        }
        if args.len() > builtin.max_args as usize {
            self.ctx.error(
                node,
                format!("Too many actual parameters ({})", builtin.name),
            );
            return;
        }

        let Some(argc) = self.lower_arguments(node, args, line) else {
            return;
        };
        let name = self.ctx.strings.push(builtin.name);
        self.emit(Instruction::CallNative { name, argc }, line);
        if !want_value {
            self.emit(Instruction::Pop, line);
        }
    }
}

/// Argument nodes of a call; each is an `Expression`, `None` if the argument was skipped.
fn call_arguments(nodes: impl Iterator<Item = SyntaxNode<Lang>>) -> Vec<Option<SyntaxNode<Lang>>> {
    nodes
        .map(|arg| child_nodes(&arg).next().map(|_| arg))
        .collect()
}

fn is_operator(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::Plus
            | SyntaxKind::Minus
            | SyntaxKind::Star
            | SyntaxKind::Slash
            | SyntaxKind::Percent
            | SyntaxKind::Eq
            | SyntaxKind::Ne
            | SyntaxKind::Lt
            | SyntaxKind::Le
            | SyntaxKind::Gt
            | SyntaxKind::Ge
            | SyntaxKind::And
            | SyntaxKind::Or
    )
}

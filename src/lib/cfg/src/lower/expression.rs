use crate::lower::function::{FunctionBuilder, Slot};
use crate::lower::literal;
use crate::lower::syntax::{child_nodes, child_tokens, first_token_kind, identifier_text};
use crate::types::{self, Type};
use crate::Terminator;
use onessa_ast::{Lang, SyntaxKind};
use onessa_core::builtins::{self, Returns};
use onessa_core::bytecode::{Constant, Instruction, ValueType};
use rowan::SyntaxNode;

impl FunctionBuilder<'_> {
    /// Emits code that pushes the value of the expression onto the stack and returns its
    /// static type.
    pub fn lower_expression(&mut self, node: &SyntaxNode<Lang>) -> Type {
        let line = self.ctx.line(node);
        match node.kind() {
            SyntaxKind::Expression | SyntaxKind::ParenExpression => {
                match child_nodes(node).next() {
                    Some(inner) => self.lower_expression(&inner),
                    None => {
                        self.ctx.error(node, "Expected an expression");
                        Type::Any
                    }
                }
            }
            SyntaxKind::Literal => match literal::literal(self.ctx, node) {
                Some(constant) => {
                    self.emit(Instruction::PushC(constant), line);
                    constant_type(constant)
                }
                None => Type::Any,
            },
            SyntaxKind::Identifier => {
                let name = identifier_text(node);
                match self.resolve(&name) {
                    Some(slot) => {
                        self.emit_load(slot, line);
                        self.slot_type(slot)
                    }
                    None => {
                        self.ctx
                            .error(node, format!("Variable is not defined ({name})"));
                        Type::Any
                    }
                }
            }
            SyntaxKind::UnaryExpression => self.lower_unary(node, line),
            SyntaxKind::BinaryExpression => self.lower_binary(node, line),
            SyntaxKind::TernaryExpression => self.lower_ternary(node, line),
            SyntaxKind::CallExpression => self.lower_call(node, true),
            SyntaxKind::MemberExpression => {
                self.ctx
                    .error(node, "Object properties are not supported yet");
                Type::Any
            }
            SyntaxKind::IndexExpression => {
                if self.lower_index_operands(node) {
                    self.emit(Instruction::IndexGet, line);
                }
                Type::Any
            }
            SyntaxKind::NewExpression => self.lower_new(node, line),
            kind => {
                self.ctx
                    .error(node, format!("Unexpected expression {kind:?}"));
                Type::Any
            }
        }
    }

    /// Reports a type error in strict mode; outside it the error surfaces at run time.
    pub fn type_error(&mut self, node: &SyntaxNode<Lang>, result: Result<(), String>) {
        if let (true, Err(message)) = (self.ctx.strict, result) {
            self.ctx.error(node, message);
        }
    }

    /// The result of a typing rule: its type, or `Произвольный` after reporting the error.
    fn typed(&mut self, node: &SyntaxNode<Lang>, result: Result<Type, String>) -> Type {
        match result {
            Ok(t) => t,
            Err(message) => {
                self.type_error(node, Err(message));
                Type::Any
            }
        }
    }

    /// Lowers a condition of `Если`, `Пока` or `?(,,)`.
    pub fn lower_condition(&mut self, node: &SyntaxNode<Lang>) {
        let condition = self.lower_expression(node);
        self.type_error(node, types::condition(condition));
    }

    fn lower_unary(&mut self, node: &SyntaxNode<Lang>, line: u32) -> Type {
        let Some(operand) = child_nodes(node).next() else {
            return Type::Any;
        };
        let operand_type = self.lower_expression(&operand);
        let operator = first_token_kind(node).unwrap_or(SyntaxKind::Plus);
        match operator {
            SyntaxKind::Minus => self.emit(Instruction::Neg, line),
            SyntaxKind::Not => self.emit(Instruction::Not, line),
            // Unary plus converts nothing in v0: arithmetic operands must already be numbers.
            _ => {}
        }
        self.typed(node, types::unary(operator, operand_type))
    }

    fn lower_binary(&mut self, node: &SyntaxNode<Lang>, line: u32) -> Type {
        let mut operands = child_nodes(node);
        let (Some(lhs), Some(rhs)) = (operands.next(), operands.next()) else {
            return Type::Any;
        };
        let Some(operator) = child_tokens(node)
            .map(|t| t.kind())
            .find(|k| is_operator(*k))
        else {
            return Type::Any;
        };

        if let SyntaxKind::And | SyntaxKind::Or = operator {
            return self.lower_short_circuit(&lhs, &rhs, operator == SyntaxKind::And, line);
        }

        let lhs_type = self.lower_expression(&lhs);
        let rhs_type = self.lower_expression(&rhs);
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
        self.typed(node, types::binary(operator, lhs_type, rhs_type))
    }

    /// `А И Б` evaluates `Б` only if `А` is `Истина`; `А ИЛИ Б` only if `А` is `Ложь`.
    /// The value of `А` is kept on the stack when `Б` is skipped.
    fn lower_short_circuit(
        &mut self,
        lhs: &SyntaxNode<Lang>,
        rhs: &SyntaxNode<Lang>,
        is_and: bool,
        line: u32,
    ) -> Type {
        let evaluate_rhs = self.new_block();
        let end = self.new_block();

        let lhs_type = self.lower_expression(lhs);
        self.type_error(lhs, types::condition(lhs_type));
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
        let rhs_type = self.lower_expression(rhs);
        self.type_error(rhs, types::condition(rhs_type));
        self.terminate(Terminator::Jump(end), line);
        self.switch_to(end);
        lhs_type.join(rhs_type)
    }

    fn lower_ternary(&mut self, node: &SyntaxNode<Lang>, line: u32) -> Type {
        let mut parts = child_nodes(node);
        let (Some(condition), Some(if_true), Some(if_false)) =
            (parts.next(), parts.next(), parts.next())
        else {
            return Type::Any;
        };
        let then = self.new_block();
        let otherwise = self.new_block();
        let end = self.new_block();

        self.lower_condition(&condition);
        self.terminate(Terminator::Branch { then, otherwise }, line);
        self.switch_to(then);
        let true_type = self.lower_expression(&if_true);
        self.terminate(Terminator::Jump(end), line);
        self.switch_to(otherwise);
        let false_type = self.lower_expression(&if_false);
        self.terminate(Terminator::Jump(end), line);
        self.switch_to(end);
        true_type.join(false_type)
    }

    /// Pushes the collection and the index of `collection[index]`. Returns `false` if the
    /// node is incomplete.
    pub fn lower_index_operands(&mut self, node: &SyntaxNode<Lang>) -> bool {
        let mut children = child_nodes(node);
        let (Some(collection), Some(index)) = (children.next(), children.next()) else {
            return false;
        };
        let collection_type = self.lower_expression(&collection);
        let check = match collection_type {
            Type::Known(ValueType::Array) | Type::Any | Type::Never => Ok(()),
            other => Err(format!("Indexing is not supported for {other}")),
        };
        self.type_error(&collection, check);
        let index_type = self.lower_expression(&index);
        let check = match index_type {
            Type::Known(ValueType::Number) | Type::Any | Type::Never => Ok(()),
            other => Err(format!("Index must be Число, not {other}")),
        };
        self.type_error(&index, check);
        true
    }

    /// `Новый Тип` or `Новый Тип(аргументы)`.
    fn lower_new(&mut self, node: &SyntaxNode<Lang>, line: u32) -> Type {
        let Some(target) = child_nodes(node).next() else {
            return Type::Any;
        };
        let (type_node, args) = match target.kind() {
            SyntaxKind::Identifier => (target, Vec::new()),
            SyntaxKind::CallExpression => {
                let mut children = child_nodes(&target);
                let Some(callee) = children.next() else {
                    return Type::Any;
                };
                (callee, call_arguments(children))
            }
            _ => (target, Vec::new()),
        };
        if type_node.kind() != SyntaxKind::Identifier {
            self.ctx
                .error(&type_node, "Expected a type name after 'Новый'");
            return Type::Any;
        }
        let Some((argc, _)) = self.lower_arguments(node, &args, line) else {
            return Type::Any;
        };
        let name = identifier_text(&type_node);
        let value_type = builtins::lookup_type(&name);
        if value_type.is_none() {
            self.type_error(&type_node, Err(format!("Type is not defined ({name})")));
        }
        let type_name = self.ctx.strings.push(&name);
        self.emit(Instruction::New { type_name, argc }, line);
        value_type.map_or(Type::Any, Type::Known)
    }

    /// Pushes the arguments, skipped ones as `Неопределено`. Returns their count and types.
    fn lower_arguments(
        &mut self,
        node: &SyntaxNode<Lang>,
        args: &[Option<SyntaxNode<Lang>>],
        line: u32,
    ) -> Option<(u8, Vec<Type>)> {
        let Ok(argc) = u8::try_from(args.len()) else {
            self.ctx.error(node, "Too many actual parameters");
            return None;
        };
        let mut types = Vec::with_capacity(args.len());
        for arg in args {
            types.push(match arg {
                Some(arg) => self.lower_expression(arg),
                None => {
                    self.emit(Instruction::PushC(Constant::Undefined), line);
                    Type::UNDEFINED
                }
            });
        }
        Some((argc, types))
    }

    /// Lowers a call. With `want_value` the result is left on the stack, otherwise it is
    /// discarded. Returns the type of the result.
    pub fn lower_call(&mut self, node: &SyntaxNode<Lang>, want_value: bool) -> Type {
        let line = self.ctx.line(node);
        let mut children = child_nodes(node);
        let Some(callee) = children.next() else {
            return Type::Any;
        };
        let args = call_arguments(children);
        if callee.kind() == SyntaxKind::MemberExpression {
            return self.lower_method_call_on_object(node, &callee, &args, want_value, line);
        }
        if callee.kind() != SyntaxKind::Identifier {
            self.ctx
                .error(&callee, "Expected a procedure or function name");
            return Type::Any;
        }
        let name = identifier_text(&callee);

        if let Some(&index) = self.ctx.signature_map.get(&name.to_lowercase()) {
            self.lower_method_call(node, index, &name, &args, want_value, line)
        } else if let Some(builtin) = builtins::lookup(&name) {
            self.lower_builtin_call(node, builtin, &args, want_value, line)
        } else {
            self.ctx.error(
                &callee,
                format!("Procedure or function is not defined ({name})"),
            );
            Type::Any
        }
    }

    /// `Объект.Метод(аргументы)`: the method is looked up at run time. In strict mode calls on
    /// a value of a known type are checked against its methods.
    fn lower_method_call_on_object(
        &mut self,
        node: &SyntaxNode<Lang>,
        callee: &SyntaxNode<Lang>,
        args: &[Option<SyntaxNode<Lang>>],
        want_value: bool,
        line: u32,
    ) -> Type {
        let mut parts = child_nodes(callee);
        let (Some(object), Some(method)) = (parts.next(), parts.next()) else {
            return Type::Any;
        };
        let object_type = self.lower_expression(&object);
        let Some((argc, _)) = self.lower_arguments(node, args, line) else {
            return Type::Any;
        };
        let method_name = identifier_text(&method);
        let result = match object_type {
            Type::Known(ValueType::Array) => match builtins::lookup_array_method(&method_name) {
                Some(m) if want_value && m.returns == Returns::Nothing => Err(format!(
                    "Procedure cannot be used as a function ({})",
                    m.name
                )),
                Some(m) if argc < m.min_args => {
                    Err(format!("Not enough actual parameters ({})", m.name))
                }
                Some(m) if argc > m.max_args => {
                    Err(format!("Too many actual parameters ({})", m.name))
                }
                Some(m) => Ok(match m.returns {
                    Returns::Nothing => Type::UNDEFINED,
                    Returns::Number => Type::NUMBER,
                    Returns::Any => Type::Any,
                }),
                None => Err(format!("Method is not defined for Массив ({method_name})")),
            },
            Type::Known(other) => Err(format!("Type {other} has no methods ({method_name})")),
            Type::Any | Type::Never => Ok(Type::Any),
        };
        let result = self.typed(&method, result);
        let name = self.ctx.strings.push(&method_name);
        self.emit(Instruction::CallMethod { name, argc }, line);
        if !want_value {
            self.emit(Instruction::Pop, line);
        }
        result
    }

    fn lower_method_call(
        &mut self,
        node: &SyntaxNode<Lang>,
        index: usize,
        name: &str,
        args: &[Option<SyntaxNode<Lang>>],
        want_value: bool,
        line: u32,
    ) -> Type {
        let signature = &self.ctx.signatures[index];
        let returns_value = signature.returns_value;
        let return_type = signature.return_type;
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
            return Type::Any;
        }
        if args.len() > params.len() {
            self.ctx
                .error(node, format!("Too many actual parameters ({name})"));
            return Type::Any;
        }

        // Arguments that are plain variables passed to non-`Знач` parameters get the final
        // parameter value written back after the call.
        let mut write_back = Vec::new();
        for (position, &(by_value, default)) in params.iter().enumerate() {
            match args.get(position) {
                Some(Some(arg)) => {
                    self.lower_expression(arg);
                    if !by_value {
                        if let Some((slot, variable)) = self.variable_argument(arg) {
                            write_back.push((position as u8, slot, variable));
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
                        return Type::Any;
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
        for (position, slot, variable) in write_back {
            self.emit(Instruction::PushOut(position), line);
            // Parameters accept values of any type, so the value written back is unknown.
            self.store(slot, Type::Any, &variable, line);
        }
        if !want_value {
            self.emit(Instruction::Pop, line);
        }
        if returns_value {
            return_type
        } else {
            Type::UNDEFINED
        }
    }

    /// The variable an argument expression names directly, if any.
    fn variable_argument(&self, arg: &SyntaxNode<Lang>) -> Option<(Slot, SyntaxNode<Lang>)> {
        let inner = child_nodes(arg).next()?;
        if inner.kind() != SyntaxKind::Identifier {
            return None;
        }
        let slot = self.resolve(&identifier_text(&inner))?;
        Some((slot, inner))
    }

    fn lower_builtin_call(
        &mut self,
        node: &SyntaxNode<Lang>,
        builtin: &builtins::Builtin,
        args: &[Option<SyntaxNode<Lang>>],
        want_value: bool,
        line: u32,
    ) -> Type {
        if want_value && !builtin.returns_value {
            self.ctx.error(
                node,
                format!("Procedure cannot be used as a function ({})", builtin.name),
            );
            return Type::Any;
        }
        if args.len() < builtin.min_args as usize {
            self.ctx.error(
                node,
                format!("Not enough actual parameters ({})", builtin.name),
            );
            return Type::Any;
        }
        if args.len() > builtin.max_args as usize {
            self.ctx.error(
                node,
                format!("Too many actual parameters ({})", builtin.name),
            );
            return Type::Any;
        }

        let Some((argc, arg_types)) = self.lower_arguments(node, args, line) else {
            return Type::Any;
        };
        let name = self.ctx.strings.push(builtin.name);
        self.emit(Instruction::CallNative { name, argc }, line);
        if !want_value {
            self.emit(Instruction::Pop, line);
        }
        builtin_type(builtin.name, &arg_types)
    }
}

fn constant_type(constant: Constant) -> Type {
    Type::Known(match constant {
        Constant::Undefined => ValueType::Undefined,
        Constant::Null => ValueType::Null,
        Constant::Boolean(_) => ValueType::Boolean,
        Constant::Number(_) => ValueType::Number,
        Constant::String(_) => ValueType::String,
        Constant::Date(_) => ValueType::Date,
    })
}

/// Result type of a global context function.
fn builtin_type(name: &str, args: &[Type]) -> Type {
    match name {
        "Строка"
        | "ВРег"
        | "НРег"
        | "Лев"
        | "Прав"
        | "Сред"
        | "СокрЛ"
        | "СокрП"
        | "СокрЛП"
        | "СтрЗаменить"
        | "Формат"
        | "ОписаниеОшибки" => Type::STRING,
        "Число" | "СтрДлина" | "СтрНайти" | "Окр" | "Цел" | "Pow" | "Sqrt" => {
            Type::NUMBER
        }
        "Булево" | "ЗначениеЗаполнено" => Type::BOOLEAN,
        "ТекущаяДата" => Type::DATE,
        "Макс" | "Мин" => args.iter().fold(Type::Never, |a, &b| a.join(b)),
        "Сообщить" => Type::UNDEFINED,
        _ => Type::Any,
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

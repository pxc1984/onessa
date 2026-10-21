use crate::error::{LoadError, RuntimeError};
use crate::native::{self, Native, NativeContext};
use crate::value::Value;
use onessa_core::bytecode::{Constant, Instruction};
use onessa_core::object::{FunctionEntry, ObjectFile};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::io::Write;
use std::rc::Rc;

/// Maximum call depth before `Переполнение стека` is raised.
const MAX_CALL_DEPTH: usize = 10_000;

struct Frame {
    function: u32,
    /// Absolute offset of the next instruction in the code section.
    pc: usize,
    /// Index of the first local of this frame in `Vm::locals`.
    locals_base: usize,
    /// Operand stack height when the frame was entered.
    stack_base: usize,
}

/// An installed `Попытка` handler.
struct Handler {
    /// Number of frames when the handler was installed.
    frame_depth: usize,
    stack_height: usize,
    target: usize,
}

/// Outcome of executing one instruction.
enum Step {
    Continue,
    Finished,
}

pub struct Vm<'a> {
    object: &'a ObjectFile,
    output: &'a mut dyn Write,
    /// Native function and string constant of every string table index used by the code.
    natives: HashMap<u32, Native>,
    strings: Vec<Rc<str>>,
    stack: Vec<Value>,
    locals: Vec<Value>,
    globals: Vec<Value>,
    frames: Vec<Frame>,
    handlers: Vec<Handler>,
    /// Final values of the parameters of the call that returned last (for `PUSH_OUT`).
    out_params: Vec<Value>,
    /// Description of the exception being handled, for `ОписаниеОшибки()` and re-raising.
    error_description: Option<Rc<str>>,
}

impl<'a> Vm<'a> {
    /// Validates the object file and prepares it for execution. `Сообщить` writes to `output`.
    pub fn new(object: &'a ObjectFile, output: &'a mut dyn Write) -> Result<Self, LoadError> {
        let strings: Vec<Rc<str>> = (0..object.string_table.len() as u32)
            .map(|idx| Rc::from(&*object.string_table.get(idx).unwrap_or_default()))
            .collect();
        let natives = validate(object, &strings)?;
        Ok(Self {
            object,
            output,
            natives,
            strings,
            stack: Vec::new(),
            locals: Vec::new(),
            globals: vec![Value::Undefined; object.function_table.global_count as usize],
            frames: Vec::new(),
            handlers: Vec::new(),
            out_params: Vec::new(),
            error_description: None,
        })
    }

    /// Executes the module body.
    pub fn run(&mut self) -> Result<(), RuntimeError> {
        let entry = self.object.function_table.entry;
        self.enter(entry, Vec::new());
        loop {
            let offset = self.frames.last().map(|f| f.pc).unwrap_or_default();
            let step = match self.step() {
                Ok(step) => step,
                Err(message) => self.raise(message, offset)?,
            };
            if let Step::Finished = step {
                return Ok(());
            }
        }
    }

    fn function(&self, idx: u32) -> &'a FunctionEntry {
        &self.object.function_table.functions[idx as usize]
    }

    fn enter(&mut self, function: u32, args: Vec<Value>) {
        let entry = self.function(function);
        let locals_base = self.locals.len();
        self.locals.extend(args);
        self.locals
            .resize(locals_base + entry.local_count as usize, Value::Undefined);
        self.frames.push(Frame {
            function,
            pc: entry.code_offset as usize,
            locals_base,
            stack_base: self.stack.len(),
        });
    }

    fn pop(&mut self) -> Result<Value, String> {
        self.stack
            .pop()
            .ok_or_else(|| "internal error: operand stack underflow".to_string())
    }

    fn pop2(&mut self) -> Result<(Value, Value), String> {
        let rhs = self.pop()?;
        let lhs = self.pop()?;
        Ok((lhs, rhs))
    }

    fn constant(&self, constant: Constant) -> Value {
        match constant {
            Constant::Undefined => Value::Undefined,
            Constant::Null => Value::Null,
            Constant::Boolean(b) => Value::Boolean(b),
            Constant::Number(n) => Value::Number(n),
            Constant::String(idx) => Value::String(self.strings[idx as usize].clone()),
            Constant::Date(secs) => {
                Value::date_from_timestamp(secs).unwrap_or(Value::Date(Value::empty_date()))
            }
        }
    }

    /// Executes the instruction at the current pc. An `Err` is an exception message.
    fn step(&mut self) -> Result<Step, String> {
        let frame = self.frames.last_mut().expect("step without a frame");
        let code = &self.object.code_section.code;
        let instruction = Instruction::decode(code, &mut frame.pc).map_err(|e| e.to_string())?;
        let locals_base = frame.locals_base;

        match instruction {
            Instruction::PushC(constant) => {
                let value = self.constant(constant);
                self.stack.push(value);
            }
            Instruction::PushV(i) => {
                let value = self.locals[locals_base + i as usize].clone();
                self.stack.push(value);
            }
            Instruction::PopV(i) => {
                let value = self.pop()?;
                self.locals[locals_base + i as usize] = value;
            }
            Instruction::PushG(i) => self.stack.push(self.globals[i as usize].clone()),
            Instruction::PopG(i) => {
                let value = self.pop()?;
                self.globals[i as usize] = value;
            }
            Instruction::Pop => {
                self.pop()?;
            }
            Instruction::Dup => {
                let value = self.stack.last().cloned().ok_or("internal error: DUP on empty stack")?;
                self.stack.push(value);
            }
            Instruction::Add => {
                let (lhs, rhs) = self.pop2()?;
                self.stack.push(add(lhs, rhs)?);
            }
            Instruction::Sub => {
                let (lhs, rhs) = self.pop2()?;
                self.stack.push(sub(lhs, rhs)?);
            }
            Instruction::Mul => {
                let (lhs, rhs) = self.pop2()?;
                self.stack
                    .push(Value::Number(lhs.to_number()? * rhs.to_number()?));
            }
            Instruction::Div | Instruction::Mod => {
                let (lhs, rhs) = self.pop2()?;
                let (lhs, rhs) = (lhs.to_number()?, rhs.to_number()?);
                if rhs == 0.0 {
                    return Err("Деление на 0".to_string());
                }
                let result = if instruction == Instruction::Div {
                    lhs / rhs
                } else {
                    lhs % rhs
                };
                self.stack.push(Value::Number(result));
            }
            Instruction::Neg => {
                let value = self.pop()?.to_number()?;
                self.stack.push(Value::Number(-value));
            }
            Instruction::Eq | Instruction::Ne => {
                let (lhs, rhs) = self.pop2()?;
                let equal = lhs == rhs;
                self.stack
                    .push(Value::Boolean(equal == (instruction == Instruction::Eq)));
            }
            Instruction::Lt | Instruction::Le | Instruction::Gt | Instruction::Ge => {
                let (lhs, rhs) = self.pop2()?;
                let ordering = lhs.compare(&rhs)?;
                let result = match instruction {
                    Instruction::Lt => ordering == Ordering::Less,
                    Instruction::Le => ordering != Ordering::Greater,
                    Instruction::Gt => ordering == Ordering::Greater,
                    _ => ordering != Ordering::Less,
                };
                self.stack.push(Value::Boolean(result));
            }
            Instruction::Not => {
                let value = self.pop()?.to_boolean()?;
                self.stack.push(Value::Boolean(!value));
            }
            Instruction::Jump(target) => self.jump(target),
            Instruction::JumpIfFalse(target) => {
                if !self.pop()?.to_boolean()? {
                    self.jump(target);
                }
            }
            Instruction::Call { function, argc } => {
                if self.frames.len() >= MAX_CALL_DEPTH {
                    return Err("Переполнение стека".to_string());
                }
                let split = self
                    .stack
                    .len()
                    .checked_sub(argc as usize)
                    .ok_or("internal error: operand stack underflow")?;
                let args = self.stack.split_off(split);
                self.enter(function, args);
            }
            Instruction::CallNative { name, argc } => {
                let split = self
                    .stack
                    .len()
                    .checked_sub(argc as usize)
                    .ok_or("internal error: operand stack underflow")?;
                let args = self.stack.split_off(split);
                let ctx = NativeContext {
                    output: &mut *self.output,
                    error_description: self.error_description.as_deref(),
                };
                let result = native::call(self.natives[&name], ctx, &args)?;
                self.stack.push(result);
            }
            Instruction::PushOut(n) => {
                let value = self
                    .out_params
                    .get(n as usize)
                    .cloned()
                    .ok_or("internal error: PUSH_OUT without a preceding call")?;
                self.stack.push(value);
            }
            Instruction::Return => return self.return_from_frame(),
            Instruction::TryBegin(target) => self.handlers.push(Handler {
                frame_depth: self.frames.len(),
                stack_height: self.stack.len(),
                target: target as usize,
            }),
            Instruction::TryEnd => {
                self.handlers.pop();
            }
            Instruction::Raise => {
                let message = self.pop()?.to_string();
                return Err(message);
            }
            Instruction::ReRaise => {
                let message = self.error_description.as_deref().unwrap_or_default();
                return Err(message.to_string());
            }
        }
        Ok(Step::Continue)
    }

    fn jump(&mut self, target: u32) {
        if let Some(frame) = self.frames.last_mut() {
            frame.pc = target as usize;
        }
    }

    fn return_from_frame(&mut self) -> Result<Step, String> {
        let value = self.pop()?;
        let frame = self.frames.pop().expect("return without a frame");
        let param_count = self.function(frame.function).param_count as usize;

        self.out_params.clear();
        self.out_params
            .extend(self.locals.drain(frame.locals_base..).take(param_count));
        self.stack.truncate(frame.stack_base);
        let depth = self.frames.len();
        self.handlers.retain(|h| h.frame_depth <= depth);

        if self.frames.is_empty() {
            return Ok(Step::Finished);
        }
        self.stack.push(value);
        Ok(Step::Continue)
    }

    /// Transfers control to the innermost handler, or fails with the exception if there is
    /// none. `offset` is the instruction that raised it, used for the error location.
    fn raise(&mut self, message: String, offset: usize) -> Result<Step, RuntimeError> {
        let Some(handler) = self.handlers.pop() else {
            let debug = self.object.debug_info.as_ref();
            return Err(RuntimeError {
                message,
                source: debug.map(|d| self.strings[d.source_name as usize].to_string()),
                line: debug.and_then(|d| d.line_at(offset as u32)),
            });
        };

        self.error_description = Some(message.into());
        while self.frames.len() > handler.frame_depth {
            let frame = self.frames.pop().expect("handler frame exists");
            self.locals.truncate(frame.locals_base);
        }
        self.stack.truncate(handler.stack_height);
        self.jump(handler.target as u32);
        Ok(Step::Continue)
    }
}

fn add(lhs: Value, rhs: Value) -> Result<Value, String> {
    Ok(match (lhs, rhs) {
        (Value::String(a), rhs) => Value::string(format!("{a}{rhs}")),
        (Value::Date(d), rhs) => Value::Date(shift_date(d, rhs.to_number()?)?),
        (lhs @ Value::Number(_), rhs) => Value::Number(lhs.to_number()? + rhs.to_number()?),
        _ => return Err("Преобразование значения к типу Число не может быть выполнено".into()),
    })
}

fn sub(lhs: Value, rhs: Value) -> Result<Value, String> {
    Ok(match (lhs, rhs) {
        (Value::Date(a), Value::Date(b)) => Value::Number((a - b).num_seconds() as f64),
        (Value::Date(d), rhs) => Value::Date(shift_date(d, -rhs.to_number()?)?),
        (lhs, rhs) => Value::Number(lhs.to_number()? - rhs.to_number()?),
    })
}

fn shift_date(date: chrono::NaiveDateTime, seconds: f64) -> Result<chrono::NaiveDateTime, String> {
    chrono::TimeDelta::try_seconds(seconds.trunc() as i64)
        .and_then(|delta| date.checked_add_signed(delta))
        .ok_or_else(|| "Недопустимое значение даты".to_string())
}

/// Checks everything the interpreter relies on without re-checking at run time: instructions
/// decode, operands are in range, jumps stay inside their function, natives exist.
fn validate(object: &ObjectFile, strings: &[Rc<str>]) -> Result<HashMap<u32, Native>, LoadError> {
    let table = &object.function_table;
    let code = &object.code_section;
    let fail = |message: String| Err(LoadError(message));

    if table.entry as usize >= table.functions.len() {
        return fail(format!("entry function #{} does not exist", table.entry));
    }

    let mut natives = HashMap::new();
    for (idx, function) in table.functions.iter().enumerate() {
        let start = function.code_offset as usize;
        let end = start + function.code_len as usize;
        if end > code.code.len() || function.param_count as u16 > function.local_count {
            return fail(format!("function #{idx} has an invalid layout"));
        }
        let instructions = code
            .instructions(start, end)
            .map_err(|e| LoadError(format!("function #{idx}: {e}")))?;
        let starts: std::collections::HashSet<usize> =
            instructions.iter().map(|(offset, _)| *offset).collect();
        if !matches!(
            instructions.last(),
            Some((_, Instruction::Return | Instruction::Raise | Instruction::ReRaise | Instruction::Jump(_)))
        ) {
            return fail(format!("function #{idx} does not end with a return"));
        }

        for (offset, instruction) in instructions {
            let bad = |what: &str| LoadError(format!("function #{idx} at {offset}: {what}"));
            match instruction {
                Instruction::PushC(Constant::String(s)) if s as usize >= strings.len() => {
                    return Err(bad("string index out of range"))
                }
                Instruction::PushV(i) | Instruction::PopV(i) if i >= function.local_count => {
                    return Err(bad("local index out of range"))
                }
                Instruction::PushG(i) | Instruction::PopG(i) if i >= table.global_count => {
                    return Err(bad("global index out of range"))
                }
                Instruction::Jump(target)
                | Instruction::JumpIfFalse(target)
                | Instruction::TryBegin(target)
                    if !starts.contains(&(target as usize)) =>
                {
                    return Err(bad("jump target is not an instruction of this function"))
                }
                Instruction::Call { function: callee, argc } => {
                    match table.functions.get(callee as usize) {
                        Some(f) if f.param_count == argc => {}
                        _ => return Err(bad("call to a missing function or with wrong arity")),
                    }
                }
                Instruction::CallNative { name, .. } => {
                    let native = strings
                        .get(name as usize)
                        .and_then(|name| Native::resolve(name))
                        .ok_or_else(|| bad("unknown native function"))?;
                    natives.insert(name, native);
                }
                _ => {}
            }
        }
    }
    Ok(natives)
}

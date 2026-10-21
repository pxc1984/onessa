use crate::error::{LoadError, RuntimeError};
use crate::native::{self, NativeContext};
use crate::objects;
use crate::program::{Arith, Cmp, Op, Program};
use crate::value::{Value, NOT_A_NUMBER};
use onessa_core::object::{DebugInfo, ObjectFile};
use rust_decimal::Decimal;
use std::cmp::Ordering;
use std::io::Write;
use std::rc::Rc;

/// Maximum call depth before `Переполнение стека` is raised.
const MAX_CALL_DEPTH: usize = 10_000;

const OVERFLOW: &str = "Переполнение при выполнении арифметической операции";
const UNDERFLOW: &str = "internal error: operand stack underflow";

struct Frame {
    function: u32,
    /// Index of the next op. Only up to date while the frame is not the running one.
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

/// An exception raised by the op at `pc`.
struct Fault {
    message: String,
    pc: usize,
}

pub struct Vm<'a> {
    program: Program,
    debug_info: Option<&'a DebugInfo>,
    output: &'a mut dyn Write,
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
        let program = Program::load(object)?;
        let globals = vec![Value::Undefined; program.global_count];
        Ok(Self {
            program,
            debug_info: object.debug_info.as_ref(),
            output,
            stack: Vec::new(),
            locals: Vec::new(),
            globals,
            frames: Vec::new(),
            handlers: Vec::new(),
            out_params: Vec::new(),
            error_description: None,
        })
    }

    /// Executes the module body.
    pub fn run(&mut self) -> Result<(), RuntimeError> {
        let entry = &self.program.functions[self.program.entry as usize];
        self.locals.resize(entry.local_count, Value::Undefined);
        self.frames.push(Frame {
            function: self.program.entry,
            pc: entry.entry,
            locals_base: 0,
            stack_base: 0,
        });
        loop {
            match self.execute() {
                Ok(()) => return Ok(()),
                Err(fault) => self.raise(fault)?,
            }
        }
    }

    /// Runs the top frame until the entry function returns or an exception is raised.
    ///
    /// The program counter and the locals base of the running frame are kept in local
    /// variables and written back to the frame only on calls.
    fn execute(&mut self) -> Result<(), Fault> {
        let ops = &self.program.ops[..];
        let constants = &self.program.constants[..];
        let functions = &self.program.functions[..];
        let stack = &mut self.stack;
        let locals = &mut self.locals;
        let globals = &mut self.globals;
        let frames = &mut self.frames;

        let frame = frames.last().expect("execute without a frame");
        let mut pc = frame.pc;
        let mut base = frame.locals_base;

        // Evaluates a `Result<_, String>`, raising the error as an exception of the current op.
        macro_rules! check {
            ($result:expr) => {
                match $result {
                    Ok(value) => value,
                    Err(message) => {
                        return Err(Fault {
                            message: message.into(),
                            pc: pc - 1,
                        })
                    }
                }
            };
        }
        macro_rules! pop {
            () => {
                check!(stack.pop().ok_or(UNDERFLOW))
            };
        }
        macro_rules! top {
            () => {
                check!(stack.last_mut().ok_or(UNDERFLOW))
            };
        }
        loop {
            let op = ops[pc];
            pc += 1;
            match op {
                Op::PushC(i) => stack.push(constants[i as usize].clone()),
                Op::PushV(i) => stack.push(locals[base + i as usize].clone()),
                Op::PopV(i) => locals[base + i as usize] = pop!(),
                Op::PushG(i) => stack.push(globals[i as usize].clone()),
                Op::PopG(i) => globals[i as usize] = pop!(),
                Op::Pop => {
                    pop!();
                }
                Op::Dup => {
                    let value = top!().clone();
                    stack.push(value);
                }
                Op::Arith(op) => {
                    let rhs = pop!();
                    let lhs = top!();
                    *lhs = check!(arith(op, lhs, &rhs));
                }
                Op::ArithV { op, slot } => {
                    let lhs = top!();
                    *lhs = check!(arith(op, lhs, &locals[base + slot as usize]));
                }
                Op::ArithC { op, constant } => {
                    let lhs = top!();
                    *lhs = check!(arith(op, lhs, &constants[constant as usize]));
                }
                Op::ArithToV { op, slot } => {
                    let rhs = pop!();
                    let lhs = &mut locals[base + slot as usize];
                    *lhs = check!(arith(op, lhs, &rhs));
                }
                Op::Neg => {
                    let value = top!();
                    *value = match value {
                        Value::Int(n) if *n != i64::MIN => Value::Int(-*n),
                        _ => Value::number(-check!(value.to_number())),
                    };
                }
                Op::Cmp(op) => {
                    let rhs = pop!();
                    let lhs = top!();
                    *lhs = Value::Boolean(check!(compare(op, lhs, &rhs)));
                }
                Op::CmpJump { op, target } => {
                    let rhs = pop!();
                    let lhs = pop!();
                    if !check!(compare(op, &lhs, &rhs)) {
                        pc = target as usize;
                    }
                }
                Op::CmpVJump { op, slot, target } => {
                    let lhs = pop!();
                    if !check!(compare(op, &lhs, &locals[base + slot as usize])) {
                        pc = target as usize;
                    }
                }
                Op::CmpCJump {
                    op,
                    constant,
                    target,
                } => {
                    let lhs = pop!();
                    if !check!(compare(op, &lhs, &constants[constant as usize])) {
                        pc = target as usize;
                    }
                }
                Op::Not => {
                    let value = top!();
                    *value = Value::Boolean(!check!(value.to_boolean()));
                }
                Op::Jump(target) => pc = target as usize,
                Op::ForCheck {
                    counter,
                    limit,
                    target,
                } => {
                    let counter = &locals[base + counter as usize];
                    let limit = &locals[base + limit as usize];
                    if !check!(compare(Cmp::Le, counter, limit)) {
                        pc = target as usize;
                    }
                }
                Op::ForStep {
                    counter,
                    limit,
                    target,
                } => {
                    let slot = &mut locals[base + counter as usize];
                    match slot {
                        Value::Int(n) if *n < i64::MAX => *n += 1,
                        _ => *slot = check!(arith(Arith::Add, slot, &Value::Int(1))),
                    }
                    let counter = &locals[base + counter as usize];
                    let limit = &locals[base + limit as usize];
                    if check!(compare(Cmp::Le, counter, limit)) {
                        pc = target as usize;
                    }
                }
                Op::JumpIfFalse(target) => {
                    let condition = match pop!() {
                        Value::Boolean(b) => b,
                        value => check!(value.to_boolean()),
                    };
                    if !condition {
                        pc = target as usize;
                    }
                }
                Op::Call { function, argc } => {
                    if frames.len() >= MAX_CALL_DEPTH {
                        check!(Err("Переполнение стека"));
                    }
                    let callee = &functions[function as usize];
                    let split = check!(stack.len().checked_sub(argc as usize).ok_or(UNDERFLOW));
                    frames.last_mut().expect("running frame").pc = pc;
                    base = locals.len();
                    locals.extend(stack.drain(split..));
                    locals.resize(base + callee.local_count, Value::Undefined);
                    frames.push(Frame {
                        function,
                        pc: callee.entry,
                        locals_base: base,
                        stack_base: stack.len(),
                    });
                    pc = callee.entry;
                }
                Op::CallNative { native, argc } => {
                    let split = check!(stack.len().checked_sub(argc as usize).ok_or(UNDERFLOW));
                    let ctx = NativeContext {
                        output: &mut *self.output,
                        error_description: self.error_description.as_deref(),
                    };
                    let result = check!(native::call(native, ctx, &stack[split..]));
                    stack.truncate(split);
                    stack.push(result);
                }
                Op::PushOut(n) => {
                    let value = check!(self
                        .out_params
                        .get(n as usize)
                        .cloned()
                        .ok_or("internal error: PUSH_OUT without a preceding call"));
                    stack.push(value);
                }
                Op::Return => {
                    let value = pop!();
                    let frame = frames.pop().expect("return without a frame");
                    let param_count = functions[frame.function as usize].param_count;
                    self.out_params.clear();
                    self.out_params
                        .extend(locals.drain(frame.locals_base..).take(param_count));
                    stack.truncate(frame.stack_base);
                    if !self.handlers.is_empty() {
                        let depth = frames.len();
                        self.handlers.retain(|h| h.frame_depth <= depth);
                    }
                    let Some(caller) = frames.last() else {
                        return Ok(());
                    };
                    pc = caller.pc;
                    base = caller.locals_base;
                    stack.push(value);
                }
                Op::TryBegin(target) => self.handlers.push(Handler {
                    frame_depth: frames.len(),
                    stack_height: stack.len(),
                    target: target as usize,
                }),
                Op::TryEnd => {
                    self.handlers.pop();
                }
                Op::Raise => {
                    let message = pop!().to_string();
                    check!(Err(message));
                }
                Op::ReRaise => {
                    let message = self.error_description.as_deref().unwrap_or_default();
                    check!(Err(message.to_string()));
                }
                Op::New { type_name, argc } => {
                    let split = check!(stack.len().checked_sub(argc as usize).ok_or(UNDERFLOW));
                    let value = check!(objects::construct(
                        &self.program.lowercase[type_name as usize],
                        &self.program.strings[type_name as usize],
                        &stack[split..],
                    ));
                    stack.truncate(split);
                    stack.push(value);
                }
                Op::IndexGet => {
                    let index = pop!();
                    let collection = top!();
                    *collection = check!(objects::index_get(collection, &index));
                }
                Op::IndexGetV(slot) => {
                    let collection = top!();
                    let index = &locals[base + slot as usize];
                    *collection = check!(objects::index_get(collection, index));
                }
                Op::IndexVV { collection, index } => {
                    let value = check!(objects::index_get(
                        &locals[base + collection as usize],
                        &locals[base + index as usize],
                    ));
                    stack.push(value);
                }
                Op::IndexVVV { collection, i, k } => {
                    let value = check!(objects::index_get2(
                        &locals[base + collection as usize],
                        &locals[base + i as usize],
                        &locals[base + k as usize],
                    ));
                    stack.push(value);
                }
                Op::IndexGetC(constant) => {
                    let collection = top!();
                    let index = &constants[constant as usize];
                    *collection = check!(objects::index_get(collection, index));
                }
                Op::IndexSet => {
                    let value = pop!();
                    let index = pop!();
                    let collection = pop!();
                    check!(objects::index_set(&collection, &index, value));
                }
                Op::CallMethod { name, argc } => {
                    let split = check!(stack.len().checked_sub(argc as usize + 1).ok_or(UNDERFLOW));
                    let result = check!(objects::call_method(
                        &stack[split],
                        &self.program.lowercase[name as usize],
                        &self.program.strings[name as usize],
                        &stack[split + 1..],
                    ));
                    stack.truncate(split);
                    stack.push(result);
                }
            }
        }
    }

    /// Transfers control to the innermost handler, or fails with the exception if there is
    /// none.
    fn raise(&mut self, fault: Fault) -> Result<(), RuntimeError> {
        let Some(handler) = self.handlers.pop() else {
            let offset = self.program.op_offsets[fault.pc];
            let debug = self.debug_info;
            return Err(RuntimeError {
                message: fault.message,
                source: debug.map(|d| self.program.strings[d.source_name as usize].to_string()),
                line: debug.and_then(|d| d.line_at(offset)),
            });
        };

        self.error_description = Some(fault.message.into());
        while self.frames.len() > handler.frame_depth {
            let frame = self.frames.pop().expect("handler frame exists");
            self.locals.truncate(frame.locals_base);
        }
        self.stack.truncate(handler.stack_height);
        if let Some(frame) = self.frames.last_mut() {
            frame.pc = handler.target;
        }
        Ok(())
    }
}

/// Arithmetic on two values. Integers are computed in `i64` and fall back to `Decimal` on
/// overflow.
#[inline(always)]
fn arith(op: Arith, lhs: &Value, rhs: &Value) -> Result<Value, String> {
    if let (Value::Int(a), Value::Int(b)) = (lhs, rhs) {
        let result = match op {
            Arith::Add => a.checked_add(*b),
            Arith::Sub => a.checked_sub(*b),
            Arith::Mul => a.checked_mul(*b),
            // `None` for a zero divisor, which `rem` then reports.
            Arith::Mod => a.checked_rem(*b),
            Arith::Div => None,
        };
        if let Some(n) = result {
            return Ok(Value::Int(n));
        }
    }
    arith_slow(op, lhs.clone(), rhs.clone())
}

#[inline(never)]
fn arith_slow(op: Arith, lhs: Value, rhs: Value) -> Result<Value, String> {
    match op {
        Arith::Add => add(lhs, rhs),
        Arith::Sub => sub(lhs, rhs),
        Arith::Mul => mul(lhs, rhs),
        Arith::Div => div(lhs, rhs),
        Arith::Mod => rem(lhs, rhs),
    }
}

#[inline(always)]
fn compare(op: Cmp, lhs: &Value, rhs: &Value) -> Result<bool, String> {
    let ordering = match (op, lhs, rhs) {
        (_, Value::Int(a), Value::Int(b)) => a.cmp(b),
        (Cmp::Eq, _, _) => return Ok(lhs == rhs),
        (Cmp::Ne, _, _) => return Ok(lhs != rhs),
        _ => lhs.compare(rhs)?,
    };
    Ok(match op {
        Cmp::Eq => ordering == Ordering::Equal,
        Cmp::Ne => ordering != Ordering::Equal,
        Cmp::Lt => ordering == Ordering::Less,
        Cmp::Le => ordering != Ordering::Greater,
        Cmp::Gt => ordering == Ordering::Greater,
        Cmp::Ge => ordering != Ordering::Less,
    })
}

fn number(result: Option<Decimal>) -> Result<Value, &'static str> {
    result.map(Value::number).ok_or(OVERFLOW)
}

fn divide(lhs: Decimal, rhs: Decimal, remainder: bool) -> Result<Value, &'static str> {
    if rhs.is_zero() {
        return Err("Деление на 0");
    }
    number(if remainder {
        lhs.checked_rem(rhs)
    } else {
        lhs.checked_div(rhs)
    })
}

fn add(lhs: Value, rhs: Value) -> Result<Value, String> {
    Ok(match (lhs, rhs) {
        (Value::String(a), rhs) => Value::string(format!("{a}{rhs}")),
        (Value::Date(d), rhs) => Value::Date(shift_date(d, seconds(&rhs)?)?),
        (lhs @ (Value::Int(_) | Value::Number(_)), rhs) => {
            number(lhs.to_number()?.checked_add(rhs.to_number()?))?
        }
        _ => return Err(NOT_A_NUMBER.into()),
    })
}

fn sub(lhs: Value, rhs: Value) -> Result<Value, String> {
    Ok(match (lhs, rhs) {
        (Value::Date(a), Value::Date(b)) => Value::Int((a - b).num_seconds()),
        (Value::Date(d), rhs) => Value::Date(shift_date(d, -seconds(&rhs)?)?),
        (lhs, rhs) => number(lhs.to_number()?.checked_sub(rhs.to_number()?))?,
    })
}

fn mul(lhs: Value, rhs: Value) -> Result<Value, String> {
    Ok(number(lhs.to_number()?.checked_mul(rhs.to_number()?))?)
}

fn div(lhs: Value, rhs: Value) -> Result<Value, String> {
    Ok(divide(lhs.to_number()?, rhs.to_number()?, false)?)
}

fn rem(lhs: Value, rhs: Value) -> Result<Value, String> {
    Ok(divide(lhs.to_number()?, rhs.to_number()?, true)?)
}

fn seconds(value: &Value) -> Result<i64, String> {
    value
        .to_number()?
        .trunc()
        .try_into()
        .map_err(|_| "Число слишком большое для даты".to_string())
}

fn shift_date(date: chrono::NaiveDateTime, seconds: i64) -> Result<chrono::NaiveDateTime, String> {
    chrono::TimeDelta::try_seconds(seconds)
        .and_then(|delta| date.checked_add_signed(delta))
        .ok_or_else(|| "Недопустимое значение даты".to_string())
}

# Bytecode design and virtual machine implementation

так вот, для того чтоб создать какой-никакой байткод для этого языка я буду
использовать виртуальную машину на базе стека

## Pipeline

```
source ──lexer──▶ tokens ──parser──▶ syntax tree (rowan, lossless)
       ──onessa-cfg──▶ control flow graph ──onessa-codegen──▶ object file (.osc)
       ──onessa-vm──▶ execution
```

```
onessa compile module.bsl          # writes module.osc
onessa run module.osc              # or `onessa run module.bsl` to compile and run
onessa disasm module.osc           # bytecode listing
```

## Execution model

* Every method has a frame with `local_count` local slots; parameters occupy the first
  `param_count` slots. Module variables (`Перем` at module level) are globals.
* The module body is compiled as a separate method (`<module>`), the entry point of the
  object file. Variables assigned in the body without `Перем` are its locals.
* Every call pushes exactly one value: functions push their result, procedures push
  `Неопределено`, which the caller discards when the call is a statement.
* Operands are little endian. Jump targets are absolute byte offsets in the code section.

## Category 1: Stack management
```
PUSH_C v       // push constant (tag + payload: undefined, null, bool, f64, string index, date)
PUSH_V idx     // push local variable with variable index (u16)
POP_V idx      // pop top of the stack into local variable
PUSH_G idx     // push module variable (u16)
POP_G idx      // pop top of the stack into module variable
POP            // discard the top of the stack
DUP            // duplicate top of the stack
```

## Category 2: Arithmetic, comparison and logic
```
ADD SUB MUL DIV MOD NEG   // numbers; ADD concatenates if the left operand is a string,
                          // date ± number shifts by seconds, date - date gives seconds
EQ NE                     // any values; values of different types are never equal
LT LE GT GE               // values of the same primitive type only
NOT                       // boolean negation
```

`И`/`ИЛИ` have no instructions: they are compiled into branches (short-circuit evaluation).

## Category 3: Control flow
```
JUMP target               // u32 offset
JUMP_IF_FALSE target      // pops the condition (Булево, or Число where 0 is Ложь)
CALL fn argc              // u32 function index, u8 argument count (always = param count)
CALL_NATIVE name argc     // u32 string index of the builtin name, u8 argument count
PUSH_OUT n                // after CALL: final value of parameter n (by-reference write-back)
RETURN                    // returns the top of the stack
```

Parameters without `Знач` are passed by reference: when the argument is a variable, the
caller follows the `CALL` with `PUSH_OUT n` + `POP_V`/`POP_G` for each such argument.

## Category 4: Exceptions
```
TRY_BEGIN handler         // installs a handler: frame depth, stack height, handler offset
TRY_END                   // removes the innermost handler
RAISE                     // pops the message and raises an exception
RERAISE                   // raises the exception being handled again
```

Runtime errors (`Деление на 0`, type errors, `Переполнение стека`) are exceptions too and can
be handled by `Попытка`. An exception unwinds frames down to the handler's frame and restores
the operand stack height. `RETURN` drops the handlers installed by the returning frame.

## Object file

See `object/mod.rs`. Sections: string table (`STR\0`), metadata references (`MDR\0`, always
empty in v0), function table (`FUN\0`, with the entry index and the number of module
variables), code (`COD\0`) and optional debug info (`DBG\0`, offset → line table).

## v0 scope

Supported: module variables, procedures and functions (defaults, `Знач`, by-reference
parameters, recursion, `Экспорт` is recorded), `Если`/`ИначеЕсли`/`Иначе`, `Пока`,
`Для ... По`, `Прервать`/`Продолжить`, `Попытка`/`Исключение`/`ВызватьИсключение`, `?(,,)`,
primitive types (Неопределено, Null, Булево, Число, Строка, Дата) and the builtins listed in
`builtins.rs`. Russian and English keywords and names are case-insensitive.

Not yet: objects (`Новый`, properties, methods), collections and `Для Каждого`, indexing,
queries and metadata, preprocessor instructions and compilation directives (`#Если`, `&НаСервере`),
`Перейти`/labels, `Выполнить`/`Вычислить`. The compiler reports these as errors. Numbers are
`f64` rather than decimal.

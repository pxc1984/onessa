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
onessa run --strict module.bsl     # compile with static typing (also for compile, disasm)
```

## Execution model

* Every method has a frame with `local_count` local slots; parameters occupy the first
  `param_count` slots. Module variables (`Перем` at module level) are globals.
* The module body is compiled as a separate method (`<module>`), the entry point of the
  object file. Variables assigned in the body without `Перем` are its locals.
* Every call pushes exactly one value: functions push their result, procedures push
  `Неопределено`, which the caller discards when the call is a statement.
* Operands are little endian. Jump targets are absolute byte offsets in the code section.
* Parameters without `Знач` are passed by reference (see `PUSH_OUT`).

The VM validates an object file once when loading it and decodes the bytecode into an array
of ops (`vm/src/program.rs`): jump targets become op indices, constants become values, and
native function, type and method names are resolved. Frequent instruction sequences are fused
into superinstructions there (indexing and arithmetic with a local or constant operand,
compare and branch, `X = X <op> expression`, `a[i]`, `a[i][k]`, stores into a local array).
They are an implementation detail of the VM and never appear in object files.

Numbers are `rust_decimal::Decimal`; integers that fit in `i64` are kept as `i64` and fall
back to `Decimal` on overflow.
## Category 1: Stack management
```
PUSH_C v       // push constant (tag + payload: undefined, null, bool, decimal, string index, date)
PUSH_V idx     // push local variable with variable index (u16)
POP_V idx      // pop top of the stack into local variable
PUSH_G idx     // push module variable (u16)
POP_G idx      // pop top of the stack into module variable
POP            // discard the top of the stack
DUP            // duplicate top of the stack
CHECK_TYPE t   // raise unless the top of the stack has type t (u8 tag); strict mode only
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
FOR_CHECK c l target      // Для: jump to target (the exit) unless local c <= local l
FOR_STEP c l target       // Для: c = c + 1, then jump to target (the body) if c <= l
CALL fn argc              // u32 function index, u8 argument count (always = param count)
CALL_NATIVE name argc     // u32 string index of the builtin name, u8 argument count
PUSH_OUT n                // after CALL: final value of parameter n (by-reference write-back)
RETURN                    // returns the top of the stack
```

Parameters without `Знач` are passed by reference: when the argument is a variable, the
caller follows the `CALL` with `PUSH_OUT n` + `POP_V`/`POP_G` for each such argument, unless
the method never changes the parameter (never assigns it, never uses it as a `Для` counter,
never passes it as a variable to a procedure or function of the module).

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

## Category 5: Objects
```
NEW type argc             // u32 string index of the type name, u8 constructor argument count
INDEX_GET                 // pops index and collection, pushes collection[index]
INDEX_SET                 // pops value, index and collection: collection[index] = value
CALL_METHOD name argc     // u32 string index of the method name, u8 argument count; pops the
                          // arguments and the object, pushes the result (Неопределено for
                          // procedures, which the caller discards when the call is a statement)
```

Type and method names are resolved at run time, case-insensitively, in Russian or English.
Objects are shared by reference. Supported types: `Массив` (`Новый Массив`, `Новый Массив(n)`,
`Новый Массив(n, m, ...)`; 0-based indexing; `Добавить`, `Вставить`, `Количество`, `ВГраница`,
`Получить`, `Установить`, `Найти`, `Удалить`, `Очистить`).

## Strict mode

With `--strict` a variable gets the type of the value first assigned to it and keeps it:

* assigning a value of another known type is a compile error;
* a value whose type is only known at run time (array elements, parameters, results of
  functions that may return different types, by-reference write-back) is checked with
  `CHECK_TYPE` before it is stored into a typed variable;
* variables first assigned `Неопределено`, `Null` or a value of an unknown type, and
  parameters, are `Произвольный` and accept anything;
* operators, conditions (`Булево` only), indexing, `Новый`, methods of `Массив` and the
  bounds of `Для` (`Число` only) are checked against the static types; numbers and strings
  are not converted into each other implicitly (`1 + "2"` is an error, `"а" + 1` is not);
* the result types of functions are inferred from their `Возврат`, recursion included.

The types are checks: the generated code is the same as without `--strict`, apart from
`CHECK_TYPE`.

## Object file

See `object/mod.rs`. Sections: string table (`STR\0`), metadata references (`MDR\0`, always
empty in v0), function table (`FUN\0`, with the entry index and the number of module
variables), code (`COD\0`) and optional debug info (`DBG\0`, offset → line table).

## v0 scope

Supported: module variables, procedures and functions (defaults, `Знач`, by-reference
parameters, recursion, `Экспорт` is recorded), `Если`/`ИначеЕсли`/`Иначе`, `Пока`,
`Для ... По`, `Прервать`/`Продолжить`, `Попытка`/`Исключение`/`ВызватьИсключение`, `?(,,)`,
primitive types (Неопределено, Null, Булево, Число, Строка, Дата), `Массив` with indexing and
methods, and the builtins listed in `builtins.rs`. Russian and English keywords and names are case-insensitive.

Not yet: object properties, other collections (`Структура`, `Соответствие`, ...) and
`Для Каждого`,
queries and metadata, preprocessor instructions and compilation directives (`#Если`, `&НаСервере`),
`Перейти`/labels, `Выполнить`/`Вычислить`. The compiler reports these as errors.

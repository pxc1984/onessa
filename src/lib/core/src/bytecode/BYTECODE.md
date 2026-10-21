# Bytecode design and virtual machine implementation

так вот, для того чтоб создать какой-никакой байткод для этого языка я буду
использовать виртуальную машину на базе стека


## Category 1: Stack management
```
PUSH_C v // push constant
PUSH_V var_idx // push local variable with variable index
POP_V var_idx // pop top of the stack into local variable
POP // discard the top of the stack
DUP // duplicate top of the stack
```

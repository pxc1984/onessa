use crate::generate;
use logos::Logos;
use onessa_core::object::{Deserializable, ObjectFile, Serializable};
use onessa_lexer::Token;

fn compile(source: &str) -> ObjectFile {
    let mut lexer = Token::lexer(source);
    let tree = onessa_ast::build_ast(&mut lexer);
    assert!(tree.errors.is_empty(), "{:?}", tree.errors);
    let module = onessa_cfg::build_cfg(&tree.syntax()).expect("lowering failed");
    generate(module, "test.bsl")
}

#[test]
fn simple_module() {
    let object = compile("А = 1;\nСообщить(А);");
    assert_eq!(
        object.disassemble(),
        "\
globals: 0, entry: fn#0
fn#0 <module> (params: 0, locals: 1)
  0000  PUSH_C 1  ; line 1
  0010  POP_V 0
  0013  PUSH_V 0  ; line 2
  0016  CALL_NATIVE str#0 1  ; \"Сообщить\"
  0022  POP
  0023  PUSH_C undefined
  0025  RETURN
"
    );
}

#[test]
fn branches_fall_through_and_jump_to_offsets() {
    let object = compile("Если Истина Тогда\n  А = 1;\nИначе\n  А = 2;\nКонецЕсли;");
    assert_eq!(
        object.disassemble(),
        "\
globals: 0, entry: fn#0
fn#0 <module> (params: 0, locals: 1)
  0000  PUSH_C true  ; line 1
  0002  JUMP_IF_FALSE 25
  0007  PUSH_C 1  ; line 2
  0017  POP_V 0
  0020  JUMP 38
  0025  PUSH_C 2  ; line 4
  0035  POP_V 0
  0038  PUSH_C undefined  ; line 5
  0040  RETURN
"
    );
}

#[test]
fn methods_have_their_own_code_ranges() {
    let object = compile("Функция Ф(А)\n  Возврат А;\nКонецФункции\nСообщить(Ф(1));");
    let functions = &object.function_table.functions;
    assert_eq!(functions.len(), 2);
    assert_eq!(object.function_table.entry, 1);
    assert_eq!(functions[0].code_offset, 0);
    assert_eq!(functions[1].code_offset, functions[0].code_len);
    assert!(functions[0].returns_value);
    assert_eq!(&*object.string_table.get(functions[0].name).unwrap(), "Ф");
}

#[test]
fn object_file_roundtrip() {
    let object = compile("Для Сч = 1 По 3 Цикл Сообщить(Сч); КонецЦикла;");
    let mut bytes = Vec::new();
    object.serialize(&mut bytes).unwrap();
    let restored = ObjectFile::deserialize(&mut bytes.as_slice()).unwrap();
    assert_eq!(restored.disassemble(), object.disassemble());
}

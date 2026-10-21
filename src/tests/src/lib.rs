//! Workspace-level tests live here: whole programs are compiled, serialized, loaded back and
//! executed, and their output is compared with the expected one.

use logos::Logos;
use onessa_core::object::{Deserializable, ObjectFile, Serializable};
use onessa_lexer::Token;
use onessa_vm::Vm;

/// Compiles `source` through the whole pipeline, including an object file round trip.
pub fn compile(source: &str) -> Result<ObjectFile, Vec<String>> {
    let mut lexer = Token::lexer(source);
    let tree = onessa_ast::build_ast(&mut lexer);
    if !tree.errors.is_empty() {
        return Err(tree.errors);
    }
    let module = onessa_cfg::build_cfg(&tree.syntax())
        .map_err(|errors| errors.iter().map(ToString::to_string).collect::<Vec<_>>())?;
    let object = onessa_codegen::generate(module, "test.bsl");

    let mut bytes = Vec::new();
    object.serialize(&mut bytes).expect("serialization failed");
    Ok(ObjectFile::deserialize(&mut bytes.as_slice()).expect("deserialization failed"))
}

/// Output of the program, or the uncaught exception after the output produced so far.
pub fn run(source: &str) -> Result<String, (String, String)> {
    let object = compile(source).unwrap_or_else(|errors| panic!("compile errors: {errors:?}"));
    let mut output = Vec::new();
    let result = Vm::new(&object, &mut output).expect("load failed").run();
    let output = String::from_utf8(output).expect("output is UTF-8");
    match result {
        Ok(()) => Ok(output),
        Err(err) => Err((output, err.to_string())),
    }
}

#[cfg(test)]
mod programs;

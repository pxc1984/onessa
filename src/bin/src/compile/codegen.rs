use log::{debug, info};
use onessa_cfg::Module;
use onessa_core::object::ObjectFile;
use std::time::Instant;

pub fn codegen(module: Module, source_name: &str) -> ObjectFile {
    info!("Generating bytecode");
    let start = Instant::now();
    let object = onessa_codegen::generate(module, source_name);
    debug!(
        "Bytecode generated successfully in {}ms ({} bytes)",
        start.elapsed().as_millis(),
        object.code_section.code.len()
    );
    object
}

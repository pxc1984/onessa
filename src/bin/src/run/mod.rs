use crate::cli::{DisasmArgs, RunArgs};
use crate::compile::compile_file;
use log::{debug, error};
use onessa_core::object::{ObjectFile, MAGIC_NUMBER};
use onessa_vm::Vm;
use std::io::Read;
use std::path::Path;

/// Runs an object file, or compiles and runs a source file. Returns the process exit code.
pub fn run(args: RunArgs) -> i32 {
    let object = load(&args.input);
    let stdout = std::io::stdout();
    let mut output = stdout.lock();
    let mut vm = match Vm::new(&object, &mut output) {
        Ok(vm) => vm,
        Err(err) => {
            error!("{err}");
            return 1;
        }
    };
    match vm.run() {
        Ok(()) => 0,
        Err(err) => {
            error!("{err}");
            1
        }
    }
}

pub fn disasm(args: DisasmArgs) -> i32 {
    print!("{}", load(&args.input).disassemble());
    0
}

fn load(path: &Path) -> ObjectFile {
    if !is_object_file(path) {
        debug!("{} is not an object file, compiling it", path.display());
        return compile_file(Some(path.to_path_buf()));
    }
    ObjectFile::from_file(path).unwrap_or_else(|err| {
        error!("Failed to load {}: {err}", path.display());
        std::process::exit(1);
    })
}

fn is_object_file(path: &Path) -> bool {
    let mut magic = [0u8; 4];
    std::fs::File::open(path)
        .and_then(|mut file| file.read_exact(&mut magic))
        .is_ok_and(|()| &magic == MAGIC_NUMBER)
}

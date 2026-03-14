use log::{debug, error};
use onessa_core::io::Source;
use std::path::PathBuf;
use std::process::exit;

pub fn open_source(path_buf: Option<PathBuf>) -> Source {
    let source = Source::open(path_buf).unwrap_or_else(|err| {
        error!("Failed to open source file: {}", err);
        exit(1);
    });
    debug!("Opened source file {:?}", source.path_buf);
    source
}

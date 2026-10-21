use log::{debug, error, info};
use onessa_ast::Lang;
use onessa_cfg::Module;
use rowan::SyntaxNode;
use std::process::exit;
use std::time::Instant;

pub fn build_cfg(syntax: &SyntaxNode<Lang>, source_name: &str, strict: bool) -> Module {
    info!("Building control flow graph");
    let start = Instant::now();
    let options = onessa_cfg::Options { strict };
    let module = onessa_cfg::build_cfg_with(syntax, options).unwrap_or_else(|diagnostics| {
        for diagnostic in &diagnostics {
            error!(
                "{source_name}:{}:{}: {}",
                diagnostic.line, diagnostic.column, diagnostic.message
            );
        }
        exit(1);
    });
    debug!(
        "CFG built successfully in {}ms",
        start.elapsed().as_millis()
    );
    #[cfg(feature = "cfg_dump")]
    debug!("{module}");
    module
}

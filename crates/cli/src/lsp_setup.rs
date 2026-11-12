use std::path::PathBuf;
use std::sync::Arc;

use dshr_config::Config;
use dshr_lsp::{LspManager, LspSpec};
use dshr_tools::ToolRegistry;

/// Registers LSP tools when at least one server is configured.
pub fn register(registry: &mut ToolRegistry, config: &Config, root: PathBuf) {
    let specs = config
        .lsp
        .servers
        .iter()
        .filter(|server| server.enabled)
        .map(|server| LspSpec {
            name: server.name.clone(),
            command: server.command.clone(),
            args: server.args.clone(),
            extensions: server.extensions.clone(),
            root: root.clone(),
        })
        .collect::<Vec<_>>();
    if specs.is_empty() {
        return;
    }
    dshr_lsp::register_tools(registry, Arc::new(LspManager::new(root, specs)));
}

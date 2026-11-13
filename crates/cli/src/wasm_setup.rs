use std::path::Path;

use dshr_tools::ToolRegistry;

/// Registers tools from `*.yaml` WASM plugin manifests in the home and project
/// plugin directories.
pub fn register(registry: &mut ToolRegistry, home: &Path, cwd: &Path) {
    let dirs = vec![home.join("plugins"), cwd.join(".dshr/plugins")];
    dshr_wasm::register_tools(registry, &dirs);
}

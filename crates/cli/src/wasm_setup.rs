use std::path::Path;

use dshr_config::PluginsConfig;
use dshr_core::{DshrError, Result};
use std::sync::Arc;

use dshr_tools::ToolRegistry;

/// Registers tools from `*.yaml` WASM plugin manifests in the home and project
/// plugin directories.
/// Creates a hot-reload supervisor sharing `registry`.
pub fn manager(
    registry: ToolRegistry,
    home: &Path,
    cwd: &Path,
    plugins: &PluginsConfig,
) -> Result<Arc<dshr_wasm::WasmManager>> {
    let dirs = if crate::runtime_flags::safe_mode() {
        Vec::new()
    } else {
        vec![home.join("plugins"), cwd.join(".dshr/plugins")]
    };
    let policy = dshr_wasm::SignaturePolicy::new(&plugins.trusted_keys)
        .map_err(|error| DshrError::config(error.to_string()))?;
    Ok(dshr_wasm::WasmManager::new(registry, dirs, policy))
}

pub fn register(
    registry: &mut ToolRegistry,
    home: &Path,
    cwd: &Path,
    plugins: &PluginsConfig,
) -> Result<()> {
    if crate::runtime_flags::safe_mode() {
        tracing::warn!("safe mode: skipping WASM plugins");
        return Ok(());
    }
    let dirs = vec![home.join("plugins"), cwd.join(".dshr/plugins")];
    let policy = dshr_wasm::SignaturePolicy::new(&plugins.trusted_keys)
        .map_err(|error| DshrError::config(error.to_string()))?;
    dshr_wasm::register_tools(registry, &dirs, &policy);
    Ok(())
}

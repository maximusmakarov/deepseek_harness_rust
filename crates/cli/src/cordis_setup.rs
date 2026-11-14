use std::path::Path;

use dshr_config::Config;
use dshr_tools::ToolRegistry;

/// Enabled plugin specs for the current workspace, in config order.
pub fn specs(config: &Config) -> Vec<dshr_cordis::CordisPluginSpec> {
    let Ok(root) = std::env::current_dir() else {
        return Vec::new();
    };
    config
        .plugins
        .cordis
        .iter()
        .filter(|plugin| plugin.enabled)
        .map(|plugin| dshr_cordis::CordisPluginSpec {
            name: plugin.name.clone(),
            entry: plugin.entry.clone(),
            root: root.clone(),
            env: plugin.env.clone(),
            grants: plugin.grants.clone(),
        })
        .collect()
}

/// Connects configured Cordis/npm plugins through their node sidecars.
///
/// One-shot commands (run, TUI, ACP, prompt, schedule) mount plugins once; the
/// HTTP server uses [`dshr_cordis::CordisManager`] instead so plugins can be
/// reloaded while it runs.
pub async fn register(registry: &ToolRegistry, config: &Config, home: &Path) {
    if crate::runtime_flags::safe_mode() {
        tracing::warn!("safe mode: skipping Cordis plugins");
        return;
    }
    for spec in specs(config) {
        let name = spec.name.clone();
        match dshr_cordis::connect(spec, home).await {
            Ok(connection) => {
                // The connection is dropped here, but the tools keep the
                // backend alive for as long as they are registered.
                registry.register_all(connection.tools);
            }
            Err(error) => {
                tracing::warn!(plugin = %name, %error, "cordis plugin skipped");
            }
        }
    }
}

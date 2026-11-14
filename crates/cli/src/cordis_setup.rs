use std::path::Path;

use dshr_config::Config;
use dshr_tools::ToolRegistry;

/// Connects configured Cordis/npm plugins through their node sidecars.
pub async fn register(registry: &mut ToolRegistry, config: &Config, home: &Path) {
    if crate::runtime_flags::safe_mode() {
        tracing::warn!("safe mode: skipping Cordis plugins");
        return;
    }
    let Ok(root) = std::env::current_dir() else {
        return;
    };
    for plugin in config.plugins.cordis.iter().filter(|plugin| plugin.enabled) {
        let spec = dshr_cordis::CordisPluginSpec {
            name: plugin.name.clone(),
            entry: plugin.entry.clone(),
            root: root.clone(),
            env: plugin.env.clone(),
            grants: plugin.grants.clone(),
        };
        match dshr_cordis::connect(spec, home).await {
            Ok(tools) => {
                for tool in tools {
                    registry.register_arc(tool);
                }
            }
            Err(error) => {
                tracing::warn!(plugin = %plugin.name, %error, "cordis plugin skipped");
            }
        }
    }
}

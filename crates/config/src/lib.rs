#![forbid(unsafe_code)]
//! Layered configuration.
//!
//! Sources are applied bottom-up, each later layer deep-merging into the
//! previous one:
//!
//! ```text
//! defaults < profile (<home>/profiles/<name>/profile.yaml) < user (<home>/config.yaml) < patches
//! ```
//!
//! Maps merge recursively; scalars and arrays are replaced. Unknown keys are
//! rejected so that typos fail loudly instead of being silently ignored.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use dshr_core::{DshrError, Result};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const CONFIG_VERSION: u32 = 1;

/// Fully resolved configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    /// Schema version of the config document.
    pub version: u32,
    /// Profile selected for this run (informational after loading).
    pub profile: String,
    pub model: ModelConfig,
    pub session: SessionConfig,
    pub logging: LoggingConfig,
    pub tools: ToolsConfig,
    pub mcp: McpConfig,
    pub lsp: LspConfig,
    pub ui: UiConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: CONFIG_VERSION,
            profile: "default".to_owned(),
            model: ModelConfig::default(),
            session: SessionConfig::default(),
            logging: LoggingConfig::default(),
            tools: ToolsConfig::default(),
            mcp: McpConfig::default(),
            lsp: LspConfig::default(),
            ui: UiConfig::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct ModelConfig {
    /// Provider route, e.g. `deepseek-account` or `openai-compatible`.
    pub provider: String,
    /// Model id passed through to the provider.
    pub name: String,
    /// Optional endpoint override.
    pub base_url: Option<String>,
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            provider: "deepseek-account".to_owned(),
            name: "deepseek-flash".to_owned(),
            base_url: None,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct SessionConfig {
    /// Session store root. `None` resolves to `<home>/sessions`.
    pub dir: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct LoggingConfig {
    /// One of `trace`, `debug`, `info`, `warn`, `error`.
    pub level: String,
    pub format: LogFormat,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_owned(),
            format: LogFormat::Json,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct UiConfig {
    /// UI language: `ru` or `en`.
    pub language: String,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            language: "ru".to_owned(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    #[default]
    Json,
    Pretty,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct ToolsConfig {
    /// Upper bound for concurrently executing tool calls.
    pub max_parallel: usize,
    /// Largest tool output kept in model context before spilling to a file.
    pub output_limit_chars: usize,
    /// If non-empty, only these tools are exposed.
    pub allowed: Vec<String>,
    /// Tools removed even when allowed.
    pub denied: Vec<String>,
}

impl Default for ToolsConfig {
    fn default() -> Self {
        Self {
            max_parallel: 8,
            output_limit_chars: 30_000,
            allowed: Vec::new(),
            denied: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct McpConfig {
    pub servers: Vec<McpServerConfig>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct LspConfig {
    pub servers: Vec<LspServerConfig>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct LspServerConfig {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub extensions: Vec<String>,
    pub enabled: bool,
}

impl Default for LspServerConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            command: String::new(),
            args: Vec::new(),
            extensions: Vec::new(),
            enabled: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct McpServerConfig {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
    pub cwd: Option<PathBuf>,
    /// Streamable HTTP endpoint; when set, `command` is ignored.
    pub url: Option<String>,
    pub headers: BTreeMap<String, String>,
    /// Optional OAuth client-credentials flow for HTTP servers.
    pub oauth: Option<McpOAuthConfig>,
    pub enabled: bool,
}

impl Default for McpServerConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            command: String::new(),
            args: Vec::new(),
            env: BTreeMap::new(),
            cwd: None,
            url: None,
            headers: BTreeMap::new(),
            oauth: None,
            enabled: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct McpOAuthConfig {
    pub token_url: String,
    pub client_id: String,
    pub client_secret: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
}

impl Config {
    /// Validates invariants that cannot be expressed by serde alone.
    pub fn validate(&self) -> Result<()> {
        if self.version != CONFIG_VERSION {
            return Err(DshrError::config(format!(
                "unsupported config version {} (expected {CONFIG_VERSION})",
                self.version
            )));
        }
        if self.model.provider.trim().is_empty() {
            return Err(DshrError::config("model.provider must not be empty"));
        }
        if self.model.name.trim().is_empty() {
            return Err(DshrError::config("model.name must not be empty"));
        }
        if !(1..=64).contains(&self.tools.max_parallel) {
            return Err(DshrError::config(
                "tools.max_parallel must be between 1 and 64",
            ));
        }
        if !(1024..=1_000_000).contains(&self.tools.output_limit_chars) {
            return Err(DshrError::config(
                "tools.output_limit_chars must be between 1024 and 1000000",
            ));
        }
        if !matches!(
            self.logging.level.as_str(),
            "trace" | "debug" | "info" | "warn" | "error"
        ) {
            return Err(DshrError::config(
                "logging.level must be one of trace, debug, info, warn, error",
            ));
        }
        for server in self.lsp.servers.iter().filter(|server| server.enabled) {
            if server.name.trim().is_empty() {
                return Err(DshrError::config("lsp server name must not be empty"));
            }
            if server.command.trim().is_empty() {
                return Err(DshrError::config(format!(
                    "lsp server '{}' has an empty command",
                    server.name
                )));
            }
            if server.extensions.is_empty() {
                return Err(DshrError::config(format!(
                    "lsp server '{}' has no extensions",
                    server.name
                )));
            }
        }
        for server in self.mcp.servers.iter().filter(|server| server.enabled) {
            if server.name.trim().is_empty() {
                return Err(DshrError::config("mcp server name must not be empty"));
            }
            if server.url.is_none() && server.command.trim().is_empty() {
                return Err(DshrError::config(format!(
                    "mcp server '{}' needs either command or url",
                    server.name
                )));
            }
        }
        Ok(())
    }
}

/// JSON Schema for the config document, generated from the Rust types.
#[must_use]
pub fn json_schema() -> serde_json::Value {
    serde_json::to_value(schemars::schema_for!(Config)).expect("schema is serializable")
}

/// One layer in the resolution chain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Source {
    /// Human-readable layer name (`defaults`, `profile`, `user`, `patch:0`, ...).
    pub layer: String,
    pub path: Option<PathBuf>,
    pub found: bool,
}

/// Result of resolving every layer.
#[derive(Debug, Clone)]
pub struct LoadedConfig {
    pub config: Config,
    pub sources: Vec<Source>,
    /// Merged document before typed deserialization, useful for diagnostics.
    pub merged: Value,
}

/// Resolves configuration files for one home/profile pair.
#[derive(Debug, Clone)]
pub struct ConfigLoader {
    home: PathBuf,
    profile: String,
    patches: Vec<PathBuf>,
}

impl ConfigLoader {
    #[must_use]
    pub fn new(home: impl Into<PathBuf>, profile: impl Into<String>) -> Self {
        Self {
            home: home.into(),
            profile: profile.into(),
            patches: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_patches(mut self, patches: Vec<PathBuf>) -> Self {
        self.patches = patches;
        self
    }

    /// `$DSH_HOME` or `~/.dshr` when no explicit path is given.
    pub fn resolve_home(explicit: Option<PathBuf>) -> Result<PathBuf> {
        if let Some(home) = explicit {
            return Ok(home);
        }
        if let Some(home) = std::env::var_os("DSH_HOME")
            && !home.is_empty()
        {
            return Ok(PathBuf::from(home));
        }
        Self::default_home()
    }

    pub fn default_home() -> Result<PathBuf> {
        dirs::home_dir()
            .map(|home| home.join(".dshr"))
            .ok_or_else(|| DshrError::config("cannot determine the home directory"))
    }

    #[must_use]
    pub fn home(&self) -> &Path {
        &self.home
    }

    #[must_use]
    pub fn profile(&self) -> &str {
        &self.profile
    }

    #[must_use]
    pub fn profile_path(&self) -> PathBuf {
        self.home
            .join("profiles")
            .join(&self.profile)
            .join("profile.yaml")
    }

    #[must_use]
    pub fn user_path(&self) -> PathBuf {
        self.home.join("config.yaml")
    }

    /// Loads and validates the merged configuration.
    pub fn load(&self) -> Result<LoadedConfig> {
        let mut sources = vec![Source {
            layer: "defaults".to_owned(),
            path: None,
            found: true,
        }];
        let mut merged = serde_json::to_value(Config::default())
            .map_err(|err| DshrError::config(format!("serialize defaults: {err}")))?;

        let profile_path = self.profile_path();
        merge_file(
            &mut merged,
            &profile_path,
            format!("profile:{}", self.profile),
            false,
            &mut sources,
        )?;

        merge_file(
            &mut merged,
            &self.user_path(),
            "user".to_owned(),
            false,
            &mut sources,
        )?;

        for (index, patch) in self.patches.iter().enumerate() {
            merge_file(
                &mut merged,
                patch,
                format!("patch:{index}"),
                true,
                &mut sources,
            )?;
        }

        let mut config: Config = serde_json::from_value(merged.clone())
            .map_err(|err| DshrError::config(format!("invalid configuration document: {err}")))?;
        config.profile.clone_from(&self.profile);
        config.validate()?;

        let session_dir = config
            .session
            .dir
            .clone()
            .unwrap_or_else(|| self.home.join("sessions"));
        config.session.dir = Some(expand_path(&session_dir, &self.home)?);

        Ok(LoadedConfig {
            config,
            sources,
            merged,
        })
    }
}

fn merge_file(
    merged: &mut Value,
    path: &Path,
    layer: String,
    required: bool,
    sources: &mut Vec<Source>,
) -> Result<()> {
    if !path.exists() {
        if required {
            return Err(DshrError::config(format!(
                "patch file does not exist: {}",
                path.display()
            )));
        }
        sources.push(Source {
            layer,
            path: Some(path.to_path_buf()),
            found: false,
        });
        return Ok(());
    }

    let text = std::fs::read_to_string(path)?;
    let overlay: Value = serde_saphyr::from_str(&text)
        .map_err(|err| DshrError::config(format!("cannot parse {}: {err}", path.display())))?;
    merge(merged, overlay);
    sources.push(Source {
        layer,
        path: Some(path.to_path_buf()),
        found: true,
    });
    Ok(())
}

fn merge(base: &mut Value, overlay: Value) {
    match (base, overlay) {
        (Value::Object(base_map), Value::Object(overlay_map)) => {
            for (key, value) in overlay_map {
                match base_map.get_mut(&key) {
                    Some(existing) => merge(existing, value),
                    None => {
                        base_map.insert(key, value);
                    }
                }
            }
        }
        (base, overlay) => *base = overlay,
    }
}

fn expand_path(path: &Path, home: &Path) -> Result<PathBuf> {
    let raw = path.to_string_lossy();
    if raw == "~" {
        return dirs::home_dir()
            .ok_or_else(|| DshrError::config("cannot determine home directory"));
    }
    if let Some(rest) = raw.strip_prefix("~/") {
        let base =
            dirs::home_dir().ok_or_else(|| DshrError::config("cannot determine home directory"))?;
        return Ok(base.join(rest));
    }
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }
    Ok(home.join(path))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(path: &Path, text: &str) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("mkdir");
        }
        std::fs::write(path, text).expect("write");
    }

    #[test]
    fn defaults_without_files() {
        let dir = tempfile::tempdir().expect("tempdir");
        let loaded = ConfigLoader::new(dir.path(), "default")
            .load()
            .expect("load");
        assert_eq!(loaded.config.version, CONFIG_VERSION);
        assert_eq!(loaded.config.model.provider, "deepseek-account");
        assert_eq!(
            loaded.config.session.dir.as_deref(),
            Some(dir.path().join("sessions").as_path())
        );
        assert_eq!(loaded.sources.len(), 3);
        assert!(!loaded.sources[1].found);
    }

    #[test]
    fn profile_and_user_merge() {
        let dir = tempfile::tempdir().expect("tempdir");
        write(
            &dir.path().join("profiles/work/profile.yaml"),
            "model:\n  name: deepseek-v4-pro\nlogging:\n  level: debug\n",
        );
        write(
            &dir.path().join("config.yaml"),
            "model:\n  provider: openai-compatible\ntools:\n  max_parallel: 4\n",
        );

        let loaded = ConfigLoader::new(dir.path(), "work").load().expect("load");
        assert_eq!(loaded.config.model.name, "deepseek-v4-pro");
        assert_eq!(loaded.config.model.provider, "openai-compatible");
        assert_eq!(loaded.config.logging.level, "debug");
        assert_eq!(loaded.config.tools.max_parallel, 4);
        assert_eq!(loaded.config.profile, "work");
    }

    #[test]
    fn patch_wins_over_user() {
        let dir = tempfile::tempdir().expect("tempdir");
        write(
            &dir.path().join("config.yaml"),
            "tools:\n  max_parallel: 4\n",
        );
        let patch = dir.path().join("override.yaml");
        write(&patch, "tools:\n  max_parallel: 2\n");

        let loaded = ConfigLoader::new(dir.path(), "default")
            .with_patches(vec![patch])
            .load()
            .expect("load");
        assert_eq!(loaded.config.tools.max_parallel, 2);
    }

    #[test]
    fn missing_patch_fails() {
        let dir = tempfile::tempdir().expect("tempdir");
        let err = ConfigLoader::new(dir.path(), "default")
            .with_patches(vec![dir.path().join("absent.yaml")])
            .load()
            .expect_err("missing patch");
        assert_eq!(err.code(), "config_error");
    }

    #[test]
    fn unknown_keys_fail_loudly() {
        let dir = tempfile::tempdir().expect("tempdir");
        write(
            &dir.path().join("config.yaml"),
            "tools:\n  max_paralel: 4\n",
        );
        let err = ConfigLoader::new(dir.path(), "default")
            .load()
            .expect_err("unknown key");
        assert_eq!(err.code(), "config_error");
        assert!(err.to_string().contains("max_paralel"), "{err}");
    }

    #[test]
    fn wrong_version_is_rejected() {
        let dir = tempfile::tempdir().expect("tempdir");
        write(&dir.path().join("config.yaml"), "version: 2\n");
        let err = ConfigLoader::new(dir.path(), "default")
            .load()
            .expect_err("bad version");
        assert!(err.to_string().contains("unsupported config version"));
    }

    #[test]
    fn relative_session_dir_resolves_under_home() {
        let dir = tempfile::tempdir().expect("tempdir");
        write(
            &dir.path().join("config.yaml"),
            "session:\n  dir: state/sessions\n",
        );
        let loaded = ConfigLoader::new(dir.path(), "default")
            .load()
            .expect("load");
        assert_eq!(
            loaded.config.session.dir,
            Some(dir.path().join("state/sessions"))
        );
    }

    #[test]
    fn schema_has_properties() {
        let schema = json_schema();
        assert!(schema.get("properties").is_some(), "{schema}");
        assert!(schema["properties"].get("model").is_some(), "{schema}");
    }
}

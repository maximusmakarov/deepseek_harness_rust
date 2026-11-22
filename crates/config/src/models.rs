//! Provider profiles and model lists: the Models settings document.
//!
//! The web UI's Models section (a port of `dsh-client-ui-settings-models`)
//! edits per-provider profiles that the runtime reads at start-up. Upstream
//! keeps them in settings namespaces; dshr keeps the same facts in a small
//! durable document at `<home>/models.yaml`, separate from the layered
//! `config.yaml` so a profile edit never rewrites unrelated configuration.
//!
//! The active route and model stay in `config.yaml` (`model.*`); this document
//! only describes how each route is reached and which models it serves. A
//! route without a profile falls back to the built-in catalog below.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Schema version of `models.yaml`.
pub const MODELS_VERSION: u32 = 1;

/// File name under the dshr home.
pub const MODELS_FILE: &str = "models.yaml";

/// Wire protocol a profile speaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Protocol {
    /// Anthropic Messages (`/v1/messages`), used by the DeepSeek adapter.
    AnthropicMessages,
    /// OpenAI Chat Completions (`/chat/completions`).
    OpenAiChat,
}

impl Protocol {
    /// Stable wire identifier used by the API and the document.
    #[must_use]
    pub fn id(self) -> &'static str {
        match self {
            Self::AnthropicMessages => "anthropic-messages",
            Self::OpenAiChat => "openai-chat",
        }
    }

    /// Parses the wire identifier (also accepts the historical aliases).
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "anthropic-messages" | "anthropic" | "messages" => Some(Self::AnthropicMessages),
            "openai-chat" | "openai" | "openai-compatible" => Some(Self::OpenAiChat),
            _ => None,
        }
    }
}

impl fmt::Display for Protocol {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.id())
    }
}

/// One model in a provider's list.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ModelEntry {
    /// Provider model id passed through on the wire.
    pub id: String,
    /// Optional human-facing name.
    pub name: Option<String>,
    /// Context window in tokens.
    pub context_window: Option<u64>,
    /// Maximum output tokens.
    pub max_tokens: Option<u32>,
    /// Declared input types (`text`, `image`); empty means inherited.
    pub input: Vec<String>,
    /// Reasoning levels, mirroring upstream's `reasoningEfforts`: absent or
    /// `false` turns reasoning off, a map declares level -> API value.
    pub reasoning_efforts: Option<ReasoningEfforts>,
}

/// Reasoning levels a model may declare.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ReasoningEfforts {
    /// `false`: the model does not reason.
    Disabled(bool),
    /// Level -> value sent on the wire, in the user's chosen order.
    Levels(BTreeMap<String, String>),
}

/// Reasoning levels the adapters accept, in escalation order.
pub const REASONING_LEVELS: [&str; 7] = ["off", "minimal", "low", "medium", "high", "xhigh", "max"];

impl ReasoningEfforts {
    /// Whether this declaration accepts no level at all.
    #[must_use]
    pub fn is_disabled(&self) -> bool {
        match self {
            Self::Disabled(enabled) => !enabled,
            Self::Levels(levels) => levels.is_empty(),
        }
    }

    /// Declared levels in escalation order (not the map's alphabetical order).
    #[must_use]
    pub fn levels(&self) -> Vec<&str> {
        match self {
            Self::Disabled(_) => Vec::new(),
            Self::Levels(levels) => REASONING_LEVELS
                .iter()
                .copied()
                .filter(|level| levels.contains_key(*level))
                .collect(),
        }
    }

    /// The value this declaration sends for `level`.
    #[must_use]
    pub fn wire_value(&self, level: &str) -> Option<&str> {
        match self {
            Self::Disabled(_) => None,
            Self::Levels(levels) => levels.get(level).map(String::as_str),
        }
    }
}

impl ModelEntry {
    /// Levels this model declares, in escalation order; empty means the model
    /// reasons with the provider's own default and offers no choice.
    #[must_use]
    pub fn reasoning_levels(&self) -> Vec<&str> {
        self.reasoning_efforts
            .as_ref()
            .map(ReasoningEfforts::levels)
            .unwrap_or_default()
    }

    /// Resolves the selected level to the value sent on the wire.
    ///
    /// `None` means "no selection": the provider's own default applies and the
    /// adapters omit the parameter. An unsupported level is an error rather
    /// than a clamp, mirroring upstream's `UNSUPPORTED_REASONING_EFFORT`.
    pub fn reasoning_effort_value(
        &self,
        level: Option<&str>,
    ) -> Result<Option<String>, ModelsError> {
        let Some(level) = level else {
            return Ok(None);
        };
        let Some(efforts) = self.reasoning_efforts.as_ref() else {
            return Err(ModelsError::Invalid(format!(
                "model '{}' declares no reasoning levels",
                self.id
            )));
        };
        match efforts.wire_value(level) {
            Some(value) => Ok(Some(value.to_owned())),
            None => Err(ModelsError::Invalid(format!(
                "model '{}' does not declare reasoning level '{level}'",
                self.id
            ))),
        }
    }
}

/// One provider profile stored in `models.yaml`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ProviderProfile {
    /// Protocol override; required for a route outside the catalog.
    pub protocol: Option<Protocol>,
    /// Endpoint override; `None` inherits the catalog/env value.
    pub base_url: Option<String>,
    /// Environment variable that carries the API key; `None` inherits the
    /// catalog default.
    pub api_key_env: Option<String>,
    /// Model list; empty means the catalog defaults apply.
    pub models: Vec<ModelEntry>,
    /// Request-retry policy; `None` inherits the adapter defaults.
    pub retry: Option<RetryPolicy>,
}

/// Retry modes, mirroring DSH's `resolveRetryPolicy`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RetryMode {
    /// Retry only the configured failure codes, at most `max_retries` times.
    #[default]
    Normal,
    /// Retry every model-request failure until success or cancellation.
    Always,
}

/// Bounded exponential backoff and symmetric jitter.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RetryBackoff {
    /// First local delay in milliseconds (default 500).
    pub initial_delay_ms: Option<u64>,
    /// Delay ceiling in milliseconds (default 10000).
    pub max_delay_ms: Option<u64>,
    /// Symmetric random multiplier range around one (default 0.1).
    pub jitter_ratio: Option<f64>,
}

/// A provider's stored retry policy. Empty fields keep the adapter default.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RetryPolicy {
    pub mode: RetryMode,
    /// Maximum eligible retries after the first request (default 5).
    pub max_retries: Option<u32>,
    /// Stable failure codes eligible under normal mode (default: the five
    /// transient codes).
    pub retryable_codes: Option<Vec<String>>,
    pub backoff: Option<RetryBackoff>,
}

/// A policy with every default applied, ready for the runtime.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedRetryPolicy {
    pub mode: RetryMode,
    pub max_retries: u32,
    pub retryable_codes: Vec<String>,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub jitter_ratio: f64,
}

/// Adapter defaults, mirrored from `dshr_llm::retry`.
pub const DEFAULT_RETRY_MAX: u32 = 5;
pub const DEFAULT_RETRY_INITIAL_DELAY_MS: u64 = 500;
pub const DEFAULT_RETRY_MAX_DELAY_MS: u64 = 10_000;
pub const DEFAULT_RETRY_JITTER_RATIO: f64 = 0.1;
pub const MAX_RETRY_DELAY_MS: u64 = 2_147_483_647;
pub const DEFAULT_RETRYABLE_CODES: [&str; 5] = [
    "EMPTY_RESPONSE",
    "RATE_LIMIT",
    "SERVER",
    "TIMEOUT",
    "TRANSPORT",
];

impl Default for ResolvedRetryPolicy {
    fn default() -> Self {
        Self {
            mode: RetryMode::Normal,
            max_retries: DEFAULT_RETRY_MAX,
            retryable_codes: DEFAULT_RETRYABLE_CODES
                .iter()
                .map(|code| (*code).to_owned())
                .collect(),
            initial_delay_ms: DEFAULT_RETRY_INITIAL_DELAY_MS,
            max_delay_ms: DEFAULT_RETRY_MAX_DELAY_MS,
            jitter_ratio: DEFAULT_RETRY_JITTER_RATIO,
        }
    }
}

impl RetryPolicy {
    /// Applies defaults without validating; callers validate first.
    #[must_use]
    pub fn resolved(&self) -> ResolvedRetryPolicy {
        let backoff = self.backoff.clone().unwrap_or_default();
        ResolvedRetryPolicy {
            mode: self.mode,
            max_retries: self.max_retries.unwrap_or(DEFAULT_RETRY_MAX),
            retryable_codes: self.retryable_codes.clone().unwrap_or_else(|| {
                DEFAULT_RETRYABLE_CODES
                    .iter()
                    .map(|code| (*code).to_owned())
                    .collect()
            }),
            initial_delay_ms: backoff
                .initial_delay_ms
                .unwrap_or(DEFAULT_RETRY_INITIAL_DELAY_MS),
            max_delay_ms: backoff.max_delay_ms.unwrap_or(DEFAULT_RETRY_MAX_DELAY_MS),
            jitter_ratio: backoff.jitter_ratio.unwrap_or(DEFAULT_RETRY_JITTER_RATIO),
        }
    }
}

/// The Models settings document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ModelsDocument {
    /// Schema version.
    pub version: u32,
    /// Monotonic revision, bumped by every write and checked by the next one.
    pub revision: u64,
    /// Provider profiles keyed by route id.
    pub providers: BTreeMap<String, ProviderProfile>,
    /// Highest first-run notice version the user acknowledged.
    pub welcome_version: Option<u32>,
}

impl Default for ModelsDocument {
    fn default() -> Self {
        Self {
            version: MODELS_VERSION,
            revision: 0,
            providers: BTreeMap::new(),
            welcome_version: None,
        }
    }
}

/// A catalog model shipped as a provider default.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CatalogModel {
    pub id: &'static str,
    pub name: &'static str,
    /// Levels the model offers before the user edits the profile, in
    /// escalation order; empty means the provider's own default applies and
    /// the model selector offers no choice.
    pub reasoning_levels: &'static [&'static str],
}

/// One built-in provider the Models page can adopt.
#[derive(Debug, Clone, Copy)]
pub struct CatalogProvider {
    /// Route id written to `config.yaml` and used as the profile key.
    pub id: &'static str,
    /// Human-facing name.
    pub display_name: &'static str,
    /// Wire protocol the adapter speaks for this route.
    pub protocol: Protocol,
    /// Endpoint used when neither the profile nor the environment sets one.
    pub default_base_url: Option<&'static str>,
    /// Environment variables that may override the endpoint.
    pub base_url_env: &'static [&'static str],
    /// Environment variable that carries the key when the profile names none.
    pub api_key_env: Option<&'static str>,
    /// Whether this route authenticates through the DeepSeek account grant.
    pub account: bool,
    /// One-line explanation shown by the add card.
    pub summary: &'static str,
    /// Models offered before the user edits the list.
    pub models: &'static [CatalogModel],
}

/// Levels the DeepSeek Messages endpoint accepts, in escalation order; its
/// models offer them without any profile configuration.
const DEEPSEEK_REASONING_LEVELS: [&str; 4] = ["off", "low", "high", "max"];

const DEEPSEEK_MODELS: [CatalogModel; 2] = [
    CatalogModel {
        id: "deepseek-flash",
        name: "DeepSeek Flash",
        reasoning_levels: &DEEPSEEK_REASONING_LEVELS,
    },
    CatalogModel {
        id: "deepseek-v4-pro",
        name: "DeepSeek V4 Pro",
        reasoning_levels: &DEEPSEEK_REASONING_LEVELS,
    },
];

const OPENAI_MODELS: [CatalogModel; 2] = [
    CatalogModel {
        id: "gpt-4o-mini",
        name: "GPT-4o mini",
        reasoning_levels: &[],
    },
    CatalogModel {
        id: "gpt-4o",
        name: "GPT-4o",
        reasoning_levels: &[],
    },
];

/// Built-in provider directory. Aliases (`deepseek`) resolve to their
/// canonical entry; every id here is also a route the CLI can build.
pub const CATALOG: [CatalogProvider; 5] = [
    CatalogProvider {
        id: "deepseek-account",
        display_name: "DeepSeek Account",
        protocol: Protocol::AnthropicMessages,
        default_base_url: Some("https://api.deepseek.com/anthropic"),
        base_url_env: &["DEEPSEEK_BASE_URL"],
        api_key_env: Some("DSH_DEEPSEEK_TOKEN"),
        account: true,
        summary: "Uses the DeepSeek account grant or a DeepSeek API key from the environment",
        models: &DEEPSEEK_MODELS,
    },
    CatalogProvider {
        id: "deepseek-official",
        display_name: "DeepSeek API",
        protocol: Protocol::AnthropicMessages,
        default_base_url: Some("https://api.deepseek.com/anthropic"),
        base_url_env: &["DEEPSEEK_BASE_URL"],
        api_key_env: Some("DEEPSEEK_API_KEY"),
        account: false,
        summary: "A DeepSeek API key from platform.deepseek.com",
        models: &DEEPSEEK_MODELS,
    },
    CatalogProvider {
        id: "openai",
        display_name: "OpenAI",
        protocol: Protocol::OpenAiChat,
        default_base_url: Some("https://api.openai.com/v1"),
        base_url_env: &["OPENAI_BASE_URL"],
        api_key_env: Some("OPENAI_API_KEY"),
        account: false,
        summary: "The official OpenAI Chat Completions endpoint",
        models: &OPENAI_MODELS,
    },
    CatalogProvider {
        id: "openai-compatible",
        display_name: "OpenAI-compatible gateway",
        protocol: Protocol::OpenAiChat,
        default_base_url: None,
        base_url_env: &["OPENAI_BASE_URL"],
        api_key_env: Some("OPENAI_API_KEY"),
        account: false,
        summary: "A self-hosted or third-party /chat/completions endpoint",
        models: &[],
    },
    CatalogProvider {
        id: "routerai",
        display_name: "RouterAI",
        protocol: Protocol::OpenAiChat,
        default_base_url: Some("https://routerai.ru/api/v1"),
        base_url_env: &["OPENAI_BASE_URL"],
        api_key_env: Some("ROUTERAI_API_KEY"),
        account: false,
        summary: "RouterAI's OpenAI-compatible gateway",
        models: &[],
    },
];

/// Resolves a route id, including the `deepseek` alias, to its catalog entry.
#[must_use]
pub fn catalog_entry(route: &str) -> Option<&'static CatalogProvider> {
    let canonical = if route == "deepseek" {
        "deepseek-account"
    } else {
        route
    };
    CATALOG.iter().find(|entry| entry.id == canonical)
}

/// Whether a route is one the catalog (or the provider factory) knows.
#[must_use]
pub fn known_route(route: &str) -> bool {
    catalog_entry(route).is_some()
}

/// Catalog levels as a declaration: absent when the catalog lists none.
fn catalog_efforts(levels: &'static [&'static str]) -> Option<ReasoningEfforts> {
    if levels.is_empty() {
        return None;
    }
    Some(ReasoningEfforts::Levels(
        levels
            .iter()
            .map(|level| ((*level).to_owned(), (*level).to_owned()))
            .collect(),
    ))
}

/// Catalog defaults as document entries.
#[must_use]
pub fn catalog_models(route: &str) -> Vec<ModelEntry> {
    catalog_entry(route)
        .map(|entry| {
            entry
                .models
                .iter()
                .map(|model| ModelEntry {
                    id: model.id.to_owned(),
                    name: Some(model.name.to_owned()),
                    reasoning_efforts: catalog_efforts(model.reasoning_levels),
                    ..ModelEntry::default()
                })
                .collect()
        })
        .unwrap_or_default()
}

/// A provider as resolved for the API and the runtime.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedProvider {
    pub route: String,
    pub display_name: String,
    pub protocol: Protocol,
    /// Explicit endpoint from the profile, else the config layer.
    pub base_url: Option<String>,
    /// Endpoint in effect once the environment/defaults are considered.
    pub effective_base_url: Option<String>,
    pub api_key_env: Option<String>,
    /// Effective model list (profile, else catalog defaults).
    pub models: Vec<ModelEntry>,
    /// Whether the list came from a profile rather than the catalog.
    pub models_from_profile: bool,
    pub account: bool,
    pub summary: String,
    /// The provider's retry policy with defaults applied.
    pub retry: ResolvedRetryPolicy,
}

impl ModelsDocument {
    /// Path of the document under a dshr home.
    #[must_use]
    pub fn path(home: &Path) -> PathBuf {
        home.join(MODELS_FILE)
    }

    /// Loads the document; a missing file is the default (empty) document.
    pub fn load(home: &Path) -> Result<Self, ModelsError> {
        let path = Self::path(home);
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::default());
            }
            Err(error) => return Err(ModelsError::Io(format!("{}: {error}", path.display()))),
        };
        let document: Self = serde_saphyr::from_str(&text)
            .map_err(|error| ModelsError::Parse(format!("{}: {error}", path.display())))?;
        document.validate()?;
        Ok(document)
    }

    /// Writes the document atomically with `0600` permissions and bumps the
    /// revision when `expected` matches the stored one.
    pub fn save(&self, home: &Path, expected: Option<u64>) -> Result<Self, ModelsError> {
        let mut next = self.clone();
        next.validate()?;
        if let Some(expected) = expected
            && expected != self.revision
        {
            return Err(ModelsError::Conflict {
                expected,
                actual: self.revision,
            });
        }
        next.revision = self.revision.saturating_add(1);
        next.write(home)?;
        Ok(next)
    }

    /// Loads, mutates and saves under the revision fence in one call.
    pub fn update(
        home: &Path,
        expected: Option<u64>,
        mutate: impl FnOnce(&mut Self),
    ) -> Result<Self, ModelsError> {
        let mut document = Self::load(home)?;
        mutate(&mut document);
        document.save(home, expected)
    }

    fn write(&self, home: &Path) -> Result<(), ModelsError> {
        std::fs::create_dir_all(home)
            .map_err(|error| ModelsError::Io(format!("{}: {error}", home.display())))?;
        let path = Self::path(home);
        let text = serde_saphyr::to_string(self)
            .map_err(|error| ModelsError::Io(format!("serialize models: {error}")))?;
        let temp = path.with_extension("yaml.tmp");
        let mut file = std::fs::File::create(&temp)
            .map_err(|error| ModelsError::Io(format!("{}: {error}", temp.display())))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            file.set_permissions(std::fs::Permissions::from_mode(0o600))
                .map_err(|error| ModelsError::Io(format!("{}: {error}", temp.display())))?;
        }
        file.write_all(text.as_bytes())
            .map_err(|error| ModelsError::Io(format!("{}: {error}", temp.display())))?;
        file.sync_all()
            .map_err(|error| ModelsError::Io(format!("{}: {error}", temp.display())))?;
        drop(file);
        std::fs::rename(&temp, &path)
            .map_err(|error| ModelsError::Io(format!("{}: {error}", path.display())))?;
        Ok(())
    }

    /// Validates the document's invariants before it is written or used.
    pub fn validate(&self) -> Result<(), ModelsError> {
        if self.version != MODELS_VERSION {
            return Err(ModelsError::Invalid(format!(
                "unsupported models version {} (expected {MODELS_VERSION})",
                self.version
            )));
        }
        for (route, profile) in &self.providers {
            validate_route(route)?;
            if !known_route(route) && profile.protocol.is_none() {
                return Err(ModelsError::Invalid(format!(
                    "provider '{route}' is not in the catalog and declares no protocol"
                )));
            }
            if let Some(base_url) = profile.base_url.as_deref()
                && !valid_base_url(base_url)
            {
                return Err(ModelsError::Invalid(format!(
                    "provider '{route}' has an invalid base URL"
                )));
            }
            if let Some(reference) = profile.api_key_env.as_deref()
                && !valid_env_name(reference)
            {
                return Err(ModelsError::Invalid(format!(
                    "provider '{route}' has an invalid API key variable name"
                )));
            }
            if let Some(retry) = &profile.retry {
                validate_retry(route, retry)?;
            }
            if profile.models.len() > 200 {
                return Err(ModelsError::Invalid(format!(
                    "provider '{route}' lists more than 200 models"
                )));
            }
            let mut ids = BTreeSet::new();
            for model in &profile.models {
                validate_model(model)?;
                if !ids.insert(model.id.as_str()) {
                    return Err(ModelsError::Invalid(format!(
                        "provider '{route}' repeats model '{}'",
                        model.id
                    )));
                }
            }
        }
        Ok(())
    }

    /// Resolves one route: profile fields over catalog defaults, with the
    /// active `config.yaml` endpoint as the last explicit value.
    #[must_use]
    pub fn resolve(&self, route: &str, config_base_url: Option<&str>) -> Option<ResolvedProvider> {
        let entry = catalog_entry(route);
        let profile = self.providers.get(route);
        let protocol = profile
            .and_then(|profile| profile.protocol)
            .or_else(|| entry.map(|entry| entry.protocol))?;
        let explicit_base = profile
            .and_then(|profile| profile.base_url.clone())
            .filter(|value| !value.trim().is_empty())
            .or_else(|| {
                config_base_url
                    .map(str::to_owned)
                    .filter(|value| !value.is_empty())
            });
        let env_base = entry.and_then(|entry| {
            entry
                .base_url_env
                .iter()
                .find_map(|name| std::env::var(name).ok())
                .filter(|value| !value.trim().is_empty())
        });
        let default_base = entry.and_then(|entry| entry.default_base_url.map(str::to_owned));
        let effective_base = explicit_base.clone().or(env_base).or(default_base);
        let api_key_env = profile
            .and_then(|profile| profile.api_key_env.clone())
            .filter(|value| !value.trim().is_empty())
            .or_else(|| entry.and_then(|entry| entry.api_key_env.map(str::to_owned)));
        let (models, models_from_profile) = match profile {
            Some(profile) if !profile.models.is_empty() => (profile.models.clone(), true),
            _ => (catalog_models(route), false),
        };
        Some(ResolvedProvider {
            route: route.to_owned(),
            display_name: entry
                .map(|entry| entry.display_name.to_owned())
                .unwrap_or_else(|| route.to_owned()),
            protocol,
            base_url: explicit_base,
            effective_base_url: effective_base,
            api_key_env,
            models,
            models_from_profile,
            account: entry.is_some_and(|entry| entry.account),
            summary: entry
                .map(|entry| entry.summary.to_owned())
                .unwrap_or_default(),
            retry: profile
                .and_then(|profile| profile.retry.as_ref())
                .map_or_else(ResolvedRetryPolicy::default, RetryPolicy::resolved),
        })
    }

    /// Every route the Models page can show: configured profiles first, then
    /// the active route even when it has no profile.
    #[must_use]
    pub fn visible_routes(&self, active: Option<&str>) -> Vec<String> {
        let mut routes: Vec<String> = self.providers.keys().cloned().collect();
        if let Some(active) = active
            && !routes.iter().any(|route| route == active)
        {
            routes.push(active.to_owned());
        }
        routes
    }
}

fn validate_route(route: &str) -> Result<(), ModelsError> {
    if route.is_empty()
        || route.len() > 64
        || !route
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
        || route.starts_with('-')
    {
        return Err(ModelsError::Invalid(format!(
            "invalid provider route '{route}'"
        )));
    }
    Ok(())
}

fn validate_retry(route: &str, retry: &RetryPolicy) -> Result<(), ModelsError> {
    if let Some(codes) = &retry.retryable_codes {
        if codes.is_empty() {
            return Err(ModelsError::Invalid(format!(
                "provider '{route}' retry policy lists no retryable codes"
            )));
        }
        let mut seen = BTreeSet::new();
        for code in codes {
            if code.is_empty()
                || code.len() > 64
                || code.chars().any(|ch| ch.is_control() || ch.is_whitespace())
            {
                return Err(ModelsError::Invalid(format!(
                    "provider '{route}' has an invalid retry code '{code}'"
                )));
            }
            if !seen.insert(code.as_str()) {
                return Err(ModelsError::Invalid(format!(
                    "provider '{route}' repeats retry code '{code}'"
                )));
            }
        }
    }
    let resolved = retry.resolved();
    if resolved.initial_delay_ms == 0 || resolved.initial_delay_ms > MAX_RETRY_DELAY_MS {
        return Err(ModelsError::Invalid(format!(
            "provider '{route}' has an invalid retry initial delay"
        )));
    }
    if resolved.max_delay_ms == 0 || resolved.max_delay_ms > MAX_RETRY_DELAY_MS {
        return Err(ModelsError::Invalid(format!(
            "provider '{route}' has an invalid retry maximum delay"
        )));
    }
    if resolved.initial_delay_ms > resolved.max_delay_ms {
        return Err(ModelsError::Invalid(format!(
            "provider '{route}' retry initial delay exceeds the maximum delay"
        )));
    }
    if !resolved.jitter_ratio.is_finite() || !(0.0..=1.0).contains(&resolved.jitter_ratio) {
        return Err(ModelsError::Invalid(format!(
            "provider '{route}' retry jitter must be between 0 and 1"
        )));
    }
    Ok(())
}

fn validate_model(model: &ModelEntry) -> Result<(), ModelsError> {
    if model.id.is_empty()
        || model.id.len() > 128
        || model
            .id
            .chars()
            .any(|ch| ch.is_control() || ch.is_whitespace())
    {
        return Err(ModelsError::Invalid(
            "a model id must be 1-128 characters without whitespace".to_owned(),
        ));
    }
    if let Some(name) = model.name.as_deref()
        && (name.trim().is_empty() || name.len() > 200)
    {
        return Err(ModelsError::Invalid(format!(
            "model '{}' has an empty or oversized name",
            model.id
        )));
    }
    if model.context_window == Some(0) {
        return Err(ModelsError::Invalid(format!(
            "model '{}' has a zero context window",
            model.id
        )));
    }
    if model.max_tokens == Some(0) {
        return Err(ModelsError::Invalid(format!(
            "model '{}' has a zero output limit",
            model.id
        )));
    }
    for input in &model.input {
        if !matches!(input.as_str(), "text" | "image") {
            return Err(ModelsError::Invalid(format!(
                "model '{}' names an unknown input type '{input}'",
                model.id
            )));
        }
    }
    if let Some(efforts) = &model.reasoning_efforts {
        match efforts {
            ReasoningEfforts::Disabled(false) => {}
            ReasoningEfforts::Disabled(true) => {
                return Err(ModelsError::Invalid(format!(
                    "model '{}' declares reasoning without any level",
                    model.id
                )));
            }
            ReasoningEfforts::Levels(levels) => {
                if levels.is_empty() {
                    return Err(ModelsError::Invalid(format!(
                        "model '{}' declares reasoning without any level",
                        model.id
                    )));
                }
                for (level, value) in levels {
                    if !REASONING_LEVELS.contains(&level.as_str()) {
                        return Err(ModelsError::Invalid(format!(
                            "model '{}' names an unknown reasoning level '{level}'",
                            model.id
                        )));
                    }
                    if value.trim().is_empty() {
                        return Err(ModelsError::Invalid(format!(
                            "model '{}' has an empty value for reasoning level '{level}'",
                            model.id
                        )));
                    }
                }
            }
        }
    }
    Ok(())
}

fn valid_base_url(value: &str) -> bool {
    value.len() <= 2048
        && (value.starts_with("https://") || value.starts_with("http://"))
        && !value.chars().any(char::is_whitespace)
}

fn valid_env_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 96
        && value
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_uppercase())
        && value
            .chars()
            .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_')
}

/// Failures of the Models document or its validation.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ModelsError {
    #[error("models document: {0}")]
    Io(String),
    #[error("models document: {0}")]
    Parse(String),
    #[error("models document: {0}")]
    Invalid(String),
    #[error("models revision conflict: expected {expected}, actual {actual}")]
    Conflict { expected: u64, actual: u64 },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn home() -> tempfile::TempDir {
        tempfile::tempdir().expect("tempdir")
    }

    #[test]
    fn catalog_ids_are_unique_and_known() {
        let mut ids = BTreeSet::new();
        for entry in &CATALOG {
            assert!(ids.insert(entry.id), "duplicate {}", entry.id);
            assert!(known_route(entry.id));
            assert!(!entry.display_name.is_empty());
            assert!(entry.api_key_env.is_some() || entry.account);
            for model in entry.models {
                assert!(!model.id.is_empty());
                assert!(!model.name.is_empty());
                // A catalog level is its own wire value and stays in the
                // escalation order the selector renders; no level means no
                // declaration at all, so the model offers no choice.
                match catalog_efforts(model.reasoning_levels) {
                    Some(declaration) => {
                        assert_eq!(
                            declaration.levels(),
                            model.reasoning_levels,
                            "{} levels out of order",
                            model.id
                        );
                        for level in model.reasoning_levels {
                            assert_eq!(declaration.wire_value(level), Some(*level));
                        }
                    }
                    None => assert!(
                        model.reasoning_levels.is_empty(),
                        "{} declares levels",
                        model.id
                    ),
                }
            }
        }
        assert!(known_route("deepseek"));
        assert_eq!(
            catalog_entry("deepseek").map(|entry| entry.id),
            Some("deepseek-account")
        );
        assert!(!known_route("no-such-route"));
    }

    #[test]
    fn a_missing_document_is_empty_and_saving_bumps_the_revision() {
        let home = home();
        let document = ModelsDocument::load(home.path()).expect("load");
        assert_eq!(document, ModelsDocument::default());
        assert_eq!(document.revision, 0);

        let saved = ModelsDocument::update(home.path(), Some(0), |document| {
            document.providers.insert(
                "deepseek-official".into(),
                ProviderProfile {
                    base_url: Some("https://gateway.example".into()),
                    api_key_env: Some("MY_DEEPSEEK_KEY".into()),
                    models: vec![ModelEntry {
                        id: "deepseek-flash".into(),
                        name: Some("Flash".into()),
                        context_window: Some(1_000_000),
                        max_tokens: Some(64_000),
                        input: vec!["text".into(), "image".into()],
                        reasoning_efforts: Some(ReasoningEfforts::Levels(BTreeMap::from([
                            ("off".into(), "off".into()),
                            ("high".into(), "high".into()),
                        ]))),
                    }],
                    ..ProviderProfile::default()
                },
            );
        })
        .expect("save");
        assert_eq!(saved.revision, 1);

        let loaded = ModelsDocument::load(home.path()).expect("load");
        assert_eq!(loaded, saved);
        let profile = &loaded.providers["deepseek-official"];
        assert_eq!(profile.base_url.as_deref(), Some("https://gateway.example"));
        assert_eq!(profile.models[0].context_window, Some(1_000_000));

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(ModelsDocument::path(home.path()))
                .expect("metadata")
                .permissions()
                .mode()
                & 0o777;
            assert_eq!(mode, 0o600);
        }
    }

    #[test]
    fn a_stale_revision_is_refused() {
        let home = home();
        ModelsDocument::update(home.path(), Some(0), |_| {}).expect("first write");
        let error = ModelsDocument::update(home.path(), Some(0), |_| {}).expect_err("stale");
        assert_eq!(
            error,
            ModelsError::Conflict {
                expected: 0,
                actual: 1
            }
        );
    }

    #[test]
    fn invalid_profiles_are_refused() {
        let mut document = ModelsDocument::default();
        document.providers.insert(
            "deepseek-official".into(),
            ProviderProfile {
                base_url: Some("ftp://example".into()),
                ..ProviderProfile::default()
            },
        );
        assert!(matches!(
            document.validate(),
            Err(ModelsError::Invalid(message)) if message.contains("base URL")
        ));

        document.providers.clear();
        document.providers.insert(
            "deepseek-official".into(),
            ProviderProfile {
                models: vec![
                    ModelEntry {
                        id: "same".into(),
                        ..ModelEntry::default()
                    },
                    ModelEntry {
                        id: "same".into(),
                        ..ModelEntry::default()
                    },
                ],
                ..ProviderProfile::default()
            },
        );
        assert!(matches!(
            document.validate(),
            Err(ModelsError::Invalid(message)) if message.contains("repeats model")
        ));

        document.providers.clear();
        document
            .providers
            .insert("custom-gateway".into(), ProviderProfile::default());
        assert!(matches!(
            document.validate(),
            Err(ModelsError::Invalid(message)) if message.contains("declares no protocol")
        ));
    }

    #[test]
    fn retry_policies_resolve_and_validate() {
        let profile = RetryPolicy {
            mode: RetryMode::Normal,
            max_retries: Some(7),
            retryable_codes: Some(vec!["RATE_LIMIT".into(), "MY_CODE".into()]),
            backoff: Some(RetryBackoff {
                initial_delay_ms: Some(250),
                max_delay_ms: Some(2_000),
                jitter_ratio: Some(0.25),
            }),
        };
        let resolved = profile.resolved();
        assert_eq!(resolved.max_retries, 7);
        assert_eq!(resolved.initial_delay_ms, 250);
        assert_eq!(resolved.jitter_ratio, 0.25);
        assert_eq!(resolved.retryable_codes.len(), 2);

        let defaults = RetryPolicy::default().resolved();
        assert_eq!(defaults.mode, RetryMode::Normal);
        assert_eq!(defaults.max_retries, 5);
        assert_eq!(defaults.initial_delay_ms, 500);
        assert_eq!(defaults.max_delay_ms, 10_000);
        assert_eq!(defaults.retryable_codes.len(), 5);

        let mut document = ModelsDocument::default();
        let mut provider = ProviderProfile {
            retry: Some(profile),
            ..ProviderProfile::default()
        };
        document
            .providers
            .insert("deepseek-official".into(), provider.clone());
        document.validate().expect("valid retry policy");

        provider.retry = Some(RetryPolicy {
            retryable_codes: Some(Vec::new()),
            ..RetryPolicy::default()
        });
        document
            .providers
            .insert("deepseek-official".into(), provider);
        assert!(matches!(
            document.validate(),
            Err(ModelsError::Invalid(message)) if message.contains("no retryable codes")
        ));

        let reversed = ProviderProfile {
            retry: Some(RetryPolicy {
                backoff: Some(RetryBackoff {
                    initial_delay_ms: Some(5_000),
                    max_delay_ms: Some(1_000),
                    jitter_ratio: None,
                }),
                ..RetryPolicy::default()
            }),
            ..ProviderProfile::default()
        };
        document
            .providers
            .insert("deepseek-official".into(), reversed);
        assert!(matches!(
            document.validate(),
            Err(ModelsError::Invalid(message)) if message.contains("exceeds the maximum")
        ));
    }

    #[test]
    fn resolve_prefers_profile_then_config_then_catalog() {
        let mut document = ModelsDocument::default();
        let resolved = document
            .resolve("deepseek-account", None)
            .expect("catalog route");
        assert_eq!(resolved.protocol, Protocol::AnthropicMessages);
        assert_eq!(
            resolved.effective_base_url.as_deref(),
            Some("https://api.deepseek.com/anthropic")
        );
        assert!(!resolved.models_from_profile);
        assert_eq!(resolved.models[0].id, "deepseek-flash");
        // The DeepSeek endpoint's models offer their levels with no profile.
        assert_eq!(
            resolved.models[0].reasoning_levels(),
            ["off", "low", "high", "max"]
        );
        assert_eq!(
            resolved.models[0]
                .reasoning_effort_value(Some("max"))
                .expect("declared level"),
            Some("max".to_owned())
        );

        document.providers.insert(
            "deepseek-account".into(),
            ProviderProfile {
                base_url: Some("https://proxy.example/anthropic".into()),
                models: vec![ModelEntry {
                    id: "custom-model".into(),
                    ..ModelEntry::default()
                }],
                ..ProviderProfile::default()
            },
        );
        let resolved = document
            .resolve("deepseek-account", Some("https://ignored.example"))
            .expect("profile route");
        assert_eq!(
            resolved.effective_base_url.as_deref(),
            Some("https://proxy.example/anthropic")
        );
        assert!(resolved.models_from_profile);
        assert_eq!(resolved.models[0].id, "custom-model");
    }

    #[test]
    fn custom_routes_need_a_protocol() {
        let mut document = ModelsDocument::default();
        document.providers.insert(
            "my-gateway".into(),
            ProviderProfile {
                protocol: Some(Protocol::OpenAiChat),
                base_url: Some("https://gateway.example/v1".into()),
                ..ProviderProfile::default()
            },
        );
        document.validate().expect("valid custom route");
        let resolved = document.resolve("my-gateway", None).expect("resolve");
        assert_eq!(resolved.protocol, Protocol::OpenAiChat);
        assert_eq!(
            resolved.effective_base_url.as_deref(),
            Some("https://gateway.example/v1")
        );
    }

    #[test]
    fn declared_reasoning_levels_resolve_in_escalation_order() {
        let model = ModelEntry {
            id: "gpt-6".into(),
            reasoning_efforts: Some(ReasoningEfforts::Levels(BTreeMap::from([
                ("off".into(), "off".into()),
                ("max".into(), "max".into()),
                ("high".into(), "gpt-high".into()),
            ]))),
            ..ModelEntry::default()
        };
        // The map is sorted by key; the levels come back in escalation order.
        assert_eq!(model.reasoning_levels(), vec!["off", "high", "max"]);
        assert_eq!(
            model.reasoning_effort_value(Some("high")).expect("level"),
            Some("gpt-high".to_owned())
        );
        assert_eq!(
            model
                .reasoning_effort_value(None)
                .expect("no selection means the provider default"),
            None
        );
        assert!(matches!(
            model.reasoning_effort_value(Some("minimal")),
            Err(ModelsError::Invalid(message)) if message.contains("does not declare reasoning level 'minimal'")
        ));

        // A model without reasoning metadata refuses any explicit level.
        let plain = ModelEntry {
            id: "gpt-4o".into(),
            ..ModelEntry::default()
        };
        assert!(plain.reasoning_levels().is_empty());
        assert!(matches!(
            plain.reasoning_effort_value(Some("high")),
            Err(ModelsError::Invalid(message)) if message.contains("declares no reasoning levels")
        ));
        assert_eq!(plain.reasoning_effort_value(None).expect("default"), None);

        // `false` declares reasoning off; an empty map counts as off too.
        let disabled = ModelEntry {
            id: "plain".into(),
            reasoning_efforts: Some(ReasoningEfforts::Disabled(false)),
            ..ModelEntry::default()
        };
        assert!(
            disabled
                .reasoning_efforts
                .as_ref()
                .expect("efforts")
                .is_disabled()
        );
        assert!(disabled.reasoning_levels().is_empty());
        assert!(ReasoningEfforts::Levels(BTreeMap::new()).is_disabled());
    }
}

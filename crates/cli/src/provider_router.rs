//! One place that turns the configured route and model into a provider.
//!
//! Every command that talks to a model used to resolve its endpoint and
//! credentials on its own: `serve` read the Models document, while `run`,
//! `tui`, `acp` and `schedule` relied on the adapters' `from_env`. That meant a
//! profile edited in Settings -> Models applied to the server only, and a key
//! saved in `<home>/credentials.json` was ignored by the local commands. This
//! module gives them all the same resolution order:
//!
//! 1. an explicit `--provider`/`--model` flag,
//! 2. the active route and model in `config.yaml`,
//! 3. the provider profile in `<home>/models.yaml`, with catalog defaults,
//! 4. the secret from the environment, then `<home>/credentials.json`.

use std::path::Path;
use std::sync::Arc;

use dshr_config::credentials;
use dshr_config::{
    Config, ModelsDocument, Protocol, ResolvedProvider, RetryMode as ConfigRetryMode,
};
use dshr_core::{DshrError, Result};
use dshr_llm::{LlmError, LlmProvider, RetryMode as LlmRetryMode, RetryPlan};
use dshr_llm_deepseek::{Auth, DeepSeekConfig, DeepSeekProvider};
use dshr_llm_mock::ReplayProvider;
use dshr_llm_openai::{OpenAiConfig, OpenAiProvider};

/// What a command line contributes to the choice of provider.
#[derive(Debug, Default, Clone, Copy)]
pub struct Request<'a> {
    /// Replay a recorded cassette instead of calling a model.
    pub cassette: Option<&'a Path>,
    /// `--provider` flag; the active route in `config.yaml` when absent.
    pub provider: Option<&'a str>,
    /// `--model` flag; the active model in `config.yaml` when absent.
    pub model: Option<&'a str>,
    /// Output-token ceiling; the adapter default when absent.
    pub max_tokens: Option<u32>,
}

/// Builds the provider the command should talk to.
///
/// Returns the error a command should show the user when the route is not in
/// the catalog and has no profile, or when no secret can be found.
pub fn build(
    request: &Request<'_>,
    config: &Config,
    home: &std::path::Path,
) -> Result<Arc<dyn LlmProvider>> {
    if let Some(path) = request.cassette {
        return Ok(Arc::new(
            ReplayProvider::from_file(path).map_err(llm_error)?,
        ));
    }
    let route = request
        .provider
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map_or_else(|| config.model.provider.clone(), str::to_owned);
    let model = request
        .model
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map_or_else(|| config.model.name.clone(), str::to_owned);
    let max_tokens = request.max_tokens.unwrap_or(2048).max(1);

    // The Models page stores per-provider profiles in `models.yaml`; a route
    // without a profile falls back to the catalog entry.
    let document =
        ModelsDocument::load(home).map_err(|error| DshrError::config(error.to_string()))?;
    let resolved = document
        .resolve(&route, config.model.base_url.as_deref())
        .ok_or_else(|| {
            DshrError::config(format!(
                "unknown provider route '{route}'; add it in Settings -> Models or use deepseek or openai-compatible"
            ))
        })?;
    if resolved.models_from_profile
        && !resolved.models.is_empty()
        && !resolved.models.iter().any(|entry| entry.id == model)
    {
        tracing::warn!(
            provider = %resolved.route,
            model = %model,
            "the active model is not in the provider's profile list; sending it anyway"
        );
    }

    let base_url = resolved
        .effective_base_url
        .clone()
        .unwrap_or_else(|| default_base_url(resolved.protocol).to_owned());
    let reference = resolved
        .api_key_env
        .clone()
        .unwrap_or_else(|| default_key_reference(&route).to_owned());
    let retry = retry_plan(&resolved);
    let reasoning_effort = reasoning_effort(&resolved, &model, config)?;
    let (secret, account_token) = read_secret(&reference, &resolved, home)
        .ok_or_else(|| {
            LlmError::MissingCredentials(format!(
                "set {reference} in the environment or save it in Settings -> Models"
            ))
        })
        .map_err(llm_error)?;

    match resolved.protocol {
        Protocol::AnthropicMessages => {
            let auth = if account_token {
                Auth::AccountToken(secret)
            } else {
                Auth::ApiKey(secret)
            };
            Ok(Arc::new(
                DeepSeekProvider::new(DeepSeekConfig {
                    base_url,
                    model,
                    max_tokens,
                    auth,
                    retry: retry.clone(),
                    reasoning_effort: reasoning_effort.clone(),
                })
                .map_err(llm_error)?,
            ))
        }
        Protocol::OpenAiChat => Ok(Arc::new(
            OpenAiProvider::new(OpenAiConfig {
                base_url,
                model,
                max_tokens,
                api_key: secret,
                retry,
                reasoning_effort,
            })
            .map_err(llm_error)?,
        )),
    }
}

/// Turns the configured level into the value the endpoint expects.
///
/// A profile that lists its models also declares which levels each one
/// offers, so the level is checked against that list and an undeclared one is
/// a configuration error rather than a silent downgrade. A model the profile
/// does not describe (catalog models, or a gateway whose model list is empty)
/// cannot be checked; the level is passed through and the adapter decides
/// whether the endpoint accepts it.
fn reasoning_effort(
    resolved: &ResolvedProvider,
    model: &str,
    config: &Config,
) -> Result<Option<String>> {
    let Some(level) = config
        .model
        .reasoning_effort
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    else {
        return Ok(None);
    };
    let Some(entry) = resolved.models.iter().find(|entry| entry.id == model) else {
        tracing::warn!(
            provider = %resolved.route,
            model = %model,
            level = %level,
            "the model is not listed in its provider profile, so the reasoning level cannot be checked"
        );
        return Ok(Some(level.to_owned()));
    };
    entry
        .reasoning_effort_value(Some(level))
        .map_err(|error| DshrError::config(format!("provider '{}': {error}", resolved.route)))
}

fn retry_plan(resolved: &ResolvedProvider) -> RetryPlan {
    let retry = &resolved.retry;
    RetryPlan::new(
        match retry.mode {
            ConfigRetryMode::Normal => LlmRetryMode::Normal,
            ConfigRetryMode::Always => LlmRetryMode::Always,
        },
        retry.max_retries,
        retry.retryable_codes.clone(),
        retry.initial_delay_ms,
        retry.max_delay_ms,
        retry.jitter_ratio as f32,
    )
}

fn default_base_url(protocol: Protocol) -> &'static str {
    match protocol {
        Protocol::AnthropicMessages => "https://api.deepseek.com/anthropic",
        Protocol::OpenAiChat => "https://api.openai.com/v1",
    }
}

fn default_key_reference(route: &str) -> &'static str {
    match route {
        "deepseek" | "deepseek-account" => "DSH_DEEPSEEK_TOKEN",
        "deepseek-official" => "DEEPSEEK_API_KEY",
        "routerai" => "ROUTERAI_API_KEY",
        _ => "OPENAI_API_KEY",
    }
}

/// Reads the profile's reference first, then the adapter's historical
/// fallbacks, so an existing environment keeps working untouched.
/// Returns the secret and whether it is the account grant (which the DeepSeek
/// adapter sends on a different header than an API key).
fn read_secret(
    reference: &str,
    resolved: &ResolvedProvider,
    home: &std::path::Path,
) -> Option<(String, bool)> {
    let read =
        |name: &str| credentials::resolve(home, name).map(|resolved| resolved.value().to_owned());
    if let Some(value) = read(reference) {
        return Some((value, reference == "DSH_DEEPSEEK_TOKEN"));
    }
    if resolved.protocol == Protocol::AnthropicMessages {
        return read("DEEPSEEK_API_KEY")
            .map(|value| (value, false))
            .or_else(|| read("DSH_DEEPSEEK_TOKEN").map(|value| (value, true)));
    }
    read("OPENAI_API_KEY")
        .or_else(|| read("ROUTERAI_API_KEY"))
        .map(|value| (value, false))
}

pub(crate) fn llm_error(error: LlmError) -> DshrError {
    DshrError::config(format!("llm: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use dshr_config::known_route;

    fn home() -> std::path::PathBuf {
        std::path::PathBuf::from("/nonexistent-dshr-provider-router-home")
    }

    #[test]
    fn the_catalog_routes_fall_back_to_their_own_endpoints() {
        assert_eq!(
            default_base_url(Protocol::AnthropicMessages),
            "https://api.deepseek.com/anthropic"
        );
        assert_eq!(
            default_base_url(Protocol::OpenAiChat),
            "https://api.openai.com/v1"
        );
    }

    #[test]
    fn the_key_reference_follows_the_route() {
        assert_eq!(default_key_reference("deepseek"), "DSH_DEEPSEEK_TOKEN");
        assert_eq!(
            default_key_reference("deepseek-account"),
            "DSH_DEEPSEEK_TOKEN"
        );
        assert_eq!(
            default_key_reference("deepseek-official"),
            "DEEPSEEK_API_KEY"
        );
        assert_eq!(default_key_reference("routerai"), "ROUTERAI_API_KEY");
        assert_eq!(default_key_reference("openai"), "OPENAI_API_KEY");
        assert_eq!(default_key_reference("openai-compatible"), "OPENAI_API_KEY");
    }

    #[test]
    fn an_unknown_route_names_the_settings_page() {
        let config = Config::default();
        let Err(error) = build(
            &Request {
                provider: Some("no-such-route"),
                ..Request::default()
            },
            &config,
            &home(),
        ) else {
            panic!("an unknown route must not build a provider");
        };
        let message = error.to_string();
        assert!(message.contains("unknown provider route 'no-such-route'"));
        assert!(message.contains("Settings -> Models"));
    }

    #[test]
    fn a_route_from_the_catalog_resolves_without_a_models_document() {
        for route in [
            "deepseek",
            "deepseek-account",
            "deepseek-official",
            "openai",
            "routerai",
        ] {
            assert!(known_route(route), "{route} must be in the catalog");
            let document = ModelsDocument::load(&home()).expect("a missing document is empty");
            assert!(
                document.resolve(route, None).is_some(),
                "{route} must resolve"
            );
        }
    }

    #[test]
    fn a_level_the_model_does_not_declare_is_refused() {
        let document = ModelsDocument::load(&home()).expect("a missing document is empty");
        let resolved = document
            .resolve("deepseek-account", None)
            .expect("the catalog route resolves");
        let mut config = Config::default();
        config.model.reasoning_effort = Some("nonsense".to_owned());
        let error = reasoning_effort(&resolved, "deepseek-flash", &config)
            .expect_err("an undeclared level must be refused");
        assert!(error.to_string().contains("deepseek-account"));
    }

    #[test]
    fn a_level_the_model_declares_becomes_the_wire_value() {
        let document = ModelsDocument::load(&home()).expect("a missing document is empty");
        let resolved = document
            .resolve("deepseek-account", None)
            .expect("the catalog route resolves");
        let mut config = Config::default();
        config.model.reasoning_effort = Some("high".to_owned());
        let level = reasoning_effort(&resolved, "deepseek-flash", &config)
            .expect("a declared level is accepted");
        assert_eq!(level.as_deref(), Some("high"));
    }

    #[test]
    fn a_blank_level_sends_nothing() {
        let document = ModelsDocument::load(&home()).expect("a missing document is empty");
        let resolved = document
            .resolve("deepseek-account", None)
            .expect("the catalog route resolves");
        let mut config = Config::default();
        config.model.reasoning_effort = Some("   ".to_owned());
        let level = reasoning_effort(&resolved, "deepseek-flash", &config)
            .expect("a blank level is accepted");
        assert_eq!(level, None);
    }

    #[test]
    fn the_retry_plan_follows_the_profile() {
        let document = ModelsDocument::load(&home()).expect("a missing document is empty");
        let resolved = document
            .resolve("deepseek-account", None)
            .expect("the catalog route resolves");
        let plan = retry_plan(&resolved);
        assert_eq!(plan.max_retries, 5);
        assert!(!plan.retryable_codes.is_empty());
    }
}

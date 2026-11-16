//! Models settings API: the provider directory joined with stored profiles,
//! the active route, and the writes that maintain `models.yaml`.
//!
//! The web UI's Models section is the only client. Writes carry the document
//! revision they were read at, so a stale card cannot overwrite a newer one;
//! a mismatch is `409` and the page reloads.

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use dshr_config::models::{CATALOG, ModelsDocument, ModelsError, Protocol, catalog_entry};
use dshr_config::{ModelEntry, ProviderProfile, RetryPolicy};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{AppState, authorize, read_user_config};

type ApiError = (StatusCode, Json<Value>);

fn error(status: StatusCode, message: impl Into<String>) -> ApiError {
    (status, Json(json!({ "error": message.into() })))
}

fn models_error(failure: &ModelsError) -> ApiError {
    match failure {
        ModelsError::Conflict { expected, actual } => error(
            StatusCode::CONFLICT,
            format!("settings changed elsewhere (expected revision {expected}, stored {actual})"),
        ),
        ModelsError::Invalid(message) | ModelsError::Parse(message) => {
            error(StatusCode::BAD_REQUEST, message.clone())
        }
        ModelsError::Io(message) => error(StatusCode::INTERNAL_SERVER_ERROR, message.clone()),
    }
}

fn authorized(state: &AppState, headers: &HeaderMap) -> Result<(), ApiError> {
    authorize(state, headers, None)
        .map_err(|_| error(StatusCode::UNAUTHORIZED, "missing or invalid bearer token"))
}

/// The active route, model and endpoint from `config.yaml`.
fn active_model(state: &AppState) -> (String, String, Option<String>) {
    let config = read_user_config(&state.config.home);
    // Without a persisted selection the live adapter names the route; its
    // `deepseek` alias still resolves to the catalog entry.
    let provider = config["model"]["provider"]
        .as_str()
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| state.provider.name().to_owned());
    let model = config["model"]["name"]
        .as_str()
        .map(str::to_owned)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| state.provider.model().to_owned());
    let base_url = config["model"]["base_url"]
        .as_str()
        .filter(|value| !value.is_empty())
        .map(str::to_owned);
    (provider, model, base_url)
}

/// Whether a referenced environment variable carries a non-empty value.
fn reference_configured(reference: &str) -> bool {
    std::env::var(reference).is_ok_and(|value| !value.trim().is_empty())
}

fn credential_configured(route: &str, reference: Option<&str>) -> Option<bool> {
    let reference = reference?;
    if route == "deepseek" || route == "deepseek-account" {
        // The DeepSeek adapter accepts either the account grant or an API key.
        return Some(
            reference_configured("DSH_DEEPSEEK_TOKEN") || reference_configured("DEEPSEEK_API_KEY"),
        );
    }
    Some(reference_configured(reference))
}

fn model_json(model: &ModelEntry) -> Value {
    json!({
        "id": model.id,
        "name": model.name,
        "context_window": model.context_window,
        "max_tokens": model.max_tokens,
        "input": model.input,
        "reasoning_efforts": model.reasoning_efforts,
    })
}

fn catalog_json() -> Value {
    Value::Array(
        CATALOG
            .iter()
            .map(|entry| {
                json!({
                    "id": entry.id,
                    "display_name": entry.display_name,
                    "protocol": entry.protocol.id(),
                    "default_base_url": entry.default_base_url,
                    "base_url_env": entry.base_url_env,
                    "api_key_env": entry.api_key_env,
                    "account": entry.account,
                    "summary": entry.summary,
                    "models": entry.models.iter().map(|model| json!({
                        "id": model.id,
                        "name": model.name,
                    })).collect::<Vec<_>>(),
                })
            })
            .collect(),
    )
}

/// One row of the Models page: the profile joined with catalog defaults.
fn provider_row(
    document: &ModelsDocument,
    route: &str,
    active: &str,
    config_base_url: Option<&str>,
) -> Value {
    let configured = document.providers.contains_key(route);
    let resolved = document.resolve(
        route,
        (route == active).then_some(config_base_url).flatten(),
    );
    let Some(resolved) = resolved else {
        return json!({
            "id": route,
            "display_name": route,
            "protocol": null,
            "configured": configured,
            "removable": configured,
            "active": route == active,
            "base_url": null,
            "effective_base_url": null,
            "api_key_env": null,
            "credential_configured": null,
            "account": false,
            "summary": "This adapter is not part of the configurable provider catalog.",
            "models": [],
            "models_from_profile": false,
            "retry": Value::Null,
            "editable": false,
            "error": Value::Null,
        });
    };
    let reference = resolved.api_key_env.as_deref();
    json!({
        "id": route,
        "display_name": resolved.display_name,
        "protocol": resolved.protocol.id(),
        "configured": configured,
        "removable": configured,
        "active": route == active,
        "base_url": resolved.base_url,
        "effective_base_url": resolved.effective_base_url,
        "api_key_env": resolved.api_key_env,
        "credential_configured": credential_configured(route, reference),
        "account": resolved.account,
        "summary": resolved.summary,
        "models": resolved.models.iter().map(model_json).collect::<Vec<_>>(),
        "models_from_profile": resolved.models_from_profile,
        "retry": document
            .providers
            .get(route)
            .and_then(|profile| profile.retry.as_ref()),
        "editable": true,
        "error": Value::Null,
    })
}

/// `GET /v1/config/providers`: the catalog, the active selection, and every
/// configured row (the active route appears even before it has a profile).
pub async fn get_providers(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    authorized(&state, &headers)?;
    let document =
        ModelsDocument::load(&state.config.home).map_err(|error| models_error(&error))?;
    let (active, model, config_base_url) = active_model(&state);
    let configured_active = (!active.is_empty()).then_some(active.as_str());
    let routes = document.visible_routes(configured_active);
    let rows: Vec<Value> = routes
        .iter()
        .map(|route| provider_row(&document, route, &active, config_base_url.as_deref()))
        .collect();
    Ok(Json(json!({
        "revision": document.revision,
        "writable": true,
        "active": { "provider": active, "model": model },
        "catalog": catalog_json(),
        "providers": rows,
        "note": "A saved profile or model list applies to `dshr serve` after a restart.",
    })))
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderWrite {
    #[serde(default)]
    pub expected_revision: Option<u64>,
    /// Wire protocol; required for a route outside the catalog.
    #[serde(default)]
    pub protocol: Option<String>,
    /// Endpoint override; `null` inherits the catalog/environment value.
    pub base_url: Option<String>,
    /// API-key variable override; `null` inherits the catalog default.
    pub api_key_env: Option<String>,
    /// Full model list; an empty list restores the catalog defaults.
    pub models: Vec<ModelEntry>,
    /// Retry policy; `null` inherits the adapter defaults.
    #[serde(default)]
    pub retry: Option<RetryPolicy>,
}

fn validated_profile(route: &str, body: ProviderWrite) -> Result<ProviderProfile, ApiError> {
    if !route
        .chars()
        .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
        || route.is_empty()
        || route.len() > 64
        || route.starts_with('-')
    {
        return Err(error(StatusCode::BAD_REQUEST, "invalid provider route"));
    }
    let declared = match body.protocol.as_deref() {
        Some(value) => Some(
            Protocol::parse(value)
                .ok_or_else(|| error(StatusCode::BAD_REQUEST, "unknown provider protocol"))?,
        ),
        None => None,
    };
    let canonical = catalog_entry(route).map(|entry| entry.protocol);
    match (canonical, declared) {
        (None, None) => {
            return Err(error(
                StatusCode::BAD_REQUEST,
                "a route outside the catalog must declare its protocol",
            ));
        }
        (Some(canonical), Some(declared)) if canonical != declared => {
            return Err(error(
                StatusCode::BAD_REQUEST,
                "the route's protocol is fixed by its adapter",
            ));
        }
        _ => {}
    }
    let base_url = body
        .base_url
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty());
    if let Some(base_url) = base_url.as_deref()
        && !(base_url.starts_with("https://") || base_url.starts_with("http://"))
    {
        return Err(error(
            StatusCode::BAD_REQUEST,
            "the base URL must start with http:// or https://",
        ));
    }
    if catalog_entry(route).is_none() && base_url.is_none() {
        return Err(error(
            StatusCode::BAD_REQUEST,
            "a route outside the catalog needs a base URL",
        ));
    }
    let api_key_env = body
        .api_key_env
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty());
    Ok(ProviderProfile {
        protocol: declared,
        base_url,
        api_key_env,
        models: body.models,
        retry: body.retry,
    })
}

/// `PUT /v1/config/providers/{id}`: replace one provider profile.
pub async fn put_provider(
    State(state): State<AppState>,
    Path(route): Path<String>,
    headers: HeaderMap,
    Json(body): Json<ProviderWrite>,
) -> Result<Json<Value>, ApiError> {
    authorized(&state, &headers)?;
    let expected = body.expected_revision;
    let profile = validated_profile(&route, body)?;
    let mut trial =
        ModelsDocument::load(&state.config.home).map_err(|error| models_error(&error))?;
    trial.providers.insert(route.clone(), profile.clone());
    trial.validate().map_err(|error| models_error(&error))?;
    let saved = ModelsDocument::update(&state.config.home, expected, |document| {
        document.providers.insert(route.clone(), profile);
    })
    .map_err(|error| models_error(&error))?;
    Ok(Json(json!({
        "ok": true,
        "revision": saved.revision,
        "provider": route,
    })))
}

#[derive(Debug, Deserialize)]
pub struct RevisionQuery {
    #[serde(default)]
    pub expected_revision: Option<u64>,
}

/// `DELETE /v1/config/providers/{id}`: drop the user profile, restoring the
/// catalog defaults for that route.
pub async fn delete_provider(
    State(state): State<AppState>,
    Path(route): Path<String>,
    Query(query): Query<RevisionQuery>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    authorized(&state, &headers)?;
    let document =
        ModelsDocument::load(&state.config.home).map_err(|error| models_error(&error))?;
    if !document.providers.contains_key(&route) {
        return Err(error(StatusCode::NOT_FOUND, "no profile for that provider"));
    }
    let saved = ModelsDocument::update(&state.config.home, query.expected_revision, |document| {
        document.providers.remove(&route);
    })
    .map_err(|error| models_error(&error))?;
    Ok(Json(json!({
        "ok": true,
        "revision": saved.revision,
        "provider": route,
    })))
}

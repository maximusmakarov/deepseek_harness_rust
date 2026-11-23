//! Models settings section (web UI).
//!
//! Native port of the React `ui-settings-models` section surface: the provider
//! directory (catalog + stored profiles), one editor card per provider, and a
//! model list with capacities and input types. Profiles are persisted by the
//! server in `<home>/models.yaml`; the revision returned with the snapshot is
//! sent back on every write so a stale card cannot overwrite a newer edit.

use gloo_net::http::Request;
use leptos::prelude::*;

use dshr_i18n::{Lang, ModelsLabels};

use crate::SlotRegion;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use wasm_bindgen_futures::spawn_local;

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
struct ActiveSelection {
    #[serde(default)]
    provider: String,
    #[serde(default)]
    model: String,
    /// The stored level; absent means the provider's own default.
    #[serde(default)]
    reasoning_effort: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
struct CatalogModel {
    #[serde(default)]
    id: String,
    #[serde(default)]
    name: String,
    /// Levels the catalog offers for this model, in escalation order.
    #[serde(default)]
    reasoning_levels: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
struct CatalogEntry {
    #[serde(default)]
    id: String,
    #[serde(default)]
    display_name: String,
    #[serde(default)]
    protocol: String,
    #[serde(default)]
    default_base_url: Option<String>,
    #[serde(default)]
    api_key_env: Option<String>,
    #[serde(default)]
    account: bool,
    #[serde(default)]
    summary: String,
    #[serde(default)]
    models: Vec<CatalogModel>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
struct ModelInfo {
    #[serde(default)]
    id: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    context_window: Option<u64>,
    #[serde(default)]
    max_tokens: Option<u32>,
    #[serde(default)]
    input: Vec<String>,
    #[serde(default)]
    reasoning_efforts: Option<Value>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
struct ProviderRow {
    #[serde(default)]
    id: String,
    #[serde(default)]
    display_name: String,
    #[serde(default)]
    protocol: String,
    #[serde(default)]
    configured: bool,
    #[serde(default)]
    removable: bool,
    #[serde(default)]
    active: bool,
    #[serde(default)]
    base_url: Option<String>,
    #[serde(default)]
    effective_base_url: Option<String>,
    #[serde(default)]
    api_key_env: Option<String>,
    #[serde(default)]
    credential_configured: Option<bool>,
    #[serde(default)]
    credential_masked: Option<String>,
    #[serde(default)]
    credential_source: Option<String>,
    #[serde(default)]
    account: bool,
    #[serde(default)]
    summary: String,
    #[serde(default)]
    models: Vec<ModelInfo>,
    #[serde(default)]
    models_from_profile: bool,
    #[serde(default)]
    retry: Option<Value>,
    #[serde(default)]
    editable: bool,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
struct ModelsSnapshot {
    #[serde(default)]
    revision: u64,
    #[serde(default)]
    writable: bool,
    #[serde(default)]
    active: ActiveSelection,
    #[serde(default)]
    catalog: Vec<CatalogEntry>,
    #[serde(default)]
    providers: Vec<ProviderRow>,
    #[serde(default)]
    note: String,
    #[serde(default)]
    welcome_version: Option<u32>,
    #[serde(default)]
    credentials_revision: u64,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
struct DiscoveredModel {
    #[serde(default)]
    id: String,
    #[serde(default)]
    name: String,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct DiscoverResponse {
    #[serde(default)]
    models: Vec<DiscoveredModel>,
}

/// A destructive action waiting for the modal confirmation.
#[derive(Clone, Debug, PartialEq)]
enum ConfirmAction {
    Provider {
        route: String,
        name: String,
    },
    Model {
        name: String,
        index: usize,
        model_id: String,
    },
}

#[derive(Clone, Debug, PartialEq)]
struct ModelDraft {
    id: String,
    name: String,
    context_window: String,
    max_tokens: String,
    text: bool,
    image: bool,
    reasoning_efforts: Option<Value>,
}

#[derive(Clone, Debug, PartialEq)]
struct ProviderDraft {
    id: String,
    display_name: String,
    protocol: String,
    base_url: String,
    api_key_env: String,
    models: Vec<ModelDraft>,
    retry: Option<RetryDraft>,
    api_key: String,
    api_key_saved: bool,
    api_key_mask: Option<String>,
    api_key_source: Option<String>,
    configured: bool,
    custom: bool,
    account: bool,
    default_base_url: Option<String>,
    default_api_key_env: Option<String>,
}

impl ProviderDraft {
    fn from_row(row: &ProviderRow, catalog: Option<&CatalogEntry>) -> Self {
        Self {
            id: row.id.clone(),
            display_name: row.display_name.clone(),
            protocol: row.protocol.clone(),
            base_url: row.base_url.clone().unwrap_or_default(),
            api_key_env: row.api_key_env.clone().unwrap_or_default(),
            models: row.models.iter().map(model_draft).collect(),
            retry: row.retry.as_ref().and_then(RetryDraft::from_value),
            api_key: String::new(),
            api_key_saved: row.credential_configured == Some(true),
            api_key_mask: row.credential_masked.clone(),
            api_key_source: row.credential_source.clone(),
            configured: row.configured,
            custom: false,
            account: row.account,
            default_base_url: catalog
                .and_then(|entry| entry.default_base_url.clone())
                .or_else(|| row.effective_base_url.clone()),
            default_api_key_env: catalog
                .and_then(|entry| entry.api_key_env.clone())
                .or_else(|| row.api_key_env.clone()),
        }
    }

    fn from_catalog(entry: &CatalogEntry) -> Self {
        Self {
            id: entry.id.clone(),
            display_name: entry.display_name.clone(),
            protocol: entry.protocol.clone(),
            base_url: String::new(),
            api_key_env: entry.api_key_env.clone().unwrap_or_default(),
            retry: None,
            api_key: String::new(),
            api_key_saved: false,
            api_key_mask: None,
            api_key_source: None,
            models: entry
                .models
                .iter()
                .map(|model| ModelDraft {
                    id: model.id.clone(),
                    name: model.name.clone(),
                    context_window: String::new(),
                    max_tokens: String::new(),
                    text: true,
                    image: false,
                    reasoning_efforts: catalog_efforts(&model.reasoning_levels),
                })
                .collect(),
            configured: false,
            custom: false,
            account: entry.account,
            default_base_url: entry.default_base_url.clone(),
            default_api_key_env: entry.api_key_env.clone(),
        }
    }

    fn new_custom() -> Self {
        Self {
            id: String::new(),
            display_name: String::new(),
            protocol: "openai-chat".into(),
            base_url: String::new(),
            api_key_env: String::new(),
            models: Vec::new(),
            retry: None,
            api_key: String::new(),
            api_key_saved: false,
            api_key_mask: None,
            api_key_source: None,
            configured: false,
            custom: true,
            account: false,
            default_base_url: None,
            default_api_key_env: None,
        }
    }
}

/// Levels the adapters accept, in escalation order.
const REASONING_LEVELS: [&str; 7] = ["off", "minimal", "low", "medium", "high", "xhigh", "max"];

/// A catalog model's levels as a declaration: absent when it lists none.
///
/// The levels the catalog does not list are written as `null`, otherwise the
/// standard levels the config layer supports by default would widen the
/// choice (the DeepSeek endpoint takes `off`/`low`/`high`/`max` only).
fn catalog_efforts(levels: &[String]) -> Option<Value> {
    (!levels.is_empty()).then(|| {
        Value::Object(
            REASONING_LEVELS
                .iter()
                .map(|level| {
                    let value = levels
                        .iter()
                        .any(|declared| declared == level)
                        .then(|| Value::String((*level).to_owned()));
                    ((*level).to_owned(), value.unwrap_or(Value::Null))
                })
                .collect(),
        )
    })
}

/// The level -> value rows of a model's reasoning declaration.
fn reasoning_entries(value: &Option<Value>) -> Vec<(String, String)> {
    match value {
        Some(Value::Object(map)) => map
            .iter()
            .filter_map(|(level, value)| {
                value
                    .as_str()
                    .map(|value| (level.clone(), value.to_owned()))
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn reasoning_object(entries: &[(String, String)]) -> Value {
    reasoning_object_with(entries, &None)
}

/// Rebuilds the declaration from the editor rows, keeping the levels the
/// profile marks unsupported (`null`): the editor does not render them, so
/// rewriting the map from its rows alone would silently re-enable them.
fn reasoning_object_with(entries: &[(String, String)], original: &Option<Value>) -> Value {
    let mut map = serde_json::Map::new();
    if let Some(Value::Object(existing)) = original {
        for (level, value) in existing {
            if value.is_null() {
                map.insert(level.clone(), Value::Null);
            }
        }
    }
    for (level, value) in entries {
        map.insert(level.clone(), Value::String(value.clone()));
    }
    Value::Object(map)
}

fn reasoning_enabled(value: &Option<Value>) -> bool {
    matches!(value, Some(Value::Object(_)))
}

/// Levels to declare when reasoning is switched on.
///
/// The catalog knows which levels a model accepts; the generic default would
/// offer `medium` for a DeepSeek model, which its endpoint rejects, and hide
/// `max`, which it accepts. A model the catalog does not describe keeps the
/// generic default, because its vocabulary is unknown.
fn reasoning_seed(snapshot: Option<&ModelsSnapshot>, route: &str, model: &str) -> Value {
    snapshot
        .and_then(|snapshot| snapshot.catalog.iter().find(|entry| entry.id == route))
        .and_then(|entry| entry.models.iter().find(|entry| entry.id == model))
        .and_then(|entry| catalog_efforts(&entry.reasoning_levels))
        .unwrap_or_else(reasoning_default)
}

/// The upstream starting point when the switch turns on.
fn reasoning_default() -> Value {
    reasoning_object(&[
        ("off".into(), "off".into()),
        ("low".into(), "low".into()),
        ("medium".into(), "medium".into()),
        ("high".into(), "high".into()),
    ])
}

/// First-run notice version this UI asks the server to acknowledge.
const WELCOME_VERSION: u32 = 1;

/// Failure-code choices, in DSH's order; the wire value stays here and the
/// display label comes from the shared string table.
const RETRY_CODE_CHOICES: [&str; 8] = [
    "EMPTY_RESPONSE",
    "RATE_LIMIT",
    "SERVER",
    "TIMEOUT",
    "TRANSPORT",
    "STREAM_CLOSED",
    "MALFORMED_RESPONSE",
    "QUOTA",
];

fn retry_code_label(code: &str, labels: &ModelsLabels) -> &'static str {
    match code {
        "EMPTY_RESPONSE" => labels.err_empty_response,
        "RATE_LIMIT" => labels.err_rate_limited,
        "SERVER" => labels.err_server_error,
        "TIMEOUT" => labels.err_timeout,
        "TRANSPORT" => labels.err_transport,
        "STREAM_CLOSED" => labels.err_stream_closed,
        "MALFORMED_RESPONSE" => labels.err_malformed_response,
        _ => labels.err_quota,
    }
}

const RETRY_DEFAULT_CODES: [&str; 5] = [
    "EMPTY_RESPONSE",
    "RATE_LIMIT",
    "SERVER",
    "TIMEOUT",
    "TRANSPORT",
];

/// The editor's retry draft; every numeric field is text so an empty field can
/// keep the inherited default.
#[derive(Clone, Debug, PartialEq)]
struct RetryDraft {
    mode: String,
    max_retries: String,
    codes: Vec<String>,
    initial_delay: String,
    max_delay: String,
    jitter: String,
}

impl RetryDraft {
    fn defaults() -> Self {
        Self {
            mode: "normal".into(),
            max_retries: "5".into(),
            codes: RETRY_DEFAULT_CODES
                .iter()
                .map(|code| (*code).to_owned())
                .collect(),
            initial_delay: "500".into(),
            max_delay: "10000".into(),
            jitter: "0.1".into(),
        }
    }

    fn from_value(value: &Value) -> Option<Self> {
        let mode = value["mode"].as_str()?;
        if mode != "normal" && mode != "always" {
            return None;
        }
        let backoff = &value["backoff"];
        let codes = value["retryable_codes"]
            .as_array()
            .map(|codes| {
                codes
                    .iter()
                    .filter_map(|code| code.as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_else(|| {
                RETRY_DEFAULT_CODES
                    .iter()
                    .map(|code| (*code).to_owned())
                    .collect()
            });
        Some(Self {
            mode: mode.to_owned(),
            max_retries: number_text(&value["max_retries"], 5),
            codes,
            initial_delay: number_text(&backoff["initial_delay_ms"], 500),
            max_delay: number_text(&backoff["max_delay_ms"], 10_000),
            jitter: backoff["jitter_ratio"]
                .as_f64()
                .map(|value| value.to_string())
                .unwrap_or_else(|| "0.1".to_owned()),
        })
    }
}

fn number_text(value: &Value, fallback: u64) -> String {
    value
        .as_u64()
        .map(|value| value.to_string())
        .unwrap_or_else(|| fallback.to_string())
}

fn retry_body(draft: &RetryDraft, lang: Lang) -> Result<Value, String> {
    let labels = lang.models_labels();
    if draft.mode != "normal" && draft.mode != "always" {
        return Err(labels.choose_retry_mode.into());
    }
    let initial = parse_number(&draft.initial_delay, labels.retry_initial_delay, lang)?;
    let max = parse_number(&draft.max_delay, labels.retry_maximum_delay, lang)?;
    if initial == 0 || max == 0 {
        return Err(labels.retry_delays_positive.into());
    }
    if initial > max {
        return Err(labels.retry_initial_exceeds_max.into());
    }
    let jitter: f64 = draft
        .jitter
        .trim()
        .parse()
        .map_err(|_| labels.retry_jitter_not_number.to_owned())?;
    if !(0.0..=1.0).contains(&jitter) {
        return Err(labels.retry_jitter_out_of_range.into());
    }
    let backoff = json!({
        "initial_delay_ms": initial,
        "max_delay_ms": max,
        "jitter_ratio": jitter,
    });
    if draft.mode == "always" {
        return Ok(json!({
            "mode": "always",
            "max_retries": Value::Null,
            "retryable_codes": Value::Null,
            "backoff": backoff,
        }));
    }
    if draft.codes.is_empty() {
        return Err(labels.select_retryable_code.into());
    }
    let max_retries = parse_number(&draft.max_retries, labels.maximum_retries, lang)?;
    Ok(json!({
        "mode": "normal",
        "max_retries": max_retries,
        "retryable_codes": draft.codes,
        "backoff": backoff,
    }))
}

fn parse_number(value: &str, label: &str, lang: Lang) -> Result<u64, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(lang.models_whole_number(label));
    }
    value.parse().map_err(|_| lang.models_whole_number(label))
}

/// Merge a discovery reply into the drafted model list: checked candidates are
/// appended when missing, unchecked candidates listed by the reply are removed,
/// and configured rows the reply does not list stay. A merge that would empty a
/// non-empty list is refused, since an empty list serves the whole catalog.
fn merge_discovered(
    models: &[ModelDraft],
    candidates: &[DiscoveredModel],
    selected: &std::collections::BTreeSet<String>,
    lang: Lang,
) -> Result<Vec<ModelDraft>, String> {
    let had_rows = !models.is_empty();
    let mut next = models.to_vec();
    for candidate in candidates {
        let present = next.iter().any(|model| model.id == candidate.id);
        if selected.contains(&candidate.id) {
            if !present {
                next.push(ModelDraft {
                    id: candidate.id.clone(),
                    name: candidate.name.clone(),
                    context_window: String::new(),
                    max_tokens: String::new(),
                    text: true,
                    image: false,
                    reasoning_efforts: None,
                });
            }
        } else if present {
            next.retain(|model| model.id != candidate.id);
        }
    }
    if next.is_empty() && had_rows {
        return Err(lang.models_labels().selection_empties_list.into());
    }
    Ok(next)
}

fn model_draft(model: &ModelInfo) -> ModelDraft {
    ModelDraft {
        id: model.id.clone(),
        name: model.name.clone().unwrap_or_default(),
        context_window: model
            .context_window
            .map(|value| value.to_string())
            .unwrap_or_default(),
        max_tokens: model
            .max_tokens
            .map(|value| value.to_string())
            .unwrap_or_default(),
        text: model.input.is_empty() || model.input.iter().any(|value| value == "text"),
        image: model.input.iter().any(|value| value == "image"),
        reasoning_efforts: model.reasoning_efforts.clone(),
    }
}

fn draft_body(draft: &ProviderDraft, lang: Lang) -> Result<Value, String> {
    let labels = lang.models_labels();
    let base_url = draft.base_url.trim();
    if !base_url.is_empty()
        && !(base_url.starts_with("https://") || base_url.starts_with("http://"))
    {
        return Err(labels.base_url_scheme.into());
    }
    let reference = draft.api_key_env.trim();
    if !reference.is_empty()
        && (!reference
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_uppercase())
            || !reference
                .chars()
                .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_'))
    {
        return Err(labels.api_key_upper_snake.into());
    }
    let mut seen = std::collections::BTreeSet::new();
    let mut models = Vec::with_capacity(draft.models.len());
    for model in &draft.models {
        let id = model.id.trim();
        if id.is_empty() {
            return Err(labels.every_model_needs_id.into());
        }
        if id.chars().any(char::is_whitespace) {
            return Err(lang.models_id_whitespace(id));
        }
        if !seen.insert(id.to_owned()) {
            return Err(lang.models_id_listed_twice(id));
        }
        let context_window = parse_capacity(&model.context_window, labels.context_window, lang)?;
        let max_tokens = parse_capacity(&model.max_tokens, labels.max_output_tokens, lang)?;
        if let Some(efforts) = &model.reasoning_efforts {
            match efforts {
                Value::Bool(false) => {}
                Value::Object(levels) => {
                    if levels.is_empty() {
                        return Err(lang.models_reasoning_without_level(id));
                    }
                    for (level, value) in levels {
                        if !REASONING_LEVELS.contains(&level.as_str()) {
                            return Err(lang.models_unknown_level(id, level));
                        }
                        // `null` is how a profile marks a level unsupported.
                        if !value.is_null()
                            && !value.as_str().is_some_and(|value| !value.trim().is_empty())
                        {
                            return Err(lang.models_empty_level_value(id, level));
                        }
                    }
                }
                _ => return Err(lang.models_invalid_reasoning(id)),
            }
        }
        let mut input = Vec::new();
        if model.text {
            input.push("text");
        }
        if model.image {
            input.push("image");
        }
        models.push(json!({
            "id": id,
            "name": (!model.name.trim().is_empty()).then(|| model.name.trim()),
            "context_window": context_window,
            "max_tokens": max_tokens,
            "input": input,
            "reasoning_efforts": model.reasoning_efforts,
        }));
    }
    let retry = match &draft.retry {
        Some(retry) => retry_body(retry, lang)?,
        None => Value::Null,
    };
    Ok(json!({
        "protocol": draft.protocol,
        "base_url": (!base_url.is_empty()).then_some(base_url),
        "api_key_env": (!reference.is_empty()).then_some(reference),
        "models": models,
        "retry": retry,
    }))
}

fn parse_capacity(value: &str, label: &str, lang: Lang) -> Result<Option<u64>, String> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    let parsed: u64 = value.parse().map_err(|_| lang.models_whole_number(label))?;
    if parsed == 0 {
        return Err(lang.models_greater_than_zero(label));
    }
    Ok(Some(parsed))
}

#[derive(Clone, Debug, Serialize)]
struct ActiveRequest {
    provider: String,
    model: String,
    /// Sent only when a level is selected, so an untouched selection keeps the
    /// provider default instead of pinning one.
    #[serde(skip_serializing_if = "Option::is_none")]
    reasoning_effort: Option<String>,
}

/// Levels the active model offers, in escalation order.
///
/// A model that declares nothing offers no control at all, and one that
/// declares a single level (`{off: off}`) has nothing to choose between.
fn offered_levels(levels: &[String]) -> Vec<String> {
    REASONING_LEVELS
        .iter()
        .filter(|level| levels.iter().any(|declared| declared == *level))
        .map(|level| (*level).to_owned())
        .collect()
}

async fn send(
    method: &str,
    path: &str,
    token: &str,
    body: Option<&Value>,
) -> Result<Value, String> {
    let request = match method {
        "GET" => Request::get(path),
        "PUT" => Request::put(path),
        "DELETE" => Request::delete(path),
        "POST" => Request::post(path),
        _ => return Err(format!("unsupported method {method}")),
    };
    let request = request.header("Authorization", &format!("Bearer {token}"));
    let response = match body {
        Some(body) => {
            request
                .header("Content-Type", "application/json")
                .json(body)
                .map_err(|error| error.to_string())?
                .send()
                .await
        }
        None => request.send().await,
    }
    .map_err(|error| error.to_string())?;
    let status = response.status();
    let text = response.text().await.unwrap_or_default();
    let value: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
    if !(200..300).contains(&status) {
        let message = value["error"]
            .as_str()
            .map(str::to_owned)
            .unwrap_or_else(|| format!("HTTP {status}"));
        return Err(message);
    }
    Ok(value)
}

fn credential_dot(row: &ProviderRow, lang: Lang) -> (&'static str, &'static str) {
    let labels = lang.models_labels();
    match row.credential_configured {
        Some(true) => ("models-dot ok", labels.key_configured),
        Some(false) => ("models-dot missing", labels.no_key_in_environment),
        None => ("models-dot unknown", labels.no_key_for_route),
    }
}

fn protocol_label(protocol: &str, lang: Lang) -> &'static str {
    let labels = lang.models_labels();
    match protocol {
        "anthropic-messages" => labels.anthropic_messages,
        "openai-chat" => labels.openai_chat_completions,
        _ => labels.custom_protocol,
    }
}

/// The conventional key reference for a route: `<ROUTE>_API_KEY`.
fn derive_key_ref(route: &str) -> String {
    let mut name: String = route
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect();
    name.push_str("_API_KEY");
    name
}

fn valid_route(route: &str) -> bool {
    !route.is_empty()
        && route.len() <= 64
        && route.starts_with(|ch: char| ch.is_ascii_lowercase())
        && route
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
        && !route.ends_with('-')
}

#[component]
pub fn ModelsSettings(token: RwSignal<String>, lang: RwSignal<Lang>) -> impl IntoView {
    // Read the labels through the signal so a late `ui.language` reply re-renders
    // the tab instead of freezing the initial guess.
    let labels = Signal::derive(move || lang.get().models_labels());
    let snapshot = RwSignal::new(None::<ModelsSnapshot>);
    let error = RwSignal::new(String::new());
    let status = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    let editing = RwSignal::new(None::<ProviderDraft>);
    let confirm = RwSignal::new(None::<ConfirmAction>);
    let add_choice = RwSignal::new(String::new());
    let add_mode = RwSignal::new("catalog".to_owned());
    let candidates = RwSignal::new(None::<Vec<DiscoveredModel>>);
    let candidate_checked = RwSignal::new(std::collections::BTreeSet::<String>::new());
    let candidate_filter = RwSignal::new(String::new());
    let discovering = RwSignal::new(false);
    let welcome_dismissed = RwSignal::new(false);
    let acknowledging = RwSignal::new(false);
    let api_key_visible = RwSignal::new(false);
    let welcome_key = RwSignal::new(String::new());

    let load = move || {
        let value = token.get_untracked();
        busy.set(true);
        spawn_local(async move {
            match send("GET", "/v1/config/providers", &value, None).await {
                Ok(answer) => match serde_json::from_value::<ModelsSnapshot>(answer) {
                    Ok(parsed) => {
                        snapshot.set(Some(parsed));
                        error.set(String::new());
                    }
                    Err(problem) => {
                        error.set(
                            lang.get_untracked()
                                .models_snapshot_error(&problem.to_string()),
                        );
                    }
                },
                Err(message) => error.set(message),
            }
            busy.set(false);
        });
    };
    Effect::new(move |_| load());

    let save = move || {
        let Some(mut draft) = editing.get_untracked() else {
            return;
        };
        let revision = snapshot
            .get_untracked()
            .map(|value| value.revision)
            .unwrap_or_default();
        if draft.custom {
            if !valid_route(&draft.id) {
                error.set(
                    lang.get_untracked()
                        .models_labels()
                        .provider_id_format
                        .into(),
                );
                return;
            }
            let taken = snapshot
                .get_untracked()
                .is_some_and(|value| value.providers.iter().any(|row| row.id == draft.id));
            if taken {
                error.set(lang.get_untracked().models_provider_exists(&draft.id));
                return;
            }
            if draft.base_url.trim().is_empty() {
                error.set(
                    lang.get_untracked()
                        .models_labels()
                        .custom_needs_base_url
                        .into(),
                );
                return;
            }
            if draft.models.is_empty() {
                error.set(
                    lang.get_untracked()
                        .models_labels()
                        .custom_needs_model
                        .into(),
                );
                return;
            }
            if draft.models.iter().any(|model| model.id.trim().is_empty()) {
                error.set(
                    lang.get_untracked()
                        .models_labels()
                        .every_model_needs_id
                        .into(),
                );
                return;
            }
        }
        let typed_key = draft.api_key.trim().to_owned();
        if !typed_key.is_empty() && draft.api_key_env.trim().is_empty() {
            draft.api_key_env = derive_key_ref(&draft.id);
        }
        let key_reference = draft.api_key_env.trim().to_owned();
        let mut body = match draft_body(&draft, lang.get_untracked()) {
            Ok(body) => body,
            Err(message) => {
                error.set(message);
                return;
            }
        };
        body["expected_revision"] = json!(revision);
        let value = token.get_untracked();
        let route = draft.id.clone();
        let credentials_revision = snapshot
            .get_untracked()
            .map(|value| value.credentials_revision)
            .unwrap_or_default();
        busy.set(true);
        spawn_local(async move {
            match send(
                "PUT",
                &format!("/v1/config/providers/{route}"),
                &value,
                Some(&body),
            )
            .await
            {
                Ok(_) => {
                    if !typed_key.is_empty() {
                        let key_body = json!({
                            "value": typed_key,
                            "expected_revision": credentials_revision,
                        });
                        if let Err(message) = send(
                            "PUT",
                            &format!("/v1/credentials/{key_reference}"),
                            &value,
                            Some(&key_body),
                        )
                        .await
                        {
                            error.set(
                                lang.get_untracked()
                                    .models_profile_saved_key_failed(&message),
                            );
                            load();
                            busy.set(false);
                            return;
                        }
                    }
                    busy.set(false);
                    status.set(lang.get_untracked().models_saved_route(&route));
                    editing.set(None);
                    candidates.set(None);
                    candidate_checked.set(std::collections::BTreeSet::new());
                    confirm.set(None);
                    error.set(String::new());
                    load();
                }
                Err(message) => error.set(message),
            }
            busy.set(false);
        });
    };

    let remove_saved_key = move || {
        let Some(draft) = editing.get_untracked() else {
            return;
        };
        let reference = if draft.api_key_env.trim().is_empty() {
            derive_key_ref(&draft.id)
        } else {
            draft.api_key_env.trim().to_owned()
        };
        let revision = snapshot
            .get_untracked()
            .map(|value| value.credentials_revision)
            .unwrap_or_default();
        let value = token.get_untracked();
        busy.set(true);
        spawn_local(async move {
            let path = format!("/v1/credentials/{reference}?expected_revision={revision}");
            match send("DELETE", &path, &value, None).await {
                Ok(_) => {
                    editing.update(|draft| {
                        if let Some(draft) = draft {
                            draft.api_key_saved = false;
                            draft.api_key_mask = None;
                            draft.api_key_source = None;
                        }
                    });
                    busy.set(false);
                    status.set(lang.get_untracked().models_key_removed(&reference));
                    error.set(String::new());
                    load();
                }
                Err(message) => error.set(message),
            }
            busy.set(false);
        });
    };

    let delete = move |route: String| {
        let revision = snapshot
            .get_untracked()
            .map(|value| value.revision)
            .unwrap_or_default();
        let value = token.get_untracked();
        busy.set(true);
        spawn_local(async move {
            let path = format!("/v1/config/providers/{route}?expected_revision={revision}");
            match send("DELETE", &path, &value, None).await {
                Ok(_) => {
                    busy.set(false);
                    status.set(lang.get_untracked().models_route_removed(&route));
                    confirm.set(None);
                    error.set(String::new());
                    load();
                }
                Err(message) => error.set(message),
            }
            busy.set(false);
        });
    };

    let activate = move |provider: String, model: String, reasoning_effort: Option<String>| {
        if model.is_empty() {
            error.set(lang.get_untracked().models_labels().no_models_yet.into());
            return;
        }
        let value = token.get_untracked();
        let body = ActiveRequest {
            provider: provider.clone(),
            model: model.clone(),
            reasoning_effort: reasoning_effort.clone(),
        };
        busy.set(true);
        spawn_local(async move {
            let body = serde_json::to_value(body).unwrap_or(Value::Null);
            match send("POST", "/v1/config/model", &value, Some(&body)).await {
                Ok(_) => {
                    let current = lang.get_untracked();
                    let effort = reasoning_effort
                        .map(|level| current.models_reasoning_suffix(&level))
                        .unwrap_or_default();
                    busy.set(false);
                    status.set(current.models_active_route(&provider, &model, &effort));
                    error.set(String::new());
                    load();
                }
                Err(message) => error.set(message),
            }
            busy.set(false);
        });
    };

    let discover = move || {
        let Some(draft) = editing.get_untracked() else {
            return;
        };
        let explicit = draft.base_url.trim().to_owned();
        let base_url = if explicit.is_empty() {
            draft.default_base_url.clone().unwrap_or_default()
        } else {
            explicit
        };
        if base_url.is_empty() {
            error.set(lang.get_untracked().models_labels().enter_base_url.into());
            return;
        }
        let reference = draft.api_key_env.trim().to_owned();
        let body = json!({
            "protocol": draft.protocol,
            "base_url": base_url,
            "api_key_env": (!reference.is_empty()).then_some(reference),
        });
        let value = token.get_untracked();
        discovering.set(true);
        spawn_local(async move {
            match send("POST", "/v1/config/providers/discover", &value, Some(&body)).await {
                Ok(answer) => match serde_json::from_value::<DiscoverResponse>(answer) {
                    Ok(response) => {
                        let configured: Vec<String> = editing
                            .get_untracked()
                            .map(|draft| {
                                draft
                                    .models
                                    .iter()
                                    .map(|model| model.id.clone())
                                    .filter(|id| !id.is_empty())
                                    .collect()
                            })
                            .unwrap_or_default();
                        let available: std::collections::BTreeSet<String> = response
                            .models
                            .iter()
                            .map(|model| model.id.clone())
                            .collect();
                        candidate_checked.set(if configured.is_empty() {
                            available
                        } else {
                            configured
                                .into_iter()
                                .filter(|id| available.contains(id))
                                .collect()
                        });
                        candidate_filter.set(String::new());
                        candidates.set(Some(response.models));
                        error.set(String::new());
                    }
                    Err(problem) => {
                        error.set(
                            lang.get_untracked()
                                .models_listing_error(&problem.to_string()),
                        );
                    }
                },
                Err(message) => error.set(message),
            }
            discovering.set(false);
        });
    };

    let apply_candidates = move || {
        let Some(list) = candidates.get_untracked() else {
            return;
        };
        let Some(mut draft) = editing.get_untracked() else {
            return;
        };
        let selected = candidate_checked.get_untracked();
        match merge_discovered(&draft.models, &list, &selected, lang.get_untracked()) {
            Ok(models) => draft.models = models,
            Err(message) => {
                error.set(message);
                return;
            }
        }
        editing.set(Some(draft));
        candidates.set(None);
        candidate_checked.set(std::collections::BTreeSet::new());
        candidate_filter.set(String::new());
    };

    let acknowledge_welcome = move || {
        let value = token.get_untracked();
        let key = welcome_key.get_untracked().trim().to_owned();
        let credentials_revision = snapshot
            .get_untracked()
            .map(|value| value.credentials_revision)
            .unwrap_or_default();
        acknowledging.set(true);
        spawn_local(async move {
            if !key.is_empty() {
                let body = json!({
                    "value": key,
                    "expected_revision": credentials_revision,
                });
                if let Err(message) = send(
                    "PUT",
                    "/v1/credentials/DEEPSEEK_API_KEY",
                    &value,
                    Some(&body),
                )
                .await
                {
                    error.set(lang.get_untracked().models_key_not_stored(&message));
                    acknowledging.set(false);
                    return;
                }
            }
            match send(
                "POST",
                "/v1/config/providers/welcome",
                &value,
                Some(&json!({"version": WELCOME_VERSION})),
            )
            .await
            {
                Ok(_) => {
                    welcome_dismissed.set(true);
                    error.set(String::new());
                    load();
                }
                Err(message) => error.set(message),
            }
            acknowledging.set(false);
        });
    };

    let add_catalog = move || {
        let choice = add_choice.get_untracked();
        let Some(entry) = snapshot
            .get_untracked()
            .and_then(|value| value.catalog.into_iter().find(|entry| entry.id == choice))
        else {
            return;
        };
        editing.set(Some(ProviderDraft::from_catalog(&entry)));
        candidates.set(None);
        candidate_checked.set(std::collections::BTreeSet::new());
        error.set(String::new());
    };
    let add_custom = move || {
        editing.set(Some(ProviderDraft::new_custom()));
        candidates.set(None);
        candidate_checked.set(std::collections::BTreeSet::new());
        error.set(String::new());
    };

    view! {
        <div class="models-section">
            <p class="tool">
                {move || snapshot.get().map(|value| {
                    let labels = labels.get();
                    if value.active.provider.is_empty() {
                        labels.no_active_provider.to_owned()
                    } else {
                        lang.get_untracked().models_active_route_line(
                            &value.active.provider,
                            &value.active.model,
                            &value.note,
                        )
                    }
                }).unwrap_or_default()}
            </p>
            {move || (!error.get().is_empty()).then(|| view! {
                <p class="models-error" role="alert">{move || error.get()}</p>
            })}
            {move || {
                // The control belongs to the active model only: it edits the
                // stored selection in config.yaml, which the running server
                // resolved once at start-up.
                let selected = snapshot.get()?;
                let route = selected.active.provider.clone();
                let model = selected.active.model.clone();
                let row = selected.providers.iter().find(|row| row.id == route)?;
                let entry = row.models.iter().find(|entry| entry.id == model)?;
                let levels = offered_levels(
                    &reasoning_entries(&entry.reasoning_efforts)
                        .into_iter()
                        .map(|(level, _)| level)
                        .collect::<Vec<_>>(),
                );
                if levels.len() < 2 {
                    return None;
                }
                let current = selected.active.reasoning_effort.clone().unwrap_or_default();
                let label = entry.name.clone().unwrap_or_else(|| entry.id.clone());
                Some(view! {
                    <div class="models-field models-reasoning">
                        <label>{lang.get_untracked().models_effort_heading(&label)}</label>
                        <select
                            prop:value=current
                            on:change=move |event| {
                                let level = event_target_value(&event);
                                activate(
                                    route.clone(),
                                    model.clone(),
                                    Some(level).filter(|level| !level.is_empty()),
                                );
                            }
                            disabled=move || busy.get()
                        >
                            <option value="">{move || labels.get().provider_default}</option>
                            {levels.into_iter().map(|level| view! {
                                <option value=level.clone()>{level.clone()}</option>
                            }).collect_view()}
                        </select>
                        <p class="tool">{move || labels.get().applies_after_restart}</p>
                    </div>
                })
            }}
            <ul class="models-rows">
                {move || {
                    let Some(snapshot_value) = snapshot.get() else {
                        return view! { <li class="tool">{move || labels.get().loading}</li> }.into_any();
                    };
                    let active_snapshot = snapshot_value.active.clone();
                    let active_model = active_snapshot.model.clone();
                    snapshot_value
                        .providers
                        .into_iter()
                        .map(|row| {
                            let card_id = row.id.clone();
                            let (dot_class, dot_label) = credential_dot(&row, lang.get_untracked());
                            let active = row.active;
                            let editable = row.editable;
                            let configured = row.configured;
                            let name = row.display_name.clone();
                            let route = row.id.clone();
                            let protocol = protocol_label(&row.protocol, lang.get_untracked());
                            let models_count = row.models.len();
                            let catalog_entry = snapshot_value
                                .catalog
                                .iter()
                                .find(|entry| entry.id == row.id)
                                .cloned();
                            let draft_for_edit = ProviderDraft::from_row(&row, catalog_entry.as_ref());
                            let first_model = row
                                .models
                                .first()
                                .map(|model| model.id.clone())
                                .unwrap_or_default();
                            let use_route = row.id.clone();
                            let use_model = if row.active {
                                active_model.clone()
                            } else {
                                first_model
                            };
                            let delete_id = row.id.clone();
                            let error_text = row.error.clone().unwrap_or_default();
                            let slot_key = row.id.clone();
                            let slot_occurrence = row.id.clone();
                            let slot_context = json!({"provider": row.id.clone()}).to_string();
                            let slot_owner = json!({
                                "provider": {
                                    "id": row.id,
                                    "displayName": row.display_name,
                                    "protocol": row.protocol,
                                    "settingsNs": "",
                                    "settingsPath": [],
                                    "active": row.active,
                                    "declared": false,
                                    "defaultBaseURL": row.effective_base_url,
                                },
                                "configured": row.configured,
                                "keyConfigured": row.credential_configured.unwrap_or(false),
                            })
                            .to_string();
                            view! {
                                <li class="models-row" data-provider=card_id.clone()>
                                    <div class="models-row-head">
                                        <span class=dot_class title=dot_label></span>
                                        <span class="models-name">{name.clone()}</span>
                                        <span class="models-tag">{route.clone()}</span>
                                        {active.then(|| view! { <span class="models-tag active">{move || labels.get().active_tag}</span> })}
                                        {(!configured).then(|| view! { <span class="models-tag">{move || labels.get().defaults_tag}</span> })}
                                        <span class="models-tag">{protocol}</span>
                                        {row.models_from_profile.then(|| view! { <span class="models-tag">{move || labels.get().custom_list}</span> })}
                                        <span class="models-actions">
                                            {editable.then(|| view! {
                                                <button
                                                    on:click=move |_| {
                                                        editing.set(Some(draft_for_edit.clone()));
                                                        candidates.set(None);
                                                        candidate_checked.set(std::collections::BTreeSet::new());
                                                        error.set(String::new());
                                                    }
                                                    disabled=move || busy.get()
                                                >{move || labels.get().edit}</button>
                                                <button
                                                    on:click=move |_| activate(use_route.clone(), use_model.clone(), None)
                                                    disabled=move || busy.get()
                                                >{move || labels.get().use_route}</button>
                                            })}
                                            {row.removable.then(|| view! {
                                                <button
                                                    on:click=move |_| {
                                                        confirm.set(Some(ConfirmAction::Provider {
                                                            route: delete_id.clone(),
                                                            name: name.clone(),
                                                        }));
                                                    }
                                                    disabled=move || busy.get()
                                                >{move || labels.get().delete}</button>
                                            })}
                                        </span>
                                    </div>
                                    {(!error_text.is_empty()).then(|| view! {
                                        <p class="models-error">{error_text.clone()}</p>
                                    })}
                                    <p class="tool">
                                        {row.summary.clone()}
                                        {row.effective_base_url.as_ref().map(|url| format!(" · {url}"))}
                                        {lang.get_untracked().models_model_count(models_count)}
                                    </p>
                                    <SlotRegion
                                        region="settings.models.card"
                                        occurrence=slot_occurrence
                                        entry_key=slot_key
                                        context=slot_context
                                        owner_props=slot_owner
                                    />
                                </li>
                            }
                        })
                        .collect_view()
                        .into_any()
                }}
            </ul>

            {move || {
                let Some(draft) = editing.get() else {
                    return view! { <span></span> }.into_any();
                };
                let custom = draft.custom;
                let editor_id = draft.id.clone();
                let header_name = if custom {
                    if draft.display_name.trim().is_empty() {
                        labels.get().new_custom_provider.to_owned()
                    } else {
                        draft.display_name.clone()
                    }
                } else {
                    draft.display_name.clone()
                };
                let header_tag = if custom { "custom".to_owned() } else { draft.id.clone() };
                let protocol = draft.protocol.clone();
                let default_base = if custom {
                    if protocol == "anthropic-messages" {
                        "https://gateway.example".to_owned()
                    } else {
                        "https://gateway.example/v1".to_owned()
                    }
                } else {
                    draft.default_base_url.clone().unwrap_or_default()
                };
                let default_key = draft.default_api_key_env.clone().unwrap_or_default();
                view! {
                    <div class="models-row models-editor-card" data-provider=editor_id.clone()>
                        <div class="models-row-head">
                            <span class="models-name">{header_name}</span>
                            <span class="models-tag">{header_tag}</span>
                            <span class="models-tag">{protocol_label(&protocol, lang.get_untracked())}</span>
                            <span class="models-actions">
                                <button on:click=move |_| {
                                editing.set(None);
                                candidates.set(None);
                                candidate_checked.set(std::collections::BTreeSet::new());
                                confirm.set(None);
                                load();
                            }>{move || labels.get().cancel}</button>
                            </span>
                        </div>
                        {custom.then(|| view! {
                            <div class="models-field">
                                <label>{move || labels.get().provider_id}</label>
                                <input
                                    placeholder="acme-gateway"
                                    prop:value=move || editing.get().map(|draft| draft.id).unwrap_or_default()
                                    on:input=move |event| {
                                        let value = event_target_value(&event);
                                        editing.update(|draft| { if let Some(draft) = draft { draft.id = value; } });
                                    }
                                />
                                <span class="tool">{move || labels.get().fixed_after_creation}</span>
                            </div>
                            <div class="models-field">
                                <label>{move || labels.get().display_name}</label>
                                <input
                                    placeholder=move || labels.get().optional
                                    prop:value=move || editing.get().map(|draft| draft.display_name).unwrap_or_default()
                                    on:input=move |event| {
                                        let value = event_target_value(&event);
                                        editing.update(|draft| { if let Some(draft) = draft { draft.display_name = value; } });
                                    }
                                />
                            </div>
                            <div class="models-field">
                                <label>{move || labels.get().protocol}</label>
                                <select
                                    prop:value=move || editing.get().map(|draft| draft.protocol).unwrap_or_default()
                                    on:change=move |event| {
                                        let value = event_target_value(&event);
                                        editing.update(|draft| { if let Some(draft) = draft { draft.protocol = value; } });
                                    }
                                >
                                    <option value="openai-chat">{move || labels.get().openai_chat_completions}</option>
                                    <option value="anthropic-messages">{move || labels.get().anthropic_messages}</option>
                                </select>
                            </div>
                        })}
                        <div class="models-field">
                            <label>{move || labels.get().base_url}</label>
                            <input
                                placeholder=default_base
                                prop:value=move || editing.get().map(|draft| draft.base_url).unwrap_or_default()
                                on:input=move |event| {
                                    let value = event_target_value(&event);
                                    editing.update(|draft| { if let Some(draft) = draft { draft.base_url = value; } });
                                }
                            />
                        </div>
                        <div class="models-field">
                            <label>{move || labels.get().api_key_variable}</label>
                            <input
                                placeholder=default_key
                                prop:value=move || editing.get().map(|draft| draft.api_key_env).unwrap_or_default()
                                on:input=move |event| {
                                    let value = event_target_value(&event);
                                    editing.update(|draft| { if let Some(draft) = draft { draft.api_key_env = value; } });
                                }
                            />
                            <span class="tool">{move || labels.get().value_lives_in_credentials}</span>
                        </div>
                        {move || {
                            let Some(draft) = editing.get() else {
                                return view! { <span></span> }.into_any();
                            };
                            let current = lang.get_untracked();
                            let placeholder = if draft.api_key_saved {
                                current.models_saved_bare(&draft.api_key_mask.clone().unwrap_or_default())
                            } else {
                                current.models_labels().not_configured.to_owned()
                            };
                            let status = if draft.api_key_saved {
                                current.models_saved_env_wins(
                                    &draft.api_key_mask.clone().unwrap_or_default(),
                                    &draft.api_key_source.clone().unwrap_or_default(),
                                )
                            } else {
                                current.models_labels().no_saved_key.to_owned()
                            };
                            view! {
                                <div class="models-field">
                                    <label>{move || labels.get().api_key}</label>
                                    <input
                                        type=move || if api_key_visible.get() { "text" } else { "password" }
                                        placeholder=placeholder
                                        autocomplete="new-password"
                                        prop:value=move || editing.get().map(|draft| draft.api_key).unwrap_or_default()
                                        on:input=move |event| {
                                            let value = event_target_value(&event);
                                            editing.update(|draft| { if let Some(draft) = draft { draft.api_key = value; } });
                                        }
                                    />
                                    <button
                                        type="button"
                                        on:click=move |_| api_key_visible.update(|visible| *visible = !*visible)
                                    >
                                        {move || if api_key_visible.get() { labels.get().hide } else { labels.get().show }}
                                    </button>
                                </div>
                                <p class="tool">{status}</p>
                                {draft.api_key_saved.then(|| view! {
                                    <div class="row">
                                        <button
                                            on:click=move |_| remove_saved_key()
                                            disabled=move || busy.get()
                                        >{move || labels.get().remove_saved_key}</button>
                                    </div>
                                })}
                            }.into_any()
                        }}
                        <div class="models-retry">
                            <details>
                                <summary>
                                    <span>{move || labels.get().retry_policy}</span>
                                    <span class="models-tag">
                                        {move || editing.get()
                                            .and_then(|draft| draft.retry)
                                            .map(|retry| retry.mode)
                                            .unwrap_or_else(|| "inherited".to_owned())}
                                    </span>
                                </summary>
                                <div class="models-field">
                                    <label>{move || labels.get().mode}</label>
                                    <select
                                        prop:value=move || editing.get()
                                            .and_then(|draft| draft.retry)
                                            .map(|retry| retry.mode)
                                            .unwrap_or_else(|| "inherit".to_owned())
                                        on:change=move |event| {
                                            let selected = event_target_value(&event);
                                            editing.update(|draft| {
                                                if let Some(draft) = draft {
                                                    if selected == "inherit" {
                                                        draft.retry = None;
                                                    } else {
                                                        let retry = draft.retry.get_or_insert_with(RetryDraft::defaults);
                                                        retry.mode = selected;
                                                    }
                                                }
                                            });
                                        }
                                    >
                                        <option value="inherit">{move || labels.get().inherited_adapter}</option>
                                        <option value="normal">{move || labels.get().transient_only}</option>
                                        <option value="always">{move || labels.get().always_retry}</option>
                                    </select>
                                </div>
                                {move || {
                                    let Some(retry) = editing.get().and_then(|draft| draft.retry) else {
                                        return view! { <span></span> }.into_any();
                                    };
                                    view! {
                                        {(retry.mode == "normal").then(|| view! {
                                            <div class="models-field">
                                                <label>{move || labels.get().max_retries}</label>
                                                <input
                                                    prop:value=retry.max_retries.clone()
                                                    on:input=move |event| {
                                                        let value = event_target_value(&event);
                                                        editing.update(|draft| {
                                                            if let Some(draft) = draft
                                                                && let Some(retry) = draft.retry.as_mut()
                                                            {
                                                                retry.max_retries = value;
                                                            }
                                                        });
                                                    }
                                                />
                                            </div>
                                            <div class="models-codes">
                                                {RETRY_CODE_CHOICES.iter().map(|code| {
                                                    let code_value = (*code).to_owned();
                                                    let checked = retry.codes.iter().any(|selected| selected == code);
                                                    let label = retry_code_label(code, &labels.get());
                                                    view! {
                                                        <label class="models-check">
                                                            <input
                                                                type="checkbox"
                                                                prop:checked=checked
                                                                on:change=move |event| {
                                                                    let checked = event_target_checked(&event);
                                                                    let code = code_value.clone();
                                                                    editing.update(|draft| {
                                                                        if let Some(draft) = draft
                                                                            && let Some(retry) = draft.retry.as_mut()
                                                                        {
                                                                            if checked {
                                                                                if !retry.codes.contains(&code) {
                                                                                    retry.codes.push(code.clone());
                                                                                }
                                                                            } else {
                                                                                retry.codes.retain(|selected| selected != &code);
                                                                            }
                                                                        }
                                                                    });
                                                                }
                                                            />
                                                            {label}
                                                        </label>
                                                    }
                                                }).collect_view()}
                                            </div>
                                        })}
                                        <div class="models-field">
                                            <label>{move || labels.get().initial_delay_ms}</label>
                                            <input
                                                prop:value=retry.initial_delay.clone()
                                                on:input=move |event| {
                                                    let value = event_target_value(&event);
                                                    editing.update(|draft| {
                                                        if let Some(draft) = draft
                                                            && let Some(retry) = draft.retry.as_mut()
                                                        {
                                                            retry.initial_delay = value;
                                                        }
                                                    });
                                                }
                                            />
                                        </div>
                                        <div class="models-field">
                                            <label>{move || labels.get().max_delay_ms}</label>
                                            <input
                                                prop:value=retry.max_delay.clone()
                                                on:input=move |event| {
                                                    let value = event_target_value(&event);
                                                    editing.update(|draft| {
                                                        if let Some(draft) = draft
                                                            && let Some(retry) = draft.retry.as_mut()
                                                        {
                                                            retry.max_delay = value;
                                                        }
                                                    });
                                                }
                                            />
                                        </div>
                                        <div class="models-field">
                                            <label>{move || labels.get().jitter_ratio}</label>
                                            <input
                                                prop:value=retry.jitter.clone()
                                                on:input=move |event| {
                                                    let value = event_target_value(&event);
                                                    editing.update(|draft| {
                                                        if let Some(draft) = draft
                                                            && let Some(retry) = draft.retry.as_mut()
                                                        {
                                                            retry.jitter = value;
                                                        }
                                                    });
                                                }
                                            />
                                        </div>
                                    }.into_any()
                                }}
                            </details>
                        </div>
                        <div class="models-models-head">
                            <strong>{move || labels.get().models}</strong>
                            <button
                                on:click=move |_| discover()
                                disabled=move || busy.get() || discovering.get()
                            >
                                {move || if discovering.get() { labels.get().fetching } else { labels.get().fetch_available }}
                            </button>
                            <span class="tool">
                                {if custom {
                                    labels.get().custom_needs_one_model
                                } else {
                                    labels.get().empty_list_inherits
                                }}
                            </span>
                        </div>
                        {move || {
                            let Some(list) = candidates.get() else {
                                return view! { <span></span> }.into_any();
                            };
                            let filter = candidate_filter.get().to_lowercase();
                            let visible: Vec<DiscoveredModel> = list
                                .into_iter()
                                .filter(|model| {
                                    filter.is_empty()
                                        || model.id.to_lowercase().contains(&filter)
                                        || model.name.to_lowercase().contains(&filter)
                                })
                                .collect();
                            let checked_set = candidate_checked.get();
                            view! {
                                <div class="models-picker">
                                    <div class="models-picker-head">
                                        <strong>{lang.get_untracked().models_available_count(visible.len())}</strong>
                                        <input
                                            placeholder=move || labels.get().filter_models
                                            prop:value=move || candidate_filter.get()
                                            on:input=move |event| candidate_filter.set(event_target_value(&event))
                                        />
                                        <button on:click=move |_| {
                                            let ids = candidates.get_untracked().unwrap_or_default()
                                                .into_iter().map(|model| model.id).collect();
                                            candidate_checked.set(ids);
                                        }>{move || labels.get().select_all}</button>
                                        <button on:click=move |_| candidate_checked.set(std::collections::BTreeSet::new())>{move || labels.get().deselect_all}</button>
                                    </div>
                                    <div class="models-candidates">
                                        {visible.into_iter().map(|candidate| {
                                            let id = candidate.id.clone();
                                            let label = id.clone();
                                            let checked = checked_set.contains(&id);
                                            view! {
                                                <label class="models-check models-candidate">
                                                    <input
                                                        type="checkbox"
                                                        prop:checked=checked
                                                        on:change=move |event| {
                                                            let checked = event_target_checked(&event);
                                                            let id = id.clone();
                                                            candidate_checked.update(|set| {
                                                                if checked {
                                                                    set.insert(id.clone());
                                                                } else {
                                                                    set.remove(&id);
                                                                }
                                                            });
                                                        }
                                                    />
                                                    <span class="model-id">{label}</span>
                                                    <span class="tool">{candidate.name}</span>
                                                </label>
                                            }
                                        }).collect_view()}
                                    </div>
                                    <div class="row">
                                        <button on:click=move |_| apply_candidates()>{move || labels.get().update_list}</button>
                                        <button on:click=move |_| {
                                            candidates.set(None);
                                            candidate_checked.set(std::collections::BTreeSet::new());
                                        }>{move || labels.get().cancel}</button>
                                    </div>
                                </div>
                            }.into_any()
                        }}
                        <div class="models-model-list">
                            {move || {
                                let Some(draft) = editing.get() else {
                                    return view! { <span></span> }.into_any();
                                };
                                let custom = draft.custom;
                                let provider_name = if draft.display_name.trim().is_empty() {
                                    draft.id.clone()
                                } else {
                                    draft.display_name.clone()
                                };
                                draft.models.iter().enumerate().map(|(index, model)| {
                                    let id_value = model.id.clone();
                                    let name_value = model.name.clone();
                                    let context_value = model.context_window.clone();
                                    let output_value = model.max_tokens.clone();
                                    let text = model.text;
                                    let image = model.image;
                                    let model_id_for_use = model.id.clone();
                                    let provider_for_use = draft.id.clone();
                                    let provider_name = provider_name.clone();
                                    let model_for_remove = model_id_for_use.clone();
                                    let can_use = !custom && !model_id_for_use.trim().is_empty();
                                    let reasoning_on = reasoning_enabled(&model.reasoning_efforts);
                                    let level_rows = reasoning_entries(&model.reasoning_efforts);
                                    view! {
                                        <div class="models-model-block">
                                        <div class="models-model-row">
                                            <input
                                                class="model-id"
                                                placeholder=move || labels.get().model_id
                                                prop:value=id_value
                                                on:input=move |event| {
                                                    let value = event_target_value(&event);
                                                    editing.update(|draft| {
                                                        if let Some(draft) = draft
                                                            && let Some(model) = draft.models.get_mut(index)
                                                        {
                                                            model.id = value;
                                                        }
                                                    });
                                                }
                                            />
                                            <input
                                                placeholder=move || labels.get().display_name
                                                prop:value=name_value
                                                on:input=move |event| {
                                                    let value = event_target_value(&event);
                                                    editing.update(|draft| {
                                                        if let Some(draft) = draft
                                                            && let Some(model) = draft.models.get_mut(index)
                                                        {
                                                            model.name = value;
                                                        }
                                                    });
                                                }
                                            />
                                            <input
                                                class="model-number"
                                                placeholder=move || labels.get().context
                                                prop:value=context_value
                                                on:input=move |event| {
                                                    let value = event_target_value(&event);
                                                    editing.update(|draft| {
                                                        if let Some(draft) = draft
                                                            && let Some(model) = draft.models.get_mut(index)
                                                        {
                                                            model.context_window = value;
                                                        }
                                                    });
                                                }
                                            />
                                            <input
                                                class="model-number"
                                                placeholder=move || labels.get().max_out
                                                prop:value=output_value
                                                on:input=move |event| {
                                                    let value = event_target_value(&event);
                                                    editing.update(|draft| {
                                                        if let Some(draft) = draft
                                                            && let Some(model) = draft.models.get_mut(index)
                                                        {
                                                            model.max_tokens = value;
                                                        }
                                                    });
                                                }
                                            />
                                            <label class="models-check">
                                                <input
                                                    type="checkbox"
                                                    prop:checked=text
                                                    on:change=move |event| {
                                                        let checked = event_target_checked(&event);
                                                        editing.update(|draft| {
                                                            if let Some(draft) = draft
                                                                && let Some(model) = draft.models.get_mut(index)
                                                            {
                                                                model.text = checked;
                                                            }
                                                        });
                                                    }
                                                />
                                                "text"
                                            </label>
                                            <label class="models-check">
                                                <input
                                                    type="checkbox"
                                                    prop:checked=image
                                                    on:change=move |event| {
                                                        let checked = event_target_checked(&event);
                                                        editing.update(|draft| {
                                                            if let Some(draft) = draft
                                                                && let Some(model) = draft.models.get_mut(index)
                                                            {
                                                                model.image = checked;
                                                            }
                                                        });
                                                    }
                                                />
                                                "image"
                                            </label>
                                            <button
                                                title=move || labels.get().make_active
                                                disabled=move || busy.get() || !can_use
                                                on:click=move |_| {
                                                    // A level is declared per model, so switching
                                                    // resets it to the provider default.
                                                    activate(
                                                        provider_for_use.clone(),
                                                        model_id_for_use.clone(),
                                                        None,
                                                    );
                                                }
                                            >{move || labels.get().use_route}</button>
                                            <button
                                                title=move || labels.get().remove_row
                                                on:click=move |_| {
                                                    confirm.set(Some(ConfirmAction::Model {
                                                        name: provider_name.clone(),
                                                        index,
                                                        model_id: model_for_remove.clone(),
                                                    }));
                                                }
                                            >"×"</button>
                                        </div>
                                        <div class="models-reasoning">
                                            <label class="models-check">
                                                <input
                                                    type="checkbox"
                                                    prop:checked=reasoning_on
                                                    on:change=move |event| {
                                                        let checked = event_target_checked(&event);
                                                        let seed = snapshot.get_untracked();
                                                        editing.update(|draft| {
                                                            if let Some(draft) = draft
                                                                && let Some(model) = draft.models.get_mut(index)
                                                            {
                                                                let route = draft.id.clone();
                                                                model.reasoning_efforts = Some(if checked {
                                                                    reasoning_seed(
                                                                        seed.as_ref(),
                                                                        &route,
                                                                        &model.id,
                                                                    )
                                                                } else {
                                                                    json!(false)
                                                                });
                                                            }
                                                        });
                                                    }
                                                />
                                                {move || labels.get().reasoning_levels}
                                            </label>
                                            {reasoning_on.then(|| view! {
                                                <div class="models-levels">
                                                    {level_rows.iter().enumerate().map(|(level_index, (level, value))| {
                                                        let level_value = level.clone();
                                                        let api_value = value.clone();
                                                        view! {
                                                            <div class="models-level-row">
                                                                <select
                                                                    prop:value=level_value
                                                                    on:change=move |event| {
                                                                        let selected = event_target_value(&event);
                                                                        editing.update(|draft| {
                                                                            if let Some(draft) = draft
                                                                                && let Some(model) = draft.models.get_mut(index)
                                                                            {
                                                                                let mut rows = reasoning_entries(&model.reasoning_efforts);
                                                                                let duplicate = rows.iter().enumerate().any(|(position, (used, _))| position != level_index && *used == selected);
                                                                                if !duplicate && let Some(row) = rows.get_mut(level_index) {
                                                                                    row.0 = selected;
                                                                                }
                                                                                let original = model.reasoning_efforts.clone();
                                                                                model.reasoning_efforts = Some(reasoning_object_with(&rows, &original));
                                                                            }
                                                                        });
                                                                    }
                                                                >
                                                                    {REASONING_LEVELS.iter().map(|choice| view! { <option value=*choice>{*choice}</option> }).collect_view()}
                                                                </select>
                                                                <input
                                                                    placeholder=move || labels.get().api_value
                                                                    prop:value=api_value
                                                                    on:input=move |event| {
                                                                        let value = event_target_value(&event);
                                                                        editing.update(|draft| {
                                                                            if let Some(draft) = draft
                                                                                && let Some(model) = draft.models.get_mut(index)
                                                                            {
                                                                                let mut rows = reasoning_entries(&model.reasoning_efforts);
                                                                                if let Some(row) = rows.get_mut(level_index) {
                                                                                    row.1 = value;
                                                                                }
                                                                                let original = model.reasoning_efforts.clone();
                                                                                model.reasoning_efforts = Some(reasoning_object_with(&rows, &original));
                                                                            }
                                                                        });
                                                                    }
                                                                />
                                                                <button on:click=move |_| {
                                                                    editing.update(|draft| {
                                                                        if let Some(draft) = draft
                                                                            && let Some(model) = draft.models.get_mut(index)
                                                                        {
                                                                            let mut rows = reasoning_entries(&model.reasoning_efforts);
                                                                            if level_index < rows.len() {
                                                                                let _ = rows.remove(level_index);
                                                                            }
                                                                            let original = model.reasoning_efforts.clone();
                                                                            model.reasoning_efforts = Some(reasoning_object_with(&rows, &original));
                                                                        }
                                                                    });
                                                                }>"×"</button>
                                                            </div>
                                                        }
                                                    }).collect_view()}
                                                    <div class="row">
                                                        <button on:click=move |_| {
                                                            editing.update(|draft| {
                                                                if let Some(draft) = draft
                                                                    && let Some(model) = draft.models.get_mut(index)
                                                                {
                                                                    let mut rows = reasoning_entries(&model.reasoning_efforts);
                                                                    if let Some(level) = REASONING_LEVELS.iter().find(|candidate| !rows.iter().any(|(used, _)| used == *candidate)) {
                                                                        rows.push(((*level).to_owned(), (*level).to_owned()));
                                                                    }
                                                                    let original = model.reasoning_efforts.clone();
                                                                    model.reasoning_efforts = Some(reasoning_object_with(&rows, &original));
                                                                }
                                                            });
                                                        }>{move || labels.get().add_level}</button>
                                                        {level_rows.is_empty().then(|| view! { <span class="tool">{move || labels.get().at_least_one_level}</span> })}
                                                    </div>
                                                </div>
                                            })}
                                        </div>
                                        </div>
                                    }
                                }).collect_view().into_any()
                            }}
                        </div>
                        <div class="row">
                            <button on:click=move |_| {
                                editing.update(|draft| {
                                    if let Some(draft) = draft {
                                        draft.models.push(ModelDraft {
                                            id: String::new(),
                                            name: String::new(),
                                            context_window: String::new(),
                                            max_tokens: String::new(),
                                            text: true,
                                            image: false,
                                            reasoning_efforts: None,
                                        });
                                    }
                                });
                            }>{move || labels.get().add_model}</button>
                            {move || (!editing.get().is_some_and(|draft| draft.custom)).then(|| view! {
                                <button on:click=move |_| {
                                    editing.update(|draft| { if let Some(draft) = draft { draft.models.clear(); } });
                                }>{move || labels.get().restore_catalog}</button>
                            })}
                        </div>
                        <div class="row">
                            <button
                                class="primary"
                                on:click=move |_| save()
                                disabled=move || busy.get()
                            >{move || labels.get().save}</button>
                            <button on:click=move |_| {
                                editing.set(None);
                                candidates.set(None);
                                candidate_checked.set(std::collections::BTreeSet::new());
                                confirm.set(None);
                                load();
                            }>{move || labels.get().cancel}</button>
                        </div>
                    </div>
                }.into_any()
            }}

            <div class="row models-add">
                <div class="models-mode">
                    <button
                        class:active=move || add_mode.get() == "catalog"
                        on:click=move |_| add_mode.set("catalog".into())
                    >{move || labels.get().catalog_provider}</button>
                    <button
                        class:active=move || add_mode.get() == "custom"
                        on:click=move |_| {
                            add_mode.set("custom".into());
                            if !editing.get_untracked().is_some_and(|draft| draft.custom) {
                                add_custom();
                            }
                        }
                    >{move || labels.get().custom_model_api}</button>
                </div>
                {move || (add_mode.get() == "catalog").then(|| view! {
                    <select
                        prop:value=move || add_choice.get()
                        on:change=move |event| add_choice.set(event_target_value(&event))
                    >
                        <option value="">{move || labels.get().choose_catalog_provider}</option>
                        {move || snapshot.get().map(|value| {
                            value.catalog.into_iter().map(|entry| {
                                let id = entry.id.clone();
                                let label = format!("{} · {}", entry.display_name, entry.id);
                                view! { <option value=id>{label}</option> }
                            }).collect_view()
                        }).unwrap_or_default()}
                    </select>
                })}
                {move || (add_mode.get() == "catalog").then(|| view! {
                    <button on:click=move |_| add_catalog() disabled=move || busy.get()>{move || labels.get().add}</button>
                })}
                {move || (add_mode.get() == "custom").then(|| view! {
                    <button on:click=move |_| add_custom() disabled=move || busy.get()>{move || labels.get().new_custom_provider}</button>
                })}
                <span class="tool">{move || if busy.get() { labels.get().working } else { "" }}</span>
            </div>
            {move || {
                let show = !welcome_dismissed.get()
                    && snapshot.get().is_some_and(|value| {
                        value.welcome_version.unwrap_or(0) < WELCOME_VERSION
                    });
                if !show {
                    return view! { <span></span> }.into_any();
                }
                let usable = snapshot.get().is_some_and(|value| {
                    value
                        .providers
                        .iter()
                        .any(|row| row.credential_configured == Some(true))
                });
                view! {
                    <div class="models-modal models-welcome" role="presentation">
                        <div class="models-modal-card" role="dialog" aria-modal="true" aria-label=move || labels.get().welcome_title>
                            <h4>{move || labels.get().welcome_title}</h4>
                            <p>{move || labels.get().welcome_intro}</p>
                            {(!usable).then(|| view! {
                                <p>{move || labels.get().welcome_no_key}</p>
                                <div class="models-field">
                                    <label>{move || labels.get().deepseek_api_key}</label>
                                    <input
                                        type="password"
                                        autocomplete="new-password"
                                        placeholder="DEEPSEEK_API_KEY"
                                        prop:value=move || welcome_key.get()
                                        on:input=move |event| welcome_key.set(event_target_value(&event))
                                    />
                                    <span class="tool">{move || labels.get().stored_in_credentials}</span>
                                </div>
                            })}
                            <div class="row">
                                <button
                                    class="primary"
                                    on:click=move |_| acknowledge_welcome()
                                    disabled=move || acknowledging.get()
                                >{move || labels.get().continue_label}</button>
                            </div>
                        </div>
                    </div>
                }.into_any()
            }}
            {move || confirm.get().map(|action| {
                let (title, body) = match &action {
                    ConfirmAction::Provider { name, .. } => (
                        labels.get().delete_provider.to_owned(),
                        lang.get_untracked().models_delete_provider_confirm(&name),
                    ),
                    ConfirmAction::Model { name, model_id, .. } => (
                        labels.get().delete_model.to_owned(),
                        lang.get_untracked().models_delete_model_confirm(&model_id, &name),
                    ),
                };
                let action_for_delete = action.clone();
                let aria_label = title.clone();
                view! {
                    <div class="models-modal" role="presentation">
                        <div class="models-modal-card" role="dialog" aria-modal="true" aria-label=aria_label>
                            <h4>{title}</h4>
                            <p>{body}</p>
                            <div class="row">
                                <button
                                    class="danger"
                                    disabled=move || busy.get()
                                    on:click=move |_| {
                                        match &action_for_delete {
                                            ConfirmAction::Provider { route, .. } => delete(route.clone()),
                                            ConfirmAction::Model { index, .. } => {
                                                editing.update(|draft| {
                                                    if let Some(draft) = draft
                                                        && *index < draft.models.len()
                                                    {
                                                        let _ = draft.models.remove(*index);
                                                    }
                                                });
                                            }
                                        }
                                        confirm.set(None);
                                    }
                                >{move || labels.get().delete}</button>
                                <button on:click=move |_| confirm.set(None)>{move || labels.get().cancel}</button>
                            </div>
                        </div>
                    </div>
                }
            })}
            <SlotRegion region="settings.models.footer" />
            <p class="tool models-status">{move || status.get()}</p>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draft() -> ProviderDraft {
        ProviderDraft {
            id: "deepseek-official".into(),
            display_name: "DeepSeek API".into(),
            protocol: "anthropic-messages".into(),
            base_url: String::new(),
            api_key_env: "DEEPSEEK_API_KEY".into(),
            models: vec![ModelDraft {
                id: "deepseek-flash".into(),
                name: "Flash".into(),
                context_window: "1000000".into(),
                max_tokens: String::new(),
                text: true,
                image: true,
                reasoning_efforts: Some(json!({"high": "high"})),
            }],
            retry: None,
            api_key: String::new(),
            api_key_saved: false,
            api_key_mask: None,
            api_key_source: None,
            configured: false,
            custom: false,
            account: false,
            default_base_url: Some("https://api.deepseek.com/anthropic".into()),
            default_api_key_env: Some("DEEPSEEK_API_KEY".into()),
        }
    }

    #[test]
    fn draft_body_rejects_bad_endpoints_keys_and_models() {
        let mut value = draft();
        value.base_url = "ftp://example".into();
        assert!(draft_body(&value, Lang::En).is_err());

        let mut value = draft();
        value.api_key_env = "lower-case".into();
        assert!(draft_body(&value, Lang::En).is_err());

        let mut value = draft();
        value.models[0].context_window = "1.5".into();
        assert!(draft_body(&value, Lang::En).is_err());

        let mut value = draft();
        value.models.push(value.models[0].clone());
        assert!(draft_body(&value, Lang::En).is_err());
    }

    #[test]
    fn draft_body_preserves_reasoning_levels_and_input_types() {
        let body = draft_body(&draft(), Lang::En).expect("valid draft");
        assert_eq!(body["protocol"], "anthropic-messages");
        assert_eq!(body["api_key_env"], "DEEPSEEK_API_KEY");
        assert!(body["base_url"].is_null());
        assert_eq!(body["models"][0]["context_window"], 1_000_000);
        assert_eq!(body["models"][0]["input"], json!(["text", "image"]));
        assert_eq!(body["models"][0]["reasoning_efforts"]["high"], "high");
    }

    #[test]
    fn a_catalog_draft_starts_from_its_default_models() {
        let entry = CatalogEntry {
            id: "openai".into(),
            display_name: "OpenAI".into(),
            protocol: "openai-chat".into(),
            default_base_url: Some("https://api.openai.com/v1".into()),
            api_key_env: Some("OPENAI_API_KEY".into()),
            models: vec![
                CatalogModel {
                    id: "gpt-4o-mini".into(),
                    name: "GPT-4o mini".into(),
                    ..CatalogModel::default()
                },
                CatalogModel {
                    id: "gpt-4o".into(),
                    name: "GPT-4o".into(),
                    ..CatalogModel::default()
                },
            ],
            ..CatalogEntry::default()
        };
        let draft = ProviderDraft::from_catalog(&entry);
        assert_eq!(draft.models.len(), 2);
        assert_eq!(draft.models[1].id, "gpt-4o");
        assert_eq!(draft.api_key_env, "OPENAI_API_KEY");
        assert!(draft.models.iter().all(|model| model.text));
        // The OpenAI routes declare no levels, so the draft keeps the field
        // absent rather than inventing a declaration.
        assert!(
            draft
                .models
                .iter()
                .all(|model| model.reasoning_efforts.is_none())
        );
    }

    #[test]
    fn a_catalog_draft_carries_the_levels_the_catalog_declares() {
        let levels = ["off", "low", "high", "max"];
        let entry = CatalogEntry {
            id: "deepseek-official".into(),
            display_name: "DeepSeek API".into(),
            protocol: "anthropic-messages".into(),
            models: vec![CatalogModel {
                id: "deepseek-flash".into(),
                name: "DeepSeek Flash".into(),
                reasoning_levels: levels.iter().map(|level| (*level).to_owned()).collect(),
            }],
            ..CatalogEntry::default()
        };
        let draft = ProviderDraft::from_catalog(&entry);
        let efforts = draft.models[0]
            .reasoning_efforts
            .clone()
            .expect("a declaration");
        assert!(reasoning_enabled(&Some(efforts.clone())));
        let entries = reasoning_entries(&Some(efforts));
        // The map itself does not carry an order; the control reimposes the
        // escalation order the catalog declared.
        let declared: Vec<String> = entries.iter().map(|(level, _)| level.clone()).collect();
        assert_eq!(offered_levels(&declared), levels);
        assert!(entries.iter().all(|(level, value)| level == value));
        // The levels the catalog leaves out are declared unsupported, so the
        // config layer's defaults cannot offer them.
        let full = draft.models[0]
            .reasoning_efforts
            .clone()
            .expect("a declaration");
        for level in REASONING_LEVELS {
            if levels.contains(&level) {
                assert_eq!(full[level], Value::String(level.to_owned()), "{level}");
            } else {
                assert!(full[level].is_null(), "{level} must be declared null");
            }
        }
    }

    fn levels_of(value: &Value) -> Vec<String> {
        let declared: Vec<String> = reasoning_entries(&Some(value.clone()))
            .into_iter()
            .map(|(level, _)| level)
            .collect();
        offered_levels(&declared)
    }

    #[test]
    fn turning_reasoning_on_follows_the_catalog() {
        let snapshot: ModelsSnapshot = serde_json::from_value(json!({
            "catalog": [{
                "id": "deepseek-account",
                "models": [{
                    "id": "deepseek-flash",
                    "reasoning_levels": ["off", "low", "high", "max"]
                }]
            }]
        }))
        .expect("a catalog snapshot");

        // The catalog's list wins: `medium` would be rejected by the endpoint
        // and `max` would otherwise be dropped.
        let seeded = reasoning_seed(Some(&snapshot), "deepseek-account", "deepseek-flash");
        assert_eq!(levels_of(&seeded), ["off", "low", "high", "max"]);
        assert!(seeded["medium"].is_null());
        assert!(seeded["minimal"].is_null());

        // A model the catalog does not describe keeps the generic default, as
        // does a snapshot that has not arrived yet.
        assert_eq!(
            levels_of(&reasoning_seed(
                Some(&snapshot),
                "deepseek-account",
                "custom-model"
            )),
            ["off", "low", "medium", "high"]
        );
        assert_eq!(
            levels_of(&reasoning_seed(None, "deepseek-account", "deepseek-flash")),
            ["off", "low", "medium", "high"]
        );
    }

    #[test]
    fn reasoning_rows_round_trip_and_refuse_bad_declarations() {
        let entries = vec![
            ("off".to_owned(), "off".to_owned()),
            ("high".to_owned(), "high".to_owned()),
        ];
        let value = reasoning_object(&entries);
        assert!(reasoning_enabled(&Some(value.clone())));
        let round = reasoning_entries(&Some(value));
        assert_eq!(round.len(), 2);
        assert!(round.contains(&("off".to_owned(), "off".to_owned())));
        assert!(round.contains(&("high".to_owned(), "high".to_owned())));

        let mut value = draft();
        value.models[0].reasoning_efforts = Some(json!({}));
        assert!(draft_body(&value, Lang::En).is_err());

        let mut value = draft();
        value.models[0].reasoning_efforts = Some(json!({"banana": "high"}));
        assert!(draft_body(&value, Lang::En).is_err());

        let mut value = draft();
        value.models[0].reasoning_efforts = Some(json!({"high": "  "}));
        assert!(draft_body(&value, Lang::En).is_err());

        let mut value = draft();
        value.models[0].reasoning_efforts = Some(json!(true));
        assert!(draft_body(&value, Lang::En).is_err());

        let mut value = draft();
        value.models[0].reasoning_efforts = Some(json!(false));
        assert!(draft_body(&value, Lang::En).is_ok());
    }

    #[test]
    fn an_unsupported_level_survives_an_edit_and_still_saves() {
        let original = Some(json!({"off": "off", "medium": null, "high": "high"}));
        // The editor only renders the levels that carry a value.
        let mut rows = reasoning_entries(&original);
        assert_eq!(rows.len(), 2);
        let high = rows
            .iter()
            .position(|(level, _)| level == "high")
            .expect("the supported high level renders");
        rows[high].1 = "gpt-high".to_owned();
        let saved = reasoning_object_with(&rows, &original);
        assert_eq!(saved["medium"], Value::Null);
        assert_eq!(saved["high"], "gpt-high");
        assert_eq!(saved["off"], "off");

        // The server accepts the post: `null` is a declaration, not an error.
        let mut value = draft();
        value.models[0].reasoning_efforts = Some(saved);
        assert!(draft_body(&value, Lang::En).is_ok());
    }

    #[test]
    fn offered_levels_follow_the_declaration_and_the_escalation_order() {
        // Declared order in the map is not the order the control shows.
        let declared = vec!["high".to_owned(), "off".to_owned(), "low".to_owned()];
        assert_eq!(offered_levels(&declared), ["off", "low", "high"]);

        // A model that declares nothing gets no control at all.
        assert!(offered_levels(&[]).is_empty());
        // One level is not a choice.
        assert_eq!(offered_levels(&["off".to_owned()]), ["off"]);
        // Unknown declarations are dropped rather than shown and rejected later.
        assert_eq!(
            offered_levels(&["banana".to_owned(), "max".to_owned()]),
            ["max"]
        );
    }

    #[test]
    fn an_active_request_only_sends_a_selected_level() {
        let default = serde_json::to_value(ActiveRequest {
            provider: "deepseek".into(),
            model: "deepseek-flash".into(),
            reasoning_effort: None,
        })
        .expect("body");
        assert_eq!(
            default,
            json!({"provider": "deepseek", "model": "deepseek-flash"})
        );

        let selected = serde_json::to_value(ActiveRequest {
            provider: "deepseek".into(),
            model: "deepseek-flash".into(),
            reasoning_effort: Some("high".into()),
        })
        .expect("body");
        assert_eq!(
            selected,
            json!({"provider": "deepseek", "model": "deepseek-flash", "reasoning_effort": "high"})
        );
    }

    #[test]
    fn the_active_selection_reads_the_stored_level() {
        let selection: ActiveSelection = serde_json::from_value(json!({
            "provider": "deepseek",
            "model": "deepseek-flash",
            "reasoning_effort": "max"
        }))
        .expect("selection");
        assert_eq!(selection.reasoning_effort.as_deref(), Some("max"));

        // Older servers omit the field; that is the provider default.
        let selection: ActiveSelection =
            serde_json::from_value(json!({"provider": "deepseek", "model": "flash"}))
                .expect("selection");
        assert_eq!(selection.reasoning_effort, None);
    }

    fn candidate(id: &str) -> DiscoveredModel {
        DiscoveredModel {
            id: id.to_owned(),
            name: id.to_owned(),
        }
    }

    fn model_row(id: &str) -> ModelDraft {
        ModelDraft {
            id: id.to_owned(),
            name: String::new(),
            context_window: String::new(),
            max_tokens: String::new(),
            text: true,
            image: false,
            reasoning_efforts: None,
        }
    }

    #[test]
    fn discovery_merges_checked_candidates_and_keeps_unlisted_rows() {
        let selected = std::collections::BTreeSet::from(["new".to_owned()]);
        let merged = merge_discovered(
            &[model_row("kept"), model_row("listed")],
            &[candidate("listed"), candidate("new")],
            &selected,
            Lang::En,
        )
        .expect("merge");
        let ids: Vec<&str> = merged.iter().map(|model| model.id.as_str()).collect();
        assert_eq!(ids, vec!["kept", "new"]);

        let empty =
            merge_discovered(&[], &[candidate("new")], &selected, Lang::En).expect("first add");
        assert_eq!(empty[0].id, "new");

        let refused = merge_discovered(
            &[model_row("listed")],
            &[candidate("listed")],
            &std::collections::BTreeSet::new(),
            Lang::En,
        );
        assert!(refused.is_err());
    }

    #[test]
    fn derives_conventional_key_references() {
        assert_eq!(
            derive_key_ref("deepseek-official"),
            "DEEPSEEK_OFFICIAL_API_KEY"
        );
        assert_eq!(derive_key_ref("my_gateway"), "MY_GATEWAY_API_KEY");
        assert_eq!(derive_key_ref("a.b c"), "A_B_C_API_KEY");
    }

    #[test]
    fn retry_drafts_round_trip_and_refuse_invalid_values() {
        let stored = json!({
            "mode": "normal",
            "max_retries": 3,
            "retryable_codes": ["RATE_LIMIT", "SERVER"],
            "backoff": {"initial_delay_ms": 250, "max_delay_ms": 2500, "jitter_ratio": 0.2}
        });
        let draft = RetryDraft::from_value(&stored).expect("retry draft");
        assert_eq!(draft.mode, "normal");
        assert_eq!(draft.max_retries, "3");
        assert_eq!(draft.codes, vec!["RATE_LIMIT", "SERVER"]);
        assert_eq!(draft.initial_delay, "250");
        assert_eq!(draft.jitter, "0.2");
        let body = retry_body(&draft, Lang::En).expect("valid retry");
        assert_eq!(body["mode"], "normal");
        assert_eq!(body["backoff"]["initial_delay_ms"], 250);

        let mut invalid = draft.clone();
        invalid.codes.clear();
        assert!(retry_body(&invalid, Lang::En).is_err());
        let mut invalid = draft.clone();
        invalid.initial_delay = "5000".into();
        invalid.max_delay = "1000".into();
        assert!(retry_body(&invalid, Lang::En).is_err());
        let mut invalid = draft.clone();
        invalid.jitter = "2".into();
        assert!(retry_body(&invalid, Lang::En).is_err());

        let always = RetryDraft {
            mode: "always".into(),
            ..draft
        };
        let body = retry_body(&always, Lang::En).expect("always policy");
        assert_eq!(body["mode"], "always");
        assert!(body["retryable_codes"].is_null());
    }

    #[test]
    fn model_rows_default_to_text_and_keep_reasoning() {
        let model = ModelInfo {
            id: "custom".into(),
            reasoning_efforts: Some(json!({"low": "low"})),
            ..ModelInfo::default()
        };
        let draft = model_draft(&model);
        assert!(draft.text);
        assert!(!draft.image);
        assert_eq!(draft.reasoning_efforts, Some(json!({"low": "low"})));
    }
}

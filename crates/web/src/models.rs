//! Models settings section (web UI).
//!
//! Native port of the React `ui-settings-models` section surface: the provider
//! directory (catalog + stored profiles), one editor card per provider, and a
//! model list with capacities and input types. Profiles are persisted by the
//! server in `<home>/models.yaml`; the revision returned with the snapshot is
//! sent back on every write so a stale card cannot overwrite a newer edit.

use gloo_net::http::Request;
use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use wasm_bindgen_futures::spawn_local;

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
struct ActiveSelection {
    #[serde(default)]
    provider: String,
    #[serde(default)]
    model: String,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
struct CatalogModel {
    #[serde(default)]
    id: String,
    #[serde(default)]
    name: String,
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
    account: bool,
    #[serde(default)]
    summary: String,
    #[serde(default)]
    models: Vec<ModelInfo>,
    #[serde(default)]
    models_from_profile: bool,
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
    configured: bool,
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
            configured: row.configured,
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
                    reasoning_efforts: None,
                })
                .collect(),
            configured: false,
            account: entry.account,
            default_base_url: entry.default_base_url.clone(),
            default_api_key_env: entry.api_key_env.clone(),
        }
    }
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

fn draft_body(draft: &ProviderDraft) -> Result<Value, String> {
    let base_url = draft.base_url.trim();
    if !base_url.is_empty()
        && !(base_url.starts_with("https://") || base_url.starts_with("http://"))
    {
        return Err("the base URL must start with http:// or https://".into());
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
        return Err("the API key variable must be UPPER_SNAKE_CASE".into());
    }
    let mut seen = std::collections::BTreeSet::new();
    let mut models = Vec::with_capacity(draft.models.len());
    for model in &draft.models {
        let id = model.id.trim();
        if id.is_empty() {
            return Err("every model needs an id".into());
        }
        if id.chars().any(char::is_whitespace) {
            return Err(format!("model id '{id}' must not contain whitespace"));
        }
        if !seen.insert(id.to_owned()) {
            return Err(format!("model id '{id}' is listed twice"));
        }
        let context_window = parse_capacity(&model.context_window, "context window")?;
        let max_tokens = parse_capacity(&model.max_tokens, "max output tokens")?;
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
    Ok(json!({
        "protocol": draft.protocol,
        "base_url": (!base_url.is_empty()).then_some(base_url),
        "api_key_env": (!reference.is_empty()).then_some(reference),
        "models": models,
    }))
}

fn parse_capacity(value: &str, label: &str) -> Result<Option<u64>, String> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    let parsed: u64 = value
        .parse()
        .map_err(|_| format!("the {label} must be a whole number"))?;
    if parsed == 0 {
        return Err(format!("the {label} must be greater than zero"));
    }
    Ok(Some(parsed))
}

#[derive(Clone, Debug, Serialize)]
struct ActiveRequest {
    provider: String,
    model: String,
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

fn credential_dot(row: &ProviderRow) -> (&'static str, &'static str) {
    match row.credential_configured {
        Some(true) => ("models-dot ok", "key configured"),
        Some(false) => ("models-dot missing", "no key in the environment"),
        None => ("models-dot unknown", "no API key for this route"),
    }
}

#[component]
pub fn ModelsSettings(token: RwSignal<String>) -> impl IntoView {
    let snapshot = RwSignal::new(None::<ModelsSnapshot>);
    let error = RwSignal::new(String::new());
    let status = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    let editing = RwSignal::new(None::<ProviderDraft>);
    let deleting = RwSignal::new(None::<String>);
    let add_choice = RwSignal::new(String::new());

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
                        error.set(format!("cannot read the models snapshot: {problem}"))
                    }
                },
                Err(message) => error.set(message),
            }
            busy.set(false);
        });
    };
    Effect::new(move |_| load());

    let save = move || {
        let Some(draft) = editing.get_untracked() else {
            return;
        };
        let revision = snapshot
            .get_untracked()
            .map(|value| value.revision)
            .unwrap_or_default();
        let mut body = match draft_body(&draft) {
            Ok(body) => body,
            Err(message) => {
                error.set(message);
                return;
            }
        };
        body["expected_revision"] = json!(revision);
        let value = token.get_untracked();
        let route = draft.id.clone();
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
                    status.set(format!("saved {route} · restart dshr serve to apply"));
                    editing.set(None);
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
                    status.set(format!("removed {route} · catalog defaults apply again"));
                    deleting.set(None);
                    error.set(String::new());
                    load();
                }
                Err(message) => error.set(message),
            }
            busy.set(false);
        });
    };

    let activate = move |provider: String, model: String| {
        if model.is_empty() {
            error.set("the provider lists no models yet".into());
            return;
        }
        let value = token.get_untracked();
        let body = ActiveRequest {
            provider: provider.clone(),
            model: model.clone(),
        };
        busy.set(true);
        spawn_local(async move {
            let body = serde_json::to_value(body).unwrap_or(Value::Null);
            match send("POST", "/v1/config/model", &value, Some(&body)).await {
                Ok(_) => {
                    status.set(format!(
                        "active route: {provider} · {model} — restart dshr serve to apply"
                    ));
                    error.set(String::new());
                    load();
                }
                Err(message) => error.set(message),
            }
            busy.set(false);
        });
    };

    let add = move || {
        let choice = add_choice.get_untracked();
        let Some(value) = snapshot.get_untracked() else {
            return;
        };
        let Some(entry) = value
            .catalog
            .iter()
            .find(|entry| entry.id == choice)
            .cloned()
        else {
            return;
        };
        let draft = ProviderDraft::from_catalog(&entry);
        let already_listed = value.providers.iter().any(|row| row.id == entry.id);
        if !already_listed {
            // A draft card needs a row to live in; the next reload replaces
            // this synthetic default row with the stored profile.
            let row = ProviderRow {
                id: entry.id.clone(),
                display_name: entry.display_name.clone(),
                protocol: entry.protocol.clone(),
                base_url: None,
                effective_base_url: entry.default_base_url.clone(),
                api_key_env: entry.api_key_env.clone(),
                account: entry.account,
                summary: entry.summary.clone(),
                models: entry
                    .models
                    .iter()
                    .map(|model| ModelInfo {
                        id: model.id.clone(),
                        name: Some(model.name.clone()),
                        ..ModelInfo::default()
                    })
                    .collect(),
                editable: true,
                ..ProviderRow::default()
            };
            snapshot.update(|value| {
                if let Some(value) = value {
                    value.providers.push(row);
                }
            });
        }
        editing.set(Some(draft));
        error.set(String::new());
    };

    view! {
        <div class="models-section">
            <p class="tool">
                {move || snapshot.get().map(|value| {
                    if value.active.provider.is_empty() {
                        "No active provider is persisted yet; choose one below.".to_owned()
                    } else {
                        format!(
                            "Active route: {} · {} — {}",
                            value.active.provider,
                            value.active.model,
                            value.note,
                        )
                    }
                }).unwrap_or_default()}
            </p>
            {move || (!error.get().is_empty()).then(|| view! {
                <p class="models-error" role="alert">{move || error.get()}</p>
            })}
            <ul class="models-rows">
                {move || {
                    let Some(snapshot_value) = snapshot.get() else {
                        return view! { <li class="tool">"loading…"</li> }.into_any();
                    };
                    let active_snapshot = snapshot_value.active.clone();
                    let catalog_snapshot = snapshot_value.catalog.clone();
                    snapshot_value
                        .providers
                        .into_iter()
                        .map(|row| {
                            let card_id = row.id.clone();
                            let (dot_class, dot_label) = credential_dot(&row);
                            let active = row.active;
                            let editable = row.editable;
                            let configured = row.configured;
                            let name = row.display_name.clone();
                            let route = row.id.clone();
                            let protocol = row.protocol.clone();
                            let models_count = row.models.len();
                            let catalog_entry = catalog_snapshot
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
                                active_snapshot.model.clone()
                            } else {
                                first_model.clone()
                            };
                            let delete_id = row.id.clone();
                            let edit_id = row.id.clone();
                            let editor_key = row.id.clone();
                            let error_text = row.error.clone().unwrap_or_default();
                            let route_for_header = route.clone();
                            view! {
                                <li class="models-row" data-provider=card_id.clone()>
                                    <div class="models-row-head">
                                        <span class=dot_class title=dot_label></span>
                                        <span class="models-name">{name.clone()}</span>
                                        <span class="models-tag">{route_for_header.clone()}</span>
                                        {active.then(|| view! { <span class="models-tag active">"active"</span> })}
                                        {(!configured).then(|| view! { <span class="models-tag">"defaults"</span> })}
                                        <span class="models-tag">{protocol.clone()}</span>
                                        {row.models_from_profile.then(|| view! { <span class="models-tag">"custom list"</span> })}
                                        <span class="models-actions">
                                            {editable.then(|| view! {
                                                <button
                                                    on:click=move |_| {
                                                        editing.set(Some(draft_for_edit.clone()));
                                                        error.set(String::new());
                                                    }
                                                    disabled=move || busy.get()
                                                >"Edit"</button>
                                                <button
                                                    on:click=move |_| activate(use_route.clone(), use_model.clone())
                                                    disabled=move || busy.get()
                                                >"Use"</button>
                                            })}
                                            {row.removable.then(|| view! {
                                                <button
                                                    on:click=move |_| { deleting.set(Some(delete_id.clone())); }
                                                    disabled=move || busy.get()
                                                >"Delete"</button>
                                            })}
                                        </span>
                                    </div>
                                    {(!error_text.is_empty()).then(|| view! {
                                        <p class="models-error">{error_text.clone()}</p>
                                    })}
                                    <p class="tool">
                                        {row.summary.clone()}
                                        {row.effective_base_url.as_ref().map(|url| format!(" · {url}"))}
                                        {format!(" · {models_count} model(s)")}
                                    </p>
                                    {move || {
                                        let route = route.clone();
                                        let name = name.clone();
                                        (deleting.get().as_deref() == Some(route.as_str())).then(|| view! {
                                            <div class="models-confirm">
                                                <span>{format!("Delete the stored profile for {name} and its model list? Catalog defaults return.")}</span>
                                                <button
                                                    on:click=move |_| delete(route.clone())
                                                    disabled=move || busy.get()
                                                >"Delete"</button>
                                                <button on:click=move |_| deleting.set(None)>"Cancel"</button>
                                            </div>
                                        })
                                    }}
                                    {move || {
                                        let open = editing.get().is_some_and(|draft| draft.id == editor_key);
                                        if !open {
                                            return view! { <span></span> }.into_any();
                                        }
                                        let Some(draft) = editing.get() else {
                                            return view! { <span></span> }.into_any();
                                        };
                                        let protocol_hint = draft.protocol.clone();
                                        let default_base = draft.default_base_url.clone().unwrap_or_default();
                                        let default_key = draft.default_api_key_env.clone().unwrap_or_default();
                                        view! {
                                            <div class="models-editor" data-edit=edit_id.clone()>
                                                <div class="models-field">
                                                    <label>"Base URL"</label>
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
                                                    <label>"API key variable"</label>
                                                    <input
                                                        placeholder=default_key
                                                        prop:value=move || editing.get().map(|draft| draft.api_key_env).unwrap_or_default()
                                                        on:input=move |event| {
                                                            let value = event_target_value(&event);
                                                            editing.update(|draft| { if let Some(draft) = draft { draft.api_key_env = value; } });
                                                        }
                                                    />
                                                    <span class="tool">{format!("protocol: {protocol_hint}")}</span>
                                                </div>
                                                <div class="models-models-head">
                                                    <strong>"Models"</strong>
                                                    <span class="tool">"Save an empty list to inherit the catalog defaults."</span>
                                                </div>
                                                <div class="models-model-list">
                                                    {move || {
                                                        let Some(draft) = editing.get() else {
                                                            return view! { <span></span> }.into_any();
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
                                                            let can_use = !model_id_for_use.trim().is_empty();
                                                            view! {
                                                                <div class="models-model-row">
                                                                    <input
                                                                        class="model-id"
                                                                        placeholder="model id"
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
                                                                        placeholder="display name"
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
                                                                        placeholder="context"
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
                                                                        placeholder="max out"
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
                                                                        title="make this the active model"
                                                                        disabled=move || busy.get() || !can_use
                                                                        on:click=move |_| {
                                                                            activate(provider_for_use.clone(), model_id_for_use.clone());
                                                                        }
                                                                    >"Use"</button>
                                                                    <button
                                                                        title="remove this row"
                                                                        on:click=move |_| {
                                                                            editing.update(|draft| {
                                                                                if let Some(draft) = draft {
                                                                                    let _ = draft.models.remove(index);
                                                                                }
                                                                            });
                                                                        }
                                                                    >"×"</button>
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
                                                    }>"Add model"</button>
                                                    <button on:click=move |_| {
                                                        editing.update(|draft| { if let Some(draft) = draft { draft.models.clear(); } });
                                                    }>"Restore catalog defaults"</button>
                                                </div>
                                                <div class="row">
                                                    <button
                                                        class="primary"
                                                        on:click=move |_| save()
                                                        disabled=move || busy.get()
                                                    >"Save"</button>
                                                    <button on:click=move |_| { editing.set(None); load(); }>"Cancel"</button>
                                                </div>
                                            </div>
                                        }.into_any()
                                    }}
                                </li>
                            }
                        })
                        .collect_view()
                        .into_any()
                }}
            </ul>
            <div class="row models-add">
                <label>"Add provider"</label>
                <select
                    prop:value=move || add_choice.get()
                    on:change=move |event| add_choice.set(event_target_value(&event))
                >
                    <option value="">"choose a catalog provider…"</option>
                    {move || snapshot.get().map(|value| {
                        value.catalog.into_iter().map(|entry| {
                            let id = entry.id.clone();
                            let label = format!("{} · {}", entry.display_name, entry.id);
                            view! { <option value=id>{label}</option> }
                        }).collect_view()
                    }).unwrap_or_default()}
                </select>
                <button on:click=move |_| add() disabled=move || busy.get()>"Add"</button>
                <span class="tool">{move || if busy.get() { "working…" } else { "" }}</span>
            </div>
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
            configured: false,
            account: false,
            default_base_url: Some("https://api.deepseek.com/anthropic".into()),
            default_api_key_env: Some("DEEPSEEK_API_KEY".into()),
        }
    }

    #[test]
    fn draft_body_rejects_bad_endpoints_keys_and_models() {
        let mut value = draft();
        value.base_url = "ftp://example".into();
        assert!(draft_body(&value).is_err());

        let mut value = draft();
        value.api_key_env = "lower-case".into();
        assert!(draft_body(&value).is_err());

        let mut value = draft();
        value.models[0].context_window = "1.5".into();
        assert!(draft_body(&value).is_err());

        let mut value = draft();
        value.models.push(value.models[0].clone());
        assert!(draft_body(&value).is_err());
    }

    #[test]
    fn draft_body_preserves_reasoning_levels_and_input_types() {
        let body = draft_body(&draft()).expect("valid draft");
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
                },
                CatalogModel {
                    id: "gpt-4o".into(),
                    name: "GPT-4o".into(),
                },
            ],
            ..CatalogEntry::default()
        };
        let draft = ProviderDraft::from_catalog(&entry);
        assert_eq!(draft.models.len(), 2);
        assert_eq!(draft.models[1].id, "gpt-4o");
        assert_eq!(draft.api_key_env, "OPENAI_API_KEY");
        assert!(draft.models.iter().all(|model| model.text));
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

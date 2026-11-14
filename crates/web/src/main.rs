use std::cell::RefCell;

use gloo_net::http::Request;
use std::collections::BTreeMap;
use leptos::prelude::*;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::Value;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
use wasm_bindgen_futures::spawn_local;
use web_sys::{EventSource, MessageEvent};

#[derive(Clone, Debug, Deserialize, PartialEq)]
struct SessionSummary {
    id: String,
    created_at: String,
    model: String,
    turns: usize,
}

#[derive(Clone, Debug, Deserialize)]
struct SessionsResponse {
    sessions: Vec<SessionSummary>,
}

#[derive(Clone, Debug, Deserialize)]
struct SessionResponse {
    messages: Vec<Message>,
}

#[derive(Clone, Debug, Deserialize)]
struct Message {
    role: String,
    content: Vec<Value>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
struct ToolInfo {
    name: String,
    description: String,
}

#[derive(Clone, Debug, Deserialize)]
struct HealthResponse {
    version: String,
}

#[derive(Clone, Debug, Deserialize)]
struct ToolsResponse {
    tools: Vec<ToolInfo>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
struct PackageHit {
    name: String,
    #[serde(default)]
    version: String,
    description: String,
    #[serde(default)]
    category: String,
}

#[derive(Clone, Debug, Deserialize)]
struct PackagesResponse {
    packages: Vec<PackageHit>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
struct InstalledPlugin {
    name: String,
    enabled: bool,
}

#[derive(Clone, Debug, Deserialize)]
struct ReviewSummary {
    average: f64,
    count: usize,
}

#[derive(Clone, Debug, Deserialize)]
struct ReviewsResponse {
    plugins: BTreeMap<String, ReviewSummary>,
}

#[derive(Clone, Debug, Deserialize)]
struct RatingsResponse {
    ratings: BTreeMap<String, u8>,
}

#[derive(Clone, Debug, Deserialize)]
struct InstalledResponse {
    plugins: Vec<InstalledPlugin>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
struct ClientHalfInfo {
    name: String,
    #[serde(default)]
    package: Option<String>,
    entry: String,
    platform: String,
    #[serde(default)]
    immediately: bool,
    #[serde(default)]
    bundle: String,
    #[serde(default)]
    inject: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
struct ClientsResponse {
    #[serde(default)]
    clients: Vec<ClientHalfInfo>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
struct LivePlugin {
    name: String,
    #[serde(default)]
    enabled: bool,
    #[serde(default)]
    running: bool,
    #[serde(default)]
    generation: u64,
    #[serde(default)]
    tools: Vec<String>,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
struct PluginStatusResponse {
    #[serde(default)]
    plugins: Vec<LivePlugin>,
}

#[derive(Clone, Debug, Deserialize)]
struct NewSessionResponse {
    session: SessionHeader,
}

#[derive(Clone, Debug, Deserialize)]
struct SessionHeader {
    id: String,
}

#[derive(Clone, Debug, PartialEq)]
struct ChatItem {
    role: String,
    text: String,
}

#[derive(Clone, Debug, PartialEq)]
struct ApprovalInfo {
    id: String,
    tool: String,
    summary: String,
    reason: String,
}

#[derive(Clone, Debug, PartialEq)]
struct QuestionInfo {
    id: String,
    question: String,
    options: Vec<String>,
}

thread_local! {
    static SOURCE: RefCell<Option<(EventSource, Closure<dyn FnMut(MessageEvent)>)>> =
        const { RefCell::new(None) };
    /// Window listener that mediates between the shell and the plugin iframe.
    /// Kept alive for the whole page, because the frame is recreated per panel.
    static PANEL_LISTENER: RefCell<Option<Closure<dyn FnMut(MessageEvent)>>> =
        const { RefCell::new(None) };
}

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(App);
}

/// `?plugin=<name>` deep link: open that plugin's panel once the client list
/// arrives (and connect automatically, since the token came with the link).
fn initial_panel() -> Option<String> {
    web_sys::window()
        .and_then(|window| window.location().search().ok())
        .and_then(|search| web_sys::UrlSearchParams::new_with_str(&search).ok())
        .and_then(|params| params.get("plugin"))
        .filter(|name| !name.trim().is_empty())
}

fn initial_token() -> String {
    if let Some(window) = web_sys::window()
        && let Ok(search) = window.location().search()
        && let Ok(params) = web_sys::UrlSearchParams::new_with_str(&search)
        && let Some(token) = params.get("token")
    {
        let token = token.trim().to_owned();
        store_token(&token);
        return token;
    }
    stored_token()
}

fn stored_token() -> String {
    web_sys::window()
        .and_then(|window| window.local_storage().ok().flatten())
        .and_then(|storage| storage.get_item("dshr-token").ok().flatten())
        .unwrap_or_default()
}

fn store_token(token: &str) {
    if let Some(storage) =
        web_sys::window().and_then(|window| window.local_storage().ok().flatten())
    {
        let _ = storage.set_item("dshr-token", token);
    }
}

async fn get_json<T: DeserializeOwned>(path: &str, token: &str) -> Result<T, String> {
    let response = Request::get(path)
        .header("Authorization", &format!("Bearer {token}"))
        .send()
        .await
        .map_err(|error| error.to_string())?;
    if !response.ok() {
        return Err(format!(
            "HTTP {}: {}",
            response.status(),
            response.status_text()
        ));
    }
    response
        .json::<T>()
        .await
        .map_err(|error| error.to_string())
}

async fn post_json<T: DeserializeOwned>(
    path: &str,
    token: &str,
    body: &Value,
) -> Result<T, String> {
    let response = Request::post(path)
        .header("Authorization", &format!("Bearer {token}"))
        .header("Content-Type", "application/json")
        .json(body)
        .map_err(|error| error.to_string())?
        .send()
        .await
        .map_err(|error| error.to_string())?;
    if !response.ok() {
        return Err(format!(
            "HTTP {}: {}",
            response.status(),
            response.status_text()
        ));
    }
    response
        .json::<T>()
        .await
        .map_err(|error| error.to_string())
}

/// One slot registered by a plugin half.
#[derive(Clone, Debug, PartialEq)]
struct ClientSlot {
    name: String,
    id: String,
    title: String,
    order: i64,
}

/// DOM id of a plugin's iframe (`@scope/pkg` is not a valid id fragment).
fn frame_id(plugin: &str) -> String {
    let mut id = String::from("plugin-frame-");
    for ch in plugin.chars() {
        if ch.is_ascii_alphanumeric() {
            id.push(ch);
        } else {
            id.push('-');
        }
    }
    id
}

fn frame_for(plugin: &str) -> Option<web_sys::HtmlIFrameElement> {
    web_sys::window()?
        .document()?
        .get_element_by_id(&frame_id(plugin))?
        .dyn_into::<web_sys::HtmlIFrameElement>()
        .ok()
}

/// Sends a shell→plugin message to the iframe of `plugin`.
fn post_to_frame(plugin: &str, kind: &str, fields: &[(&str, &str)]) {
    let Some(frame) = frame_for(plugin) else {
        return;
    };
    let Some(target) = frame.content_window() else {
        return;
    };
    let message = js_sys::Object::new();
    let _ = js_sys::Reflect::set(&message, &"v".into(), &1u32.into());
    let _ = js_sys::Reflect::set(&message, &"ns".into(), &"dshr-shell".into());
    let _ = js_sys::Reflect::set(&message, &"type".into(), &kind.into());
    for (key, value) in fields {
        let _ = js_sys::Reflect::set(&message, &(*key).into(), &(*value).into());
    }
    let _ = target.post_message(&message.into(), "*");
}

/// Builds a short diff preview for tools that change file contents.
///
/// The server sends the tool input with `tool_start`, so the shell can show
/// `-old`/`+new` lines without re-reading the session journal.
fn tool_diff_preview(name: &str, input: &serde_json::Value) -> String {
    fn lines(prefix: char, text: &str, limit: usize) -> String {
        let mut out = String::new();
        for line in text.lines().take(limit) {
            out.push(prefix);
            out.push_str(line);
            out.push('\n');
        }
        out
    }
    match name {
        "edit" => {
            let old = input["old_string"].as_str().unwrap_or_default();
            let new = input["new_string"].as_str().unwrap_or_default();
            if old.is_empty() && new.is_empty() {
                return String::new();
            }
            format!("\n{} {}", lines('-', old, 20), lines('+', new, 20))
        }
        "write" => input["content"]
            .as_str()
            .map(|content| format!("\n{}", lines('+', content, 20)))
            .unwrap_or_default(),
        "apply_patch" => input["patch"]
            .as_str()
            .map(|patch| {
                let body: Vec<&str> = patch.lines().take(40).collect();
                format!("\n{}", body.join("\n"))
            })
            .unwrap_or_default(),
        _ => String::new(),
    }
}

fn activate_slot(plugin: &str, slot: &ClientSlot) {
    post_to_frame(
        plugin,
        "activate",
        &[("slot", &slot.name), ("name", &slot.name), ("id", &slot.id)],
    );
}

fn deactivate_panel(plugin: &str) {
    post_to_frame(plugin, "deactivate", &[]);
}

fn push_theme(plugin: &str, theme: &str) {
    post_to_frame(plugin, "theme", &[("theme", theme)]);
}

/// Replies to `ctx.action(action, payload)` from a plugin frame.
fn post_action_result(plugin: &str, id: &str, ok: bool, error: &str) {
    let Some(frame) = frame_for(plugin) else {
        return;
    };
    let Some(target) = frame.content_window() else {
        return;
    };
    let message = js_sys::Object::new();
    let _ = js_sys::Reflect::set(&message, &"v".into(), &1u32.into());
    let _ = js_sys::Reflect::set(&message, &"ns".into(), &"dshr-shell".into());
    let _ = js_sys::Reflect::set(&message, &"type".into(), &"action-result".into());
    let _ = js_sys::Reflect::set(&message, &"id".into(), &id.into());
    let _ = js_sys::Reflect::set(&message, &"ok".into(), &ok.into());
    if !error.is_empty() {
        let _ = js_sys::Reflect::set(&message, &"error".into(), &error.into());
    }
    let _ = target.post_message(&message.into(), "*");
}

/// Applies the height a plugin asked for, within sane bounds for the panel.
fn apply_frame_height(plugin: &str, height: f64) {
    let Some(frame) = frame_for(plugin) else {
        return;
    };
    let height = height.clamp(120.0, 640.0);
    let element: &web_sys::HtmlElement = frame.as_ref();
    let _ = element.style().set_property("height", &format!("{height}px"));
}

fn field_of(value: &wasm_bindgen::JsValue, key: &str) -> String {
    js_sys::Reflect::get(value, &key.into())
        .ok()
        .and_then(|value| value.as_string())
        .unwrap_or_default()
}

fn number_of(value: &wasm_bindgen::JsValue, key: &str) -> f64 {
    js_sys::Reflect::get(value, &key.into())
        .ok()
        .and_then(|value| value.as_f64())
        .unwrap_or(0.0)
}

/// Slots announced by a `slots` bridge message, ordered like the bridge does.
fn read_slots(data: &wasm_bindgen::JsValue) -> Vec<ClientSlot> {
    let Ok(slots) = js_sys::Reflect::get(data, &"slots".into()) else {
        return Vec::new();
    };
    let Ok(slots) = slots.dyn_into::<js_sys::Array>() else {
        return Vec::new();
    };
    let mut out: Vec<ClientSlot> = slots
        .iter()
        .filter_map(|value| {
            let id = field_of(&value, "id");
            if id.is_empty() {
                return None;
            }
            let name = field_of(&value, "name");
            let name = if name.is_empty() {
                field_of(&value, "slot")
            } else {
                name
            };
            let title = field_of(&value, "title");
            Some(ClientSlot {
                name: if name.is_empty() { "panel".into() } else { name },
                id: id.clone(),
                title: if title.is_empty() { id } else { title },
                order: number_of(&value, "order") as i64,
            })
        })
        .collect();
    out.sort_by(|left, right| left.order.cmp(&right.order).then(left.id.cmp(&right.id)));
    out
}

/// Activation order for a half: its `inject` dependencies first, cycle-safe.
fn activation_order(name: &str, clients: &[ClientHalfInfo]) -> Vec<String> {
    fn visit(name: &str, clients: &[ClientHalfInfo], order: &mut Vec<String>) {
        if order.iter().any(|seen| seen == name) {
            return;
        }
        if let Some(half) = clients.iter().find(|half| half.name == name) {
            for dependency in &half.inject {
                visit(dependency, clients, order);
            }
        }
        order.push(name.to_owned());
    }
    let mut order = Vec::new();
    visit(name, clients, &mut order);
    order
}

/// Refresh the live plugin views after a control action (reload/toggle/install).
fn refresh_plugin_views(
    token: String,
    live: RwSignal<Vec<LivePlugin>>,
    clients: RwSignal<Vec<ClientHalfInfo>>,
) {
    spawn_local(async move {
        if let Ok(list) = get_json::<PluginStatusResponse>("/v1/plugins/status", &token).await {
            live.set(list.plugins);
        }
        if let Ok(list) = get_json::<ClientsResponse>("/v1/plugins/clients", &token).await {
            clients.set(list.clients);
        }
    });
}

fn text_of(content: &[Value]) -> String {
    content
        .iter()
        .filter_map(|block| block.get("text").and_then(Value::as_str))
        .collect::<Vec<_>>()
        .join("")
}

fn push_text(messages: RwSignal<Vec<ChatItem>>, streaming: RwSignal<bool>, text: &str) {
    messages.update(|items| {
        if streaming.get_untracked()
            && let Some(last) = items.last_mut()
            && last.role == "assistant"
        {
            last.text.push_str(text);
            return;
        }
        items.push(ChatItem {
            role: "assistant".into(),
            text: text.to_owned(),
        });
    });
    streaming.set(true);
}

fn connect_events(
    session: String,
    token: String,
    messages: RwSignal<Vec<ChatItem>>,
    streaming: RwSignal<bool>,
    busy: RwSignal<bool>,
    status: RwSignal<String>,
    approval: RwSignal<Option<ApprovalInfo>>,
    question: RwSignal<Option<QuestionInfo>>,
) {
    let url = format!(
        "/v1/sessions/{session}/events?token={}",
        js_sys::encode_uri_component(&token)
    );
    let Ok(source) = EventSource::new(&url) else {
        status.set("events: error".into());
        return;
    };
    let on_message = Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
        let Some(data) = event.data().as_string() else {
            return;
        };
        let Ok(value) = serde_json::from_str::<Value>(&data) else {
            return;
        };
        match value["type"].as_str().unwrap_or_default() {
            "text" => {
                if let Some(text) = value["text"].as_str() {
                    push_text(messages, streaming, text);
                }
            }
            "tool_start" => {
                let name = value["name"].as_str().unwrap_or("tool");
                let summary = value["summary"].as_str().unwrap_or("");
                // `input` travels as a JSON string (capped server-side), so it
                // has to be parsed before the diff fields can be read.
                let parsed: serde_json::Value = value["input"]
                    .as_str()
                    .and_then(|raw| serde_json::from_str(raw).ok())
                    .unwrap_or_default();
                let preview = tool_diff_preview(name, &parsed);
                messages.update(|items| {
                    items.push(ChatItem {
                        role: "tool".into(),
                        text: if summary.is_empty() {
                            format!("→ {name}{preview}")
                        } else {
                            format!("→ {name} · {summary}{preview}")
                        },
                    });
                });
            }
            "tool_end" => {
                let name = value["name"].as_str().unwrap_or("tool");
                let ok = value["ok"].as_bool().unwrap_or(true);
                let content = value["content"].as_str().unwrap_or("").trim();
                let mut text = format!("{} {name}", if ok { "✓" } else { "✗" });
                if !content.is_empty() {
                    text.push_str(&format!("\n{content}"));
                }
                messages.update(|items| {
                    items.push(ChatItem {
                        role: "tool".into(),
                        text,
                    });
                });
            }
            "question" => {
                question.set(Some(QuestionInfo {
                    id: value["id"].as_str().unwrap_or_default().to_owned(),
                    question: value["question"].as_str().unwrap_or_default().to_owned(),
                    options: value["options"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(|option| option.as_str().map(ToOwned::to_owned))
                        .collect(),
                }));
            }
            "approval" => {
                approval.set(Some(ApprovalInfo {
                    id: value["id"].as_str().unwrap_or_default().to_owned(),
                    tool: value["tool"].as_str().unwrap_or_default().to_owned(),
                    summary: value["summary"].as_str().unwrap_or_default().to_owned(),
                    reason: value["reason"].as_str().unwrap_or_default().to_owned(),
                }));
            }
            "done" => {
                streaming.set(false);
                busy.set(false);
                status.set("done".into());
                if let Some(text) = value["text"].as_str()
                    && !text.is_empty()
                {
                    messages.update(|items| {
                        if let Some(last) = items.last_mut()
                            && last.role == "assistant"
                        {
                            last.text = text.to_owned();
                        } else {
                            items.push(ChatItem {
                                role: "assistant".into(),
                                text: text.to_owned(),
                            });
                        }
                    });
                }
            }
            "cancelled" => {
                streaming.set(false);
                busy.set(false);
                status.set("cancelled".into());
            }
            "error" => {
                streaming.set(false);
                busy.set(false);
                let message = value["message"].as_str().unwrap_or("error");
                status.set(format!("error: {message}"));
            }
            _ => {}
        }
    });
    source.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
    SOURCE.with(|slot| {
        if let Some((previous, _)) = slot.borrow_mut().take() {
            previous.close();
        }
        *slot.borrow_mut() = Some((source, on_message));
    });
}

#[component]
fn App() -> impl IntoView {
    let token = RwSignal::new(initial_token());
    let status = RwSignal::new(String::from("offline"));
    let sessions = RwSignal::new(Vec::<SessionSummary>::new());
    let messages = RwSignal::new(Vec::<ChatItem>::new());
    let current = RwSignal::new(None::<String>);
    let prompt = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    let streaming = RwSignal::new(false);
    let approval = RwSignal::new(None::<ApprovalInfo>);
    let question = RwSignal::new(None::<QuestionInfo>);
    let answer = RwSignal::new(String::new());
    let tools = RwSignal::new(Vec::<ToolInfo>::new());
    let plugins_open = RwSignal::new(false);
    let plugin_query = RwSignal::new(String::new());
    let plugin_hits = RwSignal::new(Vec::<PackageHit>::new());
    let installed_plugins = RwSignal::new(Vec::<InstalledPlugin>::new());
    let featured = RwSignal::new(Vec::<PackageHit>::new());
    let ratings = RwSignal::new(BTreeMap::<String, u8>::new());
    let reviews = RwSignal::new(BTreeMap::<String, ReviewSummary>::new());
    let plugin_status = RwSignal::new(String::new());
    let clients = RwSignal::new(Vec::<ClientHalfInfo>::new());
    let live_plugins = RwSignal::new(Vec::<LivePlugin>::new());
    let initial_panel = initial_panel();
    let panel_plugin = RwSignal::new(None::<ClientHalfInfo>);
    let pending_panel = RwSignal::new(initial_panel.clone());
    // Plugin frames kept alive: eager (`immediately`) halves and already
    // opened panels. A mounted frame is hidden, not unmounted, when the panel
    // closes, so an eager half keeps running.
    let mounted_frames = RwSignal::new(Vec::<String>::new());
    let slots_by_plugin = RwSignal::new(BTreeMap::<String, Vec<ClientSlot>>::new());
    let active_slot = RwSignal::new(None::<ClientSlot>);
    // Bumped after a plugin reload so the mounted frame fetches the new bundle.
    let frame_versions = RwSignal::new(BTreeMap::<String, u32>::new());
    // Frames that just (re)loaded and need their slot activated again.
    let pending_activation = RwSignal::new(Vec::<String>::new());
    let panel_error = RwSignal::new(String::new());

    let bump_frame = move |name: &str| {
        let name = name.to_owned();
        frame_versions.update(|versions| {
            let version = versions.entry(name).or_insert(0);
            *version += 1;
        });
    };

    let mount_frame = move |name: &str| {
        let name = name.to_owned();
        mounted_frames.update(|frames| {
            if !frames.contains(&name) {
                frames.push(name);
            }
        });
    };

    let open_panel = move |half: ClientHalfInfo| {
        for name in activation_order(&half.name, &clients.get_untracked()) {
            mount_frame(&name);
        }
        panel_error.set(String::new());
        let known = slots_by_plugin
            .get_untracked()
            .get(&half.name)
            .and_then(|slots| slots.first().cloned());
        active_slot.set(known.clone());
        if let Some(slot) = known {
            activate_slot(&half.name, &slot);
        }
        panel_plugin.set(Some(half));
    };

    let close_panel = move || {
        if let Some(half) = panel_plugin.get_untracked() {
            deactivate_panel(&half.name);
            if !half.immediately {
                mounted_frames.update(|frames| frames.retain(|name| name != &half.name));
            }
        }
        panel_plugin.set(None);
        active_slot.set(None);
        panel_error.set(String::new());
    };
    let version = RwSignal::new(String::new());
    let custom_frame = web_sys::window()
        .and_then(|window| window.location().search().ok())
        .is_some_and(|search| search.contains("frame=custom"))
        && web_sys::window()
            .and_then(|window| js_sys::Reflect::get(&window, &"__TAURI__".into()).ok())
            .is_some_and(|value| !value.is_undefined());
    let initial_theme = web_sys::window()
        .and_then(|window| window.local_storage().ok().flatten())
        .and_then(|storage| storage.get_item("dshr-theme").ok().flatten())
        .or_else(|| {
            web_sys::window()
                .and_then(|window| window.location().search().ok())
                .filter(|search| search.contains("theme=light"))
                .map(|_| "light".to_owned())
        })
        .unwrap_or_else(|| "dark".to_owned());
    let theme = RwSignal::new(initial_theme);
    let apply_theme = move |value: String| {
        if let Some(root) = web_sys::window()
            .and_then(|window| window.document())
            .and_then(|document| document.document_element())
        {
            let _ = root.set_attribute("data-theme", &value);
        }
        if let Some(storage) = web_sys::window()
            .and_then(|window| window.local_storage().ok().flatten())
        {
            let _ = storage.set_item("dshr-theme", &value);
        }
    };
    apply_theme(theme.get_untracked());
    {
        let token_value = token.get_untracked();
        spawn_local(async move {
            if let Ok(health) = get_json::<HealthResponse>("/v1/health", &token_value).await {
                version.set(health.version);
            }
        });
    }

    let refresh_sessions = move || {
        let value = token.get_untracked();
        if value.is_empty() {
            status.set("token required".into());
            return;
        }
        store_token(&value);
        let status = status;
        spawn_local(async move {
            match get_json::<SessionsResponse>("/v1/sessions", &value).await {
                Ok(data) => {
                    sessions.set(data.sessions);
                    if let Ok(health) = get_json::<HealthResponse>("/v1/health", &value).await {
                        version.set(health.version);
                    }
                    if let Ok(installed) =
                        get_json::<InstalledResponse>("/v1/plugins/installed", &value).await
                    {
                        installed_plugins.set(installed.plugins);
                    }
                    if let Ok(catalog) =
                        get_json::<PackagesResponse>("/v1/plugins/catalog", &value).await
                    {
                        featured.set(catalog.packages);
                    }
                    if let Ok(data) =
                        get_json::<RatingsResponse>("/v1/plugins/ratings", &value).await
                    {
                        ratings.set(data.ratings);
                    }
                    if let Ok(data) =
                        get_json::<ReviewsResponse>("/v1/plugins/reviews", &value).await
                    {
                        reviews.set(data.plugins);
                    }
                    if let Ok(list) =
                        get_json::<ClientsResponse>("/v1/plugins/clients", &value).await
                    {
                        // `immediately` halves mount at startup, in activation
                        // order: a half that injects another is mounted after it.
                        let mut pending: Vec<ClientHalfInfo> = list
                            .clients
                            .iter()
                            .filter(|half| half.immediately)
                            .cloned()
                            .collect();
                        let mut ordered: Vec<String> = Vec::new();
                        while !pending.is_empty() {
                            let before = pending.len();
                            pending.retain(|half| {
                                let ready = half.inject.iter().all(|dependency| {
                                    ordered.contains(dependency)
                                        || !list
                                            .clients
                                            .iter()
                                            .any(|candidate| &candidate.name == dependency)
                                });
                                if ready {
                                    mount_frame(&half.name);
                                    ordered.push(half.name.clone());
                                }
                                !ready
                            });
                            if pending.len() == before {
                                break;
                            }
                        }
                        for half in pending {
                            mount_frame(&half.name);
                        }
                        clients.set(list.clients.clone());
                        if let Some(name) = pending_panel.get_untracked()
                            && let Some(half) =
                                list.clients.iter().find(|half| half.name == name).cloned()
                        {
                            pending_panel.set(None);
                            open_panel(half);
                        }
                    }
                    if let Ok(list) =
                        get_json::<PluginStatusResponse>("/v1/plugins/status", &value).await
                    {
                        live_plugins.set(list.plugins);
                    }
                    match get_json::<ToolsResponse>("/v1/tools", &value).await {
                        Ok(list) => tools.set(list.tools),
                        Err(error) => plugin_status.set(format!("tools: {error}")),
                    }
                    status.set("connected".into());
                }
                Err(error) => status.set(format!("error: {error}")),
            }
        });
    };

    // A `?plugin=` deep link carries the token too (the CLI/desktop opens it),
    // so connect without requiring a click.
    if initial_panel.is_some() && !token.get_untracked().trim().is_empty() {
        Effect::new(move |_| refresh_sessions());
    }

    let open_session = move |id: String| {
        let value = token.get_untracked();
        current.set(Some(id.clone()));
        messages.set(Vec::new());
        streaming.set(false);
        approval.set(None);
        let id_for_load = id.clone();
        let value_for_load = value.clone();
        let status = status;
        spawn_local(async move {
            match get_json::<SessionResponse>(
                &format!("/v1/sessions/{id_for_load}"),
                &value_for_load,
            )
            .await
            {
                Ok(data) => {
                    messages.set(
                        data.messages
                            .into_iter()
                            .filter_map(|message| {
                                let text = text_of(&message.content);
                                (!text.is_empty()).then_some(ChatItem {
                                    role: message.role,
                                    text,
                                })
                            })
                            .collect(),
                    );
                    status.set(id_for_load.clone());
                }
                Err(error) => status.set(format!("error: {error}")),
            }
        });
        connect_events(id, value, messages, streaming, busy, status, approval, question);
    };

    let new_session = move || {
        let value = token.get_untracked();
        if value.is_empty() {
            status.set("token required".into());
            return;
        }
        let status = status;
        spawn_local(async move {
            match post_json::<NewSessionResponse>("/v1/sessions", &value, &serde_json::json!({}))
                .await
            {
                Ok(data) => {
                    let id = data.session.id.clone();
                    status.set(format!("session {id}"));
                    // Select the new session immediately: `send` refuses to run
                    // without an active session, and the sidebar entry only
                    // appears after a refresh.
                    messages.set(Vec::new());
                    streaming.set(false);
                    approval.set(None);
                    current.set(Some(id.clone()));
                    connect_events(
                        id,
                        value.clone(),
                        messages,
                        streaming,
                        busy,
                        status,
                        approval,
                        question,
                    );
                    refresh_sessions();
                }
                Err(error) => status.set(format!("error: {error}")),
            }
        });
    };

    let send = move || {
        let Some(session) = current.get_untracked() else {
            status.set("no session".into());
            return;
        };
        let value = prompt.get_untracked();
        if value.trim().is_empty() {
            return;
        }
        let token_value = token.get_untracked();
        prompt.set(String::new());
        messages.update(|items| {
            items.push(ChatItem {
                role: "user".into(),
                text: value.clone(),
            });
        });
        busy.set(true);
        streaming.set(false);
        let status = status;
        spawn_local(async move {
            let result = post_json::<Value>(
                &format!("/v1/sessions/{session}/turns"),
                &token_value,
                &serde_json::json!({"prompt": value}),
            )
            .await;
            if let Err(error) = result {
                status.set(format!("error: {error}"));
                busy.set(false);
            }
        });
    };

    // One window listener for the whole page: every mounted plugin iframe
    // announces its slots, errors and actions here.
    {
        let on_message = Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
            let data = event.data();
            if !data.is_object() || field_of(&data, "ns") != "dshr-client" {
                return;
            }
            let plugin = field_of(&data, "plugin");
            match field_of(&data, "type").as_str() {
                "ready" => {
                    // A fresh or reloaded frame needs an explicit activate.
                    pending_activation.update(|plugins| {
                        if !plugins.contains(&plugin) {
                            plugins.push(plugin.clone());
                        }
                    });
                }
                "slots" => {
                    let slots = read_slots(&data);
                    slots_by_plugin.update(|map| {
                        map.insert(plugin.clone(), slots.clone());
                    });
                    let is_active = panel_plugin
                        .get_untracked()
                        .map(|half| half.name == plugin)
                        .unwrap_or(false);
                    if !is_active {
                        return;
                    }
                    let needs_activation = {
                        let pending = pending_activation.get_untracked();
                        let needs = pending.contains(&plugin);
                        if needs {
                            pending_activation.update(|plugins| {
                                plugins.retain(|candidate| candidate != &plugin);
                            });
                        }
                        needs
                    };
                    let known = active_slot
                        .get_untracked()
                        .and_then(|slot| slots.iter().find(|candidate| candidate.id == slot.id).cloned());
                    let next = known.or_else(|| slots.first().cloned());
                    if let Some(next) = next {
                        let changed = active_slot.get_untracked().as_ref().map(|slot| slot.id.clone())
                            != Some(next.id.clone());
                        if changed || needs_activation {
                            active_slot.set(Some(next.clone()));
                            activate_slot(&plugin, &next);
                        }
                    }
                }
                "resize" => apply_frame_height(&plugin, number_of(&data, "height")),
                "error" => {
                    let is_active = panel_plugin
                        .get_untracked()
                        .map(|half| half.name == plugin)
                        .unwrap_or(false);
                    if is_active {
                        let message = field_of(&data, "message");
                        panel_error.set(if message.is_empty() {
                            "plugin error".into()
                        } else {
                            message
                        });
                    }
                }
                "action" => {
                    let action = field_of(&data, "action");
                    let id = field_of(&data, "id");
                    let payload = js_sys::Reflect::get(&data, &"payload".into())
                        .unwrap_or(wasm_bindgen::JsValue::NULL);
                    match action.as_str() {
                        "close" => {
                            post_action_result(&plugin, &id, true, "");
                            close_panel();
                        }
                        "compose" => {
                            let text = field_of(&payload, "text");
                            prompt.update(|value| {
                                if !value.is_empty() && !value.ends_with('\n') {
                                    value.push('\n');
                                }
                                value.push_str(&text);
                            });
                            post_action_result(&plugin, &id, true, "");
                        }
                        "notify" => {
                            let message = field_of(&payload, "message");
                            status.set(if message.is_empty() {
                                "plugin notification".into()
                            } else {
                                message
                            });
                            post_action_result(&plugin, &id, true, "");
                        }
                        "reload" => {
                            let token_value = token.get_untracked();
                            let name = plugin.clone();
                            let reply = plugin.clone();
                            let reloaded = plugin.clone();
                            spawn_local(async move {
                                let body = serde_json::json!({"name": name});
                                let result =
                                    post_json::<Value>("/v1/plugins/reload", &token_value, &body)
                                        .await;
                                match &result {
                                    Ok(_) => {
                                        post_action_result(&reply, &id, true, "");
                                        bump_frame(&reloaded);
                                    }
                                    Err(error) => post_action_result(&reply, &id, false, error),
                                }
                                refresh_plugin_views(token_value, live_plugins, clients);
                            });
                        }
                        other => {
                            post_action_result(
                                &plugin,
                                &id,
                                false,
                                &format!("unsupported action '{other}'"),
                            );
                        }
                    }
                }
                _ => {}
            }
        });
        if let Some(window) = web_sys::window() {
            let _ = window.add_event_listener_with_callback(
                "message",
                on_message.as_ref().unchecked_ref(),
            );
        }
        PANEL_LISTENER.with(|slot| *slot.borrow_mut() = Some(on_message));
    }

    let reload_plugin = move |name: String| {
        let token_value = token.get_untracked();
        plugin_status.set(format!("reloading {name}…"));
        let reloaded = name.clone();
        spawn_local(async move {
            let body = serde_json::json!({"name": name});
            match post_json::<Value>("/v1/plugins/reload", &token_value, &body).await {
                Ok(value) => {
                    let failures = value["errors"].as_array().map_or(0, Vec::len);
                    plugin_status.set(if failures == 0 {
                        "reloaded".into()
                    } else {
                        format!("reload failed: {value}")
                    });
                    if failures == 0 {
                        bump_frame(&reloaded);
                    }
                }
                Err(error) => plugin_status.set(format!("reload error: {error}")),
            }
            refresh_plugin_views(token_value, live_plugins, clients);
        });
    };

    let install_package = move |name: String| {
        let token_value = token.get_untracked();
        plugin_status.set(format!("installing {name}…"));
        spawn_local(async move {
            let body = serde_json::json!({"name": name});
            let request = gloo_net::http::Request::post("/v1/plugins/install")
                .header("Authorization", &format!("Bearer {token_value}"))
                .header("Content-Type", "application/json")
                .body(body.to_string());
            let result = match request {
                Ok(request) => request.send().await,
                Err(error) => {
                    plugin_status.set(format!("install error: {error}"));
                    return;
                }
            };
            match result {
                Ok(response) if response.ok() => {
                    plugin_status.set(format!("installed {name} — it will appear once the config watcher reloads"));
                    refresh_plugin_views(token_value, live_plugins, clients);
                }
                Ok(response) => plugin_status.set(format!("install failed: HTTP {}", response.status())),
                Err(error) => plugin_status.set(format!("install error: {error}")),
            }
        });
    };

    let show_info = move |name: String| {
        let token_value = token.get_untracked();
        plugin_status.set(format!("loading {name}…"));
        spawn_local(async move {
            let encoded = js_sys::encode_uri_component(&name);
            let url = format!("/v1/plugins/info?name={encoded}");
            match get_json::<serde_json::Value>(&url, &token_value).await {
                Ok(info) => plugin_status.set(format!(
                    "{}@{} — {} {}",
                    info["name"].as_str().unwrap_or(&name),
                    info["version"].as_str().unwrap_or(""),
                    info["description"].as_str().unwrap_or(""),
                    info["homepage"].as_str().unwrap_or("")
                )),
                Err(error) => plugin_status.set(format!("info error: {error}")),
            }
        });
    };

    let write_review = move |name: String| {
        let rating = js_sys::eval("window.prompt('Rating 1-5', '5')")
            .ok()
            .and_then(|value| value.as_string())
            .and_then(|value| value.trim().parse::<u8>().ok());
        let Some(rating) = rating.filter(|rating| (1..=5).contains(rating)) else {
            return;
        };
        let text = js_sys::eval("window.prompt('Review (optional)', '')")
            .ok()
            .and_then(|value| value.as_string())
            .unwrap_or_default();
        let token_value = token.get_untracked();
        spawn_local(async move {
            let body = serde_json::json!({"name": name, "rating": rating, "text": text});
            if let Ok(request) = gloo_net::http::Request::post("/v1/plugins/review")
                .header("Authorization", &format!("Bearer {token_value}"))
                .header("Content-Type", "application/json")
                .body(body.to_string())
            {
                let _ = request.send().await;
                plugin_status.set("review saved".into());
            }
        });
    };

    let rate_plugin = move |name: String, rating: u8| {
        let token_value = token.get_untracked();
        plugin_status.set(format!("rating {name}: {rating}/5"));
        spawn_local(async move {
            let body = serde_json::json!({"name": name, "rating": rating});
            if let Ok(request) = gloo_net::http::Request::post("/v1/plugins/rate")
                .header("Authorization", &format!("Bearer {token_value}"))
                .header("Content-Type", "application/json")
                .body(body.to_string())
            {
                let _ = request.send().await;
                ratings.update(|map| {
                    map.insert(name, rating);
                });
            }
        });
    };

    let plugin_action = move |path: &'static str, body: serde_json::Value| {
        let token_value = token.get_untracked();
        plugin_status.set("working…".into());
        spawn_local(async move {
            let request = gloo_net::http::Request::post(path)
                .header("Authorization", &format!("Bearer {token_value}"))
                .header("Content-Type", "application/json")
                .body(body.to_string());
            match request {
                Ok(request) => match request.send().await {
                    Ok(response) if response.ok() => {
                        plugin_status.set("done".into());
                        refresh_plugin_views(token_value.clone(), live_plugins, clients);
                    }
                    Ok(response) => {
                        plugin_status.set(format!("failed: HTTP {}", response.status()));
                    }
                    Err(error) => plugin_status.set(format!("failed: {error}")),
                },
                Err(error) => plugin_status.set(format!("failed: {error}")),
            }
        });
    };

    let search_plugins = move || {
        let query = plugin_query.get_untracked();
        if query.trim().is_empty() {
            return;
        }
        let token_value = token.get_untracked();
        plugin_status.set("searching…".into());
        spawn_local(async move {
            let url = format!(
                "/v1/plugins/search?q={}",
                js_sys::encode_uri_component(&query)
            );
            match get_json::<PackagesResponse>(&url, &token_value).await {
                Ok(data) => {
                    let count = data.packages.len();
                    plugin_hits.set(data.packages);
                    plugin_status.set(format!("{count} package(s)"));
                }
                Err(error) => plugin_status.set(format!("error: {error}")),
            }
        });
    };

    let cancel = move || {
        let Some(session) = current.get_untracked() else {
            return;
        };
        let value = token.get_untracked();
        spawn_local(async move {
            let _ = post_json::<Value>(
                &format!("/v1/sessions/{session}/cancel"),
                &value,
                &serde_json::json!({}),
            )
            .await;
        });
    };

    let answer_question = move |value_for_answer: String| {
        let Some(info) = question.get_untracked() else {
            return;
        };
        let Some(session) = current.get_untracked() else {
            return;
        };
        let token_value = token.get_untracked();
        question.set(None);
        answer.set(String::new());
        spawn_local(async move {
            let _ = post_json::<Value>(
                &format!("/v1/sessions/{session}/questions/{}", info.id),
                &token_value,
                &serde_json::json!({"answer": value_for_answer}),
            )
            .await;
        });
    };

    let resolve = move |approved: bool| {
        let Some(info) = approval.get_untracked() else {
            return;
        };
        let Some(session) = current.get_untracked() else {
            return;
        };
        let value = token.get_untracked();
        approval.set(None);
        spawn_local(async move {
            let _ = post_json::<Value>(
                &format!("/v1/sessions/{session}/approvals/{}", info.id),
                &value,
                &serde_json::json!({"approved": approved}),
            )
            .await;
        });
    };

    view! {
        {move || custom_frame.then(|| view! {
            <div class="titlebar" data-tauri-drag-region="true">
                <strong>"dshr"</strong>
                <div class="spacer" data-tauri-drag-region="true"></div>
                <button on:click=move |_| { let _ = js_sys::eval("window.__TAURI__.window.getCurrentWindow().minimize()"); }>"–"</button>
                <button on:click=move |_| { let _ = js_sys::eval("window.__TAURI__.window.getCurrentWindow().toggleMaximize()"); }>"□"</button>
                <button on:click=move |_| { let _ = js_sys::eval("window.__TAURI__.window.getCurrentWindow().close()"); }>"×"</button>
            </div>
        })}
        <header>
            <strong>"dshr"</strong>
            <input
                placeholder="API token"
                autocomplete="off"
                prop:value=move || token.get()
                on:input=move |event| token.set(event_target_value(&event))
            />
            <button on:click=move |_| refresh_sessions()>"Connect"</button>
            <button on:click=move |_| new_session()>"New session"</button>
            <button on:click=move |_| plugins_open.set(true)>"Plugins"</button>
            <button on:click=move |_| {
                let next = if theme.get_untracked() == "dark" { "light" } else { "dark" };
                theme.set(next.to_owned());
                apply_theme(next.to_owned());
                // Live panels follow the shell theme without a reload.
                for name in mounted_frames.get_untracked() {
                    push_theme(&name, next);
                }
            }>"Theme"</button>
            <span id="version">{move || version.get()}</span>
            <span id="status">{move || status.get()}</span>
        </header>
        <main>
            <aside id="sessions">
                <For
                    each=move || sessions.get()
                    key=|session| session.id.clone()
                    children=move |session| {
                        let id = session.id.clone();
                        view! {
                            <button
                                class="session"
                                on:click=move |_| open_session(id.clone())
                            >
                                {format!(
                                    "{} · {} turn(s) · {}",
                                    &session.id[..session.id.len().min(12)],
                                    session.turns,
                                    session.model,
                                )}
                            </button>
                        }
                    }
                />
            </aside>
            <section id="chat">
                <div id="messages">
                    <For
                        each=move || messages.get()
                        key=|item| (item.role.clone(), item.text.clone())
                        children=|item| {
                            view! { <div class=item.role>{item.text}</div> }
                        }
                    />
                </div>
                <form on:submit=move |event| {
                    event.prevent_default();
                    send();
                }>
                    <textarea
                        rows="3"
                        placeholder="Ask dshr…"
                        prop:value=move || prompt.get()
                        on:input=move |event| prompt.set(event_target_value(&event))
                    ></textarea>
                    <div class="row">
                        <button type="submit" prop:disabled=move || busy.get()>"Send"</button>
                        <button type="button" on:click=move |_| cancel()>"Cancel"</button>
                    </div>
                </form>
            </section>
        </main>
        {move || plugins_open.get().then(|| view! {
            <dialog open>
                <h3>"Plugins"</h3>
                <div class="row">
                    <input
                        placeholder="Search npm (cordis)…"
                        prop:value=move || plugin_query.get()
                        on:input=move |event| plugin_query.set(event_target_value(&event))
                    />
                    <button on:click=move |_| search_plugins()>"Search"</button>
                    <button on:click=move |_| plugins_open.set(false)>"Close"</button>
                </div>
                <p>{move || plugin_status.get()}</p>
                <h4>"Featured"</h4>
                <ul>
                    {move || featured.get().into_iter().map(|package| {
                        let install_name = package.name.clone();
                        let info_name = package.name.clone();
                        let review_name = package.name.clone();
                        view! {
                            <li>
                                {format!(
                                    "{} — {}{} · ★{:.1} ({})",
                                    package.name,
                                    if package.category.is_empty() {
                                        String::new()
                                    } else {
                                        format!("[{}] ", package.category)
                                    },
                                    package.description,
                                    reviews
                                        .get()
                                        .get(&package.name)
                                        .map(|summary| summary.average)
                                        .unwrap_or(0.0),
                                    reviews
                                        .get()
                                        .get(&package.name)
                                        .map(|summary| summary.count)
                                        .unwrap_or(0),
                                )}
                                <button on:click=move |_| install_package(install_name.clone())>"Install"</button>
                                <button on:click=move |_| show_info(info_name.clone())>"Info"</button>
                                <button on:click=move |_| write_review(review_name.clone())>"Review"</button>
                                {(1..=5).map(|rating| {
                                    let rate_name = package.name.clone();
                                    view! {
                                        <button on:click=move |_| rate_plugin(rate_name.clone(), rating)>{rating.to_string()}</button>
                                    }
                                }).collect_view()}
                            </li>
                        }
                    }).collect_view()}
                </ul>
                <h4>"Cordis plugins"</h4>
                <ul>
                    {move || installed_plugins.get().into_iter().map(|plugin| {
                        let name = plugin.name.clone();
                        let toggle_name = plugin.name.clone();
                        let remove_name = plugin.name.clone();
                        let enabled = plugin.enabled;
                        view! {
                            <li>
                                {format!("{} ({})", plugin.name, if enabled { "enabled" } else { "disabled" })}
                                <button on:click=move |_| plugin_action(
                                    "/v1/plugins/toggle",
                                    serde_json::json!({"name": toggle_name.clone(), "enabled": !enabled}),
                                )>{if enabled { "Disable" } else { "Enable" }}</button>
                                <button on:click=move |_| plugin_action(
                                    "/v1/plugins/remove",
                                    serde_json::json!({"name": remove_name.clone()}),
                                )>"Remove"</button>
                                {name}
                            </li>
                        }
                    }).collect_view()}
                </ul>
                <h4>"Plugin panels"</h4>
                <ul>
                    {move || clients.get().into_iter().map(|client| {
                        let open = client.clone();
                        let reload_name = client.name.clone();
                        let state = live_plugins
                            .get()
                            .into_iter()
                            .find(|plugin| plugin.name == client.name)
                            .map(|plugin| {
                                if plugin.running {
                                    format!(
                                        "running · {} tool(s) · gen {}",
                                        plugin.tools.len(),
                                        plugin.generation,
                                    )
                                } else {
                                    plugin
                                        .error
                                        .clone()
                                        .unwrap_or_else(|| "stopped".into())
                                }
                            })
                            .unwrap_or_else(|| "not attached".into());
                        let package = client.package.clone().unwrap_or_default();
                        view! {
                            <li>
                                {format!(
                                    "{} ({} → {}, {}) · {}",
                                    client.name,
                                    package,
                                    client.entry,
                                    client.bundle,
                                    state,
                                )}
                                <button on:click=move |_| open_panel(open.clone())>"Open"</button>
                                <button on:click=move |_| reload_plugin(reload_name.clone())>"Reload"</button>
                            </li>
                        }
                    }).collect_view()}
                </ul>
                <h4>"Installed tools"</h4>
                <ul>
                    {move || tools.get().into_iter().map(|tool| view! {
                        <li>{format!("{} — {}", tool.name, tool.description)}</li>
                    }).collect_view()}
                </ul>
                <h4>"npm packages"</h4>
                <ul>
                    {move || plugin_hits.get().into_iter().map(|package| {
                        let install_name = package.name.clone();
                        let info_name = package.name.clone();
                        view! {
                            <li>
                                {format!(
                                    "{}@{} — {} · dshr plugin npm add {}",
                                    package.name,
                                    package.version,
                                    package.description,
                                    package.name,
                                )}
                                <button on:click=move |_| install_package(install_name.clone())>"Install"</button>
                                <button on:click=move |_| show_info(info_name.clone())>"Info"</button>
                            </li>
                        }
                    }).collect_view()}
                </ul>
            </dialog>
        })}
        {
            let stage_open = move || panel_plugin.get().is_some();
            view! {
                <div
                    id="plugin-stage"
                    class=move || if stage_open() { "plugin-stage open" } else { "plugin-stage" }
                >
                    <div class="plugin-stage-panel">
                        {move || panel_plugin.get().map(|half| {
                            let tabs_plugin = half.name.clone();
                            let title_name = half.name.clone();
                            view! {
                                <div class="plugin-stage-head">
                                    <h3>
                                        {move || {
                                            let slot = active_slot
                                                .get()
                                                .map(|slot| format!(" — {}", slot.title))
                                                .unwrap_or_default();
                                            format!("{title_name}{slot}")
                                        }}
                                    </h3>
                                    <div class="row plugin-tabs">
                                        {move || slots_by_plugin
                                            .get()
                                            .get(&tabs_plugin)
                                            .cloned()
                                            .unwrap_or_default()
                                            .into_iter()
                                            .map(|slot| {
                                                let plugin = tabs_plugin.clone();
                                                let clicked = slot.clone();
                                                let is_active = {
                                                    let id = slot.id.clone();
                                                    move || active_slot.get().map(|slot| slot.id == id).unwrap_or(false)
                                                };
                                                view! {
                                                    <button
                                                        class:active=is_active
                                                        on:click=move |_| {
                                                            active_slot.set(Some(clicked.clone()));
                                                            activate_slot(&plugin, &clicked);
                                                        }
                                                    >
                                                        {format!("{} · {}", slot.name, slot.title)}
                                                    </button>
                                                }
                                            })
                                            .collect_view()}
                                        <button on:click=move |_| close_panel()>"Close"</button>
                                    </div>
                                    <p class="tool">{move || panel_error.get()}</p>
                                </div>
                            }
                        })}
                        <div class="plugin-stage-body">
                            {move || mounted_frames.get().into_iter().filter_map(|name| {
                                let half = clients.get().into_iter().find(|half| half.name == name)?;
                                let encoded_name = js_sys::encode_uri_component(&half.name).as_string().unwrap_or_default();
                                let token_value = token.get_untracked();
                                let encoded_token = js_sys::encode_uri_component(&token_value).as_string().unwrap_or_default();
                                let encoded_theme = js_sys::encode_uri_component(&theme.get_untracked()).as_string().unwrap_or_default();
                                let src_base = format!(
                                    "/v1/plugins/{encoded_name}/host?token={encoded_token}&theme={encoded_theme}"
                                );
                                let frame_id = frame_id(&half.name);
                                let active_name = half.name.clone();
                                let version_name = half.name.clone();
                                let label = format!("plugin panel: {}", half.name);
                                let src = move || {
                                    let version = frame_versions
                                        .get()
                                        .get(&version_name)
                                        .copied()
                                        .unwrap_or(0);
                                    format!("{src_base}&v={version}")
                                };
                                Some(view! {
                                    <iframe
                                        id=frame_id
                                        title=label
                                        class=move || {
                                            let active = panel_plugin
                                                .get()
                                                .map(|current| current.name == active_name)
                                                .unwrap_or(false);
                                            if active { "plugin-frame" } else { "plugin-frame hidden" }
                                        }
                                        sandbox="allow-scripts"
                                        src=src
                                    ></iframe>
                                })
                            }).collect_view()}
                        </div>
                    </div>
                </div>
            }
        }
        {move || question.get().map(|info| view! {
            <dialog open>
                <h3>"Question"</h3>
                <p>{info.question.clone()}</p>
                <div class="row">
                    {info.options.iter().map(|option| {
                        let option = option.clone();
                        view! {
                            <button on:click=move |_| answer_question(option.clone())>
                                {option.clone()}
                            </button>
                        }
                    }).collect_view()}
                </div>
                <div class="row">
                    <input
                        placeholder="Answer"
                        prop:value=move || answer.get()
                        on:input=move |event| answer.set(event_target_value(&event))
                    />
                    <button on:click=move |_| answer_question(answer.get_untracked())>
                        "Send"
                    </button>
                </div>
            </dialog>
        })}
        {move || approval.get().map(|info| view! {
            <dialog open>
                <h3>"Approval required"</h3>
                <p>{format!("{}: {}", info.tool, info.summary)}</p>
                <p>{info.reason}</p>
                <div class="row">
                    <button on:click=move |_| resolve(true)>"Allow"</button>
                    <button on:click=move |_| resolve(false)>"Deny"</button>
                </div>
            </dialog>
        })}
    }
}

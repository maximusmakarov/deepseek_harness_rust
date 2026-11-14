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
struct ModelConfigResponse {
    #[serde(default)]
    provider: String,
    #[serde(default)]
    active_provider: String,
    model: String,
    #[serde(default)]
    base_url: Option<String>,
    #[serde(default)]
    routes: Vec<String>,
    #[serde(default)]
    note: String,
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

/// One slot registered by a plugin's client half.
///
/// A registration is rendered by its own iframe placed in the shell region the
/// slot name maps to (`region` comes from the bridge's DSH slot vocabulary
/// table). That is what lets a slot appear in the sidebar, a settings dialog or
/// under a message instead of as a tab inside one plugin panel.
#[derive(Clone, Debug, PartialEq)]
struct ClientSlot {
    plugin: String,
    name: String,
    id: String,
    title: String,
    order: i64,
    region: String,
}

/// Shell state shared by slot-frame components through the Leptos context.
#[derive(Clone, Copy)]
struct UiState {
    slots: RwSignal<Vec<ClientSlot>>,
    token: RwSignal<String>,
    theme: RwSignal<String>,
    versions: RwSignal<BTreeMap<String, u32>>,
}

fn encode(value: &str) -> String {
    js_sys::encode_uri_component(value)
        .as_string()
        .unwrap_or_default()
}

/// FNV-1a over a registration identity: frame tokens stay ASCII and short while
/// plugin names (`@scope/pkg`) and arbitrary slot ids never leak into the DOM id
/// or the `frame` query parameter.
fn token_hash(input: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in input.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100_0000_01b3);
    }
    format!("{hash:016x}")
}

fn sanitize_id(text: &str) -> String {
    text.chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                ch
            } else {
                '-'
            }
        })
        .collect()
}

/// DOM id of a frame; the token itself is opaque and assigned by the shell.
fn frame_dom_id(token: &str) -> String {
    format!("frame-{}", sanitize_id(token))
}

fn registry_token(plugin: &str) -> String {
    format!("registry:{}", token_hash(plugin))
}

/// Token of one rendered slot occurrence:
/// `slot:<region>:<registration hash>:<occurrence>`.
fn slot_token(slot: &ClientSlot, occurrence: &str) -> String {
    let occurrence = if occurrence.is_empty() {
        "0".to_owned()
    } else {
        sanitize_id(occurrence)
    };
    format!(
        "slot:{}:{}:{occurrence}",
        slot.region,
        token_hash(&format!("{}:{}", slot.plugin, slot.id)),
    )
}

fn frame_for_token(token: &str) -> Option<web_sys::HtmlIFrameElement> {
    web_sys::window()?
        .document()?
        .get_element_by_id(&frame_dom_id(token))?
        .dyn_into::<web_sys::HtmlIFrameElement>()
        .ok()
}

fn post_message(target: &web_sys::Window, kind: &str, fields: &[(&str, &str)]) {
    let message = js_sys::Object::new();
    let _ = js_sys::Reflect::set(&message, &"v".into(), &1u32.into());
    let _ = js_sys::Reflect::set(&message, &"ns".into(), &"dshr-shell".into());
    let _ = js_sys::Reflect::set(&message, &"type".into(), &kind.into());
    for (key, value) in fields {
        let _ = js_sys::Reflect::set(&message, &(*key).into(), &(*value).into());
    }
    let _ = target.post_message(&message.into(), "*");
}

/// Posts to the frame registered under `token` (theme change, diagnostics).
fn post_to_token(token: &str, kind: &str, fields: &[(&str, &str)]) {
    let Some(frame) = frame_for_token(token) else {
        return;
    };
    let Some(target) = frame.content_window() else {
        return;
    };
    post_message(&target, kind, fields);
}

/// Replies to `ctx.action(action, payload)`. The reply has to go back to the
/// frame that asked: after regional mounting several frames of one plugin are
/// alive at the same time, so addressing by plugin name would be ambiguous.
fn post_action_result(source: Option<js_sys::Object>, id: &str, ok: bool, error: &str) {
    let Some(source) = source else {
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
    let Ok(post) = js_sys::Reflect::get(&source, &"postMessage".into()) else {
        return;
    };
    let Some(post) = post.dyn_ref::<js_sys::Function>() else {
        return;
    };
    let _ = post.call2(&source, &message.into(), &"*".into());
}

/// Pushes the live theme into every frame — registry and slot frames alike.
fn push_theme_all(theme: &str) {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return;
    };
    let Ok(frames) = document.query_selector_all("iframe") else {
        return;
    };
    for index in 0..frames.length() {
        let Some(node) = frames.item(index) else {
            continue;
        };
        let Ok(frame) = node.dyn_into::<web_sys::HtmlIFrameElement>() else {
            continue;
        };
        let Some(target) = frame.content_window() else {
            continue;
        };
        post_message(&target, "theme", &[("theme", theme)]);
    }
}

/// Pushes the host state (theme, sidebar width, current session, session list)
/// into every frame. `ctx.get('sessions')` and the `useTheme`/`useSession`/
/// `useSessions` hook props read it, so a React slot component re-renders when
/// the shell switches sessions or themes.
fn push_host_state_all(theme: &str, session: Option<&str>, sessions: &[(String, String)]) {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return;
    };
    let Ok(frames) = document.query_selector_all("iframe") else {
        return;
    };
    let state = serde_json::json!({
        "theme": theme,
        "wide": true,
        "session": session.map(|id| serde_json::json!({ "id": id, "title": id })),
        "sessions": sessions
            .iter()
            .map(|(id, title)| serde_json::json!({ "id": id, "title": title }))
            .collect::<Vec<_>>(),
    });
    let state = js_sys::JSON::parse(&state.to_string()).unwrap_or(wasm_bindgen::JsValue::NULL);
    for index in 0..frames.length() {
        let Some(node) = frames.item(index) else {
            continue;
        };
        let Ok(frame) = node.dyn_into::<web_sys::HtmlIFrameElement>() else {
            continue;
        };
        let Some(target) = frame.content_window() else {
            continue;
        };
        let message = js_sys::Object::new();
        let _ = js_sys::Reflect::set(&message, &"v".into(), &1u32.into());
        let _ = js_sys::Reflect::set(&message, &"ns".into(), &"dshr-shell".into());
        let _ = js_sys::Reflect::set(&message, &"type".into(), &"host-state".into());
        let _ = js_sys::Reflect::set(&message, &"state".into(), &state);
        let _ = target.post_message(&message.into(), "*");
    }
}

/// Applies the height a slot frame asked for, clamped to a range that works in
/// every region (a footer button and a full panel share the mechanism).
fn apply_frame_height(token: &str, height: f64) {
    let Some(frame) = frame_for_token(token) else {
        return;
    };
    let height = height.clamp(24.0, 640.0);
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

/// Slots announced by a `slots` bridge message, ordered like the bridge does.
fn read_slots(data: &wasm_bindgen::JsValue, plugin: &str) -> Vec<ClientSlot> {
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
            let region = field_of(&value, "region");
            Some(ClientSlot {
                plugin: plugin.to_owned(),
                name: if name.is_empty() { "panel".into() } else { name },
                id: id.clone(),
                title: if title.is_empty() { id } else { title },
                order: number_of(&value, "order") as i64,
                region: if region.is_empty() {
                    "panel".into()
                } else {
                    region
                },
            })
        })
        .collect();
    out.sort_by(|left, right| left.order.cmp(&right.order).then(left.id.cmp(&right.id)));
    out
}

/// Mount order of the eager registry frames: `inject` dependencies first, then
/// `immediately` halves, then the rest by name; cycles are visited once.
fn registry_order(clients: &[ClientHalfInfo]) -> Vec<String> {
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
    let mut roots: Vec<&ClientHalfInfo> = clients.iter().collect();
    roots.sort_by(|left, right| {
        right
            .immediately
            .cmp(&left.immediately)
            .then(left.name.cmp(&right.name))
    });
    let mut order = Vec::new();
    for half in roots {
        visit(&half.name, clients, &mut order);
    }
    order
}

/// Renders one slot registration in its own sandboxed iframe.
///
/// The frame loads the same bundle in `mode=slot`, so the bridge renders exactly
/// this registration and its `ctx.effect`s run in an isolated document. Nothing
/// about the shell layout reaches the plugin.
#[component]
fn SlotFrame(
    entry: ClientSlot,
    token: String,
    #[prop(default = String::new())] context: String,
) -> impl IntoView {
    let ui = expect_context::<UiState>();
    let frame_id = frame_dom_id(&token);
    let title = format!("plugin slot: {}::{}", entry.plugin, entry.name);
    let version_plugin = entry.plugin.clone();
    // The initial theme travels in the URL; later changes are pushed as bridge
    // messages so toggling the theme never reloads a frame.
    let theme = if ui.theme.get_untracked() == "light" {
        "light"
    } else {
        "dark"
    };
    let base = format!(
        "/v1/plugins/{plugin}/host?token={token}&theme={theme}&frame={frame}&mode=slot&slot={name}&id={id}&context={context}",
        plugin = encode(&entry.plugin),
        token = encode(&ui.token.get_untracked()),
        theme = encode(theme),
        frame = encode(&token),
        name = encode(&entry.name),
        id = encode(&entry.id),
        context = encode(&context),
    );
    let src = move || {
        let version = ui
            .versions
            .get()
            .get(&version_plugin)
            .copied()
            .unwrap_or(0);
        format!("{base}&v={version}")
    };
    view! {
        <iframe
            id=frame_id
            class="slot-frame"
            title=title
            sandbox="allow-scripts"
            data-frame-token=token.clone()
            data-region=entry.region.clone()
            data-plugin=entry.plugin.clone()
            data-slot=entry.name.clone()
            data-slot-id=entry.id.clone()
            src=src
        ></iframe>
    }
}

/// Mount point of one shell region: renders one slot frame per registration the
/// bridge routed here. `occurrence` distinguishes repeated instances of the
/// same region (the second message, the fourth session row, ...).
#[component]
fn SlotRegion(
    region: &'static str,
    #[prop(optional)] occurrence: Option<String>,
    #[prop(optional)] context: Option<String>,
) -> impl IntoView {
    let ui = expect_context::<UiState>();
    let occurrence = occurrence.unwrap_or_default();
    let context = context.unwrap_or_default();
    let each_occurrence = occurrence.clone();
    let each = move || {
        let occurrence = each_occurrence.clone();
        ui.slots
            .get()
            .into_iter()
            .filter(|slot| slot.region == region)
            .map(|slot| {
                let token = slot_token(&slot, &occurrence);
                (slot, token)
            })
            .collect::<Vec<_>>()
    };
    view! {
        <div class="slot-region" data-region=region data-occurrence=occurrence>
            <For
                each=each
                key=|(slot, token)| (token.clone(), slot.plugin.clone(), slot.id.clone())
                children=move |(slot, token)| {
                    view! { <SlotFrame entry=slot token=token context=context.clone() /> }
                }
            />
        </div>
    }
}

/// Hidden eager frames: every client half is mounted at shell start so the shell
/// knows all slots (and their regions) before a panel is opened. This is the
/// "eager mount" half of ADR-013 §5's deferred phase.
#[component]
fn RegistryFrames(
    registry_frames: RwSignal<Vec<String>>,
    clients: RwSignal<Vec<ClientHalfInfo>>,
) -> impl IntoView {
    let ui = expect_context::<UiState>();
    view! {
        <div id="plugin-registry" hidden>
            <For
                each=move || {
                    let order = registry_frames.get();
                    let list = clients.get();
                    order
                        .into_iter()
                        .filter_map(|name| {
                            let half = list.iter().find(|half| half.name == name)?.clone();
                            let token = registry_token(&half.name);
                            Some((half, token))
                        })
                        .collect::<Vec<_>>()
                }
                key=|(half, _)| half.name.clone()
                children=move |(half, token)| {
                    let plugin = half.name.clone();
                    let version_plugin = plugin.clone();
                    let title = format!("plugin registry: {plugin}");
                    let theme = if ui.theme.get_untracked() == "light" {
                        "light"
                    } else {
                        "dark"
                    };
                    let base = format!(
                        "/v1/plugins/{plugin}/host?token={token}&theme={theme}&frame={frame}&mode=registry",
                        plugin = encode(&plugin),
                        token = encode(&ui.token.get_untracked()),
                        theme = encode(theme),
                        frame = encode(&token),
                    );
                    let src = move || {
                        let version = ui
                            .versions
                            .get()
                            .get(&version_plugin)
                            .copied()
                            .unwrap_or(0);
                        format!("{base}&v={version}")
                    };
                    view! {
                        <iframe
                            class="registry-frame"
                            title=title
                            sandbox="allow-scripts"
                            data-frame-token=token.clone()
                            data-plugin=plugin.clone()
                            src=src
                            hidden
                        ></iframe>
                    }
                }
            />
        </div>
    }
}

/// The plugin panel stage. Only the `panel` region keeps tab semantics: every
/// other region renders all its registrations side by side in place.
#[component]
fn PanelRegion(
    panel: RwSignal<Option<ClientHalfInfo>>,
    active_slot: RwSignal<Option<String>>,
    slots: RwSignal<Vec<ClientSlot>>,
    error: RwSignal<String>,
) -> impl IntoView {
    let panel_slots = Memo::new(move |_| {
        let Some(half) = panel.get() else {
            return Vec::new();
        };
        slots
            .get()
            .into_iter()
            .filter(|slot| slot.plugin == half.name && slot.region == "panel")
            .collect::<Vec<_>>()
    });
    // Keep the selected tab valid as registrations arrive or a plugin reloads.
    // Compare before writing: signals notify even when the value is unchanged,
    // and the stage is open before its first registration arrives (the panel
    // deep link runs ahead of the registry frame), which would otherwise be an
    // endless None -> None effect loop.
    Effect::new(move |_| {
        if panel.get().is_none() {
            return;
        }
        let ids = panel_slots.get();
        let valid = active_slot
            .get()
            .is_some_and(|id| ids.iter().any(|slot| slot.id == id));
        if !valid {
            let first = ids.first().map(|slot| slot.id.clone());
            if active_slot.get_untracked() != first {
                active_slot.set(first);
            }
        }
    });
    view! {
        <div
            id="plugin-stage"
            class=move || {
                if panel.get().is_some() {
                    "plugin-stage open"
                } else {
                    "plugin-stage"
                }
            }
        >
            <div class="plugin-stage-panel">
                {move || {
                    panel
                        .get()
                        .map(|half| {
                            let title = half.name.clone();
                            view! {
                                <div class="plugin-stage-head">
                                    <h3>{title}</h3>
                                    <div class="row plugin-tabs">
                                        <For
                                            each=move || panel_slots.get()
                                            key=|slot| slot.id.clone()
                                            children=move |slot| {
                                                let id_for_class = slot.id.clone();
                                                let id_for_click = slot.id.clone();
                                                let label = format!("{} · {}", slot.name, slot.title);
                                                view! {
                                                    <button
                                                        data-slot-id=slot.id.clone()
                                                        class=move || {
                                                            let active = active_slot
                                                                .get()
                                                                .is_some_and(|id| id == id_for_class);
                                                            if active { "active" } else { "" }
                                                        }
                                                        on:click=move |_| active_slot.set(Some(id_for_click.clone()))
                                                    >
                                                        {label}
                                                    </button>
                                                }
                                            }
                                        />
                                        <button on:click=move |_| {
                                            panel.set(None);
                                            active_slot.set(None);
                                        }>"Close"</button>
                                    </div>
                                    <p class="tool">{move || error.get()}</p>
                                </div>
                                <div class="plugin-stage-body slot-region" data-region="panel">
                                    {move || {
                                        let ids = panel_slots.get();
                                        let active = active_slot
                                            .get()
                                            .or_else(|| ids.first().map(|slot| slot.id.clone()));
                                        active
                                            .and_then(|id| ids.into_iter().find(|slot| slot.id == id))
                                            .map(|slot| {
                                                view! {
                                                    <SlotFrame
                                                        entry=slot.clone()
                                                        token=slot_token(&slot, "")
                                                        context="{}".to_owned()
                                                    />
                                                }
                                            })
                                    }}
                                </div>
                            }
                        })
                }}
            </div>
        </div>
    }
}

/// Applies a fresh client-half list: registry mount order, the list the eager
/// frames render from, and pruning of slots whose plugin disappeared.
fn apply_clients(
    clients: RwSignal<Vec<ClientHalfInfo>>,
    registry_frames: RwSignal<Vec<String>>,
    slots: RwSignal<Vec<ClientSlot>>,
    list: Vec<ClientHalfInfo>,
) {
    let order = registry_order(&list);
    slots.update(|all| all.retain(|slot| order.iter().any(|name| name == &slot.plugin)));
    registry_frames.set(order);
    clients.set(list);
}

/// Refresh the live plugin views after a control action (reload/toggle/install).
fn refresh_plugin_views(
    token: String,
    live: RwSignal<Vec<LivePlugin>>,
    clients: RwSignal<Vec<ClientHalfInfo>>,
    registry_frames: RwSignal<Vec<String>>,
    slots: RwSignal<Vec<ClientSlot>>,
) {
    spawn_local(async move {
        if let Ok(list) = get_json::<PluginStatusResponse>("/v1/plugins/status", &token).await {
            live.set(list.plugins);
        }
        if let Ok(list) = get_json::<ClientsResponse>("/v1/plugins/clients", &token).await {
            apply_clients(clients, registry_frames, slots, list.clients);
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
    // Every client half is mounted at startup as a hidden registry frame, in
    // `inject` order: that is how the shell knows every slot (and its region)
    // before a panel is opened. Slot frames themselves are mounted per region.
    let registry_frames = RwSignal::new(Vec::<String>::new());
    let slots = RwSignal::new(Vec::<ClientSlot>::new());
    // Selected `panel`-region slot of the open plugin stage.
    let active_slot = RwSignal::new(None::<String>);
    // Bumped after a plugin reload so every frame of that plugin fetches the
    // new bundle.
    let frame_versions = RwSignal::new(BTreeMap::<String, u32>::new());
    let panel_error = RwSignal::new(String::new());

    let bump_frame = move |name: &str| {
        let name = name.to_owned();
        frame_versions.update(|versions| {
            let version = versions.entry(name).or_insert(0);
            *version += 1;
        });
    };

    let open_panel = move |half: ClientHalfInfo| {
        panel_error.set(String::new());
        // The stage picks the first `panel` registration once it arrives.
        active_slot.set(None);
        panel_plugin.set(Some(half));
    };

    let close_panel = move || {
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
    let settings_open = RwSignal::new(false);
    let settings_provider = RwSignal::new(String::new());
    let settings_model = RwSignal::new(String::new());
    let settings_base_url = RwSignal::new(String::new());
    let settings_routes = RwSignal::new(Vec::<String>::new());
    let settings_status = RwSignal::new(String::new());
    let load_settings = move || {
        let value = token.get_untracked();
        settings_status.set("loading…".into());
        spawn_local(async move {
            match get_json::<ModelConfigResponse>("/v1/config/model", &value).await {
                Ok(config) => {
                    // The editable route is the one from the config layer; fall
                    // back to the first known route when the live process runs
                    // something else (for example the replay provider).
                    let provider = if config.provider.is_empty() {
                        config.routes.first().cloned().unwrap_or_default()
                    } else {
                        config.provider
                    };
                    settings_provider.set(provider);
                    settings_model.set(config.model);
                    settings_base_url.set(config.base_url.unwrap_or_default());
                    settings_routes.set(config.routes);
                    settings_status.set(format!(
                        "active: {} · {}",
                        config.active_provider, config.note
                    ));
                }
                Err(error) => settings_status.set(format!("error: {error}")),
            }
        });
    };
    let save_settings = move || {
        let value = token.get_untracked();
        let body = serde_json::json!({
            "provider": settings_provider.get_untracked(),
            "model": settings_model.get_untracked(),
            "base_url": settings_base_url.get_untracked(),
        });
        spawn_local(async move {
            match post_json::<Value>("/v1/config/model", &value, &body).await {
                Ok(answer) => {
                    let restart = answer["restart_required"].as_bool().unwrap_or(false);
                    settings_status.set(if restart {
                        "saved · restart `dshr serve` (или desktop), чтобы применить".into()
                    } else {
                        "saved".into()
                    });
                }
                Err(error) => settings_status.set(format!("error: {error}")),
            }
        });
    };
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
        // Every mounted frame follows the shell theme without a reload.
        push_theme_all(&value);
    };
    apply_theme(theme.get_untracked());
    provide_context(UiState {
        slots,
        token,
        theme,
        versions: frame_versions,
    });
    // Host state for C-lite hook props and `ctx.get('sessions')`. The sidebar is
    // always expanded in the dshr shell, so `wide` is constant for now.
    let session_entries = move || {
        sessions
            .get_untracked()
            .iter()
            .map(|session| (session.id.clone(), session.model.clone()))
            .collect::<Vec<_>>()
    };
    let broadcast_host_state = move || {
        let entries = session_entries();
        let current_session = current.get_untracked();
        push_host_state_all(
            &theme.get_untracked(),
            current_session.as_deref(),
            &entries,
        );
    };
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
                    broadcast_host_state();
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
                        let halves = list.clients;
                        apply_clients(clients, registry_frames, slots, halves.clone());
                        if let Some(name) = pending_panel.get_untracked()
                            && let Some(half) =
                                halves.iter().find(|half| half.name == name).cloned()
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

    // A token from the URL or localStorage connects without requiring a click:
    // every client half is mounted eagerly, so the shell needs the plugin list
    // before the first paint.
    if !token.get_untracked().trim().is_empty() {
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
        broadcast_host_state();
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
                    broadcast_host_state();
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

    // One window listener for the whole page: every mounted plugin iframe —
    // the eager registry frame and the per-region slot frames — reports here.
    {
        let on_message = Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
            let data = event.data();
            if !data.is_object() || field_of(&data, "ns") != "dshr-client" {
                return;
            }
            let plugin = field_of(&data, "plugin");
            let frame = field_of(&data, "frame");
            let mode = field_of(&data, "mode");
            let source = event.source();
            match field_of(&data, "type").as_str() {
                "ready" => {
                    // Frames get their theme from the host page; pushing it on
                    // ready keeps a reloaded frame consistent with the shell.
                    let theme = theme.get_untracked();
                    post_to_token(&frame, "theme", &[("theme", theme.as_str())]);
                    broadcast_host_state();
                }
                "slots" => {
                    // Only the eager registry frame defines the shell registry.
                    // Slot frames load the same bundle and re-register the same
                    // slots; letting them write here would duplicate frames.
                    if mode == "slot" {
                        return;
                    }
                    let registered = read_slots(&data, &plugin);
                    slots.update(|all| {
                        all.retain(|slot| slot.plugin != plugin);
                        all.extend(registered);
                        all.sort_by(|left, right| {
                            left.region
                                .cmp(&right.region)
                                .then(left.plugin.cmp(&right.plugin))
                                .then(left.order.cmp(&right.order))
                                .then(left.id.cmp(&right.id))
                        });
                    });
                }
                "resize" => apply_frame_height(&frame, number_of(&data, "height")),
                "error" => {
                    let message = field_of(&data, "message");
                    let message = if message.is_empty() {
                        "plugin error".to_owned()
                    } else {
                        message
                    };
                    // A panel-stage error belongs to the stage head; errors from
                    // a sidebar/settings/message frame surface in the status bar.
                    if frame.starts_with("slot:panel:") {
                        panel_error.set(message);
                    } else {
                        status.set(format!("{plugin}: {message}"));
                    }
                }
                "action" => {
                    let action = field_of(&data, "action");
                    let id = field_of(&data, "id");
                    let payload = js_sys::Reflect::get(&data, &"payload".into())
                        .unwrap_or(wasm_bindgen::JsValue::NULL);
                    match action.as_str() {
                        "close" => {
                            post_action_result(source, &id, true, "");
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
                            post_action_result(source, &id, true, "");
                        }
                        "notify" => {
                            let message = field_of(&payload, "message");
                            status.set(if message.is_empty() {
                                "plugin notification".into()
                            } else {
                                message
                            });
                            post_action_result(source, &id, true, "");
                        }
                        "reload" => {
                            let token_value = token.get_untracked();
                            let name = plugin.clone();
                            let reply = source.clone();
                            let reloaded = plugin.clone();
                            spawn_local(async move {
                                let body = serde_json::json!({"name": name});
                                let result =
                                    post_json::<Value>("/v1/plugins/reload", &token_value, &body)
                                        .await;
                                match &result {
                                    Ok(_) => {
                                        post_action_result(reply, &id, true, "");
                                        bump_frame(&reloaded);
                                    }
                                    Err(error) => post_action_result(reply, &id, false, error),
                                }
                                refresh_plugin_views(
                                    token_value,
                                    live_plugins,
                                    clients,
                                    registry_frames,
                                    slots,
                                );
                            });
                        }
                        other => {
                            post_action_result(
                                source,
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
            refresh_plugin_views(token_value, live_plugins, clients, registry_frames, slots);
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
                    refresh_plugin_views(token_value, live_plugins, clients, registry_frames, slots);
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
                        refresh_plugin_views(
                            token_value.clone(),
                            live_plugins,
                            clients,
                            registry_frames,
                            slots,
                        );
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

    // Message rows are keyed by position and read their text from the signal:
    // streaming must not remount a row, because that would reload the slot
    // frame mounted under it.
    let message_indices = move || {
        let session = current.get().unwrap_or_default();
        (0..messages.get().len())
            .map(|index| (session.clone(), index))
            .collect::<Vec<_>>()
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
            <SlotRegion region="shell.leading" />
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
                settings_open.set(true);
                load_settings();
            }>"Settings"</button>
            <button on:click=move |_| {
                let next = if theme.get_untracked() == "dark" { "light" } else { "dark" };
                theme.set(next.to_owned());
                apply_theme(next.to_owned());
                broadcast_host_state();
            }>"Theme"</button>
            <span id="version">{move || version.get()}</span>
            <span id="status">{move || status.get()}</span>
        </header>
        <main>
            <aside id="sessions">
                <SlotRegion region="sidebar.top" />
                <For
                    each=move || sessions.get()
                    key=|session| session.id.clone()
                    children=move |session| {
                        let id = session.id.clone();
                        let occurrence = session.id.clone();
                        let context = serde_json::json!({ "session": session.id }).to_string();
                        view! {
                            <div class="session-row">
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
                                <SlotRegion
                                    region="sidebar.session.action"
                                    occurrence=occurrence
                                    context=context
                                />
                            </div>
                        }
                    }
                />
                <SlotRegion region="sidebar.footer" />
            </aside>
            <section id="chat">
                <SlotRegion region="conversation.header" />
                <div id="messages">
                    <For
                        each=message_indices
                        key=|(session, index)| (session.clone(), *index)
                        children=move |(_, index)| {
                            let index_now = index;
                            let text = move || {
                                messages
                                    .get()
                                    .get(index_now)
                                    .map(|item| item.text.clone())
                                    .unwrap_or_default()
                            };
                            let role = messages
                                .get_untracked()
                                .get(index_now)
                                .map(|item| item.role.clone())
                                .unwrap_or_default();
                            let occurrence = index.to_string();
                            let context = serde_json::json!({
                                "index": index,
                                "role": role,
                            })
                            .to_string();
                            let tool = role == "tool";
                            let class = if tool {
                                "message tool".to_owned()
                            } else {
                                format!("message {role}")
                            };
                            view! {
                                <div class=class>
                                    {text}
                                    {if tool {
                                        view! {
                                            <SlotRegion
                                                region="tool.card"
                                                occurrence=occurrence
                                                context=context
                                            />
                                        }
                                            .into_any()
                                    } else {
                                        view! {
                                            <SlotRegion
                                                region="message.action"
                                                occurrence=occurrence
                                                context=context
                                            />
                                        }
                                            .into_any()
                                    }}
                                </div>
                            }
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
                        <SlotRegion region="composer.actions" />
                    </div>
                </form>
            </section>
        </main>
        <SlotRegion region="shell.bottom" />
        <SlotRegion region="shell.overlay" />
        <RegistryFrames registry_frames=registry_frames clients=clients />
        {move || settings_open.get().then(|| view! {
            <dialog open>
                <SlotRegion region="settings.icon" />
                <h3>"Provider settings"</h3>
                <div class="row">
                    <label>"Route"</label>
                    <select
                        prop:value=move || settings_provider.get()
                        on:change=move |event| settings_provider.set(event_target_value(&event))
                    >
                        {move || settings_routes.get().into_iter().map(|route| {
                            let label = route.clone();
                            view! { <option value=route>{label}</option> }
                        }).collect_view()}
                    </select>
                </div>
                <div class="row">
                    <label>"Model"</label>
                    <input
                        prop:value=move || settings_model.get()
                        on:input=move |event| settings_model.set(event_target_value(&event))
                    />
                </div>
                <div class="row">
                    <label>"Base URL"</label>
                    <input
                        placeholder="optional, https://…"
                        prop:value=move || settings_base_url.get()
                        on:input=move |event| settings_base_url.set(event_target_value(&event))
                    />
                </div>
                <SlotRegion region="settings.section" />
                <p class="tool">{move || settings_status.get()}</p>
                <div class="row">
                    <button on:click=move |_| save_settings()>"Save"</button>
                    <button on:click=move |_| settings_open.set(false)>"Close"</button>
                </div>
                <SlotRegion region="settings.action" />
                <p>"Ключи не вводятся в UI: они читаются из окружения (DEEPSEEK_API_KEY / OPENAI_API_KEY) или desktop.env."</p>
            </dialog>
        })}
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
                <SlotRegion region="plugins.add" />
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
                <SlotRegion region="plugins.detail" />
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
        <PanelRegion
            panel=panel_plugin
            active_slot=active_slot
            slots=slots
            error=panel_error
        />
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
        {move || approval.get().map(|info| {
            let context = serde_json::json!({ "id": info.id }).to_string();
            view! {
            <dialog open>
                <h3>"Approval required"</h3>
                <p>{format!("{}: {}", info.tool, info.summary)}</p>
                <p>{info.reason}</p>
                <SlotRegion region="approval.detail" context=context />
                <div class="row">
                    <button on:click=move |_| resolve(true)>"Allow"</button>
                    <button on:click=move |_| resolve(false)>"Deny"</button>
                </div>
            </dialog>
        }})}
    }
}

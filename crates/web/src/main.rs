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

/// File opened through an owner callback (`openFile`) in the shell inspector.
#[derive(Clone, Debug, PartialEq)]
struct FilePreview {
    path: String,
    content: String,
    error: String,
    loading: bool,
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
    /// Reference slot kind (`single`/`list`/`keyed`/`chain`/`hoc`).
    kind: String,
    /// Shadowing priority (lower renders) and the keyed registration key.
    priority: i64,
    key: String,
    /// Child slot keys the entry declared; they mount through the parent anchor.
    children: Vec<String>,
}

/// One nested-slot anchor reported by a parent frame (`renderSlot` child).
#[derive(Clone, Debug, PartialEq)]
struct SlotAnchor {
    id: String,
    host: String,
    slot: String,
    owner_props: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

/// Reference chain slot election state: candidates in render order, the index of
/// the frame currently mounted (the owner-renderer mounts ONE at a time), and
/// whether that frame won the election. `generation` re-elects when the owner
/// share changes (session switch, node data).
#[derive(Clone, Debug, PartialEq)]
struct ChainInstance {
    candidates: Vec<ClientSlot>,
    occurrence: String,
    token: String,
    index: usize,
    winner: bool,
    generation: String,
}

/// Shell state shared by slot-frame components through the Leptos context.
#[derive(Clone, Copy)]
struct UiState {
    slots: RwSignal<Vec<ClientSlot>>,
    token: RwSignal<String>,
    theme: RwSignal<String>,
    versions: RwSignal<BTreeMap<String, u32>>,
    /// Nested-slot anchors, keyed by the parent frame token.
    anchors: RwSignal<BTreeMap<String, Vec<SlotAnchor>>>,
    /// Chain election state, keyed by `<slot>|<occurrence>`.
    chains: RwSignal<BTreeMap<String, ChainInstance>>,
    /// Frame token -> error message for renderers whose frame crashed. A failed
    /// keyed-cell winner yields to the shell's built-in renderer (C5.2).
    renderer_errors: RwSignal<BTreeMap<String, String>>,
    /// Nested child frames: frame token -> (parent frame token, anchor id), so a
    /// child's measured height can be reserved in the parent's layout.
    nested: RwSignal<BTreeMap<String, (String, String)>>,
    /// Chat-view nodes (the conversation the shell projects for chat slots).
    nodes: RwSignal<Vec<ChatNode>>,
}

/// Child slot keys declared by some registration: those mount through their
/// parent entry's anchor, never flatly in the shell region.
fn child_slot_keys(slots: &[ClientSlot]) -> std::collections::BTreeSet<String> {
    slots
        .iter()
        .flat_map(|slot| slot.children.iter().cloned())
        .collect()
}

/// Candidate order for a chain slot: lowest priority first, then explicit order,
/// then plugin/registration id (stable across registry messages).
fn chain_candidates(slots: &[ClientSlot], name: &str) -> Vec<ClientSlot> {
    let mut out: Vec<ClientSlot> = slots
        .iter()
        .filter(|slot| slot.name == name && slot.kind == "chain")
        .cloned()
        .collect();
    out.sort_by(|left, right| {
        left.priority
            .cmp(&right.priority)
            .then(left.order.cmp(&right.order))
            .then(left.plugin.cmp(&right.plugin))
            .then(left.id.cmp(&right.id))
    });
    out
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

/// One `anchor` bridge message (a child slot placeholder inside a parent frame).
fn read_anchor(data: &wasm_bindgen::JsValue) -> Option<SlotAnchor> {
    let id = field_of(data, "anchor");
    if id.is_empty() {
        return None;
    }
    let rect = js_sys::Reflect::get(data, &"rect".into()).unwrap_or(wasm_bindgen::JsValue::NULL);
    Some(SlotAnchor {
        id,
        host: field_of(data, "host"),
        slot: field_of(data, "slot"),
        owner_props: field_of(data, "ownerProps"),
        x: number_of(&rect, "x"),
        y: number_of(&rect, "y"),
        width: number_of(&rect, "width"),
        height: number_of(&rect, "height"),
    })
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

/// String array field of a bridge message (e.g. an entry's declared children).
fn read_strings(value: &wasm_bindgen::JsValue, key: &str) -> Vec<String> {
    let Ok(list) = js_sys::Reflect::get(value, &key.into()) else {
        return Vec::new();
    };
    let Ok(list) = list.dyn_into::<js_sys::Array>() else {
        return Vec::new();
    };
    list.iter()
        .filter_map(|item| item.as_string())
        .collect()
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
            let kind = field_of(&value, "kind");
            let key = field_of(&value, "key");
            let children = read_strings(&value, "children");
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
                kind: if kind.is_empty() { "list".into() } else { kind },
                priority: number_of(&value, "priority") as i64,
                key,
                children,
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

/// Renders one slot registration in its own sandboxed iframe, plus the nested
/// child-slot anchors the registration reported through `renderSlot`.
///
/// The frame loads the same bundle in `mode=slot`, so the bridge renders exactly
/// this registration and its `ctx.effect`s run in an isolated document. Nothing
/// about the shell layout reaches the plugin — except the anchors: the wrapper is
/// the positioning context, so a child slot frame mounts visually inside the
/// parent's `renderSlot` placeholder (reference hoc nesting between frames).
#[component]
fn SlotFrame(
    entry: ClientSlot,
    token: String,
    #[prop(default = String::new())] context: String,
    #[prop(default = String::new())] owner_props: String,
) -> AnyView {
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
    // Big owner shares (a whole chat node) do not fit a URL: they ride in a DOM
    // attribute and are pushed as an `owner-props` message once the frame is
    // ready, so the frame re-renders with the full share.
    let deferred_owner = owner_props.chars().count() > 3000;
    let owner_query = if owner_props.is_empty() || deferred_owner {
        String::new()
    } else {
        format!("&ownerProps={}", encode(&owner_props))
    };
    let owner_attribute = if deferred_owner {
        owner_props.clone()
    } else {
        String::new()
    };
    let base = format!(
        "/v1/plugins/{plugin}/host?token={token}&theme={theme}&frame={frame}&mode=slot&slot={name}&id={id}&context={context}{owner_query}",
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
    let anchor_token = token.clone();
    let parent_token = token.clone();
    let frame_token = token.clone();
    // Leaving the DOM drops the anchors this frame reported.
    let cleanup_token = token.clone();
    on_cleanup(move || {
        ui.anchors.update(|map| {
            map.remove(&cleanup_token);
        });
    });
    let anchors = move || {
        ui.anchors
            .get()
            .get(&anchor_token)
            .cloned()
            .unwrap_or_default()
    };
    view! {
        <div class="slot-host" data-frame-token=frame_token>
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
                data-owner-props=owner_attribute
                src=src
            ></iframe>
            <For
                each=anchors
                key=|anchor| anchor.id.clone()
                children=move |anchor| {
                    let style = format!(
                        "left:{}px;top:{}px;width:{}px;",
                        anchor.x,
                        anchor.y,
                        anchor.width.max(32.0)
                    );
                    view! {
                        <div class="nested-slot" style=style>
                            <NestedSeat
                                name=anchor.slot.clone()
                                owner_props=anchor.owner_props.clone()
                                occurrence=anchor.id.clone()
                                context="{}".to_owned()
                                parent=parent_token.clone()
                            />
                        </div>
                    }
                }
            />
        </div>
    }
        .into_any()
}

/// Mounts the registrations of one child slot key at its anchor inside the
/// parent wrapper. Chain children go through the same single owner-renderer.
#[component]
fn NestedSeat(
    name: String,
    #[prop(default = String::new())] owner_props: String,
    #[prop(default = String::new())] occurrence: String,
    #[prop(default = String::new())] context: String,
    #[prop(default = String::new())] parent: String,
) -> AnyView {
    let ui = expect_context::<UiState>();
    // Register every child frame token with its anchor so `resize` can reserve
    // the child's height in the parent's layout (two-way sizing without a
    // shared JS realm).
    let cleanup_anchor = occurrence.clone();
    on_cleanup(move || {
        ui.nested.update(|map| {
            map.retain(|_, value| value.1 != cleanup_anchor);
        });
    });
    let name_nested = name.clone();
    let occurrence_nested = occurrence.clone();
    let parent_nested = parent.clone();
    Effect::new(move |_| {
        let name = name_nested.clone();
        let anchor = occurrence_nested.clone();
        let parent = parent_nested.clone();
        if anchor.is_empty() {
            return;
        }
        let tokens: Vec<String> = ui
            .slots
            .get()
            .into_iter()
            .filter(|candidate| candidate.name == name && candidate.kind != "chain")
            .map(|candidate| slot_token(&candidate, &anchor))
            .collect();
        ui.nested.update(|map| {
            map.retain(|token, value| value.1 != anchor || tokens.contains(token));
            for token in &tokens {
                map.insert(token.clone(), (parent.clone(), anchor.clone()));
            }
        });
    });
    let slot_plain = name.clone();
    let occurrence_plain = occurrence.clone();
    let plain = move || {
        let name = slot_plain.clone();
        let occurrence = occurrence_plain.clone();
        ui.slots
            .get()
            .into_iter()
            .filter(|candidate| candidate.name == name && candidate.kind != "chain")
            .map(|candidate| {
                let token = slot_token(&candidate, &occurrence);
                (candidate, token)
            })
            .collect::<Vec<_>>()
    };
    let slot_chain = name.clone();
    let chain = move || {
        !chain_candidates(&ui.slots.get(), &slot_chain).is_empty()
    };
    let slot_chain_view = name.clone();
    let occurrence_chain = occurrence.clone();
    let context_chain = context.clone();
    let owner_chain = owner_props.clone();
    view! {
        <div class="nested-seat" data-slot=name.clone()>
            {move || chain().then(|| view! {
                <ChainHost
                    name=slot_chain_view.clone()
                    occurrence=occurrence_chain.clone()
                    context=context_chain.clone()
                    owner_props=owner_chain.clone()
                />
            })}
            <For
                each=plain
                key=|(slot, token)| (token.clone(), slot.plugin.clone(), slot.id.clone())
                children=move |(slot, token)| {
                    view! {
                        <SlotFrame
                            entry=slot
                            token=token
                            context=context.clone()
                            owner_props=owner_props.clone()
                        />
                    }
                }
            />
        </div>
    }
        .into_any()
}

/// Single owner-renderer for one chain slot.
///
/// Reference chain election renders the first registration whose `select(ownerProps)`
/// does not decline, with `{...ownerProps, matched}`. The shell mounts one
/// candidate frame at a time; a frame that declines reports `chain-unselected`
/// and the shell advances, so several `select === true` registrations never
/// render several frames (the round-102 per-frame filtering is gone).
#[component]
fn ChainHost(
    name: String,
    #[prop(optional)] occurrence: Option<String>,
    #[prop(optional)] context: Option<String>,
    #[prop(optional)] owner_props: Option<String>,
) -> AnyView {
    let ui = expect_context::<UiState>();
    let occurrence = occurrence.unwrap_or_default();
    let context = context.unwrap_or_default();
    let owner_props = owner_props.unwrap_or_default();
    let key = format!("{name}|{occurrence}");
    let generation = format!("{owner_props}|{context}");
    let name_effect = name.clone();
    let key_effect = key.clone();
    let occurrence_effect = occurrence.clone();
    let generation_effect = generation.clone();
    // (Re)initialise the election whenever the candidate set or the owner share
    // changes: a different session or node is a different election.
    Effect::new(move |_| {
        let candidates = chain_candidates(&ui.slots.get(), &name_effect);
        ui.chains.update(|map| {
            let existing = map.get(&key_effect).cloned();
            let same_candidates = existing
                .as_ref()
                .is_some_and(|state| state.candidates == candidates);
            let same_generation = existing
                .as_ref()
                .is_some_and(|state| state.generation == generation_effect);
            if same_candidates && same_generation {
                return;
            }
            if candidates.is_empty() {
                map.remove(&key_effect);
                return;
            }
            let token = slot_token(&candidates[0], &occurrence_effect);
            map.insert(
                key_effect.clone(),
                ChainInstance {
                    candidates,
                    occurrence: occurrence_effect.clone(),
                    token,
                    index: 0,
                    winner: false,
                    generation: generation_effect.clone(),
                },
            );
        });
    });
    let key_view = key.clone();
    let occurrence_view = occurrence.clone();
    let current = move || {
        let map = ui.chains.get();
        let state = map.get(&key_view)?;
        if state.index >= state.candidates.len() {
            return None;
        }
        Some((state.candidates[state.index].clone(), state.token.clone()))
    };
    view! {
        <div class="chain-host" data-chain=name.clone() data-occurrence=occurrence_view.clone()>
            {move || {
                current().map(|(slot, token)| {
                    view! {
                        <SlotFrame
                            entry=slot
                            token=token
                            context=context.clone()
                            owner_props=owner_props.clone()
                        />
                    }
                })
            }}
        </div>
    }
        .into_any()
}

/// Mount point of one shell region: renders one slot frame per registration the
/// bridge routed here. `occurrence` distinguishes repeated instances of the
/// same region (the second message, the fourth session row, ...).
///
/// Chain slots in the region are served by one `ChainHost` each (single owner
/// renderer), and slots declared as another entry's children are excluded: they
/// mount through their parent's anchor instead of flatly here.
#[component]
fn SlotRegion(
    region: &'static str,
    #[prop(optional)] occurrence: Option<String>,
    #[prop(optional)] context: Option<String>,
    #[prop(optional)] owner_props: Option<String>,
    /// Keyed-region occurrence key (`conversation.chat.node` node kind, ...).
    #[prop(optional)] entry_key: Option<String>,
) -> impl IntoView {
    let ui = expect_context::<UiState>();
    let occurrence = occurrence.unwrap_or_default();
    let context = context.unwrap_or_default();
    let owner_props = owner_props.unwrap_or_default();
    let entry_key = entry_key.unwrap_or_default();
    let occurrences = occurrence.clone();
    let each_occurrence = occurrence.clone();
    let key_filter = entry_key.clone();
    let plain = move || {
        let occurrence = each_occurrence.clone();
        let children = child_slot_keys(&ui.slots.get());
        let entry_key = key_filter.clone();
        ui.slots
            .get()
            .into_iter()
            .filter(|slot| {
                slot.region == region
                    && slot.kind != "chain"
                    && !children.contains(&slot.name)
                    && (entry_key.is_empty()
                        || slot.kind != "keyed"
                        || slot.key == entry_key)
            })
            .map(|slot| {
                let token = slot_token(&slot, &occurrence);
                (slot, token)
            })
            .collect::<Vec<_>>()
    };
    let chains = move || {
        let slots = ui.slots.get();
        let children = child_slot_keys(&slots);
        let mut names: Vec<String> = Vec::new();
        for slot in slots.iter() {
            if slot.region == region
                && slot.kind == "chain"
                && !children.contains(&slot.name)
                && !names.iter().any(|seen| seen == &slot.name)
            {
                names.push(slot.name.clone());
            }
        }
        names
    };
    let chain_occurrence = occurrence.clone();
    let chain_context = context.clone();
    let chain_owner = owner_props.clone();
    view! {
        <div class="slot-region" data-region=region data-occurrence=occurrences>
            <For
                each=chains
                key=|name| name.clone()
                children=move |name| {
                    view! {
                        <ChainHost
                            name=name
                            occurrence=chain_occurrence.clone()
                            context=chain_context.clone()
                            owner_props=chain_owner.clone()
                        />
                    }
                }
            />
            <For
                each=plain
                key=|(slot, token)| (token.clone(), slot.plugin.clone(), slot.id.clone())
                children=move |(slot, token)| {
                    view! {
                        <SlotFrame
                            entry=slot
                            token=token
                            context=context.clone()
                            owner_props=owner_props.clone()
                        />
                    }
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

/// One tool call inside an assistant node (reference `ToolCallBlock` subset).
#[derive(Clone, Debug, PartialEq)]
struct ToolCallView {
    id: String,
    name: String,
    input: Value,
    summary: String,
    ok: Option<bool>,
    result: String,
}

/// One chat-view node: the shell's projection of a session message into the
/// reference turn/node model. `kind` is the keyed `conversation.chat.node`
/// renderer key (`user` / `assistant-step`), so a plugin that registers that key
/// replaces the built-in row.
#[derive(Clone, Debug, PartialEq)]
struct ChatNode {
    key: String,
    kind: String,
    turn: usize,
    status: String,
    text: String,
    reasoning: String,
    images: Vec<String>,
    calls: Vec<ToolCallView>,
    seq: usize,
    message_id: String,
}

impl ChatNode {
    fn placeholder(seq: usize) -> Self {
        Self {
            key: format!("0:{seq}"),
            kind: "assistant-step".into(),
            turn: 0,
            status: "settled".into(),
            text: String::new(),
            reasoning: String::new(),
            images: Vec::new(),
            calls: Vec::new(),
            seq,
            message_id: format!("m{seq}"),
        }
    }

    fn is_empty(&self) -> bool {
        self.text.trim().is_empty()
            && self.reasoning.trim().is_empty()
            && self.images.is_empty()
            && self.calls.is_empty()
    }
}

fn block_type(block: &Value) -> &str {
    block.get("type").and_then(Value::as_str).unwrap_or_default()
}

fn image_url(block: &Value) -> Option<String> {
    let media = block
        .get("media_type")
        .and_then(Value::as_str)
        .unwrap_or("image/png");
    let data = block.get("data").and_then(Value::as_str)?;
    Some(format!("data:{media};base64,{data}"))
}

fn text_block(block: &Value) -> &str {
    block.get("text").and_then(Value::as_str).unwrap_or_default()
}

/// Project the session transcript into the reference turn/node model.
///
/// A user message opens a turn; assistant messages become `assistant-step`
/// nodes with text/reasoning/tool-call blocks; a `tool_result` block attaches to
/// the call it answers. This is the data the `conversation.chat.*` slots render.
fn build_chat_nodes(messages: &[Message]) -> Vec<ChatNode> {
    let mut nodes: Vec<ChatNode> = Vec::new();
    let mut turn = 0usize;
    for (seq, message) in messages.iter().enumerate() {
        match message.role.as_str() {
            "assistant" => {
                let mut node = ChatNode::placeholder(seq);
                node.kind = "assistant-step".into();
                node.turn = turn;
                for block in &message.content {
                    match block_type(block) {
                        "text" => {
                            if !node.text.is_empty() {
                                node.text.push_str("\n\n");
                            }
                            node.text.push_str(text_block(block));
                        }
                        "thinking" => node.reasoning.push_str(text_block(block)),
                        "tool_use" => {
                            let name = block
                                .get("name")
                                .and_then(Value::as_str)
                                .unwrap_or("tool")
                                .to_owned();
                            let input = block.get("input").cloned().unwrap_or(Value::Null);
                            let summary = tool_diff_preview(&name, &input);
                            node.calls.push(ToolCallView {
                                id: block
                                    .get("id")
                                    .and_then(Value::as_str)
                                    .unwrap_or_default()
                                    .to_owned(),
                                name,
                                input,
                                summary,
                                ok: None,
                                result: String::new(),
                            });
                        }
                        "image" => {
                            if let Some(url) = image_url(block) {
                                node.images.push(url);
                            }
                        }
                        _ => {}
                    }
                }
                node.key = format!("{turn}:{seq}");
                if !node.is_empty() {
                    nodes.push(node);
                }
            }
            "user" => {
                let mut text = String::new();
                let mut images = Vec::new();
                let mut results = Vec::new();
                for block in &message.content {
                    match block_type(block) {
                        "text" => {
                            if !text.is_empty() {
                                text.push_str("\n\n");
                            }
                            text.push_str(text_block(block));
                        }
                        "image" => {
                            if let Some(url) = image_url(block) {
                                images.push(url);
                            }
                        }
                        "tool_result" => results.push(block.clone()),
                        _ => {}
                    }
                }
                for block in results {
                    let id = block
                        .get("tool_use_id")
                        .and_then(Value::as_str)
                        .unwrap_or_default();
                    let ok = !block
                        .get("is_error")
                        .and_then(Value::as_bool)
                        .unwrap_or(false);
                    let content = block
                        .get("content")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_owned();
                    if let Some(call) = nodes
                        .iter_mut()
                        .rev()
                        .flat_map(|node| node.calls.iter_mut())
                        .find(|call| call.id == id)
                    {
                        call.ok = Some(ok);
                        call.result = content;
                    }
                }
                if text.trim().is_empty() && images.is_empty() {
                    continue;
                }
                turn += 1;
                nodes.push(ChatNode {
                    key: format!("{turn}:{seq}"),
                    kind: "user".into(),
                    turn,
                    status: "settled".into(),
                    text,
                    reasoning: String::new(),
                    images,
                    calls: Vec::new(),
                    seq,
                    message_id: format!("m{seq}"),
                });
            }
            _ => {}
        }
    }
    nodes
}

/// Turn ordinal of the chat the streaming answer belongs to.
fn current_turn(nodes: &[ChatNode]) -> usize {
    nodes.last().map(|node| node.turn).unwrap_or(0)
}

/// Appends streamed assistant text to the running node (or opens one).
fn push_text(nodes: RwSignal<Vec<ChatNode>>, streaming: RwSignal<bool>, text: &str) {
    nodes.update(|items| {
        if streaming.get_untracked()
            && let Some(last) = items.last_mut()
            && last.kind == "assistant-step"
            && last.status == "running"
        {
            last.text.push_str(text);
            return;
        }
        let turn = current_turn(items);
        let seq = items.len();
        let mut node = ChatNode::placeholder(seq);
        node.kind = "assistant-step".into();
        node.turn = turn;
        node.status = "running".into();
        node.text = text.to_owned();
        node.key = format!("{turn}:{seq}");
        items.push(node);
    });
    streaming.set(true);
}

/// Ensures a running assistant node exists for a tool call.
fn running_assistant(nodes: &mut Vec<ChatNode>) -> &mut ChatNode {
    let needs_push = !matches!(
        nodes.last(),
        Some(node) if node.kind == "assistant-step" && node.status == "running"
    );
    if needs_push {
        let turn = current_turn(nodes);
        let seq = nodes.len();
        let mut node = ChatNode::placeholder(seq);
        node.kind = "assistant-step".into();
        node.turn = turn;
        node.status = "running".into();
        node.key = format!("{turn}:{seq}");
        nodes.push(node);
    }
    nodes.last_mut().expect("running assistant node")
}

fn push_tool_start(
    nodes: RwSignal<Vec<ChatNode>>,
    streaming: RwSignal<bool>,
    name: &str,
    summary: &str,
    input: Value,
) {
    let preview = tool_diff_preview(name, &input);
    nodes.update(|items| {
        let node = running_assistant(items);
        let seq = node.calls.len();
        node.calls.push(ToolCallView {
            id: format!("call-{seq}"),
            name: name.to_owned(),
            input,
            summary: if summary.is_empty() {
                preview
            } else {
                format!("{summary}{preview}")
            },
            ok: None,
            result: String::new(),
        });
    });
    streaming.set(true);
}

fn push_tool_end(
    nodes: RwSignal<Vec<ChatNode>>,
    name: &str,
    ok: bool,
    content: &str,
) {
    nodes.update(|items| {
        let pending = items
            .iter_mut()
            .rev()
            .flat_map(|node| node.calls.iter_mut())
            .find(|call| call.name == name && call.ok.is_none());
        match pending {
            Some(call) => {
                call.ok = Some(ok);
                call.result = content.to_owned();
            }
            None => {
                let node = running_assistant(items);
                let seq = node.calls.len();
                node.calls.push(ToolCallView {
                    id: format!("call-{seq}"),
                    name: name.to_owned(),
                    input: Value::Null,
                    summary: String::new(),
                    ok: Some(ok),
                    result: content.to_owned(),
                });
            }
        }
    });
}

fn connect_events(
    session: String,
    token: String,
    nodes: RwSignal<Vec<ChatNode>>,
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
                    push_text(nodes, streaming, text);
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
                push_tool_start(nodes, streaming, name, summary, parsed);
            }
            "tool_end" => {
                let name = value["name"].as_str().unwrap_or("tool");
                let ok = value["ok"].as_bool().unwrap_or(true);
                let content = value["content"].as_str().unwrap_or("").trim();
                push_tool_end(nodes, name, ok, content);
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
                let text = value["text"].as_str().unwrap_or_default().to_owned();
                nodes.update(|items| {
                    if let Some(last) = items.last_mut()
                        && last.kind == "assistant-step"
                    {
                        if !text.is_empty() {
                            last.text = text.clone();
                        }
                        last.status = "settled".into();
                    } else if !text.is_empty() {
                        let turn = current_turn(items);
                        let seq = items.len();
                        let mut node = ChatNode::placeholder(seq);
                        node.turn = turn;
                        node.text = text.clone();
                        node.key = format!("{turn}:{seq}");
                        items.push(node);
                    }
                });
            }
            "cancelled" => {
                streaming.set(false);
                busy.set(false);
                status.set("cancelled".into());
                nodes.update(|items| {
                    if let Some(last) = items.last_mut() {
                        last.status = "interrupted".into();
                    }
                });
            }
            "error" => {
                streaming.set(false);
                busy.set(false);
                let message = value["message"].as_str().unwrap_or("error");
                status.set(format!("error: {message}"));
                nodes.update(|items| {
                    if let Some(last) = items.last_mut() {
                        last.status = "interrupted".into();
                    }
                });
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

/// Owner share of one chat node (reference `ChatNodeOwnerProps`): the node
/// itself plus the render-site callbacks, which travel as `$callback` markers.
fn chat_node_owner(node: &ChatNode) -> String {
    fn clip(text: &str, limit: usize) -> String {
        if text.chars().count() <= limit {
            return text.to_owned();
        }
        let mut out: String = text.chars().take(limit).collect();
        out.push('…');
        out
    }
    let blocks: Vec<Value> = if node.kind == "user" {
        Vec::new()
    } else {
        node.calls
            .iter()
            .map(|call| {
                serde_json::json!({
                    "kind": "tool-call",
                    "id": call.id,
                    "name": call.name,
                    "status": match call.ok {
                        Some(true) => "done",
                        Some(false) => "error",
                        None => "running",
                    },
                    "summary": clip(&call.summary, 240),
                })
            })
            .collect()
    };
    // dshr holds image bytes inline (base64 data URLs); only share them when the
    // bundle stays small, otherwise a plugin gets the count and the `*.images`
    // seats render them.
    let images: Vec<String> = node.images.iter().take(4).cloned().collect();
    let shared_images = if serde_json::to_string(&images).map(|json| json.len() <= 16_000).unwrap_or(false) {
        images
    } else {
        Vec::new()
    };
    let data = serde_json::json!({
        "status": node.status,
        "turn": node.turn,
        "step": 0,
        "time": 0,
        "blocks": blocks,
        "text": clip(&node.text, 3000),
        "images": shared_images,
        "imageCount": node.images.len(),
    });
    serde_json::json!({
        "node": {
            "key": node.key,
            "kind": node.kind,
            "data": data,
            "location": {"kind": "turn", "turn": node.turn},
        },
        "selectedCallId": null,
        "cwd": "",
        "fileMentions": null,
        "openFile": {"$callback": "openFile"},
        "inspectCall": {"$callback": "inspectCall"},
        "forkAt": {"$callback": "forkAt"},
        "loadImage": {"$callback": "loadImage"},
    })
    .to_string()
}

/// One tool call card (reference `ToolCall` row subset): the `tool.call.toolview`
/// seat mounts under it with the call as owner props, and `inspectCall` is a
/// forwarded owner callback the plugin can invoke.
#[component]
fn ToolCallCard(call: ToolCallView, occurrence: String) -> impl IntoView {
    let status = match call.ok {
        Some(true) => "ok",
        Some(false) => "error",
        None => "running",
    };
    let owner = serde_json::json!({
        "callId": call.id,
        "toolName": call.name,
        "block": {
            "kind": if call.ok.is_some() { "tool-result" } else { "tool-call" },
            "name": call.name,
            "input": call.input,
            "result": call.result,
        },
        "cwd": "",
        "selected": false,
        "openFile": {"$callback": "openFile"},
        "inspectCall": {"$callback": "inspectCall"},
    })
    .to_string();
    let context = serde_json::json!({"call": call.id, "name": call.name}).to_string();
    let name = call.name.clone();
    let call_name = call.name.clone();
    let summary = call.summary.clone();
    let result = call.result.clone();
    let call_id = call.id.clone();
    view! {
        <div class="tool-card" data-call=call_id data-status=status>
            <div class="tool-head">
                <strong>{name}</strong>
                <span class="tool-status">{status}</span>
            </div>
            {(!summary.is_empty()).then(|| view! { <p class="tool">{summary.clone()}</p> })}
            {(!result.is_empty()).then(|| view! { <pre class="tool-output">{result.clone()}</pre> })}
            <SlotRegion
                region="tool.card"
                occurrence=occurrence
                context=context
                owner_props=owner
                entry_key=call_name
            />
        </div>
    }
}

/// Tool calls of one assistant node, keyed by call so streaming appends cards
/// without remounting the ones already on screen.
#[component]
fn ToolCallList(index: usize) -> impl IntoView {
    let ui = expect_context::<UiState>();
    let calls = move || {
        ui.nodes
            .get()
            .get(index)
            .map(|node| node.calls.clone())
            .unwrap_or_default()
    };
    view! {
        <For
            each=calls
            key=|call| call.id.clone()
            children=move |call| {
                let occurrence = call.id.clone();
                view! { <ToolCallCard call=call occurrence=occurrence /> }
            }
        />
    }
}

/// Reasoning body of one assistant node (reference
/// `conversation.chat.reasoning.body` single seat over the built-in disclosure).
#[component]
fn ChatReasoning(
    index: usize,
    occurrence: String,
    context: String,
    owner_props: String,
) -> impl IntoView {
    let ui = expect_context::<UiState>();
    let reasoning = move || {
        ui.nodes
            .get()
            .get(index)
            .map(|node| node.reasoning.clone())
            .unwrap_or_default()
    };
    let present = reasoning.clone();
    let body = reasoning.clone();
    view! {
        <Show when=move || !present().trim().is_empty()>
            <details class="reasoning">
                <summary>"Reasoning"</summary>
                <SlotRegion
                    region="chat.reasoning"
                    occurrence=occurrence.clone()
                    context=context.clone()
                    owner_props=owner_props.clone()
                />
                <pre class="reasoning-text">{body}</pre>
            </details>
        </Show>
    }
}

/// Message images (reference `conversation.chat.message.images` /
/// `conversation.chat.user.images` single seats).
#[component]
fn ChatImages(
    images: Vec<String>,
    region: &'static str,
    occurrence: String,
    context: String,
    owner_props: String,
) -> impl IntoView {
    let present = images.clone();
    let list = Signal::derive(move || images.clone());
    view! {
        <Show when=move || !present.is_empty()>
            <div class="chat-images">
                <For
                    each=move || list.get()
                    key=|url| url.clone()
                    children=move |url| view! { <img class="chat-image" src=url /> }
                />
                <SlotRegion
                    region=region
                    occurrence=occurrence.clone()
                    context=context.clone()
                    owner_props=owner_props.clone()
                />
            </div>
        </Show>
    }
}

/// Priority of the shell's built-in chat-node renderer in the keyed cell.
///
/// The reference chat package is an ordinary SlotMap entry, so `dsh-genui`
/// shadows `assistant-step` with `priority: -1` and captures the built-in
/// component. The shell keeps the shadowing contract; the capture half is
/// impossible for a Leptos view inside a sandboxed frame (documented in
/// ADR-042) and a plugin with the default priority keeps the built-in view.
const CORE_RENDERER_PRIORITY: i64 = 0;

/// Node kinds the shell's core chat view owns.
fn is_core_chat_kind(kind: &str) -> bool {
    matches!(kind, "user" | "assistant-step")
}

/// Which renderer owns one `conversation.chat.node` cell.
#[derive(Clone, Debug, PartialEq)]
enum ChatNodeRenderer {
    /// The shell's built-in view (no registration, a non-shadowing one, or a
    /// crashed winner).
    Builtin,
    /// A plugin frame, identified by the token the shell mounts for it.
    Plugin { token: String },
}

/// Resolves one chat-node cell: built-in view or the winning plugin frame.
///
/// `errors` holds frame tokens that reported a crash; a crashed winner yields
/// to the built-in view, so a broken renderer never leaves an empty message.
fn chat_node_renderer(
    slots: &[ClientSlot],
    kind: &str,
    occurrence: &str,
    errors: &BTreeMap<String, String>,
) -> ChatNodeRenderer {
    let core = is_core_chat_kind(kind);
    let mut candidates: Vec<&ClientSlot> = slots
        .iter()
        .filter(|slot| {
            slot.name == "conversation.chat.node" && slot.kind == "keyed" && slot.key == kind
        })
        .filter(|slot| !core || slot.priority < CORE_RENDERER_PRIORITY)
        .collect();
    candidates.sort_by(|left, right| {
        left.priority
            .cmp(&right.priority)
            .then(left.order.cmp(&right.order))
            .then(left.plugin.cmp(&right.plugin))
            .then(left.id.cmp(&right.id))
    });
    match candidates.first() {
        None => ChatNodeRenderer::Builtin,
        Some(slot) => {
            let token = slot_token(slot, occurrence);
            if errors.contains_key(&token) {
                ChatNodeRenderer::Builtin
            } else {
                ChatNodeRenderer::Plugin { token }
            }
        }
    }
}

/// One conversation row: the reference chat view's node seat.
///
/// The keyed `conversation.chat.node` renderer replaces the built-in view when a
/// plugin shadows that cell (core kinds: a negative priority); the per-node
/// surfaces (reasoning, images, text interaction, user.before chain) mount
/// around it.
#[component]
fn ChatNodeRow(index: usize) -> impl IntoView {
    let ui = expect_context::<UiState>();
    let initial = ui
        .nodes
        .get_untracked()
        .get(index)
        .cloned()
        .unwrap_or_else(|| ChatNode::placeholder(index));
    let key = initial.key.clone();
    let kind = initial.kind.clone();
    let turn = initial.turn;
    let occurrence = key.clone();
    let context = serde_json::json!({"node": key, "kind": kind, "index": index}).to_string();
    let owner = chat_node_owner(&initial);
    let is_user = kind == "user";
    let images = initial.images.clone();
    let user_owner = serde_json::json!({
        "source": {"role": "user", "index": index},
        "kind": "user",
        "node": {"key": initial.key, "kind": "user", "data": {"text": initial.text}},
    })
    .to_string();
    // `.message` stays on the row: the legacy `message.action` extension seat
    // and the existing smoke/e2e selectors key off it.
    let class = if is_user {
        "chat-node message user".to_owned()
    } else {
        "chat-node message assistant".to_owned()
    };
    let role = if is_user { "user" } else { "assistant" };
    let action_context = serde_json::json!({"index": index, "role": role}).to_string();
    let action_occurrence = index.to_string();
    // Whether a plugin renders this node kind instead of the built-in view.
    // Only `slots` is tracked, so streamed text never remounts the slot frame.
    // The core kinds are ordinary entries of the same keyed cell at
    // `CORE_RENDERER_PRIORITY`, so a plugin that opts in with a lower priority
    // (the reference `dsh-genui` uses `priority: -1`) shadows them; a
    // registration at the default priority keeps the built-in view.
    let kind_renderer = kind.clone();
    let occurrence_renderer = occurrence.clone();
    let renderer = move || {
        chat_node_renderer(
            &ui.slots.get(),
            &kind_renderer,
            &occurrence_renderer,
            &ui.renderer_errors.get(),
        )
    };
    let replaced = move || matches!(renderer(), ChatNodeRenderer::Plugin { .. });
    let kind_state = kind.clone();
    let occurrence_state = occurrence.clone();
    let state_attr = move || match chat_node_renderer(
        &ui.slots.get(),
        &kind_state,
        &occurrence_state,
        &ui.renderer_errors.get(),
    ) {
        ChatNodeRenderer::Builtin => "builtin".to_owned(),
        ChatNodeRenderer::Plugin { .. } => "plugin".to_owned(),
    };
    let kind_token = kind.clone();
    let occurrence_token = occurrence.clone();
    let token_attr = move || {
        match chat_node_renderer(
            &ui.slots.get(),
            &kind_token,
            &occurrence_token,
            &ui.renderer_errors.get(),
        ) {
            ChatNodeRenderer::Builtin => String::new(),
            ChatNodeRenderer::Plugin { token } => token,
        }
    };
    let entry_key = kind.clone();
    let owner_slot = owner.clone();
    let context_slot = context.clone();
    let occurrence_slot = occurrence.clone();
    let node_key = key.clone();
    let node_kind = kind.clone();
    view! {
        <div
            class=class
            data-chat-flow-key=node_key
            data-chat-flow-kind=node_kind
            data-turn=turn.to_string()
            data-renderer-state=state_attr
            data-renderer-token=token_attr
        >
            {move || {
                if replaced() {
                    view! {
                        <SlotRegion
                            region="chat.node"
                            entry_key=entry_key.clone()
                            occurrence=occurrence_slot.clone()
                            context=context_slot.clone()
                            owner_props=owner_slot.clone()
                        />
                    }
                        .into_any()
                } else {
                    let text = move || {
                        ui.nodes
                            .get()
                            .get(index)
                            .map(|node| node.text.clone())
                            .unwrap_or_default()
                    };
                    if is_user {
                        view! { <div class="bubble">{text}</div> }.into_any()
                    } else {
                        view! {
                            <div class="assistant-text">{text}</div>
                            <ToolCallList index=index />
                        }
                            .into_any()
                    }
                }
            }}
            {if is_user {
                view! {
                    <SlotRegion
                        region="chat.user.before"
                        occurrence=occurrence.clone()
                        context=context.clone()
                        owner_props=user_owner.clone()
                    />
                    <SlotRegion
                        region="message.action"
                        occurrence=action_occurrence.clone()
                        context=action_context.clone()
                        owner_props=user_owner.clone()
                    />
                    <ChatImages
                        images=images.clone()
                        region="chat.user.images"
                        occurrence=occurrence.clone()
                        context=context.clone()
                        owner_props=owner.clone()
                    />
                }
                    .into_any()
            } else {
                view! {
                    <ChatReasoning
                        index=index
                        occurrence=occurrence.clone()
                        context=context.clone()
                        owner_props=owner.clone()
                    />
                    <ChatImages
                        images=images.clone()
                        region="chat.images"
                        occurrence=occurrence.clone()
                        context=context.clone()
                        owner_props=owner.clone()
                    />
                    <SlotRegion
                        region="chat.text-interaction"
                        occurrence=occurrence.clone()
                        context=context.clone()
                        owner_props=owner.clone()
                    />
                    <SlotRegion
                        region="message.action"
                        occurrence=action_occurrence.clone()
                        context=action_context.clone()
                    />
                }
                    .into_any()
            }}
        </div>
    }
}

/// Turn tail row: the `conversation.chat.turnTail` seat plus the assistant
/// actions seat, mounted once per finished turn (reference `TurnTailNodeView`).
#[component]
fn ChatTurnTail(turn: usize, seq: usize, message_id: String) -> impl IntoView {
    let owner = serde_json::json!({
        "turn": turn,
        "seq": seq,
        "openFile": {"$callback": "openFile"},
    })
    .to_string();
    let actions_owner = serde_json::json!({"messageId": message_id}).to_string();
    let context = serde_json::json!({"turn": turn, "seq": seq}).to_string();
    view! {
        <div class="turn-tail" data-turn-tail=turn.to_string()>
            <SlotRegion
                region="chat.turnTail"
                occurrence=format!("tail-{turn}")
                context=context.clone()
                owner_props=owner
            />
            <SlotRegion
                region="chat.assistant-actions"
                occurrence=format!("actions-{turn}")
                context=context
                owner_props=actions_owner
            />
        </div>
    }
}

#[component]
fn App() -> impl IntoView {
    let token = RwSignal::new(initial_token());
    let status = RwSignal::new(String::from("offline"));
    let sessions = RwSignal::new(Vec::<SessionSummary>::new());
    let nodes = RwSignal::new(Vec::<ChatNode>::new());
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
    // Nested-slot anchors and chain election state live in the shell context so
    // SlotFrame/ChainHost components and the window message handler share them.
    let anchors = RwSignal::new(BTreeMap::<String, Vec<SlotAnchor>>::new());
    let chains = RwSignal::new(BTreeMap::<String, ChainInstance>::new());
    let renderer_errors = RwSignal::new(BTreeMap::<String, String>::new());
    let nested = RwSignal::new(BTreeMap::<String, (String, String)>::new());
    // Owner-callback surfaces: `openFile` opens a file inspector, `inspectCall`
    // opens the tool call that a plugin asked about.
    let file_preview = RwSignal::new(None::<FilePreview>);
    let call_inspector = RwSignal::new(None::<Value>);
    let open_file = move |path: String| {
        if path.trim().is_empty() {
            status.set("openFile: empty path".into());
            return;
        }
        let token_value = token.get_untracked();
        let requested = path.clone();
        file_preview.set(Some(FilePreview {
            path: path.clone(),
            content: String::new(),
            error: String::new(),
            loading: true,
        }));
        spawn_local(async move {
            let route = format!("/v1/workspace/file?path={}", encode(&requested));
            match get_json::<Value>(&route, &token_value).await {
                Ok(value) => {
                    let content = value["content"].as_str().unwrap_or_default().to_owned();
                    let truncated = value["truncated"].as_bool().unwrap_or(false);
                    file_preview.set(Some(FilePreview {
                        path: value["path"]
                            .as_str()
                            .map(str::to_owned)
                            .unwrap_or(requested.clone()),
                        content,
                        error: if truncated {
                            "truncated to the preview limit".to_owned()
                        } else {
                            String::new()
                        },
                        loading: false,
                    }));
                }
                Err(error) => {
                    file_preview.set(Some(FilePreview {
                        path: requested,
                        content: String::new(),
                        error,
                        loading: false,
                    }));
                }
            }
        });
    };
    let inspect_call = move |call_id: String| {
        let found = nodes
            .get_untracked()
            .iter()
            .flat_map(|node| node.calls.iter())
            .find(|call| call.id == call_id)
            .map(|call| {
                serde_json::json!({
                    "id": call.id,
                    "name": call.name,
                    "input": call.input,
                    "ok": call.ok,
                    "result": call.result,
                })
            });
        match found {
            Some(value) => call_inspector.set(Some(value)),
            None => status.set(format!("inspectCall: unknown call '{call_id}'")),
        }
    };

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
        anchors,
        chains,
        renderer_errors,
        nested,
        nodes,
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
        nodes.set(Vec::new());
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
                    nodes.set(build_chat_nodes(&data.messages));
                    status.set(id_for_load.clone());
                }
                Err(error) => status.set(format!("error: {error}")),
            }
        });
        connect_events(id, value, nodes, streaming, busy, status, approval, question);
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
                    nodes.set(Vec::new());
                    streaming.set(false);
                    approval.set(None);
                    current.set(Some(id.clone()));
                    broadcast_host_state();
                    connect_events(
                        id,
                        value.clone(),
                        nodes,
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
        nodes.update(|items| {
            let turn = items
                .iter()
                .filter(|node| node.kind == "user")
                .count()
                + 1;
            let seq = items.len();
            items.push(ChatNode {
                key: format!("{turn}:{seq}"),
                kind: "user".into(),
                turn,
                status: "settled".into(),
                text: value.clone(),
                reasoning: String::new(),
                images: Vec::new(),
                calls: Vec::new(),
                seq,
                message_id: format!("m{seq}"),
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
                    // A reloaded document replaces its own anchors.
                    anchors.update(|map| {
                        map.remove(&frame);
                    });
                    // Large owner shares travel out of band, after ready.
                    if let Some(element) = frame_for_token(&frame)
                        && let Some(owner) = element.get_attribute("data-owner-props")
                        && !owner.is_empty()
                    {
                        post_to_token(&frame, "owner-props", &[("ownerProps", owner.as_str())]);
                    }
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
                "anchor" => {
                    // A parent frame reported a `renderSlot` placeholder: the
                    // declared child slot mounts over this box (hoc nesting).
                    if let Some(anchor) = read_anchor(&data) {
                        anchors.update(|map| {
                            let list = map.entry(frame.clone()).or_default();
                            match list.iter_mut().find(|item| item.id == anchor.id) {
                                Some(existing) => *existing = anchor,
                                None => list.push(anchor),
                            }
                        });
                    }
                }
                "anchor-clear" => {
                    let id = field_of(&data, "anchor");
                    anchors.update(|map| {
                        if let Some(list) = map.get_mut(&frame) {
                            list.retain(|item| item.id != id);
                        }
                    });
                }
                "chain-unselected" => {
                    // The mounted candidate declined: advance the single owner
                    // renderer to the next registration (reference election).
                    let token_now = frame.clone();
                    chains.update(|map| {
                        for state in map.values_mut() {
                            if state.token != token_now || state.winner {
                                continue;
                            }
                            state.index += 1;
                            state.token = if state.index < state.candidates.len() {
                                slot_token(&state.candidates[state.index], &state.occurrence)
                            } else {
                                String::new()
                            };
                            break;
                        }
                    });
                }
                "resize" => {
                    let height = number_of(&data, "height");
                    apply_frame_height(&frame, height);
                    // A nested child frame reports its height back to the parent
                    // so the anchor placeholder reserves the space it occupies.
                    if let Some((parent, anchor)) = nested.get_untracked().get(&frame).cloned() {
                        let height = format!("{height}");
                        post_to_token(
                            &parent,
                            "anchor-size",
                            &[("anchor", anchor.as_str()), ("height", height.as_str())],
                        );
                    }
                    // The chain candidate rendered: it owns the slot.
                    let token_now = frame.clone();
                    chains.update(|map| {
                        for state in map.values_mut() {
                            if state.token == token_now && !state.winner {
                                state.winner = true;
                                break;
                            }
                        }
                    });
                }
                "error" => {
                    let message = field_of(&data, "message");
                    let message = if message.is_empty() {
                        "plugin error".to_owned()
                    } else {
                        message
                    };
                    // A crashed renderer must not leave an empty keyed cell: the
                    // resolver below falls back to the built-in view (C5.2).
                    renderer_errors.update(|map| {
                        map.insert(frame.clone(), message.clone());
                    });
                    // A panel-stage error belongs to the stage head; errors from
                    // a sidebar/settings/message frame surface in the status bar.
                    if frame.starts_with("slot:panel:") {
                        panel_error.set(message);
                    } else {
                        status.set(format!("{plugin}: {message}"));
                    }
                    // A broken chain candidate must not block the election.
                    let token_now = frame.clone();
                    chains.update(|map| {
                        for state in map.values_mut() {
                            if state.token != token_now || state.winner {
                                continue;
                            }
                            state.index += 1;
                            state.token = if state.index < state.candidates.len() {
                                slot_token(&state.candidates[state.index], &state.occurrence)
                            } else {
                                String::new()
                            };
                            break;
                        }
                    });
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
                        "callback" => {
                            // Owner props carrying functions: the shell owns the
                            // implementation, the frame only had a marker.
                            let name = field_of(&payload, "name");
                            let first = js_sys::Reflect::get(&payload, &"args".into())
                                .ok()
                                .and_then(|args| {
                                    let list = js_sys::Array::from(&args);
                                    (list.length() > 0).then(|| list.get(0))
                                })
                                .and_then(|value| value.as_string())
                                .unwrap_or_default();
                            match name.as_str() {
                                "settings.section.close" => {
                                    settings_open.set(false);
                                    post_action_result(source, &id, true, "");
                                }
                                "openFile" => {
                                    open_file(first);
                                    post_action_result(source, &id, true, "");
                                }
                                "inspectCall" => {
                                    inspect_call(first);
                                    post_action_result(source, &id, true, "");
                                }
                                "forkAt" => {
                                    status.set(format!("fork point {first}"));
                                    post_action_result(source, &id, true, "");
                                }
                                "loadImage" => {
                                    post_action_result(source, &id, true, "");
                                }
                                other => post_action_result(
                                    source,
                                    &id,
                                    false,
                                    &format!("unsupported owner callback '{other}'"),
                                ),
                            }
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

    // Chat rows are keyed by (session, node key) and read their content from the
    // node signal: streaming must not remount a row, because that would reload
    // the slot frames mounted under it.
    let node_rows = move || {
        let session = current.get().unwrap_or_default();
        let list = nodes.get();
        let mut rows = Vec::with_capacity(list.len());
        for (index, node) in list.iter().enumerate() {
            let tail = list
                .get(index + 1)
                .map(|next| next.turn != node.turn)
                .unwrap_or(true);
            rows.push((
                session.clone(),
                index,
                node.key.clone(),
                node.turn,
                node.seq,
                node.message_id.clone(),
                tail,
            ));
        }
        rows
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
                                    owner_props="{\"wide\":true}".to_string()
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
                    <SlotRegion
                        region="chat.flow"
                        occurrence="flow".to_string()
                        context="{}".to_string()
                    />
                    <For
                        each=node_rows
                        key=|row| (row.0.clone(), row.2.clone())
                        children=move |row| {
                            let (_, index, _, turn, seq, message_id, tail) = row;
                            view! {
                                <ChatNodeRow index=index />
                                {tail.then(|| view! {
                                    <ChatTurnTail turn=turn seq=seq message_id=message_id />
                                })}
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
                        {move || {
                            let session_id = current.get();
                            let session = sessions.get().into_iter().find(|summary| {
                                Some(&summary.id) == session_id.as_ref()
                            });
                            let owner = serde_json::json!({
                                "sessionId": session_id,
                                "session": session.map(|summary| serde_json::json!({
                                    "id": summary.id,
                                    "model": summary.model,
                                    "turns": summary.turns,
                                })),
                                "pendingInteraction": null,
                            })
                            .to_string();
                            view! {
                                <SlotRegion region="composer.actions" owner_props=owner.clone() />
                                <SlotRegion region="composer.chain" owner_props=owner />
                            }
                        }}
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
                <SlotRegion
                    region="settings.section"
                    owner_props="{\"close\":{\"$callback\":\"settings.section.close\"}}".to_string()
                />
                <p class="tool">{move || settings_status.get()}</p>
                <div class="row">
                    <button on:click=move |_| save_settings()>"Save"</button>
                    <button on:click=move |_| settings_open.set(false)>"Close"</button>
                </div>
                <SlotRegion region="settings.action" />
                <p>"Ключи не вводятся в UI: они читаются из окружения (DEEPSEEK_API_KEY / OPENAI_API_KEY) или desktop.env."</p>
            </dialog>
        })}
        {move || file_preview.get().map(|preview| view! {
            <dialog open class="inspector" data-file=preview.path.clone()>
                <h3>{format!("File · {}", preview.path)}</h3>
                <div class="row">
                    <button on:click=move |_| file_preview.set(None)>"Close"</button>
                    {(!preview.error.is_empty()).then(|| view! { <span class="tool">{preview.error.clone()}</span> })}
                    {preview.loading.then(|| view! { <span class="tool">"loading…"</span> })}
                </div>
                <pre class="file-content">{preview.content.clone()}</pre>
            </dialog>
        })}
        {move || call_inspector.get().map(|value| {
            let call_id = value["id"].as_str().unwrap_or_default().to_owned();
            let pretty = serde_json::to_string_pretty(&value).unwrap_or_default();
            view! {
                <dialog open class="inspector" data-call=call_id>
                    <h3>"Tool call"</h3>
                    <pre class="call-json">{pretty}</pre>
                    <div class="row">
                        <button on:click=move |_| call_inspector.set(None)>"Close"</button>
                    </div>
                </dialog>
            }
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

#[cfg(test)]
mod renderer_tests {
    use super::*;
    use std::collections::BTreeMap;

    fn node_entry(kind: &str, key: &str, priority: i64) -> ClientSlot {
        ClientSlot {
            plugin: "fixture".to_owned(),
            name: "conversation.chat.node".to_owned(),
            id: format!("{kind}:{key}:{priority}"),
            title: String::new(),
            order: 0,
            region: "chat.node".to_owned(),
            kind: kind.to_owned(),
            priority,
            key: key.to_owned(),
            children: Vec::new(),
        }
    }

    fn errors(tokens: &[&str]) -> BTreeMap<String, String> {
        tokens
            .iter()
            .map(|token| ((*token).to_owned(), "boom".to_owned()))
            .collect()
    }

    fn is_plugin(renderer: &ChatNodeRenderer) -> bool {
        matches!(renderer, ChatNodeRenderer::Plugin { .. })
    }

    #[test]
    fn core_kinds_keep_the_builtin_renderer_without_a_registration() {
        let slots: Vec<ClientSlot> = Vec::new();
        assert_eq!(
            chat_node_renderer(&slots, "assistant-step", "1:0", &BTreeMap::new()),
            ChatNodeRenderer::Builtin
        );
        assert_eq!(
            chat_node_renderer(&slots, "user", "1:0", &BTreeMap::new()),
            ChatNodeRenderer::Builtin
        );
    }

    #[test]
    fn a_default_priority_registration_does_not_take_the_builtin() {
        let slots = vec![node_entry("keyed", "assistant-step", 0)];
        assert!(!is_plugin(&chat_node_renderer(
            &slots,
            "assistant-step",
            "1:0",
            &BTreeMap::new()
        )));
    }

    #[test]
    fn a_negative_priority_shadows_a_core_kind() {
        let slots = vec![node_entry("keyed", "assistant-step", -1)];
        let renderer = chat_node_renderer(&slots, "assistant-step", "1:0", &BTreeMap::new());
        assert!(is_plugin(&renderer));
        if let ChatNodeRenderer::Plugin { token } = renderer {
            assert!(token.starts_with("slot:chat.node:"), "{token}");
        }
        let user = vec![node_entry("keyed", "user", -2)];
        assert!(is_plugin(&chat_node_renderer(
            &user,
            "user",
            "1:0",
            &BTreeMap::new()
        )));
    }

    #[test]
    fn a_shadowed_other_key_does_not_replace_this_node() {
        let slots = vec![node_entry("keyed", "reasoning", -1)];
        assert!(!is_plugin(&chat_node_renderer(
            &slots,
            "assistant-step",
            "1:0",
            &BTreeMap::new()
        )));
    }

    #[test]
    fn plugin_specific_kinds_render_their_keyed_registration() {
        let slots = vec![node_entry("keyed", "custom-card", 0)];
        assert!(is_plugin(&chat_node_renderer(
            &slots,
            "custom-card",
            "1:0",
            &BTreeMap::new()
        )));
        assert!(!is_plugin(&chat_node_renderer(
            &slots,
            "another-card",
            "1:0",
            &BTreeMap::new()
        )));
    }

    #[test]
    fn non_keyed_registrations_never_replace_a_node() {
        let slots = vec![node_entry("single", "assistant-step", -1)];
        assert!(!is_plugin(&chat_node_renderer(
            &slots,
            "assistant-step",
            "1:0",
            &BTreeMap::new()
        )));
        let list = vec![node_entry("list", "custom-card", -1)];
        assert!(!is_plugin(&chat_node_renderer(
            &list,
            "custom-card",
            "1:0",
            &BTreeMap::new()
        )));
    }

    #[test]
    fn a_crashed_winner_falls_back_to_the_builtin_view() {
        let slots = vec![node_entry("keyed", "assistant-step", -1)];
        let winner = chat_node_renderer(&slots, "assistant-step", "1:0", &BTreeMap::new());
        let ChatNodeRenderer::Plugin { token } = winner else {
            panic!("expected a plugin renderer");
        };
        assert_eq!(
            chat_node_renderer(&slots, "assistant-step", "1:0", &errors(&[token.as_str()])),
            ChatNodeRenderer::Builtin
        );
    }

    #[test]
    fn an_unrelated_crash_does_not_remove_the_winner() {
        let slots = vec![node_entry("keyed", "assistant-step", -1)];
        assert!(is_plugin(&chat_node_renderer(
            &slots,
            "assistant-step",
            "1:0",
            &errors(&["slot:chat.node:other:1"])
        )));
    }
}

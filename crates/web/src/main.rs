use std::cell::RefCell;

use gloo_net::http::Request;
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
    version: String,
    description: String,
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
struct InstalledResponse {
    plugins: Vec<InstalledPlugin>,
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
}

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(App);
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
                messages.update(|items| {
                    items.push(ChatItem {
                        role: "tool".into(),
                        text: if summary.is_empty() {
                            format!("→ {name}")
                        } else {
                            format!("→ {name} · {summary}")
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
    let plugin_status = RwSignal::new(String::new());
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
                    status.set(format!("session {}", data.session.id));
                    // The next refresh makes the sidebar show it.
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
                    plugin_status.set(format!("installed {name} — restart dshr to load it"));
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
                        plugin_status.set("done — restart dshr to apply".into());
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

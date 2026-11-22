#![forbid(unsafe_code)]
//! Minimal localization for terminal UI strings.

use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Lang {
    #[default]
    Ru,
    En,
}

static CURRENT: OnceLock<Lang> = OnceLock::new();

pub fn set_current(lang: Lang) {
    let _ = CURRENT.set(lang);
}

#[must_use]
pub fn current() -> Lang {
    *CURRENT.get().unwrap_or(&Lang::En)
}

/// Human-readable labels for Markdown session export.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExportLabels {
    pub session: &'static str,
    pub created: &'static str,
    pub provider: &'static str,
    pub model: &'static str,
    pub cwd: &'static str,
    pub user: &'static str,
    pub assistant: &'static str,
    pub thinking: &'static str,
    pub result: &'static str,
    pub image: &'static str,
    pub audio: &'static str,
}

/// Human-readable labels for the web Models settings tab.
///
/// Only display text lives here: wire keys and enum values (`openai-chat`,
/// `EMPTY_RESPONSE`, `*_API_KEY`) stay in the component, because the browser
/// posts them back unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModelsLabels {
    pub err_empty_response: &'static str,
    pub err_rate_limited: &'static str,
    pub err_server_error: &'static str,
    pub err_timeout: &'static str,
    pub err_transport: &'static str,
    pub err_stream_closed: &'static str,
    pub err_malformed_response: &'static str,
    pub err_quota: &'static str,
    pub choose_retry_mode: &'static str,
    pub retry_initial_delay: &'static str,
    pub retry_maximum_delay: &'static str,
    pub retry_delays_positive: &'static str,
    pub retry_initial_exceeds_max: &'static str,
    pub retry_jitter_not_number: &'static str,
    pub retry_jitter_out_of_range: &'static str,
    pub select_retryable_code: &'static str,
    pub maximum_retries: &'static str,
    pub selection_empties_list: &'static str,
    pub base_url_scheme: &'static str,
    pub api_key_upper_snake: &'static str,
    pub every_model_needs_id: &'static str,
    pub provider_id_format: &'static str,
    pub custom_needs_base_url: &'static str,
    pub custom_needs_model: &'static str,
    pub no_active_provider: &'static str,
    pub provider_default: &'static str,
    pub applies_after_restart: &'static str,
    pub loading: &'static str,
    pub custom_list: &'static str,
    pub edit: &'static str,
    pub use_route: &'static str,
    pub delete: &'static str,
    pub new_custom_provider: &'static str,
    pub cancel: &'static str,
    pub provider_id: &'static str,
    pub fixed_after_creation: &'static str,
    pub display_name: &'static str,
    pub optional: &'static str,
    pub protocol: &'static str,
    pub base_url: &'static str,
    pub api_key_variable: &'static str,
    pub value_lives_in_credentials: &'static str,
    pub not_configured: &'static str,
    pub api_key: &'static str,
    pub hide: &'static str,
    pub show: &'static str,
    pub remove_saved_key: &'static str,
    pub retry_policy: &'static str,
    pub mode: &'static str,
    pub inherited_adapter: &'static str,
    pub transient_only: &'static str,
    pub always_retry: &'static str,
    pub max_retries: &'static str,
    pub initial_delay_ms: &'static str,
    pub max_delay_ms: &'static str,
    pub jitter_ratio: &'static str,
    pub models: &'static str,
    pub fetching: &'static str,
    pub fetch_available: &'static str,
    pub custom_needs_one_model: &'static str,
    pub empty_list_inherits: &'static str,
    pub filter_models: &'static str,
    pub select_all: &'static str,
    pub deselect_all: &'static str,
    pub update_list: &'static str,
    pub model_id: &'static str,
    pub context: &'static str,
    pub max_out: &'static str,
    pub make_active: &'static str,
    pub remove_row: &'static str,
    pub reasoning_levels: &'static str,
    pub api_value: &'static str,
    pub add_level: &'static str,
    pub add_model: &'static str,
    pub restore_catalog: &'static str,
    pub save: &'static str,
    pub catalog_provider: &'static str,
    pub custom_model_api: &'static str,
    pub choose_catalog_provider: &'static str,
    pub add: &'static str,
    pub working: &'static str,
    pub welcome_title: &'static str,
    pub welcome_intro: &'static str,
    pub welcome_no_key: &'static str,
    pub deepseek_api_key: &'static str,
    pub stored_in_credentials: &'static str,
    pub continue_label: &'static str,
    pub delete_provider: &'static str,
    pub delete_model: &'static str,
    pub key_configured: &'static str,
    pub no_key_in_environment: &'static str,
    pub no_key_for_route: &'static str,
    pub anthropic_messages: &'static str,
    pub openai_chat_completions: &'static str,
    pub custom_protocol: &'static str,
    pub no_models_yet: &'static str,
    pub context_window: &'static str,
    pub max_output_tokens: &'static str,
    pub enter_base_url: &'static str,
    pub at_least_one_level: &'static str,
    pub no_saved_key: &'static str,
    pub active_tag: &'static str,
    pub defaults_tag: &'static str,
}

impl Lang {
    #[must_use]
    pub fn parse(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "en" | "en-us" | "english" => Self::En,
            _ => Self::Ru,
        }
    }

    #[must_use]
    pub fn hint(self) -> &'static str {
        match self {
            Self::Ru => "Enter — отправить, Esc — отменить, Ctrl-C — выход.",
            Self::En => "Enter — send, Esc — cancel, Ctrl-C — quit.",
        }
    }

    #[must_use]
    pub fn is_ru(self) -> bool {
        matches!(self, Self::Ru)
    }

    /// Wire value of the language, matching `ui.language` in `config.yaml`.
    #[must_use]
    pub fn code(self) -> &'static str {
        match self {
            Self::Ru => "ru",
            Self::En => "en",
        }
    }

    #[must_use]
    pub fn col_id(self) -> &'static str {
        "ID"
    }

    #[must_use]
    pub fn col_created(self) -> &'static str {
        if self.is_ru() {
            "СОЗДАНА"
        } else {
            "CREATED"
        }
    }

    #[must_use]
    pub fn col_model(self) -> &'static str {
        if self.is_ru() {
            "МОДЕЛЬ"
        } else {
            "MODEL"
        }
    }

    #[must_use]
    pub fn col_turns(self) -> &'static str {
        if self.is_ru() { "ХОДЫ" } else { "TURNS" }
    }

    #[must_use]
    pub fn sessions_header(self) -> &'static str {
        match self {
            Self::Ru => {
                "ID                                       СОЗДАНА                    МОДЕЛЬ               ХОДЫ"
            }
            Self::En => {
                "ID                                       CREATED                    MODEL                TURNS"
            }
        }
    }

    #[must_use]
    pub fn no_sessions(self) -> &'static str {
        match self {
            Self::Ru => "(сессий нет)",
            Self::En => "(no sessions)",
        }
    }

    #[must_use]
    pub fn no_hits(self) -> &'static str {
        match self {
            Self::Ru => "(совпадений нет)",
            Self::En => "(no hits)",
        }
    }

    #[must_use]
    pub fn indexed(self, count: usize) -> String {
        match self {
            Self::Ru => format!("проиндексировано {count} событий"),
            Self::En => format!("indexed {count} events"),
        }
    }

    #[must_use]
    pub fn session_created(self, id: &str) -> String {
        match self {
            Self::Ru => format!("создана сессия {id}"),
            Self::En => format!("created session {id}"),
        }
    }

    #[must_use]
    pub fn no_plugins(self) -> &'static str {
        match self {
            Self::Ru => "(плагинов нет)",
            Self::En => "(no plugins)",
        }
    }

    #[must_use]
    pub fn plugin_added(self, name: &str) -> String {
        match self {
            Self::Ru => format!("плагин добавлен: {name}"),
            Self::En => format!("added plugin {name}"),
        }
    }

    #[must_use]
    pub fn plugin_removed(self, name: &str) -> String {
        match self {
            Self::Ru => format!("плагин удалён: {name}"),
            Self::En => format!("removed plugin {name}"),
        }
    }

    #[must_use]
    pub fn plugin_key_written(self, path: &str) -> String {
        match self {
            Self::Ru => format!("секретный ключ записан: {path}"),
            Self::En => format!("wrote secret key: {path}"),
        }
    }

    #[must_use]
    pub fn plugin_public_key(self, key: &str) -> String {
        match self {
            Self::Ru => format!("публичный ключ: {key}"),
            Self::En => format!("public key: {key}"),
        }
    }

    #[must_use]
    pub fn plugin_signed(self, manifest: &str) -> String {
        match self {
            Self::Ru => format!("манифест подписан: {manifest}"),
            Self::En => format!("signed manifest: {manifest}"),
        }
    }

    #[must_use]
    pub fn role_user(self) -> &'static str {
        match self {
            Self::Ru => "Пользователь",
            Self::En => "User",
        }
    }

    #[must_use]
    pub fn role_assistant(self) -> &'static str {
        match self {
            Self::Ru => "Ассистент",
            Self::En => "Assistant",
        }
    }

    #[must_use]
    pub fn untitled(self) -> &'static str {
        match self {
            Self::Ru => "(без названия)",
            Self::En => "(untitled)",
        }
    }

    #[must_use]
    pub fn sessions_rewound(self, id: &str, turns: usize, events: usize) -> String {
        match self {
            Self::Ru => format!("{id}: {turns} ход(ов), {events} событ(ий)"),
            Self::En => format!("{id}: {turns} turn(s), {events} event(s)"),
        }
    }

    #[must_use]
    pub fn session_title(self, id: &str, title: &str) -> String {
        match self {
            Self::Ru => format!("{id}: {title}"),
            Self::En => format!("{id}: {title}"),
        }
    }

    #[must_use]
    pub fn release_signed(self, path: &str) -> String {
        match self {
            Self::Ru => format!("подпись записана: {path}"),
            Self::En => format!("wrote signature: {path}"),
        }
    }

    #[must_use]
    pub fn release_verified(self, path: &str) -> String {
        match self {
            Self::Ru => format!("подпись верна: {path}"),
            Self::En => format!("signature valid: {path}"),
        }
    }

    #[must_use]
    pub fn plugin_verified(self, manifest: &str) -> String {
        match self {
            Self::Ru => format!("подпись верна: {manifest}"),
            Self::En => format!("signature valid: {manifest}"),
        }
    }

    #[must_use]
    pub fn no_schedules(self) -> &'static str {
        match self {
            Self::Ru => "(расписаний нет)",
            Self::En => "(no schedules)",
        }
    }

    #[must_use]
    pub fn enabled(self) -> &'static str {
        match self {
            Self::Ru => "включено",
            Self::En => "enabled",
        }
    }

    #[must_use]
    pub fn disabled(self) -> &'static str {
        match self {
            Self::Ru => "выключено",
            Self::En => "disabled",
        }
    }

    #[must_use]
    pub fn schedule_added(self, id: &str, next: Option<&str>) -> String {
        match self {
            Self::Ru => format!("добавлено: {id} next={}", next.unwrap_or("-")),
            Self::En => format!("added {id} next={}", next.unwrap_or("-")),
        }
    }

    #[must_use]
    pub fn schedule_removed(self, id: &str) -> String {
        match self {
            Self::Ru => format!("удалено: {id}"),
            Self::En => format!("removed {id}"),
        }
    }

    #[must_use]
    pub fn schedule_state(self, id: &str, enabled: bool) -> String {
        format!(
            "{} {id}",
            if enabled {
                self.enabled()
            } else {
                self.disabled()
            }
        )
    }

    #[must_use]
    pub fn no_due(self, at: &str) -> String {
        match self {
            Self::Ru => format!("нет запусков на {at}"),
            Self::En => format!("no due schedules at {at}"),
        }
    }

    #[must_use]
    pub fn schedule_ran(self, id: &str, text: &str) -> String {
        match self {
            Self::Ru => format!("выполнено {id}: {text}"),
            Self::En => format!("ran {id}: {text}"),
        }
    }

    #[must_use]
    pub fn schedule_failed(self, id: &str, error: &str) -> String {
        match self {
            Self::Ru => format!("ошибка {id}: {error}"),
            Self::En => format!("failed {id}: {error}"),
        }
    }

    #[must_use]
    pub fn schedule_next(self, id: &str, at: &str) -> String {
        match self {
            Self::Ru => format!("следующий {id} в {at}"),
            Self::En => format!("next {id} at {at}"),
        }
    }

    #[must_use]
    pub fn run_summary(
        self,
        session: &str,
        input: u64,
        output: u64,
        steps: usize,
        tools: usize,
    ) -> String {
        match self {
            Self::Ru => format!(
                "сессия {session} · {input} вход / {output} выход · {steps} шаг(ов), {tools} вызов(ов)"
            ),
            Self::En => format!(
                "session {session} · {input} in / {output} out · {steps} step(s), {tools} tool call(s)"
            ),
        }
    }

    #[must_use]
    pub fn cancelled(self, session: &str) -> String {
        match self {
            Self::Ru => format!("отменено; сессия {session}"),
            Self::En => format!("cancelled; session {session}"),
        }
    }

    #[must_use]
    pub fn server_listening(self, address: &str) -> String {
        match self {
            Self::Ru => format!("dshr server слушает http://{address}"),
            Self::En => format!("dshr server listening on http://{address}"),
        }
    }

    #[must_use]
    pub fn present(self, value: bool) -> &'static str {
        match (self, value) {
            (Self::Ru, true) => "есть",
            (Self::Ru, false) => "нет",
            (Self::En, true) => "present",
            (Self::En, false) => "absent",
        }
    }

    #[must_use]
    pub fn approval(self, tool: &str, summary: &str, reason: &str) -> String {
        match self {
            Self::Ru => format!("Разрешить {tool}?\n\n{summary}\n\n{reason}"),
            Self::En => format!("Allow {tool}?\n\n{summary}\n\n{reason}"),
        }
    }

    #[must_use]
    pub fn thinking(self) -> &'static str {
        match self {
            Self::Ru => "думаю…",
            Self::En => "thinking…",
        }
    }

    #[must_use]
    pub fn done(self, input: u64, output: u64, steps: usize, tools: usize) -> String {
        match self {
            Self::Ru => format!(
                "[готово: {input} вход / {output} выход, {steps} шаг(ов), {tools} вызов(ов)]"
            ),
            Self::En => {
                format!("[done: {input} in / {output} out, {steps} step(s), {tools} tool call(s)]")
            }
        }
    }

    #[must_use]
    pub fn export_labels(self) -> ExportLabels {
        match self {
            Self::Ru => ExportLabels {
                session: "Сессия",
                created: "создана",
                provider: "провайдер",
                model: "модель",
                cwd: "каталог",
                user: self.role_user(),
                assistant: self.role_assistant(),
                thinking: "размышления",
                result: "результат",
                image: "изображение",
                audio: "аудио",
            },
            Self::En => ExportLabels {
                session: "Session",
                created: "created",
                provider: "provider",
                model: "model",
                cwd: "cwd",
                user: self.role_user(),
                assistant: self.role_assistant(),
                thinking: "thinking",
                result: "result",
                image: "image",
                audio: "audio",
            },
        }
    }

    /// Interpolated Models-UI messages. They return `String` because the
    /// `ModelsLabels` fields are `&'static str` and cannot hold `format!` output.
    #[must_use]
    pub fn models_saved_route(self, route: &str) -> String {
        match self {
            Self::Ru => format!("сохранено {route} · перезапустите dshr serve, чтобы применить"),
            Self::En => format!("saved {route} · restart dshr serve to apply"),
        }
    }

    #[must_use]
    pub fn models_route_removed(self, route: &str) -> String {
        match self {
            Self::Ru => format!("удалено {route} · снова действуют значения каталога"),
            Self::En => format!("removed {route} · catalog defaults apply again"),
        }
    }

    #[must_use]
    pub fn models_active_route_line(self, provider: &str, model: &str, note: &str) -> String {
        match self {
            Self::Ru => format!("Активный маршрут: {provider} · {model} — {note}"),
            Self::En => format!("Active route: {provider} · {model} — {note}"),
        }
    }

    #[must_use]
    pub fn models_active_route(self, provider: &str, model: &str, effort: &str) -> String {
        match self {
            Self::Ru => format!(
                "активный маршрут: {provider} · {model}{effort} — перезапустите dshr serve, чтобы применить"
            ),
            Self::En => {
                format!("active route: {provider} · {model}{effort} — restart dshr serve to apply")
            }
        }
    }

    #[must_use]
    pub fn models_reasoning_suffix(self, level: &str) -> String {
        match self {
            Self::Ru => format!(" · рассуждения {level}"),
            Self::En => format!(" · reasoning {level}"),
        }
    }

    #[must_use]
    pub fn models_effort_heading(self, label: &str) -> String {
        match self {
            Self::Ru => format!("Усилие рассуждения · {label}"),
            Self::En => format!("Reasoning effort · {label}"),
        }
    }

    #[must_use]
    pub fn models_model_count(self, count: usize) -> String {
        match self {
            Self::Ru => format!(" · моделей: {count}"),
            Self::En => format!(" · {count} model(s)"),
        }
    }

    #[must_use]
    pub fn models_available_count(self, count: usize) -> String {
        match self {
            Self::Ru => format!("доступных моделей: {count}"),
            Self::En => format!("{count} available model(s)"),
        }
    }

    #[must_use]
    pub fn models_whole_number(self, label: &str) -> String {
        match self {
            Self::Ru => format!("{label}: нужно целое число"),
            Self::En => format!("{label} must be a whole number"),
        }
    }

    #[must_use]
    pub fn models_greater_than_zero(self, label: &str) -> String {
        match self {
            Self::Ru => format!("{label}: значение должно быть больше нуля"),
            Self::En => format!("the {label} must be greater than zero"),
        }
    }

    #[must_use]
    pub fn models_snapshot_error(self, problem: &str) -> String {
        match self {
            Self::Ru => format!("не удалось прочитать снимок моделей: {problem}"),
            Self::En => format!("cannot read the models snapshot: {problem}"),
        }
    }

    #[must_use]
    pub fn models_listing_error(self, problem: &str) -> String {
        match self {
            Self::Ru => format!("не удалось прочитать список моделей: {problem}"),
            Self::En => format!("cannot read the model listing: {problem}"),
        }
    }

    #[must_use]
    pub fn models_key_not_stored(self, message: &str) -> String {
        match self {
            Self::Ru => format!("ключ не сохранён: {message}"),
            Self::En => format!("the key was not stored: {message}"),
        }
    }

    #[must_use]
    pub fn models_profile_saved_key_failed(self, message: &str) -> String {
        match self {
            Self::Ru => format!("профиль сохранён, но ключ не сохранён: {message}"),
            Self::En => format!("profile saved, but the key was not stored: {message}"),
        }
    }

    #[must_use]
    pub fn models_key_removed(self, reference: &str) -> String {
        match self {
            Self::Ru => format!("сохранённый ключ {reference} удалён"),
            Self::En => format!("removed the stored key {reference}"),
        }
    }

    #[must_use]
    pub fn models_provider_exists(self, id: &str) -> String {
        match self {
            Self::Ru => format!("провайдер с id '{id}' уже существует"),
            Self::En => format!("a provider with id '{id}' already exists"),
        }
    }

    #[must_use]
    pub fn models_id_whitespace(self, id: &str) -> String {
        match self {
            Self::Ru => format!("id модели '{id}' не должен содержать пробелов"),
            Self::En => format!("model id '{id}' must not contain whitespace"),
        }
    }

    #[must_use]
    pub fn models_id_listed_twice(self, id: &str) -> String {
        match self {
            Self::Ru => format!("id модели '{id}' указан дважды"),
            Self::En => format!("model id '{id}' is listed twice"),
        }
    }

    #[must_use]
    pub fn models_reasoning_without_level(self, id: &str) -> String {
        match self {
            Self::Ru => format!("модель '{id}' включает рассуждения без уровня"),
            Self::En => format!("model '{id}' enables reasoning without any level"),
        }
    }

    #[must_use]
    pub fn models_unknown_level(self, id: &str, level: &str) -> String {
        match self {
            Self::Ru => format!("модель '{id}' ссылается на неизвестный уровень '{level}'"),
            Self::En => format!("model '{id}' names an unknown reasoning level '{level}'"),
        }
    }

    #[must_use]
    pub fn models_empty_level_value(self, id: &str, level: &str) -> String {
        match self {
            Self::Ru => format!("у модели '{id}' пустое значение уровня '{level}'"),
            Self::En => format!("model '{id}' has an empty value for reasoning level '{level}'"),
        }
    }

    #[must_use]
    pub fn models_invalid_reasoning(self, id: &str) -> String {
        match self {
            Self::Ru => format!("у модели '{id}' некорректное объявление рассуждений"),
            Self::En => format!("model '{id}' has an invalid reasoning declaration"),
        }
    }

    #[must_use]
    pub fn models_saved_bare(self, route: &str) -> String {
        match self {
            Self::Ru => format!("сохранено {route}"),
            Self::En => format!("saved {route}"),
        }
    }

    #[must_use]
    pub fn models_saved_env_wins(self, route: &str, reference: &str) -> String {
        match self {
            Self::Ru => {
                format!("сохранено {route} ({reference}) — одноимённая переменная окружения важнее")
            }
            Self::En => format!(
                "saved {route} ({reference}) — an environment variable with the same name wins"
            ),
        }
    }

    #[must_use]
    pub fn models_delete_provider_confirm(self, name: &str) -> String {
        match self {
            Self::Ru => format!(
                "Удалить сохранённый профиль {name} и его список моделей? Вернутся значения каталога."
            ),
            Self::En => format!(
                "Delete the stored profile for {name} and its model list? The catalog defaults return."
            ),
        }
    }

    #[must_use]
    pub fn models_delete_model_confirm(self, model_id: &str, name: &str) -> String {
        match self {
            Self::Ru => format!(
                "Удалить '{model_id}' из {name}? Сохраните провайдера, чтобы применить список."
            ),
            Self::En => {
                format!("Remove '{model_id}' from {name}? Save the provider to apply the list.")
            }
        }
    }

    /// Display labels for the web Models settings tab.
    #[must_use]
    pub fn models_labels(self) -> ModelsLabels {
        match self {
            Self::Ru => ModelsLabels {
                err_empty_response: "Пустой ответ",
                err_rate_limited: "Лимит запросов",
                err_server_error: "Ошибка сервера",
                err_timeout: "Таймаут",
                err_transport: "Транспорт",
                err_stream_closed: "Поток закрыт",
                err_malformed_response: "Некорректный ответ",
                err_quota: "Квота",
                choose_retry_mode: "выберите режим повторов",
                retry_initial_delay: "начальная задержка повторов",
                retry_maximum_delay: "максимальная задержка повторов",
                retry_delays_positive: "задержки повторов должны быть больше нуля",
                retry_initial_exceeds_max: "начальная задержка повторов не должна превышать максимальную",
                retry_jitter_not_number: "джиттер повторов должен быть числом от 0 до 1",
                retry_jitter_out_of_range: "джиттер повторов должен быть в диапазоне от 0 до 1",
                select_retryable_code: "выберите хотя бы один код ошибки для повторов",
                maximum_retries: "максимум повторов",
                selection_empties_list: "такой выбор опустошит список моделей; оставьте хотя бы одну строку",
                base_url_scheme: "базовый URL должен начинаться с http:// или https://",
                api_key_upper_snake: "переменная ключа API должна быть в UPPER_SNAKE_CASE",
                every_model_needs_id: "каждой модели нужен id",
                provider_id_format: "id провайдера должен начинаться с буквы и содержать строчные буквы, цифры или '-'",
                custom_needs_base_url: "кастомному провайдеру нужен базовый URL",
                custom_needs_model: "кастомному провайдеру нужна хотя бы одна модель",
                no_active_provider: "Активный провайдер ещё не сохранён; выберите его ниже.",
                provider_default: "По умолчанию у провайдера",
                applies_after_restart: "Применяется к активной модели после перезапуска dshr serve.",
                loading: "загрузка…",
                custom_list: "свой список",
                edit: "Изменить",
                use_route: "Использовать",
                delete: "Удалить",
                new_custom_provider: "Новый кастомный провайдер",
                cancel: "Отмена",
                provider_id: "ID провайдера",
                fixed_after_creation: "не меняется после создания",
                display_name: "Отображаемое имя",
                optional: "необязательно",
                protocol: "Протокол",
                base_url: "Базовый URL",
                api_key_variable: "Переменная ключа API",
                value_lives_in_credentials: "значение хранится в credentials.json (0600); переменная окружения важнее",
                not_configured: "не настроено",
                api_key: "Ключ API",
                hide: "Скрыть",
                show: "Показать",
                remove_saved_key: "Удалить сохранённый ключ",
                retry_policy: "Политика повторов",
                mode: "Режим",
                inherited_adapter: "Наследуется (по умолчанию у адаптера)",
                transient_only: "Только временные ошибки",
                always_retry: "Повторять всегда",
                max_retries: "Максимум повторов",
                initial_delay_ms: "Начальная задержка (мс)",
                max_delay_ms: "Максимальная задержка (мс)",
                jitter_ratio: "Джиттер (0-1)",
                models: "Модели",
                fetching: "Загрузка…",
                fetch_available: "Получить доступные модели",
                custom_needs_one_model: "Кастомному провайдеру нужна хотя бы одна модель.",
                empty_list_inherits: "Сохраните пустой список, чтобы наследовать значения каталога.",
                filter_models: "фильтр моделей",
                select_all: "Выбрать все",
                deselect_all: "Снять выбор",
                update_list: "Обновить список",
                model_id: "id модели",
                context: "контекст",
                max_out: "макс. выход",
                make_active: "сделать активной моделью",
                remove_row: "удалить эту строку",
                reasoning_levels: "уровни рассуждения",
                api_value: "значение в API",
                add_level: "Добавить уровень",
                add_model: "Добавить модель",
                restore_catalog: "Вернуть значения каталога",
                save: "Сохранить",
                catalog_provider: "Провайдер из каталога",
                custom_model_api: "Свой API модели",
                choose_catalog_provider: "выберите провайдера из каталога…",
                add: "Добавить",
                working: "выполняется…",
                welcome_title: "Знакомство с моделями",
                welcome_intro: "Настройте провайдеров и их списки моделей. Профили хранятся в models.yaml; ключи пишутся только на запись в credentials.json (0600), а одноимённая переменная окружения важнее.",
                welcome_no_key: "Ни у одного провайдера ещё нет ключа. Сохраните его здесь или задайте DEEPSEEK_API_KEY / DSH_DEEPSEEK_TOKEN в окружении (настольные настройки пишут desktop.env).",
                deepseek_api_key: "Ключ DeepSeek API",
                stored_in_credentials: "хранится в credentials.json (0600); переменная окружения важнее",
                continue_label: "Продолжить",
                delete_provider: "Удалить провайдера",
                delete_model: "Удалить модель",
                key_configured: "ключ настроен",
                no_key_in_environment: "ключа нет в окружении",
                no_key_for_route: "нет ключа API для этого маршрута",
                anthropic_messages: "Anthropic Messages",
                openai_chat_completions: "OpenAI Chat Completions",
                custom_protocol: "Свой протокол",
                no_models_yet: "провайдер пока не перечисляет модели",
                context_window: "окно контекста",
                max_output_tokens: "максимум выходных токенов",
                enter_base_url: "введите базовый URL перед загрузкой моделей",
                at_least_one_level: "нужен хотя бы один уровень",
                no_saved_key: "для этой ссылки нет сохранённого ключа",
                active_tag: "активный",
                defaults_tag: "по умолчанию",
            },
            Self::En => ModelsLabels {
                err_empty_response: "Empty response",
                err_rate_limited: "Rate limited",
                err_server_error: "Server error",
                err_timeout: "Timeout",
                err_transport: "Transport",
                err_stream_closed: "Stream closed",
                err_malformed_response: "Malformed response",
                err_quota: "Quota",
                choose_retry_mode: "choose a retry mode",
                retry_initial_delay: "retry initial delay",
                retry_maximum_delay: "retry maximum delay",
                retry_delays_positive: "retry delays must be greater than zero",
                retry_initial_exceeds_max: "the retry initial delay must not exceed the maximum delay",
                retry_jitter_not_number: "retry jitter must be a number between 0 and 1",
                retry_jitter_out_of_range: "retry jitter must be between 0 and 1",
                select_retryable_code: "select at least one retryable error code",
                maximum_retries: "maximum retries",
                selection_empties_list: "that selection would empty the model list; keep at least one row",
                base_url_scheme: "the base URL must start with http:// or https://",
                api_key_upper_snake: "the API key variable must be UPPER_SNAKE_CASE",
                every_model_needs_id: "every model needs an id",
                provider_id_format: "the provider id must start with a letter and use lowercase letters, digits or '-'",
                custom_needs_base_url: "a custom provider needs a base URL",
                custom_needs_model: "a custom provider needs at least one model",
                no_active_provider: "No active provider is persisted yet; choose one below.",
                provider_default: "Provider default",
                applies_after_restart: "Applies to the active model after a restart of dshr serve.",
                loading: "loading…",
                custom_list: "custom list",
                edit: "Edit",
                use_route: "Use",
                delete: "Delete",
                new_custom_provider: "New custom provider",
                cancel: "Cancel",
                provider_id: "Provider ID",
                fixed_after_creation: "fixed after creation",
                display_name: "Display name",
                optional: "optional",
                protocol: "Protocol",
                base_url: "Base URL",
                api_key_variable: "API key variable",
                value_lives_in_credentials: "the value lives in credentials.json (0600); env wins",
                not_configured: "not configured",
                api_key: "API key",
                hide: "Hide",
                show: "Show",
                remove_saved_key: "Remove saved key",
                retry_policy: "Retry policy",
                mode: "Mode",
                inherited_adapter: "Inherited (adapter default)",
                transient_only: "Transient errors only",
                always_retry: "Always retry",
                max_retries: "Max retries",
                initial_delay_ms: "Initial delay (ms)",
                max_delay_ms: "Max delay (ms)",
                jitter_ratio: "Jitter (0-1)",
                models: "Models",
                fetching: "Fetching…",
                fetch_available: "Fetch available models",
                custom_needs_one_model: "A custom provider needs at least one model.",
                empty_list_inherits: "Save an empty list to inherit the catalog defaults.",
                filter_models: "filter models",
                select_all: "Select all",
                deselect_all: "Deselect all",
                update_list: "Update list",
                model_id: "model id",
                context: "context",
                max_out: "max out",
                make_active: "make this the active model",
                remove_row: "remove this row",
                reasoning_levels: "reasoning levels",
                api_value: "API value",
                add_level: "Add level",
                add_model: "Add model",
                restore_catalog: "Restore catalog defaults",
                save: "Save",
                catalog_provider: "Catalog provider",
                custom_model_api: "Custom model API",
                choose_catalog_provider: "choose a catalog provider…",
                add: "Add",
                working: "working…",
                welcome_title: "Welcome to Models",
                welcome_intro: "Configure providers and their model lists here. Profiles live in models.yaml; keys are stored write-only in credentials.json (0600), and an environment variable with the same name wins.",
                welcome_no_key: "No provider has a key yet. Save one here, or set DEEPSEEK_API_KEY / DSH_DEEPSEEK_TOKEN in the environment (desktop Settings writes desktop.env).",
                deepseek_api_key: "DeepSeek API key",
                stored_in_credentials: "stored in credentials.json (0600); env wins",
                continue_label: "Continue",
                delete_provider: "Delete provider",
                delete_model: "Delete model",
                key_configured: "key configured",
                no_key_in_environment: "no key in the environment",
                no_key_for_route: "no API key for this route",
                anthropic_messages: "Anthropic Messages",
                openai_chat_completions: "OpenAI Chat Completions",
                custom_protocol: "Custom protocol",
                no_models_yet: "the provider lists no models yet",
                context_window: "context window",
                max_output_tokens: "max output tokens",
                enter_base_url: "enter a base URL before fetching models",
                at_least_one_level: "at least one level is required",
                no_saved_key: "no saved key for this reference",
                active_tag: "active",
                defaults_tag: "defaults",
            },
        }
    }

    #[must_use]
    pub fn session_created_at(self, at: &str) -> String {
        match self {
            Self::Ru => format!("создана:  {at}"),
            Self::En => format!("created:  {at}"),
        }
    }

    #[must_use]
    pub fn session_provider(self, provider: &str) -> String {
        match self {
            Self::Ru => format!("провайдер: {provider}"),
            Self::En => format!("provider: {provider}"),
        }
    }

    #[must_use]
    pub fn session_model(self, model: &str) -> String {
        match self {
            Self::Ru => format!("модель:   {model}"),
            Self::En => format!("model:    {model}"),
        }
    }

    #[must_use]
    pub fn session_cwd(self, cwd: &str) -> String {
        match self {
            Self::Ru => format!("каталог:  {cwd}"),
            Self::En => format!("cwd:      {cwd}"),
        }
    }

    #[must_use]
    pub fn role_system(self) -> &'static str {
        match self {
            Self::Ru => "Система",
            Self::En => "System",
        }
    }

    #[must_use]
    pub fn role_tool(self) -> &'static str {
        match self {
            Self::Ru => "Инструмент",
            Self::En => "Tool",
        }
    }

    #[must_use]
    pub fn npm_no_packages(self) -> &'static str {
        match self {
            Self::Ru => "(пакетов нет)",
            Self::En => "(no packages)",
        }
    }

    #[must_use]
    pub fn npm_homepage(self, url: &str) -> String {
        match self {
            Self::Ru => format!("домашняя страница: {url}"),
            Self::En => format!("homepage: {url}"),
        }
    }

    #[must_use]
    pub fn npm_repository(self, url: &str) -> String {
        match self {
            Self::Ru => format!("репозиторий: {url}"),
            Self::En => format!("repository: {url}"),
        }
    }

    #[must_use]
    pub fn plugin_updated(self, name: &str) -> String {
        match self {
            Self::Ru => format!("плагин обновлён: {name}"),
            Self::En => format!("updated plugin {name}"),
        }
    }

    #[must_use]
    pub fn plugin_state(self, name: &str, enabled: bool) -> String {
        format!(
            "{name}: {}",
            if enabled {
                self.enabled()
            } else {
                self.disabled()
            }
        )
    }

    #[must_use]
    pub fn found(self) -> &'static str {
        match self {
            Self::Ru => "найден",
            Self::En => "found",
        }
    }

    #[must_use]
    pub fn missing(self) -> &'static str {
        match self {
            Self::Ru => "нет",
            Self::En => "missing",
        }
    }

    #[must_use]
    pub fn error_label(self) -> &'static str {
        match self {
            Self::Ru => "ошибка",
            Self::En => "error",
        }
    }

    #[must_use]
    pub fn doctor_version(self) -> &'static str {
        match self {
            Self::Ru => "версия",
            Self::En => "version",
        }
    }

    #[must_use]
    pub fn doctor_platform(self) -> &'static str {
        match self {
            Self::Ru => "платформа",
            Self::En => "platform",
        }
    }

    #[must_use]
    pub fn doctor_home(self) -> &'static str {
        match self {
            Self::Ru => "каталог",
            Self::En => "home",
        }
    }

    #[must_use]
    pub fn doctor_config(self) -> &'static str {
        match self {
            Self::Ru => "конфиг",
            Self::En => "config",
        }
    }

    #[must_use]
    pub fn doctor_profile(self) -> &'static str {
        match self {
            Self::Ru => "профиль",
            Self::En => "profile",
        }
    }

    #[must_use]
    pub fn doctor_provider(self) -> &'static str {
        match self {
            Self::Ru => "провайдер",
            Self::En => "provider",
        }
    }

    #[must_use]
    pub fn doctor_credentials(self) -> &'static str {
        match self {
            Self::Ru => "ключи",
            Self::En => "credentials",
        }
    }

    #[must_use]
    pub fn doctor_sandbox(self) -> &'static str {
        match self {
            Self::Ru => "песочница",
            Self::En => "sandbox",
        }
    }

    #[must_use]
    pub fn doctor_status(self) -> &'static str {
        match self {
            Self::Ru => "статус",
            Self::En => "status",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_formats() {
        assert_eq!(Lang::parse("EN"), Lang::En);
        assert_eq!(Lang::parse("ru"), Lang::Ru);
        assert!(Lang::En.hint().contains("send"));
        assert!(
            Lang::Ru
                .approval("bash", "echo", "shell")
                .contains("Разрешить")
        );
    }

    fn assert_localized<T: PartialEq + ToString + std::fmt::Debug>(ru: T, en: T) {
        assert_ne!(ru, en);
        assert!(!ru.to_string().is_empty());
        assert!(!en.to_string().is_empty());
    }

    #[test]
    fn session_labels_differ_between_languages() {
        assert_localized(
            Lang::Ru.session_created_at("2026-01-01"),
            Lang::En.session_created_at("2026-01-01"),
        );
        assert_localized(
            Lang::Ru.session_provider("deepseek"),
            Lang::En.session_provider("deepseek"),
        );
        assert_localized(
            Lang::Ru.session_model("flash"),
            Lang::En.session_model("flash"),
        );
        assert_localized(Lang::Ru.session_cwd("/tmp"), Lang::En.session_cwd("/tmp"));
        assert_localized(Lang::Ru.role_system(), Lang::En.role_system());
        assert_localized(Lang::Ru.role_tool(), Lang::En.role_tool());
    }

    #[test]
    fn export_labels_differ_between_languages() {
        let ru = Lang::Ru.export_labels();
        let en = Lang::En.export_labels();
        for (ru, en) in [
            (ru.session, en.session),
            (ru.created, en.created),
            (ru.provider, en.provider),
            (ru.model, en.model),
            (ru.cwd, en.cwd),
            (ru.user, en.user),
            (ru.assistant, en.assistant),
            (ru.thinking, en.thinking),
            (ru.result, en.result),
            (ru.image, en.image),
            (ru.audio, en.audio),
        ] {
            assert!(!ru.is_empty(), "ru label is empty");
            assert!(!en.is_empty(), "en label is empty");
            assert_ne!(ru, en, "labels collide: {ru}");
        }
    }

    #[test]
    fn plugin_and_doctor_labels_differ_between_languages() {
        assert_localized(Lang::Ru.npm_no_packages(), Lang::En.npm_no_packages());
        assert_localized(
            Lang::Ru.npm_homepage("https://example.test"),
            Lang::En.npm_homepage("https://example.test"),
        );
        assert_localized(
            Lang::Ru.npm_repository("https://example.test"),
            Lang::En.npm_repository("https://example.test"),
        );
        assert_localized(
            Lang::Ru.plugin_updated("demo"),
            Lang::En.plugin_updated("demo"),
        );
        assert_localized(
            Lang::Ru.plugin_state("demo", true),
            Lang::En.plugin_state("demo", true),
        );
        assert_localized(
            Lang::Ru.plugin_state("demo", false),
            Lang::En.plugin_state("demo", false),
        );
        assert_localized(Lang::Ru.found(), Lang::En.found());
        assert_localized(Lang::Ru.missing(), Lang::En.missing());
        assert_localized(Lang::Ru.error_label(), Lang::En.error_label());
        assert_localized(Lang::Ru.doctor_version(), Lang::En.doctor_version());
        assert_localized(Lang::Ru.doctor_platform(), Lang::En.doctor_platform());
        assert_localized(Lang::Ru.doctor_home(), Lang::En.doctor_home());
        assert_localized(Lang::Ru.doctor_config(), Lang::En.doctor_config());
        assert_localized(Lang::Ru.doctor_profile(), Lang::En.doctor_profile());
        assert_localized(Lang::Ru.doctor_provider(), Lang::En.doctor_provider());
        assert_localized(Lang::Ru.doctor_credentials(), Lang::En.doctor_credentials());
        assert_localized(Lang::Ru.doctor_sandbox(), Lang::En.doctor_sandbox());
        assert_localized(Lang::Ru.doctor_status(), Lang::En.doctor_status());
    }

    #[test]
    fn sessions_list_labels_are_localized() {
        assert_ne!(Lang::Ru.col_created(), Lang::En.col_created());
        assert_ne!(Lang::Ru.col_model(), Lang::En.col_model());
        assert_ne!(Lang::Ru.col_turns(), Lang::En.col_turns());
        assert_ne!(Lang::Ru.no_sessions(), Lang::En.no_sessions());
        assert!(!Lang::Ru.no_sessions().is_empty());
        assert!(!Lang::En.no_sessions().is_empty());
    }
    fn models_fields(labels: &ModelsLabels) -> Vec<(&'static str, &'static str)> {
        vec![
            ("err_empty_response", labels.err_empty_response),
            ("err_rate_limited", labels.err_rate_limited),
            ("err_server_error", labels.err_server_error),
            ("err_timeout", labels.err_timeout),
            ("err_transport", labels.err_transport),
            ("err_stream_closed", labels.err_stream_closed),
            ("err_malformed_response", labels.err_malformed_response),
            ("err_quota", labels.err_quota),
            ("choose_retry_mode", labels.choose_retry_mode),
            ("retry_initial_delay", labels.retry_initial_delay),
            ("retry_maximum_delay", labels.retry_maximum_delay),
            ("retry_delays_positive", labels.retry_delays_positive),
            (
                "retry_initial_exceeds_max",
                labels.retry_initial_exceeds_max,
            ),
            ("retry_jitter_not_number", labels.retry_jitter_not_number),
            (
                "retry_jitter_out_of_range",
                labels.retry_jitter_out_of_range,
            ),
            ("select_retryable_code", labels.select_retryable_code),
            ("maximum_retries", labels.maximum_retries),
            ("selection_empties_list", labels.selection_empties_list),
            ("base_url_scheme", labels.base_url_scheme),
            ("api_key_upper_snake", labels.api_key_upper_snake),
            ("every_model_needs_id", labels.every_model_needs_id),
            ("provider_id_format", labels.provider_id_format),
            ("custom_needs_base_url", labels.custom_needs_base_url),
            ("custom_needs_model", labels.custom_needs_model),
            ("no_active_provider", labels.no_active_provider),
            ("provider_default", labels.provider_default),
            ("applies_after_restart", labels.applies_after_restart),
            ("loading", labels.loading),
            ("custom_list", labels.custom_list),
            ("edit", labels.edit),
            ("use_route", labels.use_route),
            ("delete", labels.delete),
            ("new_custom_provider", labels.new_custom_provider),
            ("cancel", labels.cancel),
            ("provider_id", labels.provider_id),
            ("fixed_after_creation", labels.fixed_after_creation),
            ("display_name", labels.display_name),
            ("optional", labels.optional),
            ("protocol", labels.protocol),
            ("base_url", labels.base_url),
            ("api_key_variable", labels.api_key_variable),
            (
                "value_lives_in_credentials",
                labels.value_lives_in_credentials,
            ),
            ("not_configured", labels.not_configured),
            ("api_key", labels.api_key),
            ("hide", labels.hide),
            ("show", labels.show),
            ("remove_saved_key", labels.remove_saved_key),
            ("retry_policy", labels.retry_policy),
            ("mode", labels.mode),
            ("inherited_adapter", labels.inherited_adapter),
            ("transient_only", labels.transient_only),
            ("always_retry", labels.always_retry),
            ("max_retries", labels.max_retries),
            ("initial_delay_ms", labels.initial_delay_ms),
            ("max_delay_ms", labels.max_delay_ms),
            ("jitter_ratio", labels.jitter_ratio),
            ("models", labels.models),
            ("fetching", labels.fetching),
            ("fetch_available", labels.fetch_available),
            ("custom_needs_one_model", labels.custom_needs_one_model),
            ("empty_list_inherits", labels.empty_list_inherits),
            ("filter_models", labels.filter_models),
            ("select_all", labels.select_all),
            ("deselect_all", labels.deselect_all),
            ("update_list", labels.update_list),
            ("model_id", labels.model_id),
            ("context", labels.context),
            ("max_out", labels.max_out),
            ("make_active", labels.make_active),
            ("remove_row", labels.remove_row),
            ("reasoning_levels", labels.reasoning_levels),
            ("api_value", labels.api_value),
            ("add_level", labels.add_level),
            ("add_model", labels.add_model),
            ("restore_catalog", labels.restore_catalog),
            ("save", labels.save),
            ("catalog_provider", labels.catalog_provider),
            ("custom_model_api", labels.custom_model_api),
            ("choose_catalog_provider", labels.choose_catalog_provider),
            ("add", labels.add),
            ("working", labels.working),
            ("welcome_title", labels.welcome_title),
            ("welcome_intro", labels.welcome_intro),
            ("welcome_no_key", labels.welcome_no_key),
            ("deepseek_api_key", labels.deepseek_api_key),
            ("stored_in_credentials", labels.stored_in_credentials),
            ("continue_label", labels.continue_label),
            ("delete_provider", labels.delete_provider),
            ("delete_model", labels.delete_model),
            ("key_configured", labels.key_configured),
            ("no_key_in_environment", labels.no_key_in_environment),
            ("no_key_for_route", labels.no_key_for_route),
            ("anthropic_messages", labels.anthropic_messages),
            ("openai_chat_completions", labels.openai_chat_completions),
            ("custom_protocol", labels.custom_protocol),
            ("no_models_yet", labels.no_models_yet),
            ("context_window", labels.context_window),
            ("max_output_tokens", labels.max_output_tokens),
            ("enter_base_url", labels.enter_base_url),
            ("at_least_one_level", labels.at_least_one_level),
            ("no_saved_key", labels.no_saved_key),
            ("active_tag", labels.active_tag),
            ("defaults_tag", labels.defaults_tag),
        ]
    }

    #[test]
    fn models_labels_are_complete_in_both_languages() {
        let ru = Lang::Ru.models_labels();
        let en = Lang::En.models_labels();
        let ru_fields = models_fields(&ru);
        let en_fields = models_fields(&en);
        assert_eq!(ru_fields.len(), 103);
        for ((name, ru_text), (_, en_text)) in ru_fields.iter().zip(&en_fields) {
            assert!(!ru_text.is_empty(), "{name} is empty in Russian");
            assert!(!en_text.is_empty(), "{name} is empty in English");
            assert!(
                !en_text
                    .chars()
                    .any(|ch| ('\u{0400}'..='\u{04FF}').contains(&ch)),
                "{name} leaks Cyrillic into English: {en_text}"
            );
        }
    }

    #[test]
    fn models_interpolated_messages_are_localized() {
        assert_localized(
            Lang::Ru.models_saved_route("acme"),
            Lang::En.models_saved_route("acme"),
        );
        assert_localized(
            Lang::Ru.models_route_removed("acme"),
            Lang::En.models_route_removed("acme"),
        );
        assert_localized(
            Lang::Ru.models_active_route_line("p", "m", "note"),
            Lang::En.models_active_route_line("p", "m", "note"),
        );
        assert_localized(
            Lang::Ru.models_active_route("p", "m", " · high"),
            Lang::En.models_active_route("p", "m", " · high"),
        );
        assert_localized(
            Lang::Ru.models_delete_provider_confirm("acme"),
            Lang::En.models_delete_provider_confirm("acme"),
        );
        assert_localized(
            Lang::Ru.models_delete_model_confirm("flash", "acme"),
            Lang::En.models_delete_model_confirm("flash", "acme"),
        );
        assert!(Lang::En.models_whole_number("retries").contains("retries"));
        assert!(Lang::Ru.models_whole_number("retries").contains("retries"));
    }

    #[test]
    fn language_codes_round_trip() {
        assert_eq!(Lang::Ru.code(), "ru");
        assert_eq!(Lang::En.code(), "en");
        assert_eq!(Lang::parse(Lang::Ru.code()), Lang::Ru);
        assert_eq!(Lang::parse(Lang::En.code()), Lang::En);
    }
}

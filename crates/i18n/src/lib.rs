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
}

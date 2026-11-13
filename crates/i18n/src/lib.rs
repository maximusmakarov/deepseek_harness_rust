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
}

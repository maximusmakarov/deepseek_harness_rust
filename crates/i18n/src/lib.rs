#![forbid(unsafe_code)]
//! Minimal localization for terminal UI strings.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Lang {
    #[default]
    Ru,
    En,
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

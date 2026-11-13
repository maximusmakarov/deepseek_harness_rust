use clap::Command;
use dshr_i18n::Lang;

fn t(lang: Lang, key: &str) -> &'static str {
    match (lang, key) {
        (Lang::Ru, "about") => {
            "Rust-native агент-харнесс: TUI, headless и HTTP поверх одного движка."
        }
        (Lang::Ru, "home") => "Домашний каталог harness (по умолчанию $DSH_HOME или ~/.dshr)",
        (Lang::Ru, "profile") => "Имя профиля",
        (Lang::Ru, "patch") => {
            "Дополнительный слой конфигурации после user config (можно повторять)"
        }
        (Lang::Ru, "log_level") => "Уровень логов по умолчанию (RUST_LOG имеет приоритет)",
        (Lang::Ru, "lang") => "Язык CLI: en или ru",
        (Lang::Ru, "run.prompt") => "Prompt для этого хода",
        (Lang::Ru, "run.session") => "Продолжить существующую сессию",
        (Lang::Ru, "run.provider") => "Переопределить provider route",
        (Lang::Ru, "run.model") => "Переопределить модель",
        (Lang::Ru, "run.cassette") => "Проиграть записанную кассету вместо провайдера",
        (Lang::Ru, "run.record") => "Записать нормализованный поток в кассету",
        (Lang::Ru, "run.json") => "Вывести один JSON-объект вместо потока текста",
        (Lang::Ru, "run.max_tokens") => "Максимум output-токенов на запрос",
        (Lang::Ru, "run.max_steps") => "Максимум запросов к модели за ход",
        (Lang::Ru, "run.max_total_tokens") => "Лимит токенов на ход и субагентов (0 = без лимита)",
        (Lang::Ru, "run.danger") => "Разрешить shell и запись вне workspace",
        (Lang::Ru, "run.yes") => "Автоодобрять все эскалации политики",
        (Lang::Ru, "run.no_tools") => "Отключить встроенные инструменты",
        (Lang::Ru, "sessions.json") => "Вывести JSON",
        (Lang::Ru, "sessions.query") => "Строка запроса",
        (Lang::Ru, "sessions.limit") => "Максимум совпадений",
        (Lang::Ru, "sessions.fts") => "Использовать FTS5-индекс вместо скана",
        (Lang::Ru, "doctor") => "Проверить установку и конфигурацию",
        (Lang::Ru, "config") => "Показать итоговую конфигурацию",
        (Lang::Ru, "run") => "Отправить один prompt и напечатать потоковый ответ",
        (Lang::Ru, "sessions") => "Список, просмотр и экспорт сессий",
        (Lang::Ru, "tui") => "Интерактивный терминальный UI",
        (Lang::Ru, "prompt") => "Напечатать собранный системный промпт",
        (Lang::Ru, "plugin") => "Управление MCP-плагинами",
        (Lang::Ru, "serve") => "HTTP/SSE сервер и встроенный Web UI",
        (Lang::Ru, "schedule") => "Управление cron-расписаниями",
        (Lang::Ru, "acp") => "ACP v1 сервер через stdio",
        (Lang::Ru, "sessions.list") => "Список сессий",
        (Lang::Ru, "sessions.show") => "Показать сессию",
        (Lang::Ru, "sessions.export") => "Экспортировать сессию",
        (Lang::Ru, "sessions.fork") => "Форкнуть сессию по номеру события",
        (Lang::Ru, "sessions.rewind") => "Отрезать ходы после первых N",
        (Lang::Ru, "sessions.title") => "Задать заголовок сессии",
        (Lang::Ru, "sessions.search") => "Полнотекстовый поиск по сессиям",
        (Lang::Ru, "sessions.reindex") => "Построить или перестроить FTS5-индекс",
        (Lang::Ru, "schedule.add") => "Добавить расписание",
        (Lang::Ru, "schedule.list") => "Список расписаний",
        (Lang::Ru, "schedule.remove") => "Удалить расписание",
        (Lang::Ru, "schedule.enable") => "Включить расписание",
        (Lang::Ru, "schedule.disable") => "Выключить расписание",
        (Lang::Ru, "schedule.run_due") => "Запустить просроченные расписания",
        (Lang::Ru, "schedule.daemon") => "Запустить демон расписаний",
        (Lang::En, "about") => {
            "Rust-native agent harness: TUI, headless, and HTTP surfaces over one engine."
        }
        (_, "home") => "Harness home directory (defaults to $DSH_HOME or ~/.dshr)",
        (_, "profile") => "Profile name",
        (_, "patch") => "Extra configuration layer applied after the user config (repeatable)",
        (_, "log_level") => "Log level for the default filter (RUST_LOG still wins)",
        (_, "lang") => "CLI language: en or ru",
        (_, "run.prompt") => "Prompt for this turn",
        (_, "run.session") => "Resume an existing session instead of starting a new one",
        (_, "run.provider") => "Override the provider route from config",
        (_, "run.model") => "Override the model id",
        (_, "run.cassette") => "Replay a recorded cassette instead of calling a provider",
        (_, "run.record") => "Record the normalized chunk stream to a cassette",
        (_, "run.json") => "Emit one JSON object instead of streaming text",
        (_, "run.max_tokens") => "Maximum output tokens per model request",
        (_, "run.max_steps") => "Maximum model requests in one turn",
        (_, "run.max_total_tokens") => {
            "Maximum tokens across this turn and delegated agents (0 = unlimited)"
        }
        (_, "run.danger") => "Allow shell commands and writes outside the workspace",
        (_, "run.yes") => "Auto-approve every request that the policy escalates",
        (_, "run.no_tools") => "Disable built-in tools",
        (_, "sessions.json") => "Emit JSON",
        (_, "sessions.query") => "Query string",
        (_, "sessions.limit") => "Maximum number of hits",
        (_, "sessions.fts") => "Use the SQLite FTS5 index instead of a scan",
        (_, "doctor") => "Check the local installation and configuration",
        (_, "config") => "Inspect the resolved configuration",
        (_, "run") => "Send one prompt and print the streamed answer",
        (_, "sessions") => "List, show, or export sessions",
        (_, "tui") => "Interactive terminal UI",
        (_, "prompt") => "Print the assembled system prompt",
        (_, "plugin") => "Manage MCP plugins",
        (_, "serve") => "Run the HTTP/SSE server and embedded web UI",
        (_, "schedule") => "Manage and run cron schedules",
        (_, "acp") => "Run the ACP v1 server over stdio",
        (_, "sessions.list") => "List sessions in the store",
        (_, "sessions.show") => "Show one session's header and messages",
        (_, "sessions.export") => "Export one session",
        (_, "sessions.fork") => "Fork a session at a sequence number",
        (_, "sessions.rewind") => "Drop turns after the first N",
        (_, "sessions.title") => "Set a session title",
        (_, "sessions.search") => "Full-text search across sessions",
        (_, "sessions.reindex") => "Build or rebuild the FTS5 search index",
        (_, "schedule.add") => "Add a schedule",
        (_, "schedule.list") => "List schedules",
        (_, "schedule.remove") => "Remove a schedule",
        (_, "schedule.enable") => "Enable a schedule",
        (_, "schedule.disable") => "Disable a schedule",
        (_, "schedule.run_due") => "Run due schedules",
        (_, "schedule.daemon") => "Run the schedule daemon",
        _ => "",
    }
}

#[must_use]
pub fn scan_lang() -> Lang {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if let Some(value) = arg.strip_prefix("--lang=") {
            return Lang::parse(value);
        }
        if arg == "--lang"
            && let Some(value) = args.next()
        {
            return Lang::parse(&value);
        }
    }
    std::env::var("DSHR_LANG")
        .map(|value| Lang::parse(&value))
        .unwrap_or(Lang::En)
}

#[must_use]
pub fn localize(mut command: Command, lang: Lang) -> Command {
    command = command.about(t(lang, "about"));
    for id in ["home", "profile", "patch", "log_level", "lang"] {
        command = command.mut_arg(id, |arg| arg.help(t(lang, id)));
    }
    command = command.mut_subcommand("doctor", |c| c.about(t(lang, "doctor")));
    command = command.mut_subcommand("config", |c| c.about(t(lang, "config")));
    command = command.mut_subcommand("run", |c| {
        let mut c = c.about(t(lang, "run"));
        for id in [
            "prompt",
            "session",
            "provider",
            "model",
            "cassette",
            "record",
            "json",
            "max_tokens",
            "max_steps",
            "max_total_tokens",
            "danger",
            "yes",
            "no_tools",
        ] {
            c = c.mut_arg(id, |arg| arg.help(t(lang, &format!("run.{id}"))));
        }
        c
    });
    command = command.mut_subcommand("tui", |c| c.about(t(lang, "tui")));
    command = command.mut_subcommand("prompt", |c| c.about(t(lang, "prompt")));
    command = command.mut_subcommand("plugin", |c| c.about(t(lang, "plugin")));
    command = command.mut_subcommand("serve", |c| c.about(t(lang, "serve")));
    command = command.mut_subcommand("acp", |c| c.about(t(lang, "acp")));
    command = command.mut_subcommand("sessions", |c| {
        let mut c = c.about(t(lang, "sessions"));
        for (name, key) in [
            ("list", "sessions.list"),
            ("show", "sessions.show"),
            ("export", "sessions.export"),
            ("fork", "sessions.fork"),
            ("rewind", "sessions.rewind"),
            ("title", "sessions.title"),
            ("search", "sessions.search"),
            ("reindex", "sessions.reindex"),
        ] {
            c = c.mut_subcommand(name, |s| {
                let mut s = s.about(t(lang, key));
                match name {
                    "list" => {
                        s = s.mut_arg("json", |a| a.help(t(lang, "sessions.json")));
                    }
                    "search" => {
                        for (arg, key) in [
                            ("query", "sessions.query"),
                            ("limit", "sessions.limit"),
                            ("json", "sessions.json"),
                            ("fts", "sessions.fts"),
                        ] {
                            s = s.mut_arg(arg, |a| a.help(t(lang, key)));
                        }
                    }
                    _ => {}
                }
                s
            });
        }
        c
    });
    command = command.mut_subcommand("schedule", |c| {
        let mut c = c.about(t(lang, "schedule"));
        for (name, key) in [
            ("add", "schedule.add"),
            ("list", "schedule.list"),
            ("remove", "schedule.remove"),
            ("enable", "schedule.enable"),
            ("disable", "schedule.disable"),
            ("run-due", "schedule.run_due"),
            ("daemon", "schedule.daemon"),
        ] {
            c = c.mut_subcommand(name, |s| {
                let mut s = s.about(t(lang, key));
                match name {
                    "list" => {
                        s = s.mut_arg("json", |a| a.help(t(lang, "sessions.json")));
                    }
                    "search" => {
                        for (arg, key) in [
                            ("query", "sessions.query"),
                            ("limit", "sessions.limit"),
                            ("json", "sessions.json"),
                            ("fts", "sessions.fts"),
                        ] {
                            s = s.mut_arg(arg, |a| a.help(t(lang, key)));
                        }
                    }
                    _ => {}
                }
                s
            });
        }
        c
    });
    command
}

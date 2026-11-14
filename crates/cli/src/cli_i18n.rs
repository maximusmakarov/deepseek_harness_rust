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
        (Lang::Ru, "plugin") => "Плагины MCP и подпись WASM",
        (Lang::Ru, "release") => "Подпись и проверка релизных артефактов",
        (Lang::Ru, "release.keygen") => "Сгенерировать пару ключей Ed25519",
        (Lang::Ru, "release.sign") => "Подписать релизный артефакт",
        (Lang::Ru, "release.verify") => "Проверить подпись артефакта",
        (Lang::Ru, "release.public-key") => "Напечатать публичный ключ для секретного",
        (Lang::Ru, "release.file") => "Файл",
        (Lang::Ru, "release.key") => "Секретный ключ (hex) или $DSHR_RELEASE_KEY",
        (Lang::Ru, "release.key_public") => "Публичный ключ (hex)",
        (Lang::Ru, "release.out") => "Файл подписи (по умолчанию <file>.sig)",
        (Lang::Ru, "release.signature") => "Файл отделённой подписи",
        (Lang::Ru, "serve") => "HTTP/SSE сервер и встроенный Web UI",
        (Lang::Ru, "schedule") => "Управление cron-расписаниями",
        (Lang::Ru, "acp") => "ACP v1 сервер через stdio",
        (Lang::Ru, "sessions.id") => "ID сессии",
        (Lang::Ru, "sessions.format") => "Формат: markdown или json",
        (Lang::Ru, "sessions.at") => "Последний копируемый номер события",
        (Lang::Ru, "sessions.turn") => "Сколько ходов оставить",
        (Lang::Ru, "sessions.title_value") => "Новый заголовок",
        (Lang::Ru, "sessions.list") => "Список сессий",
        (Lang::Ru, "sessions.show") => "Показать сессию",
        (Lang::Ru, "sessions.export") => "Экспортировать сессию",
        (Lang::Ru, "sessions.fork") => "Форкнуть сессию по номеру события",
        (Lang::Ru, "sessions.rewind") => "Отрезать ходы после первых N",
        (Lang::Ru, "sessions.title") => "Задать заголовок сессии",
        (Lang::Ru, "sessions.search") => "Полнотекстовый поиск по сессиям",
        (Lang::Ru, "sessions.reindex") => "Построить или перестроить FTS5-индекс",
        (Lang::Ru, "sessions.import") => "Импортировать сессию DSH v4 (session.v4.jsonl.zstd)",
        (Lang::Ru, "sessions.import.path") => "Каталог или файл сессии",
        (Lang::Ru, "sessions.import.provider") => "Метка провайдера в заголовке импорта",
        (Lang::Ru, "sessions.import.model") => "Метка модели в заголовке импорта",
        (Lang::Ru, "sessions.import.all") => "Импортировать все файлы сессий под путём",
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
        (_, "plugin") => "MCP plugins and WASM signing",
        (_, "release") => "Sign and verify release artifacts",
        (_, "release.keygen") => "Generate an Ed25519 key pair",
        (_, "release.sign") => "Sign a release artifact",
        (_, "release.verify") => "Verify a release artifact signature",
        (_, "release.public-key") => "Print the public key for a secret key",
        (_, "release.file") => "File to sign or verify",
        (_, "release.key") => "Secret key (hex) or $DSHR_RELEASE_KEY",
        (_, "release.key_public") => "Public key (hex)",
        (_, "release.out") => "Signature output (default: <file>.sig)",
        (_, "release.signature") => "Detached signature file",
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
        (_, "sessions.import") => "Import a DSH v4 session (session.v4.jsonl.zstd)",
        (_, "sessions.import.path") => "Session directory or file",
        (_, "sessions.import.provider") => "Provider label stored in the imported header",
        (_, "sessions.import.model") => "Model label stored in the imported header",
        (_, "sessions.import.all") => "Import every session file under the path",
        (_, "schedule.add") => "Add a schedule",
        (_, "schedule.list") => "List schedules",
        (_, "schedule.remove") => "Remove a schedule",
        (_, "schedule.enable") => "Enable a schedule",
        (_, "schedule.disable") => "Disable a schedule",
        (_, "schedule.run_due") => "Run due schedules",
        (_, "schedule.daemon") => "Run the schedule daemon",
        (Lang::Ru, "doctor.json") => "Вывести машиночитаемый JSON",
        (Lang::Ru, "config.show") => "Напечатать итоговую конфигурацию",
        (Lang::Ru, "config.show.json") => "Вывести JSON вместо YAML",
        (Lang::Ru, "config.show.schema") => "Напечатать JSON Schema конфигурации",
        (Lang::Ru, "config.show.sources") => "Показать цепочку слоёв в stderr",
        (Lang::Ru, "prompt.quiet") => "Печатать промпт с завершающим переводом строки",
        (Lang::Ru, "plugin.list") => "Список MCP-плагинов",
        (Lang::Ru, "plugin.add") => "Добавить или заменить MCP-плагин",
        (Lang::Ru, "plugin.remove") => "Удалить MCP-плагин",
        (Lang::Ru, "plugin.keygen") => "Сгенерировать пару ключей Ed25519 для WASM-плагинов",
        (Lang::Ru, "plugin.sign") => "Подписать манифест WASM-плагина",
        (Lang::Ru, "plugin.verify") => "Проверить подпись манифеста WASM-плагина",
        (Lang::Ru, "plugin.json") => "Вывести JSON",
        (Lang::Ru, "plugin.manifest") => "Путь к манифесту плагина (*.yaml)",
        (Lang::Ru, "plugin.key") => "Файл секретного ключа из `plugin keygen`",
        (Lang::Ru, "plugin.out") => "Куда записать секретный ключ (hex); публичный печатается",
        (Lang::Ru, "plugin.name") => "Имя плагина/сервера",
        (Lang::Ru, "plugin.command") => "Команда запуска stdio-сервера",
        (Lang::Ru, "plugin.args") => "Аргумент (можно повторять)",
        (Lang::Ru, "plugin.env") => "Переменная окружения KEY=VALUE (можно повторять)",
        (Lang::Ru, "plugin.url") => "Streamable HTTP endpoint вместо команды",
        (Lang::Ru, "plugin.npm") => "Установка и управление npm/Cordis-плагинами",
        (Lang::Ru, "plugin.npm.add") => "Установить npm-пакет как Cordis-плагин",
        (Lang::Ru, "plugin.npm.add.spec") => "npm-спецификация: пакет, @scope/name@version, file:…",
        (Lang::Ru, "plugin.npm.add.name") => {
            "Имя конфига и пакета, если не выводится из спецификации"
        }
        (Lang::Ru, "plugin.npm.remove") => "Удалить установленный Cordis-плагин",
        (Lang::Ru, "plugin.npm.remove.name") => "Имя плагина",
        (Lang::Ru, "plugin.npm.update") => "Обновить установленные Cordis-плагины",
        (Lang::Ru, "plugin.npm.update.name") => "Имя плагина; обновить все, если не задано",
        (Lang::Ru, "plugin.npm.enable") => "Включить Cordis-плагин",
        (Lang::Ru, "plugin.npm.enable.name") => "Имя плагина",
        (Lang::Ru, "plugin.npm.disable") => "Выключить Cordis-плагин без удаления",
        (Lang::Ru, "plugin.npm.disable.name") => "Имя плагина",
        (Lang::Ru, "plugin.npm.list") => "Список настроенных Cordis-плагинов",
        (Lang::Ru, "plugin.npm.list.json") => "Вывести JSON",
        (Lang::Ru, "plugin.npm.search") => "Искать Cordis-плагины в npm registry",
        (Lang::Ru, "plugin.npm.search.query") => "Запрос к npm registry",
        (Lang::Ru, "plugin.npm.search.limit") => "Максимум совпадений",
        (Lang::Ru, "plugin.npm.search.json") => "Вывести JSON",
        (Lang::Ru, "plugin.npm.info") => "Показать метаданные npm-пакета",
        (Lang::Ru, "plugin.npm.info.name") => "Имя npm-пакета",
        (Lang::Ru, "plugin.npm.info.json") => "Вывести JSON",
        (Lang::Ru, "serve.bind") => "Адрес привязки",
        (Lang::Ru, "serve.port") => "Порт (0 — свободный)",
        (Lang::Ru, "serve.token") => "Bearer-токен; генерируется, если не задан",
        (Lang::Ru, "serve.provider") => "Переопределить provider route",
        (Lang::Ru, "serve.model") => "Переопределить модель",
        (Lang::Ru, "serve.cassette") => "Проиграть записанную кассету",
        (Lang::Ru, "serve.danger") => "Разрешить shell и запись вне workspace",
        (Lang::Ru, "serve.no_tools") => "Отключить встроенные инструменты",
        (Lang::Ru, "serve.max_steps") => "Максимум запросов к модели за ход",
        (Lang::Ru, "serve.max_tokens") => "Максимум output-токенов на запрос",
        (Lang::Ru, "acp.provider") => "Переопределить provider route",
        (Lang::Ru, "acp.model") => "Переопределить модель",
        (Lang::Ru, "acp.cassette") => "Проиграть записанную кассету",
        (Lang::Ru, "acp.danger") => "Разрешить shell и запись вне workspace",
        (Lang::Ru, "acp.max_steps") => "Максимум запросов к модели за ход",
        (Lang::Ru, "acp.max_tokens") => "Максимум output-токенов на запрос",
        (Lang::Ru, "tui.session") => "Продолжить существующую сессию",
        (Lang::Ru, "tui.cassette") => "Проиграть записанную кассету",
        (Lang::Ru, "tui.provider") => "Переопределить provider route",
        (Lang::Ru, "tui.model") => "Переопределить модель",
        (Lang::Ru, "tui.danger") => "Разрешить shell и запись вне workspace",
        (Lang::Ru, "tui.no_tools") => "Отключить встроенные инструменты",
        (Lang::Ru, "tui.max_steps") => "Максимум запросов к модели за ход",
        (Lang::Ru, "tui.max_tokens") => "Максимум output-токенов на запрос",
        (Lang::Ru, "schedule.add.cron") => "Cron-выражение",
        (Lang::Ru, "schedule.add.prompt") => "Prompt расписания",
        (Lang::Ru, "schedule.add.name") => "Стабильный id (генерируется, если не задан)",
        (Lang::Ru, "schedule.add.session") => "Продолжить эту сессию",
        (Lang::Ru, "schedule.add.workspace") => "Workspace для запуска",
        (Lang::Ru, "schedule.list.json") => "Вывести JSON",
        (Lang::Ru, "schedule.id") => "ID расписания",
        (Lang::Ru, "schedule.run_due.now") => {
            "Оценить на этот RFC3339 момент (по умолчанию сейчас)"
        }
        (Lang::Ru, "schedule.daemon.interval_secs") => "Интервал опроса, секунды",
        (Lang::Ru, "schedule.run.cassette") => "Проиграть записанную кассету",
        (Lang::Ru, "schedule.run.provider") => "Переопределить provider route",
        (Lang::Ru, "schedule.run.model") => "Переопределить модель",
        (Lang::Ru, "schedule.run.danger") => "Запускать с danger-full-access",
        (Lang::Ru, "schedule.run.max_steps") => "Максимум запросов к модели за ход",
        (Lang::En, "doctor.json") => "Emit machine-readable JSON",
        (Lang::En, "config.show") => "Print the merged configuration",
        (Lang::En, "config.show.json") => "Emit JSON instead of YAML",
        (Lang::En, "config.show.schema") => "Print the JSON Schema of the configuration document",
        (Lang::En, "config.show.sources") => "Print the layer resolution chain to stderr",
        (Lang::En, "prompt.quiet") => "Emit the prompt with a trailing newline only",
        (Lang::En, "plugin.list") => "List configured MCP plugins",
        (Lang::En, "plugin.add") => "Add or replace an MCP plugin",
        (Lang::En, "plugin.remove") => "Remove an MCP plugin",
        (Lang::En, "plugin.keygen") => "Generate an Ed25519 key pair for WASM plugins",
        (Lang::En, "plugin.sign") => "Sign a WASM plugin manifest",
        (Lang::En, "plugin.verify") => "Verify a WASM plugin manifest signature",
        (Lang::En, "plugin.json") => "Emit JSON",
        (Lang::En, "plugin.manifest") => "Path to the plugin manifest (*.yaml)",
        (Lang::En, "plugin.key") => "Secret key file written by `plugin keygen`",
        (Lang::En, "plugin.out") => {
            "Where to write the secret key (hex); the public key is printed"
        }
        (Lang::En, "plugin.name") => "Plugin/server name",
        (Lang::En, "plugin.command") => "Command to launch a stdio server",
        (Lang::En, "plugin.args") => "Argument (repeatable)",
        (Lang::En, "plugin.env") => "Environment entry KEY=VALUE (repeatable)",
        (Lang::En, "plugin.url") => "Streamable HTTP endpoint instead of a command",
        (Lang::En, "plugin.npm") => "Install and manage npm/Cordis plugins",
        (Lang::En, "plugin.npm.add") => "Install an npm package as a Cordis plugin",
        (Lang::En, "plugin.npm.add.spec") => "npm spec (package, @scope/name@version, file:…)",
        (Lang::En, "plugin.npm.add.name") => {
            "Config name and package name when it cannot be derived from the spec"
        }
        (Lang::En, "plugin.npm.remove") => "Remove an installed Cordis plugin",
        (Lang::En, "plugin.npm.remove.name") => "Plugin name",
        (Lang::En, "plugin.npm.update") => "Update installed Cordis plugins",
        (Lang::En, "plugin.npm.update.name") => "Plugin name; all plugins when omitted",
        (Lang::En, "plugin.npm.enable") => "Enable a Cordis plugin",
        (Lang::En, "plugin.npm.enable.name") => "Plugin name",
        (Lang::En, "plugin.npm.disable") => "Disable a Cordis plugin without removing it",
        (Lang::En, "plugin.npm.disable.name") => "Plugin name",
        (Lang::En, "plugin.npm.list") => "List configured Cordis plugins",
        (Lang::En, "plugin.npm.list.json") => "Emit JSON",
        (Lang::En, "plugin.npm.search") => "Search the npm registry for Cordis plugins",
        (Lang::En, "plugin.npm.search.query") => "Registry query",
        (Lang::En, "plugin.npm.search.limit") => "Maximum number of hits",
        (Lang::En, "plugin.npm.search.json") => "Emit JSON",
        (Lang::En, "plugin.npm.info") => "Show npm package metadata",
        (Lang::En, "plugin.npm.info.name") => "Package name",
        (Lang::En, "plugin.npm.info.json") => "Emit JSON",
        (Lang::En, "serve.bind") => "Bind address",
        (Lang::En, "serve.port") => "Port (0 selects a free port)",
        (Lang::En, "serve.token") => "Bearer token; generated when omitted",
        (Lang::En, "serve.provider") => "Provider route override",
        (Lang::En, "serve.model") => "Model override",
        (Lang::En, "serve.cassette") => "Replay a cassette instead of calling a provider",
        (Lang::En, "serve.danger") => "Allow shell and out-of-workspace writes without approval",
        (Lang::En, "serve.no_tools") => "Disable built-in tools",
        (Lang::En, "serve.max_steps") => "Maximum model requests per turn",
        (Lang::En, "serve.max_tokens") => "Maximum output tokens per request",
        (Lang::En, "acp.provider") => "Provider route override",
        (Lang::En, "acp.model") => "Model override",
        (Lang::En, "acp.cassette") => "Replay a cassette instead of calling a provider",
        (Lang::En, "acp.danger") => "Allow shell and writes outside the workspace",
        (Lang::En, "acp.max_steps") => "Maximum model requests per turn",
        (Lang::En, "acp.max_tokens") => "Maximum output tokens per request",
        (Lang::En, "tui.session") => "Resume an existing session",
        (Lang::En, "tui.cassette") => "Replay a recorded cassette instead of calling a provider",
        (Lang::En, "tui.provider") => "Override the provider route from config",
        (Lang::En, "tui.model") => "Override the model id",
        (Lang::En, "tui.danger") => "Allow shell commands and writes outside the workspace",
        (Lang::En, "tui.no_tools") => "Disable built-in tools",
        (Lang::En, "tui.max_steps") => "Maximum model requests per turn",
        (Lang::En, "tui.max_tokens") => "Maximum output tokens per request",
        (Lang::En, "schedule.add.cron") => "Cron expression",
        (Lang::En, "schedule.add.prompt") => "Prompt for the scheduled run",
        (Lang::En, "schedule.add.name") => "Stable id (generated when omitted)",
        (Lang::En, "schedule.add.session") => "Resume this session instead of creating a new one",
        (Lang::En, "schedule.add.workspace") => "Workspace for the scheduled run",
        (Lang::En, "schedule.list.json") => "Emit JSON",
        (Lang::En, "schedule.id") => "Schedule id",
        (Lang::En, "schedule.run_due.now") => "Evaluate at this RFC3339 instant (default: now)",
        (Lang::En, "schedule.daemon.interval_secs") => "Poll interval in seconds",
        (Lang::En, "schedule.run.cassette") => "Replay a recorded cassette",
        (Lang::En, "schedule.run.provider") => "Override the provider route",
        (Lang::En, "schedule.run.model") => "Override the model id",
        (Lang::En, "schedule.run.danger") => "Run schedules with danger-full-access",
        (Lang::En, "schedule.run.max_steps") => "Maximum model requests per turn",
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

fn with_args(mut command: Command, lang: Lang, prefix: &str, ids: &[&str]) -> Command {
    for id in ids {
        let key = format!("{prefix}.{id}");
        command = command.mut_arg(*id, |arg| arg.help(t(lang, &key)));
    }
    command
}

#[must_use]
pub fn localize(mut command: Command, lang: Lang) -> Command {
    command = command.about(t(lang, "about"));
    for id in ["home", "profile", "patch", "log_level", "lang"] {
        command = command.mut_arg(id, |arg| arg.help(t(lang, id)));
    }
    command = command.mut_subcommand("doctor", |c| {
        with_args(c.about(t(lang, "doctor")), lang, "doctor", &["json"])
    });
    command = command.mut_subcommand("config", |c| {
        c.about(t(lang, "config")).mut_subcommand("show", |show| {
            with_args(
                show.about(t(lang, "config.show")),
                lang,
                "config.show",
                &["json", "schema", "sources"],
            )
        })
    });
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
    command = command.mut_subcommand("tui", |c| {
        with_args(
            c.about(t(lang, "tui")),
            lang,
            "tui",
            &[
                "session",
                "cassette",
                "provider",
                "model",
                "danger",
                "no_tools",
                "max_steps",
                "max_tokens",
            ],
        )
    });
    command = command.mut_subcommand("prompt", |c| {
        with_args(c.about(t(lang, "prompt")), lang, "prompt", &["quiet"])
    });
    command = command.mut_subcommand("plugin", |c| c.about(t(lang, "plugin")));
    command = command.mut_subcommand("serve", |c| {
        with_args(
            c.about(t(lang, "serve")),
            lang,
            "serve",
            &[
                "bind",
                "port",
                "token",
                "provider",
                "model",
                "cassette",
                "danger",
                "no_tools",
                "max_steps",
                "max_tokens",
            ],
        )
    });
    command = command.mut_subcommand("acp", |c| {
        with_args(
            c.about(t(lang, "acp")),
            lang,
            "acp",
            &[
                "provider",
                "model",
                "cassette",
                "danger",
                "max_steps",
                "max_tokens",
            ],
        )
    });
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
            ("import", "sessions.import"),
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
                    "show" => {
                        s = s.mut_arg("id", |a| a.help(t(lang, "sessions.id")));
                    }
                    "export" => {
                        for (arg, key) in [("id", "sessions.id"), ("format", "sessions.format")] {
                            s = s.mut_arg(arg, |a| a.help(t(lang, key)));
                        }
                    }
                    "fork" => {
                        for (arg, key) in [("id", "sessions.id"), ("at", "sessions.at")] {
                            s = s.mut_arg(arg, |a| a.help(t(lang, key)));
                        }
                    }
                    "rewind" => {
                        for (arg, key) in [("id", "sessions.id"), ("turn", "sessions.turn")] {
                            s = s.mut_arg(arg, |a| a.help(t(lang, key)));
                        }
                    }
                    "title" => {
                        for (arg, key) in [("id", "sessions.id"), ("title", "sessions.title_value")]
                        {
                            s = s.mut_arg(arg, |a| a.help(t(lang, key)));
                        }
                    }
                    "import" => {
                        for (arg, key) in [
                            ("path", "sessions.import.path"),
                            ("provider", "sessions.import.provider"),
                            ("model", "sessions.import.model"),
                            ("all", "sessions.import.all"),
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
    command = command.mut_subcommand("plugin", |c| {
        let mut c = c.about(t(lang, "plugin"));
        c = c.mut_subcommand("list", |list| {
            with_args(
                list.about(t(lang, "plugin.list")),
                lang,
                "plugin",
                &["json"],
            )
        });
        c = c.mut_subcommand("add", |add| {
            with_args(
                add.about(t(lang, "plugin.add")),
                lang,
                "plugin",
                &["name", "command", "args", "env", "url"],
            )
        });
        c = c.mut_subcommand("remove", |remove| {
            with_args(
                remove.about(t(lang, "plugin.remove")),
                lang,
                "plugin",
                &["name"],
            )
        });
        c = c.mut_subcommand("keygen", |keygen| {
            with_args(
                keygen.about(t(lang, "plugin.keygen")),
                lang,
                "plugin",
                &["out"],
            )
        });
        c = c.mut_subcommand("sign", |sign| {
            with_args(
                sign.about(t(lang, "plugin.sign")),
                lang,
                "plugin",
                &["manifest", "key"],
            )
        });
        c = c.mut_subcommand("verify", |verify| {
            with_args(
                verify.about(t(lang, "plugin.verify")),
                lang,
                "plugin",
                &["manifest", "key"],
            )
        });
        c = c.mut_subcommand("npm", |npm| {
            let mut npm = npm.about(t(lang, "plugin.npm"));
            for (name, key) in [
                ("add", "plugin.npm.add"),
                ("remove", "plugin.npm.remove"),
                ("update", "plugin.npm.update"),
                ("enable", "plugin.npm.enable"),
                ("disable", "plugin.npm.disable"),
                ("list", "plugin.npm.list"),
                ("search", "plugin.npm.search"),
                ("info", "plugin.npm.info"),
            ] {
                npm = npm.mut_subcommand(name, |s| {
                    let s = s.about(t(lang, key));
                    match name {
                        "add" => with_args(s, lang, "plugin.npm.add", &["spec", "name"]),
                        "remove" => with_args(s, lang, "plugin.npm.remove", &["name"]),
                        "update" => with_args(s, lang, "plugin.npm.update", &["name"]),
                        "enable" => with_args(s, lang, "plugin.npm.enable", &["name"]),
                        "disable" => with_args(s, lang, "plugin.npm.disable", &["name"]),
                        "list" => with_args(s, lang, "plugin.npm.list", &["json"]),
                        "search" => {
                            with_args(s, lang, "plugin.npm.search", &["query", "limit", "json"])
                        }
                        "info" => with_args(s, lang, "plugin.npm.info", &["name", "json"]),
                        _ => s,
                    }
                });
            }
            npm
        });
        c
    });
    command = command.mut_subcommand("release", |c| {
        let mut c = c.about(t(lang, "release"));
        c = c.mut_subcommand("keygen", |s| {
            with_args(
                s.about(t(lang, "release.keygen")),
                lang,
                "release",
                &["out"],
            )
        });
        c = c.mut_subcommand("sign", |s| {
            let s = s.about(t(lang, "release.sign"));
            let mut s = s.mut_arg("file", |arg| arg.help(t(lang, "release.file")));
            s = s.mut_arg("key", |arg| arg.help(t(lang, "release.key")));
            s.mut_arg("out", |arg| arg.help(t(lang, "release.out")))
        });
        c = c.mut_subcommand("verify", |s| {
            let s = s.about(t(lang, "release.verify"));
            let mut s = s.mut_arg("file", |arg| arg.help(t(lang, "release.file")));
            s = s.mut_arg("key", |arg| arg.help(t(lang, "release.key_public")));
            s.mut_arg("signature", |arg| arg.help(t(lang, "release.signature")))
        });
        c = c.mut_subcommand("public-key", |s| {
            s.about(t(lang, "release.public-key"))
                .mut_arg("key", |arg| arg.help(t(lang, "release.key")))
        });
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
                let s = s.about(t(lang, key));
                match name {
                    "add" => with_args(
                        s,
                        lang,
                        "schedule.add",
                        &["cron", "prompt", "name", "session", "workspace"],
                    ),
                    "list" => with_args(s, lang, "schedule.list", &["json"]),
                    "remove" | "enable" | "disable" => with_args(s, lang, "schedule", &["id"]),
                    "run-due" => {
                        let s = with_args(
                            s,
                            lang,
                            "schedule.run",
                            &["cassette", "provider", "model", "danger", "max_steps"],
                        );
                        s.mut_arg("now", |arg| arg.help(t(lang, "schedule.run_due.now")))
                    }
                    "daemon" => {
                        let s = with_args(
                            s,
                            lang,
                            "schedule.run",
                            &["cassette", "provider", "model", "danger", "max_steps"],
                        );
                        s.mut_arg("interval_secs", |arg| {
                            arg.help(t(lang, "schedule.daemon.interval_secs"))
                        })
                    }
                    _ => s,
                }
            });
        }
        c
    });
    command
}

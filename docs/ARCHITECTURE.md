# Архитектура `dshr`

Целевая архитектура Rust-аналога DeepSeek Harness. Документ описывает **что строим**, а не порядок работ (см. [PLAN.md](PLAN.md)) и не декомпозицию (см. [ROADMAP.md](ROADMAP.md)).

Версия 1.0 · 12.11.2026

---

## 1. Обзор

`dshr` — это **улучшенная Rust-реализация** концепции DSH Desktop: event-sourced агентный рантайм с заменяемыми швами, без Electron/Node/pnpm; плагины — WASM по умолчанию, Cordis/npm — через отдельный JS-рантайм (см. [IMPROVEMENTS.md](IMPROVEMENTS.md)). Три инварианта:

1. **Журнал — истина.** История модели, UI, resume, fork, аудит и телеметрия выводятся из одного append-only лога.
2. **Швы — единственная точка расширения.** LLM, FS, shell, sandbox, tools, MCP, storage, UI — трейты.
3. **Один движок — много поверхностей.** TUI, headless и HTTP API работают с одним `AgentHandle`.

```text
┌───────────────────────────────────────────────────────────────────┐
│  Поверхности: dshr-cli · dshr-tui · dshr-server (HTTP/WS) · SDK   │
├───────────────────────────────────────────────────────────────────┤
│  Оркестрация: dshr-agent (turn/step) · dshr-subagent · dshr-jobs  │
│               dshr-prompt · dshr-compaction · dshr-skills         │
├───────────────────────────────────────────────────────────────────┤
│  Возможности: dshr-tools (+ tool-*) · dshr-mcp · dshr-workflow    │
├───────────────────────────────────────────────────────────────────┤
│  Инфраструктура: dshr-session (+ jsonl, index) · dshr-llm (+     │
│  адаптеры) · dshr-sandbox · dshr-plugin · dshr-config             │
├───────────────────────────────────────────────────────────────────┤
│  Ядро: dshr-core (ids, errors, время, редакция, JSON)             │
└───────────────────────────────────────────────────────────────────┘
```

---

## 2. Workspace и крейты

```text
dshr/
├── Cargo.toml                 # [workspace], resolver = "3"
├── rust-toolchain.toml        # stable, edition 2024
├── crates/
│   ├── core/                  # dshr-core      ids, ошибки, время, redaction
│   ├── config/                # dshr-config    профили, бандлы, слои YAML
│   ├── llm/                   # dshr-llm       Message/StreamChunk/Adapter
│   ├── llm-deepseek/          #                SSE, DeepSeek API
│   ├── llm-openai/            #                OpenAI-совместимые
│   ├── llm-anthropic/         #                (T2)
│   ├── llm-mock/              #                mock + cassette replay
│   ├── session/               # dshr-session   события, derive_messages, проекции
│   ├── session-jsonl/         #                JSONL-хранилище, миграции
│   ├── session-index/         #                SQLite + FTS5
│   ├── prompt/                # dshr-prompt    сборка системного промпта
│   ├── tools/                 # dshr-tools     реестр, пайплайн, схемы
│   ├── tool-fs/               #                read/write/edit/patch/glob/grep
│   ├── tool-shell/            #                bash/pwsh, persistent
│   ├── tool-web/              #                search/fetch
│   ├── tool-basic/            #                todo/ask_user/present
│   ├── tool-subagent/         #                subagent_*
│   ├── tool-jobs/             #                job_*
│   ├── tool-skill/            #                skill
│   ├── agent/                 # dshr-agent     цикл, inbox, отмена
│   ├── subagent/              # dshr-subagent  дерево, провайдеры
│   ├── jobs/                  # dshr-jobs      фоновые процессы
│   ├── compaction/            # dshr-compaction
│   ├── skills/                # dshr-skills
│   ├── mcp/                   # dshr-mcp       клиент (rmcp), ресурсы
│   ├── sandbox/               # dshr-sandbox   политика + бэкенды ОС
│   ├── plugin/                # dshr-plugin    wasmtime host, манифест
│   ├── workflow/              # dshr-workflow  (T3)
│   ├── schedule/              # T3: расписания
│   ├── goal/                  # T3: цели
│   ├── agent-team/            # T3: команды агентов
│   ├── worktree/              # T3: git-ветки
│   ├── lsp/                   # T3: LSP
│   ├── telemetry/             # T3: OTel
│   ├── terminal/              # T3: persistent PTY
│   ├── server/                # dshr-server    axum REST/WS
│   ├── sdk/                   # dshr-sdk       JSON-RPC клиент/сервер
│   ├── tui/                   # dshr-tui       ratatui
│   ├── cli/                   # dshr-cli       clap-бинарь
│   └── testkit/               # dshr-testkit   фикстуры, mock LLM/MCP
├── xtask/                     # генерация схем, доков, релиз
└── docs/                      # ADR, гайды, этот план
```

Правила зависимостей:

- `core` не зависит ни от чего, кроме std/сериализации.
- `llm`, `session`, `config`, `sandbox` зависят только от `core`.
- `agent` не знает про TUI/server/CLI.
- `tui`, `server`, `cli` — верхний слой; обратных зависимостей нет.
- Никаких циклических зависимостей: проверяется `cargo deny`/`xtask deps`.

---

## 3. Ядро: типы и идентификаторы

```rust
// dshr-core/src/ids.rs
macro_rules! id_type {
    ($name:ident) => {
        #[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub String);

        impl $name {
            pub fn new() -> Self { Self(uuid::Uuid::now_v7().to_string()) }
        }
    };
}

id_type!(SessionId);
id_type!(TurnId);
id_type!(StepId);
id_type!(ToolCallId);
id_type!(JobId);
```

- **Время:** `time::OffsetDateTime` (UTC) + монотонные `seq` для порядка событий.
- **Ошибки:** `thiserror` в библиотеках, `anyhow` только в бинаре; каждый домен — свой enum с кодом (`DshrError` + `code`).
- **Редакция:** `Redactor` вычищает токены/ключи/пути до записи в лог.

---

## 4. LLM-слой

### 4.1. Модель сообщений

```rust
pub struct Message {
    pub role: Role,                 // System | User | Assistant | Tool
    pub content: Vec<ContentBlock>,
}

pub enum ContentBlock {
    Text { text: String },
    Image { media_type: String, data: Vec<u8> },
    Reasoning { text: String },                  // reasoning-контент провайдера
    ToolCall { id: ToolCallId, name: String, args: serde_json::Value },
    ToolResult { call_id: ToolCallId, content: String, is_error: bool },
}
```

### 4.2. Стрим

```rust
pub enum StreamChunk {
    Start { model: String },
    TextDelta(String),
    ReasoningDelta(String),
    ToolCallStart { index: u32, id: String, name: String },
    ToolCallArgsDelta { index: u32, json_fragment: String },
    Usage(Usage),
    Done { stop_reason: StopReason },
}
```

Инвариант: адаптер отдаёт **единый** поток чанков независимо от формата провайдера. Всё остальное (сборка сообщения, tool-call JSON, ретраи) — общий `BlockAssembler`.

Маппинг DeepSeek/Anthropic Messages: `message_start` → `Start`; `content_block_delta.text_delta` → `TextDelta`; `thinking_delta` → `ReasoningDelta`; `content_block_start` с `tool_use` → `ToolCallStart`; `input_json_delta` → `ToolCallArgsDelta`; `ping` игнорируется; `message_delta/message_stop` → `Usage`/`Done`.

### 4.3. Контракт адаптера

```rust
#[async_trait]
pub trait LlmAdapter: Send + Sync {
    fn id(&self) -> &str;
    fn capabilities(&self) -> &Capabilities;   // tools, vision, reasoning, cache
    async fn stream(
        &self,
        request: Request,
        cancel: CancellationToken,
    ) -> Result<BoxStream<'static, Result<StreamChunk, LlmError>>, LlmError>;
}
```

- Аутентификация: два режима — account-токен продукта (`x-dsh-auth-token`, `POST /anthropic/v1/messages`) и API-key (`x-api-key`/Bearer). Каталог моделей — `GET /models` с тем же auth, TTL-кэш, fallback на встроенный список.
- Ретраи: только идемпотентные ошибки (429/5xx/обрыв до первого токена), экспоненциальный backoff с jitter, лимит.
- Отмена: `CancellationToken` + `select!`; после отмены не пишем частичное сообщение в журнал (только `assistant/attempt` при необходимости).
- `Usage`: prompt/completion/cached/reasoning токены — в событие для метрик и compaction.

---

## 5. Журнал сессий

### 5.1. События

```rust
#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SessionEvent {
    SessionCreated { seq: u64, at: OffsetDateTime, model: ModelSelection },
    TurnStart { seq: u64, turn: TurnId, at: OffsetDateTime },
    UserMessage { seq: u64, turn: TurnId, content: Vec<ContentBlock> },
    StepStart { seq: u64, turn: TurnId, step: StepId, request: RequestEnvelope },
    AssistantMessage { seq: u64, turn: TurnId, step: StepId, content: Vec<ContentBlock>, usage: Usage },
    AssistantAttempt { seq: u64, turn: TurnId, step: StepId, error: String, retryable: bool },
    ToolCall { seq: u64, turn: TurnId, step: StepId, call: ToolCall },
    ToolResult { seq: u64, turn: TurnId, step: StepId, result: ToolResult },
    Compaction { seq: u64, turn: TurnId, summary: String, replaced: Range<u64> },
    PermissionDecision { seq: u64, tool: String, decision: Decision, scope: Scope },
    TurnEnd { seq: u64, turn: TurnId, reason: TurnEndReason },
}
```

Wire-имена событий — `snake_case` (`turn_start`, `tool_result`); в тексте плана используется DSH-нотация `turn/start` для узнаваемости. Соответствие фиксируется ADR-001.

Правила:
- `seq` монотонен в рамках сессии; запись сериализуется актором.
- Событие неизменяемо после записи; исправления — новыми событиями.
- Модель видит только результат `derive_messages()` — чистой функции от журнала.

### 5.2. Хранилище

```text
~/.dshr/
├── config.yaml
├── profiles/<name>/profile.yaml
├── plugins/<name>/...
├── credentials/            # ссылки; значения — в keyring
├── sessions/<project-hash>/<session-id>/
│   ├── header.json         # версия, модель, cwd, created_at
│   └── session.v1.jsonl
└── index.sqlite            # FTS5, метаданные
```

- Запись: `O_APPEND`, батчи по 1–16 событий, `fsync` на границе turn (настраиваемо).
- Восстановление: обрезанный хвост отбрасывается, вставляется `turn/end{reason: interrupted}`.
- Версионирование: `session.vN.jsonl`; миграции — цепочка `vN → vN+1`, старые файлы не переименовываются и не удаляются.
- Индекс: SQLite (WAL) + FTS5 по тексту; перестраивается из журналов, не является источником истины.

### 5.3. Проекции

`Projection` — чистая свёртка событий: `messages`, `timeline`, `usage`, `todo`, `jobs`, `files_touched`. UI читает снимок проекции + поток новых событий.

---

## 6. Агентный цикл

### 6.1. Состояния

```text
Idle → Claiming → Assembling → Streaming → ExecutingTools → (Streaming | Stopping) → Idle
```

- **Turn** открывается первым входом и закрывается, когда не осталось работы.
- **Step** = один запрос к модели + его tool-calls.
- Входы (пользователь, субагент, расписание) попадают в **inbox**; обрабатываются по одному.

### 6.2. Псевдокод

```rust
pub async fn run_turn(&self, inputs: Vec<Input>) -> Result<TurnOutcome> {
    let turn = self.begin_turn()?;
    self.inbox.push(inputs);
    loop {
        let mut step = self.assemble_step().await?;      // prompt + admitted tool schemas
        let mut stream = self.llm.stream(step.request(), self.cancel.clone()).await?;
        let mut calls = Vec::new();
        while let Some(chunk) = stream.next().await {
            self.on_chunk(&mut step, chunk, &mut calls)?; // events + UI stream
        }
        self.commit_assistant(step, calls.is_empty())?;

        if calls.is_empty() {
            if self.inbox.is_empty() { break; }
            continue;
        }
        let results = self.tools.execute(calls, &self.policy).await; // bounded parallelism
        for r in results { self.commit_tool_result(r)?; }
        if self.step_count() >= self.limits.max_steps {
            return self.end_turn(TurnEndReason::MaxSteps);
        }
    }
    self.end_turn(TurnEndReason::Completed)
}
```

### 6.3. Отмена и ошибки

- Отмена пользователем → `turn/end{reason: cancelled}`; частичный ассистентский ответ сохраняется как `assistant/attempt`, не как сообщение.
- Ошибка инструмента → `tool/result{is_error: true}` и продолжение (модель решает).
- Ошибка провайдера после начала стрима → `assistant/attempt`, ретрай только по политике.
- Паника инструмента перехватывается (`catch_unwind` в границе задачи) и превращается в ошибку результата.

---

## 7. Инструменты

Реализация — `dshr-tools` + `dshr-agent`; принятое решение и границы — [ADR-014](adr/014-tool-approvals.md).

### 7.1. Контракт

```rust
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &'static str;
    fn description(&self) -> &str;
    fn schema(&self) -> schemars::Schema;              // JSON Schema для модели
    fn presentation(&self) -> Presentation;            // как рисовать в UI
    async fn execute(&self, ctx: &ToolCtx<'_>, args: serde_json::Value)
        -> Result<ToolOutcome, ToolError>;
}

pub struct ToolOutcome {
    pub content: String,       // то, что увидит модель
    pub is_error: bool,
    pub spill: Option<SpillRef>, // большой вывод — в файл, в контекст — ссылка
    pub files: Vec<FileEffect>,  // какие файлы изменены (для diff/аудита)
}
```

### 7.2. Пайплайн

```text
tool/call → schema validate → approval policy → sandbox policy
          → execute (timeout, cancel) → truncate/spill → tool/result
```

- **Валидация:** схема + семантические проверки (пути внутри workspace, запрет NUL, лимит размера).
- **Параллелизм:** семафор (по умолчанию 8), независимые вызовы; инструменты с общим ресурсом (shell, git) — через мьютекс.
- **Лимиты:** таймаут на вызов, максимум вывода в контекст (~30k символов), остальное — spill-файл с `SpillRef`.
- **Регистрация:** scoped — инструмент виден только агенту/сессии, где зарегистрирован (аналог `ctx.tools` + scope).

### 7.3. Approvals

```rust
pub enum PermissionMode { ReadOnly, WorkspaceWrite, DangerFull }

pub struct PermissionPolicy {
    pub mode: PermissionMode,
    pub rules: Vec<Rule>,          // tool/pattern → allow|deny|ask
    pub remember: RememberScope,   // session | project | command
}
```

- Решение фиксируется событием `permission/decision` (аудит).
- «Запомнить» ограничено областью; сброс — командой.
- Неизвестное правило → `ask` → при отсутствии ответчика `deny` (fail-closed).

---

## 8. Промпт и контекст

Сборка секций (стабильный префикс → кэш провайдера):

1. Идентичность и правила поведения.
2. Окружение: cwd, ОС, git-статус, дата, модель.
3. `AGENTS.md` (глобальный + проектный), иерархия.
4. Каталог skills (только имя + описание).
5. Допущенные схемы инструментов (по режиму/политике).
6. Инжектированный контекст (напоминания, заметки, goal).

Инварианты:
- Секции детерминированы и версионируются; golden-тесты на сборку.
- Схемы инструментов не меняются между шагами без причины (иначе ломается кэш).
- Runtime-напоминания добавляются как отдельные секции с записью в журнал.

---

## 9. Compaction

- Триггер: `usage > threshold` (по умолчанию 80 % окна) или число сообщений.
- Счётчик токенов: `usage` провайдера после ответа + локальная оценка до запроса (`tiktoken-rs`/`tokenizers`); выбор — ADR-008.
- Стратегия: суммаризация старых turn'ов через LLM + сохранение последних N сообщений; tool-результаты > лимита заменяются на `SpillRef`-ссылки.
- Результат: событие `compaction` с диапазоном заменённых `seq` и текстом summary; `derive_messages()` учитывает.
- Отказоустойчивость: ошибка суммаризации не ломает сессию — fallback на pruning.
- Настройки: порог, keep-last, модель суммаризации, максимум summary-токенов.

---

## 10. MCP

- Клиент на базе `rmcp` (официальный Rust SDK): stdio и Streamable HTTP.
- Namespacing: `mcp__<server>__<tool>`; конфликты — ошибка конфигурации.
- Область: сервер может быть доступен всем сессиям или одной (scoped).
- Ресурсы и промпты — отдельные инструменты/секции; инструкции сервера инжектируются в промпт.
- Отказоустойчивость: таймауты, reconnect с backoff, деградация без падения сессии.
- Безопасность: MCP-сервер — недоверенный код; его вывод санитизируется, размер ограничен.

---

## 11. Субагенты

```rust
pub trait SubagentProvider: Send + Sync {
    async fn start(&self, req: SubagentStartRequest, parent: &AgentHandle)
        -> Result<SubagentHandle, SubagentError>;
}
```

- **spawn:** новый контекст, своя сессия, родитель видит только финальный результат.
- **fork:** наследование журнала до границы turn (сессия-потомок с seed).
- Дерево: `parent_id`, глубина, лимит; сообщения между агентами; ожидание/отмена.
- Бюджеты: токены и время на поддерево; превышение → остановка с отчётом.

---

## 12. Песочница

### 12.1. Политика

```rust
pub struct SandboxPolicy {
    pub fs: FsPolicy,          // ReadOnly{roots} | WorkspaceWrite{roots, deny} | DangerFull
    pub network: NetworkMode,  // None | Loopback | AllowAll | AllowList(Vec<String>)
    pub exec: ExecMode,        // Deny | AllowList | AllowAll
    pub env_allow: Vec<String>,
}
```

### 12.2. Бэкенды

| ОС | Механизм | Статус |
|---|---|---|
| macOS | Seatbelt-профиль (`sandbox-exec`) с генерацией правил из политики | T2 |
| Linux | bwrap + Landlock (fs); seccomp-bpf — расширение dshr | T2 |
| Windows | ACL restricted token + Job Object (best effort) | T3 |
| Универсальный fallback | Контейнер/VM-провайдер (Docker/Podman) | T3 |

DSH ограничивает политикой только файловые эффекты (`SandboxMode`: `read-only` / `workspace-write` / `danger-full-access`); сеть и видимость процессов — вне этого словаря и являются расширением dshr (ADR-009). Enforcement в DSH бывает `full` и `partial`; мы обязаны сообщать уровень так же и не выдавать `partial` за полную гарантию.

- Обёртка: `ConfinedArgv` — argv + env + политика; провайдер обязан вернуть ошибку, если не может применить политику.
- Проверка перед запуском: путь бинаря, cwd, доступность backend.
- Аудит: каждое ограничение и отказ — событие/лог.

---

## 13. Плагины: один шов, четыре рантайма

Всё расширяемое снаружи идёт через один трейт и один жизненный цикл; различается только исполнение. Решение о совместимости — [ADR-013](adr/013-cordis-compat.md).

| Рантайм | Исполнение | Доверие | Артефакт |
|---|---|---|---|
| `native` | in-process, компилируемый крейт | полное | бинарь + строка композиции |
| `wasm` | wasmtime, Component Model, WIT | ограниченное (capabilities) | `.wasm` + `plugin.toml` |
| `cordis` | настоящий `cordis` в JS-рантайме (sidecar-процесс по умолчанию, embedded — опция) | как в DSH, но builtins опосредованы | npm-пакет + lock |
| `external` | sidecar JSON-RPC / MCP | процесс в песочнице | процесс + манифест |

```rust
#[async_trait]
pub trait PluginRuntime: Send + Sync {
    async fn load(&self, manifest: &PluginManifest) -> Result<LoadedPlugin>;
    async fn activate(&self, plugin: &LoadedPlugin, caps: &CapabilitySet) -> Result<()>;
    async fn replace(&self, plugin: &LoadedPlugin, next: ArtifactRef) -> Result<Replaced>;
    async fn stop(&self, plugin: &LoadedPlugin) -> Result<()>;
}
```

Общее для всех тиров: манифест (`plugin.toml` или `package.json`), capability-модель, health-check перед переключением, откат версии, единый реестр вкладов (инструменты, хуки, команды, слоты UI) и единый аудит. Вызов между тирами — только через host-посредник.

### 13.1. WASM-компоненты

- `wasmtime` + Component Model, WIT; `wasmtime-wasi` с урезанными правами: preopens только `/data`, `/cache`, `/tmp` (с TTL), сеть не пробрасывается.
- Один host-импорт — диспетчер `call(op, payload)`: новая возможность добавляется операцией, а не новым импортом (обоснование — §13.4).
- Манифест `plugin.toml`: имя, версия, `api_version`, capabilities (`fs.read`, `fs.write`, `net`, `exec`), точки расширения.
- Лимиты по умолчанию: 64 МиБ памяти на инстанс (конфигурируемо), fuel/epoch на вызов, ограничение размера входа/выхода; изоляция паник.
- Проверка целостности при загрузке: хеш/подпись не сошлись — плагин не активируется.
- Горячая замена: загрузка новой версии рядом, health-check, переключение, откат.
- Хуки: `on_session_start`, `on_tool_call`, `on_prompt`, `command`.

```wit
package dshr:plugin@1.0.0;

interface types {
  record outcome { content: string, is-error: bool }
}

interface host {
  /// Единственный импорт: операция + payload, ответ — JSON-строка.
  call: func(op: string, payload: string) -> result<string, string>;
}

interface tool {
  use types.{outcome};
  execute: func(args-json: string) -> result<outcome, string>;
}

world tool-plugin {
  import host;
  export tool;
}
```

### 13.2. Cordis-совместимость (JS-рантайм)

- Исполняем реальный `cordis` (MIT, вендорим версию) и реальные npm-пакеты — не эмулируем API.
- `cordis-sidecar` (по умолчанию): процесс-рантайм из runtime pack, общение по JSON-RPC, живёт в песочнице. `cordis-embedded` (опция): V8/`deno_core` в отдельном изоляте, «один бинарь» ценой размера и старта.
- Builtins: `path`/`url`/`events`/`util`/`buffer` — чистый JS; `fs` → `ctx.fs`; `child_process` → exec + approvals; `net` → deny by default; `process` → фильтрованный env; `crypto` → host; нативные `.node`-аддоны не грузятся.
- `@dshr/cordis-compat@1`: сервисы `ctx.*` поверх host-RPC, привязка к API DSH `0.2.x`, матрица поддержки и conformance-набор из реальных плагинов.
- Client-половина: бандл рендерится в изолированном iframe и общается с Leptos-шеллом через типизированный `postMessage`-мост; полного React-паритета слотов нет.

### 13.3. Sidecar

Для плагинов, которым нужен произвольный рантайм: процесс + JSON-RPC 2.0 (stdio), тот же манифест и capability-модель. Процесс живёт в песочнице, общение — по протоколу. Используется и как обходной путь для плагинов с нативными зависимостями.

### 13.4. Соседние проекты: что перенимаем

Сверка модели с production-примерами: у каждого берём только то, что усиливает нас, и отдельно фиксируем, чего не копируем.

| Проект | Что там плагин | ABI / рантайм | Что перенимаем |
|---|---|---|---|
| [Zellij](https://github.com/zellij-org/zellij) | панели и статус внутри терминального мультиплексора | `wasm32-wasip1`, wasmi + WASI | один диспетчерский импорт `host.call`, `PermissionType` с кэшем грантов, preopens `/data`+`/cache`+`/tmp`, лимит памяти на инстанс (в 0.44 — 16 МиБ) |
| [Zed](https://github.com/zed-industries/zed) | языки, темы, LSP-адаптеры | `wasm32-wasip2`, `zed_extension_api` | `api_version` в манифесте, матрица совместимости, SDK-крейт с semver |
| [dprint](https://github.com/dprint/dprint) | форматтеры | wasm-тир + process-тир | явное разделение «песочница» и «нужен свой рантайм» вместо попытки всё загнать в wasm |
| [swc](https://github.com/swc-project/swc) | трансформеры AST | wasm, wasmer/[wasmtime](https://github.com/swc-project/swc/pull/10958) | раннер как сменный бэкенд, а не часть ядра |
| [Envoy](https://github.com/envoyproxy/envoy)/[Kong](https://konghq.com/blog/product-releases/webassembly-in-kong-gateway-3-4)/[APISIX](https://github.com/apache/apisix)/[NGINX](https://github.com/GetPageSpeed/ngx_wasm_module) | HTTP-фильтры | [proxy-wasm](https://github.com/proxy-wasm/spec) | «один ABI — много хостов»: нужен шов адаптеров, чтобы чужой ABI можно было подключить без правки ядра |
| [Traefik](https://github.com/traefik/plugindemowasm) | middlewares | http-wasm | проверка целостности при загрузке: не прошёл валидацию — плагин не стартует |
| [Redpanda](https://docs.redpanda.com/current/develop/data-transforms/run-transforms/)/[Benthos](https://github.com/redpanda-data/benthos) | обработка потока сообщений | wasm + wazero, PDK | PDK-подход для языков, где WIT-биндингов ещё нет |
| [Shopify Functions](https://github.com/Shopify/shopify-function-wasm-api) | логика checkout | собственный wasm API + trampoline | явная сериализация границ входа/выхода, без разделяемой памяти |
| [Wassette](https://github.com/microsoft/wassette)/[hyper-mcp](https://github.com/cicero-im/hyper-mcp) | MCP-инструменты | Wasmtime/Extism + OCI | `oci://` как первоклассная ссылка на артефакт, deny-by-default права |
| [Extism](https://github.com/extism/extism) | фреймворк плагинных систем | PDK на 10+ языках | host-controlled HTTP без WASI, лимитеры и таймеры как часть контракта, а не рантайма |

**Что из этого становится нашим решением:**

1. **Один импорт вместо N.** WIT-мир получает единственный host-импорт `call(op, payload)`; новая возможность — это новая операция в типизированном enum, а не новый импорт. Экспорт инструмента остаётся типизированным (§13.1).
2. **Кэш грантов.** `PermissionGrant { capability, scope, decided_at, source }` хранится в профиле; каждое использование гранта — событие аудита. Встроенные плагины проходят проверку так же, как сторонние — в отличие от Zellij, где builtin её обходит.
3. **Лимиты по умолчанию, а не «настроим потом».** Память, fuel/epoch на вызов, потолок входа/выхода, preopens только `/data` и `/cache`.
4. **Версия API в манифесте.** `api_version` + матрица «плагин × версия хоста» + SDK-крейт с semver; ломающее изменение = новая мажорная версия мира.
5. **Сменный рантайм-бэкенд.** `wasmtime` по умолчанию, `wasmi` — опция для холодного старта и слабых машин. Zellij прошёл wasmer → wasmtime → wasmi и заплатил за это миграциями; у нас бэкенд выбирается конфигом.
6. **`oci://` наравне с локальным путём.** CAS, подпись и откат остаются, меняется только адресация.
7. **Шов для чужих ABI.** Host-функции спрятаны за `host_api`; адаптеры proxy-wasm / http-wasm / Extism PDK возможны без правки ядра. Сейчас не делаем — но архитектура это допускает.
8. **PDK — опция, не основа.** Первым идёт свой Rust SDK на WIT; языки без готовых биндингов идут в process-тир (как у dprint), PDK-слой — если появится спрос.

**Чего не копируем:** наследование env в WASI-контексте; единственный синхронный диспетчер как обязательный путь (у нас async и батчинг); protobuf поверх пайпов (у нас WIT/canonical ABI); загрузку плагинов только на старте (нам нужна горячая замена — берём только integrity-check); дистрибуцию через Git-репозитории (у нас CAS/OCI); собственный нестандартный ABI с trampoline.

### 13.5. Чего не делаем

- Не грузим нативные `.so`/`.dylib` (ABI Rust нестабилен, безопасность нулевая).
- Не даём плагинам ambient authority: только через host-функции — включая Cordis-тир, где `fs`/`net`/`exec` опосредованы, а не отданы как в Node.
- Не обещаем полной совместимости клиентских UI-плагинов DSH: мост слотов, не React-совместимость шелла.

---

## 14. Конфигурация и композиция

Слои (снизу вверх):

```text
defaults → profile (набор бандлов) → user config.yaml → --patch → env/CLI
```

- Формат: YAML, валидация JSON Schema (`schemars`), ошибки с путём до поля.
- Профиль: список бандлов + патчи; бандл — каталог с `bundle.yaml` и плагинами.
- `dshr --dump-config` печатает итоговое дерево; `dshr plugin add|remove|list` управляет установкой.
- Lock-файл фиксирует версии; обновление — явное.

---

## 15. Сервер и SDK

### 15.1. API

| Метод | Путь | Назначение |
|---|---|---|
| POST | `/v1/sessions` | создать сессию |
| GET | `/v1/sessions` | список (пагинация, фильтры) |
| GET | `/v1/sessions/{id}` | заголовок + проекция |
| POST | `/v1/sessions/{id}/turns` | отправить вход |
| GET | `/v1/sessions/{id}/events` | SSE-поток событий |
| POST | `/v1/sessions/{id}/approvals/{req}` | ответ на approval |
| GET | `/v1/sessions/{id}/files` | изменённые файлы |
| POST | `/v1/sessions/{id}/cancel` | отмена turn |
| GET | `/v1/health` | health/ready |

- Аутентификация: bearer-токен, генерируется при старте/настройке; CORS для localhost.
- Стрим: SSE для событий, WS для интерактивных сессий (опционально).
- OpenAPI генерируется из кода и проверяется в CI.

### 15.2. SDK

JSON-RPC 2.0 поверх stdio (для встраивания) и HTTP-клиент. Python/TS-биндинги — тонкие обёртки; протокол версионируется.

---

## 16. UI

### 16.1. TUI (T1)

- `ratatui` + `crossterm`; раскладка: чат, лента инструментов, статус (модель, токены, стоимость), модалки approval, панель jobs, diff-view.
- Стрим-рендер с батчингом кадров (≤ 60 fps), markdown-рендер (`pulldown-cmark` + `syntect`).
- Клавиши: `Enter` отправить, `Esc` отменить, `Ctrl-C` дважды — выход, `Tab` — панели.

### 16.2. Web (T2)

- Бэкенд: `axum` + статика; фронтенд — Leptos (ADR-003).
- Компоненты: чат, карточки вызовов с diff, approval-модалка, список сессий, настройки провайдеров.
- Переподключение: snapshot проекции + события с `seq`, идемпотентность.

### 16.3. Desktop (T3)

Тонкая Tauri-оболочка над `dshr serve` (loopback HTTP/WS), а не второй движок: окно, трей, lifecycle, автообновление, подпись. Это прямой перенос идеи DSH Desktop «host отдельно, оболочка тонкая», но без Electron/Chromium: используется системный webview. Web UI — тот же, что в M6; desktop не добавляет второй протокол и не отдаёт нативные API в страницу.

---

## 17. Наблюдаемость

- `tracing` с полями `session_id`, `turn_id`, `step_id`, `tool_call_id`, `job_id`.
- OTLP-экспорт (traces + metrics) опционально.
- Метрики: TTFT, длительность шага, длительность инструмента, токены, ошибки по классам, размер контекста.
- Аудит: отдельный лог решений (approvals, sandbox denials, plugin grants).

---

## 18. Маппинг DSH → Rust

| DSH-подсистема | Роль | Rust-крейт | Тир |
|---|---|---|---|
| `core/session` | журнал и проекции | `dshr-session` | T1 |
| `core/agent-loop` | цикл turn/step | `dshr-agent` | T1 |
| `core/tools` | реестр и пайплайн | `dshr-tools` | T1 |
| `core/system-prompt` | сборка промпта | `dshr-prompt` | T1 |
| `llm/llm`, `llm-deepseek` | адаптеры | `dshr-llm`, `dshr-llm-*` | T1 |
| `tool-fs`, `tool-fs-search` | файлы/поиск | `dshr-tool-fs` | T1 |
| `tool-bash` | shell | `dshr-tool-shell` | T1 |
| `terminal` | persistent PTY | `dshr-terminal` | T3 |
| `tool-ask-user`, `tool-todo`, `tool-present` | UX-инструменты | `dshr-tool-basic` | T1 |
| `web` (search/fetch) | веб-доступ | `dshr-tool-web` | T1 |
| `session-persistence-jsonl` | хранение | `dshr-session-jsonl` | T1 |
| `session-query` | поиск | `dshr-session-index` | T1 |
| `mcp-client`, `mcp-resources` | MCP | `dshr-mcp` | T1 |
| `approval`, `permission-presets` | права | `dshr-agent` + `dshr-tools` | T1 |
| `sandbox-local`, `sandbox-policy` | песочница | `dshr-sandbox` | T2 |
| `subagent`, `tool-subagent` | субагенты | `dshr-subagent` | T1 |
| `jobs-local`, `tool-jobs` | фоновые задачи | `dshr-jobs` | T1 |
| `compaction-*` | сжатие | `dshr-compaction` | T1 |
| `skill`, `skill-filesystem` | навыки | `dshr-skills` | T1 |
| `plugin-manager`, `extensions` | плагины | `dshr-plugin` | T2 |
| `settings`, `credentials-local` | настройки/ключи | `dshr-config` | T1 |
| `schedule`, `tool-schedule` | расписания | `dshr-schedule` | T3 |
| `workflow-ptc`, `tool-workflow` | workflows | `dshr-workflow` | T3 |
| `goal`, `tool-goal` | цели | `dshr-goal` | T3 |
| `agent-team` | команды | `dshr-agent-team` | T3 |
| `web-app`, `client-ui-*` | веб | `dshr-server` + UI | T2 |
| `acp-app`, `sdk-app` | интеграции | `dshr-sdk` | T3 |
| `worktrees`, `forge` | git | `dshr-worktree` | T3 |
| `lsp`, `office-kit`, `cua` | LSP/Office/CUA | отдельные крейты | T3 |
| `otel`, `telemetry` | телеметрия | `dshr-telemetry` | T3 |

---

## 19. ADR (решения по умолчанию)

| ADR | Решение | Обоснование | Последствия |
|---|---|---|---|
| ADR-001 | Свой формат сессий (`session.v1.jsonl`; zstd — T2), импорт DSH — адаптер | Контроль эволюции, независимость от 0.2.x | Нет прямой совместимости «из коробки» |
| ADR-002 | WASM-плагины + sidecar | Безопасность, изоляция, языковая нейтральность | Cordis/npm — отдельным рантаймом, см. ADR-013 |
| ADR-003 | TUI сразу, Web — Leptos | Mono-Rust, общие типы, меньше рассинхрона | Меньше готовых UI-компонентов |
| ADR-004 | JSONL — истина, SQLite — индекс | Простой формат, восстановимость, FTS | Дублирование данных, перестройка индекса |
| ADR-005 | tokio + mpsc-актор журнала | Один писатель, предсказуемость | Актор — точка сериализации |
| ADR-006 | `schemars` + `jsonschema` | Единая схема для модели и валидации | Схемы нужно держать стабильными |
| ADR-007 | Имя `dshr` (рабочее), MIT/Apache-2.0 | Юридическая чистота | Переименование до публикации |
| ADR-008 | Счётчик токенов: `usage` + локальная оценка | Порог compaction нужен до первого ответа | Два источника, нужна калибровка |
| ADR-009 | `network`/`exec` в песочнице — расширение сверх файлового `SandboxMode` | DSH не описывает сеть/процессы | Больше поверхность атаки, нужна документация |
| ADR-010 | DeepSeek official — Anthropic Messages wire, два режима auth | Подтверждено живой проверкой 12.11.2026 | Нужен Anthropic-совместимый транспорт, не только OpenAI |
| ADR-011 | Desktop — Tauri над `dshr serve`, без Electron | Тонкая оболочка, один протокол, меньше размер | Зависимость от системного webview, различия ОС |
| ADR-012 | YAML-парсер — `serde-saphyr` 2.1 | Убрать `unsafe-libyaml` (RUSTSEC-2024-0449), безопасный Rust-парсер | MSRV 1.89, точные ошибки парсинга |
| ADR-013 | Cordis-совместимость рядом с WASM | Сохранить экосистему npm и HMR | Runtime pack, дрейф API, мост UI вместо React-паритета |

Каждый ADR на этапе M0 оформляется отдельным файлом `docs/adr/NNN-*.md` по шаблону: контекст → варианты → решение → последствия.

---

## 20. Производительность

| Метрика | Бюджет |
|---|---|
| Оверхед harness на turn (без LLM) | < 50 мс p95 |
| Сборка промпта (100 инструментов) | < 10 мс |
| Запись события (fsync off / on) | < 0.1 мс / < 5 мс |
| Поиск по сессиям (10k событий) | < 100 мс |
| Память на сессию (100k токенов) | < 200 МБ |

Контроль: `criterion`-бенчи в CI (ночной прогон), регрессия > 20 % — блокер.

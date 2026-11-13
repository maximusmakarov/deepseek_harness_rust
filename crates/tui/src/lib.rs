#![forbid(unsafe_code)]
//! Minimal terminal UI: chat transcript, streaming text, tool cards, and an
//! approval modal. The agent runs on a background thread with its own Tokio
//! runtime; the UI thread only renders and forwards keys.

use std::io::{self, Stdout};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use dshr_agent::{
    Agent, AgentConfig, AgentError, AgentObserver, ApprovalPolicy, ApprovalRequest, Approver,
    PermissionMode, ToolCallInfo,
};
use dshr_core::{DshrError, Result};
use dshr_i18n::Lang;
use dshr_llm::{LlmProvider, Usage};
use dshr_session::Session;
use dshr_session_jsonl::SessionStore;
use dshr_tools::{ToolOutcome, ToolRegistry};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;

pub struct TuiOptions {
    pub provider: Arc<dyn LlmProvider>,
    pub tools: Arc<ToolRegistry>,
    pub store: Arc<SessionStore>,
    pub session: Session,
    pub system_prompt: String,
    pub max_steps: usize,
    pub max_tokens: u32,
    pub output_limit: usize,
    pub mode: PermissionMode,
    pub lang: Lang,
}

enum Command {
    Prompt(String),
    Quit,
}

enum UiEvent {
    Text(String),
    ToolStart {
        name: String,
        summary: String,
    },
    ToolEnd {
        name: String,
        ok: bool,
    },
    Done {
        usage: Usage,
        steps: usize,
        tool_calls: usize,
    },
    Cancelled,
    Error(String),
    Approval {
        request: ApprovalRequest,
        respond: oneshot::Sender<bool>,
    },
    Question {
        question: String,
        options: Vec<String>,
        respond: oneshot::Sender<Option<String>>,
    },
}

struct ChannelApprover {
    events: Sender<UiEvent>,
}

#[async_trait::async_trait]
impl Approver for ChannelApprover {
    async fn approve(&self, request: ApprovalRequest) -> bool {
        let (respond, answer) = oneshot::channel();
        if self
            .events
            .send(UiEvent::Approval { request, respond })
            .is_err()
        {
            return false;
        }
        answer.await.unwrap_or(false)
    }
}

struct TuiObserver {
    events: Sender<UiEvent>,
}

#[async_trait::async_trait]
impl AgentObserver for TuiObserver {
    async fn on_text(&self, text: &str) {
        let _ = self.events.send(UiEvent::Text(text.to_owned()));
    }

    async fn on_tool_start(&self, call: &ToolCallInfo) {
        let _ = self.events.send(UiEvent::ToolStart {
            name: call.name.clone(),
            summary: summarize(&call.input),
        });
    }

    async fn on_tool_end(&self, call: &ToolCallInfo, outcome: &ToolOutcome) {
        let _ = self.events.send(UiEvent::ToolEnd {
            name: call.name.clone(),
            ok: !outcome.is_error,
        });
    }

    async fn on_question(&self, question: &str, options: &[String]) -> Option<String> {
        let (respond, answer) = oneshot::channel();
        if self
            .events
            .send(UiEvent::Question {
                question: question.to_owned(),
                options: options.to_vec(),
                respond,
            })
            .is_err()
        {
            return None;
        }
        answer.await.ok().flatten()
    }
}

fn summarize(value: &serde_json::Value) -> String {
    let text = value.to_string();
    let mut end = text.len().min(80);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}{}", &text[..end], if text.len() > 80 { "…" } else { "" })
}

pub fn run(options: TuiOptions) -> Result<()> {
    let (event_tx, event_rx) = mpsc::channel();
    let (command_tx, command_rx) = mpsc::channel();
    let cancel = Arc::new(Mutex::new(None::<CancellationToken>));
    let workspace = PathBuf::from(&options.session.header.cwd);
    let model = options.session.header.model.clone();
    let session_id = options.session.header.id.to_string();
    let lang = options.lang;

    spawn_agent(options, command_rx, event_tx, cancel.clone(), workspace);

    let mut terminal = enter_terminal()?;
    let guard = RawModeGuard;
    let mut app = App::new(model, session_id, lang);

    let result = event_loop(&mut terminal, &mut app, &event_rx, &command_tx, &cancel);
    drop(guard);
    leave_terminal(&mut terminal)?;
    result
}

fn spawn_agent(
    options: TuiOptions,
    commands: Receiver<Command>,
    events: Sender<UiEvent>,
    cancel: Arc<Mutex<Option<CancellationToken>>>,
    workspace: PathBuf,
) {
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");
        runtime.block_on(async move {
            let approver: Arc<dyn Approver> = Arc::new(ChannelApprover {
                events: events.clone(),
            });
            let policy = ApprovalPolicy {
                mode: options.mode,
                workspace,
                approver,
            };
            let mut agent = Agent::new(
                options.provider,
                options.tools,
                options.store,
                options.session,
                policy,
                AgentConfig {
                    system_prompt: options.system_prompt,
                    max_steps: options.max_steps,
                    max_tokens: options.max_tokens,
                    output_limit: options.output_limit,
                    compaction: Some(dshr_compaction::CompactionConfig::default()),
                    budget: None,
                },
            );

            while let Ok(command) = commands.recv() {
                match command {
                    Command::Quit => break,
                    Command::Prompt(prompt) => {
                        let token = CancellationToken::new();
                        if let Ok(mut slot) = cancel.lock() {
                            *slot = Some(token.clone());
                        }
                        let observer = TuiObserver {
                            events: events.clone(),
                        };
                        let outcome = agent.run_turn(&prompt, token, &observer).await;
                        if let Ok(mut slot) = cancel.lock() {
                            *slot = None;
                        }
                        let event = match outcome {
                            Ok(report) => UiEvent::Done {
                                usage: report.usage,
                                steps: report.steps,
                                tool_calls: report.tool_calls,
                            },
                            Err(AgentError::Cancelled) => UiEvent::Cancelled,
                            Err(error) => UiEvent::Error(error.to_string()),
                        };
                        let _ = events.send(event);
                    }
                }
            }
        });
    });
}

struct PendingQuestion {
    question: String,
    options: Vec<String>,
    answer: String,
    respond: oneshot::Sender<Option<String>>,
}

struct App {
    lines: Vec<String>,
    stream: String,
    input: String,
    status: String,
    busy: bool,
    quit: bool,
    approval: Option<(ApprovalRequest, oneshot::Sender<bool>)>,
    question: Option<PendingQuestion>,
    model: String,
    session: String,
    lang: Lang,
}

impl App {
    fn new(model: String, session: String, lang: Lang) -> Self {
        Self {
            lines: vec!["dshr".to_owned(), lang.hint().to_owned(), String::new()],
            stream: String::new(),
            input: String::new(),
            status: String::new(),
            busy: false,
            quit: false,
            approval: None,
            question: None,
            model,
            session,
            lang,
        }
    }

    fn apply(&mut self, event: UiEvent) {
        match event {
            UiEvent::Text(text) => self.stream.push_str(&text),
            UiEvent::ToolStart { name, summary } => {
                self.flush_stream();
                self.lines.push(format!("→ {name} {summary}"));
            }
            UiEvent::ToolEnd { name, ok } => {
                self.lines
                    .push(format!("← {name} {}", if ok { "ok" } else { "error" }));
            }
            UiEvent::Done {
                usage,
                steps,
                tool_calls,
            } => {
                self.flush_stream();
                self.lines.push(self.lang.done(
                    usage.input_tokens,
                    usage.output_tokens,
                    steps,
                    tool_calls,
                ));
                self.status.clear();
                self.busy = false;
            }
            UiEvent::Cancelled => {
                self.flush_stream();
                self.lines.push("[cancelled]".to_owned());
                self.busy = false;
            }
            UiEvent::Error(error) => {
                self.flush_stream();
                self.lines.push(format!("[error] {error}"));
                self.busy = false;
            }
            UiEvent::Approval { request, respond } => {
                self.approval = Some((request, respond));
            }
            UiEvent::Question {
                question,
                options,
                respond,
            } => {
                self.flush_stream();
                self.question = Some(PendingQuestion {
                    question,
                    options,
                    answer: String::new(),
                    respond,
                });
            }
        }
    }

    fn flush_stream(&mut self) {
        if !self.stream.is_empty() {
            self.lines.push(std::mem::take(&mut self.stream));
            self.lines.push(String::new());
        }
    }

    fn submit(&mut self, commands: &Sender<Command>) {
        let prompt = self.input.trim().to_owned();
        if prompt.is_empty() || self.busy {
            return;
        }
        self.lines.push(format!("> {prompt}"));
        self.input.clear();
        self.busy = true;
        self.status = self.lang.thinking().to_owned();
        let _ = commands.send(Command::Prompt(prompt));
    }

    fn handle_key(
        &mut self,
        key: KeyEvent,
        commands: &Sender<Command>,
        cancel: &Mutex<Option<CancellationToken>>,
    ) {
        if key.kind != KeyEventKind::Press {
            return;
        }

        if let Some(mut pending) = self.question.take() {
            match key.code {
                KeyCode::Esc => {
                    let _ = pending.respond.send(None);
                }
                KeyCode::Enter => {
                    let _ = pending.respond.send(Some(pending.answer.trim().to_owned()));
                }
                KeyCode::Backspace => {
                    pending.answer.pop();
                    self.question = Some(pending);
                }
                KeyCode::Char(character)
                    if pending.options.is_empty() || !character.is_ascii_digit() =>
                {
                    pending.answer.push(character);
                    self.question = Some(pending);
                }
                KeyCode::Char(character) if character.is_ascii_digit() => {
                    let index = character.to_digit(10).unwrap_or(0) as usize;
                    if index >= 1 && index <= pending.options.len() {
                        let _ = pending
                            .respond
                            .send(Some(pending.options[index - 1].clone()));
                    } else {
                        self.question = Some(pending);
                    }
                }
                _ => {
                    self.question = Some(pending);
                }
            }
            return;
        }

        if let Some((_, respond)) = self.approval.take() {
            match key.code {
                KeyCode::Char('y' | 'Y') => {
                    let _ = respond.send(true);
                }
                KeyCode::Char('n' | 'N') | KeyCode::Esc => {
                    let _ = respond.send(false);
                }
                KeyCode::Char('c')
                    if key
                        .modifiers
                        .contains(crossterm::event::KeyModifiers::CONTROL) =>
                {
                    let _ = respond.send(false);
                    self.quit = true;
                    let _ = commands.send(Command::Quit);
                }
                _ => {
                    // Keep waiting; put the request back is not possible without
                    // storing it, so deny on any other key.
                }
            }
            return;
        }

        match key.code {
            KeyCode::Char('c')
                if key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL) =>
            {
                if let Ok(slot) = cancel.lock()
                    && let Some(token) = slot.as_ref()
                {
                    token.cancel();
                }
                self.quit = true;
                let _ = commands.send(Command::Quit);
            }
            KeyCode::Esc => {
                if self.busy {
                    if let Ok(slot) = cancel.lock()
                        && let Some(token) = slot.as_ref()
                    {
                        token.cancel();
                    }
                } else {
                    self.input.clear();
                }
            }
            KeyCode::Enter => self.submit(commands),
            KeyCode::Backspace => {
                self.input.pop();
            }
            KeyCode::Char(character) => self.input.push(character),
            _ => {}
        }
    }

    fn render(&self, frame: &mut ratatui::Frame<'_>) {
        let area = frame.area();
        let chunks = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(3),
            Constraint::Length(2),
        ])
        .split(area);

        let header = format!(
            "dshr · {} · {}",
            self.model,
            &self.session[..self.session.len().min(12)]
        );
        frame.render_widget(
            Paragraph::new(header).style(Style::default().fg(Color::Cyan)),
            chunks[0],
        );

        let mut body = self.lines.clone();
        if !self.stream.is_empty() {
            body.push(self.stream.clone());
        }
        let visible = chunks[1].height.saturating_sub(2) as usize;
        let scroll = body.len().saturating_sub(visible) as u16;
        frame.render_widget(
            Paragraph::new(body.join("\n"))
                .block(Block::default().borders(Borders::ALL))
                .wrap(Wrap { trim: false })
                .scroll((scroll, 0)),
            chunks[1],
        );

        let prompt = if self.busy {
            format!("{} {}", self.status, self.input)
        } else {
            format!("> {}", self.input)
        };
        frame.render_widget(
            Paragraph::new(prompt).style(Style::default().fg(Color::Yellow)),
            chunks[2],
        );

        if let Some(pending) = &self.question {
            let popup = centered(area, 70, 9);
            frame.render_widget(Clear, popup);
            let mut text = format!("{}\n\n", pending.question);
            for (index, option) in pending.options.iter().enumerate() {
                text.push_str(&format!("{}. {option}\n", index + 1));
            }
            text.push_str(&format!("\n> {}", pending.answer));
            frame.render_widget(
                Paragraph::new(text)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title("question: Enter / Esc"),
                    )
                    .wrap(Wrap { trim: true }),
                popup,
            );
        }

        if let Some((request, _)) = &self.approval {
            let popup = centered(area, 70, 7);
            frame.render_widget(Clear, popup);
            let text = self
                .lang
                .approval(&request.tool, &request.summary, &request.reason);
            frame.render_widget(
                Paragraph::new(text)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title("approval: y / n"),
                    )
                    .wrap(Wrap { trim: true })
                    .style(
                        Style::default()
                            .fg(Color::Black)
                            .bg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ),
                popup,
            );
        }
    }
}

fn centered(area: Rect, percent_x: u16, height: u16) -> Rect {
    let width = area.width * percent_x / 100;
    let x = area.x + (area.width - width) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width, height.min(area.height))
}

fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut App,
    events: &Receiver<UiEvent>,
    commands: &Sender<Command>,
    cancel: &Mutex<Option<CancellationToken>>,
) -> Result<()> {
    loop {
        while let Ok(event) = events.try_recv() {
            app.apply(event);
        }
        terminal
            .draw(|frame| app.render(frame))
            .map_err(|error| DshrError::config(format!("draw: {error}")))?;

        if app.quit {
            break;
        }
        if event::poll(Duration::from_millis(50)).map_err(DshrError::Io)?
            && let Event::Key(key) = event::read().map_err(DshrError::Io)?
        {
            app.handle_key(key, commands, cancel);
        }
    }
    Ok(())
}

struct RawModeGuard;

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
    }
}

fn enter_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode().map_err(DshrError::Io)?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).map_err(DshrError::Io)?;
    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend).map_err(|error| DshrError::config(format!("terminal: {error}")))
}

fn leave_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    disable_raw_mode().map_err(DshrError::Io)?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen).map_err(DshrError::Io)?;
    terminal.show_cursor().map_err(DshrError::Io)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> App {
        App::new("deepseek-flash".into(), "ses_test".into(), Lang::Ru)
    }

    #[test]
    fn streams_text_into_transcript() {
        let mut app = app();
        app.apply(UiEvent::Text("hello".into()));
        app.apply(UiEvent::Text(" world".into()));
        assert_eq!(app.stream, "hello world");
        app.apply(UiEvent::Done {
            usage: Usage::default(),
            steps: 1,
            tool_calls: 0,
        });
        assert!(app.lines.iter().any(|line| line == "hello world"));
        assert!(!app.busy);
    }

    #[test]
    fn approval_requires_answer() {
        let mut app = app();
        let (respond, answer) = oneshot::channel();
        app.apply(UiEvent::Approval {
            request: ApprovalRequest {
                tool: "bash".into(),
                summary: "echo hi".into(),
                input: serde_json::json!({}),
                reason: "shell".into(),
            },
            respond,
        });
        let (commands, _receiver) = mpsc::channel();
        let cancel = Mutex::new(None);
        app.handle_key(
            KeyEvent::new(KeyCode::Char('y'), crossterm::event::KeyModifiers::NONE),
            &commands,
            &cancel,
        );
        assert!(answer.blocking_recv().expect("answer"));
        assert!(app.approval.is_none());
    }

    #[test]
    fn question_accepts_a_typed_answer() {
        let mut app = app();
        let (respond, answer) = oneshot::channel();
        app.apply(UiEvent::Question {
            question: "Color?".into(),
            options: Vec::new(),
            respond,
        });
        let (commands, _receiver) = mpsc::channel();
        let cancel = Mutex::new(None);
        for character in "blue".chars() {
            app.handle_key(
                KeyEvent::new(
                    KeyCode::Char(character),
                    crossterm::event::KeyModifiers::NONE,
                ),
                &commands,
                &cancel,
            );
        }
        app.handle_key(
            KeyEvent::new(KeyCode::Enter, crossterm::event::KeyModifiers::NONE),
            &commands,
            &cancel,
        );
        assert_eq!(
            answer.blocking_recv().expect("answer"),
            Some("blue".to_owned())
        );
        assert!(app.question.is_none());
    }

    #[test]
    fn question_options_are_selected_by_digit() {
        let mut app = app();
        let (respond, answer) = oneshot::channel();
        app.apply(UiEvent::Question {
            question: "Pick".into(),
            options: vec!["a".into(), "b".into()],
            respond,
        });
        let (commands, _receiver) = mpsc::channel();
        let cancel = Mutex::new(None);
        app.handle_key(
            KeyEvent::new(KeyCode::Char('2'), crossterm::event::KeyModifiers::NONE),
            &commands,
            &cancel,
        );
        assert_eq!(
            answer.blocking_recv().expect("answer"),
            Some("b".to_owned())
        );
    }

    #[test]
    fn enter_sends_prompt_only_when_idle() {
        let mut app = app();
        let (commands, receiver) = mpsc::channel();
        let cancel = Mutex::new(None);
        app.input = "hello".into();
        app.handle_key(
            KeyEvent::new(KeyCode::Enter, crossterm::event::KeyModifiers::NONE),
            &commands,
            &cancel,
        );
        assert!(matches!(receiver.recv().expect("command"), Command::Prompt(p) if p == "hello"));
        assert!(app.busy);
    }
}

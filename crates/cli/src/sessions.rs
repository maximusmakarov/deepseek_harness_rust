use clap::{Args, Subcommand};
use dshr_config::ConfigLoader;
use dshr_core::{DshrError, Result};
use dshr_i18n::current;
use dshr_llm::{ContentBlock, Role};
use dshr_search::SessionSearchIndex;
use dshr_session_jsonl::{MarkdownLabels, SessionStore};

#[derive(Debug, Args)]
pub struct SessionsArgs {
    #[command(subcommand)]
    pub action: SessionsAction,
}

#[derive(Debug, Subcommand)]
pub enum SessionsAction {
    /// List sessions in the store
    List {
        /// Emit JSON instead of a table
        #[arg(long)]
        json: bool,
    },
    /// Show one session's header and messages
    Show {
        /// Session id
        id: String,
    },
    /// Export one session
    Export {
        /// Session id
        id: String,
        /// Output format: markdown or json
        #[arg(long, default_value = "markdown")]
        format: String,
    },
    /// Fork a session at a sequence number
    Fork {
        /// Source session id
        id: String,
        /// Last sequence number to copy
        #[arg(long)]
        at: Option<u64>,
    },
    /// Drop turns after the first N
    Rewind {
        /// Session id
        id: String,
        /// Number of turns to keep
        #[arg(long, default_value_t = 0)]
        turn: usize,
    },
    /// Set a session title
    Title {
        /// Session id
        id: String,
        /// New title
        title: String,
    },
    /// Full-text search across sessions
    Search {
        /// Query string
        query: String,
        /// Maximum number of hits
        #[arg(long, default_value_t = 20)]
        limit: usize,
        /// Emit JSON
        #[arg(long)]
        json: bool,
        /// Use the SQLite FTS5 index instead of a scan
        #[arg(long)]
        fts: bool,
    },
    /// Build or rebuild the FTS5 search index
    Reindex,
    /// Import a DSH v4 session (session.v4.jsonl.zstd)
    Import {
        /// Session directory or file
        path: std::path::PathBuf,
        /// Provider label stored in the imported header
        #[arg(long)]
        provider: Option<String>,
        /// Model label stored in the imported header
        #[arg(long)]
        model: Option<String>,
        /// Import every session file under the path
        #[arg(long)]
        all: bool,
    },
}

pub fn execute(loader: &ConfigLoader, args: &SessionsArgs) -> Result<u8> {
    let config = loader.load()?.config;
    let dir = config
        .session
        .dir
        .clone()
        .ok_or_else(|| DshrError::config("session.dir is not resolved"))?;
    let store = SessionStore::new(dir);

    match &args.action {
        SessionsAction::List { json } => list(&store, *json),
        SessionsAction::Show { id } => show(&store, id),
        SessionsAction::Export { id, format } => export(&store, id, format),
        SessionsAction::Fork { id, at } => fork(&store, id, *at),
        SessionsAction::Rewind { id, turn } => rewind(&store, id, *turn),
        SessionsAction::Title { id, title } => set_title(&store, id, title),
        SessionsAction::Search {
            query,
            limit,
            json,
            fts,
        } => search(&store, query, *limit, *json, *fts),
        SessionsAction::Reindex => reindex(&store),
        SessionsAction::Import {
            path,
            provider,
            model,
            all,
        } => {
            let provider = provider.as_deref().unwrap_or("dsh-import");
            let model = model.as_deref().unwrap_or("dsh-import");
            if *all {
                let count = crate::dsh_import::import_tree(&store, path, provider, model)?;
                println!("{count}");
            } else {
                println!("{}", crate::dsh_import::import(&store, path, provider, model)?);
            }
            Ok(0)
        }
    }
}

fn fork(store: &SessionStore, id: &str, at: Option<u64>) -> Result<u8> {
    let session = store.fork(id, at)?;
    println!("{}", session.header.id);
    Ok(0)
}

fn rewind(store: &SessionStore, id: &str, turn: usize) -> Result<u8> {
    let session = store.rewind(id, turn)?;
    println!(
        "{}",
        current().sessions_rewound(
            session.header.id.as_str(),
            session.summary().turns,
            session.events.len()
        )
    );
    Ok(0)
}

fn set_title(store: &SessionStore, id: &str, title: &str) -> Result<u8> {
    let session = store.set_title(id, title)?;
    println!(
        "{}",
        current().session_title(
            session.header.id.as_str(),
            session
                .header
                .title
                .as_deref()
                .unwrap_or(current().untitled())
        )
    );
    Ok(0)
}

fn search(store: &SessionStore, query: &str, limit: usize, json: bool, fts: bool) -> Result<u8> {
    let hits = if fts && query.chars().count() >= 3 {
        let mut index = SessionSearchIndex::open(store.root())
            .map_err(|error| DshrError::config(error.to_string()))?;
        if !SessionSearchIndex::exists(store.root()) {
            index
                .rebuild(store)
                .map_err(|error| DshrError::config(error.to_string()))?;
        }
        index
            .search(query, limit)
            .map_err(|error| DshrError::config(error.to_string()))?
    } else {
        store.search(query, limit)?
    };
    if json {
        let encoded = serde_json::to_string_pretty(&hits)
            .map_err(|error| DshrError::config(format!("serialize hits: {error}")))?;
        println!("{encoded}");
    } else if hits.is_empty() {
        println!("{}", current().no_hits());
    } else {
        for hit in hits {
            println!(
                "{} {} {} · {}",
                hit.session_id, hit.seq, hit.kind, hit.snippet
            );
        }
    }
    Ok(0)
}

fn reindex(store: &SessionStore) -> Result<u8> {
    let mut index = SessionSearchIndex::open(store.root())
        .map_err(|error| DshrError::config(error.to_string()))?;
    let count = index
        .rebuild(store)
        .map_err(|error| DshrError::config(error.to_string()))?;
    println!("{}", current().indexed(count));
    Ok(0)
}

fn list(store: &SessionStore, json: bool) -> Result<u8> {
    let summaries = store.list()?;
    if json {
        let encoded = serde_json::to_string_pretty(&summaries)
            .map_err(|err| DshrError::config(format!("serialize sessions: {err}")))?;
        println!("{encoded}");
        return Ok(0);
    }

    let lang = current();
    println!(
        "{:<40} {:<26} {:<20} {}",
        lang.col_id(),
        lang.col_created(),
        lang.col_model(),
        lang.col_turns()
    );
    for summary in summaries {
        println!(
            "{:<40} {:<26} {:<20} {}",
            summary.id, summary.created_at, summary.model, summary.turns
        );
    }
    Ok(0)
}

fn show(store: &SessionStore, id: &str) -> Result<u8> {
    let session = store.open(id)?;
    println!("# {}", session.header.id);
    let lang = current();
    if lang.is_ru() {
        println!("создана:  {}", session.header.created_at);
        println!("провайдер: {}", session.header.provider);
        println!("модель:   {}", session.header.model);
        println!("каталог:  {}", session.header.cwd);
    } else {
        println!("created:  {}", session.header.created_at);
        println!("provider: {}", session.header.provider);
        println!("model:    {}", session.header.model);
        println!("cwd:      {}", session.header.cwd);
    }
    println!();

    for message in session.derive_messages() {
        println!(
            "## {}",
            match message.role {
                Role::User => lang.role_user(),
                Role::Assistant => lang.role_assistant(),
                Role::System => "System",
                Role::Tool => "Tool",
            }
        );
        println!();
        println!("{}", text_of(&message.content));
        println!();
    }
    Ok(0)
}

fn export(store: &SessionStore, id: &str, format: &str) -> Result<u8> {
    let session = store.open(id)?;
    let text = match format {
        "json" => store.export_json(&session)?,
        "markdown" | "md" => {
            let lang = current();
            let labels = if lang.is_ru() {
                MarkdownLabels {
                    session: "Сессия",
                    created: "создана",
                    provider: "провайдер",
                    model: "модель",
                    cwd: "каталог",
                    user: "Пользователь",
                    assistant: "Ассистент",
                    thinking: "размышления",
                    result: "результат",
                    image: "изображение",
                    audio: "аудио",
                }
            } else {
                MarkdownLabels::default()
            };
            store.export_markdown_with(&session, &labels)?
        }
        other => {
            return Err(DshrError::invalid_input(format!(
                "unknown export format '{other}'; expected markdown or json"
            )));
        }
    };
    print!("{text}");
    if !text.ends_with('\n') {
        println!();
    }
    Ok(0)
}

fn text_of(content: &[ContentBlock]) -> String {
    content
        .iter()
        .filter_map(|block| match block {
            ContentBlock::Text { text } => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("")
}

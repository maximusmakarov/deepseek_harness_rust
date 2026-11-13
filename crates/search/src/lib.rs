#![forbid(unsafe_code)]
//! SQLite FTS5 index over session events.
//!
//! Opt-in: the default `sessions search` remains a scan until the benchmark
//! gate in ADR-016 is exceeded. `sessions reindex` builds this index.

use std::path::{Path, PathBuf};

use dshr_core::DshrError;
use dshr_session_jsonl::{SearchHit, SessionStore, event_text};
use rusqlite::{Connection, params};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SearchError {
    #[error("search index I/O: {0}")]
    Io(String),

    #[error("search index: {0}")]
    Sqlite(String),
}

impl From<DshrError> for SearchError {
    fn from(error: DshrError) -> Self {
        Self::Io(error.to_string())
    }
}

impl From<rusqlite::Error> for SearchError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Sqlite(error.to_string())
    }
}

pub struct SessionSearchIndex {
    connection: Connection,
}

impl SessionSearchIndex {
    pub fn open(root: impl AsRef<Path>) -> Result<Self, SearchError> {
        let root = root.as_ref();
        std::fs::create_dir_all(root).map_err(|error| SearchError::Io(error.to_string()))?;
        let path = root.join("search.sqlite");
        let connection =
            Connection::open(&path).map_err(|error| SearchError::Sqlite(error.to_string()))?;
        connection
            .execute_batch(
                "CREATE VIRTUAL TABLE IF NOT EXISTS search USING fts5(
                    session_id UNINDEXED,
                    seq UNINDEXED,
                    kind UNINDEXED,
                    text,
                    tokenize='trigram'
                );",
            )
            .map_err(|error| SearchError::Sqlite(error.to_string()))?;
        Ok(Self { connection })
    }

    pub fn rebuild(&mut self, store: &SessionStore) -> Result<usize, SearchError> {
        let transaction = self.connection.transaction()?;
        transaction.execute("DELETE FROM search", [])?;
        let mut indexed = 0;
        for summary in store.list()? {
            let Ok(session) = store.open(summary.id.as_str()) else {
                continue;
            };
            for event in &session.events {
                let text = event_text(event);
                if text.trim().is_empty() {
                    continue;
                }
                transaction.execute(
                    "INSERT INTO search (session_id, seq, kind, text) VALUES (?1, ?2, ?3, ?4)",
                    params![
                        session.header.id.as_str(),
                        event.seq() as i64,
                        event.kind(),
                        text
                    ],
                )?;
                indexed += 1;
            }
        }
        transaction.commit()?;
        Ok(indexed)
    }

    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchHit>, SearchError> {
        if query.chars().count() < 3 {
            return Ok(Vec::new());
        }
        let phrase = format!("\"{}\"", query.replace('"', "\"\""));
        let mut statement = self.connection.prepare(
            "SELECT session_id, seq, kind, snippet(search, 3, '', '', '…', 16)
             FROM search WHERE search MATCH ?1 ORDER BY rank LIMIT ?2",
        )?;
        let rows = statement.query_map(params![phrase, limit as i64], |row| {
            Ok(SearchHit {
                session_id: row.get::<_, String>(0)?,
                seq: row.get::<_, i64>(1)? as u64,
                kind: row.get::<_, String>(2)?,
                snippet: row.get::<_, String>(3)?.replace('\n', " "),
            })
        })?;
        let mut hits = Vec::new();
        for row in rows {
            hits.push(row?);
        }
        Ok(hits)
    }

    #[must_use]
    pub fn exists(root: impl AsRef<Path>) -> bool {
        root.as_ref().join("search.sqlite").is_file()
    }

    #[must_use]
    pub fn path(root: impl AsRef<Path>) -> PathBuf {
        root.as_ref().join("search.sqlite")
    }
}

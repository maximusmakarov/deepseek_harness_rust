#![forbid(unsafe_code)]
//! Durable cron schedules.
//!
//! Schedules are stored as YAML and evaluated in UTC. `run-due` executes
//! entries whose `next_run` is in the past and advances `next_run`.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use chrono::{DateTime, SecondsFormat, Utc};
use cron::Schedule;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ScheduleError {
    #[error("schedule I/O: {0}")]
    Io(String),

    #[error("schedule parse: {0}")]
    Parse(String),

    #[error("invalid schedule: {0}")]
    Invalid(String),

    #[error("schedule '{0}' not found")]
    NotFound(String),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ScheduleEntry {
    #[serde(default)]
    pub id: String,
    pub cron: String,
    pub prompt: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub workspace: Option<PathBuf>,
    #[serde(default)]
    pub last_run: Option<String>,
    #[serde(default)]
    pub next_run: Option<String>,
}

fn default_true() -> bool {
    true
}

impl ScheduleEntry {
    pub fn next_after(&self, now: DateTime<Utc>) -> Result<Option<DateTime<Utc>>, ScheduleError> {
        let schedule = Schedule::from_str(&self.cron)
            .map_err(|error| ScheduleError::Invalid(format!("cron '{}': {error}", self.cron)))?;
        Ok(schedule.after(&now).next())
    }
}

pub struct ScheduleStore {
    path: PathBuf,
    entries: Vec<ScheduleEntry>,
}

impl ScheduleStore {
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, ScheduleError> {
        let path = path.into();
        let entries = if path.is_file() {
            let text = std::fs::read_to_string(&path)
                .map_err(|error| ScheduleError::Io(format!("{}: {error}", path.display())))?;
            serde_saphyr::from_str(&text)
                .map_err(|error| ScheduleError::Parse(error.to_string()))?
        } else {
            Vec::new()
        };
        Ok(Self { path, entries })
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    #[must_use]
    pub fn entries(&self) -> &[ScheduleEntry] {
        &self.entries
    }

    pub fn add(
        &mut self,
        mut entry: ScheduleEntry,
        now: DateTime<Utc>,
    ) -> Result<ScheduleEntry, ScheduleError> {
        if entry.prompt.trim().is_empty() {
            return Err(ScheduleError::Invalid("prompt is required".into()));
        }
        if entry.id.trim().is_empty() {
            entry.id = format!("sch_{}", uuid::Uuid::new_v4().simple());
        }
        if self.entries.iter().any(|existing| existing.id == entry.id) {
            return Err(ScheduleError::Invalid(format!(
                "schedule '{}' already exists",
                entry.id
            )));
        }
        let next = entry
            .next_after(now)?
            .ok_or_else(|| ScheduleError::Invalid("cron has no future occurrence".into()))?;
        entry.next_run = Some(format_time(next));
        entry.last_run = None;
        self.entries.push(entry.clone());
        self.save()?;
        Ok(entry)
    }

    pub fn remove(&mut self, id: &str) -> Result<(), ScheduleError> {
        let before = self.entries.len();
        self.entries.retain(|entry| entry.id != id);
        if self.entries.len() == before {
            return Err(ScheduleError::NotFound(id.to_owned()));
        }
        self.save()
    }

    pub fn set_enabled(&mut self, id: &str, enabled: bool) -> Result<(), ScheduleError> {
        let entry = self
            .entries
            .iter_mut()
            .find(|entry| entry.id == id)
            .ok_or_else(|| ScheduleError::NotFound(id.to_owned()))?;
        entry.enabled = enabled;
        self.save()
    }

    pub fn due(&self, now: DateTime<Utc>) -> Vec<ScheduleEntry> {
        self.entries
            .iter()
            .filter(|entry| entry.enabled)
            .filter(|entry| {
                entry
                    .next_run
                    .as_deref()
                    .and_then(parse_time)
                    .is_some_and(|next| next <= now)
            })
            .cloned()
            .collect()
    }

    pub fn mark_run(
        &mut self,
        id: &str,
        now: DateTime<Utc>,
    ) -> Result<Option<DateTime<Utc>>, ScheduleError> {
        let entry = self
            .entries
            .iter_mut()
            .find(|entry| entry.id == id)
            .ok_or_else(|| ScheduleError::NotFound(id.to_owned()))?;
        entry.last_run = Some(format_time(now));
        let next = entry.next_after(now)?;
        entry.next_run = next.map(format_time);
        self.save()?;
        Ok(next)
    }

    fn save(&self) -> Result<(), ScheduleError> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| ScheduleError::Io(error.to_string()))?;
        }
        let text = serde_saphyr::to_string(&self.entries)
            .map_err(|error| ScheduleError::Parse(error.to_string()))?;
        let temporary = self.path.with_extension("yaml.tmp");
        std::fs::write(&temporary, text).map_err(|error| ScheduleError::Io(error.to_string()))?;
        std::fs::rename(&temporary, &self.path)
            .map_err(|error| ScheduleError::Io(error.to_string()))
    }
}

/// Prevents two schedulers from running the same store concurrently.
pub struct ScheduleLock {
    path: PathBuf,
}

impl ScheduleLock {
    pub fn acquire(path: impl Into<PathBuf>) -> Result<Self, ScheduleError> {
        let path = path.into();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| ScheduleError::Io(error.to_string()))?;
        }
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(mut file) => {
                let _ = writeln!(file, "{}", std::process::id());
                Ok(Self { path })
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => Err(
                ScheduleError::Invalid(format!("another scheduler holds {}", path.display())),
            ),
            Err(error) => Err(ScheduleError::Io(error.to_string())),
        }
    }
}

impl Drop for ScheduleLock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

pub fn parse_time(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|time| time.with_timezone(&Utc))
}

pub fn format_time(time: DateTime<Utc>) -> String {
    time.to_rfc3339_opts(SecondsFormat::Secs, true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn entry(cron: &str) -> ScheduleEntry {
        ScheduleEntry {
            id: "test".into(),
            cron: cron.into(),
            prompt: "do it".into(),
            enabled: true,
            session_id: None,
            workspace: None,
            last_run: None,
            next_run: None,
        }
    }

    #[test]
    fn adds_and_persists() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("schedules.yaml");
        let now = Utc.with_ymd_and_hms(2026, 1, 1, 0, 1, 0).unwrap();
        let mut store = ScheduleStore::open(&path).expect("open");
        store.add(entry("0 */5 * * * *"), now).expect("add");
        assert!(path.is_file());
        drop(store);
        let store = ScheduleStore::open(&path).expect("reopen");
        assert_eq!(store.entries().len(), 1);
        assert_eq!(
            store.entries()[0].next_run.as_deref(),
            Some("2026-01-01T00:05:00Z")
        );
    }

    #[test]
    fn rejects_bad_cron() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut store = ScheduleStore::open(dir.path().join("schedules.yaml")).expect("open");
        let now = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        assert!(store.add(entry("not a cron"), now).is_err());
    }

    #[test]
    fn finds_due_and_advances() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut store = ScheduleStore::open(dir.path().join("schedules.yaml")).expect("open");
        let now = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        store.add(entry("0 */5 * * * *"), now).expect("add");
        let later = Utc.with_ymd_and_hms(2026, 1, 1, 0, 6, 0).unwrap();
        assert_eq!(store.due(later).len(), 1);
        store.mark_run("test", later).expect("mark");
        assert_eq!(
            store.entries()[0].last_run.as_deref(),
            Some("2026-01-01T00:06:00Z")
        );
        assert!(
            store.entries()[0]
                .next_run
                .as_deref()
                .is_some_and(|value| parse_time(value).is_some_and(|next| next > later))
        );
    }

    #[test]
    fn lock_is_exclusive_and_released() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("schedules.lock");
        let lock = ScheduleLock::acquire(&path).expect("lock");
        assert!(ScheduleLock::acquire(&path).is_err());
        drop(lock);
        assert!(ScheduleLock::acquire(&path).is_ok());
    }

    #[test]
    fn disables_and_removes() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut store = ScheduleStore::open(dir.path().join("schedules.yaml")).expect("open");
        let now = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        store.add(entry("0 */5 * * * *"), now).expect("add");
        store.set_enabled("test", false).expect("disable");
        assert!(store.due(now).is_empty());
        store.remove("test").expect("remove");
        assert!(store.entries().is_empty());
    }
}

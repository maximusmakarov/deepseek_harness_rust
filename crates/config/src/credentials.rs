//! Write-only credential storage.
//!
//! dshr used to read API keys from the environment only. The Models page can
//! now store a key the user types, the way DSH's credentials service does:
//! `<home>/credentials.json` is written atomically with `0600`, the API never
//! returns a value (only a masked preview), and an environment variable with
//! the same name always wins, so `desktop.env` keeps working.

use std::collections::BTreeMap;
use std::fmt;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// File name under the dshr home.
pub const CREDENTIALS_FILE: &str = "credentials.json";
/// Schema version of the document.
pub const CREDENTIALS_VERSION: u32 = 1;

/// Where a resolved secret came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredentialSource {
    /// An environment variable (including `desktop.env`).
    Environment,
    /// The per-home credentials document.
    Config,
}

impl CredentialSource {
    #[must_use]
    pub fn id(self) -> &'static str {
        match self {
            Self::Environment => "env",
            Self::Config => "config",
        }
    }
}

/// A resolved secret. Deliberately not `Serialize` and redacted in `Debug`.
#[derive(Clone, PartialEq, Eq)]
pub struct ResolvedCredential {
    value: String,
    source: CredentialSource,
}

impl ResolvedCredential {
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }

    #[must_use]
    pub fn source(&self) -> CredentialSource {
        self.source
    }
}

impl fmt::Debug for ResolvedCredential {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ResolvedCredential")
            .field("value", &"<redacted>")
            .field("source", &self.source)
            .finish()
    }
}

/// What the API may reveal about one reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CredentialDescription {
    pub configured: bool,
    pub source: Option<CredentialSource>,
    pub masked: String,
}

/// The durable credentials document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CredentialsDocument {
    pub version: u32,
    pub revision: u64,
    /// Reference -> secret. Values never leave the server.
    pub values: BTreeMap<String, String>,
}

impl Default for CredentialsDocument {
    fn default() -> Self {
        Self {
            version: CREDENTIALS_VERSION,
            revision: 0,
            values: BTreeMap::new(),
        }
    }
}

/// Credential storage failures.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CredentialsError {
    #[error("credentials document: {0}")]
    Io(String),
    #[error("credentials document: {0}")]
    Parse(String),
    #[error("credentials revision conflict: expected {expected}, actual {actual}")]
    Conflict { expected: u64, actual: u64 },
}

impl CredentialsDocument {
    #[must_use]
    pub fn path(home: &Path) -> PathBuf {
        home.join(CREDENTIALS_FILE)
    }

    /// Loads the document; a missing file is the empty document.
    pub fn load(home: &Path) -> Result<Self, CredentialsError> {
        let path = Self::path(home);
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::default());
            }
            Err(error) => return Err(CredentialsError::Io(format!("{}: {error}", path.display()))),
        };
        let document: Self = serde_json::from_str(&text)
            .map_err(|error| CredentialsError::Parse(format!("{}: {error}", path.display())))?;
        if document.version != CREDENTIALS_VERSION {
            return Err(CredentialsError::Parse(format!(
                "{}: unsupported credentials version {}",
                path.display(),
                document.version
            )));
        }
        Ok(document)
    }

    /// Stores one secret, bumping the revision under the optional fence.
    pub fn set(
        home: &Path,
        reference: &str,
        value: &str,
        expected: Option<u64>,
    ) -> Result<Self, CredentialsError> {
        let mut document = Self::load(home)?;
        if let Some(expected) = expected
            && expected != document.revision
        {
            return Err(CredentialsError::Conflict {
                expected,
                actual: document.revision,
            });
        }
        document.revision = document.revision.saturating_add(1);
        document
            .values
            .insert(reference.to_owned(), value.to_owned());
        document.write(home)?;
        Ok(document)
    }

    /// Removes one secret, bumping the revision under the optional fence.
    pub fn remove(
        home: &Path,
        reference: &str,
        expected: Option<u64>,
    ) -> Result<Self, CredentialsError> {
        let mut document = Self::load(home)?;
        if let Some(expected) = expected
            && expected != document.revision
        {
            return Err(CredentialsError::Conflict {
                expected,
                actual: document.revision,
            });
        }
        if document.values.remove(reference).is_none() {
            return Ok(document);
        }
        document.revision = document.revision.saturating_add(1);
        document.write(home)?;
        Ok(document)
    }

    fn write(&self, home: &Path) -> Result<(), CredentialsError> {
        std::fs::create_dir_all(home)
            .map_err(|error| CredentialsError::Io(format!("{}: {error}", home.display())))?;
        let path = Self::path(home);
        let text = serde_json::to_string_pretty(self)
            .map_err(|error| CredentialsError::Io(format!("serialize credentials: {error}")))?;
        let temp = path.with_extension("json.tmp");
        let mut file = std::fs::File::create(&temp)
            .map_err(|error| CredentialsError::Io(format!("{}: {error}", temp.display())))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            file.set_permissions(std::fs::Permissions::from_mode(0o600))
                .map_err(|error| CredentialsError::Io(format!("{}: {error}", temp.display())))?;
        }
        file.write_all(text.as_bytes())
            .map_err(|error| CredentialsError::Io(format!("{}: {error}", temp.display())))?;
        file.sync_all()
            .map_err(|error| CredentialsError::Io(format!("{}: {error}", temp.display())))?;
        drop(file);
        std::fs::rename(&temp, &path)
            .map_err(|error| CredentialsError::Io(format!("{}: {error}", path.display())))?;
        Ok(())
    }
}

/// The mask shown for a stored or environment secret.
#[must_use]
pub fn mask(value: &str) -> String {
    let count = value.chars().count();
    if count == 0 {
        return String::new();
    }
    if count <= 8 {
        return "•".repeat(count);
    }
    let start: String = value.chars().take(4).collect();
    let end: String = value
        .chars()
        .rev()
        .take(4)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    format!("{start}…{end}")
}

/// Resolves a reference: a non-empty environment variable wins, then the
/// per-home document.
#[must_use]
pub fn resolve(home: &Path, reference: &str) -> Option<ResolvedCredential> {
    if let Ok(value) = std::env::var(reference)
        && !value.trim().is_empty()
    {
        return Some(ResolvedCredential {
            value,
            source: CredentialSource::Environment,
        });
    }
    let document = CredentialsDocument::load(home).ok()?;
    document
        .values
        .get(reference)
        .filter(|value| !value.trim().is_empty())
        .map(|value| ResolvedCredential {
            value: value.clone(),
            source: CredentialSource::Config,
        })
}

/// Describes one reference without exposing the secret.
#[must_use]
pub fn describe(home: &Path, reference: &str) -> CredentialDescription {
    match resolve(home, reference) {
        Some(resolved) => CredentialDescription {
            configured: true,
            source: Some(resolved.source()),
            masked: mask(resolved.value()),
        },
        None => CredentialDescription {
            configured: false,
            source: None,
            masked: String::new(),
        },
    }
}

/// A usable credential reference: an environment-style variable name.
#[must_use]
pub fn valid_reference(reference: &str) -> bool {
    !reference.is_empty()
        && reference.len() <= 96
        && reference
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_uppercase())
        && reference
            .chars()
            .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_')
}

/// A value an HTTP header can carry: printable ASCII, 1..=4096 characters.
#[must_use]
pub fn valid_secret(value: &str) -> bool {
    !value.is_empty() && value.len() <= 4096 && value.chars().all(|ch| ('!'..='~').contains(&ch))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stores_and_masks_secrets() {
        let home = tempfile::tempdir().expect("home");
        assert!(!describe(home.path(), "MY_KEY").configured);

        CredentialsDocument::set(home.path(), "MY_KEY", "sk-1234567890abcdef", Some(0))
            .expect("store");
        let description = describe(home.path(), "MY_KEY");
        assert!(description.configured);
        assert_eq!(description.source, Some(CredentialSource::Config));
        assert_eq!(description.masked, "sk-1…cdef");

        let text = std::fs::read_to_string(CredentialsDocument::path(home.path())).expect("file");
        assert!(text.contains("sk-1234567890abcdef"));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(CredentialsDocument::path(home.path()))
                .expect("metadata")
                .permissions()
                .mode()
                & 0o777;
            assert_eq!(mode, 0o600);
        }

        CredentialsDocument::remove(home.path(), "MY_KEY", Some(1)).expect("remove");
        assert!(!describe(home.path(), "MY_KEY").configured);
    }

    #[test]
    fn a_stale_revision_is_refused_and_the_value_is_not_replaced() {
        let home = tempfile::tempdir().expect("home");
        CredentialsDocument::set(home.path(), "A_KEY", "first-value", Some(0)).expect("first");
        let error = CredentialsDocument::set(home.path(), "A_KEY", "second-value", Some(0))
            .expect_err("stale");
        assert_eq!(
            error,
            CredentialsError::Conflict {
                expected: 0,
                actual: 1
            }
        );
        assert_eq!(
            CredentialsDocument::load(home.path()).expect("load").values["A_KEY"],
            "first-value"
        );
    }

    #[test]
    fn references_and_values_are_validated() {
        assert!(valid_reference("DEEPSEEK_API_KEY"));
        assert!(!valid_reference("lower"));
        assert!(!valid_reference("1ABC"));
        assert!(valid_secret("sk-123"));
        assert!(!valid_secret("has space"));
        assert!(!valid_secret(""));
        assert_eq!(mask("short"), "•••••");
    }
}

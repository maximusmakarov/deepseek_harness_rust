//! Registry v2 client: abbreviated packuments with an on-disk cache.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use dshr_core::{DshrError, Result};
use semver::Version;
use serde::Deserialize;

use crate::{Requirement, encode_name};

/// Registry used when `DSHR_NPM_REGISTRY` is unset.
pub const DEFAULT_REGISTRY: &str = "https://registry.npmjs.org";

/// One published version of a package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionMeta {
    pub version: Version,
    /// Absolute tarball URL.
    pub tarball: String,
    /// SRI string from `dist.integrity` (`sha512-<base64>`).
    pub integrity: Option<String>,
    /// Legacy `dist.shasum` (sha1 hex).
    pub shasum: Option<String>,
    pub dependencies: BTreeMap<String, String>,
    pub optional_dependencies: BTreeMap<String, String>,
}

/// The parts of a packument the resolver needs.
#[derive(Debug, Clone)]
pub struct Packument {
    pub name: String,
    pub dist_tags: BTreeMap<String, String>,
    pub versions: BTreeMap<String, VersionMeta>,
}

impl Packument {
    /// Picks the version a requirement asks for.
    pub fn resolve(&self, requirement: &Requirement) -> Result<&VersionMeta> {
        match requirement {
            Requirement::Tag(tag) => {
                let version = self.dist_tags.get(tag).ok_or_else(|| {
                    DshrError::not_found(format!("npm: {} has no dist-tag '{tag}'", self.name))
                })?;
                self.versions.get(version).ok_or_else(|| {
                    DshrError::not_found(format!(
                        "npm: {} tag '{tag}' points at missing version {version}",
                        self.name
                    ))
                })
            }
            Requirement::Range(range) => self
                .versions
                .values()
                .filter(|meta| range.matches(&meta.version))
                .max_by(|left, right| left.version.cmp(&right.version))
                .ok_or_else(|| {
                    DshrError::not_found(format!(
                        "npm: no version of {} matches '{range}'",
                        self.name
                    ))
                }),
            Requirement::Any(ranges) => self
                .versions
                .values()
                .filter(|meta| ranges.iter().any(|range| range.matches(&meta.version)))
                .max_by(|left, right| left.version.cmp(&right.version))
                .ok_or_else(|| {
                    let ranges: Vec<String> = ranges.iter().map(ToString::to_string).collect();
                    DshrError::not_found(format!(
                        "npm: no version of {} matches '{}'",
                        self.name,
                        ranges.join(" || ")
                    ))
                }),
            Requirement::Invalid(spec) => Err(DshrError::invalid_input(format!(
                "npm: {} requires '{spec}', which is not a registry range (workspace/link specs cannot be installed from a registry)",
                self.name
            ))),
        }
    }
}

/// Registry client with a disk cache of packuments.
#[derive(Debug, Clone)]
pub struct Registry {
    base: String,
    cache: PathBuf,
    client: reqwest::Client,
    offline: bool,
}

impl Registry {
    /// Registry for a dshr home; honours `DSHR_NPM_REGISTRY`.
    #[must_use]
    pub fn for_home(home: &Path) -> Self {
        let base = std::env::var("DSHR_NPM_REGISTRY")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_REGISTRY.to_owned());
        Self::new(base, home.join("npm").join("registry"))
    }

    #[must_use]
    pub fn new(base: impl Into<String>, cache: PathBuf) -> Self {
        let client = reqwest::Client::builder()
            .user_agent(concat!("dshr/", env!("CARGO_PKG_VERSION")))
            .timeout(Duration::from_secs(60))
            .build()
            .unwrap_or_default();
        Self {
            base: base.into().trim_end_matches('/').to_owned(),
            cache,
            client,
            offline: false,
        }
    }

    /// Only read cached packuments; useful for reproducible/offline installs.
    #[must_use]
    pub fn offline(mut self) -> Self {
        self.offline = true;
        self
    }

    #[must_use]
    pub fn base(&self) -> &str {
        &self.base
    }

    fn cache_path(&self, name: &str) -> PathBuf {
        let safe: String = name
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_') {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        self.cache.join(format!("{safe}.json"))
    }

    /// Fetches (or reads from cache) the packument of `name`.
    pub async fn packument(&self, name: &str) -> Result<Packument> {
        let cache = self.cache_path(name);
        if self.offline {
            let body = std::fs::read_to_string(&cache).map_err(|error| {
                DshrError::not_found(format!(
                    "npm: no cached metadata for {name} ({}): {error}",
                    cache.display()
                ))
            })?;
            return parse_packument(name, &body);
        }
        let url = format!("{}/{}", self.base, encode_name(name));
        let response = self
            .client
            .get(&url)
            .header("Accept", "application/vnd.npm.install-v1+json")
            .send()
            .await;
        let body = match response {
            Ok(response) if response.status() == reqwest::StatusCode::NOT_FOUND => {
                return Err(DshrError::not_found(format!(
                    "npm: package {name} not found"
                )));
            }
            Ok(response) if !response.status().is_success() => {
                return Err(DshrError::config(format!(
                    "npm: registry returned HTTP {} for {name}",
                    response.status()
                )));
            }
            Ok(response) => response
                .text()
                .await
                .map_err(|error| DshrError::config(format!("npm: {name}: {error}")))?,
            Err(error) => {
                // A stale cache beats failing an offline machine.
                if let Ok(body) = std::fs::read_to_string(&cache) {
                    tracing::warn!(%name, %error, "npm registry unreachable, using cached metadata");
                    return parse_packument(name, &body);
                }
                return Err(DshrError::config(format!(
                    "npm: cannot reach {} for {name}: {error}",
                    self.base
                )));
            }
        };
        let packument = parse_packument(name, &body)?;
        if let Some(parent) = cache.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let staging = cache.with_extension("json.part");
        std::fs::write(&staging, body)?;
        std::fs::rename(&staging, &cache)?;
        Ok(packument)
    }
}

fn parse_packument(name: &str, body: &str) -> Result<Packument> {
    let raw: RawPackument = serde_json::from_str(body)
        .map_err(|error| DshrError::config(format!("npm: bad metadata for {name}: {error}")))?;
    let mut versions = BTreeMap::new();
    for (key, entry) in raw.versions {
        let Ok(version) = Version::parse(&entry.version) else {
            continue;
        };
        if version.to_string() != key {
            continue;
        }
        versions.insert(
            key,
            VersionMeta {
                version,
                tarball: entry.dist.tarball,
                integrity: entry.dist.integrity,
                shasum: entry.dist.shasum,
                dependencies: entry.dependencies,
                optional_dependencies: entry.optional_dependencies,
            },
        );
    }
    if versions.is_empty() {
        return Err(DshrError::not_found(format!(
            "npm: {name} has no usable versions"
        )));
    }
    Ok(Packument {
        name: name.to_owned(),
        dist_tags: raw.dist_tags,
        versions,
    })
}

#[derive(Debug, Deserialize)]
struct RawPackument {
    #[serde(default, rename = "dist-tags")]
    dist_tags: BTreeMap<String, String>,
    #[serde(default)]
    versions: BTreeMap<String, RawVersion>,
}

#[derive(Debug, Deserialize)]
struct RawVersion {
    version: String,
    #[serde(default)]
    dependencies: BTreeMap<String, String>,
    #[serde(default, rename = "optionalDependencies")]
    optional_dependencies: BTreeMap<String, String>,
    dist: RawDist,
}

#[derive(Debug, Deserialize)]
struct RawDist {
    tarball: String,
    #[serde(default)]
    integrity: Option<String>,
    #[serde(default)]
    shasum: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    const BODY: &str = r#"{
        "name": "demo",
        "dist-tags": {"latest": "2.0.0", "beta": "3.0.0-beta.1"},
        "versions": {
            "1.0.0": {"version": "1.0.0", "dist": {"tarball": "https://x/1.tgz", "integrity": "sha512-aa"}},
            "2.0.0": {"version": "2.0.0", "dist": {"tarball": "https://x/2.tgz", "shasum": "bb"},
                       "dependencies": {"dep": "^1.0.0"}},
            "3.0.0-beta.1": {"version": "3.0.0-beta.1", "dist": {"tarball": "https://x/3.tgz"}},
            "9.9.9": {"version": "9.9.9", "dist": {"tarball": "https://x/9.tgz"}}
        }
    }"#;

    #[test]
    fn resolves_tags_ranges_and_prereleases() {
        let packument = parse_packument("demo", BODY).expect("parse");
        let latest = packument
            .resolve(&Requirement::parse("latest"))
            .expect("latest");
        assert_eq!(latest.version.to_string(), "2.0.0");
        let beta = packument
            .resolve(&Requirement::parse("beta"))
            .expect("beta");
        assert_eq!(beta.version.to_string(), "3.0.0-beta.1");
        let caret = packument
            .resolve(&Requirement::parse("^1.0.0"))
            .expect("caret");
        assert_eq!(caret.version.to_string(), "1.0.0");
        assert!(packument.resolve(&Requirement::parse("^5")).is_err());
        assert!(packument.resolve(&Requirement::parse("nope")).is_err());
        assert!(
            packument
                .resolve(&Requirement::parse("*"))
                .expect("star")
                .version
                == Version::parse("9.9.9").expect("v"),
            "prereleases are not picked by a plain range"
        );
    }

    #[test]
    fn parses_dependencies() {
        let packument = parse_packument("demo", BODY).expect("parse");
        let meta = packument
            .resolve(&Requirement::parse("2.0.0"))
            .expect("exact");
        assert_eq!(
            meta.dependencies.get("dep").map(String::as_str),
            Some("^1.0.0")
        );
        assert_eq!(meta.integrity, None, "this version only has a shasum");
        assert_eq!(meta.shasum.as_deref(), Some("bb"));
        let old = packument
            .resolve(&Requirement::parse("1.0.0"))
            .expect("exact");
        assert_eq!(old.integrity.as_deref(), Some("sha512-aa"));
    }
}

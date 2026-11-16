#![forbid(unsafe_code)]
//! Minimal npm support for Cordis plugins: registry client, semver resolver and
//! a content-addressed store.
//!
//! The resolver deliberately implements a small, predictable subset of npm:
//!
//! - registry v2 packuments (`Accept: application/vnd.npm.install-v1+json`)
//!   fetched from `DSHR_NPM_REGISTRY` (default `https://registry.npmjs.org`)
//!   and cached under `<home>/npm/registry`;
//! - semver ranges, exact versions and dist-tags;
//! - transitive `dependencies`; `optionalDependencies` are installed on a
//!   best-effort basis, `devDependencies` are ignored;
//! - tarballs verified against the registry `integrity` (sha512, sha1 fallback)
//!   and unpacked once into a content-addressed store (`<home>/npm/cas`);
//! - no lifecycle scripts: nothing from a package runs during installation.
//!
//! Layout follows the npm model: packages live in `<target>/node_modules` when
//! the name is free and nest under their dependent when a different version is
//! already hoisted. `dshr-npm-lock.json` records exact versions and integrity.

mod install;
mod registry;
mod store;

pub use install::{InstallOptions, InstallReport, InstalledPackage, Installer};
pub use registry::{Packument, Registry, VersionMeta};
pub use store::Store;

use dshr_core::{DshrError, Result};
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};

/// One package request: a name plus a version range, exact version or dist-tag.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageSpec {
    pub name: String,
    /// Range, exact version or dist-tag; `None` means the `latest` tag.
    pub requirement: Option<String>,
}

impl PackageSpec {
    pub fn new(name: impl Into<String>, requirement: Option<String>) -> Self {
        Self {
            name: name.into(),
            requirement: requirement.filter(|value| !value.trim().is_empty()),
        }
    }

    /// Parses `name`, `name@range`, `@scope/name` and `@scope/name@range`.
    pub fn parse(spec: &str) -> Result<Self> {
        let spec = spec.trim();
        if spec.is_empty() {
            return Err(DshrError::invalid_input("npm: empty package spec"));
        }
        if spec.starts_with("file:")
            || spec.starts_with('.')
            || spec.starts_with('/')
            || spec.contains("://")
            || spec.starts_with("git+")
        {
            return Err(DshrError::invalid_input(format!(
                "npm: '{spec}' is not a registry package spec"
            )));
        }
        let (name, requirement) = split_spec(spec);
        if name.is_empty() || name.starts_with('/') {
            return Err(DshrError::invalid_input(format!(
                "npm: cannot parse the package name from '{spec}'"
            )));
        }
        Ok(Self::new(name, requirement))
    }

    #[must_use]
    pub fn requirement(&self) -> Requirement {
        match self.requirement.as_deref() {
            None => Requirement::Tag("latest".to_owned()),
            Some(value) => Requirement::parse(value),
        }
    }

    /// `name@range` form used in lock files and reports.
    #[must_use]
    pub fn display(&self) -> String {
        match &self.requirement {
            Some(requirement) => format!("{}@{requirement}", self.name),
            None => self.name.clone(),
        }
    }
}

/// How to pick a version out of a packument.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Requirement {
    Tag(String),
    Range(VersionReq),
    /// A `||` union of ranges.
    Any(Vec<VersionReq>),
    /// A dependency spec the resolver cannot honour (`workspace:^`, `link:…`).
    Invalid(String),
}

impl Requirement {
    #[must_use]
    pub fn parse(value: &str) -> Self {
        let value = value.trim();
        if value.is_empty() {
            return Self::Tag("latest".to_owned());
        }
        if value.contains('|') {
            let alternatives: Vec<&str> = value
                .split('|')
                .map(str::trim)
                .filter(|part| !part.is_empty())
                .collect();
            let parsed: Option<Vec<VersionReq>> = alternatives
                .iter()
                .map(|part| VersionReq::parse(&normalise_range(part)).ok())
                .collect();
            return match parsed {
                Some(ranges) if !ranges.is_empty() => Self::Any(ranges),
                _ => Self::Invalid(value.to_owned()),
            };
        }
        // npm treats a bare version as exact; the semver crate would read
        // `0.0.1-rc.1` as a caret range and pick a newer release.
        if !value.starts_with(['=', '^', '~', '>', '<', '*'])
            && let Ok(version) = Version::parse(value)
            && let Ok(range) = VersionReq::parse(&format!("={version}"))
        {
            return Self::Range(range);
        }
        match VersionReq::parse(&normalise_range(value)) {
            Ok(range) => Self::Range(range),
            Err(_) if is_tag(value) => Self::Tag(value.to_owned()),
            Err(_) => Self::Invalid(value.to_owned()),
        }
    }

    /// Stable key for de-duplicating repeated requests.
    #[must_use]
    pub fn key(&self) -> String {
        match self {
            Self::Tag(tag) => format!("tag:{tag}"),
            Self::Range(range) => format!("range:{range}"),
            Self::Any(ranges) => {
                let parts: Vec<String> = ranges.iter().map(ToString::to_string).collect();
                format!("any:{}", parts.join("|"))
            }
            Self::Invalid(spec) => format!("invalid:{spec}"),
        }
    }

    #[must_use]
    pub fn matches(&self, version: &Version) -> bool {
        match self {
            Self::Tag(tag) => tag == &version.to_string(),
            Self::Range(range) => range.matches(version),
            Self::Any(ranges) => ranges.iter().any(|range| range.matches(version)),
            Self::Invalid(_) => false,
        }
    }
}

/// npm ranges tolerate spaces (`>= 1.2.3 < 2`, `1.2.3 - 2.0.0`); the semver
/// crate does not, so normalise them before parsing.
#[must_use]
pub fn normalise_range(value: &str) -> String {
    let collapsed = value.split_whitespace().collect::<Vec<_>>().join(" ");
    // Hyphen range: `a - b` means `>=a <=b`.
    if let Some((left, right)) = collapsed.split_once(" - ")
        && !left.contains(' ')
        && !right.contains(' ')
    {
        return format!(">={left}, <={right}");
    }
    let mut out = collapsed;
    for operator in [">=", "<=", ">", "<", "=", "^", "~"] {
        out = out.replace(&format!("{operator} "), operator);
    }
    let out = out.replace(" ,", ",").replace(", ", ",");
    // In npm a space between comparators means AND; the semver crate wants a
    // comma.
    out.split_whitespace().collect::<Vec<_>>().join(", ")
}

/// npm dist-tag names are a small alphanumeric subset.
fn is_tag(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
}

/// Splits `name@version` while keeping scoped names intact.
fn split_spec(spec: &str) -> (String, Option<String>) {
    if let Some(rest) = spec.strip_prefix('@') {
        return match rest.find('/') {
            Some(slash) => {
                let name_end = slash + 1;
                let tail = &spec[name_end + 1..];
                match tail.find('@') {
                    Some(at) => (
                        spec[..name_end + 1 + at].to_owned(),
                        Some(tail[at + 1..].to_owned()),
                    ),
                    None => (spec.to_owned(), None),
                }
            }
            None => (spec.to_owned(), None),
        };
    }
    match spec.find('@') {
        Some(at) if at > 0 => (spec[..at].to_owned(), Some(spec[at + 1..].to_owned())),
        _ => (spec.to_owned(), None),
    }
}

/// Percent-encodes a package name for a registry URL (`@scope/name`).
#[must_use]
pub fn encode_name(name: &str) -> String {
    name.replace('/', "%2F")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_registry_specs() {
        let spec = PackageSpec::parse("lodash").expect("plain");
        assert_eq!(spec.name, "lodash");
        assert_eq!(spec.requirement, None);

        let spec = PackageSpec::parse("lodash@^4.17.0").expect("range");
        assert_eq!(spec.name, "lodash");
        assert_eq!(spec.requirement.as_deref(), Some("^4.17.0"));

        let spec = PackageSpec::parse("@scope/pkg").expect("scoped");
        assert_eq!(spec.name, "@scope/pkg");
        assert_eq!(spec.requirement, None);

        let spec = PackageSpec::parse("@scope/pkg@1.2.3").expect("scoped version");
        assert_eq!(spec.name, "@scope/pkg");
        assert_eq!(spec.requirement.as_deref(), Some("1.2.3"));

        let spec = PackageSpec::parse("dsh-plugin-market@latest").expect("tag");
        assert_eq!(spec.requirement.as_deref(), Some("latest"));
    }

    #[test]
    fn rejects_non_registry_specs() {
        for spec in [
            "file:../x",
            "./local",
            "/abs",
            "git+https://x/y.git",
            "https://x/y.tgz",
            "",
        ] {
            assert!(PackageSpec::parse(spec).is_err(), "{spec} must be rejected");
        }
    }

    #[test]
    fn requirement_matches_ranges_and_tags() {
        let range = Requirement::parse("^1.2.0");
        assert!(range.matches(&Version::parse("1.9.9").expect("v")));
        assert!(!range.matches(&Version::parse("2.0.0").expect("v")));
        let tag = Requirement::parse("next");
        assert_eq!(tag, Requirement::Tag("next".to_owned()));
        assert!(
            !tag.matches(&Version::parse("1.2.3").expect("v")),
            "a dist-tag never matches a bare version"
        );
        assert_eq!(
            Requirement::parse(""),
            Requirement::Tag("latest".to_owned())
        );
    }

    #[test]
    fn normalises_npm_range_syntax() {
        let spaced = Requirement::parse(">= 2.1.2 < 3.0.0");
        assert!(
            spaced.matches(&Version::parse("2.5.0").expect("v")),
            "{spaced:?}"
        );
        assert!(!spaced.matches(&Version::parse("3.0.0").expect("v")));

        let hyphen = Requirement::parse("1.2.3 - 2.0.0");
        assert!(
            hyphen.matches(&Version::parse("1.9.9").expect("v")),
            "{hyphen:?}"
        );
        assert!(!hyphen.matches(&Version::parse("2.0.1").expect("v")));

        let union = Requirement::parse("^1.0.0 || ^3.0.0");
        assert!(
            union.matches(&Version::parse("3.2.1").expect("v")),
            "{union:?}"
        );
        assert!(!union.matches(&Version::parse("2.0.0").expect("v")));
    }

    #[test]
    fn bare_versions_are_exact() {
        let exact = Requirement::parse("0.0.1-rc.1");
        assert!(
            exact.matches(&Version::parse("0.0.1-rc.1").expect("v")),
            "{exact:?}"
        );
        assert!(
            !exact.matches(&Version::parse("0.0.1-rc.5").expect("v")),
            "a bare version must not drift to a newer prerelease"
        );
        let caret = Requirement::parse("^0.0.1-rc.1");
        assert!(caret.matches(&Version::parse("0.0.1-rc.5").expect("v")));
    }

    #[test]
    fn marks_uninstallable_specs_as_invalid() {
        for spec in ["workspace:^", "link:../vendor/x", "npm:other@1"] {
            assert!(
                matches!(Requirement::parse(spec), Requirement::Invalid(_)),
                "{spec} must be invalid"
            );
        }
        assert!(matches!(Requirement::parse("latest"), Requirement::Tag(_)));
    }

    #[test]
    fn encodes_scoped_names() {
        assert_eq!(encode_name("@scope/pkg"), "@scope%2Fpkg");
        assert_eq!(encode_name("plain"), "plain");
    }
}

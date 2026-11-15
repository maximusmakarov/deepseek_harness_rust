//! Client-half discovery for Cordis/npm plugins.
//!
//! The reference implementation publishes browser UI with `dsh.client` metadata
//! in `package.json` (`platform: "web"`, optional `immediately`/`inject`) plus a
//! `./client` export. `dshr` keeps the metadata but not the React module graph:
//! the host serves the bundle into a sandboxed iframe and the bundle talks to
//! the shell over the `postMessage` bridge in [`BRIDGE_JS`] (ADR-013 §5).
//!
//! Discovery is deliberately strict: the bundle must live inside the plugin
//! package, must be a file, and the platform must be `web`.

use std::path::{Path, PathBuf};

use crate::CordisPluginSpec;

/// The bridge runtime embedded into every plugin host page.
pub const BRIDGE_JS: &str = include_str!("bridge.js");

/// Generated DSH client slot catalog (122 declared slots: kind/scope/framework
/// standard kit), trimmed from the reference repository's
/// `cordis-client-runner/src/client/slot-catalog.ts`. The host page publishes it
/// as `window.__DSHR_SLOT_META__`, and the bridge validates registrations against
/// it exactly like the reference SlotCore.
pub const SLOT_CATALOG_JSON: &str = include_str!("slot-catalog.json");

/// How the browser half is delivered into the frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BundleMode {
    /// One self-contained file, base64-inlined; the frame makes no requests.
    Inline,
    /// A module graph fetched from a capability-addressed asset route, so
    /// relative imports and dynamic chunks resolve as the bundler emitted them.
    Assets,
}

/// Which job a client frame performs in the shell.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FrameMode {
    /// One hidden frame per client half: publish registrations, render on command.
    #[default]
    Registry,
    /// One frame per slot registration: render exactly the requested registration.
    Slot,
}

/// Per-frame bridge configuration embedded into the generated host page.
///
/// The shell builds one for every frame it creates. `frame` is the opaque token
/// echoed by every bridge message, while the slot fields are only used by slot
/// mode; `context` stays a JSON string until the bridge parses it into the
/// render props.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize)]
pub struct FrameSpec {
    pub frame: String,
    pub mode: FrameMode,
    pub slot: String,
    pub id: String,
    pub context: String,
    pub theme: String,
}

/// Version of the host-provided module-table runtime (tier B PoC).
///
/// The Web UI serves exactly this versioned artifact at [`host_runtime_url`] with
/// an immutable cache header, so every frame references one URL and the browser
/// downloads React once per page instead of each half embedding its own copy.
pub const HOST_RUNTIME_VERSION: &str = "18.3.1";

/// Module ids the shared host runtime publishes in `window.__DSHR_MODULES__`.
pub const HOST_MODULES: [&str; 6] = [
    "react",
    "react/jsx-runtime",
    "react/jsx-dev-runtime",
    "react-dom",
    "react-dom/client",
    "@deepseek-ai/dsh-client-ui-primitives",
];

/// URL of the shared host runtime, relative to the server origin.
#[must_use]
pub fn host_runtime_url() -> String {
    format!("/v1/runtime/react-{HOST_RUNTIME_VERSION}.js")
}

/// Extra scripts a host page runs before the plugin bundle, in this order:
/// host-provided module table, then the half's own runtime (which may override
/// entries), then the bundle itself.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PageRuntime<'a> {
    /// Same-origin URL of the shared host module table (`dsh.client.host_modules`).
    pub shared: Option<&'a str>,
    /// Plugin-shipped module table source (`dsh.client.runtime`).
    pub plugin: Option<&'a [u8]>,
}

/// A plugin's browser half, resolved to a concrete bundle file.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct ClientHalf {
    /// Plugin name from the config.
    pub name: String,
    /// npm package name from `package.json`, when present.
    pub package: Option<String>,
    pub platform: String,
    /// Load the bundle even when the shell never opens its panel.
    pub immediately: bool,
    /// Activation ordering hints from `dsh.client.inject` (kept as metadata).
    pub inject: Vec<String>,
    /// Bundle path relative to the package directory, for display.
    pub entry: String,
    /// How to deliver the bundle into the frame.
    pub bundle: BundleMode,
    /// Optional plugin-provided runtime script (`dsh.client.runtime`), executed
    /// before the bundle. It is the plugin's own module table: it may publish
    /// `window.__DSHR_MODULES__` so bundles built against the reference
    /// `__ModuleLoader__`/`require` contract can resolve e.g. a bundled React.
    pub runtime: Option<String>,
    /// Module ids the half expects from the shared host runtime
    /// (`dsh.client.host_modules`); non-empty means the host page loads the
    /// versioned host module table before the bundle.
    pub host_modules: Vec<String>,
    /// Absolute, canonicalized runtime path (confined to the package).
    #[serde(skip)]
    pub runtime_path: Option<PathBuf>,
    /// Absolute, canonicalized bundle path.
    #[serde(skip)]
    pub path: PathBuf,
    /// Canonical package directory (asset reads are confined to it).
    #[serde(skip)]
    pub directory: PathBuf,
}

#[derive(Debug, thiserror::Error)]
pub enum ClientHalfError {
    #[error("plugin '{0}' has no package.json")]
    MissingManifest(String),

    #[error("plugin '{0}' declares no dsh.client browser half")]
    NoClientHalf(String),

    #[error("plugin '{0}': {1}")]
    Io(String, String),

    #[error("plugin '{0}': {1}")]
    Invalid(String, String),
}

/// Resolves the browser bundle declared by a plugin package.
///
/// Looks for `dsh.client.entry` first, then the `exports["./client"]` target
/// (browser/import/default/require keys, in that order).
pub fn discover_client_half(spec: &CordisPluginSpec) -> Result<ClientHalf, ClientHalfError> {
    let name = spec.name.clone();
    let directory = spec.package_dir();
    let manifest_path = directory.join("package.json");
    let text = std::fs::read_to_string(&manifest_path).map_err(|_| {
        ClientHalfError::MissingManifest(format!("{} ({})", name, manifest_path.display()))
    })?;
    let manifest: serde_json::Value = serde_json::from_str(&text).map_err(|error| {
        ClientHalfError::Invalid(name.clone(), format!("package.json: {error}"))
    })?;
    let client = manifest
        .get("dsh")
        .and_then(|dsh| dsh.get("client"))
        .ok_or_else(|| ClientHalfError::NoClientHalf(name.clone()))?;
    let client = client.as_object().ok_or_else(|| {
        ClientHalfError::Invalid(name.clone(), "dsh.client must be an object".into())
    })?;

    let platform = client
        .get("platform")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("web")
        .to_owned();
    if platform != "web" {
        return Err(ClientHalfError::Invalid(
            name.clone(),
            format!("dsh.client.platform '{platform}' is not supported (expected 'web')"),
        ));
    }

    let declared = client
        .get("entry")
        .and_then(serde_json::Value::as_str)
        .map(ToOwned::to_owned)
        .or_else(|| {
            manifest
                .get("exports")
                .and_then(|exports| export_target(exports.get("./client")?))
        })
        .ok_or_else(|| {
            ClientHalfError::Invalid(
                name.clone(),
                "dsh.client needs an entry or an exports[\"./client\"] target".into(),
            )
        })?;

    let relative = Path::new(&declared);
    if relative.is_absolute()
        || relative.components().any(|part| {
            matches!(
                part,
                std::path::Component::ParentDir | std::path::Component::RootDir
            )
        })
    {
        return Err(ClientHalfError::Invalid(
            name.clone(),
            format!("client entry '{declared}' must be package-relative"),
        ));
    }
    let path = directory.join(relative);
    let canonical_root = std::fs::canonicalize(&directory).map_err(|error| {
        ClientHalfError::Io(name.clone(), format!("{}: {error}", directory.display()))
    })?;
    let canonical = std::fs::canonicalize(&path).map_err(|error| {
        ClientHalfError::Io(
            name.clone(),
            format!("client entry {}: {error}", path.display()),
        )
    })?;
    if !canonical.starts_with(&canonical_root) {
        return Err(ClientHalfError::Invalid(
            name.clone(),
            format!("client entry '{declared}' escapes the plugin package"),
        ));
    }
    if !canonical.is_file() {
        return Err(ClientHalfError::Invalid(
            name.clone(),
            format!("client entry '{declared}' is not a file"),
        ));
    }

    let bundle = match client
        .get("bundle")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("auto")
    {
        "inline" => BundleMode::Inline,
        "assets" => BundleMode::Assets,
        _ => {
            // A bundle with relative specifiers cannot be inlined: blob URLs
            // have no base for `./chunk.js`. Detect that instead of making
            // every author declare a mode.
            let text = std::fs::read_to_string(&canonical)
                .map(|text| text.chars().take(2 * 1024 * 1024).collect::<String>())
                .unwrap_or_default();
            if declares_relative_imports(&text) {
                BundleMode::Assets
            } else {
                BundleMode::Inline
            }
        }
    };

    // `dsh.client.host_modules` opts into the shared host module table (tier B):
    // every requested id must be one the host runtime provides. The array is
    // deduplicated but keeps the declared order for display.
    let host_modules = match client.get("host_modules") {
        None => Vec::new(),
        Some(value) => {
            let list = value.as_array().ok_or_else(|| {
                ClientHalfError::Invalid(
                    name.clone(),
                    "dsh.client.host_modules must be an array of module ids".into(),
                )
            })?;
            let mut modules: Vec<String> = Vec::new();
            for entry in list {
                let id = entry.as_str().ok_or_else(|| {
                    ClientHalfError::Invalid(
                        name.clone(),
                        "dsh.client.host_modules entries must be strings".into(),
                    )
                })?;
                if !HOST_MODULES.contains(&id) {
                    return Err(ClientHalfError::Invalid(
                        name.clone(),
                        format!(
                            "host module '{id}' is not provided by the host (known: {})",
                            HOST_MODULES.join(", ")
                        ),
                    ));
                }
                if !modules.iter().any(|seen| seen == id) {
                    modules.push(id.to_owned());
                }
            }
            modules
        }
    };

    // `dsh.client.runtime` is a classic script shipped with the half. It runs
    // before the bundle in the same sandboxed document and is the plugin's own
    // module table (a bundled React, a mini framework, ...). Path rules are the
    // same as for the bundle: package-relative, existing, confined to the package.
    let runtime = match client.get("runtime") {
        None => None,
        Some(value) => {
            let declared = value.as_str().ok_or_else(|| {
                ClientHalfError::Invalid(
                    name.clone(),
                    "dsh.client.runtime must be a package-relative string".into(),
                )
            })?;
            let relative = Path::new(declared);
            if declared.is_empty()
                || relative.is_absolute()
                || relative.components().any(|part| {
                    matches!(
                        part,
                        std::path::Component::ParentDir | std::path::Component::RootDir
                    )
                })
            {
                return Err(ClientHalfError::Invalid(
                    name.clone(),
                    format!("client runtime '{declared}' must be package-relative"),
                ));
            }
            let path = directory.join(relative);
            let canonical = std::fs::canonicalize(&path).map_err(|error| {
                ClientHalfError::Io(
                    name.clone(),
                    format!("client runtime {}: {error}", path.display()),
                )
            })?;
            if !canonical.starts_with(&canonical_root) {
                return Err(ClientHalfError::Invalid(
                    name.clone(),
                    format!("client runtime '{declared}' escapes the plugin package"),
                ));
            }
            if !canonical.is_file() {
                return Err(ClientHalfError::Invalid(
                    name.clone(),
                    format!("client runtime '{declared}' is not a file"),
                ));
            }
            Some((declared.trim_start_matches("./").to_owned(), canonical))
        }
    };

    Ok(ClientHalf {
        name,
        host_modules,
        runtime: runtime.as_ref().map(|(relative, _)| relative.clone()),
        runtime_path: runtime.map(|(_, path)| path),
        package: manifest
            .get("name")
            .and_then(serde_json::Value::as_str)
            .map(ToOwned::to_owned),
        platform,
        immediately: client
            .get("immediately")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false),
        inject: client
            .get("inject")
            .and_then(serde_json::Value::as_array)
            .map(|entries| {
                entries
                    .iter()
                    .filter_map(serde_json::Value::as_str)
                    .map(ToOwned::to_owned)
                    .collect()
            })
            .unwrap_or_default(),
        entry: relative
            .to_string_lossy()
            .trim_start_matches("./")
            .to_owned(),
        bundle,
        path: canonical,
        directory: canonical_root,
    })
}

/// True when the bundle references a sibling file, i.e. cannot be inlined as a
/// single blob module.
#[must_use]
pub fn declares_relative_imports(source: &str) -> bool {
    const SPECIFIERS: [&str; 12] = [
        "from \"./",
        "from './",
        "from \"../",
        "from '../",
        "import(\"./",
        "import('./",
        "import(\"../",
        "import('../",
        "require(\"./",
        "require('./",
        "require(\"../",
        "require('../",
    ];
    SPECIFIERS
        .iter()
        .any(|specifier| source.contains(specifier))
}

/// Resolves a capability-addressed asset path inside the plugin package.
pub fn resolve_asset(half: &ClientHalf, relative: &str) -> Result<PathBuf, ClientHalfError> {
    let relative = Path::new(relative);
    if relative.is_absolute()
        || relative.components().any(|part| {
            matches!(
                part,
                std::path::Component::ParentDir | std::path::Component::RootDir
            )
        })
    {
        return Err(ClientHalfError::Invalid(
            half.name.clone(),
            format!("asset '{relative:?}' must be package-relative"),
        ));
    }
    let path = half.directory.join(relative);
    let canonical = std::fs::canonicalize(&path).map_err(|error| {
        ClientHalfError::Io(
            half.name.clone(),
            format!("asset {}: {error}", path.display()),
        )
    })?;
    if !canonical.starts_with(&half.directory) || !canonical.is_file() {
        return Err(ClientHalfError::Invalid(
            half.name.clone(),
            format!("asset '{relative:?}' escapes the plugin package"),
        ));
    }
    Ok(canonical)
}

/// Follows an `exports` target: a string, or an object whose `browser`,
/// `import`, `default` or `require` key holds the actual path.
fn export_target(value: &serde_json::Value) -> Option<String> {
    if let Some(text) = value.as_str() {
        return Some(text.to_owned());
    }
    let object = value.as_object()?;
    for key in ["browser", "import", "default", "require"] {
        if let Some(target) = object.get(key).and_then(export_target) {
            return Some(target);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use dshr_config::CordisGrants;

    fn package(dir: &Path, manifest: &str, files: &[(&str, &str)]) -> CordisPluginSpec {
        std::fs::write(dir.join("package.json"), manifest).expect("manifest");
        for (name, contents) in files {
            let path = dir.join(name);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).expect("parent");
            }
            std::fs::write(path, contents).expect("bundle");
        }
        CordisPluginSpec {
            name: "demo".into(),
            entry: dir.to_path_buf(),
            root: dir.to_path_buf(),
            env: Default::default(),
            grants: CordisGrants::default(),
        }
    }

    #[test]
    fn resolves_dsh_client_entry() {
        let dir = tempfile::tempdir().expect("tempdir");
        let spec = package(
            dir.path(),
            r#"{"name":"@acme/demo","dsh":{"client":{"platform":"web","entry":"dist/client.js","immediately":true,"inject":["other"]}}}"#,
            &[("dist/client.js", "export default () => {}")],
        );
        let half = discover_client_half(&spec).expect("client half");
        assert_eq!(half.package.as_deref(), Some("@acme/demo"));
        assert_eq!(half.entry, "dist/client.js");
        assert!(half.immediately);
        assert_eq!(half.inject, vec!["other".to_owned()]);
        assert!(half.path.is_absolute());
    }

    #[test]
    fn resolves_the_plugin_runtime_script() {
        let dir = tempfile::tempdir().expect("tempdir");
        let spec = package(
            dir.path(),
            r#"{"name":"demo","dsh":{"client":{"platform":"web","entry":"client.js","runtime":"runtime/modules.js"}}}"#,
            &[
                ("client.js", "export default 1;"),
                ("runtime/modules.js", "window.__DSHR_MODULES__ = {};"),
            ],
        );
        let half = discover_client_half(&spec).expect("client half");
        assert_eq!(half.runtime.as_deref(), Some("runtime/modules.js"));
        let runtime = half.runtime_path.as_ref().expect("runtime path");
        assert!(runtime.ends_with("runtime/modules.js"), "{runtime:?}");
        assert!(runtime.is_absolute(), "{runtime:?}");
    }

    #[test]
    fn rejects_a_runtime_outside_the_package() {
        let dir = tempfile::tempdir().expect("tempdir");
        let outside = tempfile::tempdir().expect("outside");
        std::fs::write(outside.path().join("modules.js"), "window.x = 1;").expect("outside");
        let spec = package(
            dir.path(),
            r#"{"name":"demo","dsh":{"client":{"platform":"web","entry":"client.js","runtime":"../modules.js"}}}"#,
            &[("client.js", "export default 1;")],
        );
        let error = discover_client_half(&spec).expect_err("traversal rejected");
        assert!(
            matches!(error, ClientHalfError::Invalid(_, ref message) if message.contains("package-relative")),
            "{error}"
        );
    }

    #[test]
    fn resolves_host_modules_from_the_shared_runtime() {
        let dir = tempfile::tempdir().expect("tempdir");
        let spec = package(
            dir.path(),
            r#"{"name":"demo","dsh":{"client":{"platform":"web","entry":"client.js","host_modules":["react","react-dom/client","react"]}}}"#,
            &[("client.js", "export default 1;")],
        );
        let half = discover_client_half(&spec).expect("client half");
        assert_eq!(
            half.host_modules,
            vec!["react".to_owned(), "react-dom/client".to_owned()]
        );
        assert!(
            half.runtime.is_none(),
            "host modules are not a shipped runtime"
        );
    }

    #[test]
    fn accepts_the_design_system_primitives_shim() {
        let dir = tempfile::tempdir().expect("tempdir");
        let spec = package(
            dir.path(),
            r#"{"name":"demo","dsh":{"client":{"platform":"web","entry":"client.js","host_modules":["react","@deepseek-ai/dsh-client-ui-primitives"]}}}"#,
            &[("client.js", "export default 1;")],
        );
        let half = discover_client_half(&spec).expect("client half");
        assert!(
            half.host_modules
                .iter()
                .any(|id| id == "@deepseek-ai/dsh-client-ui-primitives"),
            "{:?}",
            half.host_modules
        );
    }

    #[test]
    fn rejects_a_host_module_the_host_does_not_provide() {
        let dir = tempfile::tempdir().expect("tempdir");
        let spec = package(
            dir.path(),
            r#"{"name":"demo","dsh":{"client":{"platform":"web","entry":"client.js","host_modules":["react","vue"]}}}"#,
            &[("client.js", "export default 1;")],
        );
        let error = discover_client_half(&spec).expect_err("unknown module rejected");
        assert!(
            matches!(error, ClientHalfError::Invalid(_, ref message) if message.contains("not provided by the host")),
            "{error}"
        );
    }

    #[test]
    fn falls_back_to_the_client_export() {
        let dir = tempfile::tempdir().expect("tempdir");
        let spec = package(
            dir.path(),
            r#"{"name":"demo","dsh":{"client":{"platform":"web"}},"exports":{"./client":{"browser":"./browser.js","default":"./node.js"}}}"#,
            &[("browser.js", "export default () => {}"), ("node.js", "")],
        );
        let half = discover_client_half(&spec).expect("client half");
        assert_eq!(half.entry, "browser.js");
        assert!(!half.immediately);
    }

    #[test]
    fn rejects_paths_outside_the_package() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("package.json"), r#"{"name":"demo"}"#).expect("manifest");
        let outside = tempfile::tempdir().expect("outside");
        std::fs::write(outside.path().join("evil.js"), "export default 1").expect("evil");
        let spec = package(
            dir.path(),
            r#"{"name":"demo","dsh":{"client":{"platform":"web","entry":"../evil.js"}}}"#,
            &[],
        );
        let error = discover_client_half(&spec).expect_err("traversal rejected");
        assert!(
            matches!(error, ClientHalfError::Invalid(_, ref message) if message.contains("package-relative")),
            "{error}"
        );
    }

    #[test]
    fn rejects_non_web_platform_and_missing_manifest() {
        let dir = tempfile::tempdir().expect("tempdir");
        let spec = package(
            dir.path(),
            r#"{"name":"demo","dsh":{"client":{"platform":"desktop","entry":"client.js"}}}"#,
            &[("client.js", "")],
        );
        assert!(matches!(
            discover_client_half(&spec),
            Err(ClientHalfError::Invalid(_, _))
        ));

        let empty = tempfile::tempdir().expect("empty");
        let spec = CordisPluginSpec {
            name: "missing".into(),
            entry: empty.path().to_path_buf(),
            root: empty.path().to_path_buf(),
            env: Default::default(),
            grants: CordisGrants::default(),
        };
        assert!(matches!(
            discover_client_half(&spec),
            Err(ClientHalfError::MissingManifest(_))
        ));
    }

    #[test]
    fn rejects_plugins_without_a_client_half() {
        let dir = tempfile::tempdir().expect("tempdir");
        let spec = package(dir.path(), r#"{"name":"host-only"}"#, &[]);
        assert!(matches!(
            discover_client_half(&spec),
            Err(ClientHalfError::NoClientHalf(_))
        ));
    }
}

/// Renders the sandboxed iframe host page for one client half.
///
/// Both the bridge and the bundle are inlined: the frame runs with an opaque
/// origin (`sandbox="allow-scripts"` without `allow-same-origin`), so it cannot
/// fetch the bundle with the shell's token and `'self'` does not match. The
/// bundle is base64-encoded into the page and imported from a blob URL, which
/// keeps the frame network-free — the CSP below allows no requests at all.
///
/// `spec` becomes `window.__DSHR_FRAME__`, the bridge's v1.2 frame config.
#[must_use]
pub fn host_page(
    half: &ClientHalf,
    bundle: &[u8],
    spec: &FrameSpec,
    runtime: PageRuntime<'_>,
) -> String {
    use base64::Engine as _;
    let encoded = base64::engine::general_purpose::STANDARD.encode(bundle);
    page(
        half,
        spec,
        &format!(
            "URL.createObjectURL(new Blob([Uint8Array.from(atob(\"{encoded}\"), (char) => char.charCodeAt(0))], {{ type: \"text/javascript\" }}))"
        ),
        true,
        runtime,
    )
}

/// Host page for a multi-file bundle.
///
/// The entry is fetched from the capability-addressed asset route, so its
/// relative imports and dynamic chunks resolve against that URL; the CSP for
/// this page allows scripts from that origin only, and no other network.
#[must_use]
pub fn host_page_assets(
    half: &ClientHalf,
    asset_base: &str,
    spec: &FrameSpec,
    runtime: PageRuntime<'_>,
) -> String {
    let base = js_string(asset_base);
    let entry = js_string(&half.entry);
    page(
        half,
        spec,
        &format!("({base} + \"/\" + encodeURI({entry}).replace(/^\\.\\//, \"\"))"),
        false,
        runtime,
    )
}

/// The frame config object as embedded into the page. Field order matters only
/// for readability; the bridge reads it by key.
#[derive(serde::Serialize)]
struct FrameConfig<'a> {
    plugin: &'a str,
    frame: &'a str,
    mode: FrameMode,
    slot: &'a str,
    id: &'a str,
    context: &'a str,
    theme: &'a str,
}

fn page(
    half: &ClientHalf,
    spec: &FrameSpec,
    url_expression: &str,
    revoke: bool,
    runtime: PageRuntime<'_>,
) -> String {
    let name = js_string(&half.name);
    let theme_value = if spec.theme == "light" {
        "light"
    } else {
        "dark"
    };
    let theme = js_string(theme_value);
    let revoke = if revoke {
        "URL.revokeObjectURL(url);"
    } else {
        ""
    };
    // The bridge config is a JSON object (not a JS literal) and `context` stays
    // a JSON string inside it; `<` is escaped so no value can close the tag.
    let slot_meta = escape_script(SLOT_CATALOG_JSON);
    let frame_config = escape_script(
        &serde_json::to_string(&FrameConfig {
            plugin: &half.name,
            frame: &spec.frame,
            mode: spec.mode,
            slot: &spec.slot,
            id: &spec.id,
            context: &spec.context,
            theme: theme_value,
        })
        .unwrap_or_default(),
    );
    // The shared host module table (tier B) is one versioned, cached artifact the
    // frame loads from the server origin; it runs before the plugin's own runtime
    // so the plugin can override or extend entries.
    let shared_script = runtime.shared.map_or_else(String::new, |url| {
        format!(
            "<script src=\"{}\" data-dshr-shared-runtime=\"1\"></script>\n",
            html_attribute(url)
        )
    });
    // The plugin's own runtime script runs before the bundle. It is raw JS from
    // the plugin package; callers reject a payload containing `</script>` so it
    // cannot escape the script element.
    let runtime_script = runtime.plugin.map_or_else(String::new, |source| {
        format!(
            "<script>\n/* dshr plugin runtime (dsh.client.runtime): may publish window.__DSHR_MODULES__ */\n{}\n</script>\n",
            String::from_utf8_lossy(source)
        )
    });
    format!(
        r#"<!doctype html>
<html lang="en" data-theme={theme}>
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>dshr plugin</title>
<style>
  :root {{ color-scheme: dark; --dshr-fg: #e6e6e6; --dshr-muted: #9b9b9b; --dshr-bg: transparent; }}
  :root[data-theme="light"] {{ color-scheme: light; --dshr-fg: #1c1c1c; --dshr-muted: #5a5a5a; }}
  html, body {{ margin: 0; padding: 0; background: var(--dshr-bg); color: var(--dshr-fg);
    font: 14px/1.45 -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; }}
  #dshr-slot-root {{ padding: 12px 14px; }}
  #dshr-slot-root > * {{ max-width: 100%; }}
</style>
</head>
<body>
<script>window.__DSHR_PLUGIN__ = {name};</script>
<script>window.__DSHR_FRAME__ = {frame_config};</script>
<script>window.__DSHR_SLOT_META__ = {slot_meta};</script>
<script>
{BRIDGE_JS}
</script>
{shared_script}{runtime_script}
<script type="module">
const url = {url_expression};

// `require` stand-in for bundles built against the reference module table.
// The plugin's own runtime (`dsh.client.runtime`) may publish modules here, so a
// half can ship a bundled React/mini-framework and resolve it by id; a request
// for a module the half did not ship fails loudly instead of returning undefined.
function requireShim(specifier) {{
  const modules = window.__DSHR_MODULES__;
  if (modules && Object.prototype.hasOwnProperty.call(modules, specifier)) {{
    return modules[specifier];
  }}
  throw new Error(
    "module '" + specifier + "' is not available in the dshr client bridge"
      + (modules ? "" : " (declare dsh.client.runtime to ship your own module table)")
  );
}}

function activateModule(module) {{
  const loaded = window.dshr.__loaded || [];
  for (const definition of loaded) {{
    const exported = definition.factory(requireShim);
    if (exported && typeof exported.apply === "function") return exported.apply(window.dshr);
  }}
  const exported = module && (module.default || module);
  if (typeof exported === "function") return exported(window.dshr);
  if (exported && typeof exported.apply === "function") return exported.apply(window.dshr);
  if (exported && typeof exported.activate === "function") return exported.activate(window.dshr);
  return undefined;
}}

import(url).then(function (module) {{
  return Promise.resolve(activateModule(module)).then(function () {{
    document.documentElement.dataset.dshrReady = "1";
    if (window.dshr.slots.list().length === 0) {{
      window.dshr.log("plugin bundle registered no slots");
    }}
  }});
}}).catch(function (error) {{
  document.documentElement.dataset.dshrReady = "error";
  window.dshr.__fail(error);
}}).finally(function () {{
  // The bridge announces `ready` (and resolves its slot frame) only once the
  // bundle activation settles, so the shell never races the registrations.
  if (window.dshr && typeof window.dshr.__frameActivated === "function") {{
    window.dshr.__frameActivated();
  }}
  {revoke}
}});
</script>
</body>
</html>
"#,
    )
}

fn escape_script(text: &str) -> String {
    // Escape `<` so no injected value can close the surrounding script element.
    text.replace('<', "\\u003c")
}

fn html_attribute(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn js_string(text: &str) -> String {
    // JSON strings are valid JS string literals.
    escape_script(&serde_json::to_string(text).unwrap_or_else(|_| "\"\"".to_owned()))
}

#[cfg(test)]
mod host_page_tests {
    use super::*;

    fn half(name: &str) -> ClientHalf {
        ClientHalf {
            name: name.to_owned(),
            package: Some("@acme/demo".into()),
            platform: "web".into(),
            immediately: false,
            inject: Vec::new(),
            entry: "dist/client.js".into(),
            bundle: BundleMode::Inline,
            runtime: None,
            runtime_path: None,
            host_modules: Vec::new(),
            path: PathBuf::from("/tmp/client.js"),
            directory: PathBuf::from("/tmp"),
        }
    }

    fn registry(theme: &str) -> FrameSpec {
        FrameSpec {
            frame: "registry:demo".into(),
            theme: theme.into(),
            ..FrameSpec::default()
        }
    }

    #[test]
    fn host_page_inlines_the_bridge_and_escapes_values() {
        let page = host_page(
            &half("de\"mo<script>"),
            b"export default 1;",
            &registry("light"),
            PageRuntime::default(),
        );
        assert!(page.contains("dshr-client")); // bridge protocol namespace
        assert!(page.contains("data-theme=\"light\""), "{page}");
        assert!(page.contains("de\\\"mo\\u003cscript>"), "{page}");
        assert!(page.contains("ZXhwb3J0IGRlZmF1bHQgMTs="), "{page}"); // base64 bundle
        assert!(
            page.contains("__DSHR_PLUGIN__ = \"de\\\"mo\\u003cscript>\""),
            "{page}"
        );
        assert!(!page.contains("de\"mo<script>"), "{page}");
    }

    #[test]
    fn host_page_injects_the_plugin_runtime_before_the_bundle() {
        let page = host_page(
            &half("demo"),
            b"export default 1;",
            &registry("dark"),
            PageRuntime {
                plugin: Some(b"window.__DSHR_MODULES__ = { framework: {} };"),
                ..PageRuntime::default()
            },
        );
        assert!(page.contains("dshr plugin runtime"), "{page}");
        let runtime_at = page.find("dshr plugin runtime").expect("runtime marker");
        let bundle_at = page.find("const url = ").expect("bundle loader");
        assert!(runtime_at < bundle_at, "runtime must run before the bundle");
        assert!(
            page.contains("Object.prototype.hasOwnProperty.call(modules, specifier)"),
            "{page}"
        );
    }

    #[test]
    fn host_page_loads_the_shared_runtime_before_the_plugin_runtime() {
        let page = host_page(
            &half("demo"),
            b"export default 1;",
            &registry("dark"),
            PageRuntime {
                shared: Some("/v1/runtime/react-18.3.1.js"),
                plugin: Some(b"window.__DSHR_MODULES__ = { custom: {} };"),
            },
        );
        assert!(
            page.contains(r#"<script src="/v1/runtime/react-18.3.1.js" data-dshr-shared-runtime="1"></script>"#),
            "{page}"
        );
        let shared_at = page
            .find("data-dshr-shared-runtime")
            .expect("shared runtime");
        let plugin_at = page.find("dshr plugin runtime").expect("plugin runtime");
        let bundle_at = page.find("const url = ").expect("bundle loader");
        assert!(
            shared_at < plugin_at,
            "shared runtime runs before the plugin runtime"
        );
        assert!(
            plugin_at < bundle_at,
            "plugin runtime runs before the bundle"
        );
    }

    #[test]
    fn host_page_omits_the_shared_runtime_when_not_requested() {
        let page = host_page(
            &half("demo"),
            b"",
            &FrameSpec::default(),
            PageRuntime::default(),
        );
        assert!(!page.contains("data-dshr-shared-runtime"), "{page}");
        assert!(!page.contains("/v1/runtime/"), "{page}");
    }

    #[test]
    fn host_page_without_a_runtime_keeps_the_module_table_error() {
        let page = host_page(
            &half("demo"),
            b"",
            &FrameSpec::default(),
            PageRuntime::default(),
        );
        assert!(!page.contains("dshr plugin runtime"), "{page}");
        assert!(
            page.contains("declare dsh.client.runtime to ship your own module table"),
            "{page}"
        );
    }

    #[test]
    fn host_page_embeds_the_registry_config() {
        let page = host_page(
            &half("demo"),
            b"export default 1;",
            &registry("dark"),
            PageRuntime::default(),
        );
        assert!(page.contains("window.__DSHR_FRAME__ = "), "{page}");
        assert!(page.contains("\"plugin\":\"demo\""), "{page}");
        assert!(page.contains("\"frame\":\"registry:demo\""), "{page}");
        assert!(page.contains("\"mode\":\"registry\""), "{page}");
        assert!(page.contains("\"theme\":\"dark\""), "{page}");
        assert!(page.contains("data-theme=\"dark\""), "{page}");
        // The slot-only fields stay empty for a registry frame.
        assert!(page.contains("\"slot\":\"\""), "{page}");
        assert!(page.contains("\"id\":\"\""), "{page}");
        assert!(page.contains("\"context\":\"\""), "{page}");
        assert!(page.contains("__frameActivated"), "{page}");
        // A default spec is a registry frame in dark mode: the pre-v1.2 API had
        // only a theme, so the old call sites map onto these defaults.
        let default_page = host_page(
            &half("demo"),
            b"",
            &FrameSpec::default(),
            PageRuntime::default(),
        );
        assert!(
            default_page.contains("\"mode\":\"registry\""),
            "{default_page}"
        );
        assert!(
            default_page.contains("data-theme=\"dark\""),
            "{default_page}"
        );
    }

    #[test]
    fn host_page_embeds_the_slot_config() {
        let spec = FrameSpec {
            frame: "slot:demo:demo-panel:0".into(),
            mode: FrameMode::Slot,
            slot: "panel".into(),
            id: "demo-panel".into(),
            context: r#"{"index":0}"#.into(),
            theme: "dark".into(),
        };
        let page = host_page(
            &half("demo"),
            b"export default 1;",
            &spec,
            PageRuntime::default(),
        );
        assert!(page.contains("\"mode\":\"slot\""), "{page}");
        assert!(
            page.contains("\"frame\":\"slot:demo:demo-panel:0\""),
            "{page}"
        );
        assert!(page.contains("\"slot\":\"panel\""), "{page}");
        assert!(page.contains("\"id\":\"demo-panel\""), "{page}");
        // `context` is a JSON string inside the config; the bridge parses it.
        assert!(page.contains(r#""context":"{\"index\":0}""#), "{page}");
    }

    #[test]
    fn host_page_escapes_script_breakers_in_the_config() {
        let spec = FrameSpec {
            frame: "slot:demo:x".into(),
            mode: FrameMode::Slot,
            slot: "panel".into(),
            id: "x</script><script>alert(1)</script>".into(),
            context: r#"{"html":"</script>"}"#.into(),
            theme: "light".into(),
        };
        let page = host_page(&half("demo"), b"", &spec, PageRuntime::default());
        // Every config value survives serialization without closing the script.
        assert!(!page.contains("x</script>"), "{page}");
        assert!(!page.contains(r#"{"html":"</script>"}"#), "{page}");
        assert!(page.contains(r"\u003c/script"), "{page}");
        assert!(page.contains("x\\u003c/script>"), "{page}");
        assert!(page.contains("data-theme=\"light\""), "{page}");
    }

    #[test]
    fn frame_spec_serializes_with_snake_case_modes() {
        let spec = FrameSpec {
            frame: "slot:demo:x:0".into(),
            mode: FrameMode::Slot,
            slot: "settings.section".into(),
            id: "x".into(),
            context: r#"{"a":1}"#.into(),
            theme: "dark".into(),
        };
        let value = serde_json::to_value(&spec).expect("serialize");
        assert_eq!(value["mode"], "slot");
        assert_eq!(value["frame"], "slot:demo:x:0");
        assert_eq!(value["context"], r#"{"a":1}"#);
        let default = serde_json::to_value(FrameSpec::default()).expect("serialize");
        assert_eq!(default["mode"], "registry");
        assert_eq!(FrameMode::default(), FrameMode::Registry);
        assert_eq!(
            default,
            serde_json::json!({
                "frame": "",
                "mode": "registry",
                "slot": "",
                "id": "",
                "context": "",
                "theme": "",
            })
        );
    }

    #[test]
    fn asset_host_page_imports_the_capability_url() {
        let mut half = half("demo");
        half.bundle = BundleMode::Assets;
        let page = host_page_assets(
            &half,
            "/v1/plugins/demo/asset/cap123",
            &registry("dark"),
            PageRuntime::default(),
        );
        assert!(page.contains("/v1/plugins/demo/asset/cap123"), "{page}");
        assert!(page.contains("encodeURI(\"dist/client.js\")"), "{page}");
        assert!(!page.contains("atob("), "{page}");
        assert!(!page.contains("revokeObjectURL"), "{page}");
        assert!(page.contains("data-theme=\"dark\""), "{page}");
        assert!(page.contains("\"frame\":\"registry:demo\""), "{page}");
    }
}

#[cfg(test)]
mod bundle_mode_tests {
    use super::*;

    fn package(dir: &std::path::Path, manifest: &str, files: &[(&str, &str)]) {
        std::fs::write(dir.join("package.json"), manifest).expect("manifest");
        for (name, contents) in files {
            let path = dir.join(name);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).expect("parent");
            }
            std::fs::write(path, contents).expect("asset");
        }
    }

    fn spec(dir: &std::path::Path) -> CordisPluginSpec {
        CordisPluginSpec {
            name: "demo".into(),
            entry: dir.to_path_buf(),
            root: dir.to_path_buf(),
            env: Default::default(),
            grants: crate::CordisGrants::default(),
        }
    }

    const MANIFEST: &str =
        r#"{"name":"demo","dsh":{"client":{"platform":"web","entry":"client.js"}}}"#;

    #[test]
    fn a_single_file_bundle_is_inlined() {
        let dir = tempfile::tempdir().expect("tempdir");
        package(
            dir.path(),
            MANIFEST,
            &[(
                "client.js",
                "export default function (ctx) { ctx.slots.list(); }",
            )],
        );
        let half = discover_client_half(&spec(dir.path())).expect("half");
        assert_eq!(half.bundle, BundleMode::Inline);
    }

    #[test]
    fn relative_imports_switch_to_assets() {
        let dir = tempfile::tempdir().expect("tempdir");
        package(
            dir.path(),
            MANIFEST,
            &[
                (
                    "client.js",
                    "import { panel } from \"./lib/panel.js\";\nexport default (ctx) => panel(ctx);",
                ),
                ("lib/panel.js", "export function panel() {}"),
            ],
        );
        let half = discover_client_half(&spec(dir.path())).expect("half");
        assert_eq!(half.bundle, BundleMode::Assets);
    }

    #[test]
    fn the_manifest_can_override_the_mode() {
        let dir = tempfile::tempdir().expect("tempdir");
        package(
            dir.path(),
            r#"{"name":"demo","dsh":{"client":{"platform":"web","entry":"client.js","bundle":"inline"}}}"#,
            &[("client.js", "import \"./lib.js\";")],
        );
        let half = discover_client_half(&spec(dir.path())).expect("half");
        assert_eq!(half.bundle, BundleMode::Inline);
    }

    #[test]
    fn assets_resolve_inside_the_package_only() {
        let dir = tempfile::tempdir().expect("tempdir");
        package(
            dir.path(),
            MANIFEST,
            &[
                ("client.js", "export default 1;"),
                ("lib/panel.js", "export const x = 1;"),
            ],
        );
        let half = discover_client_half(&spec(dir.path())).expect("half");
        let resolved = resolve_asset(&half, "lib/panel.js").expect("asset");
        assert!(resolved.ends_with("lib/panel.js"));
        assert!(resolve_asset(&half, "../escape.js").is_err());
        assert!(resolve_asset(&half, "/etc/passwd").is_err());
        assert!(resolve_asset(&half, "missing.js").is_err());
    }
    #[cfg(unix)]
    #[test]
    fn asset_symlink_escape_is_rejected() {
        use std::os::unix::fs::symlink;

        let dir = tempfile::tempdir().expect("tempdir");
        let outside = tempfile::tempdir().expect("outside");
        std::fs::write(outside.path().join("secret.txt"), "secret").expect("secret");
        package(dir.path(), MANIFEST, &[("client.js", "export default 1;")]);
        symlink(
            outside.path().join("secret.txt"),
            dir.path().join("leak.js"),
        )
        .expect("symlink");
        let half = discover_client_half(&spec(dir.path())).expect("half");
        assert!(
            resolve_asset(&half, "leak.js").is_err(),
            "a symlink that resolves outside the package must be rejected"
        );
    }
}

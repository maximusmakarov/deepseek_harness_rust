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

    Ok(ClientHalf {
        name,
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
#[must_use]
pub fn host_page(half: &ClientHalf, bundle: &[u8], theme: &str) -> String {
    use base64::Engine as _;
    let encoded = base64::engine::general_purpose::STANDARD.encode(bundle);
    page(
        half,
        theme,
        &format!(
            "URL.createObjectURL(new Blob([Uint8Array.from(atob(\"{encoded}\"), (char) => char.charCodeAt(0))], {{ type: \"text/javascript\" }}))"
        ),
        true,
    )
}

/// Host page for a multi-file bundle.
///
/// The entry is fetched from the capability-addressed asset route, so its
/// relative imports and dynamic chunks resolve against that URL; the CSP for
/// this page allows scripts from that origin only, and no other network.
#[must_use]
pub fn host_page_assets(half: &ClientHalf, asset_base: &str, theme: &str) -> String {
    let base = js_string(asset_base);
    let entry = js_string(&half.entry);
    page(
        half,
        theme,
        &format!("({base} + \"/\" + encodeURI({entry}).replace(/^\\.\\//, \"\"))"),
        false,
    )
}

fn page(half: &ClientHalf, theme: &str, url_expression: &str, revoke: bool) -> String {
    let name = js_string(&half.name);
    let theme = js_string(if theme == "light" { "light" } else { "dark" });
    let revoke = if revoke {
        "URL.revokeObjectURL(url);"
    } else {
        ""
    };
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
<script>
{BRIDGE_JS}
</script>
<script type="module">
const url = {url_expression};

// `require` stand-in for bundles built against the reference module table:
// React and other host modules are not available (ADR-013 §5), so a request
// for them fails loudly instead of returning undefined.
function requireShim(specifier) {{
  throw new Error("module '" + specifier + "' is not available in the dshr client bridge");
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
  {revoke}
}});
</script>
</body>
</html>
"#,
    )
}

fn js_string(text: &str) -> String {
    // JSON strings are valid JS string literals; escape `<` so a name can never
    // close the surrounding script element.
    serde_json::to_string(text)
        .unwrap_or_else(|_| "\"\"".to_owned())
        .replace('<', "\\u003c")
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
            path: PathBuf::from("/tmp/client.js"),
            directory: PathBuf::from("/tmp"),
        }
    }

    #[test]
    fn host_page_inlines_the_bridge_and_escapes_values() {
        let page = host_page(&half("de\"mo<script>"), b"export default 1;", "light");
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
    fn asset_host_page_imports_the_capability_url() {
        let mut half = half("demo");
        half.bundle = BundleMode::Assets;
        let page = host_page_assets(&half, "/v1/plugins/demo/asset/cap123", "dark");
        assert!(page.contains("/v1/plugins/demo/asset/cap123"), "{page}");
        assert!(page.contains("encodeURI(\"dist/client.js\")"), "{page}");
        assert!(!page.contains("atob("), "{page}");
        assert!(!page.contains("revokeObjectURL"), "{page}");
        assert!(page.contains("data-theme=\"dark\""), "{page}");
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

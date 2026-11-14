//! Supervisor tests: hot reload, config sync and status reporting.
//!
//! Every test starts a real node sidecar (the same path production uses) and
//! skips when node is unavailable.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use dshr_config::CordisGrants;
use dshr_cordis::manager::ToolFilter;
use dshr_cordis::{CordisManager, CordisPluginSpec, WatchOptions, manager::TOOL_PREFIX};
use dshr_core::SessionId;
use dshr_tools::{ToolCtx, ToolRegistry};
use serde_json::json;
use tokio_util::sync::CancellationToken;

fn node_available() -> bool {
    std::process::Command::new("node")
        .arg("--version")
        .output()
        .is_ok()
}

fn plugin_source(version: &str) -> String {
    format!(
        r#"
export default {{
  apply(ctx) {{
    ctx.tool({{
      name: "echo",
      description: "echo {version}",
      inputSchema: {{ type: "object" }},
      handler: () => "{version}",
    }});
  }},
}};
"#
    )
}

fn write_plugin(dir: &Path, version: &str) {
    std::fs::write(dir.join("plugin.mjs"), plugin_source(version)).expect("plugin");
}

fn spec(root: &Path, name: &str) -> CordisPluginSpec {
    CordisPluginSpec {
        name: name.to_owned(),
        entry: PathBuf::from("plugin.mjs"),
        root: root.to_path_buf(),
        env: Default::default(),
        grants: CordisGrants::default(),
    }
}

async fn call(registry: &ToolRegistry, name: &str) -> String {
    let tool = registry.get(name).expect("tool registered");
    let ctx = ToolCtx {
        cwd: std::env::temp_dir(),
        session_id: SessionId::new(),
        output_limit: 10_000,
        spill_dir: std::env::temp_dir().join("dshr-cordis-spills"),
        cancel: CancellationToken::new(),
        prompt: None,
    };
    tool.execute(&ctx, json!({}))
        .await
        .expect("tool call")
        .content
}

#[tokio::test]
async fn reload_swaps_tools_without_restarting_the_registry() {
    if !node_available() {
        eprintln!("node is not installed; skipping Cordis manager test");
        return;
    }
    let dir = tempfile::tempdir().expect("tempdir");
    write_plugin(dir.path(), "v1");
    let registry = Arc::new(ToolRegistry::new());
    let manager = CordisManager::new(
        Arc::clone(&registry),
        dir.path().to_path_buf(),
        dir.path().to_path_buf(),
        ToolFilter::default(),
    );

    manager
        .start(spec(dir.path(), "demo"))
        .await
        .expect("start");
    let tool = format!("{TOOL_PREFIX}demo__echo");
    assert_eq!(call(&registry, &tool).await, "v1");
    assert_eq!(manager.statuses().len(), 1);
    let status = &manager.statuses()[0];
    assert!(status.running && status.enabled);
    assert!(status.tools.iter().any(|name| name == &tool), "{status:?}");
    assert!(
        status
            .tools
            .iter()
            .all(|name| name.starts_with("cordis__demo__")),
        "{status:?}"
    );
    assert_eq!(status.error, None);

    // Edit the entry, then reload: the served tool must switch to the new
    // generation while the registry handle stays the same.
    write_plugin(dir.path(), "v2");
    manager.reload("demo").await.expect("reload");
    assert_eq!(call(&registry, &tool).await, "v2");
    assert!(manager.statuses()[0].generation > status.generation);

    manager.stop("demo").await;
    assert!(!registry.contains(&tool));
    let status = &manager.statuses()[0];
    assert!(status.enabled && !status.running);
    assert!(status.tools.is_empty());
}

#[tokio::test]
async fn sync_adds_removes_and_restarts_plugins() {
    if !node_available() {
        eprintln!("node is not installed; skipping Cordis manager test");
        return;
    }
    let dir = tempfile::tempdir().expect("tempdir");
    write_plugin(dir.path(), "v1");
    let registry = Arc::new(ToolRegistry::new());
    let manager = CordisManager::new(
        Arc::clone(&registry),
        dir.path().to_path_buf(),
        dir.path().to_path_buf(),
        ToolFilter::default(),
    );
    let demo = format!("{TOOL_PREFIX}demo__echo");

    manager.sync(vec![spec(dir.path(), "demo")]).await;
    assert!(registry.contains(&demo));

    // Removing the plugin from the config stops the sidecar and drops its
    // tools; nothing restarts until it comes back.
    manager.sync(Vec::new()).await;
    assert!(!registry.contains(&demo));
    assert!(!manager.is_running("demo"));
    assert!(manager.statuses().is_empty());

    // Re-adding it starts a fresh generation.
    write_plugin(dir.path(), "v3");
    manager.sync(vec![spec(dir.path(), "demo")]).await;
    assert_eq!(call(&registry, &demo).await, "v3");
}

#[tokio::test]
async fn a_changed_spec_is_reloaded_by_sync() {
    if !node_available() {
        eprintln!("node is not installed; skipping Cordis manager test");
        return;
    }
    let dir = tempfile::tempdir().expect("tempdir");
    write_plugin(dir.path(), "v2");
    let registry = Arc::new(ToolRegistry::new());
    let manager = CordisManager::new(
        Arc::clone(&registry),
        dir.path().to_path_buf(),
        dir.path().to_path_buf(),
        ToolFilter::default(),
    );
    let first = manager.statuses();
    assert!(first.is_empty());

    manager.sync(vec![spec(dir.path(), "demo")]).await;
    let generation = manager.statuses()[0].generation;

    let mut changed = spec(dir.path(), "demo");
    changed.env.insert("EXTRA".into(), "1".into());
    manager.sync(vec![changed]).await;
    assert!(manager.statuses()[0].generation > generation);
}

#[tokio::test]
async fn a_failed_start_is_reported_and_retried_later() {
    if !node_available() {
        eprintln!("node is not installed; skipping Cordis manager test");
        return;
    }
    let dir = tempfile::tempdir().expect("tempdir");
    write_plugin(dir.path(), "v1");
    let registry = Arc::new(ToolRegistry::new());
    let manager = CordisManager::new(
        Arc::clone(&registry),
        dir.path().to_path_buf(),
        dir.path().to_path_buf(),
        ToolFilter::default(),
    );

    let mut broken = spec(dir.path(), "demo");
    broken.entry = PathBuf::from("missing.mjs");
    manager
        .start(broken.clone())
        .await
        .expect_err("missing entry");
    let status = &manager.statuses()[0];
    assert!(status.enabled && !status.running);
    assert!(status.error.is_some(), "{status:?}");
    assert!(registry.names().is_empty());

    // Fixing the entry and reloading recovers without a restart.
    manager
        .start(spec(dir.path(), "demo"))
        .await
        .expect("start");
    let status = &manager.statuses()[0];
    assert!(status.running);
    assert_eq!(status.error, None);
}

#[tokio::test]
async fn watches_entry_files_and_reloads() {
    if !node_available() {
        eprintln!("node is not installed; skipping Cordis manager test");
        return;
    }
    let dir = tempfile::tempdir().expect("tempdir");
    write_plugin(dir.path(), "v1");
    let registry = Arc::new(ToolRegistry::new());
    let manager = CordisManager::new(
        Arc::clone(&registry),
        dir.path().to_path_buf(),
        dir.path().to_path_buf(),
        ToolFilter::default(),
    );
    manager.sync(vec![spec(dir.path(), "demo")]).await;
    let tool = format!("{TOOL_PREFIX}demo__echo");
    assert_eq!(call(&registry, &tool).await, "v1");
    let generation = manager.statuses()[0].generation;

    let watch = manager.spawn_watch(WatchOptions {
        interval: Duration::from_millis(100),
        config_files: Vec::new(),
        refresh: None,
    });

    // Give the watcher one tick to prime its stamps, then edit the entry.
    tokio::time::sleep(Duration::from_millis(250)).await;
    write_plugin(dir.path(), "v4");

    let reloaded = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            if call(&registry, &tool).await == "v4" {
                return true;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .unwrap_or(false);
    watch.abort();
    assert!(reloaded, "watcher did not pick up the edited entry");
    assert!(manager.statuses()[0].generation > generation);
}

#[tokio::test]
async fn watcher_reconciles_plugins_from_a_refreshed_config() {
    if !node_available() {
        eprintln!("node is not installed; skipping Cordis manager test");
        return;
    }
    let dir = tempfile::tempdir().expect("tempdir");
    write_plugin(dir.path(), "v1");
    let config = dir.path().join("profile.yaml");
    std::fs::write(&config, "plugins: {}\n").expect("config");
    let registry = Arc::new(ToolRegistry::new());
    let manager = CordisManager::new(
        Arc::clone(&registry),
        dir.path().to_path_buf(),
        dir.path().to_path_buf(),
        ToolFilter::default(),
    );
    let root = dir.path().to_path_buf();
    let watch = manager.spawn_watch(WatchOptions {
        interval: Duration::from_millis(100),
        config_files: vec![config.clone()],
        refresh: Some(Arc::new(move || Ok(vec![spec(&root, "demo")]))),
    });

    tokio::time::sleep(Duration::from_millis(250)).await;
    std::fs::write(&config, "plugins: { enabled: [demo] }\n").expect("config");

    let tool = format!("{TOOL_PREFIX}demo__echo");
    let started = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            if registry.contains(&tool) {
                return true;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .unwrap_or(false);
    watch.abort();
    assert!(started, "config refresh did not start the plugin");
    assert_eq!(call(&registry, &tool).await, "v1");
}

#[tokio::test]
async fn a_broken_config_keeps_running_plugins() {
    if !node_available() {
        eprintln!("node is not installed; skipping Cordis manager test");
        return;
    }
    let dir = tempfile::tempdir().expect("tempdir");
    write_plugin(dir.path(), "v1");
    let config = dir.path().join("profile.yaml");
    std::fs::write(&config, "plugins: {}\n").expect("config");
    let registry = Arc::new(ToolRegistry::new());
    let manager = CordisManager::new(
        Arc::clone(&registry),
        dir.path().to_path_buf(),
        dir.path().to_path_buf(),
        ToolFilter::default(),
    );
    manager.sync(vec![spec(dir.path(), "demo")]).await;
    let tool = format!("{TOOL_PREFIX}demo__echo");
    assert!(registry.contains(&tool));

    let watch = manager.spawn_watch(WatchOptions {
        interval: Duration::from_millis(100),
        config_files: vec![config.clone()],
        refresh: Some(Arc::new(|| Err("config is mid-save".into()))),
    });
    tokio::time::sleep(Duration::from_millis(250)).await;
    std::fs::write(&config, "plugins: [\n").expect("config");
    tokio::time::sleep(Duration::from_millis(500)).await;
    watch.abort();

    // A read error must not be interpreted as "no plugins configured".
    assert!(manager.is_running("demo"));
    assert!(registry.contains(&tool));
    assert_eq!(manager.statuses()[0].error, None);
}

#![forbid(unsafe_code)]
//! Sandboxed WASM plugin host.
//!
//! ABI `dshr.wasm.v1`: the guest exports `memory`, `list_tools() -> i64` and
//! `call_tool(i32, i32) -> i64` (pointer/length packed as `ptr << 32 | len`).
//! Optional `alloc(i32) -> i32` receives the JSON request. No host imports.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use dshr_llm::ToolDef;
use dshr_tools::{Tool, ToolCtx, ToolError, ToolOutcome};
use serde::Deserialize;
use serde_json::Value;
use thiserror::Error;
use wasmi::{Engine, Linker, Module, Store};

const FUEL: u64 = 10_000_000;

#[derive(Debug, Error)]
pub enum WasmError {
    #[error("wasm plugin I/O: {0}")]
    Io(String),

    #[error("wasm plugin parse: {0}")]
    Parse(String),

    #[error("wasm plugin trap: {0}")]
    Runtime(String),

    #[error("wasm plugin manifest: {0}")]
    Manifest(String),
}

#[derive(Debug, Deserialize)]
pub struct WasmPluginManifest {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub wasm: PathBuf,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

pub struct WasmPlugin {
    name: String,
    #[allow(dead_code)]
    description: String,
    engine: Engine,
    module: Module,
}

impl WasmPlugin {
    pub fn load(manifest_path: &Path) -> Result<Self, WasmError> {
        let text = std::fs::read_to_string(manifest_path)
            .map_err(|error| WasmError::Io(error.to_string()))?;
        let manifest: WasmPluginManifest = serde_saphyr::from_str(&text)
            .map_err(|error| WasmError::Manifest(error.to_string()))?;
        if !manifest.enabled {
            return Err(WasmError::Manifest("plugin is disabled".into()));
        }
        let wasm_path = manifest_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(&manifest.wasm);
        let bytes = std::fs::read(&wasm_path).map_err(|error| WasmError::Io(error.to_string()))?;
        Self::from_bytes(&manifest.name, &manifest.description, &bytes)
    }

    pub fn from_bytes(name: &str, description: &str, bytes: &[u8]) -> Result<Self, WasmError> {
        let mut config = wasmi::Config::default();
        config.consume_fuel(true);
        let engine = Engine::new(&config);
        let module =
            Module::new(&engine, bytes).map_err(|error| WasmError::Parse(error.to_string()))?;
        Ok(Self {
            name: name.to_owned(),
            description: description.to_owned(),
            engine,
            module,
        })
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn tools(&self) -> Result<Vec<ToolDef>, WasmError> {
        let response = self.call_entry("list_tools", &[])?;
        let parsed: ToolsResponse = serde_json::from_slice(&response)
            .map_err(|error| WasmError::Parse(error.to_string()))?;
        Ok(parsed
            .tools
            .into_iter()
            .map(|tool| ToolDef {
                name: tool.name,
                description: tool.description,
                input_schema: tool.input_schema,
            })
            .collect())
    }

    pub fn call_tool(&self, name: &str, arguments: &Value) -> Result<ToolOutcome, WasmError> {
        let request = serde_json::to_vec(&serde_json::json!({
            "tool": name,
            "arguments": arguments
        }))
        .map_err(|error| WasmError::Parse(error.to_string()))?;
        let response = self.call_entry("call_tool", &request)?;
        let parsed: CallResponse = serde_json::from_slice(&response)
            .map_err(|error| WasmError::Parse(error.to_string()))?;
        Ok(ToolOutcome {
            content: parsed.content,
            is_error: parsed.is_error,
            files: Vec::new(),
        })
    }

    fn call_entry(&self, function: &str, request: &[u8]) -> Result<Vec<u8>, WasmError> {
        let mut store = Store::new(&self.engine, ());
        store
            .set_fuel(FUEL)
            .map_err(|error| WasmError::Runtime(error.to_string()))?;
        let linker = Linker::new(&self.engine);
        let instance = linker
            .instantiate_and_start(&mut store, &self.module)
            .map_err(|error| WasmError::Runtime(error.to_string()))?;

        let (pointer, length) = if function == "call_tool" {
            let alloc = instance.get_typed_func::<i32, i32>(&store, "alloc").ok();
            if let Some(alloc) = alloc {
                let ptr = alloc
                    .call(&mut store, request.len() as i32)
                    .map_err(|error| WasmError::Runtime(error.to_string()))?;
                let memory = instance
                    .get_memory(&store, "memory")
                    .ok_or_else(|| WasmError::Runtime("missing memory".into()))?;
                let start = ptr as usize;
                let end = start
                    .checked_add(request.len())
                    .ok_or_else(|| WasmError::Runtime("allocation overflow".into()))?;
                memory
                    .data_mut(&mut store)
                    .get_mut(start..end)
                    .ok_or_else(|| WasmError::Runtime("allocation out of bounds".into()))?
                    .copy_from_slice(request);
                (ptr, request.len() as i32)
            } else {
                (0, 0)
            }
        } else {
            (0, 0)
        };

        let packed = if function == "call_tool" {
            let func = instance
                .get_typed_func::<(i32, i32), i64>(&store, function)
                .map_err(|error| WasmError::Runtime(error.to_string()))?;
            func.call(&mut store, (pointer, length))
                .map_err(|error| WasmError::Runtime(error.to_string()))?
        } else {
            let func = instance
                .get_typed_func::<(), i64>(&store, function)
                .map_err(|error| WasmError::Runtime(error.to_string()))?;
            func.call(&mut store, ())
                .map_err(|error| WasmError::Runtime(error.to_string()))?
        };
        let ptr = (packed >> 32) as usize;
        let len = (packed & 0xffff_ffff) as usize;
        let memory = instance
            .get_memory(&store, "memory")
            .ok_or_else(|| WasmError::Runtime("missing memory".into()))?;
        let data = memory.data(&store);
        let end = ptr
            .checked_add(len)
            .ok_or_else(|| WasmError::Runtime("result overflow".into()))?;
        data.get(ptr..end)
            .map(ToOwned::to_owned)
            .ok_or_else(|| WasmError::Runtime("result out of bounds".into()))
    }
}

#[derive(Debug, Deserialize)]
struct ToolsResponse {
    tools: Vec<WasmToolDef>,
}

#[derive(Debug, Deserialize)]
struct WasmToolDef {
    name: String,
    #[serde(default)]
    description: String,
    #[serde(rename = "inputSchema", alias = "input_schema")]
    input_schema: Value,
}

#[derive(Debug, Deserialize)]
struct CallResponse {
    #[serde(default)]
    content: String,
    #[serde(default, rename = "isError")]
    is_error: bool,
}

pub struct WasmTool {
    plugin: Arc<WasmPlugin>,
    definition: ToolDef,
    namespaced: String,
    remote_name: String,
}

impl WasmTool {
    #[must_use]
    pub fn new(plugin: Arc<WasmPlugin>, mut definition: ToolDef) -> Self {
        let remote_name = definition.name.clone();
        let namespaced = format!("wasm__{}__{}", plugin.name(), remote_name);
        definition.name = namespaced.clone();
        Self {
            plugin,
            definition,
            namespaced,
            remote_name,
        }
    }
}

#[async_trait]
impl Tool for WasmTool {
    fn name(&self) -> &str {
        &self.namespaced
    }

    fn description(&self) -> &str {
        &self.definition.description
    }

    fn schema(&self) -> Value {
        self.definition.input_schema.clone()
    }

    async fn execute(&self, _ctx: &ToolCtx, args: Value) -> Result<ToolOutcome, ToolError> {
        self.plugin
            .call_tool(&self.remote_name, &args)
            .map_err(|error| ToolError::Failed(error.to_string()))
    }
}

pub fn discover(dirs: &[PathBuf]) -> Vec<Arc<WasmPlugin>> {
    let mut plugins = Vec::new();
    for dir in dirs {
        let Ok(entries) = std::fs::read_dir(dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|value| value.to_str()) != Some("yaml") {
                continue;
            }
            match WasmPlugin::load(&path) {
                Ok(plugin) => plugins.push(Arc::new(plugin)),
                Err(error) => tracing::warn!(path = %path.display(), %error, "wasm plugin skipped"),
            }
        }
    }
    plugins
}

pub fn register_tools(registry: &mut dshr_tools::ToolRegistry, dirs: &[PathBuf]) {
    for plugin in discover(dirs) {
        match plugin.tools() {
            Ok(tools) => {
                for definition in tools {
                    registry.register_arc(Arc::new(WasmTool::new(Arc::clone(&plugin), definition)));
                }
            }
            Err(error) => tracing::warn!(plugin = plugin.name(), %error, "wasm tools skipped"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (String, Vec<u8>) {
        let tools = r#"{"tools":[{"name":"echo","description":"Echo","inputSchema":{"type":"object","properties":{}}}]}"#;
        let call = r#"{"content":"wasm echo","isError":false}"#;
        let wat = format!(
            r#"(module
              (memory (export "memory") 1)
              (data (i32.const 1024) "{}")
              (data (i32.const 2048) "{}")
              (func (export "list_tools") (result i64)
                i64.const 1024
                i64.const 32
                i64.shl
                i64.const {}
                i64.or)
              (func (export "call_tool") (param i32 i32) (result i64)
                i64.const 2048
                i64.const 32
                i64.shl
                i64.const {}
                i64.or))
            "#,
            tools.replace('"', "\\\""),
            call.replace('"', "\\\""),
            tools.len(),
            call.len()
        );
        let bytes = wat::parse_str(&wat).expect("wat");
        (wat, bytes)
    }

    #[test]
    fn lists_and_calls_tools() {
        let (_text, bytes) = fixture();
        let plugin = Arc::new(WasmPlugin::from_bytes("demo", "demo", &bytes).expect("plugin"));
        let tools = plugin.tools().expect("tools");
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "echo");
        let outcome = plugin
            .call_tool("echo", &serde_json::json!({}))
            .expect("call");
        assert_eq!(outcome.content, "wasm echo");
    }

    #[test]
    fn loads_manifest_relative_to_yaml() {
        let dir = tempfile::tempdir().expect("tempdir");
        let (text, bytes) = fixture();
        let _ = text;
        std::fs::write(dir.path().join("demo.wasm"), bytes).expect("wasm");
        std::fs::write(
            dir.path().join("demo.yaml"),
            "name: demo\ndescription: demo\nwasm: demo.wasm\n",
        )
        .expect("yaml");
        let plugin = WasmPlugin::load(&dir.path().join("demo.yaml")).expect("load");
        assert_eq!(plugin.name(), "demo");
    }
}

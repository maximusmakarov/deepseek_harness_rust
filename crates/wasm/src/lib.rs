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
use ring::rand::{SecureRandom, SystemRandom};
use ring::signature::{ED25519, Ed25519KeyPair, KeyPair, UnparsedPublicKey};
use serde::Deserialize;
use serde_json::Value;
use thiserror::Error;
use wasmi::{Engine, Linker, Module, Store, StoreLimits, StoreLimitsBuilder};

const FUEL: u64 = 10_000_000;
const MEMORY_LIMIT: usize = 64 * 1024 * 1024;

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
    /// Optional hex SHA-256 of the wasm file.
    #[serde(default)]
    pub sha256: Option<String>,
    /// Optional Ed25519 signature (hex) over `dshr.wasm.v1\n<name>\n<sha256>`.
    #[serde(default)]
    pub signature: Option<String>,
    /// Optional signer public key (hex); informational, trust comes from config.
    #[serde(default)]
    pub signer: Option<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

/// Trust policy for plugin signatures.
#[derive(Debug, Clone, Default)]
pub struct SignaturePolicy {
    /// Ed25519 public keys allowed to sign plugins. A non-empty list also
    /// requires every plugin to be signed.
    pub trusted_keys: Vec<[u8; 32]>,
}

impl SignaturePolicy {
    /// Builds a policy from hexadecimal public keys.
    pub fn new(keys_hex: &[String]) -> Result<Self, WasmError> {
        let mut trusted_keys = Vec::with_capacity(keys_hex.len());
        for key in keys_hex {
            trusted_keys.push(parse_key_hex(key)?);
        }
        Ok(Self { trusted_keys })
    }

    #[must_use]
    pub fn requires_signature(&self) -> bool {
        !self.trusted_keys.is_empty()
    }
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
        Self::load_with_policy(manifest_path, &SignaturePolicy::default())
    }

    pub fn load_with_policy(
        manifest_path: &Path,
        policy: &SignaturePolicy,
    ) -> Result<Self, WasmError> {
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
        if let Some(expected) = &manifest.sha256 {
            let actual = sha256_hex(&bytes);
            if !actual.eq_ignore_ascii_case(expected) {
                return Err(WasmError::Manifest(format!(
                    "wasm sha256 mismatch: expected {expected}, got {actual}"
                )));
            }
        }
        verify_signature(
            &manifest.name,
            &bytes,
            manifest.signature.as_deref(),
            policy,
        )?;
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
        let limits = StoreLimitsBuilder::new()
            .memory_size(MEMORY_LIMIT)
            .instances(1)
            .memories(1)
            .tables(1)
            .build();
        let mut store = Store::new(&self.engine, limits);
        store.limiter(|limits: &mut StoreLimits| limits);
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

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Signs the wasm referenced by a manifest and stores `signature`/`signer`.
///
/// Returns the signer public key in hex.
pub fn sign_manifest(manifest_path: &Path, secret_key_hex: &str) -> Result<String, WasmError> {
    let secret = parse_secret_hex(secret_key_hex)?;
    let key = Ed25519KeyPair::from_seed_unchecked(&secret)
        .map_err(|error| WasmError::Manifest(format!("invalid Ed25519 secret key: {error}")))?;
    let text =
        std::fs::read_to_string(manifest_path).map_err(|error| WasmError::Io(error.to_string()))?;
    let manifest: WasmPluginManifest =
        serde_saphyr::from_str(&text).map_err(|error| WasmError::Manifest(error.to_string()))?;
    let wasm_path = manifest_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(&manifest.wasm);
    let bytes = std::fs::read(&wasm_path).map_err(|error| WasmError::Io(error.to_string()))?;
    let message = signed_message(&manifest.name, &bytes);
    let signature = key.sign(&message);
    let public = hex_encode(key.public_key().as_ref());

    let mut document: Value =
        serde_saphyr::from_str(&text).map_err(|error| WasmError::Manifest(error.to_string()))?;
    let Some(map) = document.as_object_mut() else {
        return Err(WasmError::Manifest(
            "manifest root must be a mapping".into(),
        ));
    };
    map.insert(
        "signature".into(),
        Value::String(hex_encode(signature.as_ref())),
    );
    map.insert("signer".into(), Value::String(public.clone()));
    let encoded = serde_saphyr::to_string(&document)
        .map_err(|error| WasmError::Manifest(error.to_string()))?;
    std::fs::write(manifest_path, encoded).map_err(|error| WasmError::Io(error.to_string()))?;
    Ok(public)
}

/// Verifies a manifest signature against an explicit public key.
pub fn verify_manifest(manifest_path: &Path, public_key_hex: &str) -> Result<(), WasmError> {
    let key = parse_key_hex(public_key_hex)?;
    let text =
        std::fs::read_to_string(manifest_path).map_err(|error| WasmError::Io(error.to_string()))?;
    let manifest: WasmPluginManifest =
        serde_saphyr::from_str(&text).map_err(|error| WasmError::Manifest(error.to_string()))?;
    let wasm_path = manifest_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(&manifest.wasm);
    let bytes = std::fs::read(&wasm_path).map_err(|error| WasmError::Io(error.to_string()))?;
    let policy = SignaturePolicy {
        trusted_keys: vec![key],
    };
    verify_signature(
        &manifest.name,
        &bytes,
        manifest.signature.as_deref(),
        &policy,
    )
}

/// Generates a new Ed25519 key pair as `(public_hex, secret_hex)`.
pub fn generate_keypair() -> Result<(String, String), WasmError> {
    let mut seed = [0u8; 32];
    SystemRandom::new()
        .fill(&mut seed)
        .map_err(|error| WasmError::Io(error.to_string()))?;
    let key = Ed25519KeyPair::from_seed_unchecked(&seed)
        .map_err(|error| WasmError::Manifest(error.to_string()))?;
    Ok((hex_encode(key.public_key().as_ref()), hex_encode(&seed)))
}

fn signed_message(name: &str, wasm: &[u8]) -> Vec<u8> {
    format!("dshr.wasm.v1\n{name}\n{}", sha256_hex(wasm)).into_bytes()
}

fn verify_signature(
    name: &str,
    wasm: &[u8],
    signature: Option<&str>,
    policy: &SignaturePolicy,
) -> Result<(), WasmError> {
    let Some(signature) = signature.filter(|value| !value.trim().is_empty()) else {
        if policy.requires_signature() {
            return Err(WasmError::Manifest(format!(
                "plugin {name}: signature is required"
            )));
        }
        return Ok(());
    };
    if policy.trusted_keys.is_empty() {
        return Err(WasmError::Manifest(format!(
            "plugin {name}: signature present but no trusted keys are configured"
        )));
    }
    let signature = parse_signature_hex(signature)?;
    let message = signed_message(name, wasm);
    if policy.trusted_keys.iter().any(|key| {
        UnparsedPublicKey::new(&ED25519, key)
            .verify(&message, &signature)
            .is_ok()
    }) {
        return Ok(());
    }
    Err(WasmError::Manifest(format!(
        "plugin {name}: signature does not verify against a trusted key"
    )))
}

fn parse_key_hex(value: &str) -> Result<[u8; 32], WasmError> {
    hex_decode(value)
        .ok_or_else(|| WasmError::Manifest("trusted key must be a 32-byte hex string".into()))
}

fn parse_secret_hex(value: &str) -> Result<[u8; 32], WasmError> {
    hex_decode(value)
        .ok_or_else(|| WasmError::Manifest("secret key must be a 32-byte hex string".into()))
}

fn parse_signature_hex(value: &str) -> Result<[u8; 64], WasmError> {
    hex_decode(value)
        .ok_or_else(|| WasmError::Manifest("signature must be a 64-byte hex string".into()))
}

fn hex_decode<const N: usize>(value: &str) -> Option<[u8; N]> {
    let value = value.trim();
    if value.len() != N * 2 {
        return None;
    }
    let mut bytes = [0u8; N];
    for (index, chunk) in value.as_bytes().as_chunks::<2>().0.iter().enumerate() {
        let text = std::str::from_utf8(chunk).ok()?;
        bytes[index] = u8::from_str_radix(text, 16).ok()?;
    }
    Some(bytes)
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub fn discover(dirs: &[PathBuf], policy: &SignaturePolicy) -> Vec<Arc<WasmPlugin>> {
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
            match WasmPlugin::load_with_policy(&path, policy) {
                Ok(plugin) => plugins.push(Arc::new(plugin)),
                Err(error) => tracing::warn!(path = %path.display(), %error, "wasm plugin skipped"),
            }
        }
    }
    plugins
}

pub fn register_tools(
    registry: &mut dshr_tools::ToolRegistry,
    dirs: &[PathBuf],
    policy: &SignaturePolicy,
) {
    for plugin in discover(dirs, policy) {
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

    fn write_plugin(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
        std::fs::write(dir.join(format!("{name}.wasm")), bytes).expect("wasm");
        let manifest = dir.join(format!("{name}.yaml"));
        std::fs::write(&manifest, format!("name: {name}\nwasm: {name}.wasm\n")).expect("yaml");
        manifest
    }

    #[test]
    fn verifies_signature_from_trusted_key() {
        let dir = tempfile::tempdir().expect("tempdir");
        let (_text, bytes) = fixture();
        let manifest = write_plugin(dir.path(), "demo", &bytes);
        let (public, secret) = generate_keypair().expect("keypair");
        sign_manifest(&manifest, &secret).expect("sign");

        let policy = SignaturePolicy::new(&[public]).expect("policy");
        let plugin = WasmPlugin::load_with_policy(&manifest, &policy).expect("load");
        assert_eq!(plugin.name(), "demo");
    }

    #[test]
    fn rejects_unsigned_plugin_when_signature_is_required() {
        let dir = tempfile::tempdir().expect("tempdir");
        let (_text, bytes) = fixture();
        let manifest = write_plugin(dir.path(), "demo", &bytes);
        let (public, _secret) = generate_keypair().expect("keypair");
        let policy = SignaturePolicy::new(&[public]).expect("policy");
        let error = match WasmPlugin::load_with_policy(&manifest, &policy) {
            Ok(_) => panic!("unsigned plugin accepted"),
            Err(error) => error,
        };
        assert!(
            error.to_string().contains("signature is required"),
            "{error}"
        );
    }

    #[test]
    fn rejects_signature_from_untrusted_key() {
        let dir = tempfile::tempdir().expect("tempdir");
        let (_text, bytes) = fixture();
        let manifest = write_plugin(dir.path(), "demo", &bytes);
        let (_public, secret) = generate_keypair().expect("keypair");
        sign_manifest(&manifest, &secret).expect("sign");
        let (other_public, _other_secret) = generate_keypair().expect("keypair");

        let policy = SignaturePolicy::new(&[other_public]).expect("policy");
        let error = match WasmPlugin::load_with_policy(&manifest, &policy) {
            Ok(_) => panic!("untrusted signature accepted"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("does not verify"), "{error}");
    }

    #[test]
    fn signature_is_bound_to_the_plugin_name() {
        let dir = tempfile::tempdir().expect("tempdir");
        let (_text, bytes) = fixture();
        let manifest = write_plugin(dir.path(), "demo", &bytes);
        let (public, secret) = generate_keypair().expect("keypair");
        sign_manifest(&manifest, &secret).expect("sign");
        // Reuse the signature under another plugin name.
        let text = std::fs::read_to_string(&manifest).expect("read");
        let mut document: Value = serde_saphyr::from_str(&text).expect("yaml");
        document["name"] = Value::String("other".into());
        std::fs::write(
            &manifest,
            serde_saphyr::to_string(&document).expect("encode"),
        )
        .expect("yaml");

        let policy = SignaturePolicy::new(&[public]).expect("policy");
        let error = match WasmPlugin::load_with_policy(&manifest, &policy) {
            Ok(_) => panic!("renamed signature accepted"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("does not verify"), "{error}");
    }

    #[test]
    fn sign_and_verify_roundtrip() {
        let dir = tempfile::tempdir().expect("tempdir");
        let (_text, bytes) = fixture();
        let manifest = write_plugin(dir.path(), "demo", &bytes);
        let (public, secret) = generate_keypair().expect("keypair");
        let signer = sign_manifest(&manifest, &secret).expect("sign");
        assert_eq!(signer, public);
        verify_manifest(&manifest, &public).expect("verify");
        let (other_public, _other_secret) = generate_keypair().expect("keypair");
        assert!(verify_manifest(&manifest, &other_public).is_err());
    }

    #[test]
    fn rejects_signature_without_trusted_keys() {
        let dir = tempfile::tempdir().expect("tempdir");
        let (_text, bytes) = fixture();
        let manifest = write_plugin(dir.path(), "demo", &bytes);
        let (_public, secret) = generate_keypair().expect("keypair");
        sign_manifest(&manifest, &secret).expect("sign");
        let error = match WasmPlugin::load_with_policy(&manifest, &SignaturePolicy::default()) {
            Ok(_) => panic!("signature without trust accepted"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("no trusted keys"), "{error}");
    }

    #[test]
    fn verifies_manifest_sha256() {
        let dir = tempfile::tempdir().expect("tempdir");
        let (_text, bytes) = fixture();
        std::fs::write(dir.path().join("demo.wasm"), &bytes).expect("wasm");
        let hash = sha256_hex(&bytes);
        std::fs::write(
            dir.path().join("demo.yaml"),
            format!("name: demo\nwasm: demo.wasm\nsha256: {hash}\n"),
        )
        .expect("yaml");
        WasmPlugin::load(&dir.path().join("demo.yaml")).expect("valid hash");
        std::fs::write(
            dir.path().join("demo.yaml"),
            "name: demo\nwasm: demo.wasm\nsha256: deadbeef\n",
        )
        .expect("yaml");
        let error = match WasmPlugin::load(&dir.path().join("demo.yaml")) {
            Ok(_) => panic!("bad hash accepted"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("sha256"));
    }
}

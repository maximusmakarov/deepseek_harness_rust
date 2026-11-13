#![forbid(unsafe_code)]
//! Process sandbox: policy plus platform backends.
//!
//! `confine` returns an argv that, when spawned, applies the policy. If a
//! confined mode has no available backend the call fails — fail-closed.

use std::path::PathBuf;
use std::sync::Arc;

use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SandboxMode {
    ReadOnly,
    WorkspaceWrite,
    DangerFull,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkMode {
    Deny,
    Allow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Enforcement {
    Full,
    Partial,
}

#[derive(Debug, Clone)]
pub struct SandboxPolicy {
    pub mode: SandboxMode,
    pub workspace: PathBuf,
    pub network: NetworkMode,
}

impl SandboxPolicy {
    #[must_use]
    pub fn workspace_write(workspace: impl Into<PathBuf>) -> Self {
        Self {
            mode: SandboxMode::WorkspaceWrite,
            workspace: workspace.into(),
            network: NetworkMode::Deny,
        }
    }

    #[must_use]
    pub fn read_only(workspace: impl Into<PathBuf>) -> Self {
        Self {
            mode: SandboxMode::ReadOnly,
            workspace: workspace.into(),
            network: NetworkMode::Deny,
        }
    }
}

#[derive(Debug, Error)]
pub enum SandboxError {
    #[error("sandbox backend unavailable: {0}")]
    Unavailable(String),

    #[error("sandbox profile error: {0}")]
    Profile(String),

    #[error("sandbox I/O: {0}")]
    Io(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfinedCommand {
    pub program: String,
    pub args: Vec<String>,
    pub enforcement: Enforcement,
    pub backend: String,
}

pub trait SandboxBackend: Send + Sync {
    fn name(&self) -> &str;

    fn available(&self) -> bool;

    fn confine(
        &self,
        policy: &SandboxPolicy,
        program: &str,
        args: &[String],
    ) -> Result<ConfinedCommand, SandboxError>;
}

pub struct Sandbox {
    backend: Arc<dyn SandboxBackend>,
    policy: SandboxPolicy,
}

impl Sandbox {
    #[must_use]
    pub fn platform(policy: SandboxPolicy) -> Self {
        Self {
            backend: default_backend(),
            policy,
        }
    }

    #[must_use]
    pub fn with_backend(policy: SandboxPolicy, backend: Arc<dyn SandboxBackend>) -> Self {
        Self { backend, policy }
    }

    #[must_use]
    pub fn policy(&self) -> &SandboxPolicy {
        &self.policy
    }

    #[must_use]
    pub fn backend_name(&self) -> &str {
        self.backend.name()
    }

    pub fn confine(&self, program: &str, args: &[String]) -> Result<ConfinedCommand, SandboxError> {
        if self.policy.mode == SandboxMode::DangerFull {
            return Ok(ConfinedCommand {
                program: program.to_owned(),
                args: args.to_vec(),
                enforcement: Enforcement::Full,
                backend: "none".to_owned(),
            });
        }
        if !self.backend.available() {
            return Err(SandboxError::Unavailable(format!(
                "{} is not available on this host",
                self.backend.name()
            )));
        }
        let command = self.backend.confine(&self.policy, program, args)?;
        tracing::debug!(
            backend = %command.backend,
            enforcement = ?command.enforcement,
            program = %program,
            "sandboxed command"
        );
        Ok(command)
    }
}

fn default_backend() -> Arc<dyn SandboxBackend> {
    #[cfg(target_os = "macos")]
    {
        Arc::new(SeatbeltBackend)
    }
    #[cfg(target_os = "linux")]
    {
        Arc::new(LinuxBackend {
            wrapper: std::env::current_exe().unwrap_or_default(),
        })
    }
    #[cfg(target_os = "windows")]
    {
        Arc::new(WindowsBackend {
            wrapper: std::env::current_exe().unwrap_or_default(),
        })
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        Arc::new(UnsupportedBackend)
    }
}

pub struct UnsupportedBackend;

impl SandboxBackend for UnsupportedBackend {
    fn name(&self) -> &str {
        "unsupported"
    }

    fn available(&self) -> bool {
        false
    }

    fn confine(
        &self,
        _policy: &SandboxPolicy,
        _program: &str,
        _args: &[String],
    ) -> Result<ConfinedCommand, SandboxError> {
        Err(SandboxError::Unavailable(
            "no sandbox backend for this platform".into(),
        ))
    }
}

#[cfg(target_os = "windows")]
pub struct WindowsBackend {
    wrapper: PathBuf,
}

#[cfg(target_os = "windows")]
impl SandboxBackend for WindowsBackend {
    fn name(&self) -> &str {
        "appcontainer"
    }

    fn available(&self) -> bool {
        std::env::var_os("DSHR_WINDOWS_SANDBOX").is_some() && self.wrapper.is_file()
    }

    fn confine(
        &self,
        policy: &SandboxPolicy,
        program: &str,
        args: &[String],
    ) -> Result<ConfinedCommand, SandboxError> {
        let mode = match policy.mode {
            SandboxMode::ReadOnly => "read-only",
            SandboxMode::WorkspaceWrite => "workspace-write",
            SandboxMode::DangerFull => "danger-full-access",
        };
        let network = match policy.network {
            NetworkMode::Deny => "deny",
            NetworkMode::Allow => "allow",
        };
        let mut confined = vec![
            "__sandbox-exec".to_owned(),
            "--mode".to_owned(),
            mode.to_owned(),
            "--workspace".to_owned(),
            policy.workspace.display().to_string(),
            "--network".to_owned(),
            network.to_owned(),
            "--".to_owned(),
            program.to_owned(),
        ];
        confined.extend(args.iter().cloned());
        Ok(ConfinedCommand {
            program: self.wrapper.display().to_string(),
            args: confined,
            enforcement: Enforcement::Partial,
            backend: "appcontainer".to_owned(),
        })
    }
}

#[cfg(target_os = "linux")]
pub struct LinuxBackend {
    wrapper: PathBuf,
}

#[cfg(target_os = "linux")]
impl SandboxBackend for LinuxBackend {
    fn name(&self) -> &str {
        "landlock"
    }

    fn available(&self) -> bool {
        self.wrapper.is_file()
    }

    fn confine(
        &self,
        policy: &SandboxPolicy,
        program: &str,
        args: &[String],
    ) -> Result<ConfinedCommand, SandboxError> {
        let mode = match policy.mode {
            SandboxMode::ReadOnly => "read-only",
            SandboxMode::WorkspaceWrite => "workspace-write",
            SandboxMode::DangerFull => "danger-full-access",
        };
        let network = match policy.network {
            NetworkMode::Deny => "deny",
            NetworkMode::Allow => "allow",
        };
        let mut confined = vec![
            "__sandbox-exec".to_owned(),
            "--mode".to_owned(),
            mode.to_owned(),
            "--workspace".to_owned(),
            policy.workspace.display().to_string(),
            "--network".to_owned(),
            network.to_owned(),
            "--".to_owned(),
            program.to_owned(),
        ];
        confined.extend(args.iter().cloned());
        Ok(ConfinedCommand {
            program: self.wrapper.display().to_string(),
            args: confined,
            enforcement: Enforcement::Full,
            backend: "landlock".to_owned(),
        })
    }
}

#[cfg(target_os = "macos")]
pub struct SeatbeltBackend;

#[cfg(target_os = "macos")]
impl SandboxBackend for SeatbeltBackend {
    fn name(&self) -> &str {
        "seatbelt"
    }

    fn available(&self) -> bool {
        std::path::Path::new("/usr/bin/sandbox-exec").is_file()
    }

    fn confine(
        &self,
        policy: &SandboxPolicy,
        program: &str,
        args: &[String],
    ) -> Result<ConfinedCommand, SandboxError> {
        let profile = seatbelt_profile(policy)?;
        let mut confined = vec![
            "-p".to_owned(),
            profile,
            "--".to_owned(),
            program.to_owned(),
        ];
        confined.extend(args.iter().cloned());
        Ok(ConfinedCommand {
            program: "/usr/bin/sandbox-exec".to_owned(),
            args: confined,
            enforcement: Enforcement::Full,
            backend: "seatbelt".to_owned(),
        })
    }
}

#[cfg(target_os = "macos")]
fn seatbelt_profile(policy: &SandboxPolicy) -> Result<String, SandboxError> {
    let workspace = policy
        .workspace
        .canonicalize()
        .unwrap_or_else(|_| policy.workspace.clone());
    let workspace = escape(&workspace.display().to_string());
    let temp = escape(&std::env::temp_dir().display().to_string());

    let mut profile = String::from(
        "(version 1)\n\
(deny default)\n\
(allow process*)\n\
(allow file-read*)\n\
(allow sysctl-read)\n\
(allow mach-lookup)\n\
(allow signal (target self))\n",
    );

    if policy.network == NetworkMode::Allow {
        profile.push_str("(allow network*)\n");
    }

    match policy.mode {
        SandboxMode::ReadOnly => {
            profile.push_str(
                "(allow file-write-data (literal \"/dev/null\") (literal \"/dev/stdout\") (literal \"/dev/stderr\"))\n",
            );
        }
        SandboxMode::WorkspaceWrite => {
            profile.push_str(&format!(
                "(allow file-write* (subpath \"{workspace}\") (subpath \"{temp}\") (literal \"/dev/null\") (literal \"/dev/stdout\") (literal \"/dev/stderr\"))\n"
            ));
        }
        SandboxMode::DangerFull => {}
    }

    Ok(profile)
}

#[cfg(target_os = "macos")]
fn escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn danger_full_bypasses_backend() {
        let dir = tempfile::tempdir().expect("tempdir");
        let sandbox = Sandbox::with_backend(
            SandboxPolicy {
                mode: SandboxMode::DangerFull,
                workspace: dir.path().to_path_buf(),
                network: NetworkMode::Allow,
            },
            Arc::new(UnsupportedBackend),
        );
        let command = sandbox
            .confine("sh", &["-c".into(), "true".into()])
            .expect("bypass");
        assert_eq!(command.program, "sh");
        assert_eq!(command.backend, "none");
    }

    #[test]
    fn confined_mode_fails_closed_without_backend() {
        let dir = tempfile::tempdir().expect("tempdir");
        let sandbox = Sandbox::with_backend(
            SandboxPolicy::workspace_write(dir.path()),
            Arc::new(UnsupportedBackend),
        );
        let error = sandbox
            .confine("sh", &["-c".into(), "true".into()])
            .expect_err("unavailable");
        assert!(matches!(error, SandboxError::Unavailable(_)));
    }

    #[cfg(target_os = "windows")]
    #[test]
    #[ignore = "AppContainer is opt-in via DSHR_WINDOWS_SANDBOX"]
    fn appcontainer_wraps_command() {
        let dir = tempfile::tempdir().expect("tempdir");
        let sandbox = Sandbox::platform(SandboxPolicy::workspace_write(dir.path()));
        let command = sandbox
            .confine("cmd", &["/C".into(), "echo ok".into()])
            .expect("confine");
        assert_eq!(command.backend, "appcontainer");
        assert_eq!(command.args[0], "__sandbox-exec");
        assert_eq!(command.enforcement, Enforcement::Partial);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn landlock_wraps_command() {
        let dir = tempfile::tempdir().expect("tempdir");
        let sandbox = Sandbox::platform(SandboxPolicy::workspace_write(dir.path()));
        let command = sandbox
            .confine("sh", &["-c".into(), "true".into()])
            .expect("confine");
        assert_eq!(command.backend, "landlock");
        assert_eq!(command.args[0], "__sandbox-exec");
        assert_eq!(command.enforcement, Enforcement::Full);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn seatbelt_profile_mentions_workspace() {
        let dir = tempfile::tempdir().expect("tempdir");
        let profile =
            seatbelt_profile(&SandboxPolicy::workspace_write(dir.path())).expect("profile");
        assert!(profile.contains("(deny default)"), "{profile}");
        assert!(
            profile.contains(dir.path().to_str().expect("utf8")),
            "{profile}"
        );
    }

    #[cfg(target_os = "macos")]
    #[tokio::test]
    async fn seatbelt_confines_writes() {
        let dir = tempfile::tempdir().expect("tempdir");
        let workspace = dir.path().join("ws");
        std::fs::create_dir_all(&workspace).expect("ws");
        let sandbox = Sandbox::platform(SandboxPolicy::workspace_write(&workspace));

        let inside = workspace.join("inside.txt");
        run(&sandbox, &format!("printf ok > '{}'", inside.display()))
            .await
            .expect("inside allowed");

        let outside = dir.path().join("outside.txt");
        let status = run(&sandbox, &format!("printf no > '{}'", outside.display())).await;
        assert!(status.is_err(), "outside write must fail");
        assert!(!outside.exists());
    }

    #[cfg(target_os = "macos")]
    async fn run(sandbox: &Sandbox, command: &str) -> Result<(), String> {
        let confined = sandbox
            .confine("sh", &["-c".to_owned(), command.to_owned()])
            .map_err(|error| error.to_string())?;
        let status = tokio::process::Command::new(&confined.program)
            .args(&confined.args)
            .status()
            .await
            .map_err(|error| error.to_string())?;
        if status.success() {
            Ok(())
        } else {
            Err(format!("exit {status}"))
        }
    }
}

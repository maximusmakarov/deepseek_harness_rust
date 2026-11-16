//! Login-shell environment capture for GUI launches.
//!
//! macOS and Linux GUI launches inherit only the session manager environment, so
//! exports from `~/.zprofile`/`~/.zshrc` (or the shell's equivalents) never reach
//! the agent: `DEEPSEEK_API_KEY` is missing and `dshr serve` reports "no provider".
//! The first server start runs the user's login shell once (`-ilc`, `env -0`) and
//! forwards its environment to the child, minus shell bookkeeping variables and
//! names owned by the desktop shell.
//!
//! The delimiter output (`\x1e` … `\x1f`) keeps prompts, banners and update
//! checkers from startup files out of the payload.

use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

const DEFAULT_TIMEOUT_MS: u64 = 10_000;
const MIN_TIMEOUT_MS: u64 = 1_000;
const MAX_TIMEOUT_MS: u64 = 2_147_483_647;
const BEGIN: u8 = 0x1e;
const END: u8 = 0x1f;
const PROBE: &str = "printf '\\036'; env -0; printf '\\037'";

/// Variables that describe the probe rather than the user's session.
const PROBE_VARIABLES: [&str; 7] = [
    "PWD",
    "OLDPWD",
    "SHLVL",
    "_",
    "DISABLE_AUTO_UPDATE",
    "ZSH_TMUX_AUTOSTARTED",
    "ZSH_TMUX_AUTOSTART",
];

/// Names the desktop shell owns; the login shell must not override them.
const OWNED_VARIABLES: [&str; 1] = ["DSH_HOME"];
const OWNED_PREFIXES: [&str; 2] = ["DSHR_", "TAURI_"];

/// Environment captured from the login shell.
#[derive(Debug)]
pub struct LoginEnvironment {
    pub shell: String,
    pub variables: Vec<(String, String)>,
}

impl LoginEnvironment {
    pub fn len(&self) -> usize {
        self.variables.len()
    }
}

/// Captures once per process; later server starts reuse the result.
pub fn environment() -> Option<&'static LoginEnvironment> {
    static CACHE: OnceLock<Option<LoginEnvironment>> = OnceLock::new();
    CACHE.get_or_init(capture).as_ref()
}

/// Applies the login-shell environment to a child command, once per process.
pub fn apply(command: &mut Command) -> Option<&'static LoginEnvironment> {
    let environment = environment()?;
    command.envs(
        environment
            .variables
            .iter()
            .map(|(key, value)| (key, value)),
    );
    Some(environment)
}

/// Runs the login shell once and parses its environment.
#[cfg(unix)]
pub fn capture() -> Option<LoginEnvironment> {
    let shell = login_shell()?;
    let timeout = timeout_from(
        std::env::var("DSHR_DESKTOP_LOGIN_SHELL_TIMEOUT_MS")
            .ok()
            .as_deref(),
    );
    let mut child = Command::new(&shell)
        .arg("-ilc")
        .arg(PROBE)
        .env("DISABLE_AUTO_UPDATE", "true")
        .env("ZSH_TMUX_AUTOSTARTED", "true")
        .env("ZSH_TMUX_AUTOSTART", "false")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let (sender, receiver) = std::sync::mpsc::channel();
    let mut stdout = child.stdout.take()?;
    // A startup file may leave a background process holding stdout, so the
    // reader is bounded by the same deadline instead of joining forever.
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = stdout.read_to_end(&mut bytes);
        let _ = sender.send(bytes);
    });
    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(25));
            }
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
    let bytes = receiver
        .recv_timeout(deadline.saturating_duration_since(Instant::now()))
        .ok()?;
    let variables = variables_from_dump(&bytes);
    if variables.is_empty() {
        return None;
    }
    Some(LoginEnvironment {
        shell: shell.display().to_string(),
        variables,
    })
}

/// Windows GUI launches inherit the user and system environment from the registry.
#[cfg(not(unix))]
pub fn capture() -> Option<LoginEnvironment> {
    None
}

/// The user's login shell: `$SHELL` when absolute, otherwise the system shells.
pub fn login_shell() -> Option<PathBuf> {
    if let Some(shell) = std::env::var_os("SHELL") {
        let path = PathBuf::from(shell);
        if path.is_absolute() && path.is_file() {
            return Some(path);
        }
    }
    ["/bin/zsh", "/bin/bash", "/bin/sh"]
        .into_iter()
        .map(PathBuf::from)
        .find(|path| path.is_file())
}

/// Parses the delimited NUL-separated `env -0` payload and drops owned names.
pub fn variables_from_dump(bytes: &[u8]) -> Vec<(String, String)> {
    let Some(payload) = payload(bytes) else {
        return Vec::new();
    };
    payload
        .split(|byte| *byte == 0)
        .filter_map(|entry| {
            let text = std::str::from_utf8(entry).ok()?;
            let (key, value) = text.split_once('=')?;
            if key.is_empty() || !keep(key) {
                return None;
            }
            Some((key.to_owned(), value.to_owned()))
        })
        .collect()
}

fn payload(bytes: &[u8]) -> Option<&[u8]> {
    let start = bytes.iter().position(|byte| *byte == BEGIN)? + 1;
    let end = bytes.iter().rposition(|byte| *byte == END)?;
    (end > start).then(|| &bytes[start..end])
}

fn keep(key: &str) -> bool {
    !PROBE_VARIABLES.contains(&key)
        && !OWNED_VARIABLES.contains(&key)
        && !OWNED_PREFIXES.iter().any(|prefix| key.starts_with(prefix))
}

fn timeout_from(value: Option<&str>) -> Duration {
    let millis = value
        .and_then(|value| value.trim().parse::<u64>().ok())
        .map_or(DEFAULT_TIMEOUT_MS, |millis| {
            millis.clamp(MIN_TIMEOUT_MS, MAX_TIMEOUT_MS)
        });
    Duration::from_millis(millis)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dump(entries: &[&str]) -> Vec<u8> {
        let mut bytes = vec![BEGIN];
        for entry in entries {
            bytes.extend_from_slice(entry.as_bytes());
            bytes.push(0);
        }
        bytes.push(END);
        bytes
    }

    #[test]
    fn parses_the_delimited_payload() {
        let bytes = dump(&[
            "PATH=/usr/bin:/bin",
            "DEEPSEEK_API_KEY=sk-test",
            "no-equals",
        ]);
        let variables = variables_from_dump(&bytes);
        assert_eq!(
            variables,
            vec![
                ("PATH".to_owned(), "/usr/bin:/bin".to_owned()),
                ("DEEPSEEK_API_KEY".to_owned(), "sk-test".to_owned())
            ]
        );
    }

    #[test]
    fn ignores_startup_noise_before_the_payload() {
        let mut bytes = b"update available\ncompinit: something\n".to_vec();
        bytes.extend(dump(&["PATH=/bin"]));
        assert_eq!(
            variables_from_dump(&bytes),
            vec![("PATH".to_owned(), "/bin".to_owned())]
        );
        assert!(variables_from_dump(b"no delimiters here").is_empty());
        assert!(variables_from_dump(b"").is_empty());
    }

    #[test]
    fn drops_probe_and_shell_owned_variables() {
        let bytes = dump(&[
            "PWD=/Users/example",
            "OLDPWD=/",
            "SHLVL=2",
            "_=/usr/bin/env",
            "DISABLE_AUTO_UPDATE=true",
            "DSH_HOME=/Users/example/.dshr",
            "DSHR_DESKTOP_PORT=0",
            "TAURI_DEBUG=true",
            "DSH_DEEPSEEK_TOKEN=secret",
            "HOME=/Users/example",
        ]);
        let keys: Vec<String> = variables_from_dump(&bytes)
            .into_iter()
            .map(|(key, _)| key)
            .collect();
        assert_eq!(keys, vec!["DSH_DEEPSEEK_TOKEN", "HOME"]);
    }

    #[test]
    fn keeps_values_that_contain_separators() {
        let bytes = dump(&["PROMPT=a b=c", "MULTI=one"]);
        let variables = variables_from_dump(&bytes);
        assert_eq!(variables[0], ("PROMPT".to_owned(), "a b=c".to_owned()));
        assert_eq!(variables[1], ("MULTI".to_owned(), "one".to_owned()));
    }

    #[test]
    fn clamps_the_probe_timeout() {
        assert_eq!(
            timeout_from(None),
            Duration::from_millis(DEFAULT_TIMEOUT_MS)
        );
        assert_eq!(
            timeout_from(Some("junk")),
            Duration::from_millis(DEFAULT_TIMEOUT_MS)
        );
        assert_eq!(
            timeout_from(Some("10")),
            Duration::from_millis(MIN_TIMEOUT_MS)
        );
        assert_eq!(
            timeout_from(Some("999999999999")),
            Duration::from_millis(MAX_TIMEOUT_MS)
        );
        assert_eq!(timeout_from(Some(" 2500 ")), Duration::from_millis(2_500));
    }

    #[test]
    fn finds_an_absolute_login_shell() {
        let shell = login_shell().expect("login shell");
        assert!(shell.is_absolute(), "{}", shell.display());
        assert!(shell.is_file(), "{}", shell.display());
    }

    #[cfg(unix)]
    #[test]
    fn capture_reads_the_real_login_shell() {
        // Runs the user's shell once; CI shells are interactive but headless.
        let environment = capture();
        if let Some(environment) = environment {
            assert!(
                environment.shell.starts_with('/'),
                "{:?}",
                environment.shell
            );
            assert!(
                environment
                    .variables
                    .iter()
                    .any(|(key, _)| key == "PATH" || key == "HOME"),
                "no PATH/HOME in capture"
            );
        }
    }
}

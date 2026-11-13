//! Thin Tauri shell over `dshr serve`.
//!
//! The desktop process starts a loopback server with a fresh bearer token,
//! waits for `/v1/health`, and opens the Leptos UI. No native APIs are exposed
//! to the page; the protocol is the same HTTP/SSE contract. The tray owns the
//! lifecycle: closing the window hides it, profiles restart the shell, and the
//! diagnostics export is available without a running window.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use tauri::menu::{Menu, MenuItem, Submenu};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

struct DesktopState {
    server: Mutex<Option<Child>>,
    port: u16,
    token: String,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let folder = match folder_argument(&args) {
        Ok(folder) => folder,
        Err(error) => {
            eprintln!("dshr-desktop: {error}");
            std::process::exit(1);
        }
    };
    if args.iter().any(|argument| argument == "--check") {
        match check_server(folder.as_deref()) {
            Ok(url) => println!("desktop check ok: {url}"),
            Err(error) => {
                eprintln!("desktop check failed: {error}");
                std::process::exit(1);
            }
        }
        return;
    }
    if args
        .iter()
        .any(|argument| argument == "--export-diagnostics")
    {
        match export_diagnostics() {
            Ok(path) => println!("{}", path.display()),
            Err(error) => {
                eprintln!("diagnostics failed: {error}");
                std::process::exit(1);
            }
        }
        return;
    }

    tauri::Builder::default()
        .setup(move |app| {
            let home = home_dir();
            let first_run = ensure_home(&home);
            let profile = current_profile(&home);
            let (port, token, child) = match start_server(&home, &profile, folder.as_deref()) {
                Ok(started) => started,
                Err(error) => {
                    show_error(
                        "dshr could not start",
                        &format!(
                            "{error}\n\nConfigure the provider (DEEPSEEK_API_KEY / config.yaml) \
                             and check `dshr doctor`."
                        ),
                    );
                    return Err(error);
                }
            };
            if first_run {
                notify("dshr", "First run: home initialized");
            }
            app.manage(DesktopState {
                server: Mutex::new(Some(child)),
                port,
                token: token.clone(),
            });

            let url = format!("http://127.0.0.1:{port}/?token={token}");
            let window = WebviewWindowBuilder::new(app, "main", WebviewUrl::External(url.parse()?))
                .title("dshr")
                .inner_size(1280.0, 860.0)
                .min_inner_size(720.0, 480.0)
                .build()?;

            build_tray(app, &home, &profile)?;
            let _ = window;
            std::thread::spawn(|| {
                if let Ok(Some(release)) = check_for_updates() {
                    notify(
                        "dshr update available",
                        &format!("{} is available", release.version),
                    );
                }
            });
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .build(tauri::generate_context!())
        .expect("build dshr desktop")
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event {
                stop_server(app);
            }
        });
}

fn build_tray(
    app: &mut tauri::App,
    home: &Path,
    profile: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let show = MenuItem::with_id(app, "show", "Show window", true, None::<&str>)?;
    let hide = MenuItem::with_id(app, "hide", "Hide window", true, None::<&str>)?;
    let browser = MenuItem::with_id(app, "browser", "Open in browser", true, None::<&str>)?;
    let terminal = MenuItem::with_id(app, "terminal", "Open terminal", true, None::<&str>)?;
    let new_profile = MenuItem::with_id(app, "new-profile", "New profile…", true, None::<&str>)?;
    let config_folder = MenuItem::with_id(
        app,
        "config-folder",
        "Open config folder",
        true,
        None::<&str>,
    )?;
    let diagnostics = MenuItem::with_id(
        app,
        "diagnostics",
        "Export diagnostics…",
        true,
        None::<&str>,
    )?;
    let updates = MenuItem::with_id(app, "updates", "Check for updates…", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

    let profiles = Submenu::new(app, "Profile", true)?;
    for name in list_profiles(home) {
        let label = if name == profile {
            format!("✓ {name}")
        } else {
            name.clone()
        };
        let item = MenuItem::with_id(app, format!("profile:{name}"), label, true, None::<&str>)?;
        profiles.append(&item)?;
    }

    let menu = Menu::with_items(
        app,
        &[
            &show,
            &hide,
            &profiles,
            &new_profile,
            &browser,
            &terminal,
            &config_folder,
            &updates,
            &diagnostics,
            &quit,
        ],
    )?;
    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or("missing default window icon")?;
    TrayIconBuilder::with_id("dshr")
        .icon(icon)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "hide" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }
            }
            "browser" => {
                if let Some(state) = app.try_state::<DesktopState>() {
                    open_url(&format!(
                        "http://127.0.0.1:{}/?token={}",
                        state.port, state.token
                    ));
                }
            }
            "terminal" => open_terminal(),
            "config-folder" => open_reveal(&home_dir()),
            "new-profile" => {
                if let Some(name) = prompt_text("New profile", "Profile name", "") {
                    let home = home_dir();
                    match create_profile(&home, &name) {
                        Ok(name) => {
                            let _ = std::fs::write(home.join("desktop-profile"), &name);
                            stop_server(app);
                            app.restart();
                        }
                        Err(error) => show_error("New profile", &error),
                    }
                }
            }
            "updates" => {
                std::thread::spawn(|| match check_for_updates() {
                    Ok(Some(release)) => {
                        notify(
                            "dshr update available",
                            &format!("{} — opening the release page", release.version),
                        );
                        open_url(&release.url);
                    }
                    Ok(None) => notify("dshr", "You are running the latest version"),
                    Err(error) => notify("dshr update check failed", &error),
                });
            }
            "diagnostics" => {
                let _ = export_diagnostics();
            }
            "quit" => {
                stop_server(app);
                app.exit(0);
            }
            other if other.starts_with("profile:") => {
                let name = other.trim_start_matches("profile:").to_owned();
                let home = home_dir();
                if let Err(error) = std::fs::write(home.join("desktop-profile"), &name) {
                    eprintln!("profile switch failed: {error}");
                    return;
                }
                stop_server(app);
                app.restart();
            }
            _ => {}
        })
        .build(app)?;
    Ok(())
}

fn start_server(
    home: &Path,
    profile: &str,
    folder: Option<&Path>,
) -> Result<(u16, String, Child), Box<dyn std::error::Error>> {
    let port = desktop_port()?;
    let token = format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    );
    let binary = find_binary()?;
    std::fs::create_dir_all(home)?;
    let log_path = home.join("desktop.log");
    let log = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)?;
    let err = log.try_clone()?;
    log_line(
        home,
        &format!(
            "start profile={profile} port={port} bin={} folder={}",
            binary.display(),
            folder.map_or("-".into(), |path| path.display().to_string())
        ),
    );
    let mut command = Command::new(&binary);
    if let Some(folder) = folder {
        command.current_dir(folder);
    }
    let child = command
        .args([
            "--home",
            &home.display().to_string(),
            "--profile",
            profile,
            "serve",
            "--bind",
            "127.0.0.1",
            "--port",
            &port.to_string(),
            "--token",
            &token,
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::from(log))
        .stderr(Stdio::from(err))
        .spawn()?;
    wait_health(port, Duration::from_secs(20))?;
    Ok((port, token, child))
}

fn stop_server(app: &AppHandle) {
    if let Some(state) = app.try_state::<DesktopState>()
        && let Ok(mut slot) = state.server.lock()
        && let Some(mut child) = slot.take()
    {
        let _ = child.kill();
        let _ = child.wait();
    }
}

fn desktop_port() -> Result<u16, Box<dyn std::error::Error>> {
    if let Ok(value) = std::env::var("DSHR_DESKTOP_PORT") {
        let port: u16 = value.parse()?;
        return Ok(port);
    }
    Ok(free_port()?)
}

fn home_dir() -> PathBuf {
    if let Some(home) = std::env::var_os("DSH_HOME") {
        return PathBuf::from(home);
    }
    std::env::var_os("HOME")
        .map(|home| PathBuf::from(home).join(".dshr"))
        .unwrap_or_else(|| PathBuf::from(".dshr"))
}

fn current_profile(home: &Path) -> String {
    std::fs::read_to_string(home.join("desktop-profile"))
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "default".to_owned())
}

fn list_profiles(home: &Path) -> Vec<String> {
    let mut profiles = vec!["default".to_owned()];
    if let Ok(entries) = std::fs::read_dir(home.join("profiles")) {
        for entry in entries.flatten() {
            if entry.path().is_dir()
                && let Some(name) = entry.file_name().to_str()
                && !profiles.iter().any(|existing| existing == name)
            {
                profiles.push(name.to_owned());
            }
        }
    }
    profiles.sort();
    profiles
}

fn log_line(home: &Path, message: &str) {
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(home.join("desktop.log"))
    {
        let _ = writeln!(file, "{} {message}", now_seconds());
    }
}

fn now_seconds() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

fn export_diagnostics() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let home = home_dir();
    let profile = current_profile(&home);
    let stamp = now_seconds();
    let directory = home.join(format!("diagnostics-{stamp}"));
    std::fs::create_dir_all(&directory)?;
    let mut info = String::new();
    info.push_str(&format!("dshr-desktop {}\n", env!("CARGO_PKG_VERSION")));
    info.push_str(&format!("profile: {profile}\n"));
    info.push_str(&format!("home: {}\n", home.display()));
    info.push_str(&format!("cwd: {}\n", std::env::current_dir()?.display()));
    info.push_str(&format!("os: {}\n", std::env::consts::OS));
    info.push_str(&format!("arch: {}\n", std::env::consts::ARCH));
    if let Ok(output) = Command::new("sw_vers").output() {
        info.push_str(&String::from_utf8_lossy(&output.stdout));
    }
    std::fs::write(directory.join("system-info.txt"), info)?;
    if home.join("desktop.log").is_file() {
        let _ = std::fs::copy(home.join("desktop.log"), directory.join("desktop.log"));
    }
    if let Ok(binary) = find_binary()
        && let Ok(output) = Command::new(binary).arg("doctor").output()
    {
        std::fs::write(directory.join("doctor.txt"), output.stdout)?;
        std::fs::write(directory.join("doctor.err"), output.stderr)?;
    }
    let archive = home.join(format!("dshr-diagnostics-{stamp}.tar.gz"));
    let status = Command::new("tar")
        .args([
            "-czf",
            &archive.display().to_string(),
            "-C",
            &directory.display().to_string(),
            ".",
        ])
        .status()?;
    if !status.success() {
        return Err("tar failed".into());
    }
    let _ = std::fs::remove_dir_all(&directory);
    open_reveal(&archive);
    Ok(archive)
}

#[derive(Debug, PartialEq)]
struct Release {
    version: String,
    url: String,
}

fn fetch_latest_release() -> Result<Release, String> {
    let output = Command::new("curl")
        .args([
            "-fsSL",
            "-H",
            "User-Agent: dshr-desktop",
            "https://api.github.com/repos/maximusmakarov/deepseek_harness_rust/releases/latest",
        ])
        .output()
        .map_err(|error| format!("curl: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "GitHub API HTTP error: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let text = String::from_utf8_lossy(&output.stdout);
    parse_release(&text).ok_or_else(|| "invalid release payload".to_owned())
}

fn parse_release(text: &str) -> Option<Release> {
    let value: serde_json::Value = serde_json::from_str(text).ok()?;
    let tag = value["tag_name"].as_str()?.trim_start_matches('v');
    let url = value["html_url"].as_str()?;
    Some(Release {
        version: tag.to_owned(),
        url: url.to_owned(),
    })
}

fn check_for_updates() -> Result<Option<Release>, String> {
    let release = fetch_latest_release()?;
    if is_newer(env!("CARGO_PKG_VERSION"), &release.version) {
        Ok(Some(release))
    } else {
        Ok(None)
    }
}

fn is_newer(current: &str, latest: &str) -> bool {
    fn version(value: &str) -> (u64, u64, u64) {
        let mut parts = value
            .trim_start_matches('v')
            .split(['.', '-', '+'])
            .take(3)
            .map(|part| part.parse().unwrap_or(0));
        (
            parts.next().unwrap_or(0),
            parts.next().unwrap_or(0),
            parts.next().unwrap_or(0),
        )
    }
    version(latest) > version(current)
}

fn notify(title: &str, body: &str) {
    #[cfg(target_os = "macos")]
    {
        let script = format!(
            "display notification \"{}\" with title \"{}\"",
            body.replace('"', "'"),
            title.replace('"', "'")
        );
        let _ = Command::new("osascript").args(["-e", &script]).status();
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let _ = Command::new("notify-send").args([title, body]).status();
    }
    #[cfg(not(unix))]
    {
        eprintln!("{title}: {body}");
    }
}

fn open_reveal(path: &Path) {
    #[cfg(target_os = "macos")]
    let _ = Command::new("open").arg("-R").arg(path).status();
    #[cfg(target_os = "windows")]
    let _ = Command::new("explorer")
        .arg(format!("/select,{}", path.display()))
        .status();
    #[cfg(all(unix, not(target_os = "macos")))]
    let _ = Command::new("xdg-open")
        .arg(path.parent().unwrap_or(path))
        .status();
}

fn open_url(url: &str) {
    #[cfg(target_os = "macos")]
    let _ = Command::new("open").arg(url).status();
    #[cfg(target_os = "windows")]
    let _ = Command::new("cmd").args(["/C", "start", "", url]).status();
    #[cfg(all(unix, not(target_os = "macos")))]
    let _ = Command::new("xdg-open").arg(url).status();
}

fn open_terminal() {
    #[cfg(target_os = "macos")]
    let _ = Command::new("open").args(["-a", "Terminal"]).status();
    #[cfg(target_os = "windows")]
    let _ = Command::new("cmd").args(["/C", "start", "cmd"]).status();
    #[cfg(all(unix, not(target_os = "macos")))]
    let _ = Command::new("x-terminal-emulator").status();
}

fn check_server(folder: Option<&Path>) -> Result<String, Box<dyn std::error::Error>> {
    let home = home_dir();
    let profile = current_profile(&home);
    let (port, _token, mut child) = start_server(&home, &profile, folder)?;
    let _ = child.kill();
    let _ = child.wait();
    Ok(format!("http://127.0.0.1:{port}/?token=…"))
}

fn ensure_home(home: &Path) -> bool {
    let fresh = !home.is_dir();
    if fresh {
        let _ = std::fs::create_dir_all(home);
        let readme = "dshr desktop home\n\n\
                      Configure the provider with DEEPSEEK_API_KEY or a config.yaml,\n\
                      then restart the app. Profiles live in ./profiles/<name>.\n";
        let _ = std::fs::write(home.join("README.txt"), readme);
    }
    if !home.join("desktop-profile").is_file() {
        let _ = std::fs::write(home.join("desktop-profile"), "default\n");
    }
    fresh
}

fn valid_profile_name(name: &str) -> bool {
    !name.is_empty()
        && name != "default"
        && name.len() <= 64
        && name.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.')
        })
}

fn create_profile(home: &Path, name: &str) -> Result<String, String> {
    let name = name.trim();
    if !valid_profile_name(name) {
        return Err(
            "profile name must be 1-64 ASCII letters, digits, '.', '-' or '_' and not 'default'"
                .to_owned(),
        );
    }
    let directory = home.join("profiles").join(name);
    if directory.exists() {
        return Err(format!("profile '{name}' already exists"));
    }
    std::fs::create_dir_all(&directory)
        .map_err(|error| format!("create {}: {error}", directory.display()))?;
    Ok(name.to_owned())
}

fn prompt_text(title: &str, prompt: &str, default: &str) -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        let script = format!(
            "display dialog \"{}\" default answer \"{}\" buttons {{\"Cancel\", \"OK\"}} default button \"OK\" with title \"{}\"",
            escape_osascript(prompt),
            escape_osascript(default),
            escape_osascript(title)
        );
        let output = Command::new("osascript")
            .args(["-e", &script])
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let text = String::from_utf8_lossy(&output.stdout);
        text.split("text returned:")
            .nth(1)
            .map(|value| value.trim().to_owned())
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let output = Command::new("zenity")
            .args(["--entry", "--title", title, "--text", prompt])
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        Some(String::from_utf8_lossy(&output.stdout).trim().to_owned())
    }
    #[cfg(not(unix))]
    {
        let _ = (title, prompt, default);
        None
    }
}

#[cfg(target_os = "macos")]
fn escape_osascript(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn show_error(title: &str, message: &str) {
    #[cfg(target_os = "macos")]
    {
        let script = format!(
            "display dialog \"{}\" buttons {{\"OK\"}} default button 1 with title \"{}\" with icon caution",
            escape_osascript(message),
            escape_osascript(title)
        );
        let _ = Command::new("osascript").args(["-e", &script]).status();
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let _ = Command::new("zenity")
            .args(["--error", "--title", title, "--text", message])
            .status();
    }
    #[cfg(not(unix))]
    {
        eprintln!("{title}: {message}");
    }
}

fn folder_argument(args: &[String]) -> Result<Option<PathBuf>, String> {
    let mut positional = args.iter().skip(1).filter(|arg| !arg.starts_with('-'));
    let Some(value) = positional.next() else {
        return Ok(None);
    };
    let path = PathBuf::from(value);
    if !path.exists() {
        return Err(format!("folder does not exist: {}", path.display()));
    }
    if !path.is_dir() {
        return Err(format!("not a folder: {}", path.display()));
    }
    Ok(Some(std::fs::canonicalize(&path).map_err(|error| {
        format!("folder {}: {error}", path.display())
    })?))
}

fn free_port() -> std::io::Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    Ok(listener.local_addr()?.port())
}

fn wait_health(port: u16, timeout: Duration) -> std::io::Result<()> {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if TcpStream::connect_timeout(
            &format!("127.0.0.1:{port}").parse().expect("loopback addr"),
            Duration::from_millis(250),
        )
        .is_ok()
        {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(150));
    }
    Err(std::io::Error::other(format!(
        "dshr serve did not start on port {port} within {timeout:?}"
    )))
}

fn find_binary() -> std::io::Result<PathBuf> {
    if let Some(path) = std::env::var_os("DSHR_BIN") {
        return Ok(PathBuf::from(path));
    }
    if let Ok(current) = std::env::current_exe()
        && let Some(parent) = current.parent()
    {
        let sibling = parent.join(if cfg!(windows) { "dshr.exe" } else { "dshr" });
        if sibling.is_file() {
            return Ok(sibling);
        }
    }
    if let Some(path) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path) {
            let candidate = dir.join(if cfg!(windows) { "dshr.exe" } else { "dshr" });
            if candidate.is_file() {
                return Ok(candidate);
            }
        }
    }
    Err(std::io::Error::other(
        "dshr binary not found; build `dshr-cli` or set DSHR_BIN",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn free_port_is_usable() {
        let port = free_port().expect("port");
        assert_ne!(port, 0);
        let listener = TcpListener::bind(("127.0.0.1", port)).expect("rebind");
        drop(listener);
    }

    #[test]
    fn validates_and_creates_profiles() {
        assert!(valid_profile_name("work"));
        assert!(valid_profile_name("work-2.x"));
        assert!(!valid_profile_name(""));
        assert!(!valid_profile_name("default"));
        assert!(!valid_profile_name("../escape"));
        assert!(!valid_profile_name("a b"));

        let dir = std::env::temp_dir().join(format!("dshr-profile-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("dir");
        assert_eq!(create_profile(&dir, "work").expect("create"), "work");
        assert!(dir.join("profiles/work").is_dir());
        assert!(create_profile(&dir, "work").is_err());
        assert!(create_profile(&dir, "..").is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn initializes_a_empty_home() {
        let dir = std::env::temp_dir().join(format!("dshr-home-test-{}", uuid::Uuid::new_v4()));
        assert!(ensure_home(&dir));
        assert!(dir.join("README.txt").is_file());
        assert_eq!(
            std::fs::read_to_string(dir.join("desktop-profile")).expect("profile"),
            "default\n"
        );
        assert!(!ensure_home(&dir));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn compares_versions() {
        assert!(is_newer("0.1.0", "0.1.1"));
        assert!(is_newer("0.1.0", "v0.2.0"));
        assert!(!is_newer("0.2.0", "0.2.0"));
        assert!(!is_newer("0.2.0", "0.1.9"));
        assert!(is_newer("1.0.0", "1.0.1-rc.1"));
    }

    #[test]
    fn parses_a_release_payload() {
        let release = parse_release(
            r#"{"tag_name":"v9.9.9","html_url":"https://example.test/releases/v9.9.9"}"#,
        )
        .expect("release");
        assert_eq!(release.version, "9.9.9");
        assert_eq!(release.url, "https://example.test/releases/v9.9.9");
        assert!(parse_release("{}").is_none());
    }

    #[test]
    fn parses_a_folder_argument() {
        let dir = std::env::temp_dir().join(format!("dshr-desktop-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("dir");
        let args = vec!["dshr-desktop".to_owned(), dir.display().to_string()];
        assert_eq!(
            folder_argument(&args).expect("folder").expect("some"),
            std::fs::canonicalize(&dir).expect("canonical")
        );
        let file = dir.join("file.txt");
        std::fs::write(&file, b"x").expect("file");
        let args = vec!["dshr-desktop".to_owned(), file.display().to_string()];
        assert!(folder_argument(&args).is_err());
        assert!(
            folder_argument(&["dshr-desktop".to_owned()])
                .expect("none")
                .is_none()
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn lists_default_and_custom_profiles() {
        let dir = std::env::temp_dir().join(format!("dshr-desktop-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(dir.join("profiles/work")).expect("profile");
        std::fs::create_dir_all(dir.join("profiles/other")).expect("profile");
        let profiles = list_profiles(&dir);
        assert_eq!(profiles, vec!["default", "other", "work"]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn finds_a_binary_from_the_environment() {
        let dir = std::env::temp_dir().join(format!("dshr-desktop-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("dir");
        let binary = dir.join("dshr");
        std::fs::write(&binary, b"#!/bin/sh\n").expect("write");
        // SAFETY: the test process owns this variable; no other test reads it.
        unsafe { std::env::set_var("DSHR_BIN", &binary) };
        assert_eq!(find_binary().expect("binary"), binary);
        unsafe { std::env::remove_var("DSHR_BIN") };
        let _ = std::fs::remove_dir_all(&dir);
    }
}

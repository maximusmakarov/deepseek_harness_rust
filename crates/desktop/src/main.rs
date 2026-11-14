//! Thin Tauri shell over `dshr serve`.
//!
//! The desktop process starts a loopback server with a fresh bearer token,
//! waits for `/v1/health`, and opens the Leptos UI. No native APIs are exposed
//! to the page; the protocol is the same HTTP/SSE contract. The tray owns the
//! lifecycle: closing the window hides it, profiles restart the shell, and the
//! diagnostics export is available without a running window.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::collections::BTreeMap;
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
        .invoke_handler(tauri::generate_handler![recovery_action])
        .setup(move |app| {
            let home = home_dir();
            let first_run = ensure_home(&home);
            let profile = current_profile(&home);
            let folder = folder.clone().or_else(|| load_desktop_folder(&home));
            let provider = match ensure_provider(&home, true) {
                Ok(provider) => provider,
                Err(error) => {
                    show_error("Provider setup", &error);
                    return Err(error.into());
                }
            };
            let (port, token, child) =
                match start_server(&home, &profile, folder.as_deref(), provider.as_ref()) {
                    Ok(started) => started,
                    Err(error) => {
                        show_recovery_window(app.handle(), &error)?;
                        return Ok(());
                    }
                };
            save_last_good(&home, &profile);
            if first_run {
                notify("dshr", "First run: home initialized");
            }
            app.manage(DesktopState {
                server: Mutex::new(Some(child)),
                port,
                token: token.clone(),
            });

            let url = format!("http://127.0.0.1:{port}/?token={token}");
            let window_settings = load_window_settings(&home);
            let mut builder =
                WebviewWindowBuilder::new(app, "main", WebviewUrl::External(url.parse()?))
                    .title("dshr")
                    .inner_size(1280.0, 860.0)
                    .min_inner_size(720.0, 480.0);
            #[cfg(target_os = "macos")]
            {
                if window_settings.mode == "enhanced" {
                    builder = builder
                        .title_bar_style(tauri::TitleBarStyle::Overlay)
                        .hidden_title(true);
                } else if window_settings.mode == "extended" {
                    builder = builder
                        .title_bar_style(tauri::TitleBarStyle::Overlay)
                        .hidden_title(true)
                        .traffic_light_position(tauri::LogicalPosition::new(16.0, 18.0));
                }
                if window_settings.material {
                    builder = builder.transparent(true);
                }
            }
            let window = builder.build()?;
            let handle = app.handle().clone();
            window.on_window_event(move |event| {
                if let tauri::WindowEvent::DragDrop(tauri::DragDropEvent::Drop { paths, .. }) =
                    event
                    && let Some(path) = paths.iter().find(|path| path.is_dir())
                {
                    let home = home_dir();
                    if save_desktop_folder(&home, Some(path)).is_ok() {
                        stop_server(&handle);
                        handle.restart();
                    }
                }
            });
            #[cfg(target_os = "macos")]
            if window_settings.material {
                use window_vibrancy::{NSVisualEffectMaterial, apply_vibrancy};
                if let Err(error) =
                    apply_vibrancy(&window, NSVisualEffectMaterial::Sidebar, None, Some(16.0))
                {
                    eprintln!("vibrancy: {error}");
                }
            }

            build_tray(app, &home, &profile)?;
            let _ = window;
            let channel_home = home.clone();
            std::thread::spawn(move || {
                if let Ok(Some(release)) = check_for_updates(load_update_channel(&channel_home)) {
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
    let provider = MenuItem::with_id(app, "provider", "Provider setup…", true, None::<&str>)?;
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
    let download = MenuItem::with_id(
        app,
        "download-update",
        "Download update…",
        true,
        None::<&str>,
    )?;
    let safe_restart = MenuItem::with_id(
        app,
        "safe-restart",
        "Restart in safe mode",
        true,
        None::<&str>,
    )?;
    let normal_restart = MenuItem::with_id(
        app,
        "normal-restart",
        "Restart normally",
        true,
        None::<&str>,
    )?;
    let channel = load_update_channel(home);
    let updates_menu = Submenu::new(app, "Updates", true)?;
    let stable = MenuItem::with_id(
        app,
        "channel:stable",
        if channel == UpdateChannel::Stable {
            "✓ Stable"
        } else {
            "Stable"
        },
        true,
        None::<&str>,
    )?;
    let beta = MenuItem::with_id(
        app,
        "channel:beta",
        if channel == UpdateChannel::Beta {
            "✓ Beta (prereleases)"
        } else {
            "Beta (prereleases)"
        },
        true,
        None::<&str>,
    )?;
    updates_menu.append(&stable)?;
    updates_menu.append(&beta)?;

    let rollback = MenuItem::with_id(app, "rollback", "Rollback profile", true, None::<&str>)?;
    let install = MenuItem::with_id(
        app,
        "install-update",
        "Install downloaded update…",
        true,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

    let window_settings = load_window_settings(home);
    let network = load_network_settings(home);
    let network_menu = Submenu::new(app, "Network", true)?;
    let local = MenuItem::with_id(
        app,
        "network:local",
        if network.bind == "127.0.0.1" {
            "✓ Local only"
        } else {
            "Local only"
        },
        true,
        None::<&str>,
    )?;
    let lan = MenuItem::with_id(
        app,
        "network:lan",
        if network.bind == "0.0.0.0" {
            "✓ LAN (0.0.0.0)"
        } else {
            "LAN (0.0.0.0)"
        },
        true,
        None::<&str>,
    )?;
    let set_port = MenuItem::with_id(app, "network:port", "Set port…", true, None::<&str>)?;
    network_menu.append(&local)?;
    network_menu.append(&lan)?;
    network_menu.append(&set_port)?;

    let window_menu = Submenu::new(app, "Window", true)?;
    let mode_compat = MenuItem::with_id(
        app,
        "window:mode:compatibility",
        if window_settings.mode == "compatibility" {
            "✓ Compatibility"
        } else {
            "Compatibility"
        },
        true,
        None::<&str>,
    )?;
    let mode_enhanced = MenuItem::with_id(
        app,
        "window:mode:enhanced",
        if window_settings.mode == "enhanced" {
            "✓ Enhanced"
        } else {
            "Enhanced"
        },
        true,
        None::<&str>,
    )?;
    let mode_extended = MenuItem::with_id(
        app,
        "window:mode:extended",
        if window_settings.mode == "extended" {
            "✓ Extended"
        } else {
            "Extended"
        },
        true,
        None::<&str>,
    )?;
    let material = MenuItem::with_id(
        app,
        "window:material",
        if window_settings.material {
            "✓ Material"
        } else {
            "Material"
        },
        true,
        None::<&str>,
    )?;
    window_menu.append(&mode_compat)?;
    window_menu.append(&mode_enhanced)?;
    window_menu.append(&mode_extended)?;
    window_menu.append(&material)?;

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
            &window_menu,
            &network_menu,
            &updates_menu,
            &new_profile,
            &browser,
            &terminal,
            &provider,
            &config_folder,
            &updates,
            &download,
            &safe_restart,
            &normal_restart,
            &rollback,
            &install,
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
            "safe-restart" => {
                let home = home_dir();
                let _ = std::fs::write(home.join("desktop-safe-mode"), "1\n");
                stop_server(app);
                app.restart();
            }
            "install-update" => {
                let home = home_dir();
                match install_downloaded_update(&home) {
                    Ok(bundle) => {
                        notify(
                            "dshr update installing",
                            &format!("Replacing {} and restarting", bundle.display()),
                        );
                        stop_server(app);
                        app.exit(0);
                    }
                    Err(error) => notify("dshr update install failed", &error),
                }
            }
            "rollback" => {
                let home = home_dir();
                match load_last_good(&home) {
                    Some(profile) => {
                        let _ = std::fs::write(home.join("desktop-profile"), &profile);
                        stop_server(app);
                        app.restart();
                    }
                    None => notify("dshr", "No last-known-good profile yet"),
                }
            }
            "normal-restart" => {
                let home = home_dir();
                let _ = std::fs::remove_file(home.join("desktop-safe-mode"));
                stop_server(app);
                app.restart();
            }
            "provider" => {
                let home = home_dir();
                match prompt_provider_setup() {
                    Ok(settings) => match save_provider_settings(&home, &settings) {
                        Ok(()) => {
                            stop_server(app);
                            app.restart();
                        }
                        Err(error) => show_error("Provider setup", &error),
                    },
                    Err(error) => show_error("Provider setup", &error),
                }
            }
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
            "channel:stable" => {
                let home = home_dir();
                if save_update_channel(&home, UpdateChannel::Stable).is_ok() {
                    notify("dshr", "Update channel: stable");
                }
            }
            "channel:beta" => {
                let home = home_dir();
                if save_update_channel(&home, UpdateChannel::Beta).is_ok() {
                    notify("dshr", "Update channel: beta");
                }
            }
            "updates" => {
                std::thread::spawn(|| match check_for_updates(load_update_channel(&home_dir())) {
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
            "download-update" => {
                std::thread::spawn(|| {
                    let home = home_dir();
                    match check_for_updates(load_update_channel(&home)) {
                        Ok(Some(release)) => match download_update(&home, &release) {
                            Ok(path) => {
                                notify(
                                    "dshr update downloaded",
                                    &format!("{} — quit and replace the app", path.display()),
                                );
                                open_reveal(&path);
                            }
                            Err(error) => notify("dshr update download failed", &error),
                        },
                        Ok(None) => notify("dshr", "You are running the latest version"),
                        Err(error) => notify("dshr update check failed", &error),
                    }
                });
            }
            "diagnostics" => {
                let _ = export_diagnostics();
            }
            "quit" => {
                stop_server(app);
                app.exit(0);
            }
            other if other.starts_with("network:") => {
                let home = home_dir();
                let mut settings = load_network_settings(&home);
                match other {
                    "network:local" => settings.bind = "127.0.0.1".to_owned(),
                    "network:lan" => {
                        show_error(
                            "LAN access",
                            "The server will bind 0.0.0.0. The bearer token is still required, but every host on the network can reach the UI.",
                        );
                        settings.bind = "0.0.0.0".to_owned();
                    }
                    "network:port" => {
                        match prompt_text(
                            "Local web port",
                            "Port (0 = random)",
                            &settings.port.to_string(),
                        ) {
                            Some(value) => match value.trim().parse::<u16>() {
                                Ok(port) => settings.port = port,
                                Err(error) => {
                                    show_error("Local web port", &format!("invalid port: {error}"));
                                    return;
                                }
                            },
                            None => return,
                        }
                    }
                    _ => return,
                }
                if save_network_settings(&home, &settings).is_ok() {
                    stop_server(app);
                    app.restart();
                }
            }
            other if other.starts_with("window:") => {
                let home = home_dir();
                let mut settings = load_window_settings(&home);
                match other {
                    "window:mode:compatibility" => settings.mode = "compatibility".to_owned(),
                    "window:mode:enhanced" => settings.mode = "enhanced".to_owned(),
                    "window:mode:extended" => settings.mode = "extended".to_owned(),
                    "window:material" => settings.material = !settings.material,
                    _ => return,
                }
                if save_window_settings(&home, &settings).is_ok() {
                    stop_server(app);
                    app.restart();
                }
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
    provider: Option<&ProviderSettings>,
) -> Result<(u16, String, Child), Box<dyn std::error::Error>> {
    let network = load_network_settings(home);
    let port = desktop_port(&network)?;
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
    let safe_mode = home.join("desktop-safe-mode").is_file();
    let mut arguments = vec![
        "--home".to_owned(),
        home.display().to_string(),
        "--profile".to_owned(),
        profile.to_owned(),
        "serve".to_owned(),
        "--bind".to_owned(),
        network.bind.clone(),
        "--port".to_owned(),
        port.to_string(),
        "--token".to_owned(),
        token.clone(),
    ];
    if safe_mode {
        arguments.push("--safe-mode".to_owned());
    }
    if let Some(provider) = provider {
        if let Some(route) = &provider.provider {
            arguments.push("--provider".to_owned());
            arguments.push(route.clone());
        }
        if let Some(model) = &provider.model {
            arguments.push("--model".to_owned());
            arguments.push(model.clone());
        }
    }
    if let Some(provider) = provider {
        command.envs(&provider.env);
    }
    let child = command
        .args(&arguments)
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

fn desktop_port(network: &NetworkSettings) -> Result<u16, Box<dyn std::error::Error>> {
    if let Ok(value) = std::env::var("DSHR_DESKTOP_PORT") {
        let port: u16 = value.parse()?;
        return Ok(port);
    }
    if network.port != 0 {
        return Ok(network.port);
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

fn desktop_folder_path(home: &Path) -> PathBuf {
    home.join("desktop-folder")
}

fn load_desktop_folder(home: &Path) -> Option<PathBuf> {
    let value = std::fs::read_to_string(desktop_folder_path(home)).ok()?;
    let path = PathBuf::from(value.trim());
    path.is_dir().then_some(path)
}

fn save_desktop_folder(home: &Path, folder: Option<&Path>) -> Result<(), String> {
    match folder {
        Some(path) => std::fs::write(desktop_folder_path(home), format!("{}\n", path.display()))
            .map_err(|error| error.to_string()),
        None => match std::fs::remove_file(desktop_folder_path(home)) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.to_string()),
        },
    }
}

fn last_good_path(home: &Path) -> PathBuf {
    home.join("desktop-last-good")
}

fn load_last_good(home: &Path) -> Option<String> {
    std::fs::read_to_string(last_good_path(home))
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty() && value != "default")
}

fn save_last_good(home: &Path, profile: &str) {
    if profile != "default" {
        let _ = std::fs::write(last_good_path(home), format!("{profile}\n"));
    }
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
    macos_zip: Option<String>,
    checksums: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UpdateChannel {
    Stable,
    Beta,
}

fn channel_path(home: &Path) -> PathBuf {
    home.join("desktop-update-channel")
}

fn load_update_channel(home: &Path) -> UpdateChannel {
    match std::fs::read_to_string(channel_path(home)) {
        Ok(value) if value.trim() == "beta" => UpdateChannel::Beta,
        _ => UpdateChannel::Stable,
    }
}

fn save_update_channel(home: &Path, channel: UpdateChannel) -> Result<(), String> {
    let value = match channel {
        UpdateChannel::Stable => "stable",
        UpdateChannel::Beta => "beta",
    };
    std::fs::write(channel_path(home), format!("{value}\n")).map_err(|error| error.to_string())
}

fn fetch_latest_release(channel: UpdateChannel) -> Result<Release, String> {
    let url = match channel {
        UpdateChannel::Stable => {
            "https://api.github.com/repos/maximusmakarov/deepseek_harness_rust/releases/latest"
        }
        UpdateChannel::Beta => {
            "https://api.github.com/repos/maximusmakarov/deepseek_harness_rust/releases?per_page=20"
        }
    };
    let output = Command::new("curl")
        .args(["-fsSL", "-H", "User-Agent: dshr-desktop", url])
        .output()
        .map_err(|error| format!("curl: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "GitHub API HTTP error: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let text = String::from_utf8_lossy(&output.stdout);
    match channel {
        UpdateChannel::Stable => {
            parse_release(&text).ok_or_else(|| "invalid release payload".to_owned())
        }
        UpdateChannel::Beta => {
            parse_beta_releases(&text).ok_or_else(|| "no release with assets found".to_owned())
        }
    }
}

fn parse_beta_releases(text: &str) -> Option<Release> {
    let value: serde_json::Value = serde_json::from_str(text).ok()?;
    value.as_array()?.iter().find_map(|release| {
        let assets = release["assets"].as_array()?;
        if assets.is_empty() {
            return None;
        }
        parse_release(&release.to_string())
    })
}

fn parse_release(text: &str) -> Option<Release> {
    let value: serde_json::Value = serde_json::from_str(text).ok()?;
    let tag = value["tag_name"].as_str()?.trim_start_matches('v');
    let url = value["html_url"].as_str()?;
    let mut macos_zip = None;
    let mut checksums = None;
    for asset in value["assets"].as_array().into_iter().flatten() {
        let name = asset["name"].as_str().unwrap_or_default();
        let download = asset["browser_download_url"].as_str();
        if name.ends_with("-macos.zip") {
            macos_zip = download.map(ToOwned::to_owned);
        } else if name == "SHA256SUMS" {
            checksums = download.map(ToOwned::to_owned);
        }
    }
    Some(Release {
        version: tag.to_owned(),
        url: url.to_owned(),
        macos_zip,
        checksums,
    })
}

fn download_update(home: &Path, release: &Release) -> Result<PathBuf, String> {
    let archive_url = release
        .macos_zip
        .as_deref()
        .ok_or_else(|| "release has no macOS zip asset".to_owned())?;
    let name = archive_url.rsplit('/').next().unwrap_or("dshr-macos.zip");
    let directory = home.join("updates").join(&release.version);
    std::fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    let archive = directory.join(name);
    curl_to(archive_url, &archive)?;
    if let Some(checksums_url) = &release.checksums {
        let sums = directory.join("SHA256SUMS");
        curl_to(checksums_url, &sums)?;
        let text = std::fs::read_to_string(&sums).map_err(|error| error.to_string())?;
        let expected = checksum_for(&text, name)
            .ok_or_else(|| format!("{name} is not listed in SHA256SUMS"))?;
        let actual = sha256_file(&archive)?;
        if !actual.eq_ignore_ascii_case(&expected) {
            return Err(format!(
                "checksum mismatch for {name}: expected {expected}, got {actual}"
            ));
        }
    }
    let unpacked = directory.join("app");
    std::fs::create_dir_all(&unpacked).map_err(|error| error.to_string())?;
    let status = Command::new("ditto")
        .args([
            "-x",
            "-k",
            &archive.display().to_string(),
            &unpacked.display().to_string(),
        ])
        .status()
        .map_err(|error| format!("ditto: {error}"))?;
    if !status.success() {
        return Err("unzip failed".to_owned());
    }
    find_app(&unpacked).ok_or_else(|| "unpacked archive has no .app".to_owned())
}

fn curl_to(url: &str, destination: &Path) -> Result<(), String> {
    let status = Command::new("curl")
        .args([
            "-fsSL",
            "-H",
            "User-Agent: dshr-desktop",
            url,
            "-o",
            &destination.display().to_string(),
        ])
        .status()
        .map_err(|error| format!("curl: {error}"))?;
    if !status.success() {
        return Err(format!("download failed: {url}"));
    }
    Ok(())
}

fn checksum_for(text: &str, name: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let mut parts = line.split_whitespace();
        let hash = parts.next()?;
        let file = parts.next()?.trim_start_matches('*');
        (file == name && hash.len() == 64).then(|| hash.to_owned())
    })
}

fn sha256_file(path: &Path) -> Result<String, String> {
    for (program, args) in [("shasum", vec!["-a", "256"]), ("sha256sum", Vec::new())] {
        let Ok(output) = Command::new(program).args(&args).arg(path).output() else {
            continue;
        };
        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout);
            if let Some(hash) = text.split_whitespace().next()
                && hash.len() == 64
            {
                return Ok(hash.to_ascii_lowercase());
            }
        }
    }
    Err("no shasum/sha256sum available".to_owned())
}

fn install_downloaded_update(home: &Path) -> Result<PathBuf, String> {
    #[cfg(not(target_os = "macos"))]
    {
        let _ = home;
        return Err("silent install is implemented for macOS .app only".to_owned());
    }
    #[cfg(target_os = "macos")]
    {
        let source = newest_downloaded_app(home)
            .ok_or_else(|| "no downloaded .app found; use Download update… first".to_owned())?;
        let executable =
            std::env::current_exe().map_err(|error| format!("current exe: {error}"))?;
        let target = current_app_bundle(&executable)
            .ok_or_else(|| "the app is not running from a .app bundle".to_owned())?;
        if source == target {
            return Err("downloaded app is the running app".to_owned());
        }
        let helper = home.join("updates").join("install-update.sh");
        std::fs::create_dir_all(helper.parent().expect("parent"))
            .map_err(|error| error.to_string())?;
        let script = "#!/bin/sh\nPID=$1\nSOURCE=$2\nTARGET=$3\nwhile kill -0 \"$PID\" 2>/dev/null; do sleep 0.2; done\nrm -rf \"$TARGET\"\ncp -R \"$SOURCE\" \"$TARGET\"\nopen \"$TARGET\"\n".to_owned();
        std::fs::write(&helper, script).map_err(|error| error.to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&helper, std::fs::Permissions::from_mode(0o755))
                .map_err(|error| error.to_string())?;
        }
        let child = Command::new("sh")
            .args([
                helper.display().to_string(),
                std::process::id().to_string(),
                source.display().to_string(),
                target.display().to_string(),
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| format!("spawn updater: {error}"))?;
        // Detach: the helper waits for this process to exit.
        std::mem::forget(child);
        Ok(target)
    }
}

fn newest_downloaded_app(home: &Path) -> Option<PathBuf> {
    let root = home.join("updates");
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    let entries = std::fs::read_dir(&root).ok()?;
    for entry in entries.flatten() {
        let Some(app) = find_app(&entry.path()) else {
            continue;
        };
        let modified = std::fs::metadata(&app)
            .and_then(|metadata| metadata.modified())
            .unwrap_or(std::time::UNIX_EPOCH);
        if best.as_ref().is_none_or(|(time, _)| modified > *time) {
            best = Some((modified, app));
        }
    }
    best.map(|(_, path)| path)
}

fn current_app_bundle(executable: &Path) -> Option<PathBuf> {
    let mut path = executable;
    while let Some(parent) = path.parent() {
        if path.extension().and_then(|value| value.to_str()) == Some("app") {
            return Some(path.to_path_buf());
        }
        path = parent;
    }
    None
}

fn find_app(directory: &Path) -> Option<PathBuf> {
    let entries = std::fs::read_dir(directory).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) == Some("app") {
            return Some(path);
        }
        if path.is_dir()
            && let Some(found) = find_app(&path)
        {
            return Some(found);
        }
    }
    None
}

fn check_for_updates(channel: UpdateChannel) -> Result<Option<Release>, String> {
    let release = fetch_latest_release(channel)?;
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
    let provider = ensure_provider(&home, false)?;
    let (port, _token, mut child) = start_server(&home, &profile, folder, provider.as_ref())?;
    let _ = child.kill();
    let _ = child.wait();
    Ok(format!("http://127.0.0.1:{port}/?token=…"))
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct NetworkSettings {
    #[serde(default = "default_bind")]
    bind: String,
    #[serde(default)]
    port: u16,
}

fn default_bind() -> String {
    "127.0.0.1".to_owned()
}

impl Default for NetworkSettings {
    fn default() -> Self {
        Self {
            bind: default_bind(),
            port: 0,
        }
    }
}

fn network_settings_path(home: &Path) -> PathBuf {
    home.join("desktop-network.json")
}

fn load_network_settings(home: &Path) -> NetworkSettings {
    let Ok(text) = std::fs::read_to_string(network_settings_path(home)) else {
        return NetworkSettings::default();
    };
    let mut settings: NetworkSettings = serde_json::from_str(&text).unwrap_or_default();
    if !matches!(settings.bind.as_str(), "127.0.0.1" | "0.0.0.0") {
        settings.bind = default_bind();
    }
    settings
}

fn save_network_settings(home: &Path, settings: &NetworkSettings) -> Result<(), String> {
    let text = serde_json::to_string_pretty(settings).map_err(|error| error.to_string())?;
    std::fs::write(network_settings_path(home), text)
        .map_err(|error| format!("write desktop-network.json: {error}"))
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct WindowSettings {
    #[serde(default = "default_window_mode")]
    mode: String,
    #[serde(default)]
    material: bool,
}

fn default_window_mode() -> String {
    "compatibility".to_owned()
}

impl Default for WindowSettings {
    fn default() -> Self {
        Self {
            mode: default_window_mode(),
            material: false,
        }
    }
}

fn window_settings_path(home: &Path) -> PathBuf {
    home.join("desktop-window.json")
}

fn load_window_settings(home: &Path) -> WindowSettings {
    let Ok(text) = std::fs::read_to_string(window_settings_path(home)) else {
        return WindowSettings::default();
    };
    let mut settings: WindowSettings = serde_json::from_str(&text).unwrap_or_default();
    if !matches!(
        settings.mode.as_str(),
        "compatibility" | "enhanced" | "extended"
    ) {
        settings.mode = default_window_mode();
    }
    settings
}

fn save_window_settings(home: &Path, settings: &WindowSettings) -> Result<(), String> {
    let text = serde_json::to_string_pretty(settings).map_err(|error| error.to_string())?;
    std::fs::write(window_settings_path(home), text)
        .map_err(|error| format!("write desktop-window.json: {error}"))
}

#[derive(Clone, Debug, Default)]
struct ProviderSettings {
    provider: Option<String>,
    model: Option<String>,
    env: BTreeMap<String, String>,
}

fn desktop_env_path(home: &Path) -> PathBuf {
    home.join("desktop.env")
}

fn has_process_credentials() -> bool {
    [
        "DEEPSEEK_API_KEY",
        "DSH_DEEPSEEK_TOKEN",
        "OPENAI_API_KEY",
        "ROUTERAI_API_KEY",
    ]
    .iter()
    .any(|key| std::env::var_os(key).is_some())
}

fn load_provider_settings(home: &Path) -> Option<ProviderSettings> {
    let text = std::fs::read_to_string(desktop_env_path(home)).ok()?;
    let mut settings = ProviderSettings::default();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        match key.trim() {
            "DSHR_DESKTOP_PROVIDER" => settings.provider = Some(value.trim().to_owned()),
            "DSHR_DESKTOP_MODEL" => settings.model = Some(value.trim().to_owned()),
            key => {
                settings.env.insert(key.to_owned(), value.trim().to_owned());
            }
        }
    }
    Some(settings)
}

fn save_provider_settings(home: &Path, settings: &ProviderSettings) -> Result<(), String> {
    let mut text = String::from("# dshr desktop provider settings\n");
    if let Some(provider) = &settings.provider {
        text.push_str(&format!("DSHR_DESKTOP_PROVIDER={provider}\n"));
    }
    if let Some(model) = &settings.model {
        text.push_str(&format!("DSHR_DESKTOP_MODEL={model}\n"));
    }
    for (key, value) in &settings.env {
        text.push_str(&format!("{key}={value}\n"));
    }
    std::fs::write(desktop_env_path(home), text)
        .map_err(|error| format!("write desktop.env: {error}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(
            desktop_env_path(home),
            std::fs::Permissions::from_mode(0o600),
        )
        .map_err(|error| format!("chmod desktop.env: {error}"))?;
    }
    Ok(())
}

fn ensure_provider(home: &Path, interactive: bool) -> Result<Option<ProviderSettings>, String> {
    if has_process_credentials() {
        return Ok(None);
    }
    if let Some(settings) = load_provider_settings(home) {
        return Ok(Some(settings));
    }
    if !interactive {
        return Err(
            "provider is not configured: set DEEPSEEK_API_KEY/OPENAI_API_KEY or run the app once"
                .to_owned(),
        );
    }
    let settings = prompt_provider_setup()?;
    save_provider_settings(home, &settings)?;
    Ok(Some(settings))
}

fn prompt_provider_setup() -> Result<ProviderSettings, String> {
    let choice = prompt_choice(
        "Provider setup",
        "Which provider should dshr use?",
        &["DeepSeek", "OpenAI-compatible"],
    )?
    .ok_or_else(|| "provider setup cancelled".to_owned())?;
    let (route, model_default, key_env) = if choice == 0 {
        ("deepseek-account", "deepseek-flash", "DEEPSEEK_API_KEY")
    } else {
        ("openai-compatible", "gpt-4o-mini", "OPENAI_API_KEY")
    };
    let model = prompt_text("Provider setup", "Model", model_default)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| model_default.to_owned());
    let key = prompt_secret("Provider setup", "API key")?
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "API key is required".to_owned())?;
    let mut env = BTreeMap::new();
    env.insert(key_env.to_owned(), key.trim().to_owned());
    if route == "openai-compatible" {
        let base = prompt_text("Provider setup", "Base URL", "https://api.openai.com/v1")
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "https://api.openai.com/v1".to_owned());
        env.insert("OPENAI_BASE_URL".to_owned(), base);
    }
    Ok(ProviderSettings {
        provider: Some(route.to_owned()),
        model: Some(model),
        env,
    })
}

#[tauri::command]
fn recovery_action(app: tauri::AppHandle, action: String) {
    let home = home_dir();
    match action.as_str() {
        "retry" => app.restart(),
        "safe" => {
            let _ = std::fs::write(home.join("desktop-safe-mode"), "1\n");
            app.restart()
        }
        "diagnostics" => match export_diagnostics() {
            Ok(path) => notify("diagnostics exported", &path.display().to_string()),
            Err(error) => notify("diagnostics failed", &error.to_string()),
        },
        "quit" => app.exit(0),
        "config" => {
            let _ = Command::new("open").arg(&home).spawn();
        }
        action if action.starts_with("profile:") => {
            let name = action.trim_start_matches("profile:");
            let _ = std::fs::write(home.join("desktop-profile"), format!("{name}\n"));
            app.restart()
        }
        _ => {}
    }
}

fn show_recovery_window(
    app: &tauri::AppHandle,
    error: &dyn std::fmt::Display,
) -> tauri::Result<()> {
    let html = recovery_html(error, &home_dir());
    let url: tauri::Url = data_url(&html)
        .parse::<tauri::Url>()
        .map_err(tauri::Error::InvalidUrl)?;
    WebviewWindowBuilder::new(app, "recovery", WebviewUrl::External(url))
        .title("dshr recovery")
        .inner_size(620.0, 420.0)
        .build()?;
    Ok(())
}

fn recovery_html(error: &dyn std::fmt::Display, home: &Path) -> String {
    let error = escape_html(&error.to_string());
    let mut profiles = String::new();
    for profile in list_profiles(home) {
        profiles.push_str(&format!(
            "<button onclick=\"window.__TAURI__.core.invoke('recovery_action',{{action:'profile:{profile}'}})\">{profile}</button>"
        ));
    }
    format!(
        "<!doctype html><meta charset=\"utf-8\"><style>body{{font:14px ui-sans-serif;margin:24px;background:#1b1b1b;color:#eee}}button{{margin:6px;padding:8px 14px;border-radius:6px;border:1px solid #555;background:#222;color:#eee;cursor:pointer}}pre{{white-space:pre-wrap;color:#f88}}</style><h2>dshr could not start</h2><pre>{error}</pre><p>Retry after fixing the provider/config, or start in safe mode (external plugins disabled).</p><button onclick=\"window.__TAURI__.core.invoke('recovery_action',{{action:'retry'}})\">Retry</button><button onclick=\"window.__TAURI__.core.invoke('recovery_action',{{action:'safe'}})\">Safe mode</button><button onclick=\"window.__TAURI__.core.invoke('recovery_action',{{action:'diagnostics'}})\">Export diagnostics</button><button onclick=\"window.__TAURI__.core.invoke('recovery_action',{{action:'config'}})\">Open config folder</button><button onclick=\"window.__TAURI__.core.invoke('recovery_action',{{action:'quit'}})\">Quit</button><h3>Profiles</h3>{profiles}"
    )
}

fn data_url(html: &str) -> String {
    let mut out = String::from("data:text/html;charset=utf-8,");
    for byte in html.bytes() {
        match byte {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char);
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn prompt_choice(title: &str, prompt: &str, options: &[&str]) -> Result<Option<usize>, String> {
    #[cfg(target_os = "macos")]
    {
        let buttons = options
            .iter()
            .map(|option| format!("\"{}\"", escape_osascript(option)))
            .collect::<Vec<_>>()
            .join(", ");
        let script = format!(
            "display dialog \"{}\" buttons {{{}, \"Cancel\"}} default button 1 with title \"{}\"",
            escape_osascript(prompt),
            buttons,
            escape_osascript(title)
        );
        let output = Command::new("osascript")
            .args(["-e", &script])
            .output()
            .map_err(|error| format!("osascript: {error}"))?;
        if !output.status.success() {
            return Ok(None);
        }
        let text = String::from_utf8_lossy(&output.stdout);
        Ok(options
            .iter()
            .position(|option| text.contains(&format!("button returned:{option}"))))
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let _ = (title, prompt, options);
        Err("provider setup dialogs are implemented on macOS; set env variables".to_owned())
    }
    #[cfg(not(unix))]
    {
        let _ = (title, prompt, options);
        Err("provider setup dialogs are implemented on macOS; set env variables".to_owned())
    }
}

fn prompt_secret(title: &str, prompt: &str) -> Result<Option<String>, String> {
    #[cfg(target_os = "macos")]
    {
        let script = format!(
            "display dialog \"{}\" default answer \"\" with hidden answer buttons {{\"Cancel\", \"OK\"}} default button \"OK\" with title \"{}\"",
            escape_osascript(prompt),
            escape_osascript(title)
        );
        let output = Command::new("osascript")
            .args(["-e", &script])
            .output()
            .map_err(|error| format!("osascript: {error}"))?;
        if !output.status.success() {
            return Ok(None);
        }
        let text = String::from_utf8_lossy(&output.stdout);
        Ok(text
            .split("text returned:")
            .nth(1)
            .map(|value| value.trim().to_owned()))
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (title, prompt);
        Err("provider setup dialogs are implemented on macOS; set env variables".to_owned())
    }
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
    fn selects_update_assets_and_checksums() {
        let release = parse_release(
            r#"{"tag_name":"v1.2.3","html_url":"https://example.test",
                "assets":[
                  {"name":"dshr-1.2.3-macos.zip","browser_download_url":"https://example.test/app.zip"},
                  {"name":"SHA256SUMS","browser_download_url":"https://example.test/sums"}
                ]}"#,
        )
        .expect("release");
        assert_eq!(
            release.macos_zip.as_deref(),
            Some("https://example.test/app.zip")
        );
        assert_eq!(
            release.checksums.as_deref(),
            Some("https://example.test/sums")
        );
        let sums = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa  dshr-1.2.3-macos.zip\nbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb  other.zip\n";
        assert_eq!(
            checksum_for(sums, "dshr-1.2.3-macos.zip").as_deref(),
            Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
        );
        assert!(checksum_for(sums, "missing.zip").is_none());
    }

    #[test]
    fn recovery_window_lists_profiles_and_escapes_errors() {
        let home =
            std::env::temp_dir().join(format!("dshr-recovery-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&home).expect("home");
        let profile = home.join("profiles").join("work");
        std::fs::create_dir_all(&profile).expect("profile");
        std::fs::write(profile.join("config.yaml"), "profile: work\n").expect("config");
        let html = recovery_html(&"<boom>", &home);
        assert!(html.contains("profile:work"), "{html}");
        assert!(html.contains("&lt;boom&gt;"));
        assert!(html.contains("Open config folder"));
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn saves_and_loads_the_update_channel() {
        let home = std::env::temp_dir().join(format!("dshr-channel-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&home).expect("home");
        assert_eq!(load_update_channel(&home), UpdateChannel::Stable);
        save_update_channel(&home, UpdateChannel::Beta).expect("save");
        assert_eq!(load_update_channel(&home), UpdateChannel::Beta);
        let releases = r#"[{"tag_name":"v9.0.0","html_url":"u","assets":[]},
            {"tag_name":"v9.1.0","html_url":"u","assets":[{"name":"x-macos.zip","browser_download_url":"x"}]}]"#;
        let release = parse_beta_releases(releases).expect("release");
        assert_eq!(release.version, "9.1.0");
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn saves_and_loads_the_desktop_folder() {
        let home = std::env::temp_dir().join(format!("dshr-folder-test-{}", uuid::Uuid::new_v4()));
        let workspace = home.join("workspace");
        std::fs::create_dir_all(&workspace).expect("workspace");
        save_desktop_folder(&home, Some(&workspace)).expect("save");
        assert_eq!(
            load_desktop_folder(&home).as_deref(),
            Some(workspace.as_path())
        );
        save_desktop_folder(&home, None).expect("clear");
        assert!(load_desktop_folder(&home).is_none());
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn saves_and_loads_network_settings() {
        let dir = std::env::temp_dir().join(format!("dshr-network-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("dir");
        let settings = NetworkSettings {
            bind: "0.0.0.0".into(),
            port: 4317,
        };
        save_network_settings(&dir, &settings).expect("save");
        let loaded = load_network_settings(&dir);
        assert_eq!(loaded.bind, "0.0.0.0");
        assert_eq!(loaded.port, 4317);
        std::fs::write(
            dir.join("desktop-network.json"),
            r#"{"bind":"bogus","port":0}"#,
        )
        .expect("write");
        assert_eq!(load_network_settings(&dir).bind, "127.0.0.1");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn finds_the_newest_downloaded_app() {
        let home = std::env::temp_dir().join(format!("dshr-updates-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(home.join("updates/1.2.3/app/dshr.app")).expect("app");
        let found = newest_downloaded_app(&home).expect("app");
        assert!(found.ends_with("dshr.app"), "{}", found.display());
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn finds_the_running_app_bundle() {
        let executable = Path::new("/Applications/dshr.app/Contents/MacOS/dshr-desktop");
        assert_eq!(
            current_app_bundle(executable).as_deref(),
            Some(Path::new("/Applications/dshr.app"))
        );
        assert!(current_app_bundle(Path::new("/tmp/dshr")).is_none());
    }

    #[test]
    fn remembers_the_last_known_good_profile() {
        let dir = std::env::temp_dir().join(format!("dshr-lastgood-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("dir");
        save_last_good(&dir, "default");
        assert!(load_last_good(&dir).is_none());
        save_last_good(&dir, "work");
        assert_eq!(load_last_good(&dir).as_deref(), Some("work"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn saves_and_loads_window_settings() {
        let dir = std::env::temp_dir().join(format!("dshr-window-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("dir");
        let settings = WindowSettings {
            mode: "enhanced".into(),
            material: true,
        };
        save_window_settings(&dir, &settings).expect("save");
        let loaded = load_window_settings(&dir);
        assert_eq!(loaded.mode, "enhanced");
        assert!(loaded.material);
        save_window_settings(
            &dir,
            &WindowSettings {
                mode: "extended".into(),
                material: false,
            },
        )
        .expect("save extended");
        assert_eq!(load_window_settings(&dir).mode, "extended");
        std::fs::write(
            dir.join("desktop-window.json"),
            r#"{"mode":"bogus","material":false}"#,
        )
        .expect("write");
        assert_eq!(load_window_settings(&dir).mode, "compatibility");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn saves_and_loads_provider_settings() {
        let dir = std::env::temp_dir().join(format!("dshr-provider-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("dir");
        let settings = ProviderSettings {
            provider: Some("deepseek-account".into()),
            model: Some("deepseek-flash".into()),
            env: BTreeMap::from([("DEEPSEEK_API_KEY".to_owned(), "secret".to_owned())]),
        };
        save_provider_settings(&dir, &settings).expect("save");
        let loaded = load_provider_settings(&dir).expect("load");
        assert_eq!(loaded.provider.as_deref(), Some("deepseek-account"));
        assert_eq!(loaded.model.as_deref(), Some("deepseek-flash"));
        assert_eq!(loaded.env["DEEPSEEK_API_KEY"], "secret");
        let _ = std::fs::remove_dir_all(&dir);
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

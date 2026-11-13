//! Thin Tauri shell over `dshr serve`.
//!
//! The desktop process starts a loopback server with a fresh bearer token,
//! waits for `/v1/health`, and opens the embedded Leptos UI. No native APIs
//! are exposed to the page; the protocol is the same HTTP/SSE contract.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

struct ServerProcess(Mutex<Option<Child>>);

fn main() {
    if std::env::args().any(|argument| argument == "--check") {
        match check_server() {
            Ok(url) => println!("desktop check ok: {url}"),
            Err(error) => {
                eprintln!("desktop check failed: {error}");
                std::process::exit(1);
            }
        }
        return;
    }
    tauri::Builder::default()
        .setup(|app| {
            let port = free_port()?;
            let token = format!(
                "{}{}",
                uuid::Uuid::new_v4().simple(),
                uuid::Uuid::new_v4().simple()
            );
            let binary = find_binary()?;
            let child = Command::new(&binary)
                .args([
                    "serve",
                    "--bind",
                    "127.0.0.1",
                    "--port",
                    &port.to_string(),
                    "--token",
                    &token,
                ])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()?;
            app.manage(ServerProcess(Mutex::new(Some(child))));
            wait_health(port, Duration::from_secs(20))?;

            let url = format!("http://127.0.0.1:{port}/?token={token}");
            WebviewWindowBuilder::new(app, "main", WebviewUrl::External(url.parse()?))
                .title("dshr")
                .inner_size(1280.0, 860.0)
                .min_inner_size(720.0, 480.0)
                .build()?;
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("build dshr desktop")
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event
                && let Some(state) = app.try_state::<ServerProcess>()
                && let Ok(mut slot) = state.0.lock()
                && let Some(mut child) = slot.take()
            {
                let _ = child.kill();
                let _ = child.wait();
            }
        });
}

/// Headless check: starts the sidecar server, waits for health, then stops it.
fn check_server() -> Result<String, Box<dyn std::error::Error>> {
    let port = free_port()?;
    let token = format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    );
    let binary = find_binary()?;
    let mut child = Command::new(&binary)
        .args([
            "serve",
            "--bind",
            "127.0.0.1",
            "--port",
            &port.to_string(),
            "--token",
            &token,
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    let result = wait_health(port, Duration::from_secs(20));
    let _ = child.kill();
    let _ = child.wait();
    result?;
    Ok(format!("http://127.0.0.1:{port}/?token=…"))
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
    fn finds_a_binary_from_the_environment() {
        let dir = std::env::temp_dir().join(format!("dshr-desktop-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("dir");
        let binary = dir.join("dshr");
        std::fs::write(&binary, b"#!/bin/sh\n").expect("write");
        // SAFETY: tests that mutate process env are serialized by cargo's test
        // harness only if they share a lock; this test is standalone.
        unsafe { std::env::set_var("DSHR_BIN", &binary) };
        assert_eq!(find_binary().expect("binary"), binary);
        unsafe { std::env::remove_var("DSHR_BIN") };
        let _ = std::fs::remove_dir_all(&dir);
    }
}

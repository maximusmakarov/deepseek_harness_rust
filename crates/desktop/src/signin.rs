//! "Sign in with DeepSeek": the platform's browser OAuth flow.
//!
//! Contract taken from the official client (`@deepseek-ai/dsh-deepseek-account`
//! 0.2.2-rc.3): PKCE (S256) authorization code against
//! `https://platform.deepseek.com/auth-api/v0/dsh/{auth_init,auth_exchange}`,
//! five `x-client-*` identity headers, a loopback `/oauth/callback` on the host
//! web server, and the account grant returned by the exchange used as
//! `x-dsh-auth-token`.
//!
//! The user approves the login in the system browser; this module only drives
//! the request/exchange and never sees the password.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::time::{Duration, Instant};

use base64::Engine;
use ring::digest;

/// Platform deployment used for the account API and the browser pages.
pub const PLATFORM_ORIGIN: &str = "https://platform.deepseek.com";

/// Resolves the platform origin; `DSHR_DESKTOP_PLATFORM_ORIGIN` overrides it for
/// tests, but only with HTTPS or a loopback HTTP origin (the official client's
/// `allowLoopbackHttp` rule), so a stray value cannot downgrade the connection.
pub fn platform_origin() -> Result<String, String> {
    let value = std::env::var("DSHR_DESKTOP_PLATFORM_ORIGIN")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| PLATFORM_ORIGIN.to_owned());
    let value = value.trim().trim_end_matches('/').to_owned();
    let loopback = value.starts_with("http://127.0.0.1") || value.starts_with("http://localhost");
    if value.starts_with("https://") || loopback {
        Ok(value)
    } else {
        Err(format!(
            "platform origin must be https or loopback http, got {value}"
        ))
    }
}
/// How long the browser approval may take by default.
pub const LOGIN_TIMEOUT: Duration = Duration::from_secs(600);

/// Login timeout, overridable for tests and slow logins (5 s … 1 h).
pub fn login_timeout() -> Duration {
    std::env::var("DSHR_DESKTOP_LOGIN_TIMEOUT_SECS")
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .map(|seconds| Duration::from_secs(seconds.clamp(5, 3600)))
        .unwrap_or(LOGIN_TIMEOUT)
}

/// Successful sign-in: the account grant plus an optional display name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedIn {
    pub token: String,
    pub user: Option<String>,
}

#[derive(Debug)]
struct Pkce {
    verifier: String,
    challenge: String,
}

fn base64url(bytes: &[u8]) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

fn pkce() -> Pkce {
    // 48 random bytes from UUIDv4 values give a valid RFC 7636 verifier.
    let mut seed = Vec::with_capacity(48);
    for _ in 0..3 {
        seed.extend_from_slice(uuid::Uuid::new_v4().as_bytes());
    }
    let verifier = base64url(&seed);
    let challenge = base64url(digest::digest(&digest::SHA256, verifier.as_bytes()).as_ref());
    Pkce {
        verifier,
        challenge,
    }
}

fn state_value() -> String {
    let mut bytes = Vec::with_capacity(24);
    for _ in 0..2 {
        bytes.extend_from_slice(uuid::Uuid::new_v4().as_bytes());
    }
    base64url(&bytes)
}

/// Platform identity headers shared by init/exchange/profile calls.
fn client_headers() -> Vec<(&'static str, String)> {
    let offset = timezone_offset_seconds();
    vec![
        ("x-client-bundle-id", String::new()),
        (
            "x-client-platform",
            if cfg!(target_os = "windows") {
                "desktop-win".to_owned()
            } else {
                "desktop-mac".to_owned()
            },
        ),
        ("x-client-version", env!("CARGO_PKG_VERSION").to_owned()),
        ("x-client-locale", "en_US".to_owned()),
        ("x-client-timezone-offset", offset.to_string()),
    ]
}

/// Current UTC offset in seconds, as the platform expects it.
fn timezone_offset_seconds() -> i64 {
    use chrono::Offset;
    chrono::Local::now().offset().fix().local_minus_utc() as i64
}

fn post_json(url: &str, body: &serde_json::Value) -> Result<serde_json::Value, String> {
    let mut command = std::process::Command::new("curl");
    command.args(["-sS", "--max-time", "30", "-X", "POST", url]);
    command.args(["-H", "content-type: application/json"]);
    for (name, value) in client_headers() {
        command.arg("-H").arg(format!("{name}: {value}"));
    }
    command
        .arg("--data-binary")
        .arg(body.to_string())
        .arg("-w")
        .arg("\n%{http_code}")
        .stdin(std::process::Stdio::null());
    let output = command.output().map_err(|error| format!("curl: {error}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let (body_text, status) = match stdout.rsplit_once('\n') {
        Some((body, status)) => (body.to_owned(), status.trim().parse::<u16>().unwrap_or(0)),
        None => (stdout, 0),
    };
    if !output.status.success() && status == 0 {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("platform request failed: {}", stderr.trim()));
    }
    let parsed: serde_json::Value = serde_json::from_str(&body_text)
        .map_err(|error| format!("platform response (HTTP {status}) is not JSON: {error}"))?;
    if !(200..300).contains(&status) {
        let message = parsed
            .get("msg")
            .and_then(serde_json::Value::as_str)
            .or_else(|| {
                parsed
                    .get("data")
                    .and_then(|data| data.get("biz_msg"))
                    .and_then(serde_json::Value::as_str)
            })
            .unwrap_or("no message");
        return Err(format!("platform HTTP {status}: {message}"));
    }
    Ok(parsed)
}

/// Unwraps `{code, msg, data:{biz_code, biz_msg, biz_data}}` and returns `biz_data`.
fn business_data(envelope: &serde_json::Value) -> Result<serde_json::Value, String> {
    let code = envelope.get("code").and_then(serde_json::Value::as_i64);
    if code != Some(0) {
        let msg = envelope
            .get("msg")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("platform rejected the request");
        return Err(format!("platform error {code:?}: {msg}"));
    }
    let data = envelope
        .get("data")
        .ok_or_else(|| "platform response has no data".to_owned())?;
    let biz_code = data.get("biz_code").and_then(serde_json::Value::as_i64);
    if biz_code != Some(0) {
        let msg = data
            .get("biz_msg")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("platform business error");
        return Err(format!("platform error {biz_code:?}: {msg}"));
    }
    data.get("biz_data")
        .cloned()
        .ok_or_else(|| "platform response has no biz_data".to_owned())
}

/// Runs the whole browser sign-in. `open` receives the authorize URL, the
/// callback URL and the expected state (tests drive the callback from there).

/// Browser sign-in with an explicit timeout and a cancellation predicate (the
/// Settings window's "Cancel" button flips it).
pub fn sign_in_with(
    origin: &str,
    timeout: Duration,
    cancelled: &dyn Fn() -> bool,
    open: impl Fn(&str, &str, &str) -> Result<(), String>,
) -> Result<SignedIn, String> {
    let origin = origin.trim_end_matches('/').to_owned();
    let listener =
        TcpListener::bind("127.0.0.1:0").map_err(|error| format!("callback bind: {error}"))?;
    listener
        .set_nonblocking(true)
        .map_err(|error| format!("callback listener: {error}"))?;
    let port = listener
        .local_addr()
        .map_err(|error| format!("callback address: {error}"))?
        .port();
    let redirect_uri = format!("http://127.0.0.1:{port}/oauth/callback");
    let pkce = pkce();
    let state = state_value();

    let init = post_json(
        &format!("{origin}/auth-api/v0/dsh/auth_init"),
        &serde_json::json!({
            "code_challenge": pkce.challenge,
            "code_challenge_method": "S256",
            "state": state,
            "redirect_uri": redirect_uri,
            "locale": "en_US",
            "login_source": "desktop",
        }),
    )?;
    let init = business_data(&init)?;
    let authorize_url = init
        .get("authorize_url")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "auth_init returned no authorize_url".to_owned())?;
    open(authorize_url, &redirect_uri, &state)?;

    let code = wait_for_callback(&listener, &state, timeout, cancelled)?;
    if cancelled() {
        return Err("sign-in cancelled".to_owned());
    }
    let exchange = post_json(
        &format!("{origin}/auth-api/v0/dsh/auth_exchange"),
        &serde_json::json!({
            "code": code,
            "code_verifier": pkce.verifier,
            "redirect_uri": redirect_uri,
            "device_id": uuid::Uuid::new_v4().to_string(),
            "device_model": format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH),
            "os_version": std::env::consts::OS,
        }),
    )?;
    let exchange = business_data(&exchange)?;
    let token = exchange
        .get("token")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "auth_exchange returned no token".to_owned())?
        .to_owned();
    let user = exchange
        .get("user")
        .and_then(|user| user.get("name").or_else(|| user.get("email")))
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned);
    Ok(SignedIn { token, user })
}

/// Waits for `GET /oauth/callback?code=…&state=…` and answers the browser.
fn wait_for_callback(
    listener: &TcpListener,
    expected_state: &str,
    timeout: Duration,
    cancelled: &dyn Fn() -> bool,
) -> Result<String, String> {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if cancelled() {
            return Err("sign-in cancelled".to_owned());
        }
        match listener.accept() {
            Ok((mut stream, _)) => {
                let _ = stream.set_nonblocking(false);
                let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
                let mut buffer = Vec::new();
                let mut chunk = [0u8; 1024];
                loop {
                    match stream.read(&mut chunk) {
                        Ok(0) => break,
                        Ok(read) => {
                            buffer.extend_from_slice(&chunk[..read]);
                            if buffer.windows(4).any(|window| window == b"\r\n\r\n")
                                || buffer.len() > 16 * 1024
                            {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }
                let request = String::from_utf8_lossy(&buffer);
                let path = request
                    .lines()
                    .next()
                    .and_then(|line| line.split_whitespace().nth(1))
                    .unwrap_or("/");
                match parse_callback(path, expected_state) {
                    Ok(code) => {
                        let body = "<!doctype html><meta charset=\"utf-8\"><title>dshr</title>\
                                    <body style=\"font:14px ui-sans-serif;margin:40px\">\
                                    <h2>Signed in to dshr</h2><p>You can close this window and \
                                    return to the app.</p>";
                        let response = format!(
                            "HTTP/1.1 200 OK\r\ncontent-type: text/html; charset=utf-8\r\n\
                             content-length: {}\r\nconnection: close\r\n\r\n{body}",
                            body.len()
                        );
                        let _ = stream.write_all(response.as_bytes());
                        return Ok(code);
                    }
                    Err(error) => {
                        let body = format!("<!doctype html><meta charset=\"utf-8\"><p>{error}");
                        let response = format!(
                            "HTTP/1.1 400 Bad Request\r\ncontent-type: text/html; charset=utf-8\r\n\
                             content-length: {}\r\nconnection: close\r\n\r\n{body}",
                            body.len()
                        );
                        let _ = stream.write_all(response.as_bytes());
                        // A stray or foreign request must not abort the attempt.
                    }
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(_) => return Err("callback listener failed".to_owned()),
        }
    }
    Err("sign-in timed out waiting for the browser".to_owned())
}

/// Extracts the authorization code after validating path and state.
fn parse_callback(path: &str, expected_state: &str) -> Result<String, String> {
    let query = path
        .split_once('?')
        .map(|(_, query)| query)
        .ok_or_else(|| "callback has no query".to_owned())?;
    if !path.starts_with("/oauth/callback?") {
        return Err("unexpected callback path".to_owned());
    }
    let mut code = None;
    let mut state = None;
    let mut code_count = 0;
    for pair in query.split('&') {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        match key {
            "code" => {
                code_count += 1;
                code = Some(percent_decode(value));
            }
            "state" => state = Some(percent_decode(value)),
            _ => {}
        }
    }
    if code_count != 1 {
        return Err("callback must carry exactly one code".to_owned());
    }
    let state = state.ok_or_else(|| "callback has no state".to_owned())?;
    if state != expected_state {
        return Err("callback state does not match".to_owned());
    }
    code.ok_or_else(|| "callback has no code".to_owned())
}

fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'%' if index + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[index + 1..index + 3]).unwrap_or("");
                match u8::from_str_radix(hex, 16) {
                    Ok(byte) => {
                        out.push(byte);
                        index += 3;
                    }
                    Err(_) => {
                        out.push(b'%');
                        index += 1;
                    }
                }
            }
            b'+' => {
                out.push(b' ');
                index += 1;
            }
            byte => {
                out.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unwraps_the_platform_envelope() {
        let probe = serde_json::json!({
            "code": 0,
            "msg": "",
            "data": {"biz_code": 0, "biz_msg": "", "biz_data": {
                "authorize_url": "https://platform.deepseek.com/dsh/authorize?authorize_id=abc",
                "authorize_id": "abc",
                "expires_in": 600
            }}
        });
        let data = business_data(&probe).expect("ok");
        assert_eq!(
            data["authorize_url"],
            "https://platform.deepseek.com/dsh/authorize?authorize_id=abc"
        );
        let failed = serde_json::json!({"code": 0, "data": {"biz_code": 40003, "biz_msg": "bad"}});
        assert!(business_data(&failed).is_err());
        let rejected = serde_json::json!({"code": 1001, "msg": "nope"});
        assert!(business_data(&rejected).is_err());
    }

    #[test]
    fn parses_the_oauth_callback() {
        let code = parse_callback("/oauth/callback?code=ab%2Fcd&state=s1", "s1").expect("code");
        assert_eq!(code, "ab/cd");
        assert!(parse_callback("/oauth/callback?code=a&state=other", "s1").is_err());
        assert!(parse_callback("/oauth/callback?code=a&code=b&state=s1", "s1").is_err());
        assert!(parse_callback("/oauth/callback?state=s1", "s1").is_err());
        assert!(parse_callback("/other?code=a&state=s1", "s1").is_err());
    }

    /// Raw HTTP GET for the simulated browser hop.
    fn http_get(url: &str) {
        use std::io::Write as _;
        let (host, rest) = url
            .trim_start_matches("http://")
            .split_once('/')
            .expect("url");
        let mut stream = std::net::TcpStream::connect(host).expect("connect");
        let request = format!("GET /{rest} HTTP/1.1\r\nhost: {host}\r\nconnection: close\r\n\r\n");
        stream.write_all(request.as_bytes()).expect("write");
        let mut response = String::new();
        let _ = stream.read_to_string(&mut response);
        assert!(response.starts_with("HTTP/1.1 200"), "callback: {response}");
    }

    #[test]
    fn full_sign_in_round_trip_against_a_stub_platform() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("stub bind");
        listener.set_nonblocking(false).expect("blocking");
        let origin = format!(
            "http://127.0.0.1:{}",
            listener.local_addr().expect("addr").port()
        );
        let bodies = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let recorded = bodies.clone();
        std::thread::spawn(move || {
            for _ in 0..2 {
                let Ok((mut stream, _)) = listener.accept() else {
                    return;
                };
                let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
                let mut buffer = Vec::new();
                let mut chunk = [0u8; 2048];
                loop {
                    match stream.read(&mut chunk) {
                        Ok(0) => break,
                        Ok(read) => {
                            buffer.extend_from_slice(&chunk[..read]);
                            let text = String::from_utf8_lossy(&buffer).to_string();
                            let (head, body) = text.split_once("\r\n\r\n").unwrap_or((&text, ""));
                            let length = head
                                .lines()
                                .find_map(|line| {
                                    line.to_ascii_lowercase()
                                        .strip_prefix("content-length:")
                                        .map(|value| value.trim().parse::<usize>().unwrap_or(0))
                                })
                                .unwrap_or(0);
                            if body.len() >= length {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }
                let text = String::from_utf8_lossy(&buffer).to_string();
                let path = text
                    .lines()
                    .next()
                    .and_then(|line| line.split_whitespace().nth(1))
                    .unwrap_or("/")
                    .to_owned();
                let body = text
                    .split_once("\r\n\r\n")
                    .map(|(_, body)| body.to_owned())
                    .unwrap_or_default();
                recorded.lock().expect("bodies").push(body);
                let payload = if path.contains("auth_init") {
                    serde_json::json!({"code": 0, "msg": "", "data": {"biz_code": 0, "biz_msg": "", "biz_data": {
                        "authorize_url": "https://platform.deepseek.com/dsh/authorize?authorize_id=stub",
                        "authorize_id": "stub", "expires_in": 600}}})
                } else {
                    serde_json::json!({"code": 0, "msg": "", "data": {"biz_code": 0, "biz_msg": "", "biz_data": {
                        "token": "stub-token", "authorized_url": "https://platform.deepseek.com/dsh/authorized",
                        "user": {"name": "Stub User"}}}})
                };
                let body = payload.to_string();
                let response = format!(
                    "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(response.as_bytes());
            }
        });

        let signed = sign_in_with(
            &origin,
            Duration::from_secs(10),
            &|| false,
            |authorize_url, redirect_uri, state| {
                assert!(authorize_url.contains("/dsh/authorize"));
                let url = format!("{redirect_uri}?code=stub-code&state={state}");
                std::thread::spawn(move || http_get(&url));
                Ok(())
            },
        )
        .expect("sign in");
        assert_eq!(signed.token, "stub-token");
        assert_eq!(signed.user.as_deref(), Some("Stub User"));

        let bodies = bodies.lock().expect("bodies");
        let init: serde_json::Value = serde_json::from_str(&bodies[0]).expect("init body");
        assert_eq!(init["code_challenge_method"], "S256");
        assert_eq!(init["login_source"], "desktop");
        assert_eq!(init["locale"], "en_US");
        assert!(
            init["redirect_uri"]
                .as_str()
                .expect("redirect")
                .starts_with("http://127.0.0.1:")
        );
        let exchange: serde_json::Value = serde_json::from_str(&bodies[1]).expect("exchange body");
        assert_eq!(exchange["code"], "stub-code");
        let verifier = exchange["code_verifier"].as_str().expect("verifier");
        let expected = base64url(digest::digest(&digest::SHA256, verifier.as_bytes()).as_ref());
        assert_eq!(init["code_challenge"].as_str(), Some(expected.as_str()));
    }

    #[test]
    fn cancellation_ends_the_flow() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("stub bind");
        listener.set_nonblocking(false).expect("blocking");
        let origin = format!(
            "http://127.0.0.1:{}",
            listener.local_addr().expect("addr").port()
        );
        std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buffer = [0u8; 4096];
                let _ = stream.read(&mut buffer);
                let body = serde_json::json!({"code": 0, "data": {"biz_code": 0, "biz_data": {
                    "authorize_url": "https://platform.deepseek.com/dsh/authorize?authorize_id=stub",
                    "authorize_id": "stub", "expires_in": 600}}})
                .to_string();
                let response = format!(
                    "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(response.as_bytes());
            }
        });
        let error = sign_in_with(&origin, Duration::from_secs(30), &|| true, |_, _, _| Ok(()))
            .expect_err("cancelled");
        assert!(error.contains("cancelled"), "{error}");
    }

    #[test]
    fn pkce_challenge_is_s256_of_the_verifier() {
        let pkce = pkce();
        let expected =
            base64url(digest::digest(&digest::SHA256, pkce.verifier.as_bytes()).as_ref());
        assert_eq!(pkce.challenge, expected);
        assert!(pkce.verifier.len() >= 43);
    }
}

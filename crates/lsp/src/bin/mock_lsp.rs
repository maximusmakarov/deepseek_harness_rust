use std::io::{BufRead, Write};

use serde_json::{Value, json};

fn main() {
    let stdin = std::io::stdin();
    let mut reader = stdin.lock();
    let mut stdout = std::io::stdout();
    while let Some(body) = read_message(&mut reader) {
        let Ok(value) = serde_json::from_slice::<Value>(&body) else {
            continue;
        };
        let method = value["method"].as_str().unwrap_or_default();
        match method {
            "initialize" => {
                send(
                    &mut stdout,
                    &json!({
                        "jsonrpc": "2.0",
                        "id": value["id"],
                        "result": {
                            "capabilities": {
                                "hoverProvider": true,
                                "definitionProvider": true,
                                "referencesProvider": true
                            }
                        }
                    }),
                );
            }
            "initialized" => {}
            "textDocument/didOpen" => {
                let uri = value["params"]["textDocument"]["uri"]
                    .as_str()
                    .unwrap_or("");
                send(
                    &mut stdout,
                    &json!({
                        "jsonrpc": "2.0",
                        "method": "textDocument/publishDiagnostics",
                        "params": {
                            "uri": uri,
                            "diagnostics": [{
                                "range": {"start": {"line": 0, "character": 0}, "end": {"line": 0, "character": 1}},
                                "severity": 1,
                                "message": "mock diagnostic"
                            }]
                        }
                    }),
                );
            }
            "textDocument/hover" => {
                send(
                    &mut stdout,
                    &json!({
                        "jsonrpc": "2.0",
                        "id": value["id"],
                        "result": {"contents": {"kind": "plaintext", "value": "mock hover"}}
                    }),
                );
            }
            "textDocument/definition" | "textDocument/references" => {
                let uri = value["params"]["textDocument"]["uri"]
                    .as_str()
                    .unwrap_or("");
                send(
                    &mut stdout,
                    &json!({
                        "jsonrpc": "2.0",
                        "id": value["id"],
                        "result": [{
                            "uri": uri,
                            "range": {"start": {"line": 1, "character": 2}, "end": {"line": 1, "character": 3}}
                        }]
                    }),
                );
            }
            _ => {
                if value.get("id").is_some() {
                    send(
                        &mut stdout,
                        &json!({
                            "jsonrpc": "2.0",
                            "id": value["id"],
                            "error": {"code": -32601, "message": "method not found"}
                        }),
                    );
                }
            }
        }
    }
}

fn read_message(reader: &mut impl BufRead) -> Option<Vec<u8>> {
    let mut length = None;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).ok()? == 0 {
            return None;
        }
        let trimmed = line.trim_end();
        if trimmed.is_empty() {
            break;
        }
        if let Some(value) = trimmed.strip_prefix("Content-Length:") {
            length = value.trim().parse::<usize>().ok();
        }
    }
    let mut body = vec![0_u8; length?];
    reader.read_exact(&mut body).ok()?;
    Some(body)
}

fn send(stdout: &mut impl Write, value: &Value) {
    let body = serde_json::to_vec(value).expect("json");
    let _ = write!(stdout, "Content-Length: {}\r\n\r\n", body.len());
    let _ = stdout.write_all(&body);
    let _ = stdout.flush();
}

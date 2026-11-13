use std::io::{BufRead, Write};

use serde_json::{Value, json};

fn main() {
    let stdin = std::io::stdin();
    let mut reader = stdin.lock();
    let mut stdout = std::io::stdout();
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).unwrap_or(0) == 0 {
            break;
        }
        if line.trim().is_empty() {
            continue;
        }
        let Ok(value) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        let Some(id) = value.get("id").cloned() else {
            continue;
        };
        let method = value["method"].as_str().unwrap_or_default();
        let response = match method {
            "initialize" => json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "protocolVersion": "2024-11-05",
                    "capabilities": {"tools": {}, "resources": {}, "prompts": {}, "sampling": {}},
                    "serverInfo": {"name": "mock", "version": "0.1.0"},
                    "instructions": "mock server"
                }
            }),
            "tools/list" => json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "tools": [{
                        "name": "echo",
                        "description": "Echo text",
                        "inputSchema": {
                            "type": "object",
                            "required": ["text"],
                            "properties": {"text": {"type": "string"}}
                        }
                    }]
                }
            }),
            "prompts/list" => json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "prompts": [{
                        "name": "mock",
                        "description": "Mock prompt",
                        "arguments": [{"name": "topic", "required": true}]
                    }]
                }
            }),
            "prompts/get" => json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "description": "Mock prompt",
                    "messages": [{"role": "user", "content": {"type": "text", "text": "prompt body"}}]
                }
            }),
            "resources/list" => json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "resources": [{
                        "uri": "mock://resource",
                        "name": "mock",
                        "description": "Mock resource"
                    }]
                }
            }),
            "resources/read" => json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "contents": [{"uri": "mock://resource", "text": "resource body"}]
                }
            }),
            "tools/call" => {
                let text = value["params"]["arguments"]["text"].as_str().unwrap_or("");
                let output = if text == "sample" {
                    send(
                        &mut stdout,
                        &json!({
                            "jsonrpc": "2.0",
                            "id": 900,
                            "method": "sampling/createMessage",
                            "params": {
                                "messages": [{"role": "user", "content": {"type": "text", "text": "sample this"}}],
                                "maxTokens": 32
                            }
                        }),
                    );
                    let mut nested = String::new();
                    let response = if reader.read_line(&mut nested).unwrap_or(0) > 0 {
                        serde_json::from_str::<Value>(nested.trim()).ok()
                    } else {
                        None
                    };
                    let sampled = response
                        .as_ref()
                        .and_then(|value| value["result"]["content"]["text"].as_str())
                        .unwrap_or("no-response");
                    format!("sampled: {sampled}")
                } else {
                    format!("echo: {text}")
                };
                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "content": [{"type": "text", "text": output}],
                        "isError": false
                    }
                })
            }
            _ => json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {"code": -32601, "message": "method not found"}
            }),
        };
        send(&mut stdout, &response);
    }
}

fn send(stdout: &mut impl Write, value: &Value) {
    let _ = writeln!(stdout, "{value}");
    let _ = stdout.flush();
}

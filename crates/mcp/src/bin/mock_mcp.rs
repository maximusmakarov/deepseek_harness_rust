use std::io::{BufRead, Write};

use serde_json::{Value, json};

fn main() {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
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
                    "capabilities": {"tools": {}},
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
                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "content": [{"type": "text", "text": format!("echo: {text}")}],
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
        let _ = writeln!(stdout, "{response}");
        let _ = stdout.flush();
    }
}

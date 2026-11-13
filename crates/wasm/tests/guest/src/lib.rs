#[allow(warnings)]
mod bindings;

use bindings::Guest;
use serde_json::json;

struct Component;

impl Guest for Component {
    fn list_tools() -> String {
        json!({
            "tools": [{
                "name": "echo",
                "description": "Echo from a WIT component",
                "inputSchema": {
                    "type": "object",
                    "properties": {"text": {"type": "string"}}
                }
            }]
        })
        .to_string()
    }

    fn call_tool(name: String, arguments: String) -> String {
        let parsed: serde_json::Value =
            serde_json::from_str(&arguments).unwrap_or(serde_json::Value::Null);
        json!({
            "content": format!("wit {name}: {}", parsed["text"].as_str().unwrap_or_default()),
            "isError": false
        })
        .to_string()
    }
}

bindings::export!(Component with_types_in bindings);

use std::sync::Arc;

use async_trait::async_trait;
use dshr_tools::{BashTool, Tool, ToolCtx, ToolError, ToolOutcome};
use serde_json::Value;

use crate::AcpTerminalClient;

pub struct AcpBashTool {
    terminal: Arc<AcpTerminalClient>,
    fallback: BashTool,
}

impl AcpBashTool {
    #[must_use]
    pub fn new(terminal: Arc<AcpTerminalClient>) -> Self {
        Self {
            terminal,
            fallback: BashTool::default(),
        }
    }
}

#[async_trait]
impl Tool for AcpBashTool {
    fn name(&self) -> &str {
        "bash"
    }

    fn description(&self) -> &str {
        self.fallback.description()
    }

    fn schema(&self) -> Value {
        self.fallback.schema()
    }

    async fn execute(&self, ctx: &ToolCtx, args: Value) -> Result<ToolOutcome, ToolError> {
        let command = args["command"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs("command is required".into()))?;
        if let Some((output, code)) = self
            .terminal
            .run(ctx.session_id.as_str(), command, &ctx.cwd.to_string_lossy())
            .await
            .map_err(ToolError::Failed)?
        {
            return Ok(ToolOutcome {
                content: if output.is_empty() {
                    "(no output)".to_owned()
                } else {
                    output
                },
                is_error: code != 0,
                files: Vec::new(),
            });
        }
        self.fallback.execute(ctx, args).await
    }
}

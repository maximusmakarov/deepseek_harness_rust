use std::sync::Arc;

use async_trait::async_trait;
use dshr_tools::{ReadTool, Tool, ToolCtx, ToolError, ToolOutcome, WriteTool, resolve_path};
use serde_json::Value;

use crate::AcpFsClient;

pub struct AcpReadTool {
    fs: Arc<AcpFsClient>,
}

impl AcpReadTool {
    #[must_use]
    pub fn new(fs: Arc<AcpFsClient>) -> Self {
        Self { fs }
    }
}

#[async_trait]
impl Tool for AcpReadTool {
    fn name(&self) -> &str {
        "read"
    }

    fn description(&self) -> &str {
        ReadTool.description()
    }

    fn schema(&self) -> Value {
        ReadTool.schema()
    }

    async fn execute(&self, ctx: &ToolCtx, args: Value) -> Result<ToolOutcome, ToolError> {
        let path = args["path"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs("path is required".into()))?;
        let absolute = resolve_path(&ctx.cwd, path);
        if args["offset"].is_null()
            && args["limit"].is_null()
            && let Some(text) = self
                .fs
                .read_file(&absolute.to_string_lossy())
                .await
                .map_err(ToolError::Failed)?
        {
            return Ok(ToolOutcome::text(text));
        }
        ReadTool.execute(ctx, args).await
    }
}

pub struct AcpWriteTool {
    fs: Arc<AcpFsClient>,
}

impl AcpWriteTool {
    #[must_use]
    pub fn new(fs: Arc<AcpFsClient>) -> Self {
        Self { fs }
    }
}

#[async_trait]
impl Tool for AcpWriteTool {
    fn name(&self) -> &str {
        "write"
    }

    fn description(&self) -> &str {
        WriteTool.description()
    }

    fn schema(&self) -> Value {
        WriteTool.schema()
    }

    async fn execute(&self, ctx: &ToolCtx, args: Value) -> Result<ToolOutcome, ToolError> {
        let path = args["path"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs("path is required".into()))?;
        let content = args["content"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs("content is required".into()))?;
        let absolute = resolve_path(&ctx.cwd, path);
        if self
            .fs
            .write_file(&absolute.to_string_lossy(), content)
            .await
            .map_err(ToolError::Failed)?
        {
            return Ok(ToolOutcome::text(format!(
                "wrote {} bytes via ACP client",
                content.len()
            )));
        }
        WriteTool.execute(ctx, args).await
    }
}

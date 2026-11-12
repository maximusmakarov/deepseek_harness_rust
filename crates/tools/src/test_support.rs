use std::path::Path;

use dshr_core::SessionId;
use tokio_util::sync::CancellationToken;

use crate::registry::ToolCtx;

pub fn ctx(cwd: &Path) -> ToolCtx {
    ToolCtx {
        cwd: cwd.to_path_buf(),
        session_id: SessionId::new(),
        output_limit: 30_000,
        spill_dir: cwd.join(".spills"),
        cancel: CancellationToken::new(),
    }
}

use dshr_config::Config;
use dshr_llm::LlmProvider;
use dshr_mcp::{McpServerSpec, connect_server_with_sampling};
use dshr_tools::ToolRegistry;

/// Connects configured MCP servers and registers their namespaced tools.
///
/// A failing server is logged and skipped: partial availability is better than
/// refusing to start the harness.
pub async fn register(
    registry: &mut ToolRegistry,
    config: &Config,
    sampling: Option<std::sync::Arc<dyn LlmProvider>>,
) {
    if crate::runtime_flags::safe_mode() {
        tracing::warn!("safe mode: skipping MCP plugins");
        return;
    }
    for server in config.mcp.servers.iter().filter(|server| server.enabled) {
        let spec = McpServerSpec {
            name: server.name.clone(),
            command: server.command.clone(),
            args: server.args.clone(),
            env: server.env.clone(),
            cwd: server.cwd.clone(),
            url: server.url.clone(),
            headers: server.headers.clone(),
            oauth: server.oauth.as_ref().map(|oauth| dshr_mcp::McpOAuthSpec {
                token_url: oauth.token_url.clone(),
                client_id: oauth.client_id.clone(),
                client_secret: oauth.client_secret.clone(),
                scope: oauth.scope.clone(),
            }),
        };
        match connect_server_with_sampling(spec, sampling.clone()).await {
            Ok((_client, tools)) => {
                let count = tools.len();
                for tool in tools {
                    registry.register_arc(tool);
                }
                tracing::info!(server = %server.name, count, "MCP server connected");
            }
            Err(error) => {
                tracing::warn!(
                    server = %server.name,
                    error = %error,
                    "MCP server unavailable"
                );
            }
        }
    }
}

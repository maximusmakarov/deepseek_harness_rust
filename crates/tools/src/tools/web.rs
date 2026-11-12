use std::net::IpAddr;
use std::time::Duration;

use async_trait::async_trait;
use serde_json::{Value, json};
use url::Url;

use crate::registry::{Tool, ToolCtx, ToolError, ToolOutcome};

const MAX_BYTES: usize = 5 * 1024 * 1024;
const MAX_REDIRECTS: usize = 3;

pub struct WebFetchTool {
    allow_private: bool,
    client: reqwest::Client,
}

impl Default for WebFetchTool {
    fn default() -> Self {
        Self::new(false)
    }
}

impl WebFetchTool {
    #[must_use]
    pub fn new(allow_private: bool) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .redirect(reqwest::redirect::Policy::none())
            .user_agent("dshr/0.1")
            .build()
            .expect("http client");
        Self {
            allow_private,
            client,
        }
    }
}

#[async_trait]
impl Tool for WebFetchTool {
    fn name(&self) -> &str {
        "web_fetch"
    }

    fn description(&self) -> &str {
        "Fetch an HTTP(S) URL and return readable text. Blocks private addresses."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["url"],
            "additionalProperties": false,
            "properties": {"url": {"type": "string", "minLength": 1}}
        })
    }

    async fn execute(&self, _ctx: &ToolCtx, args: Value) -> Result<ToolOutcome, ToolError> {
        let raw = args["url"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs("url is required".into()))?;
        let mut url = Url::parse(raw)
            .map_err(|error| ToolError::InvalidArgs(format!("invalid URL: {error}")))?;

        for _ in 0..=MAX_REDIRECTS {
            self.check_url(&url).await?;
            let response = self
                .client
                .get(url.clone())
                .send()
                .await
                .map_err(|error| ToolError::Failed(format!("fetch: {error}")))?;

            if response.status().is_redirection() {
                let Some(location) = response.headers().get(reqwest::header::LOCATION) else {
                    return Err(ToolError::Failed("redirect without Location".into()));
                };
                let location = location
                    .to_str()
                    .map_err(|_| ToolError::Failed("invalid Location header".into()))?;
                url = url
                    .join(location)
                    .map_err(|error| ToolError::Failed(format!("invalid redirect: {error}")))?;
                continue;
            }

            let status = response.status();
            if !status.is_success() {
                return Err(ToolError::Failed(format!("HTTP {status} for {url}")));
            }
            if response
                .content_length()
                .is_some_and(|len| len > MAX_BYTES as u64)
            {
                return Err(ToolError::Failed("response is larger than 5 MiB".into()));
            }
            let content_type = response
                .headers()
                .get(reqwest::header::CONTENT_TYPE)
                .and_then(|value| value.to_str().ok())
                .unwrap_or("")
                .to_owned();
            let bytes = response
                .bytes()
                .await
                .map_err(|error| ToolError::Failed(format!("read body: {error}")))?;
            if bytes.len() > MAX_BYTES {
                return Err(ToolError::Failed("response is larger than 5 MiB".into()));
            }

            let text = if content_type.contains("html") {
                html2text::from_read(&bytes[..], 100)
                    .unwrap_or_else(|error| format!("[html parse error: {error}]"))
            } else {
                String::from_utf8_lossy(&bytes).into_owned()
            };
            return Ok(ToolOutcome::text(format!(
                "url: {url}\ncontent-type: {content_type}\n\n{}",
                text.trim()
            )));
        }

        Err(ToolError::Failed("too many redirects".into()))
    }
}

impl WebFetchTool {
    async fn check_url(&self, url: &Url) -> Result<(), ToolError> {
        if !matches!(url.scheme(), "http" | "https") {
            return Err(ToolError::InvalidArgs(
                "only http and https are supported".into(),
            ));
        }
        let host = url
            .host_str()
            .ok_or_else(|| ToolError::InvalidArgs("URL has no host".into()))?;
        if host.eq_ignore_ascii_case("localhost") && !self.allow_private {
            return Err(ToolError::Failed("private addresses are blocked".into()));
        }
        if let Ok(ip) = host.parse::<IpAddr>() {
            if is_private(ip) && !self.allow_private {
                return Err(ToolError::Failed("private addresses are blocked".into()));
            }
            return Ok(());
        }
        let port = url.port_or_known_default().unwrap_or(80);
        let addresses = tokio::net::lookup_host((host, port))
            .await
            .map_err(|error| ToolError::Failed(format!("resolve {host}: {error}")))?;
        for address in addresses {
            if is_private(address.ip()) && !self.allow_private {
                return Err(ToolError::Failed("private addresses are blocked".into()));
            }
        }
        Ok(())
    }
}

fn is_private(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            ip.is_private()
                || ip.is_loopback()
                || ip.is_link_local()
                || ip.is_unspecified()
                || ip.is_broadcast()
                || ip.is_multicast()
        }
        IpAddr::V6(ip) => {
            ip.is_loopback()
                || ip.is_unspecified()
                || ip.is_multicast()
                || ip.is_unique_local()
                || ip.is_unicast_link_local()
        }
    }
}

pub struct WebSearchTool {
    api_key: Option<String>,
    base_url: String,
}

impl Default for WebSearchTool {
    fn default() -> Self {
        Self::from_env()
    }
}

impl WebSearchTool {
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            api_key: std::env::var("BRAVE_SEARCH_API_KEY")
                .ok()
                .filter(|value| !value.trim().is_empty()),
            base_url: std::env::var("BRAVE_SEARCH_BASE")
                .unwrap_or_else(|_| "https://api.search.brave.com/res/v1/web/search".to_owned()),
        }
    }

    #[must_use]
    pub fn new(api_key: Option<String>, base_url: impl Into<String>) -> Self {
        Self {
            api_key,
            base_url: base_url.into(),
        }
    }
}

#[async_trait]
impl Tool for WebSearchTool {
    fn name(&self) -> &str {
        "web_search"
    }

    fn description(&self) -> &str {
        "Search the web through the Brave Search API."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["query"],
            "additionalProperties": false,
            "properties": {
                "query": {"type": "string", "minLength": 1},
                "count": {"type": "integer", "minimum": 1, "maximum": 20}
            }
        })
    }

    async fn execute(&self, _ctx: &ToolCtx, args: Value) -> Result<ToolOutcome, ToolError> {
        let query = args["query"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs("query is required".into()))?;
        let count = args["count"].as_u64().unwrap_or(5).clamp(1, 20);
        let Some(key) = &self.api_key else {
            return Err(ToolError::Failed("BRAVE_SEARCH_API_KEY is not set".into()));
        };

        let response = reqwest::Client::new()
            .get(&self.base_url)
            .query(&[("q", query), ("count", &count.to_string())])
            .header("X-Subscription-Token", key)
            .header("Accept", "application/json")
            .timeout(Duration::from_secs(20))
            .send()
            .await
            .map_err(|error| ToolError::Failed(format!("search: {error}")))?;
        let status = response.status();
        if !status.is_success() {
            return Err(ToolError::Failed(format!("search HTTP {status}")));
        }
        let value: Value = response
            .json()
            .await
            .map_err(|error| ToolError::Failed(format!("search JSON: {error}")))?;

        let mut lines = Vec::new();
        if let Some(results) = value["web"]["results"].as_array() {
            for item in results {
                let title = item["title"].as_str().unwrap_or("");
                let url = item["url"].as_str().unwrap_or("");
                let description = item["description"].as_str().unwrap_or("");
                lines.push(format!("- {title}\n  {url}\n  {description}"));
            }
        }
        if lines.is_empty() {
            Ok(ToolOutcome::text("(no results)"))
        } else {
            Ok(ToolOutcome::text(lines.join("\n")))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::ctx;
    use dshr_llm_mock::MockServer;

    #[test]
    fn flags_private_addresses() {
        assert!(is_private("127.0.0.1".parse().expect("ip")));
        assert!(is_private("10.0.0.1".parse().expect("ip")));
        assert!(is_private("192.168.1.1".parse().expect("ip")));
        assert!(is_private("::1".parse().expect("ip")));
        assert!(!is_private("8.8.8.8".parse().expect("ip")));
    }

    #[tokio::test]
    async fn fetch_rejects_localhost() {
        let dir = tempfile::tempdir().expect("tempdir");
        let error = WebFetchTool::new(false)
            .execute(&ctx(dir.path()), json!({"url": "http://127.0.0.1:9/"}))
            .await
            .expect_err("private");
        assert!(error.to_string().contains("private"));
    }

    #[tokio::test]
    async fn fetch_converts_html() {
        let server =
            MockServer::start("<html><body><h1>Title</h1><p>Hello world</p></body></html>")
                .await
                .expect("server");
        let dir = tempfile::tempdir().expect("tempdir");
        let outcome = WebFetchTool::new(true)
            .execute(&ctx(dir.path()), json!({"url": server.base_url()}))
            .await
            .expect("fetch");
        assert!(
            outcome.content.contains("Hello world"),
            "{}",
            outcome.content
        );
        server.abort();
    }

    #[tokio::test]
    async fn search_parses_results() {
        let body = json!({
            "web": {"results": [
                {"title": "Rust", "url": "https://rust-lang.org", "description": "Language"}
            ]}
        })
        .to_string();
        let server = MockServer::start(body).await.expect("server");
        let tool = WebSearchTool::new(Some("test".into()), server.base_url());
        let dir = tempfile::tempdir().expect("tempdir");
        let outcome = tool
            .execute(&ctx(dir.path()), json!({"query": "rust"}))
            .await
            .expect("search");
        assert!(
            outcome.content.contains("rust-lang.org"),
            "{}",
            outcome.content
        );
        server.abort();
    }

    #[tokio::test]
    async fn search_without_key_fails() {
        let tool = WebSearchTool::new(None, "http://127.0.0.1:9/");
        let dir = tempfile::tempdir().expect("tempdir");
        let error = tool
            .execute(&ctx(dir.path()), json!({"query": "rust"}))
            .await
            .expect_err("key");
        assert!(error.to_string().contains("BRAVE_SEARCH_API_KEY"));
    }
}

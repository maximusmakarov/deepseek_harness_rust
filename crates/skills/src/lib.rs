#![forbid(unsafe_code)]
//! Skill discovery and lazy loading.
//!
//! A skill is a directory with `SKILL.md`. Optional front matter carries
//! `name` and `description`; the body is loaded only when the model calls the
//! `skill` tool.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use dshr_tools::{Tool, ToolCtx, ToolError, ToolOutcome};
use serde_json::{Value, json};

#[derive(Debug, Clone)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub body: String,
    pub path: PathBuf,
}

#[derive(Debug, Clone, Default)]
pub struct SkillRegistry {
    skills: BTreeMap<String, Skill>,
}

impl SkillRegistry {
    /// Discovers `<dir>/<name>/SKILL.md` under each directory, first match wins.
    #[must_use]
    pub fn discover(dirs: &[PathBuf]) -> Self {
        let mut skills = BTreeMap::new();
        for base in dirs {
            let Ok(entries) = std::fs::read_dir(base) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }
                let file = path.join("SKILL.md");
                let Ok(text) = std::fs::read_to_string(&file) else {
                    continue;
                };
                let fallback = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("skill")
                    .to_owned();
                let (name, description, body) = parse_skill(&text, &fallback);
                skills.entry(name.clone()).or_insert(Skill {
                    name,
                    description,
                    body,
                    path: file,
                });
            }
        }
        Self { skills }
    }

    #[must_use]
    pub fn catalog(&self) -> Vec<(String, String)> {
        self.skills
            .values()
            .map(|skill| (skill.name.clone(), skill.description.clone()))
            .collect()
    }

    #[must_use]
    pub fn names(&self) -> Vec<String> {
        self.skills.keys().cloned().collect()
    }

    #[must_use]
    pub fn get(&self, name: &str) -> Option<&Skill> {
        self.skills.get(name)
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.skills.is_empty()
    }

    #[must_use]
    pub fn tool(self: &Arc<Self>) -> SkillTool {
        SkillTool {
            registry: Arc::clone(self),
        }
    }
}

fn parse_skill(text: &str, fallback: &str) -> (String, String, String) {
    let mut name = fallback.to_owned();
    let mut description = String::new();
    let mut body = text.trim().to_owned();

    if let Some(rest) = text.strip_prefix("---\n")
        && let Some(end) = rest.find("\n---")
    {
        let front = &rest[..end];
        body = rest[end + 4..].trim().to_owned();
        for line in front.lines() {
            let Some((key, value)) = line.split_once(':') else {
                continue;
            };
            let value = value.trim().trim_matches('"').trim_matches('\'');
            match key.trim() {
                "name" if !value.is_empty() => name = value.to_owned(),
                "description" if !value.is_empty() => description = value.to_owned(),
                _ => {}
            }
        }
    }

    if description.is_empty() {
        description = body
            .lines()
            .find(|line| !line.trim().is_empty() && !line.starts_with('#'))
            .unwrap_or("")
            .trim()
            .chars()
            .take(160)
            .collect();
    }

    (name, description, body)
}

pub struct SkillTool {
    registry: Arc<SkillRegistry>,
}

#[async_trait]
impl Tool for SkillTool {
    fn name(&self) -> &'static str {
        "skill"
    }

    fn description(&self) -> &'static str {
        "Load the full SKILL.md body for a discovered skill."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["name"],
            "additionalProperties": false,
            "properties": {
                "name": {"type": "string", "minLength": 1}
            }
        })
    }

    async fn execute(&self, _ctx: &ToolCtx, args: Value) -> Result<ToolOutcome, ToolError> {
        let name = args["name"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs("name is required".into()))?;
        let skill = self
            .registry
            .get(name)
            .ok_or_else(|| ToolError::InvalidArgs(format!("unknown skill '{name}'")))?;
        Ok(ToolOutcome {
            content: format!("# Skill: {}\n\n{}", skill.name, skill.body),
            is_error: false,
            files: vec![skill.path.display().to_string()],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dshr_core::SessionId;
    use std::path::Path;
    use tokio_util::sync::CancellationToken;

    fn ctx() -> ToolCtx {
        ToolCtx {
            cwd: PathBuf::from("."),
            session_id: SessionId::new(),
            output_limit: 10_000,
            spill_dir: PathBuf::from(".spills"),
            cancel: CancellationToken::new(),
            prompt: None,
        }
    }

    fn write_skill(base: &Path, name: &str, text: &str) {
        let dir = base.join(name);
        std::fs::create_dir_all(&dir).expect("mkdir");
        std::fs::write(dir.join("SKILL.md"), text).expect("write");
    }

    #[test]
    fn discovers_frontmatter_and_body() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_skill(
            dir.path(),
            "rust",
            "---\nname: rust\ndescription: Rust guidelines\n---\nUse cargo fmt.\n",
        );
        let registry = SkillRegistry::discover(&[dir.path().to_path_buf()]);
        assert_eq!(
            registry.catalog(),
            vec![("rust".into(), "Rust guidelines".into())]
        );
        assert_eq!(registry.get("rust").expect("skill").body, "Use cargo fmt.");
    }

    #[test]
    fn falls_back_to_directory_and_first_line() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_skill(dir.path(), "docs", "Write docs first.\n");
        let registry = SkillRegistry::discover(&[dir.path().to_path_buf()]);
        let skill = registry.get("docs").expect("skill");
        assert_eq!(skill.description, "Write docs first.");
    }

    #[tokio::test]
    async fn skill_tool_returns_body() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_skill(
            dir.path(),
            "rust",
            "---\nname: rust\ndescription: d\n---\nbody",
        );
        let registry = Arc::new(SkillRegistry::discover(&[dir.path().to_path_buf()]));
        let outcome = registry
            .tool()
            .execute(&ctx(), json!({"name": "rust"}))
            .await
            .expect("load");
        assert!(outcome.content.contains("body"));
    }

    #[tokio::test]
    async fn unknown_skill_is_rejected() {
        let registry = Arc::new(SkillRegistry::default());
        let error = registry
            .tool()
            .execute(&ctx(), json!({"name": "missing"}))
            .await
            .expect_err("missing");
        assert!(matches!(error, ToolError::InvalidArgs(_)));
    }
}

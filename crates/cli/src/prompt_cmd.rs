use std::sync::Arc;

use clap::Args;
use dshr_config::ConfigLoader;
use dshr_core::{DshrError, Result};
use dshr_skills::SkillRegistry;
use dshr_tools::ToolRegistry;

#[derive(Debug, Args)]
pub struct PromptArgs {
    /// Emit the prompt with a trailing newline only
    #[arg(long)]
    pub quiet: bool,
}

pub async fn execute(loader: &ConfigLoader, _args: &PromptArgs) -> Result<u8> {
    let config = loader.load()?.config;
    let cwd = std::env::current_dir().map_err(DshrError::Io)?;
    let cwd_string = cwd.display().to_string();

    let mut tools = ToolRegistry::with_builtin();
    let skills = std::sync::Arc::new(SkillRegistry::discover(&[
        loader.home().join("skills"),
        cwd.join(".dshr/skills"),
    ]));
    if !skills.is_empty() {
        tools.register_arc(std::sync::Arc::new(skills.tool()));
    }
    crate::mcp_setup::register(&mut tools, &config, None).await;
    dshr_office::register_tools(&mut tools);
    dshr_browser::register_tools(&mut tools, Arc::new(dshr_browser::BrowserManager::new()));
    crate::wasm_setup::register(
        &mut tools,
        loader.home(),
        std::path::Path::new(&cwd),
        &config.plugins,
    )?;
    crate::cordis_setup::register(&tools, &config, loader.home()).await;

    let tools = tools.filtered(&config.tools.allowed, &config.tools.denied);
    let prompt = crate::run_cmd::build_system_prompt(
        loader.home(),
        &cwd_string,
        &config.model.provider,
        &config.model.name,
        &tools,
        &skills,
    );
    print!("{prompt}");
    if !prompt.ends_with('\n') {
        println!();
    }
    Ok(0)
}

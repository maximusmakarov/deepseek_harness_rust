use std::env;

use clap::Args;
use dshr_config::ConfigLoader;
use dshr_core::Result;
use dshr_i18n::{Lang, current};
use serde::Serialize;

#[derive(Debug, Args)]
pub struct DoctorArgs {
    /// Emit machine-readable JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Serialize)]
struct Credentials {
    dsh_deepseek_token: bool,
    deepseek_api_key: bool,
    routerai_api_key: bool,
}

#[derive(Debug, Serialize)]
struct Report {
    version: &'static str,
    os: &'static str,
    arch: &'static str,
    home: String,
    home_exists: bool,
    config_path: String,
    config_found: bool,
    profile: String,
    profile_path: String,
    profile_found: bool,
    provider: String,
    model: String,
    credentials: Credentials,
    sandbox: &'static str,
    status: &'static str,
    error: Option<String>,
}

pub fn run(loader: &ConfigLoader, args: &DoctorArgs) -> Result<u8> {
    let loaded = loader.load();
    let config = loaded.as_ref().ok().map(|loaded| &loaded.config);
    let error = loaded.as_ref().err().map(std::string::ToString::to_string);

    let report = Report {
        version: env!("CARGO_PKG_VERSION"),
        os: env::consts::OS,
        arch: env::consts::ARCH,
        home: loader.home().display().to_string(),
        home_exists: loader.home().exists(),
        config_path: loader.user_path().display().to_string(),
        config_found: loader.user_path().exists(),
        profile: loader.profile().to_owned(),
        profile_path: loader.profile_path().display().to_string(),
        profile_found: loader.profile_path().exists(),
        provider: config.map_or_else(|| "unknown".to_owned(), |c| c.model.provider.clone()),
        model: config.map_or_else(|| "unknown".to_owned(), |c| c.model.name.clone()),
        credentials: Credentials {
            dsh_deepseek_token: env_present("DSH_DEEPSEEK_TOKEN"),
            deepseek_api_key: env_present("DEEPSEEK_API_KEY"),
            routerai_api_key: env_present("ROUTERAI_API_KEY"),
        },
        sandbox: "n/a (M5)",
        status: if error.is_some() { "error" } else { "ok" },
        error,
    };

    if args.json {
        let json = serde_json::to_string_pretty(&report).map_err(|err| {
            dshr_core::DshrError::config(format!("serialize doctor report: {err}"))
        })?;
        println!("{json}");
    } else {
        print_human(&report);
    }

    Ok(u8::from(report.status == "error"))
}

fn print_human(report: &Report) {
    let lang = current();
    println!("dshr doctor");
    println!("  {:<12} {}", lang.doctor_version(), report.version);
    println!(
        "  {:<12} {} {}",
        lang.doctor_platform(),
        report.os,
        report.arch
    );
    println!(
        "  {:<12} {} ({})",
        lang.doctor_home(),
        report.home,
        if report.home_exists {
            lang.present(true)
        } else {
            lang.missing()
        }
    );
    println!(
        "  {:<12} {} ({})",
        lang.doctor_config(),
        report.config_path,
        file_status(lang, report.config_found)
    );
    println!(
        "  {:<12} {}: {} ({})",
        lang.doctor_profile(),
        report.profile,
        report.profile_path,
        file_status(lang, report.profile_found)
    );
    println!(
        "  {:<12} {}/{}",
        lang.doctor_provider(),
        report.provider,
        report.model
    );
    println!(
        "  {:<12} DSH_DEEPSEEK_TOKEN={}, DEEPSEEK_API_KEY={}, ROUTERAI_API_KEY={}",
        lang.doctor_credentials(),
        lang.present(report.credentials.dsh_deepseek_token),
        lang.present(report.credentials.deepseek_api_key),
        lang.present(report.credentials.routerai_api_key),
    );
    println!("  {:<12} {}", lang.doctor_sandbox(), report.sandbox);
    if let Some(error) = &report.error {
        println!("  {:<12} {error}", lang.error_label());
    }
    println!("{}: {}", lang.doctor_status(), report.status);
}

fn file_status(lang: Lang, exists: bool) -> &'static str {
    if exists { lang.found() } else { lang.missing() }
}

fn env_present(key: &str) -> bool {
    env::var_os(key).is_some_and(|value| !value.is_empty())
}

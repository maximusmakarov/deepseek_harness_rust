#![cfg(unix)]

use std::time::Duration;

use expectrl::{Expect, spawn};

fn write_cassette(dir: &std::path::Path) -> std::path::PathBuf {
    let path = dir.join("cassette.json");
    std::fs::write(
        &path,
        r#"{"model":"deepseek-flash","calls":[[
            {"type":"start","model":"deepseek-flash"},
            {"type":"text_delta","text":"pong"},
            {"type":"done","stop_reason":"end_turn"}]]}"#,
    )
    .expect("write cassette");
    path
}

#[test]
fn tui_accepts_a_prompt_and_streams_the_reply() {
    let dir = tempfile::tempdir().expect("tempdir");
    let cassette = write_cassette(dir.path());
    let binary = env!("CARGO_BIN_EXE_dshr");

    let mut session = spawn(format!(
        "{binary} --home {} tui --cassette {}",
        dir.path().display(),
        cassette.display()
    ))
    .expect("spawn tui");
    session.set_expect_timeout(Some(Duration::from_secs(15)));

    session.expect("dshr").expect("header");
    session.send("hello").expect("type prompt");
    session.send("\r").expect("submit");
    session.expect("pong").expect("reply");
}

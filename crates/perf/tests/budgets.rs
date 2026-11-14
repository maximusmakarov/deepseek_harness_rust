//! Performance budget gate: ARCHITECTURE §20.
//!
//! Run with `cargo test -p dshr-perf --release -- --nocapture`. The gate fails
//! when a p95 measurement exceeds its budget by more than the 20 % regression
//! allowance; `DSHR_PERF_STRICT=1` compares against the budget exactly.

use std::time::{Duration, Instant};

use dshr_core::TurnId;
use dshr_llm::{ContentBlock, Usage};
use dshr_perf::{percentile, rss_kb, usage, within_budget};
use dshr_prompt::PromptContext;
use dshr_search::SessionSearchIndex;
use dshr_session::SessionEvent;
use dshr_session_jsonl::SessionStore;

const PROMPT_BUDGET: Duration = Duration::from_millis(10);
const APPEND_BUDGET: Duration = Duration::from_millis(5);
const SEARCH_BUDGET: Duration = Duration::from_millis(100);
const TURN_BUDGET: Duration = Duration::from_millis(50);
const MEMORY_BUDGET_MB: u64 = 200;

/// 100 tool names: the prompt section is the largest deterministic part of a
/// turn, and 100 tools is the documented budget scenario.
fn hundred_tools() -> Vec<String> {
    (0..100).map(|index| format!("tool_{index:03}")).collect()
}

fn prompt_context(
    cwd: &std::path::Path,
    home: &std::path::Path,
    tools: Vec<String>,
) -> PromptContext {
    PromptContext {
        cwd: cwd.to_path_buf(),
        home: home.to_path_buf(),
        provider: "deepseek-account".to_owned(),
        model: "deepseek-flash".to_owned(),
        date: "2026-11-14".to_owned(),
        tools,
        skills: vec![
            ("dsh-doc".to_owned(), "DeepSeek Harness docs".to_owned()),
            ("office".to_owned(), "Office documents".to_owned()),
        ],
    }
}

fn text_event(seq: u64, turn: &TurnId, body: String) -> SessionEvent {
    if seq.is_multiple_of(2) {
        SessionEvent::UserMessage {
            seq,
            at: "2026-11-14T00:00:00Z".to_owned(),
            turn: turn.clone(),
            content: vec![ContentBlock::text(body)],
        }
    } else {
        SessionEvent::AssistantMessage {
            seq,
            at: "2026-11-14T00:00:00Z".to_owned(),
            turn: turn.clone(),
            content: vec![ContentBlock::text(body)],
            usage: Usage::default(),
            stop_reason: None,
        }
    }
}

/// A session holding roughly 100k tokens (≈4 characters per token).
fn hundred_k_session(store: &SessionStore) -> dshr_session::Session {
    let mut session = store
        .create("deepseek-account", "deepseek-flash", "/workspace")
        .expect("create session");
    let turn = TurnId::new();
    // 2 000 messages × ~200 characters ≈ 400 000 characters ≈ 100k tokens at
    // the documented ~4 characters per token. Spreading the budget over many
    // events makes the per-event overhead visible instead of one huge string.
    for _ in 0..2_000_u64 {
        let body = "lorem ipsum dolor sit amet ".repeat(7);
        let event = text_event(session.next_seq(), &turn, body);
        store.append(&mut session, event).expect("append");
    }
    session
}

fn check(name: &str, samples: &mut [Duration], budget: Duration, failures: &mut Vec<String>) {
    let p95 = percentile(samples, 0.95);
    let min = percentile(samples, 0.0);
    let median = percentile(samples, 0.5);
    let ok = within_budget(p95, budget);
    println!(
        "{} {name:<38} p95={}  (min={min:?} p50={median:?})",
        if ok { "PASS" } else { "FAIL" },
        usage(p95, budget)
    );
    if !ok {
        failures.push(format!("{name}: p95 {p95:?} exceeds {budget:?}"));
    }
}

#[test]
fn perf_budgets_hold() {
    // Debug builds are 10-40x slower than release, so timings are meaningless.
    // The CI gate runs `cargo test -p dshr-perf --release`; a plain
    // `cargo test --workspace` must stay green and simply skip the budgets.
    if cfg!(debug_assertions) {
        println!("SKIP: perf budgets are measured in release (cargo test -p dshr-perf --release)");
        return;
    }
    let mut failures = Vec::new();

    // 1. Prompt assembly with 100 tools. Real directories are used because
    // `build` stats `AGENTS.md` along the cwd ancestry; synthetic paths such as
    // `/home` trigger autofs lookups on macOS and would measure the wrong thing.
    let layout = tempfile::tempdir().expect("tempdir");
    let cwd = layout.path().join("project");
    let home = layout.path().join("home");
    std::fs::create_dir_all(&cwd).expect("cwd");
    std::fs::create_dir_all(&home).expect("home");
    std::fs::write(
        home.join("AGENTS.md"),
        "# Global rules\nKeep diffs small.\n",
    )
    .expect("agents");
    let context = prompt_context(&cwd, &home, hundred_tools());
    let started = Instant::now();
    let prompt = dshr_prompt::build(&context);
    let first_prompt = started.elapsed();
    assert!(prompt.contains("tool_099"), "prompt lists every tool");
    let mut samples: Vec<Duration> = (0..200)
        .map(|_| {
            let started = Instant::now();
            let built = dshr_prompt::build(&context);
            std::hint::black_box(built.len());
            started.elapsed()
        })
        .collect();
    samples.push(first_prompt);
    check(
        "prompt build (100 tools)",
        &mut samples,
        PROMPT_BUDGET,
        &mut failures,
    );

    // 2. Journal append with fsync.
    let dir = tempfile::tempdir().expect("tempdir");
    let store = SessionStore::new(dir.path());
    let mut session = store
        .create("deepseek-account", "deepseek-flash", "/workspace")
        .expect("create session");
    let turn = TurnId::new();
    let mut samples = Vec::with_capacity(200);
    for _ in 0..200 {
        let event = text_event(session.next_seq(), &turn, "append sample".repeat(8));
        let started = Instant::now();
        store.append(&mut session, event).expect("append");
        samples.push(started.elapsed());
    }
    check(
        "event append (fsync on)",
        &mut samples,
        APPEND_BUDGET,
        &mut failures,
    );

    // 3. Session search over 10k events.
    let search_dir = tempfile::tempdir().expect("tempdir");
    let search_store = SessionStore::new(search_dir.path());
    for session_index in 0..100 {
        let mut value = search_store
            .create("deepseek-account", "deepseek-flash", "/workspace")
            .expect("create session");
        let turn = TurnId::new();
        let _ = session_index;
        for event_index in 0..100 {
            let event = text_event(
                value.next_seq(),
                &turn,
                format!("session event {event_index} needle target"),
            );
            search_store.append(&mut value, event).expect("append");
        }
    }
    let mut index = SessionSearchIndex::open(search_dir.path()).expect("index");
    let indexed = index.rebuild(&search_store).expect("rebuild");
    assert_eq!(indexed, 10_000, "index covers 10k events");
    let mut samples = Vec::with_capacity(20);
    for _ in 0..20 {
        let started = Instant::now();
        let hits = index.search("needle target", 20).expect("search");
        assert!(!hits.is_empty(), "search finds indexed events");
        samples.push(started.elapsed());
    }
    check(
        "session search (10k events)",
        &mut samples,
        SEARCH_BUDGET,
        &mut failures,
    );

    // 4. Turn-context assembly without an LLM: derive messages + prompt.
    let session_dir = tempfile::tempdir().expect("tempdir");
    let session_store = SessionStore::new(session_dir.path());
    let before = rss_kb();
    let session = hundred_k_session(&session_store);
    let after = rss_kb();
    let mut samples = Vec::with_capacity(50);
    for _ in 0..50 {
        let started = Instant::now();
        let messages = session.derive_messages();
        std::hint::black_box(messages.len());
        let built = dshr_prompt::build(&context);
        std::hint::black_box(built.len());
        samples.push(started.elapsed());
    }
    check(
        "turn context (100k tokens, no LLM)",
        &mut samples,
        TURN_BUDGET,
        &mut failures,
    );

    // 5. Resident memory of the loaded 100k-token session.
    match (before, after) {
        (Some(before), Some(after)) => {
            let delta_mb = after.saturating_sub(before) / 1024;
            let ok = delta_mb < MEMORY_BUDGET_MB;
            println!(
                "{} {:<38} {delta_mb} MB / < {MEMORY_BUDGET_MB} MB",
                if ok { "PASS" } else { "FAIL" },
                "session memory (100k tokens)"
            );
            if !ok {
                failures.push(format!(
                    "session memory: {delta_mb} MB exceeds {MEMORY_BUDGET_MB} MB"
                ));
            }
        }
        _ => println!("SKIP session memory (100k tokens): ps unavailable on this host"),
    }

    assert!(
        failures.is_empty(),
        "performance budget regressions: {}",
        failures.join(" | ")
    );
}

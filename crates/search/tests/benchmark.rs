use std::time::Instant;

use dshr_core::TurnId;
use dshr_llm::ContentBlock;
use dshr_search::SessionSearchIndex;
use dshr_session::SessionEvent;
use dshr_session_jsonl::SessionStore;

#[test]
#[ignore = "synthetic benchmark; run explicitly"]
fn benchmark_scan_vs_fts() {
    let sessions = std::env::var("DSHR_BENCH_SESSIONS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(1_000_usize);
    let events = std::env::var("DSHR_BENCH_EVENTS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(20_usize);
    let dir = tempfile::tempdir().expect("tempdir");
    let store = SessionStore::new(dir.path());

    let started = Instant::now();
    for session in 0..sessions {
        let mut value = store
            .create("bench", "bench", "/tmp")
            .expect("create session");
        for event in 0..events {
            let seq = value.next_seq();
            let event = SessionEvent::UserMessage {
                seq,
                at: "2026-01-01T00:00:00Z".to_owned(),
                turn: TurnId::new(),
                content: vec![ContentBlock::text(format!(
                    "session {session} event {event} needle target"
                ))],
            };
            store.append(&mut value, event).expect("append");
        }
    }
    let build = started.elapsed();

    let started = Instant::now();
    let scan = store.search("needle target", 20).expect("scan");
    let scan_time = started.elapsed();

    let mut index = SessionSearchIndex::open(dir.path()).expect("index");
    let started = Instant::now();
    let indexed = index.rebuild(&store).expect("rebuild");
    let index_time = started.elapsed();

    let started = Instant::now();
    let fts = index.search("needle target", 20).expect("fts");
    let fts_time = started.elapsed();

    println!(
        "sessions={sessions} events={events} total={} build={build:?} scan_hits={} scan={scan_time:?} indexed={indexed} index={index_time:?} fts_hits={} fts={fts_time:?}",
        sessions * events,
        scan.len(),
        fts.len()
    );
    assert!(!scan.is_empty());
    assert!(!fts.is_empty());
}

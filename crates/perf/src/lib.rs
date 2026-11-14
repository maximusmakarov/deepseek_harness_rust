#![forbid(unsafe_code)]
//! Shared helpers for the performance budget gate (ARCHITECTURE §20).
//!
//! Every budget is measured on the hot path the harness actually runs: prompt
//! assembly, journal append, session search, turn-context assembly and the
//! resident memory of a 100k-token session.

use std::process::Command;
use std::time::Duration;

/// Duration at the given percentile (`p` in `0.0..=1.0`) of the samples.
///
/// Samples are sorted in place; the percentile index is rounded up so a single
/// outlier cannot be hidden by an even split.
#[must_use]
pub fn percentile(samples: &mut [Duration], p: f64) -> Duration {
    assert!(!samples.is_empty(), "percentile needs at least one sample");
    samples.sort_unstable();
    let last = samples.len() - 1;
    let rank = (p.clamp(0.0, 1.0) * last as f64).ceil() as usize;
    samples[rank.min(last)]
}

/// Resident set size of the current process in kilobytes.
///
/// `ps` is used instead of platform APIs so the same code runs on macOS,
/// Linux and Windows (via the `ps` shipped with Git for Windows). Returns
/// `None` when the process cannot be queried, and callers skip the check
/// instead of failing on an unsupported host.
#[must_use]
pub fn rss_kb() -> Option<u64> {
    let pid = std::process::id();
    let output = Command::new("ps")
        .args(["-o", "rss=", "-p", &pid.to_string()])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8_lossy(&output.stdout).trim().parse().ok()
}

/// Budget verdict with a 20 % regression allowance, matching the plan rule
/// "regression > 20 % is a blocker". `DSHR_PERF_STRICT=1` removes the margin.
#[must_use]
pub fn within_budget(measured: Duration, budget: Duration) -> bool {
    let strict = std::env::var_os("DSHR_PERF_STRICT").is_some_and(|value| value == "1");
    let allowed = if strict { budget } else { budget.mul_f64(1.2) };
    measured <= allowed
}

/// Formats a measurement as `value / budget (used %)`.
#[must_use]
pub fn usage(measured: Duration, budget: Duration) -> String {
    let ratio = measured.as_secs_f64() / budget.as_secs_f64() * 100.0;
    format!("{measured:?} / {budget:?} ({ratio:.1}%)")
}

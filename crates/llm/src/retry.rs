//! Provider-owned retry policy, mirroring DSH's `resolveRetryPolicy`.
//!
//! Adapters capture one plan when the provider is built and execute it on
//! request failures. The plan is pure data plus timing helpers, so it stays in
//! this provider-neutral crate while the persisted configuration lives in
//! `dshr-config`.

use std::collections::BTreeSet;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Failure codes a policy may retry, using DSH's stable names.
pub const RETRY_CODES: [&str; 8] = [
    "EMPTY_RESPONSE",
    "RATE_LIMIT",
    "SERVER",
    "TIMEOUT",
    "TRANSPORT",
    "STREAM_CLOSED",
    "MALFORMED_RESPONSE",
    "QUOTA",
];

/// Codes a default normal policy retries, like upstream.
pub const DEFAULT_RETRYABLE_CODES: [&str; 5] = [
    "EMPTY_RESPONSE",
    "RATE_LIMIT",
    "SERVER",
    "TIMEOUT",
    "TRANSPORT",
];

pub const DEFAULT_MAX_RETRIES: u32 = 5;
pub const DEFAULT_INITIAL_DELAY_MS: u64 = 500;
pub const DEFAULT_MAX_DELAY_MS: u64 = 10_000;
pub const DEFAULT_JITTER_RATIO: f32 = 0.1;
pub const MAX_TIMER_DELAY_MS: u64 = 2_147_483_647;

/// How a policy decides that a failure is eligible.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RetryMode {
    /// Retry only the configured failure codes, at most `max_retries` times.
    #[default]
    Normal,
    /// Retry every model-request failure until success or cancellation.
    Always,
}

impl RetryMode {
    #[must_use]
    pub fn id(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Always => "always",
        }
    }

    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "normal" => Some(Self::Normal),
            "always" => Some(Self::Always),
            _ => None,
        }
    }
}

/// One classified failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryCode {
    EmptyResponse,
    RateLimit,
    Server,
    Timeout,
    Transport,
    StreamClosed,
    MalformedResponse,
    Quota,
    /// A failure no configured code names (for example a 401).
    Other,
}

impl RetryCode {
    #[must_use]
    pub fn id(self) -> &'static str {
        match self {
            Self::EmptyResponse => "EMPTY_RESPONSE",
            Self::RateLimit => "RATE_LIMIT",
            Self::Server => "SERVER",
            Self::Timeout => "TIMEOUT",
            Self::Transport => "TRANSPORT",
            Self::StreamClosed => "STREAM_CLOSED",
            Self::MalformedResponse => "MALFORMED_RESPONSE",
            Self::Quota => "QUOTA",
            Self::Other => "OTHER",
        }
    }

    /// Classifies an HTTP status; 4xx statuses other than 429 are not retried
    /// by a normal policy, while always mode retries them too.
    #[must_use]
    pub fn from_status(status: u16) -> Self {
        match status {
            429 => Self::RateLimit,
            500..=599 => Self::Server,
            _ => Self::Other,
        }
    }
}

/// One resolved policy.
#[derive(Debug, Clone, PartialEq)]
pub struct RetryPlan {
    pub mode: RetryMode,
    pub max_retries: u32,
    pub retryable_codes: BTreeSet<String>,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub jitter_ratio: f32,
}

impl Default for RetryPlan {
    fn default() -> Self {
        Self {
            mode: RetryMode::Normal,
            max_retries: DEFAULT_MAX_RETRIES,
            retryable_codes: DEFAULT_RETRYABLE_CODES
                .iter()
                .map(|code| (*code).to_owned())
                .collect(),
            initial_delay: Duration::from_millis(DEFAULT_INITIAL_DELAY_MS),
            max_delay: Duration::from_millis(DEFAULT_MAX_DELAY_MS),
            jitter_ratio: DEFAULT_JITTER_RATIO,
        }
    }
}

impl RetryPlan {
    /// Builds a plan from resolved settings. Values outside the accepted range
    /// are clamped, so a hand-edited document cannot disable a provider with a
    /// nonsensical delay.
    #[must_use]
    pub fn new(
        mode: RetryMode,
        max_retries: u32,
        retryable_codes: impl IntoIterator<Item = String>,
        initial_delay_ms: u64,
        max_delay_ms: u64,
        jitter_ratio: f32,
    ) -> Self {
        let max_delay = Duration::from_millis(max_delay_ms.clamp(1, MAX_TIMER_DELAY_MS));
        let initial_delay =
            Duration::from_millis(initial_delay_ms.clamp(1, MAX_TIMER_DELAY_MS)).min(max_delay);
        Self {
            mode,
            max_retries,
            retryable_codes: retryable_codes.into_iter().collect(),
            initial_delay,
            max_delay,
            jitter_ratio: if jitter_ratio.is_finite() {
                jitter_ratio.clamp(0.0, 1.0)
            } else {
                DEFAULT_JITTER_RATIO
            },
        }
    }

    /// Whether a classified failure is eligible under this policy.
    #[must_use]
    pub fn should_retry(&self, code: RetryCode) -> bool {
        match self.mode {
            RetryMode::Always => true,
            RetryMode::Normal => self.retryable_codes.contains(code.id()),
        }
    }

    /// Whether another retry is allowed. `attempt` is the retry number,
    /// starting at 1 for the first repeat.
    #[must_use]
    pub fn has_attempts_left(&self, attempt: u32) -> bool {
        match self.mode {
            RetryMode::Always => true,
            RetryMode::Normal => attempt <= self.max_retries,
        }
    }

    /// The delay before retry number `attempt`, honoring a provider
    /// `Retry-After` when one was sent.
    #[must_use]
    pub fn delay(&self, attempt: u32, retry_after: Option<Duration>) -> Duration {
        let exponential = self
            .initial_delay
            .saturating_mul(1u32 << attempt.saturating_sub(1).min(16));
        let base = retry_after.unwrap_or(exponential).min(self.max_delay);
        self.jitter(base, attempt)
    }

    fn jitter(&self, base: Duration, attempt: u32) -> Duration {
        if self.jitter_ratio <= 0.0 {
            return base;
        }
        // A tiny deterministic PRNG keeps the crate dependency-free while the
        // spread still differs between retries and provider routes.
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.subsec_nanos() as u64)
            ^ u64::from(attempt).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        let mut value = seed.max(1);
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        let unit = (value % 1_000_000) as f32 / 1_000_000.0;
        let factor = 1.0 + self.jitter_ratio * (2.0 * unit - 1.0);
        let millis = (base.as_secs_f64() * 1000.0 * f64::from(factor)).max(1.0);
        Duration::from_millis(millis.round() as u64).min(self.max_delay)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_upstream() {
        let plan = RetryPlan::default();
        assert_eq!(plan.mode, RetryMode::Normal);
        assert_eq!(plan.max_retries, 5);
        assert_eq!(plan.initial_delay, Duration::from_millis(500));
        assert_eq!(plan.max_delay, Duration::from_millis(10_000));
        assert!((plan.jitter_ratio - 0.1).abs() < f32::EPSILON);
        assert!(plan.should_retry(RetryCode::RateLimit));
        assert!(!plan.should_retry(RetryCode::Other));
    }

    #[test]
    fn normal_mode_stops_at_max_retries_and_always_does_not() {
        let mut plan = RetryPlan {
            max_retries: 2,
            ..RetryPlan::default()
        };
        assert!(plan.has_attempts_left(1));
        assert!(plan.has_attempts_left(2));
        assert!(!plan.has_attempts_left(3));

        plan.mode = RetryMode::Always;
        assert!(plan.has_attempts_left(10_000));
        assert!(plan.should_retry(RetryCode::Other));
    }

    #[test]
    fn backoff_is_bounded_and_honors_retry_after() {
        let plan = RetryPlan {
            jitter_ratio: 0.0,
            ..RetryPlan::default()
        };
        assert_eq!(plan.delay(1, None), Duration::from_millis(500));
        assert_eq!(plan.delay(2, None), Duration::from_millis(1000));
        assert_eq!(plan.delay(10, None), Duration::from_millis(10_000));
        assert_eq!(
            plan.delay(2, Some(Duration::from_secs(2))),
            Duration::from_secs(2)
        );
        assert_eq!(
            plan.delay(1, Some(Duration::from_secs(60))),
            Duration::from_millis(10_000)
        );
    }

    #[test]
    fn a_plan_clamps_invalid_timing() {
        let plan = RetryPlan::new(
            RetryMode::Normal,
            1,
            ["RATE_LIMIT".to_owned()],
            50_000,
            1_000,
            f32::NAN,
        );
        assert_eq!(plan.initial_delay, Duration::from_millis(1_000));
        assert_eq!(plan.max_delay, Duration::from_millis(1_000));
        assert!((plan.jitter_ratio - 0.1).abs() < f32::EPSILON);
    }
}

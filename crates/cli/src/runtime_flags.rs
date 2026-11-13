//! Process-wide runtime switches set from global CLI flags.

use std::sync::atomic::{AtomicBool, Ordering};

static SAFE_MODE: AtomicBool = AtomicBool::new(false);

pub fn set_safe_mode(value: bool) {
    SAFE_MODE.store(value, Ordering::SeqCst);
}

#[must_use]
pub fn safe_mode() -> bool {
    SAFE_MODE.load(Ordering::SeqCst)
}

//! Wall-clock helper: current Unix time in seconds.

use std::time::{SystemTime, UNIX_EPOCH};

/// Current Unix time in seconds (`0` on a clock error). The shared source for the
/// stats recorder's event timestamps and the `update-check` throttle.
#[must_use]
pub(crate) fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

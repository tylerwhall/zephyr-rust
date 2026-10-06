//! Application-side adapters for Zephyr's monotonic clock.

use std::time::Instant;
use zephyr_core::Ticks;

/// Converts an instant without rounding its tick count or inspecting its layout.
///
/// The std port and this crate use the same Zephyr image's tick epoch and rate,
/// but may use independent instances of zephyr-core.
pub fn instant_ticks(instant: Instant) -> Ticks {
    Ticks::from(instant.as_zephyr_ticks())
}

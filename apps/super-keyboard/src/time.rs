//! The clock and timer the gesture engine runs on.

use std::{sync::OnceLock, time::Duration};

use web_time::Instant;

/// Monotonic time since the app started. [`super_keyboard_core::Gestures`] only
/// compares these values, so the origin does not matter.
pub fn now() -> Duration {
    static START: OnceLock<Instant> = OnceLock::new();
    START.get_or_init(Instant::now).elapsed()
}

/// Waits for `duration` on whichever runtime the platform uses.
pub async fn sleep(duration: Duration) {
    #[cfg(not(target_arch = "wasm32"))]
    tokio::time::sleep(duration).await;
    #[cfg(target_arch = "wasm32")]
    gloo_timers::future::sleep(duration).await;
}

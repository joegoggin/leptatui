//! Redraw wakeup coordination for the app runner.
//!
//! Async resources and actions can complete outside terminal input handling.
//! This module gives those background tasks a crate-internal path to wake the
//! runner so completed work can be rendered promptly.

use std::sync::{
    OnceLock,
    atomic::{AtomicU64, Ordering},
};

use tokio::sync::watch;

/// Monotonic generation counter incremented for each redraw request.
static REDRAW_GENERATION: AtomicU64 = AtomicU64::new(0);
/// Shared watch channel sender used to notify app loops about redraw requests.
static REDRAW_SENDER: OnceLock<watch::Sender<u64>> = OnceLock::new();
#[cfg(test)]
/// Global test mutex that prevents redraw-channel cross-talk between tests.
static REDRAW_TEST_LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();

#[cfg(test)]
/// Acquires the redraw test lock.
///
/// # Returns
///
/// A [`tokio::sync::MutexGuard`] that serializes redraw-channel tests.
pub(crate) async fn redraw_test_lock() -> tokio::sync::MutexGuard<'static, ()> {
    REDRAW_TEST_LOCK
        .get_or_init(|| tokio::sync::Mutex::new(()))
        .lock()
        .await
}

/// Requests that active app runners perform a redraw.
pub(crate) fn request_redraw() {
    let generation = REDRAW_GENERATION.fetch_add(1, Ordering::AcqRel) + 1;
    let _ = redraw_sender().send_replace(generation);
}

/// Subscribes to future redraw requests.
pub(crate) fn subscribe_redraws() -> watch::Receiver<u64> {
    redraw_sender().subscribe()
}

/// Returns the shared redraw watch-channel sender.
///
/// # Returns
///
/// A static [`watch::Sender`] initialized to the current redraw generation.
fn redraw_sender() -> &'static watch::Sender<u64> {
    REDRAW_SENDER.get_or_init(|| {
        let generation = REDRAW_GENERATION.load(Ordering::Acquire);
        let (sender, _receiver) = watch::channel(generation);
        sender
    })
}

#[cfg(test)]
/// Tests for redraw wakeup delivery.
mod tests {
    use std::time::Duration;

    use tokio::time::timeout;

    use super::*;

    /// Verifies redraw requests wake subscribed app loops.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// let mut redraws = subscribe_redraws();
    /// request_redraw();
    /// redraws.changed().await
    /// ```
    ///
    /// # Assertions
    ///
    /// - The redraw request is delivered to the subscriber before timeout.
    /// - The redraw sender remains available after the request.
    #[tokio::test(flavor = "current_thread")]
    async fn request_redraw_wakes_subscriber() {
        let _redraw_guard = redraw_test_lock().await;
        let mut redraws = subscribe_redraws();

        request_redraw();

        timeout(Duration::from_secs(1), redraws.changed())
            .await
            .expect("redraw request should arrive")
            .expect("redraw sender should stay available");
    }
}

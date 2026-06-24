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

static REDRAW_GENERATION: AtomicU64 = AtomicU64::new(0);
static REDRAW_SENDER: OnceLock<watch::Sender<u64>> = OnceLock::new();
#[cfg(test)]
static REDRAW_TEST_LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();

#[cfg(test)]
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

fn redraw_sender() -> &'static watch::Sender<u64> {
    REDRAW_SENDER.get_or_init(|| {
        let generation = REDRAW_GENERATION.load(Ordering::Acquire);
        let (sender, _receiver) = watch::channel(generation);
        sender
    })
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tokio::time::timeout;

    use super::*;

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

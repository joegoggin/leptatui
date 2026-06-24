//! Async executor initialization helpers.

use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

/// Initializes the Any Spawner Tokio executor used by Leptos effects.
pub(crate) fn init_tokio_executor() {
    let _ = any_spawner::Executor::init_tokio();
}

/// Tracks the latest generation of spawned async work.
#[derive(Clone, Default)]
pub(crate) struct LatestTask {
    generation: Arc<AtomicU64>,
}

impl LatestTask {
    /// Advances to the next generation and returns its id.
    pub(crate) fn next(&self) -> u64 {
        self.generation.fetch_add(1, Ordering::AcqRel) + 1
    }

    /// Returns whether `generation` is still the latest task generation.
    pub(crate) fn is_current(&self, generation: u64) -> bool {
        self.generation.load(Ordering::Acquire) == generation
    }
}

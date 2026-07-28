//! Signal-backed asynchronous actions for terminal apps.
//!
//! Actions mirror Leptos action semantics: the asynchronous handler can return
//! any output type, including an application-owned [`std::result::Result`].
//! Leptatui only manages pending, input, value, and version signals and
//! requests terminal redraws when those signals change.

use std::{future::Future, pin::Pin, sync::Arc};

use leptos::prelude::{Get, GetUntracked, ReadSignal, Set, Update, WriteSignal, signal};

use crate::{
    app::request_redraw,
    executor::{LatestTask, init_tokio_executor},
};

/// Boxed future returned by an action handler.
type BoxActionFuture<O> = Pin<Box<dyn Future<Output = O> + Send>>;
/// Shared asynchronous handler invoked for each dispatched input.
type ActionHandler<I, O> = Arc<dyn Fn(&I) -> BoxActionFuture<O> + Send + Sync>;

/// Reactive handle for an asynchronous mutation.
///
/// The action stores the latest output without interpreting it. Applications
/// represent recoverable failures with an output such as `Result<T, E>` and
/// handle that result in their own components or contexts.
pub struct Action<I, O> {
    /// Signal indicating whether the latest dispatch is running.
    pending: ReadSignal<bool>,
    /// Setter for the pending signal.
    set_pending: WriteSignal<bool>,
    /// Signal containing the latest in-flight input.
    input: ReadSignal<Option<I>>,
    /// Setter for the input signal.
    set_input: WriteSignal<Option<I>>,
    /// Signal containing the latest completed output.
    value: ReadSignal<Option<O>>,
    /// Setter for the output signal.
    set_value: WriteSignal<Option<O>>,
    /// Signal containing the number of visible completions.
    version: ReadSignal<usize>,
    /// Setter for the completion-version signal.
    set_version: WriteSignal<usize>,
    /// Asynchronous handler invoked for each dispatch.
    handler: ActionHandler<I, O>,
    /// Monotonic dispatch identifier used to ignore stale completions.
    latest_dispatch: LatestTask,
}

impl<I, O> Clone for Action<I, O> {
    /// Clones the action signals and shared handler.
    fn clone(&self) -> Self {
        Self {
            pending: self.pending,
            set_pending: self.set_pending,
            input: self.input,
            set_input: self.set_input,
            value: self.value,
            set_value: self.set_value,
            version: self.version,
            set_version: self.set_version,
            handler: Arc::clone(&self.handler),
            latest_dispatch: self.latest_dispatch.clone(),
        }
    }
}

impl<I, O> Action<I, O>
where
    I: Clone + Send + Sync + 'static,
    O: Send + Sync + 'static,
{
    /// Creates an action from an asynchronous mutation handler.
    ///
    /// # Arguments
    ///
    /// * `handler` — Function that receives a dispatched input by reference and
    ///   returns the asynchronous output.
    ///
    /// # Returns
    ///
    /// An [`Action`] with idle pending, input, and value signals.
    pub fn new<F, Fut>(handler: F) -> Self
    where
        F: Fn(&I) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = O> + Send + 'static,
    {
        init_tokio_executor();

        let (pending, set_pending) = signal(false);
        let (input, set_input) = signal(None);
        let (value, set_value) = signal(None);
        let (version, set_version) = signal(0);
        let handler: ActionHandler<I, O> = Arc::new(move |input| Box::pin(handler(input)));

        Self {
            pending,
            set_pending,
            input,
            set_input,
            value,
            set_value,
            version,
            set_version,
            handler,
            latest_dispatch: LatestTask::default(),
        }
    }

    /// Dispatches the action with an input value.
    ///
    /// The latest dispatch controls the visible state. Older tasks may finish,
    /// but their completions cannot replace the latest output.
    ///
    /// # Arguments
    ///
    /// * `input` — Value passed to the asynchronous handler.
    ///
    /// # Panics
    ///
    /// Panics if called outside a Tokio runtime.
    pub fn dispatch(&self, input: I) {
        let dispatch_id = self.latest_dispatch.next();
        let task_input = input.clone();
        let future = (self.handler)(&task_input);

        let _ = self.set_input.try_set(Some(input));
        let _ = self.set_pending.try_set(true);
        request_redraw();

        let latest_dispatch = self.latest_dispatch.clone();
        let set_pending = self.set_pending;
        let set_input = self.set_input;
        let set_value = self.set_value;
        let set_version = self.set_version;

        tokio::spawn(async move {
            let output = future.await;

            if latest_dispatch.is_current(dispatch_id) {
                let _ = set_value.try_set(Some(output));
                let _ = set_input.try_set(None);
                let _ = set_pending.try_set(false);
                set_version.update(|version| *version += 1);
                request_redraw();
            }
        });
    }

    /// Clears the most recent completed output.
    ///
    /// Clearing does not cancel an in-flight dispatch or modify its input.
    pub fn clear(&self) {
        let _ = self.set_value.try_set(None);
        request_redraw();
    }

    /// Returns the signal indicating whether the latest dispatch is running.
    ///
    /// # Returns
    ///
    /// A [`ReadSignal<bool>`] containing the pending state.
    pub fn pending(&self) -> ReadSignal<bool> {
        self.pending
    }

    /// Returns the signal containing the latest in-flight input.
    ///
    /// # Returns
    ///
    /// A [`ReadSignal<Option<I>>`] that clears after the latest completion.
    pub fn input(&self) -> ReadSignal<Option<I>> {
        self.input
    }

    /// Returns the signal containing the latest completed output.
    ///
    /// # Returns
    ///
    /// A [`ReadSignal<Option<O>>`] that retains its value during later
    /// dispatches.
    pub fn value(&self) -> ReadSignal<Option<O>> {
        self.value
    }

    /// Returns the signal containing the visible completion count.
    ///
    /// # Returns
    ///
    /// A [`ReadSignal<usize>`] incremented after each latest completion.
    pub fn version(&self) -> ReadSignal<usize> {
        self.version
    }

    /// Returns whether the latest dispatch is running.
    ///
    /// # Returns
    ///
    /// A [`bool`] containing the tracked pending state.
    pub fn is_pending(&self) -> bool {
        self.pending.get()
    }

    /// Returns whether the latest dispatch is running without tracking.
    ///
    /// # Returns
    ///
    /// A [`bool`] containing the untracked pending state.
    pub fn is_pending_untracked(&self) -> bool {
        self.pending.get_untracked()
    }
}

#[cfg(test)]
/// Tests for action redraw wakeups.
mod tests {
    use std::{
        sync::{Arc, Mutex},
        time::Duration,
    };

    use leptos::prelude::{GetUntracked, Owner};
    use tokio::{sync::oneshot, task::yield_now, time::timeout};

    use crate::app::{redraw_test_lock, subscribe_redraws};

    use super::*;

    /// Result returned by the controlled test action handler.
    type TestActionResult = std::result::Result<String, &'static str>;
    /// Shared sender slot for completing a pending test action.
    type PendingAction = Arc<Mutex<Option<oneshot::Sender<TestActionResult>>>>;

    /// Verifies successful action completion requests a redraw.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// Action::new(|input| async move { Ok(input) })
    /// action.dispatch(5)
    /// ```
    ///
    /// # Assertions
    ///
    /// - Dispatching sends a redraw request.
    /// - Successful completion sends another redraw request.
    /// - The action stores `Ok("5:saved")` as its output value.
    #[tokio::test(flavor = "current_thread")]
    async fn successful_completion_requests_redraw() {
        assert_completion_requests_redraw(Ok(String::from("saved"))).await;
    }

    /// Verifies failed action completion requests a redraw.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// Action::new(|_| async move { Err("offline") })
    /// action.dispatch(5)
    /// ```
    ///
    /// # Assertions
    ///
    /// - Dispatching sends a redraw request.
    /// - Failed completion sends another redraw request.
    /// - The action stores `Err("offline")` as its output value.
    #[tokio::test(flavor = "current_thread")]
    async fn error_completion_requests_redraw() {
        assert_completion_requests_redraw(Err("offline")).await;
    }

    /// Verifies redraw behavior for one controlled action response.
    ///
    /// # Arguments
    ///
    /// * `response` — Result sent into the pending action task.
    async fn assert_completion_requests_redraw(response: TestActionResult) {
        let _redraw_guard = redraw_test_lock().await;
        let owner = Owner::new();
        let expected = response.clone();
        let pending = PendingAction::default();
        let pending_for_action = Arc::clone(&pending);
        let mut redraws = subscribe_redraws();

        let action: Action<i32, TestActionResult> = owner.with(|| {
            Action::new(move |input| {
                let input = *input;
                let pending = Arc::clone(&pending_for_action);

                async move {
                    let receiver = insert_pending_action(&pending);
                    let value = receiver.await.expect("test action response")?;
                    Ok(format!("{input}:{value}"))
                }
            })
        });

        action.dispatch(5);

        timeout(Duration::from_secs(1), redraws.changed())
            .await
            .expect("pending redraw request should arrive")
            .expect("redraw sender should stay available");
        redraws.borrow_and_update();

        wait_until_pending_action(&pending).await;
        send_action_response(&pending, response);

        timeout(Duration::from_secs(1), redraws.changed())
            .await
            .expect("completion redraw request should arrive")
            .expect("redraw sender should stay available");

        timeout(Duration::from_secs(1), async {
            while action.value().get_untracked().is_none() {
                yield_now().await;
            }
        })
        .await
        .expect("action output should be stored");

        let result = action.value().get_untracked();

        match expected {
            Ok(value) => {
                assert_eq!(result, Some(Ok(format!("5:{value}"))));
            }
            Err(error) => assert_eq!(result, Some(Err(error))),
        }
    }

    /// Inserts a sender for the current test action and returns its receiver.
    ///
    /// # Arguments
    ///
    /// * `pending` — Shared slot storing the action response sender.
    ///
    /// # Returns
    ///
    /// A [`oneshot::Receiver`] awaited by the test handler.
    fn insert_pending_action(pending: &PendingAction) -> oneshot::Receiver<TestActionResult> {
        let (sender, receiver) = oneshot::channel();
        *pending.lock().expect("pending action lock") = Some(sender);
        receiver
    }

    /// Waits until the controlled action registers its response sender.
    ///
    /// # Arguments
    ///
    /// * `pending` — Shared slot inspected for the pending sender.
    async fn wait_until_pending_action(pending: &PendingAction) {
        timeout(Duration::from_secs(1), async {
            while pending.lock().expect("pending action lock").is_none() {
                yield_now().await;
            }
        })
        .await
        .expect("pending action should be registered");
    }

    /// Sends a controlled response to the pending action.
    ///
    /// # Arguments
    ///
    /// * `pending` — Shared slot containing the response sender.
    /// * `response` — Result delivered to the action handler.
    fn send_action_response(pending: &PendingAction, response: TestActionResult) {
        let sender = pending
            .lock()
            .expect("pending action lock")
            .take()
            .expect("pending action should exist");
        sender.send(response).expect("send action response");
    }
}

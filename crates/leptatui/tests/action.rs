//! Action tests.
//!
//! These tests verify Leptatui's signal-backed async mutation state transitions.

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use leptatui::prelude::*;
use tokio::{sync::oneshot, task::yield_now, time::timeout};

/// Result returned by the controlled test action handler.
type TestActionResult = std::result::Result<String, &'static str>;
/// Shared sender slot for completing a pending test action.
type PendingAction = Arc<Mutex<Option<oneshot::Sender<TestActionResult>>>>;

/// Verifies dispatch sets pending state and captures the dispatched input.
///
/// # Example Under Test
///
/// ```text
/// let action = create_action(|input| async move { Ok(format!("{input}:saved")) });
/// action.dispatch(7);
/// ```
///
/// # Assertions
///
/// - The action handler registers a pending response sender.
/// - The action reports that it is pending.
/// - The action stores `7` as the latest input.
/// - The action has no result while pending.
#[tokio::test(flavor = "current_thread")]
async fn dispatch_sets_pending_state_and_captures_input() {
    let owner = Owner::new();
    let pending = PendingAction::default();
    let pending_for_action = Arc::clone(&pending);

    let action: Action<i32, String, &'static str> = owner.with(|| {
        create_action(move |input| {
            let pending = Arc::clone(&pending_for_action);

            async move {
                let receiver = insert_pending_action(&pending);
                let value = receiver.await.expect("test action response")?;
                Ok(format!("{input}:{value}"))
            }
        })
    });

    action.dispatch(7);

    wait_until(|| has_pending_action(&pending)).await;
    assert!(action.is_pending());
    assert_eq!(action.input_untracked(), Some(7));
    assert_eq!(action.result_untracked(), None);
}

/// Verifies successful completion stores the output and clears pending state.
///
/// # Example Under Test
///
/// ```text
/// action.dispatch(3);
/// send_action_response(Ok("saved"));
/// ```
///
/// # Assertions
///
/// - The action eventually exposes `3:saved` as its successful value.
/// - The action is no longer pending after completion.
/// - The action keeps `3` as the latest input.
/// - The action stores `Ok("3:saved")` as its full result.
/// - The action exposes no error.
#[tokio::test(flavor = "current_thread")]
async fn successful_completion_stores_output() {
    let owner = Owner::new();
    let pending = PendingAction::default();
    let pending_for_action = Arc::clone(&pending);

    let action: Action<i32, String, &'static str> = owner.with(|| {
        create_action(move |input| {
            let pending = Arc::clone(&pending_for_action);

            async move {
                let receiver = insert_pending_action(&pending);
                let value = receiver.await.expect("test action response")?;
                Ok(format!("{input}:{value}"))
            }
        })
    });

    action.dispatch(3);

    wait_until(|| has_pending_action(&pending)).await;
    send_action_response(&pending, Ok(String::from("saved")));

    wait_until(|| action.value().as_deref() == Some("3:saved")).await;
    assert!(!action.is_pending());
    assert_eq!(action.input_untracked(), Some(3));
    assert_eq!(action.result_untracked(), Some(Ok(String::from("3:saved"))));
    assert_eq!(action.error(), None);
}

/// Verifies failed completion stores the error and clears pending state.
///
/// # Example Under Test
///
/// ```text
/// action.dispatch(4);
/// send_action_response(Err("offline"));
/// ```
///
/// # Assertions
///
/// - The action eventually exposes `offline` as its error.
/// - The action is no longer pending after completion.
/// - The action keeps `4` as the latest input.
/// - The action stores `Err("offline")` as its full result.
/// - The action exposes no successful value.
#[tokio::test(flavor = "current_thread")]
async fn failed_completion_stores_error() {
    let owner = Owner::new();
    let pending = PendingAction::default();
    let pending_for_action = Arc::clone(&pending);

    let action: Action<i32, String, &'static str> = owner.with(|| {
        create_action(move |input| {
            let pending = Arc::clone(&pending_for_action);

            async move {
                let receiver = insert_pending_action(&pending);
                let value = receiver.await.expect("test action response")?;
                Ok(format!("{input}:{value}"))
            }
        })
    });

    action.dispatch(4);

    wait_until(|| has_pending_action(&pending)).await;
    send_action_response(&pending, Err("offline"));

    wait_until(|| action.error() == Some("offline")).await;
    assert!(!action.is_pending());
    assert_eq!(action.input_untracked(), Some(4));
    assert_eq!(action.result_untracked(), Some(Err("offline")));
    assert_eq!(action.value(), None);
}

/// Inserts a sender for the current test action and returns its receiver.
///
/// # Arguments
///
/// * `pending` — Shared slot that stores the sender side of the action response.
///
/// # Returns
///
/// A [`oneshot::Receiver`] awaited by the test action handler.
fn insert_pending_action(pending: &PendingAction) -> oneshot::Receiver<TestActionResult> {
    let (sender, receiver) = oneshot::channel();
    *pending.lock().expect("pending action lock") = Some(sender);
    receiver
}

/// Returns whether a controlled action response sender has been registered.
///
/// # Arguments
///
/// * `pending` — Shared slot inspected for the pending sender.
///
/// # Returns
///
/// A [`bool`] indicating whether the test action is waiting for a response.
fn has_pending_action(pending: &PendingAction) -> bool {
    pending.lock().expect("pending action lock").is_some()
}

/// Sends the controlled action response to the pending task.
///
/// # Arguments
///
/// * `pending` — Shared slot containing the sender side of the action response.
/// * `response` — Result to deliver to the action handler.
fn send_action_response(pending: &PendingAction, response: TestActionResult) {
    let sender = pending
        .lock()
        .expect("pending action lock")
        .take()
        .expect("pending action should exist");
    sender.send(response).expect("send action response");
}

/// Waits until a predicate becomes true.
///
/// # Arguments
///
/// * `predicate` — Condition polled between Tokio task yields.
async fn wait_until(mut predicate: impl FnMut() -> bool) {
    timeout(Duration::from_secs(1), async {
        while !predicate() {
            yield_now().await;
        }
    })
    .await
    .expect("condition should become true");
}

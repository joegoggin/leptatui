//! Action tests.
//!
//! These tests verify Leptatui's signal-backed async mutation state transitions.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use leptatui::prelude::*;
use tokio::sync::oneshot;

mod support;

use support::{settle_tasks, wait_until};

/// Result returned by the controlled test action handler.
type TestActionResult = std::result::Result<String, &'static str>;
/// Shared sender slot for completing a pending test action.
type PendingAction = Arc<Mutex<Option<oneshot::Sender<TestActionResult>>>>;
/// Pending action senders keyed by dispatched input.
type PendingActions = Arc<Mutex<HashMap<i32, oneshot::Sender<TestActionResult>>>>;

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

    let action = create_test_action(&owner, &pending);

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

    let action = create_test_action(&owner, &pending);

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

    let action = create_test_action(&owner, &pending);

    action.dispatch(4);

    wait_until(|| has_pending_action(&pending)).await;
    send_action_response(&pending, Err("offline"));

    wait_until(|| action.error() == Some("offline")).await;
    assert!(!action.is_pending());
    assert_eq!(action.input_untracked(), Some(4));
    assert_eq!(action.result_untracked(), Some(Err("offline")));
    assert_eq!(action.value(), None);
}

/// Verifies a slower stale dispatch cannot overwrite a newer action result.
///
/// # Example Under Test
///
/// ```text
/// action.dispatch(1);
/// action.dispatch(2);
/// send_dispatch_response(2, Ok("second"));
/// send_dispatch_response(1, Ok("first"));
/// ```
///
/// # Assertions
///
/// - Dispatches are registered for inputs `1` and `2`.
/// - Completing input `2` first stores `2:second` as the action value.
/// - Completing stale input `1` afterward does not replace the newer value.
/// - The action keeps `2` as the latest input and full result.
///
/// # Why
///
/// Slow responses from older dispatches must not overwrite fresher mutation UI state.
#[tokio::test(flavor = "current_thread")]
async fn stale_dispatch_completion_does_not_overwrite_newer_result() {
    let owner = Owner::new();
    let pending = PendingActions::default();

    let action = create_keyed_test_action(&owner, &pending);

    action.dispatch(1);
    wait_until(|| has_pending_dispatch(&pending, 1)).await;

    action.dispatch(2);
    wait_until(|| has_pending_dispatch(&pending, 2)).await;

    send_dispatch_response(&pending, 2, Ok(String::from("second")));
    wait_until(|| action.value().as_deref() == Some("2:second")).await;

    send_dispatch_response(&pending, 1, Ok(String::from("first")));
    settle_tasks().await;

    assert_eq!(action.value().as_deref(), Some("2:second"));
    assert_eq!(action.input_untracked(), Some(2));
    assert_eq!(
        action.result_untracked(),
        Some(Ok(String::from("2:second")))
    );
}

/// Creates the controlled action used by action state tests.
///
/// # Arguments
///
/// * `owner` — Leptos owner that keeps the action signals alive for the test.
/// * `pending` — Shared slot used to control the action response.
///
/// # Returns
///
/// An [`Action`] that formats successful responses as `{input}:{value}`.
fn create_test_action(owner: &Owner, pending: &PendingAction) -> Action<i32, String, &'static str> {
    let pending_for_action = Arc::clone(pending);

    owner.with(|| {
        create_action(move |input| {
            let pending = Arc::clone(&pending_for_action);

            async move {
                let receiver = insert_pending_action(&pending);
                let value = receiver.await.expect("test action response")?;
                Ok(format!("{input}:{value}"))
            }
        })
    })
}

/// Creates a controlled action with pending senders keyed by input.
///
/// # Arguments
///
/// * `owner` — Leptos owner that keeps the action signals alive for the test.
/// * `pending` — Shared map used to control responses by dispatched input.
///
/// # Returns
///
/// An [`Action`] that formats successful responses as `{input}:{value}`.
fn create_keyed_test_action(
    owner: &Owner,
    pending: &PendingActions,
) -> Action<i32, String, &'static str> {
    let pending_for_action = Arc::clone(pending);

    owner.with(|| {
        create_action(move |input| {
            let pending = Arc::clone(&pending_for_action);

            async move {
                let receiver = insert_pending_dispatch(&pending, input);
                let value = receiver.await.expect("test action response")?;
                Ok(format!("{input}:{value}"))
            }
        })
    })
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

/// Inserts a pending dispatch sender for an input and returns its receiver.
///
/// # Arguments
///
/// * `pending` — Shared map that stores dispatch response senders by input.
/// * `input` — Dispatched input associated with the action task.
///
/// # Returns
///
/// A [`oneshot::Receiver`] awaited by the test action handler.
fn insert_pending_dispatch(
    pending: &PendingActions,
    input: i32,
) -> oneshot::Receiver<TestActionResult> {
    let (sender, receiver) = oneshot::channel();
    pending
        .lock()
        .expect("pending dispatch lock")
        .insert(input, sender);
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

/// Returns whether an input has a pending controlled dispatch.
///
/// # Arguments
///
/// * `pending` — Shared map inspected for the input.
/// * `input` — Dispatched input to look up.
///
/// # Returns
///
/// A [`bool`] indicating whether the input is waiting for a response.
fn has_pending_dispatch(pending: &PendingActions, input: i32) -> bool {
    pending
        .lock()
        .expect("pending dispatch lock")
        .contains_key(&input)
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

/// Sends a controlled dispatch response for an input.
///
/// # Arguments
///
/// * `pending` — Shared map containing the pending sender.
/// * `input` — Dispatched input whose task should be completed.
/// * `response` — Result to deliver to the action handler.
fn send_dispatch_response(pending: &PendingActions, input: i32, response: TestActionResult) {
    let sender = pending
        .lock()
        .expect("pending dispatch lock")
        .remove(&input)
        .expect("pending dispatch should exist");
    sender.send(response).expect("send dispatch response");
}

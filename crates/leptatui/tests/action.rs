//! Action tests.
//!
//! These tests verify Leptatui's signal-backed async mutation state transitions.

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use leptatui::prelude::*;
use tokio::{sync::oneshot, task::yield_now, time::timeout};

type TestActionResult = std::result::Result<String, &'static str>;
type PendingAction = Arc<Mutex<Option<oneshot::Sender<TestActionResult>>>>;

/// Verifies dispatch sets pending state and captures the dispatched input.
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

fn insert_pending_action(pending: &PendingAction) -> oneshot::Receiver<TestActionResult> {
    let (sender, receiver) = oneshot::channel();
    *pending.lock().expect("pending action lock") = Some(sender);
    receiver
}

fn has_pending_action(pending: &PendingAction) -> bool {
    pending.lock().expect("pending action lock").is_some()
}

fn send_action_response(pending: &PendingAction, response: TestActionResult) {
    let sender = pending
        .lock()
        .expect("pending action lock")
        .take()
        .expect("pending action should exist");
    sender.send(response).expect("send action response");
}

async fn wait_until(mut predicate: impl FnMut() -> bool) {
    timeout(Duration::from_secs(1), async {
        while !predicate() {
            yield_now().await;
        }
    })
    .await
    .expect("condition should become true");
}

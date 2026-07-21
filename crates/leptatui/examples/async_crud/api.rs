//! In-memory asynchronous ticket API.

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use super::model::{MutationResult, Ticket, TicketMutation, TicketStatus};

/// Result type returned by the mock API.
pub(super) type ApiResult<T> = std::result::Result<T, String>;

/// In-memory mock API used by the demo.
#[derive(Clone)]
pub(super) struct MockApi {
    /// Shared mutable state for mock endpoint calls.
    inner: Arc<Mutex<MockState>>,
}

/// Mutable state behind the mock API.
#[derive(Debug)]
struct MockState {
    /// Tickets returned by the list endpoint.
    tickets: Vec<Ticket>,
    /// Identifier assigned to the next generated ticket.
    next_id: u64,
    /// Whether the next list call should fail.
    fail_next_list: bool,
}

impl MockApi {
    /// Creates a mock API with seed tickets.
    ///
    /// # Returns
    ///
    /// A [`MockApi`] initialized with two tickets.
    pub(super) fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(MockState {
                tickets: vec![
                    Ticket {
                        id: 1,
                        title: String::from("Wire resource state to the view"),
                        status: TicketStatus::Open,
                    },
                    Ticket {
                        id: 2,
                        title: String::from("Style async state labels"),
                        status: TicketStatus::Done,
                    },
                ],
                next_id: 3,
                fail_next_list: false,
            })),
        }
    }

    /// Lists the current mock tickets after an artificial delay.
    ///
    /// # Returns
    ///
    /// An [`ApiResult`] containing the ticket list on success.
    ///
    /// # Errors
    ///
    /// Returns [`String`] if the next list call has been configured to fail.
    pub(super) async fn list(&self) -> ApiResult<Vec<Ticket>> {
        tokio::time::sleep(Duration::from_millis(900)).await;

        let mut state = self.inner.lock().expect("mock api lock");
        if state.fail_next_list {
            state.fail_next_list = false;
            return Err(String::from("mock list endpoint failed"));
        }

        Ok(state.tickets.clone())
    }

    /// Applies a mock ticket mutation after an artificial delay.
    ///
    /// # Arguments
    ///
    /// * `mutation` — Mutation to apply to the mock ticket state.
    ///
    /// # Returns
    ///
    /// An [`ApiResult`] containing the mutation result on success.
    ///
    /// # Errors
    ///
    /// Returns [`String`] if the mutation is configured to fail or if no ticket
    /// exists for [`TicketMutation::ToggleFirst`].
    pub(super) async fn mutate(&self, mutation: TicketMutation) -> ApiResult<MutationResult> {
        tokio::time::sleep(Duration::from_millis(650)).await;

        let mut state = self.inner.lock().expect("mock api lock");
        match mutation {
            TicketMutation::Create => {
                let ticket = Ticket {
                    id: state.next_id,
                    title: format!("Generated ticket {}", state.next_id),
                    status: TicketStatus::Open,
                };
                state.next_id += 1;
                state.tickets.push(ticket.clone());

                Ok(MutationResult::Created(ticket))
            }
            TicketMutation::ToggleFirst => {
                let ticket = state
                    .tickets
                    .first_mut()
                    .ok_or_else(|| String::from("no ticket available to update"))?;
                ticket.status = ticket.status.toggled();

                Ok(MutationResult::Updated(ticket.clone()))
            }
            TicketMutation::Fail => Err(String::from("mock mutation endpoint failed")),
        }
    }

    /// Configures the next list call to return an error.
    pub(super) fn fail_next_list(&self) {
        self.inner.lock().expect("mock api lock").fail_next_list = true;
    }
}

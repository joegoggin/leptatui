//! Ticket domain types used by the async CRUD example.

/// Ticket record returned by the mock API.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct Ticket {
    /// Stable ticket identifier.
    pub(super) id: u64,
    /// Human-readable ticket title.
    pub(super) title: String,
    /// Current ticket status.
    pub(super) status: TicketStatus,
}

/// Status values supported by the mock ticket API.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum TicketStatus {
    /// Ticket is active and incomplete.
    Open,
    /// Ticket has been completed.
    Done,
}

impl TicketStatus {
    /// Returns the opposite status value.
    ///
    /// # Returns
    ///
    /// A [`TicketStatus`] toggled from open to done or done to open.
    pub(super) fn toggled(self) -> Self {
        match self {
            Self::Open => Self::Done,
            Self::Done => Self::Open,
        }
    }

    /// Returns the display label for this status.
    ///
    /// # Returns
    ///
    /// A static string label for the status.
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Done => "done",
        }
    }
}

/// Mutations supported by the mock ticket API.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum TicketMutation {
    /// Creates a generated ticket.
    Create,
    /// Toggles the first ticket in the current list.
    ToggleFirst,
    /// Simulates a failed mutation response.
    Fail,
}

impl TicketMutation {
    /// Returns the display label for this mutation.
    ///
    /// # Returns
    ///
    /// A static string label for the mutation.
    pub(super) fn label(&self) -> &'static str {
        match self {
            Self::Create => "create ticket",
            Self::ToggleFirst => "toggle first ticket",
            Self::Fail => "simulated mutation failure",
        }
    }
}

/// Successful result returned by a mock ticket mutation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum MutationResult {
    /// Mutation created the contained [`Ticket`].
    Created(Ticket),
    /// Mutation updated the contained [`Ticket`].
    Updated(Ticket),
}

impl MutationResult {
    /// Returns the display label for this mutation result.
    ///
    /// # Returns
    ///
    /// A [`String`] describing the created or updated ticket.
    pub(super) fn label(&self) -> String {
        match self {
            Self::Created(ticket) => format!("Created #{}", ticket.id),
            Self::Updated(ticket) => {
                format!("Updated #{} to {}", ticket.id, ticket.status.label())
            }
        }
    }
}

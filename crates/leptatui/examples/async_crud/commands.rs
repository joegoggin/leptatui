//! Shared CRUD controller state and command dispatch.

use leptatui::prelude::*;

use super::{
    api::MockApi,
    model::{MutationResult, Ticket, TicketMutation},
};

/// User commands supported by keyboard shortcuts and buttons.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum CrudCommand {
    /// Creates a generated ticket.
    Create,
    /// Toggles the first ticket in the current list.
    UpdateFirst,
    /// Reloads the ticket list.
    Reload,
    /// Fails the next ticket-list load and reloads.
    FailLoad,
    /// Dispatches a simulated failed mutation.
    FailAction,
    /// Exits the demo.
    Quit,
}

/// Context shared by the demo components and key handlers.
#[derive(Clone)]
pub(super) struct CrudContext {
    /// Mock API used for list and mutation requests.
    pub(super) api: MockApi,
    /// Resource containing the current ticket list state.
    pub(super) tickets: Resource<Result<Vec<Ticket>, String>>,
    /// Action used to run ticket mutations.
    pub(super) mutation: Action<TicketMutation, Result<MutationResult, String>>,
    /// Signal setter that increments to refresh the ticket list.
    pub(super) refresh: WriteSignal<u64>,
}

/// Handles keyboard shortcuts for the CRUD demo.
///
/// # Arguments
///
/// * `key` — Key event received by the app.
/// * `context` — Shared CRUD context used to dispatch work.
///
/// # Returns
///
/// A [`KeyControl`] indicating whether the key was handled or exits the app.
pub(super) fn handle_key_event(key: KeyEvent, context: &CrudContext) -> KeyControl {
    match key.code {
        KeyCode::Char('n') => run_crud_command(context, CrudCommand::Create).into(),
        KeyCode::Char('u') => run_crud_command(context, CrudCommand::UpdateFirst).into(),
        KeyCode::Char('r') => run_crud_command(context, CrudCommand::Reload).into(),
        KeyCode::Char('l') => run_crud_command(context, CrudCommand::FailLoad).into(),
        KeyCode::Char('e') => run_crud_command(context, CrudCommand::FailAction).into(),
        KeyCode::Char('q') => run_crud_command(context, CrudCommand::Quit).into(),
        _ => KeyControl::Pass,
    }
}

/// Runs one CRUD demo command.
///
/// # Arguments
///
/// * `context` — Shared CRUD context used by the command.
/// * `command` — Command selected by a key binding or button.
///
/// # Returns
///
/// An [`AppControl`] indicating whether the app should continue or exit.
pub(super) fn run_crud_command(context: &CrudContext, command: CrudCommand) -> AppControl {
    match command {
        CrudCommand::Create => context.mutation.dispatch(TicketMutation::Create),
        CrudCommand::UpdateFirst => context.mutation.dispatch(TicketMutation::ToggleFirst),
        CrudCommand::Reload => refresh_list(context),
        CrudCommand::FailLoad => fail_next_list(context),
        CrudCommand::FailAction => context.mutation.dispatch(TicketMutation::Fail),
        CrudCommand::Quit => return AppControl::Exit,
    }

    AppControl::Continue
}

/// Increments the refresh signal to reload tickets.
///
/// # Arguments
///
/// * `context` — Shared CRUD context containing the refresh setter.
fn refresh_list(context: &CrudContext) {
    context.refresh.update(|version| *version += 1);
}

/// Marks the next list call as failed and triggers a reload.
///
/// # Arguments
///
/// * `context` — Shared CRUD context containing the mock API and refresh setter.
fn fail_next_list(context: &CrudContext) {
    context.api.fail_next_list();
    refresh_list(context);
}

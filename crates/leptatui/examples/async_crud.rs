//! Async CRUD-style mock API demo.
//!
//! This binary demonstrates resources, actions, context, stylesheet classes,
//! and the app runner working together for a mock async terminal workflow.

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use leptatui::prelude::*;

/// Result type returned by the mock API.
type ApiResult<T> = std::result::Result<T, String>;

/// Ticket record returned by the mock API.
#[derive(Clone, Debug, Eq, PartialEq)]
struct Ticket {
    /// Stable ticket identifier.
    id: u64,
    /// Human-readable ticket title.
    title: String,
    /// Current ticket status.
    status: TicketStatus,
}

/// Status values supported by the mock ticket API.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TicketStatus {
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
    fn toggled(self) -> Self {
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
    fn label(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Done => "done",
        }
    }
}

/// Mutations supported by the mock ticket API.
#[derive(Clone, Debug, Eq, PartialEq)]
enum TicketMutation {
    /// Creates a generated ticket.
    Create,
    /// Toggles the first ticket in the current list.
    ToggleFirst,
    /// Simulates a failed mutation response.
    Fail,
}

/// User commands supported by keyboard shortcuts and buttons.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CrudCommand {
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

impl TicketMutation {
    /// Returns the display label for this mutation.
    ///
    /// # Returns
    ///
    /// A static string label for the mutation.
    fn label(&self) -> &'static str {
        match self {
            Self::Create => "create ticket",
            Self::ToggleFirst => "toggle first ticket",
            Self::Fail => "simulated mutation failure",
        }
    }
}

/// Successful result returned by a mock ticket mutation.
#[derive(Clone, Debug, Eq, PartialEq)]
enum MutationResult {
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
    fn label(&self) -> String {
        match self {
            Self::Created(ticket) => format!("Created #{}", ticket.id),
            Self::Updated(ticket) => {
                format!("Updated #{} to {}", ticket.id, ticket.status.label())
            }
        }
    }
}

/// In-memory mock API used by the demo.
#[derive(Clone)]
struct MockApi {
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
    fn new() -> Self {
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
    async fn list(&self) -> ApiResult<Vec<Ticket>> {
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
    async fn mutate(&self, mutation: TicketMutation) -> ApiResult<MutationResult> {
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
    fn fail_next_list(&self) {
        self.inner.lock().expect("mock api lock").fail_next_list = true;
    }
}

/// Context shared by the demo components and key handlers.
#[derive(Clone)]
struct CrudContext {
    /// Mock API used for list and mutation requests.
    api: MockApi,
    /// Resource containing the current ticket list state.
    tickets: Resource<Vec<Ticket>, String>,
    /// Action used to run ticket mutations.
    mutation: Action<TicketMutation, MutationResult, String>,
    /// Signal setter that increments to refresh the ticket list.
    refresh: WriteSignal<u64>,
}

/// Renders the async CRUD demo.
///
/// # Returns
///
/// A [`View`] containing the ticket list, mutation status, controls, and help text.
#[component]
fn AsyncCrudDemo() -> View {
    let api = MockApi::new();
    let (refresh, set_refresh) = signal(0_u64);

    let api_for_resource = api.clone();
    let tickets = create_resource(
        move || refresh.get(),
        move |_| {
            let api = api_for_resource.clone();

            async move { api.list().await }
        },
    );

    let api_for_action = api.clone();
    let refresh_after_action = set_refresh;
    let mutation = create_action(move |mutation: TicketMutation| {
        let api = api_for_action.clone();
        let refresh_after_action = refresh_after_action;

        async move {
            let result = api.mutate(mutation).await?;
            refresh_after_action.update(|version| *version += 1);
            Ok(result)
        }
    });

    let context = CrudContext {
        api,
        tickets,
        mutation,
        refresh: set_refresh,
    };

    provide_context(context.clone());

    let context_for_keys = context.clone();
    use_key_event(KeyEventKind::Press, move |key| {
        handle_key_event(key, &context_for_keys)
    });

    stylesheet! {
        .crud-screen => {
            fg: Color::White,
            bg: Color::Black,
            border_type: BorderType::Rounded,
            padding: TuiSpacing::uniform(1)
        }

        .crud-title => { fg: Color::LightCyan, modifier: Modifier::BOLD }
        .section-title => { fg: Color::Yellow, modifier: Modifier::BOLD }
        .ready => { fg: Color::LightGreen }
        .pending => { fg: Color::Yellow, modifier: Modifier::BOLD }
        .success => { fg: Color::LightGreen, modifier: Modifier::BOLD }
        .error => { fg: Color::LightRed, modifier: Modifier::BOLD }
        .danger => { fg: Color::LightRed }
        .muted => { fg: Color::Gray }
        .ticket-open => { fg: Color::White }
        .ticket-done => { fg: Color::DarkGray }
        .crud-actions => { direction: LayoutDirection::Row }

        Button => {
            fg: Color::White,
            bg: Color::Black,
            borders: Borders::ALL,
            border_type: BorderType::Rounded,
            padding: TuiSpacing::horizontal(1)
        }

        Button:focus => {
            fg: Color::Black,
            bg: Color::Yellow,
            modifier: Modifier::BOLD,
            border_type: BorderType::Thick
        }

        @media (max-width: 60) {
            .crud-screen => { padding: TuiSpacing::ZERO }
            .crud-actions => { direction: LayoutDirection::Column }
            Button => { padding: TuiSpacing::ZERO }
        }
    }

    view! {
        <Block class="crud-screen">
            <Column>
                <Text class="crud-title">"Async CRUD mock API"</Text>
                <TicketList />
                <MutationStatus />
                <CrudControls />
                <Text class="muted">
                    "n create | u update first | r reload | l fail load | e fail action | q quit"
                </Text>
            </Column>
        </Block>
    }
}

/// Renders the current ticket-list resource state.
///
/// # Returns
///
/// A [`View`] showing pending, ready, or error content for the ticket list.
#[component]
fn TicketList() -> View {
    let context = expect_context::<CrudContext>();

    dynamic(move || match context.tickets.get_untracked() {
        ResourceState::Pending => text("Loading tickets from mock API...").with_classes("pending"),
        ResourceState::Ready(tickets) => render_ticket_list(tickets),
        ResourceState::Error(error) => text(format!("Load error: {error}")).with_classes("error"),
    })
}

/// Renders the current mutation action state.
///
/// # Returns
///
/// A [`View`] showing idle, pending, success, or error mutation status.
#[component]
fn MutationStatus() -> View {
    let context = expect_context::<CrudContext>();

    dynamic(move || render_mutation_status(context.mutation.get_untracked()))
}

/// Renders button controls for demo mutations and reloads.
///
/// # Returns
///
/// A [`View`] containing the CRUD control row.
#[component]
fn CrudControls() -> View {
    let context = expect_context::<CrudContext>();

    let create = context.clone();
    let update = context.clone();
    let reload = context.clone();
    let fail_list = context.clone();
    let fail_action = context.clone();

    view! {
        <Row class="crud-actions">
            <Button on_press=move || {
                run_crud_command(&create, CrudCommand::Create)
            }>
                "Create"
            </Button>
            <Button on_press=move || {
                run_crud_command(&update, CrudCommand::UpdateFirst)
            }>
                "Update first"
            </Button>
            <Button on_press=move || {
                run_crud_command(&reload, CrudCommand::Reload)
            }>
                "Reload"
            </Button>
            <Button on_press=move || {
                run_crud_command(&fail_list, CrudCommand::FailLoad)
            }>
                "Fail load"
            </Button>
            <Button class="danger" on_press=move || {
                run_crud_command(&fail_action, CrudCommand::FailAction)
            }>
                "Fail action"
            </Button>
            <Button class="danger" on_press=move || {
                run_crud_command(&context, CrudCommand::Quit)
            }>
                "Quit"
            </Button>
        </Row>
    }
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
fn handle_key_event(key: KeyEvent, context: &CrudContext) -> KeyControl {
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
fn run_crud_command(context: &CrudContext, command: CrudCommand) -> AppControl {
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

/// Renders a ticket-list view.
///
/// # Arguments
///
/// * `tickets` — Tickets returned by the mock API.
///
/// # Returns
///
/// A [`View`] containing a section header and ticket rows.
fn render_ticket_list(tickets: Vec<Ticket>) -> View {
    let mut rows = Vec::with_capacity(tickets.len() + 1);
    rows.push(text(format!("Tickets ({})", tickets.len())).with_classes("section-title ready"));

    if tickets.is_empty() {
        rows.push(text("No tickets returned").with_classes("muted"));
    } else {
        rows.extend(tickets.into_iter().map(render_ticket));
    }

    column(rows).with_classes("ticket-list")
}

/// Renders one ticket row.
///
/// # Arguments
///
/// * `ticket` — Ticket to display.
///
/// # Returns
///
/// A [`View`] containing the ticket id, status, and title.
fn render_ticket(ticket: Ticket) -> View {
    let class = match ticket.status {
        TicketStatus::Open => "ticket-open",
        TicketStatus::Done => "ticket-done",
    };

    text(format!(
        "#{} [{}] {}",
        ticket.id,
        ticket.status.label(),
        ticket.title
    ))
    .with_classes(class)
}

/// Renders mutation action status.
///
/// # Arguments
///
/// * `state` — Current mutation action state.
///
/// # Returns
///
/// A [`View`] describing the current mutation state.
fn render_mutation_status(state: ActionState<TicketMutation, MutationResult, String>) -> View {
    if state.is_pending() {
        let label = state
            .input()
            .map(TicketMutation::label)
            .unwrap_or("mutation");

        return text(format!("Mutation pending: {label}")).with_classes("pending");
    }

    match state.result() {
        Some(Ok(result)) => {
            text(format!("Last mutation: {}", result.label())).with_classes("success")
        }
        Some(Err(error)) => text(format!("Mutation error: {error}")).with_classes("error"),
        None => text("Mutation idle").with_classes("muted"),
    }
}

/// Runs the async CRUD example.
///
/// # Returns
///
/// An empty [`Result`] when the app exits successfully.
///
/// # Errors
///
/// Returns [`Error::Io`] if terminal setup, rendering, input, or cleanup fails.
#[tokio::main]
async fn main() -> Result<()> {
    App::new(AsyncCrudDemo::new()).run().await
}

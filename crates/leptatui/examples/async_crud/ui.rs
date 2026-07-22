//! Reactive components, styles, and render helpers.

use leptatui::prelude::*;

use super::{
    api::MockApi,
    commands::{CrudCommand, CrudContext, handle_key_event, run_crud_command},
    model::{MutationResult, Ticket, TicketMutation, TicketStatus},
};

/// Renders the async CRUD demo.
///
/// # Returns
///
/// A [`View`] containing the ticket list, mutation status, controls, and help text.
#[component]
pub(super) fn AsyncCrudDemo() -> impl IntoView {
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
fn TicketList() -> impl IntoView {
    let context = expect_context::<CrudContext>();

    dynamic(move || match context.tickets.get_untracked() {
        ResourceState::Pending => text("Loading tickets from mock API...")
            .with_classes("pending")
            .into_view(),
        ResourceState::Ready(tickets) => render_ticket_list(tickets),
        ResourceState::Error(error) => text(format!("Load error: {error}"))
            .with_classes("error")
            .into_view(),
    })
}

/// Renders the current mutation action state.
///
/// # Returns
///
/// A [`View`] showing idle, pending, success, or error mutation status.
#[component]
fn MutationStatus() -> impl IntoView {
    let context = expect_context::<CrudContext>();

    dynamic(move || render_mutation_status(context.mutation.get_untracked()))
}

/// Renders button controls for demo mutations and reloads.
///
/// # Returns
///
/// A [`View`] containing the CRUD control row.
#[component]
fn CrudControls() -> impl IntoView {
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

/// Renders a ticket-list view.
///
/// # Arguments
///
/// * `tickets` — Tickets returned by the mock API.
///
/// # Returns
///
/// A [`View`] containing a section header and ticket rows.
fn render_ticket_list(tickets: Vec<Ticket>) -> AnyView {
    let mut rows = Vec::with_capacity(tickets.len() + 1);
    rows.push(
        text(format!("Tickets ({})", tickets.len()))
            .with_classes("section-title ready")
            .into_view(),
    );

    if tickets.is_empty() {
        rows.push(
            text("No tickets returned")
                .with_classes("muted")
                .into_view(),
        );
    } else {
        rows.extend(tickets.into_iter().map(render_ticket));
    }

    column(rows).with_classes("ticket-list").into_view()
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
fn render_ticket(ticket: Ticket) -> AnyView {
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
    .into_view()
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
fn render_mutation_status(state: ActionState<TicketMutation, MutationResult, String>) -> AnyView {
    if state.is_pending() {
        let label = state
            .input()
            .map(TicketMutation::label)
            .unwrap_or("mutation");

        return text(format!("Mutation pending: {label}"))
            .with_classes("pending")
            .into_view();
    }

    match state.result() {
        Some(Ok(result)) => text(format!("Last mutation: {}", result.label()))
            .with_classes("success")
            .into_view(),
        Some(Err(error)) => text(format!("Mutation error: {error}"))
            .with_classes("error")
            .into_view(),
        None => text("Mutation idle").with_classes("muted").into_view(),
    }
}

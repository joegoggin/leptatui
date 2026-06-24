//! Async CRUD-style mock API demo.
//!
//! This binary demonstrates resources, actions, context, stylesheet classes,
//! and the app runner working together for a mock async terminal workflow.

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use leptatui::prelude::*;

type ApiResult<T> = std::result::Result<T, String>;

#[derive(Clone, Debug, Eq, PartialEq)]
struct Ticket {
    id: u64,
    title: String,
    status: TicketStatus,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TicketStatus {
    Open,
    Done,
}

impl TicketStatus {
    fn toggled(self) -> Self {
        match self {
            Self::Open => Self::Done,
            Self::Done => Self::Open,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Done => "done",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum TicketMutation {
    Create,
    ToggleFirst,
    Fail,
}

impl TicketMutation {
    fn label(&self) -> &'static str {
        match self {
            Self::Create => "create ticket",
            Self::ToggleFirst => "toggle first ticket",
            Self::Fail => "simulated mutation failure",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum MutationResult {
    Created(Ticket),
    Updated(Ticket),
}

impl MutationResult {
    fn label(&self) -> String {
        match self {
            Self::Created(ticket) => format!("Created #{}", ticket.id),
            Self::Updated(ticket) => {
                format!("Updated #{} to {}", ticket.id, ticket.status.label())
            }
        }
    }
}

#[derive(Clone)]
struct MockApi {
    inner: Arc<Mutex<MockState>>,
}

#[derive(Debug)]
struct MockState {
    tickets: Vec<Ticket>,
    next_id: u64,
    fail_next_list: bool,
}

impl MockApi {
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

    async fn list(&self) -> ApiResult<Vec<Ticket>> {
        tokio::time::sleep(Duration::from_millis(900)).await;

        let mut state = self.inner.lock().expect("mock api lock");
        if state.fail_next_list {
            state.fail_next_list = false;
            return Err(String::from("mock list endpoint failed"));
        }

        Ok(state.tickets.clone())
    }

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

    fn fail_next_list(&self) {
        self.inner.lock().expect("mock api lock").fail_next_list = true;
    }
}

#[derive(Clone)]
struct CrudContext {
    api: MockApi,
    tickets: Resource<Vec<Ticket>, String>,
    mutation: Action<TicketMutation, MutationResult, String>,
    refresh: WriteSignal<u64>,
}

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

#[component]
fn TicketList() -> View {
    let context = expect_context::<CrudContext>();

    dynamic(move || match context.tickets.get_untracked() {
        ResourceState::Pending => {
            text("Loading tickets from mock API...").with_classes("pending")
        }
        ResourceState::Ready(tickets) => render_ticket_list(tickets),
        ResourceState::Error(error) => {
            text(format!("Load error: {error}")).with_classes("error")
        }
    })
}

#[component]
fn MutationStatus() -> View {
    let context = expect_context::<CrudContext>();

    dynamic(move || render_mutation_status(context.mutation.get_untracked()))
}

#[component]
fn CrudControls() -> View {
    let context = expect_context::<CrudContext>();

    let create = context.mutation.clone();
    let update = context.mutation.clone();
    let fail_action = context.mutation.clone();
    let reload = context.clone();
    let fail_list = context.clone();

    view! {
        <Row class="crud-actions">
            <Button on_press=move || {
                create.dispatch(TicketMutation::Create);
                AppControl::Continue
            }>
                "Create"
            </Button>
            <Button on_press=move || {
                update.dispatch(TicketMutation::ToggleFirst);
                AppControl::Continue
            }>
                "Update first"
            </Button>
            <Button on_press=move || {
                refresh_list(&reload);
                AppControl::Continue
            }>
                "Reload"
            </Button>
            <Button on_press=move || {
                fail_next_list(&fail_list);
                AppControl::Continue
            }>
                "Fail load"
            </Button>
            <Button class="danger" on_press=move || {
                fail_action.dispatch(TicketMutation::Fail);
                AppControl::Continue
            }>
                "Fail action"
            </Button>
            <Button class="danger" on_press=|| AppControl::Exit>
                "Quit"
            </Button>
        </Row>
    }
}

fn handle_key_event(key: KeyEvent, context: &CrudContext) -> KeyControl {
    match key.code {
        KeyCode::Char('n') => {
            context.mutation.dispatch(TicketMutation::Create);
            KeyControl::Handled
        }
        KeyCode::Char('u') => {
            context.mutation.dispatch(TicketMutation::ToggleFirst);
            KeyControl::Handled
        }
        KeyCode::Char('r') => {
            refresh_list(context);
            KeyControl::Handled
        }
        KeyCode::Char('l') => {
            fail_next_list(context);
            KeyControl::Handled
        }
        KeyCode::Char('e') => {
            context.mutation.dispatch(TicketMutation::Fail);
            KeyControl::Handled
        }
        KeyCode::Char('q') => KeyControl::Exit,
        _ => KeyControl::Pass,
    }
}

fn refresh_list(context: &CrudContext) {
    context.refresh.update(|version| *version += 1);
}

fn fail_next_list(context: &CrudContext) {
    context.api.fail_next_list();
    refresh_list(context);
}

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

#[tokio::main]
async fn main() -> Result<()> {
    App::new(AsyncCrudDemo::new()).run().await
}

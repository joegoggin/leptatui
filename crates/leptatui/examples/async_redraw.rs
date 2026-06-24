//! Async redraw example.
//!
//! This binary demonstrates resource and action completions waking the app
//! runner without requiring key input.

use std::time::Duration;

use leptatui::prelude::*;

#[component]
fn AsyncRedraw() -> View {
    let request = RwSignal::new(0usize);

    let resource = create_resource(
        move || request.get(),
        |request| async move {
            tokio::time::sleep(Duration::from_secs(2)).await;

            if request % 2 == 0 {
                Ok(format!("loaded request {request}"))
            } else {
                Err("simulated resource error")
            }
        },
    );

    let action = create_action(|request: usize| async move {
        tokio::time::sleep(Duration::from_millis(900)).await;

        if request % 2 == 0 {
            Ok(format!("saved request {request}"))
        } else {
            Err("simulated action error")
        }
    });

    let action_for_keys = action.clone();

    use_key_event(KeyEventKind::Press, move |key| match key.code {
        KeyCode::Char('r') => {
            request.update(|request| *request += 1);
            KeyControl::Handled
        }
        KeyCode::Char('a') => {
            action_for_keys.dispatch(request.get_untracked());
            KeyControl::Handled
        }
        KeyCode::Char('q') => KeyControl::Exit,
        _ => KeyControl::Pass,
    });

    stylesheet! {
        .panel => {
            border_type: BorderType::Rounded,
            padding: TuiSpacing::uniform(1)
        }
        .title => { fg: Color::LightCyan, modifier: Modifier::BOLD }
        .help => { fg: Color::Gray }
        .error => { fg: Color::LightRed }
        .ready => { fg: Color::LightGreen }

        @media (max-width: 60) {
            .panel => { padding: TuiSpacing::ZERO }
        }
    }

    view! {
        <Block class="async-panel">
            <Column>
                <Text class="async-title">"Async redraw"</Text>
                {move || {
                    let label = match resource.get_untracked() {
                        ResourceState::Pending => String::from("Resource: pending"),
                        ResourceState::Ready(value) => format!("Resource: {value}"),
                        ResourceState::Error(error) => format!("Resource: {error}"),
                    };

                    view! { <Text class="async-ready">{label}</Text> }
                }}
                {move || {
                    let state = action.get_untracked();
                    let label = if state.is_pending() {
                        String::from("Action: pending")
                    } else {
                        match state.result() {
                            Some(Ok(value)) => format!("Action: {value}"),
                            Some(Err(error)) => format!("Action: {error}"),
                            None => String::from("Action: idle"),
                        }
                    };

                    view! { <Text class="async-ready">{label}</Text> }
                }}
                <Text class="async-help">"r reloads resource. a dispatches action. q quits."</Text>
            </Column>
        </Block>
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    App::new(AsyncRedraw::new()).run().await
}

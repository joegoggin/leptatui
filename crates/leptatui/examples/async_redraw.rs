//! Async redraw example.
//!
//! This binary demonstrates resource and action completions waking the app
//! runner without requiring key input.

use std::time::Duration;

use leptatui::prelude::*;

/// Renders resource and action state that update after async completions.
///
/// # Returns
///
/// A [`View`] containing resource state, action state, and keyboard help.
#[component]
fn AsyncRedraw() -> impl IntoView {
    let request = RwSignal::new(0usize);

    let resource = create_resource(
        move || request.get(),
        |request| async move {
            tokio::time::sleep(Duration::from_secs(2)).await;

            if request.is_multiple_of(2) {
                Ok(format!("loaded request {request}"))
            } else {
                Err("simulated resource error")
            }
        },
    );

    let action = create_action(|request: usize| async move {
        tokio::time::sleep(Duration::from_millis(900)).await;

        if request.is_multiple_of(2) {
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
        <Block class="panel">
            <Div>
                <Text class="title">"Async redraw"</Text>
                {move || {
                    let (label, class) = match resource.get_untracked() {
                        ResourceState::Pending => (String::from("Resource: pending"), "ready"),
                        ResourceState::Ready(value) => (format!("Resource: {value}"), "ready"),
                        ResourceState::Error(error) => (format!("Resource: {error}"), "error"),
                    };

                    view! { <Text class={class}>{label}</Text> }
                }}
                {move || {
                    let state = action.get_untracked();
                    let (label, class) = if state.is_pending() {
                        (String::from("Action: pending"), "ready")
                    } else {
                        match state.result() {
                            Some(Ok(value)) => (format!("Action: {value}"), "ready"),
                            Some(Err(error)) => (format!("Action: {error}"), "error"),
                            None => (String::from("Action: idle"), "ready"),
                        }
                    };

                    view! { <Text class={class}>{label}</Text> }
                }}
                <Text class="help">"r reloads resource. a dispatches action. q quits."</Text>
            </Div>
        </Block>
    }
}

/// Runs the async redraw example.
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
    let view = view! { <AsyncRedraw /> };
    App::new(view).run().await
}

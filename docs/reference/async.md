# Async reference

Start with [Async data](../guide/09-async-data.md).

Use `Resource::new` for asynchronous reads keyed by reactive source state and
`Action::new` for POST-like mutations triggered by buttons or key handlers.
Both types publish signal-backed loading, input, and value state and request
redraws when work changes. Fetchers and handlers can return any output type;
application failures remain ordinary `Result<T, E>` values handled by
components or contexts.

Resources retain their previous value while reloading and ignore stale
completions from older source keys. Actions retain their previous output,
expose the current input only while pending, and ignore stale completions. A
common pattern is to keep a refresh signal in root context, key the resource
from that signal, and increment it after a successful action.

```rust
use leptatui::prelude::*;

#[derive(Clone)]
struct Todos {
    items: Resource<Result<Vec<String>, String>>,
    create: Action<String, Result<String, String>>,
    refresh: WriteSignal<u64>,
}

async fn load_todos() -> std::result::Result<Vec<String>, String> {
    Ok(vec![String::from("Write terminal docs")])
}

async fn create_todo(title: String) -> std::result::Result<String, String> {
    Ok(title)
}

#[component]
fn TodoApp() -> impl IntoView {
    let (refresh, set_refresh) = signal(0_u64);

    let items = Resource::new(
        move || refresh.get(),
        |_| async move { load_todos().await },
    );

    let create = Action::new(move |title: &String| {
        let title = title.clone();
        let set_refresh = set_refresh;

        async move {
            let saved = create_todo(title).await?;
            set_refresh.update(|version| *version += 1);
            Ok(saved)
        }
    });

    provide_context(Todos {
        items,
        create,
        refresh: set_refresh,
    });

    view! {
        <Div>
            <TodoList />
            <TodoActions />
        </Div>
    }
}

#[component]
fn TodoList() -> impl IntoView {
    let todos = expect_context::<Todos>();

    dynamic(move || match todos.items.get() {
        Some(Ok(items)) => div(items.into_iter().map(text).collect::<Vec<_>>()),
        Some(Err(error)) => text(format!("Load failed: {error}")),
        None => text("Loading todos..."),
    })
}

#[component]
fn TodoActions() -> impl IntoView {
    let todos = expect_context::<Todos>();
    let create = todos.create.clone();
    let reload = todos.refresh;

    view! {
        <Div style={TuiStyle::new().display(Display::Flex)}>
            <Button on_press=move || {
                create.dispatch(String::from("Review async state"));
                AppControl::Continue
            }>
                "Create"
            </Button>
            <Button on_press=move || {
                reload.update(|version| *version += 1);
                AppControl::Continue
            }>
                "Reload"
            </Button>
        </Div>
    }
}
```

See `cargo run --example async_redraw` for a small async redraw example and
`cargo run --example async_crud` for resources, actions, context, stylesheets,
and app startup working together.

[Documentation index](../README.md) · [Feature index](features.md)

# 9. Async data

Prerequisites: [Chapter 8: Handling failures](08-handling-failures.md).

Add a mock service without requiring a network, account, or backend. Start
from the [complete async checkpoint](../checkpoints/09-async-notes.md), which
keeps the routed editor and adds a data toolbar to the shell.

## Read with a resource

A resource runs an async fetcher when its reactive source changes or when you
call `refetch`. The checkpoint creates one during component setup:

```rust
let loaded = Resource::new(
    || (),
    |_| async {
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        Ok::<_, String>(String::from("A note from the mock service"))
    },
);
```

`get()` returns `None` before completion, then `Some(output)`. The output can
itself be `Ok(value)` or `Err(error)`. `is_loading()` is separate from that
value: reloading retains the previous result. Older requests cannot overwrite
the result of a newer request.

The toolbar deliberately requires **Use loaded note** before replacing the
draft. A background read should not unexpectedly erase text the user is typing.
**Reload** fetches again; it does not apply the result automatically.

## Write with an action

An action starts only when you dispatch an input. Clone borrowed input before
moving it into its future:

```rust
let save = Action::new(|body: &String| {
    let body = body.clone();
    async move {
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        if body.trim().is_empty() {
            Err(String::from("Write something before saving"))
        } else {
            Ok(format!("Accepted {} bytes", body.len()))
        }
    }
});
```

`dispatch(body.get_untracked())` takes a snapshot. `is_pending()` reports work
in progress, `input()` is present while pending, `value()` retains the latest
output, and `version()` records completions. `clear()` clears action state.
Actions also ignore stale completions. If you want to refresh a resource after
a successful mutation, call `refetch()` or update the signal used as its source.

The mock save acknowledges the text but does not persist it. The next chapter
replaces it with real file operations. Start the app, wait for loading to finish
without pressing a key, apply the loaded note, then save. Clear the body and
save again to see a local error while the editor stays usable.

For work outside resources/actions, `spawn` schedules a `Send` future and
`spawn_local` supports local futures. Create async work under the running Tokio
app. `spawn_local` requires the managed app's local task set (or an explicit
Tokio local task set). Prefer resource/action handles when the UI needs pending and result
state. See the [async reference](../reference/async.md) and run
`cargo run --example async_crud` for refetch-after-mutation behavior.

[Previous: Handling failures](08-handling-failures.md) · [Documentation index](../README.md) · [Next: Files and editors](10-files-and-editors.md)

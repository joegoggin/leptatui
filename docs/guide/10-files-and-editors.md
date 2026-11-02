# 10. Files and editors

Prerequisites: [Chapter 9: Async data](09-async-data.md).

Replace the mock service with real note files. The
[complete file-backed checkpoint](../checkpoints/10-file-notes.md) keeps the
same editor and routes, and replaces the data toolbar with load/save controls.
It stores the body as `notes-data/note.md`, relative to the notes project's
working directory. The title remains in memory.

## Scope file operations to a directory

`use_file_system` initializes a canonical directory boundary and can fail, so
the toolbar returns `ViewResult<impl IntoView>`. Use options to create a missing
root deliberately:

```rust
let files = use_file_system_with_options(
    "notes-data",
    FileSystemOptions::new().create_root(true),
)?;
let latest_read = RwSignal::new(None::<FileOperation<String>>);
let latest_write = RwSignal::new(None::<FileOperation<()>>);
```

Each operation starts immediately and returns a reactive handle. Keep it to
show loading and results:

```rust
latest_read.set(Some(files.read_file_as_string("note.md")));
latest_write.set(Some(files.write_and_replace_file("note.md", "# My note
")));
```

A `FileOperation<T>` is an `Action<(), std::io::Result<T>>`. Observe pending,
value, and version state; `dispatch(())` retries the same captured arguments.
Retain separate handles if both read and write status need to stay visible.

Run the checkpoint, edit the body, activate **Save body**, then **Load body**.
Choose **Use loaded note** to apply the read result. Restart the app and load
again to verify persistence. Loading before saving displays a missing-file
error locally; it does not erase the draft.

Relative paths resolve inside the root. Absolute paths and symlink targets
must remain inside it. Leading `~` expands to the user's home and still obeys
the boundary. Recursive root deletion is prohibited. The
[filesystem reference](../reference/files-and-editors.md) and
`cargo run --example file_system_showcase` cover path resolution, metadata,
directory listings, binary/text reads, directory creation, writes, append,
atomic replacement, copy, rename, deletion, retries, and expected failures.

## Choose a file

The selector owns its own full-screen interface. In a component body, create
`let selector = use_file_selector();`, clone it for a button callback, and call:

```rust
selector.select_with_options(
    FileSelectorOptions::new()
        .starting_directory("notes-data")
        .extensions(["md", "markdown"]),
);
```

Read `selector.get_file()` inside a tracked closure or effect to observe a
successful canonical path selection. `select()` uses the current directory
without extension filtering; `clear()` removes the saved selection. Opening or
canceling preserves the previous selection. Enter selects, Esc or Ctrl+C
cancels, and `q` exits the application. A picked path is not automatically
inside your filesystem root: use an appropriate root before operating on it.

## Open an external editor

Create `let editor = use_editor();` during component setup. A button callback
can call `editor.edit_text(body)` to edit the signal through a temporary
Markdown file, or `editor.edit_file("notes-data/note.md")` to edit a file.
The editor is chosen from `VISUAL`, then `EDITOR`, then `vi`. For example,
set `VISUAL="nvim -f"` before launching your app.

The runtime restores the terminal, waits for the editor, and resumes the same
mounted UI. Observe `editor.status()` for `Pending`, `Complete`, or `Error` and
call `clear()` to clear a completed status. A failed text edit preserves the
signal. A successful file edit does not automatically update your draft;
load the file again explicitly.

For another synchronous terminal integration, call
`use_app_handle().suspend_terminal(move || { /* synchronous work */ })` from
an event callback after obtaining the handle during setup. Work runs on the
app thread while the terminal is restored; do not run it directly during
normal rendering. Run `cargo run --example external_editor` or explore the
[Markdown editor application](../../examples/markdown-editor/README.md).

[Previous: Async data](09-async-data.md) · [Documentation index](../README.md) · [Next: Documents and media](11-documents-and-media.md)

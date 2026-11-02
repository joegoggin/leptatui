# Files and editors reference

Start with [Files and editors](../guide/10-files-and-editors.md).

## On this page

- [External Editor](#external-editor)
- [File Selector](#file-selector)
- [Root-Scoped File System](#root-scoped-file-system)
- [Deferred capabilities](#deferred-capabilities)

## External Editor

`use_editor()` returns a cloneable handle for opening an existing file or
round-tripping a reactive string through a temporary Markdown file. Leptatui
temporarily restores normal terminal modes, waits for the configured editor to
exit, re-enters the TUI, and redraws the mounted component tree.

The editor command comes from the first non-empty `VISUAL` or `EDITOR` value
and falls back to `vi`. Values may contain shell-word quoted arguments, but the
command runs directly without shell expansion. The handle exposes one shared,
reactive `EditorStatus` that moves from `Pending` to `Complete` or `Error`.
Call `clear()` after consuming a terminal status to return it to `None`.

```rust,no_run
use leptatui::prelude::*;

#[component]
fn EditDraft() -> impl IntoView {
    let editor = use_editor();
    let status_editor = editor.clone();
    let draft = RwSignal::new(String::from("# Draft\n"));

    use_key_event(KeyEventKind::Press, move |key| match key.code {
        KeyCode::Char('e') => {
            editor.edit_text(draft);
            KeyControl::Handled
        }
        KeyCode::Char('q') => KeyControl::Exit,
        _ => KeyControl::Pass,
    });

    view! {
        <Div>
            <Text>{draft}</Text>
            <Text>{move || match status_editor.status() {
                None => String::from("Editor idle"),
                Some(EditorStatus::Pending) => String::from("Editor open"),
                Some(EditorStatus::Complete) => String::from("Edit complete"),
                Some(EditorStatus::Error(error)) => format!("Edit failed: {error}"),
            }}</Text>
        </Div>
    }
}
```

`edit_file(path)` passes the supplied path to the editor. `edit_text(signal)`
writes the signal snapshot to a uniquely named `.md` file, applies valid UTF-8
contents only after a successful editor exit, and removes the temporary file
before reporting completion. Failures preserve the original signal value. All
clones of an `Editor` observe the same status, and if multiple edits are queued
before the terminal is suspended, the last completed edit determines the final
status.

Run `cargo run --example external_editor` to edit the in-memory draft. Supply
an optional file with `cargo run --example external_editor -- path/to/file` to
demonstrate both methods.

## File Selector

`use_file_selector()` returns a cloneable handle for Leptatui's
standalone file picker. The picker temporarily owns the complete terminal and
event stream, so application styles and layouts cannot affect it. `Enter`
selects the highlighted file, `Esc` or `Ctrl-C` cancels without changing the
previously stored path, and `q` exits the application.

```rust,no_run
use leptatui::prelude::*;

#[component]
fn OpenDocument() -> impl IntoView {
    let selector = use_file_selector();
    let button_selector = selector.clone();
    let navigate = use_navigate();

    Effect::new(move || {
        if let Some(path) = selector.get_file() {
            navigate(
                &format!("/documents/{}", path.display()),
                NavigateOptions::default(),
            );
        }
    });

    view! {
        <Div>
            <Button on_press=move || {
                button_selector.select_with_options(
                    FileSelectorOptions::new()
                        .starting_directory("docs")
                        .extensions(["md", "markdown"]),
                );
                AppControl::Continue
            }>
                "Select a file"
            </Button>
        </Div>
    }
}
```

`select()` starts in the process current directory and accepts any regular
file. `select_with_options()` can choose the initial directory, extension
allowlist, and initial hidden-file visibility. A successful selection replaces
the stored canonical `PathBuf`; `get_file()` reactively reads it and `clear()`
removes it explicitly. Reading it inside an `Effect` reruns that effect after a
selection or clear. Opening or cancelling never clears an existing value.

## Root-Scoped File System

`use_file_system()` canonicalizes a directory boundary and returns a cloneable
handle whose methods start blocking I/O outside the terminal event thread.
Call it directly inside any component that needs filesystem access rather than
passing a handle through the component tree. Relative paths resolve below that
root. Absolute paths and symlink
targets are accepted only when their canonical targets remain contained.
An exact leading `~` or `~/` expands to the current user's home directory for
both root initialization and every action path; named-user forms such as
`~alice` remain literal. Expanded paths still must remain within the configured
root.

```rust,no_run
use leptatui::prelude::*;

#[component]
fn ConfigDirButton() -> ViewResult<impl IntoView> {
    let files = use_file_system("~/.config")?;

    view! {
        <Button on_press=move || {
            // Ignoring the handle is valid for fire-and-forget work.
            files.create_dir("my-app/cache");
            AppControl::Continue
        }>
            "Create config directory"
        </Button>
    }
}

#[component]
fn ProjectsDirButton() -> ViewResult<impl IntoView> {
    let files = use_file_system("~/Projects")?;
    let latest_read = RwSignal::new(None::<FileOperation<String>>);

    view! {
        <Button on_press=move || {
            // Retain the already-started operation to render pending/value/version.
            latest_read.set(Some(files.read_file_as_string("README.md")));
            AppControl::Continue
        }>
            "Read project guide"
        </Button>
    }
}
```

Every operation method returns an already-dispatched `FileOperation<T>`, an
alias for `Action<(), std::io::Result<T>>`. Keep it to observe reactive state,
call `dispatch(())` to retry the same captured arguments, or ignore it when no
result is needed.

The API includes path resolution, metadata, deterministic directory listings,
binary and UTF-8 reads, recursive directory creation, truncating, appending,
and atomic writes, non-overwriting copy and rename, and file or recursive
directory deletion. `rename()` supports files, directories, and symbolic links.
Missing roots remain an error unless `FileSystemOptions::create_root(true)` is
selected, and recursive deletion can never target the scoped root itself.

Run `cargo run --example file_system_showcase` for a guided demonstration that
executes every operation in dependency order, retains reactive results, and
offers a separate expected-failure tour.

## Deferred capabilities

Filesystem watching, recursive walking and glob matching, streaming, configurable open options, permission and symlink management, and platform directory discovery are follow-up capabilities, not current filesystem APIs.

[Documentation index](../README.md) · [Feature index](features.md)

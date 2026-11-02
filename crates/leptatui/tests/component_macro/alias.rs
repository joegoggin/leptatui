/// Verifies macros compile when the runtime crate dependency is renamed.
///
/// # Example Under Test
///
/// ```text
/// ui = { package = "leptatui", path = "..." }
/// use ui::prelude::*;
/// #[component]
/// fn Greeting() -> impl IntoView { view! { <Text>"hi"</Text> } }
/// #[derive(RouteParams)]
/// struct GreetingParams { name: String }
/// ```
///
/// # Assertions
///
/// - `cargo check` succeeds in a temporary downstream crate.
/// - The downstream crate imports only the renamed `ui` dependency.
/// - Component, view, and typed-parameter macros resolve the renamed runtime.
///
/// # Why
///
/// Generated proc-macro code should resolve the runtime crate path from the
/// caller's dependency name instead of hardcoding `::leptatui`.
#[test]
fn macros_compile_with_renamed_runtime_dependency() {
    let project_dir = create_alias_fixture();
    let output = Command::new("cargo")
        .arg("check")
        .arg("--quiet")
        .current_dir(&project_dir)
        .output()
        .expect("cargo check should run for alias fixture");

    assert!(
        output.status.success(),
        "cargo check failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Creates a temporary crate that depends on Leptatui under a renamed package key.
///
/// # Returns
///
/// A [`PathBuf`] containing the generated fixture crate directory.
fn create_alias_fixture() -> PathBuf {
    let project_dir = alias_fixture_dir();
    if project_dir.exists() {
        fs::remove_dir_all(&project_dir).expect("stale alias fixture should be removable");
    }

    fs::create_dir_all(project_dir.join("src")).expect("alias fixture src should be creatable");

    let leptatui_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .canonicalize()
        .expect("leptatui manifest directory should be canonicalizable");

    fs::write(
        project_dir.join("Cargo.toml"),
        format!(
            r#"[package]
name = "leptatui-alias-check"
version = "0.0.0"
edition = "2024"

[dependencies]
ui = {{ package = "leptatui", path = "{}" }}
"#,
            leptatui_path.display()
        ),
    )
    .expect("alias fixture manifest should be writable");

    fs::write(
        project_dir.join("src/main.rs"),
        r#"use ui::prelude::*;

#[component]
fn Greeting() -> impl IntoView {
    view! { <Text>"hi"</Text> }
}

#[derive(RouteParams)]
struct GreetingParams {
    name: String,
}

#[derive(QueryParams)]
struct GreetingQuery {
    page: Option<usize>,
}

fn main() {
    let _view: AnyView = Greeting::new().into_view();
    fn route_model<T: ui::RouteParams>() {}
    fn query_model<T: ui::QueryParams>() {}
    route_model::<GreetingParams>();
    query_model::<GreetingQuery>();
}
"#,
    )
    .expect("alias fixture source should be writable");

    project_dir
}

/// Returns a unique temporary directory for a renamed-dependency fixture crate.
///
/// # Returns
///
/// A [`PathBuf`] under Cargo's target temp directory or the system temp
/// directory.
fn alias_fixture_dir() -> PathBuf {
    let base = std::env::var_os("CARGO_TARGET_TMPDIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after the Unix epoch")
        .as_nanos();

    base.join(format!(
        "leptatui-alias-check-{}-{timestamp}",
        std::process::id()
    ))
}

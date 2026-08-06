/// View with local signal state created during generated setup.
#[component]
fn MacroSignalRoot() -> impl leptatui::IntoView {
    MACRO_SIGNAL_SETUP_RUNS.fetch_add(1, Ordering::SeqCst);
    let (count, set_count) = signal(0);
    let increment = set_count;

    use_key_event(KeyEventKind::Press, move |key| {
        if key.code == KeyCode::Char('i') {
            increment.update(|count| *count += 1);
            return KeyControl::Handled;
        }

        KeyControl::Pass
    });

    div((
        dynamic(move || text(format!("Count: {}", count.get()))),
        button("Increment").on_press(move || {
            set_count.update(|count| *count += 1);
            AppControl::Continue
        }),
    ))
}

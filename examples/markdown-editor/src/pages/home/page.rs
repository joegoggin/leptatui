//! Home route-level component and keyboard behavior.

use std::{path::PathBuf, rc::Rc};

use leptatui::prelude::*;

use crate::{contexts::use_notifications, pages::viewer_location, services::RecentFilesStore};

use super::{
    components::{RecentFilesList, RecentFilesListProps},
    style::use_home_page_styles,
};

/// Renders the landing page and its recent-file actions.
///
/// # Returns
///
/// A Home page component.
#[component]
pub(crate) fn HomePage() -> impl IntoView {
    let file_selector = use_file_selector();
    let shortcut_file_selector = file_selector.clone();
    let button_file_selector = file_selector.clone();
    let selected_navigate = use_navigate();
    let notifications = use_notifications();
    let recent_files_store = RecentFilesStore::standard();
    let current_directory = std::env::current_dir().unwrap_or_default();
    let (recent_files, recent_error) = recent_files_store.load_valid();
    if let Some(error) = &recent_error {
        notifications.show_warning("Recent files unavailable", error.to_string());
    }

    let open_error = RwSignal::new(None::<String>);
    let open_store = recent_files_store.clone();
    let open_file: Rc<dyn Fn(PathBuf)> =
        Rc::new(
            move |file| match recorded_viewer_location(&open_store, &file) {
                Ok(target) => selected_navigate(&target, NavigateOptions::default()),
                Err(error) => open_error.set(Some(format!("{error:#}"))),
            },
        );
    let selected_open_file = Rc::clone(&open_file);
    let selected_file_selector = file_selector.clone();
    Effect::new(move || {
        if let Some(file) = file_selector.get_file() {
            selected_file_selector.clear();
            selected_open_file(file);
        }
    });

    use_key_event(KeyEventKind::Press, move |key| {
        if key.code == KeyCode::Char('o') && key.modifiers == KeyModifiers::NONE {
            shortcut_file_selector
                .select_with_options(FileSelectorOptions::new().extensions(["md", "markdown"]));
            return KeyControl::Handled;
        }

        KeyControl::Pass
    });

    let directory_label = format!("Current directory: {}", current_directory.display());
    let recent_error = recent_error.map(|error| error.to_string());
    let open_error_view = dynamic(move || {
        open_error.with(|error| {
            error.as_ref().map_or_else(
                || text("").into_view(),
                |error| {
                    leptatui::__private::__view_error(
                        ViewError::msg(error.clone()),
                        file!(),
                        line!(),
                    )
                },
            )
        })
    });

    use_home_page_styles();

    view! {
        <Div class="home-page">
            <Text class="home-page__title">"Markdown editor"</Text>
            <Text class="home-page__path">{directory_label}</Text>
            <Div class="home-page__actions">
                <Button on_press=move || {
                    button_file_selector
                        .select_with_options(
                            FileSelectorOptions::new().extensions(["md", "markdown"]),
                        );
                    AppControl::Continue
                }>"Open file"</Button>
            </Div>
            <Block class="home-page__content">
                <RecentFilesList
                    entries=recent_files
                    error=recent_error
                    base=current_directory
                    on_open=open_file
                />
            </Block>
            <Text class="home-page__help">"o open file | Tab/Enter actions | q quit"</Text>
            {open_error_view}
        </Div>
    }
}

/// Records a Home-selected path before producing its Viewer location.
fn recorded_viewer_location(
    store: &RecentFilesStore,
    path: &std::path::Path,
) -> ViewResult<String> {
    store.record(path).map_err(ViewError::from)?;
    Ok(viewer_location(path))
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        sync::atomic::{AtomicU64, Ordering},
    };

    use super::*;

    /// Verifies a Viewer location is returned only after the path is persisted.
    #[test]
    fn selected_file_is_recorded_before_viewer_location_is_returned() {
        static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

        let directory = std::env::temp_dir().join(format!(
            "leptatui-home-open-{}-{}",
            std::process::id(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
        ));
        let path = directory.join("guide.md");
        fs::create_dir_all(&directory).expect("fixture directory should be created");
        fs::write(&path, "# Guide").expect("Markdown fixture should be written");
        let store = RecentFilesStore::memory();

        let target = recorded_viewer_location(&store, &path)
            .expect("recording should produce a Viewer location");

        assert_eq!(target, viewer_location(&path));
        assert_eq!(
            store.load_valid().0,
            [fs::canonicalize(&path).expect("fixture path should canonicalize")]
        );

        fs::remove_dir_all(directory).expect("fixture directory should be removed");
    }

    /// Verifies a failed record does not produce a Viewer location.
    #[test]
    fn failed_record_does_not_produce_viewer_location() {
        static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

        let store = RecentFilesStore::memory();
        let missing = std::env::temp_dir().join(format!(
            "leptatui-home-open-missing-{}-{}.md",
            std::process::id(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
        ));

        assert!(recorded_viewer_location(&store, &missing).is_err());
    }
}

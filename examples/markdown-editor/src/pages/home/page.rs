//! Home route-level component and keyboard behavior.

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
    let recent_files_store = expect_context::<RecentFilesStore>();
    let current_directory = std::env::current_dir().unwrap_or_default();
    let (recent_files, recent_error) = recent_files_store.load_valid();
    if let Some(error) = &recent_error {
        notifications.show_warning("Recent files unavailable", error.to_string());
    }

    Effect::new(move || {
        if let Some(file) = file_selector.get_file() {
            selected_navigate(&viewer_location(&file), NavigateOptions::default());
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
                <RecentFilesList entries=recent_files error=recent_error base=current_directory />
            </Block>
            <Text class="home-page__help">"o open file | Tab/Enter actions | q quit"</Text>
        </Div>
    }
}

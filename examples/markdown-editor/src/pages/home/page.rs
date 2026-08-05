//! Home route-level component and keyboard behavior.

use leptatui::prelude::*;

use crate::{contexts::use_notifications, services::RecentFilesStore};

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
    let shortcut_navigate = use_navigate();
    let button_navigate = use_navigate();
    let notifications = use_notifications();
    let recent_files_store = expect_context::<RecentFilesStore>();
    let current_directory = std::env::current_dir().unwrap_or_default();
    let (recent_files, recent_error) = recent_files_store.load_valid();
    if let Some(error) = &recent_error {
        notifications.show_warning("Recent files unavailable", error.to_string());
    }

    use_key_event(KeyEventKind::Press, move |key| {
        if key.code == KeyCode::Char('o') && key.modifiers == KeyModifiers::NONE {
            shortcut_navigate("/files", NavigateOptions::default());
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
                    button_navigate("/files", NavigateOptions::default());
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

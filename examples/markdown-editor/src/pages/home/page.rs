//! Home route-level component and keyboard behavior.

use leptatui::prelude::*;

use crate::hooks::{use_files, use_workspace};

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
    let workspace = use_workspace().workspace;
    let files = use_files();

    use_key_event(KeyEventKind::Press, move |key| {
        if key.code == KeyCode::Char('o') && key.modifiers == KeyModifiers::NONE {
            shortcut_navigate("/files", NavigateOptions::default());
            return KeyControl::Handled;
        }

        KeyControl::Pass
    });

    let root = workspace.root().to_path_buf();
    let root_label = format!("Root: {}", root.display());
    let recent_root = root.clone();

    use_home_page_styles();

    view! {
        <Div class="home-page">
            <Text class="home-page__title">"Markdown editor"</Text>
            <Text class="home-page__path">{root_label}</Text>
            <Div class="home-page__actions">
                <Button on_press=move || {
                    button_navigate("/files", NavigateOptions::default());
                    AppControl::Continue
                }>"Open file"</Button>
            </Div>
            <Block class="home-page__content">
                {move || {
                    let root = recent_root.clone();
                    view! {
                        <RecentFilesList
                            entries=files.recent_files.get_untracked()
                            error=files
                                .recent_files_error
                                .get_untracked()
                                .map(|error| error.to_string())
                            root=root
                        />
                    }
                }}
            </Block>
            <Text class="home-page__help">"o open file | Tab/Enter actions | q quit"</Text>
        </Div>
    }
}

//! Routed Markdown editor application component.

use leptatui::prelude::*;

use crate::{
    layouts::{RootLayout, RootLayoutProps},
    pages::{HomePage, NotFoundPage, ViewerPage},
};

/// Provides routing and global controls.
///
/// # Arguments
///
/// * `initial_path` — First router location for the application session.
///
/// # Returns
///
/// A routed Markdown editor application.
#[component]
pub(crate) fn AppRouter(#[prop(into)] initial_path: String) -> impl IntoView {
    use_key_event(KeyEventKind::Press, |key| {
        if key.code == KeyCode::Char('q') && key.modifiers == KeyModifiers::NONE {
            return KeyControl::Exit;
        }

        KeyControl::Pass
    });

    view! {
        <Router initial_path=initial_path>
            <RootLayout>
                <Routes fallback=NotFoundPage>
                    <Route path="/" view=HomePage />
                    <Route path="/view" view=ViewerPage />
                </Routes>
            </RootLayout>
        </Router>
    }
}

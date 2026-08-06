//! Routed Markdown editor application component.

use leptatui::prelude::*;

use crate::{
    contexts::{Notifications, provide_notification_context},
    layouts::{RootLayout, RootLayoutProps},
    pages::{HomePage, NotFoundPage, ViewerPage},
    services::RecentFilesStore,
};

/// Provides application services, routing, notifications, and global controls.
///
/// An existing recent-file context is retained. A missing context is
/// initialized with its production implementation.
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
    if use_context::<RecentFilesStore>().is_none() {
        provide_context(RecentFilesStore::standard());
    }

    provide_notification_context();

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
                    <Route path="/view/*path" view=ViewerPage />
                </Routes>
                <Notifications />
            </RootLayout>
        </Router>
    }
}

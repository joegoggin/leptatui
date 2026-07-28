//! Pass fixture for route-driven page switches in `view!`.
//!
//! This binary verifies declarative routes accept component page factories.

use leptatui::prelude::*;

/// Home page branch.
#[component]
fn HomePage() -> impl IntoView {
    view! { <Text>"Home"</Text> }
}

/// Counter page branch.
#[component]
fn CounterPage() -> impl IntoView {
    view! { <Text>"Counter"</Text> }
}

/// Settings page branch.
#[component]
fn SettingsPage() -> impl IntoView {
    view! { <Text>"Settings"</Text> }
}

/// Fallback page branch.
#[component]
fn NotFoundPage() -> impl IntoView {
    view! { <Text>"Not found"</Text> }
}

/// Exercises three declarative route definitions.
fn main() {
    let _view = view! {
        <Router initial_path="/counter">
            <Routes fallback=NotFoundPage>
                <Route path="/" view=HomePage />
                <Route path="/counter" view=CounterPage />
                <Route path="/settings" view=SettingsPage />
            </Routes>
        </Router>
    };
}

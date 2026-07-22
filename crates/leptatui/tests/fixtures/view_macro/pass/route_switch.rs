//! Pass fixture for route-driven page switches in `view!`.
//!
//! This binary verifies a dynamic child can branch on route state and return
//! different component pages without manual view-node construction.

use leptatui::prelude::*;

/// Route values for the page switch fixture.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Page {
    Home,
    Counter,
    Settings,
}

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

/// Exercises a three-branch route switch inside a dynamic child.
fn main() {
    Owner::new().with(|| {
        let route_state = provide_route(Page::Home);
        let route = route_state.route();
        let navigate = route_state.navigate();

        navigate.update(|route| *route = Page::Counter);
        navigate.update(|route| *route = Page::Settings);

        let view = view! {
            <Column>
                {move || match route.get_untracked() {
                    Page::Home => view! { <HomePage /> },
                    Page::Counter => view! { <CounterPage /> },
                    Page::Settings => view! { <SettingsPage /> },
                }}
            </Column>
        };

        assert_eq!(view.metadata().view_type(), ViewType::Column);
    });
}

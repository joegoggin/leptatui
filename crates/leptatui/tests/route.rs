//! Public router hook and history tests.

use leptatui::{
    NavigateOptions, Router, RouterProps, use_history, use_location, use_navigate, use_query_map,
};
use leptos::prelude::{GetUntracked, Owner};

/// Verifies location, query, navigation, and history hooks share router state.
///
/// # Example Under Test
///
/// ```text
/// /docs?page=1 -> /settings?mode=dark -> back -> forward
/// ```
///
/// # Assertions
///
/// - The initial pathname and decoded query are available.
/// - Push navigation updates pathname and query state.
/// - Back and forward restore their respective entries.
/// - Replace navigation does not add another back entry.
#[test]
fn router_hooks_navigate_through_in_memory_history() {
    Owner::new().with(|| {
        leptatui::__private::__with_context_scope(|| {
            let _router = Router::with_props(
                RouterProps::builder()
                    .initial_path("/docs?page=1")
                    .children(Box::new(|| {
                        let location = use_location();
                        let query = use_query_map();
                        let navigate = use_navigate();
                        let history = use_history();

                        assert_eq!(location.pathname().get_untracked(), "/docs");
                        assert_eq!(query.get_untracked().get("page"), Some("1"));

                        navigate("/settings?mode=dark", NavigateOptions::default());
                        assert_eq!(location.pathname().get_untracked(), "/settings");
                        assert_eq!(query.get_untracked().get("mode"), Some("dark"));
                        assert!(history.can_go_back());

                        history.back();
                        assert_eq!(location.pathname().get_untracked(), "/docs");
                        assert!(history.can_go_forward());

                        history.forward();
                        assert_eq!(location.pathname().get_untracked(), "/settings");

                        navigate("/profile", NavigateOptions { replace: true });
                        history.back();
                        assert_eq!(location.pathname().get_untracked(), "/docs");

                        Vec::new()
                    }))
                    .build(),
            );
        });
    });
}

//! Pass fixture for `Link` expansion.

use leptatui::prelude::*;

/// Exercises `Link` content, target, and selector metadata lowering.
fn main() {
    let style = TuiStyle::new().foreground(Color::Yellow);
    let view: View = view! {
        <Link
            href="https://example.com"
            class="navigation primary"
            id="guide"
            style={style}
        >{"Guide".to_owned()}</Link>
    };

    match view {
        View::Link {
            label,
            target,
            metadata,
        } => {
            assert_eq!(label.to_string(), "Guide");
            assert_eq!(
                target,
                LinkTarget::Url("https://example.com".to_owned())
            );
            assert_eq!(metadata.view_type(), ViewType::Link);
            assert_eq!(metadata.id(), Some("guide"));
            assert_eq!(
                metadata.classes(),
                &[String::from("navigation"), String::from("primary")]
            );
            assert_eq!(metadata.inline_style(), Some(style));
        }
        other => panic!("expected link view, got {other:?}"),
    }
}

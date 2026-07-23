//! Pass fixture for `Link` expansion.

use leptatui::prelude::*;

/// Exercises `Link` content, target, and selector metadata lowering.
fn main() {
    let style = TuiStyle::new().foreground(Color::Yellow);
    let view = view! {
        <Link
            href="https://example.com"
            class="navigation primary"
            id="guide"
            style={style}
        >{"Guide".to_owned()}</Link>
    };

    assert_eq!(view.content().to_string(), "Guide");
    assert_eq!(
        view.target(),
        &LinkTarget::Url("https://example.com".to_owned())
    );
    assert_eq!(view.metadata().view_type(), ViewType::Link);
    assert_eq!(view.metadata().id(), Some("guide"));
    assert_eq!(
        view.metadata().classes(),
        &[String::from("navigation"), String::from("primary")]
    );
    assert_eq!(view.metadata().inline_style(), Some(style));
}

//! Pass fixture for `Image` expansion.

use std::path::PathBuf;

use leptatui::prelude::*;

/// Exercises `Image` attributes and builder lowering.
fn main() {
    let style = TuiStyle::new().foreground(Color::Yellow);
    let path = PathBuf::from("assets/logo.png");
    let view = view! {
        <Image
            src={ImageSource::Path(path.clone())}
            alt="Project logo"
            class="media primary"
            id="logo"
            style={style}
        />
    };

    assert_eq!(view.source(), &ImageSource::Path(path));
    assert_eq!(view.alt_text(), Some("Project logo"));
    assert_eq!(view.metadata().view_type(), ViewType::Image);
    assert_eq!(view.metadata().id(), Some("logo"));
    assert_eq!(
        view.metadata().classes(),
        &[String::from("media"), String::from("primary")]
    );
    assert_eq!(view.metadata().inline_style(), Some(style));
}

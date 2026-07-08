//! Pass fixture for `Image` expansion.

use std::path::PathBuf;

use leptatui::prelude::*;

/// Exercises `Image` attributes and builder lowering.
fn main() {
    let style = TuiStyle::new().foreground(Color::Yellow);
    let path = PathBuf::from("assets/logo.png");
    let view: View = view! {
        <Image
            src={ImageSource::Path(path.clone())}
            alt="Project logo"
            class="media primary"
            id="logo"
            style={style}
        />
    };

    match view {
        View::Image {
            source,
            alt,
            metadata,
        } => {
            assert_eq!(source, ImageSource::Path(path));
            assert_eq!(alt.as_deref(), Some("Project logo"));
            assert_eq!(metadata.view_type(), ViewType::Image);
            assert_eq!(metadata.id(), Some("logo"));
            assert_eq!(
                metadata.classes(),
                &[String::from("media"), String::from("primary")]
            );
            assert_eq!(metadata.inline_style(), Some(style));
        }
        other => panic!("expected image view, got {other:?}"),
    }
}

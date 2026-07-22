//! Full-screen local Markdown reader.
//!
//! This binary loads one UTF-8 Markdown file before terminal startup and
//! renders it as a responsive, syntax-highlighted semantic document. Local
//! Markdown links and heading fragments remain inside the reader with cached
//! back/forward history.

use std::{env, error::Error, ffi::OsString, io, path::PathBuf};

use leptatui::prelude::*;

/// Renders a loaded Markdown document as a full-screen reader.
///
/// # Arguments
///
/// * `document` — Semantic Markdown view loaded before terminal startup.
#[component]
fn MarkdownReader(document: View) -> View {
    use_key_event(KeyEventKind::Press, |key| {
        if key.code == KeyCode::Char('q') {
            return KeyControl::Exit;
        }

        KeyControl::Pass
    });

    stylesheet! {
        .reader-shell => {
            fg: Color::White,
            border_type: BorderType::Rounded,
            padding: TuiSpacing::uniform(1)
        }

        H1 => { fg: Color::LightCyan }
        H2 => { fg: Color::LightBlue }
        H3 => { fg: Color::LightGreen }
        H4 => { fg: Color::LightYellow }
        H5 => { fg: Color::LightMagenta }
        H6 => { fg: Color::Gray }
        Paragraph => { fg: Color::White }
        OrderedList => { fg: Color::LightCyan }
        UnorderedList => { fg: Color::LightGreen }
        Table => { fg: Color::White }
        TableHead => { fg: Color::LightCyan }
        CodeBlock => { fg: Color::LightBlue }
        Link:focus => { fg: Color::Black, bg: Color::LightCyan }

        @media (max-width: 60) {
            .reader-shell => {
                border_type: BorderType::Plain,
                padding: TuiSpacing::ZERO
            }
        }
    }

    view! { <Block class="reader-shell">{document}</Block> }
}

/// Returns the supplied Markdown path or the bundled showcase path.
///
/// # Arguments
///
/// * `args` — Process arguments including the executable name.
///
/// # Returns
///
/// A [`PathBuf`] containing the requested or default Markdown file path.
///
/// # Errors
///
/// Returns [`io::ErrorKind::InvalidInput`] if more than one path is supplied.
fn markdown_path(args: impl IntoIterator<Item = OsString>) -> io::Result<PathBuf> {
    let mut args = args.into_iter();
    let executable = args
        .next()
        .unwrap_or_else(|| OsString::from("markdown_reader"));

    match (args.next(), args.next()) {
        (None, None) => {
            Ok(PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("examples/assets/markdown_showcase.md"))
        }
        (Some(path), None) => Ok(PathBuf::from(path)),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("usage: {} [MARKDOWN_PATH]", executable.to_string_lossy()),
        )),
    }
}

/// Loads and runs the full-screen Markdown reader.
///
/// # Returns
///
/// An empty [`std::result::Result`] when the reader exits successfully.
///
/// # Errors
///
/// Returns an error if command-line validation, terminal setup, rendering,
/// input, external link activation, or cleanup fails. Markdown loading
/// failures render in the reader and remain reachable through page history.
#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn Error>> {
    let path = markdown_path(env::args_os())?;
    let document = view! { <Markdown src=path syntax_theme=SyntaxTheme::Dark line_numbers=true /> };
    let reader =
        MarkdownReader::with_props(MarkdownReaderProps::builder().document(document).build());

    App::new(reader).run().await?;
    Ok(())
}

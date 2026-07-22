//! Internal metadata shared by component-boundary traversal.

/// Focused built-in control metadata used by internal view traversal.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FocusedControl {
    /// A button is focused.
    Button,
    /// A link is focused.
    Link,
    /// A single-line input is focused.
    Input {
        /// Whether the input is currently in insert mode.
        insert_mode: bool,
        /// Whether the input is currently in visual mode.
        visual_mode: bool,
    },
    /// A multiline text area is focused.
    TextArea {
        /// Whether the text area is currently in insert mode.
        insert_mode: bool,
        /// Whether the text area is currently in visual mode.
        visual_mode: bool,
    },
}

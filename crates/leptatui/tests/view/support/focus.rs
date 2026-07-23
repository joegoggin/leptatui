/// Returns flattened focus states for all buttons in a view tree.
///
/// # Arguments
///
/// * `view` — View tree to inspect.
///
/// # Returns
///
/// A [`Vec<bool>`] containing focus state for each button.
fn button_focuses(view: &dyn View) -> Vec<bool> {
    if let Some(button) = view.as_any().downcast_ref::<ButtonView>() {
        return vec![button.metadata().is_focused()];
    }

    view.children()
        .iter()
        .flat_map(|child| button_focuses(child.as_view()))
        .collect()
}

/// Returns flattened focus states for all focusable controls in a view tree.
///
/// # Arguments
///
/// * `view` — View tree to inspect.
///
/// # Returns
///
/// A [`Vec<bool>`] containing focus state for each focusable control.
fn control_focuses(view: &dyn View) -> Vec<bool> {
    if let Some(button) = view.as_any().downcast_ref::<ButtonView>() {
        return vec![button.metadata().is_focused()];
    }
    if let Some(link) = view.as_any().downcast_ref::<LinkView>() {
        return vec![link.metadata().is_focused()];
    }
    if let Some(editor) = view.as_any().downcast_ref::<InputView>() {
        return vec![editor.metadata().is_focused()];
    }
    if let Some(editor) = view.as_any().downcast_ref::<TextAreaView>() {
        return vec![editor.metadata().is_focused()];
    }

    view.children()
        .iter()
        .flat_map(|child| control_focuses(child.as_view()))
        .collect()
}

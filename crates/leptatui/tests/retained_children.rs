//! Rendering and lifecycle tests for retained component children.

mod support;

use std::{cell::Cell, rc::Rc};

use leptatui::prelude::*;
use ratatui::{Terminal, backend::TestBackend};
use support::{draw_view, key, rendered_text};

/// Wraps directly rendered children in the guide's panel layout.
///
/// # Arguments
///
/// * `children` — Deferred panel content.
///
/// # Returns
///
/// A panel retaining its nested content.
#[component]
fn Panel(children: Children) -> impl IntoView {
    view! { <Block><Div>{children}</Div></Block> }
}

/// Wraps callable children in the same panel layout.
///
/// # Arguments
///
/// * `children` — Deferred panel content.
///
/// # Returns
///
/// A panel using the compatible callable syntax.
#[component]
fn CallablePanel(children: Children) -> impl IntoView {
    view! { <Block><Div>{children()}</Div></Block> }
}

/// Provides context before consuming directly rendered children.
///
/// # Arguments
///
/// * `children` — Content that reads the panel's context.
///
/// # Returns
///
/// A view whose descendants receive the consuming component's context.
#[component]
fn ContextPanel(children: Children) -> impl IntoView {
    provide_context("inner");
    view! { <Div>{children}</Div> }
}

/// Verifies direct and callable children render the same ordered layout.
///
/// # Example Under Test
///
/// ```text
/// Panel and CallablePanel containing two text children, then empty content.
/// ```
///
/// # Assertions
///
/// - Direct rendering matches callable rendering for multiple and zero children.
/// - Both children appear in source order.
#[test]
fn direct_children_preserve_layout_and_order() -> leptatui::Result<()> {
    Owner::new().with(|| {
        let direct = view! { <Panel><Text>"First"</Text><Text>"Second"</Text></Panel> };
        let callable =
            view! { <CallablePanel><Text>"First"</Text><Text>"Second"</Text></CallablePanel> };
        let mut actual = Terminal::new(TestBackend::new(20, 6))?;
        let mut expected = Terminal::new(TestBackend::new(20, 6))?;
        draw_view(&mut actual, direct.as_view())?;
        draw_view(&mut expected, callable.as_view())?;
        assert_eq!(actual.backend().buffer(), expected.backend().buffer());
        let output = rendered_text(&actual);
        assert!(
            output.contains("First") && output.contains("Second"),
            "{output:?}"
        );
        assert!(output.find("First").unwrap() < output.find("Second").unwrap());

        let empty = Panel::with_props(PanelProps::builder().children(Box::new(Vec::new)).build());
        let callable_empty = CallablePanel::with_props(
            CallablePanelProps::builder()
                .children(Box::new(Vec::new))
                .build(),
        );
        draw_view(&mut actual, &empty)?;
        draw_view(&mut expected, &callable_empty)?;
        assert_eq!(actual.backend().buffer(), expected.backend().buffer());
        Ok(())
    })
}

/// Verifies direct conversion stays lazy and cloned handles share their factory.
///
/// # Example Under Test
///
/// ```text
/// Retain a boxed factory, clone the handle, convert to a view, render and call.
/// ```
///
/// # Assertions
///
/// - Conversion does not invoke the factory.
/// - Rendering and repeated calls invoke the original factory once in total.
/// - The converted view renders the factory's content.
#[test]
fn retained_handles_are_lazy_and_share_their_children() -> leptatui::Result<()> {
    Owner::new().with(|| {
        let calls = Rc::new(Cell::new(0));
        let observed = calls.clone();
        let children = leptatui::__private::__retain_children(Box::new(move || {
            calls.set(calls.get() + 1);
            vec![text("Retained").into_view()]
        }));
        let callable = children.clone();
        let view = children.into_view();
        assert_eq!(observed.get(), 0);
        let mut terminal = Terminal::new(TestBackend::new(20, 4))?;
        draw_view(&mut terminal, view.as_view())?;
        assert!(rendered_text(&terminal).contains("Retained"));
        assert_eq!(callable().len(), 1);
        assert_eq!(callable().len(), 1);
        draw_view(&mut terminal, view.as_view())?;
        assert_eq!(observed.get(), 1);
        Ok(())
    })
}

/// Verifies deferred factories see the consuming component's context.
///
/// # Example Under Test
///
/// ```text
/// Outer context "outer", ContextPanel context "inner", factory reads context.
/// ```
///
/// # Assertions
///
/// - The factory is deferred until consumption.
/// - The factory receives the inner context and executes once across renders.
#[test]
fn direct_children_use_the_consuming_context() -> leptatui::Result<()> {
    Owner::new().with(|| {
        provide_context("outer");
        let calls = Rc::new(Cell::new(0));
        let observed = calls.clone();
        let panel = ContextPanel::with_props(
            ContextPanelProps::builder()
                .children(Box::new(move || {
                    calls.set(calls.get() + 1);
                    vec![text(expect_context::<&'static str>()).into_view()]
                }))
                .build(),
        );
        assert_eq!(observed.get(), 0);
        let mut terminal = Terminal::new(TestBackend::new(20, 4))?;
        draw_view(&mut terminal, &panel)?;
        assert!(rendered_text(&terminal).contains("inner"));
        draw_view(&mut terminal, &panel)?;
        assert_eq!(observed.get(), 1);
        Ok(())
    })
}

/// Creates local editable state beneath a changing label.
///
/// # Arguments
///
/// * `label` — Reactive label supplied by the parent.
/// * `setups` — Counter recording child initialization.
///
/// # Returns
///
/// A locally controlled input alongside the current label.
#[component]
fn StatefulChild(label: String, #[prop(static)] setups: Rc<Cell<usize>>) -> impl IntoView {
    setups.set(setups.get() + 1);
    let local = RwSignal::new(String::new());
    view! { <Div><Text>{label}</Text><Input value=local /></Div> }
}

/// Verifies direct children retain local edits during reactive updates.
///
/// # Example Under Test
///
/// ```text
/// Panel > StatefulChild, type "a", change label from First to Second, type "b".
/// ```
///
/// # Assertions
///
/// - The changed label renders and the child initializes once.
/// - Local input contents, focus, and insert mode survive the label update.
#[test]
fn direct_children_preserve_reactive_child_state() -> leptatui::Result<()> {
    Owner::new().with(|| {
        let label = RwSignal::new(String::from("First"));
        let setups = Rc::new(Cell::new(0));
        let observed = setups.clone();
        let mut view = view! { <Panel><StatefulChild label={label.get()} setups=setups /></Panel> };
        let mut terminal = Terminal::new(TestBackend::new(30, 8))?;
        draw_view(&mut terminal, view.as_view())?;
        for code in [KeyCode::Tab, KeyCode::Char('i'), KeyCode::Char('a')] {
            View::handle_event(view.as_view_mut(), key(code))?;
        }
        label.set(String::from("Second"));
        draw_view(&mut terminal, view.as_view())?;
        View::handle_event(view.as_view_mut(), key(KeyCode::Char('b')))?;
        draw_view(&mut terminal, view.as_view())?;
        let output = rendered_text(&terminal);
        assert!(output.contains("Second"), "{output}");
        assert!(output.contains("ab"), "{output}");
        assert_eq!(observed.get(), 1);
        Ok(())
    })
}

/// Verifies retained fragments preserve clipping and scrolling geometry.
///
/// # Example Under Test
///
/// ```text
/// Panel with eight rows in a 20x5 terminal, then PageDown.
/// ```
///
/// # Assertions
///
/// - Initial and scrolled output match an ordinary Block > Div tree.
/// - Scrolling reveals the last row while preserving the panel border.
#[test]
fn direct_children_preserve_scrolling_geometry() -> leptatui::Result<()> {
    Owner::new().with(|| {
        let mut panel = Panel::with_props(
            PanelProps::builder()
                .children(Box::new(|| {
                    (0..8)
                        .map(|index| text(format!("Row {index}")).into_view())
                        .collect()
                }))
                .build(),
        );
        let mut ordinary = block(div((0..8)
            .map(|index| text(format!("Row {index}")))
            .collect::<Vec<_>>()));
        let mut actual = Terminal::new(TestBackend::new(20, 5))?;
        let mut expected = Terminal::new(TestBackend::new(20, 5))?;
        draw_view(&mut actual, &panel)?;
        draw_view(&mut expected, &ordinary)?;
        assert_eq!(actual.backend().buffer(), expected.backend().buffer());
        assert!(rendered_text(&actual).contains("Row 0"));
        assert!(!rendered_text(&actual).contains("Row 7"));
        View::handle_event(&mut panel, key(KeyCode::PageDown))?;
        View::handle_event(&mut ordinary, key(KeyCode::PageDown))?;
        draw_view(&mut actual, &panel)?;
        draw_view(&mut expected, &ordinary)?;
        assert_eq!(actual.backend().buffer(), expected.backend().buffer());
        assert!(rendered_text(&actual).contains("Row 7"));
        Ok(())
    })
}

/// View with a local stylesheet applied to its own text view.
#[component]
fn MacroStyledText() -> impl leptatui::IntoView {
    stylesheet! {
        .scoped => { fg: Color::Yellow, bg: Color::Blue }
    }

    text("Scoped").with_classes("scoped")
}

/// View whose local stylesheet removes its own child from layout.
#[component]
fn MacroHiddenLayoutChild() -> impl leptatui::IntoView {
    stylesheet! {
        .hidden => { display: Display::None }
    }

    text("Hidden").with_classes("hidden")
}

/// View whose stylesheet targets a shared class name.
#[component]
fn MacroStyledSibling() -> impl leptatui::IntoView {
    stylesheet! {
        .shared => { fg: Color::Yellow }
    }

    text("Styled").with_classes("shared")
}

/// View with a class that should not receive sibling styles.
#[component]
fn MacroPlainSibling() -> impl leptatui::IntoView {
    text("Plain").with_classes("shared")
}

/// Parent component whose stylesheet should apply to child component internals.
#[component]
fn MacroParentStylesChild() -> impl leptatui::IntoView {
    stylesheet! {
        Text => { fg: Color::Green }
    }

    component(MacroPlainSibling::new())
}

/// Parent and child components with equal-specificity text rules.
#[component]
fn MacroParentWithChildOverride() -> impl leptatui::IntoView {
    stylesheet! {
        Text => { fg: Color::Green }
    }

    component(MacroChildStyleOverride::new())
}

/// Child component whose equal-specificity stylesheet should be later in source order.
#[component]
fn MacroChildStyleOverride() -> impl leptatui::IntoView {
    stylesheet! {
        Text => { fg: Color::Yellow }
    }

    text("Override")
}

/// Parent component with a class rule that should beat a child type rule.
#[component]
fn MacroParentSpecificityBeatsChild() -> impl leptatui::IntoView {
    stylesheet! {
        .specific => { fg: Color::Green }
    }

    component(MacroChildLowerSpecificity::new())
}

/// Child component with a lower-specificity type rule.
#[component]
fn MacroChildLowerSpecificity() -> impl leptatui::IntoView {
    stylesheet! {
        Text => { fg: Color::Yellow }
    }

    text("Specific").with_classes("specific")
}

/// View whose stylesheet resolves against theme context it provides.
#[component]
fn MacroThemedStylesheet() -> impl leptatui::IntoView {
    provide_context(ThemeVariables::new().color("text", Color::LightCyan));

    stylesheet! {
        .themed => { fg: theme_color("text") }
    }

    text("Theme").with_classes("themed")
}

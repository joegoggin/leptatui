/// View whose internal layout changes height under a media rule.
#[component]
fn MacroResponsiveCaseRow() -> impl leptatui::IntoView {
    view! {
        <Row class="case-row">
            <Text>"type < class"</Text>
            <Text>"Sample"</Text>
        </Row>
    }
}

/// Parent component that must reserve the responsive child component height.
#[component]
fn MacroResponsiveCaseRoot() -> impl leptatui::IntoView {
    stylesheet! {
        @media (max-width: 60) {
            .case-row => { direction: LayoutDirection::Column }
        }
    }

    view! {
        <Column>
            <Text>"Intro"</Text>
            <MacroResponsiveCaseRow />
        </Column>
    }
}

/// View with an overflowing internal layout.
#[component]
fn MacroScrollableList() -> impl leptatui::IntoView {
    column([
        text("One"),
        text("Two"),
        text("Three"),
        text("Four"),
        text("Five"),
        text("Six"),
    ])
}

/// Parent component whose default scroll keys must reach a child component.
#[component]
fn MacroScrollableBoundaryRoot() -> impl leptatui::IntoView {
    row([component(MacroScrollableList::new())])
}

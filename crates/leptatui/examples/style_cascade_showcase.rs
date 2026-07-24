//! Style cascade showcase.
//!
//! This binary demonstrates CSS-like specificity, source order, descendant
//! selectors, component stylesheet boundaries, inline styles, `!important`,
//! inheritance, and focus styling.

use leptatui::prelude::*;

/// Repeated labeled row for one cascade case.
#[component]
fn CaseRow(#[prop(into)] title: String, children: Children) -> impl IntoView {
    view! {
        <Div class="case-row">
            <Text class="case-label">{title}</Text>
            <Div class="case-sample">{div(children())}</Div>
        </Div>
    }
}

/// Child component whose internals are styled by the parent component stylesheet.
#[component]
fn ChildReceivesParentStyle() -> impl IntoView {
    view! {
        <Text class="parent-reaches-child">"parent stylesheet crosses the component boundary"</Text>
    }
}

/// Child component with a lower-specificity local stylesheet.
#[component]
fn ChildLowerSpecificity() -> impl IntoView {
    stylesheet! {
        Text => { fg: Color::LightRed }
    }

    view! { <Text class="parent-specific">"parent .class beats child Text"</Text> }
}

/// Button component used to show focus selectors across component boundaries.
#[component]
fn ActionButton(#[prop(into)] label: String, on_press: fn() -> AppControl) -> impl IntoView {
    view! {
        <Button class="focus-button" on_press=on_press>
            {label}
        </Button>
    }
}

/// Root component for the cascade showcase.
#[component]
fn StyleCascadeShowcase() -> impl IntoView {
    use_key_event(KeyEventKind::Press, |key| {
        if key.code == KeyCode::Char('q') {
            return KeyControl::Exit;
        }

        KeyControl::Pass
    });

    stylesheet! {
        .screen => {
            fg: Color::White,
            bg: Color::Black,
            border_type: BorderType::Rounded,
            padding: TuiSpacing::uniform(1),

            // Baseline type rule. Class, id, descendant, inline, and important
            // cases below deliberately override it.
            Text => { fg: Color::DarkGray }

            .title => { fg: Color::Yellow, modifier: Modifier::BOLD }
            .intro => { fg: Color::Gray }
            .case-label => { fg: Color::White, modifier: Modifier::BOLD }
            .case-sample => { fg: Color::Gray }

            // Type < class specificity.
            .class-wins => { fg: Color::LightGreen }

            // Equal specificity: later source order wins.
            .source-first => { fg: Color::LightBlue }
            .source-second => { fg: Color::LightMagenta }

            // Descendant selector specificity beats a later class selector.
            .descendant-scope => {
                Text => { fg: Color::LightCyan, modifier: Modifier::BOLD }
            }
            .descendant-target => { fg: Color::LightRed }

            // Id specificity beats classes.
            .id-class => { fg: Color::Yellow }
            #id-wins => { fg: Color::LightCyan, modifier: Modifier::BOLD }

            // Parent component stylesheet crosses child component boundaries.
            .parent-reaches-child => { fg: Color::LightGreen }

            // Higher-specificity parent rule beats lower-specificity child Text.
            .parent-specific => { fg: Color::LightGreen, modifier: Modifier::BOLD }

            // Normal inline style beats this normal stylesheet rule.
            .inline-demo => { fg: Color::LightRed }

            // Important stylesheet declaration beats normal inline style.
            .important-demo => { fg: Color::LightRed !important }

            // Foreground and modifiers inherit; background does not become an
            // inherited declaration for descendants.
            .inheritance-parent => {
                fg: Color::LightBlue,
                bg: Color::DarkGray,
                modifier: Modifier::BOLD,
                padding: TuiSpacing::uniform(1)
            }

            Button => {
                fg: Color::White,
                bg: Color::Black,
                border_type: BorderType::Rounded
            }

            Button:focus => {
                fg: Color::Black,
                bg: Color::Yellow,
                modifier: Modifier::BOLD,
                border_type: BorderType::Thick
            }
        }

        .case-row => { display: Display::Flex }
        .focus-actions => { display: Display::Flex }

        @media (max-width: 60) {
            .screen => { padding: TuiSpacing::ZERO }
            .case-row => { flex_direction: FlexDirection::Column }
            .inheritance-parent => { padding: TuiSpacing::ZERO }
            .focus-actions => { flex_direction: FlexDirection::Column }
            Button => {
                padding: TuiSpacing::ZERO
            }
        }
    }

    view! {
        <Block class="screen">
            <Div>
                <Text class="title">"Style cascade showcase"</Text>
                <Text class="intro">
                    "Each row names the cascade rule that decides the sample style."
                </Text>

                <CaseRow title="type < class">
                    <Text class="class-wins">
                        "class selector overrides the baseline Text rule"
                    </Text>
                </CaseRow>

                <CaseRow title="source order">
                    <Text class="source-first source-second">
                        "later .source-second wins at equal specificity"
                    </Text>
                </CaseRow>

                <CaseRow title="descendant specificity">
                    <Block class="descendant-scope">
                        <Text class="descendant-target">
                            ".descendant-scope Text beats .descendant-target"
                        </Text>
                    </Block>
                </CaseRow>

                <CaseRow title="id specificity">
                    <Text id="id-wins" class="id-class">
                        "#id-wins beats .id-class"
                    </Text>
                </CaseRow>

                <CaseRow title="parent to child">
                    <ChildReceivesParentStyle />
                </CaseRow>

                <CaseRow title="parent specificity">
                    <ChildLowerSpecificity />
                </CaseRow>

                <CaseRow title="inline style">
                    <Text class="inline-demo" style=TuiStyle::new().foreground(Color::LightCyan)>
                        "normal inline foreground beats normal .inline-demo"
                    </Text>
                </CaseRow>

                <CaseRow title="!important">
                    <Text class="important-demo" style=TuiStyle::new().foreground(Color::LightCyan)>
                        "important .important-demo beats normal inline foreground"
                    </Text>
                </CaseRow>

                <CaseRow title="inheritance">
                    <Block class="inheritance-parent">
                        <Text>
                            "child inherits foreground and bold, not border/padding/background declarations"
                        </Text>
                    </Block>
                </CaseRow>

                <CaseRow title="Button:focus">
                    <Div class="focus-actions">
                        <ActionButton label="Continue" on_press=|| AppControl::Continue />
                        <ActionButton label="Quit" on_press=|| AppControl::Exit />
                    </Div>
                </CaseRow>

                <Text class="intro">"Tab moves focus. Enter or Space activates. q exits."</Text>
            </Div>
        </Block>
    }
}

/// Runs the style cascade showcase example application.
#[tokio::main]
async fn main() -> Result<()> {
    App::new(StyleCascadeShowcase::new()).run().await
}

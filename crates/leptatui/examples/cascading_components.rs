//! Multi-component stylesheet cascade example.
//!
//! This binary demonstrates component tags, props, children, parent stylesheet
//! cascade through component boundaries, and component-local style overrides.

use leptatui::prelude::*;

/// Row of label/value status text.
#[component]
fn StatusRow(
    #[prop(into)] label: String,
    #[prop(into)] value: String,
    #[prop(into)] tone: String,
) -> View {
    view! {
        <Row class={format!("status-row {tone}")}>
            <Text class="status-label">{label}</Text>
            <Text class="status-value">{value}</Text>
        </Row>
    }
}

/// Metric row with local styles that override the parent cascade.
#[component]
fn MetricBadge(#[prop(into)] label: String, #[prop(into)] value: String) -> View {
    stylesheet! {
        .metric-badge => {
            Text => { fg: Color::DarkGray }
            .metric-value => { fg: Color::LightCyan, modifier: Modifier::BOLD }
        }
    }

    view! {
        <Row class="metric-badge">
            <Text class="metric-label">{label}</Text>
            <Text class="metric-value">{value}</Text>
        </Row>
    }
}

/// Reusable action button component.
#[component]
fn CommandButton(#[prop(into)] label: String, on_press: fn() -> AppControl) -> View {
    view! {
        <Button class="command-button" on_press={on_press}>{label}</Button>
    }
}

/// Panel component that receives nested child views.
#[component]
fn Panel(#[prop(into)] title: String, children: Children) -> View {
    view! {
        <Block class="panel">
            <Column>
                <Text class="panel-title">{title}</Text>
                {column(children())}
            </Column>
        </Block>
    }
}

/// Root component for the cascading styles example.
#[component]
fn CascadeDemo() -> View {
    use_key_event(KeyEventKind::Press, |key| {
        if key.code == KeyCode::Char('q') {
            return KeyControl::Exit;
        }

        KeyControl::Pass
    });

    stylesheet! {
        .dashboard => {
            fg: Color::White,
            bg: Color::Black,
            border_type: BorderType::Rounded,
            padding: TuiSpacing::uniform(1),

            Text => { fg: Color::Gray }
            .dashboard-title => { fg: Color::Yellow, modifier: Modifier::BOLD }
            .panel => {
                fg: Color::White,
                border_type: BorderType::Rounded,
                padding: TuiSpacing::uniform(1),

                Text => { fg: Color::White }
                .panel-title => { fg: Color::LightBlue, modifier: Modifier::BOLD }
                .status-label => { fg: Color::Gray }
                .status-value => { modifier: Modifier::BOLD }
                .success => { fg: Color::LightGreen }
                .warning => { fg: Color::Yellow }
                .danger => { fg: Color::LightRed }
            }

            Button => {
                fg: Color::White,
                bg: Color::Black,
                border_type: BorderType::Rounded,

                &:focus => {
                    fg: Color::Black,
                    bg: Color::Yellow,
                    modifier: Modifier::BOLD,
                    border_type: BorderType::Thick
                }
            }
        }
    }

    view! {
        <Block class="dashboard">
            <Column>
                <Text class="dashboard-title">"Cascading component styles"</Text>
                <Panel title="Service status">
                    <StatusRow label="API" value="Healthy" tone="success" />
                    <StatusRow label="Queue" value="Backlogged" tone="warning" />
                    <StatusRow label="Deploy" value="Paused" tone="danger" />
                    <MetricBadge label="Latency" value="42ms" />
                </Panel>
                <Panel title="Controls">
                    <Row>
                        <CommandButton label="Refresh" on_press=|| AppControl::Continue />
                        <CommandButton label="Quit" on_press=|| AppControl::Exit />
                    </Row>
                </Panel>
                <Text class="footer">"Tab moves focus. Enter or Space activates. q exits."</Text>
            </Column>
        </Block>
    }
}

/// Runs the cascading components example application.
#[tokio::main]
async fn main() -> Result<()> {
    App::new(CascadeDemo::new()).run().await
}

//! Responsive CSS Grid dashboard example.
//!
//! This binary demonstrates repeated fractional tracks, explicit spanning,
//! gaps, intrinsic rows, and a media query that stacks dashboard panels on
//! narrow terminals.

use leptatui::prelude::*;

/// Creates one fractionally sized explicit grid track.
///
/// # Arguments
///
/// * `fraction` — Fractional weight assigned to the track.
///
/// # Returns
///
/// A [`GridTemplateTrack`] containing the fractional weight.
fn fractional_track(fraction: f32) -> GridTemplateTrack {
    GridTemplateTrack::from(GridTrackSize::from(Fraction::new(fraction)))
}

/// Creates one automatically sized explicit grid track.
///
/// # Returns
///
/// A [`GridTemplateTrack`] using intrinsic automatic sizing.
fn automatic_track() -> GridTemplateTrack {
    GridTemplateTrack::from(GridTrackSize::Auto)
}

/// Root component for the responsive grid dashboard.
#[component]
fn ResponsiveGrid() -> impl IntoView {
    use_key_event(KeyEventKind::Press, |key| {
        if key.code == KeyCode::Char('q') {
            return KeyControl::Exit;
        }

        KeyControl::Pass
    });

    stylesheet! {
        .app-shell => {
            border_type: BorderType::Rounded,
            padding: TuiSpacing::uniform(1)
        }
        .app-title => {
            fg: Color::LightCyan,
            modifier: Modifier::BOLD
        }
        .dashboard => {
            display: Display::Grid,
            grid_template_columns: vec![
                GridTemplateTrack::repeat(
                    GridRepeat::count(2),
                    vec![GridTrackSize::from(Fraction::new(1.0))]
                )
            ],
            grid_template_rows: vec![
                automatic_track(),
                automatic_track()
            ],
            gap: Axes::all(Length::cells(1.0)),
            align_items: AlignItems::Stretch
        }
        .dashboard-title => {
            grid_column: GridLine::new(
                GridPlacement::line(1),
                GridPlacement::line(-1)
            )
        }
        .summary => {
            padding: TuiSpacing::uniform(1)
        }
        .activity => {
            padding: TuiSpacing::uniform(1)
        }
        .section-title => {
            fg: Color::Yellow,
            modifier: Modifier::BOLD
        }
        .metric => {
            fg: Color::LightGreen,
            modifier: Modifier::BOLD
        }
        .hint => { fg: Color::DarkGray }

        @media (max-width: 60) {
            .app-shell => {
                border_type: BorderType::Plain,
                padding: TuiSpacing::ZERO
            }
            .dashboard => {
                grid_template_columns: vec![fractional_track(1.0)],
                grid_template_rows: vec![
                    automatic_track(),
                    automatic_track(),
                    automatic_track()
                ]
            }
            .summary => { padding: TuiSpacing::ZERO }
            .activity => { padding: TuiSpacing::ZERO }
        }
    }

    view! {
        <Block class="app-shell">
            <Div>
                <Div class="dashboard">
                    <Div class="dashboard-title">
                        <Text class="app-title">"Leptatui grid dashboard"</Text>
                        <Text class="hint">
                            "Resize below 61 columns to stack the dashboard."
                        </Text>
                    </Div>
                    <Block class="summary">
                        <Div>
                            <Text class="section-title">"Weekly summary"</Text>
                            <Text class="metric">"$24,800 revenue"</Text>
                            <Text>"128 active projects"</Text>
                        </Div>
                    </Block>
                    <Block class="activity">
                        <Div>
                            <Text class="section-title">"Recent activity"</Text>
                            <Text>"Deploy completed"</Text>
                            <Text>"Three reviews ready"</Text>
                        </Div>
                    </Block>
                </Div>
                <Text class="hint">"Press q to quit."</Text>
            </Div>
        </Block>
    }
}

/// Runs the responsive grid dashboard application.
///
/// # Returns
///
/// An empty [`Result`] when the application exits successfully.
///
/// # Errors
///
/// Returns [`Error::Io`] if terminal setup, rendering, input, or cleanup fails.
/// Returns [`Error::EventTask`] if the blocking event task fails.
#[tokio::main]
async fn main() -> Result<()> {
    App::new(ResponsiveGrid::new()).run().await
}

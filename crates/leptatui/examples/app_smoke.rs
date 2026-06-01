use crossterm::event::{Event, KeyCode, KeyEventKind};
use leptatui::prelude::*;
use ratatui::{Frame, widgets::Paragraph};

struct Root;

impl AppRoot for Root {
    fn render(&mut self, frame: &mut Frame<'_>) -> Result<()> {
        frame.render_widget(
            Paragraph::new("Leptatui smoke runner. Press q to quit."),
            frame.area(),
        );
        Ok(())
    }

    fn handle_event(&mut self, event: Event) -> Result<AppControl> {
        if matches!(
            event,
            Event::Key(key)
                if key.kind == KeyEventKind::Press
                    && matches!(key.code, KeyCode::Char('q') | KeyCode::Esc)
        ) {
            return Ok(AppControl::Exit);
        }

        Ok(AppControl::Continue)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    App::new(Root).run().await
}

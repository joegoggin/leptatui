use crossterm::event::{Event, KeyCode, KeyEventKind};
use leptatui::prelude::*;

struct Root;

impl Component for Root {
    fn render(&mut self, ctx: &mut RenderCtx<'_, '_>) -> Result<()> {
        ctx.render_node(&block(column([
            text("Leptatui smoke runner. Press q or Esc to quit."),
            button("Quit"),
        ])))
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

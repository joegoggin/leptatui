use std::time::Duration;

use crossterm::event::Event;
use leptatui::{App, AppControl, AppRoot, Result};
use ratatui::Frame;

struct TestRoot {
    events: usize,
}

impl AppRoot for TestRoot {
    fn render(&mut self, _frame: &mut Frame<'_>) -> Result<()> {
        Ok(())
    }

    fn handle_event(&mut self, _event: Event) -> Result<AppControl> {
        self.events += 1;
        Ok(AppControl::Exit)
    }
}

#[test]
fn app_accepts_root_component_contract() {
    let _app = App::new(TestRoot { events: 0 }).with_redraw_interval(Duration::from_millis(50));
}

#[test]
fn app_control_is_comparable() {
    assert_eq!(AppControl::Continue, AppControl::Continue);
    assert_ne!(AppControl::Continue, AppControl::Exit);
}

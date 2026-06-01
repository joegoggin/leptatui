use std::time::Duration;

use crossterm::event::Event;
use leptatui::{App, AppControl, AppRoot, Component, RenderCtx, Result};

struct TestRoot {
    events: usize,
}

impl Component for TestRoot {
    fn render(&mut self, _ctx: &mut RenderCtx<'_, '_>) -> Result<()> {
        Ok(())
    }

    fn handle_event(&mut self, _event: Event) -> Result<AppControl> {
        self.events += 1;
        Ok(AppControl::Exit)
    }
}

#[test]
fn app_accepts_component_contract() {
    fn assert_app_root<R: AppRoot>(root: R) {
        let _app = App::new(root).with_redraw_interval(Duration::from_millis(50));
    }

    assert_app_root(TestRoot { events: 0 });
}

#[test]
fn app_control_is_comparable() {
    assert_eq!(AppControl::Continue, AppControl::Continue);
    assert_ne!(AppControl::Continue, AppControl::Exit);
}

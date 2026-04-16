use gpui::{App, Application, Bounds, KeyBinding, Window, WindowBounds, WindowOptions, px, size};

use crate::state::PomoAppState;
use crate::view::{CloseWindow, RootView};

pub struct PomoApp;

impl PomoApp {
    pub fn run() {
        Application::new().run(|app: &mut App| {
            app.set_global(PomoAppState::default());
            app.bind_keys([KeyBinding::new("cmd-w", CloseWindow, None)]);
            app.on_window_closed(|app| {
                if app.windows().is_empty() {
                    app.quit();
                }
            })
            .detach();

            let bounds = Bounds::centered(None, size(px(500.), px(500.0)), app);
            let window_options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            };

            app.open_window(window_options, |window: &mut Window, app: &mut App| {
                RootView::new(app, window)
            })
            .unwrap();

            app.activate(true);
        });
    }
}

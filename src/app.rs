use gpui::{
    App, Application, Bounds, KeyBinding, TitlebarOptions, Window, WindowBounds, WindowOptions,
    px, size,
};

use crate::config::Config;
use crate::state::PomoAppState;
use crate::view::{CloseWindow, RootView};

pub struct PomoApp;

impl PomoApp {
    pub fn run() {
        Application::new().run(|app: &mut App| {
            let config = Config::load();
            app.set_global(PomoAppState {
                focus_minutes: config.focus_minutes,
                break_minutes: config.break_minutes,
                total_sessions: config.total_sessions,
                millis_left: config.focus_minutes as u64 * 60_000,
                ..Default::default()
            });
            app.bind_keys([KeyBinding::new("cmd-w", CloseWindow, None)]);
            app.on_window_closed(|app| {
                if app.windows().is_empty() {
                    app.quit();
                }
            })
            .detach();

            let bounds = Bounds::centered(None, size(px(350.), px(350.0)), app);
            let window_options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                is_resizable: false,
                titlebar: Some(TitlebarOptions {
                    title: Some("Pomo".into()),
                    ..Default::default()
                }),
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

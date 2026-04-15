use std::time::Duration;

use gpui::{
    App, Application, Bounds, ClickEvent, Context, Entity, FocusHandle, Global, KeyBinding, Window, WindowBounds, WindowOptions, actions, div, prelude::*, px, size
};

actions!(close, [CloseWindow]);
actions!(start, [Toggle]);

pub struct PomoAppState {
    pub running: bool,
    pub seconds: u64,
    pub seconds_left: u64,
}
impl Global for PomoAppState {}

impl Default for PomoAppState {
    fn default() -> Self {
        Self {
            running: false,
            seconds: 600,
            seconds_left: 600,
        }
    }
}

fn _button(text: &str, callback: impl Fn(&mut Window, &mut App) + 'static) -> impl IntoElement {
    div()
        .id(text.to_string())
        .flex_none()
        .px_2()
        .bg(gpui::rgb(0xf7f7f7))
        .active(|this| this.opacity(0.85))
        .border_1()
        .border_color(gpui::rgb(0xe0e0e0))
        .rounded_sm()
        .cursor_pointer()
        .child(text.to_string())
         .on_click(move |_, window, cx| callback(window, cx))
}

pub struct PomoApp {}

pub struct RootView {
    focus_handle: FocusHandle
}

impl RootView {
    pub fn new(cx: &mut App, window: &mut Window) -> Entity<RootView> {
        cx.new(|cx|
            {
                let focus_handle = cx.focus_handle();
                focus_handle.focus(window);
                Self {focus_handle}
            })
    }
    pub fn reset_timer(&mut self, _event: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
        let state = cx.global_mut::<PomoAppState>();
        if state.seconds_left != 0 {
            return
        }
        state.seconds_left = state.seconds;
        state.running = false;
        cx.notify();
    }

    pub fn start_timer(&mut self, _event: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
        let state = cx.global_mut::<PomoAppState>();
        state.running = !state.running;
        let is_now_running = state.running;

        if state.seconds_left == 0 {
            // state.seconds_left = state.seconds;
            return
        }

        if is_now_running {
            cx.spawn(async |this, cx| {
                loop {
                    let should_continue = this.update(cx, |_, cx| {
                        let running = cx.global::<PomoAppState>().running;
                        if running {
                            let state = cx.global_mut::<PomoAppState>();
                            if state.seconds_left > 0 {
                                state.seconds_left -= 1;
                            } else {
                                state.running = false;
                            }
                            cx.notify();
                        }
                        running
                    });

                    match should_continue {
                        Ok(true) => {}
                        _ =>  break,
                    }

                    cx.background_executor()
                        .timer(Duration::from_millis(10))
                        .await;
                }
            }).detach();
        } 
    }

}

impl Render for RootView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let app_state = cx.global::<PomoAppState>();

        let seconds = app_state.seconds;
        let seconds_left = app_state.seconds_left;
        let label = match app_state.running {
            true=> "Pause",
            false => {
                if seconds_left == 0 {"Reset"}
                else if seconds_left == seconds {"Start"}
                else {"Continue"}
            }
        };


        div()
            .on_action(|_: &CloseWindow, window, _| {
                window.remove_window();
            })
            .track_focus(&self.focus_handle)
            .flex()
            .flex_col()
            .id("root")
            .size_full()
            .bg(gpui::white())
            .justify_center()
            .items_center()
            .text_xl()
            .font_weight(gpui::FontWeight(900.))
            .text_color(gpui::green())
            .child({
                let minutes = seconds_left / 60;
                let seconds = seconds_left % 60;
                format!("Time {:02}:{:02}", minutes, seconds)

            })
            .child(
                div()
                .id("btn_toggle".to_string())
                .flex_none()
                .px_2()
                .bg(gpui::rgb(0xf7f7f7))
                .active(|this| this.opacity(0.85))
                .border_1()
                .border_color(gpui::rgb(0xe0e0e0))
                .rounded_sm()
                .cursor_pointer()
                .child(label)
                .on_click( cx.listener(Self::start_timer))
                .on_click( cx.listener(Self::reset_timer))
            )
    }
}

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


            app.open_window(window_options, |window, app| RootView::new(app, window))
                .unwrap();

            app.activate(true);
        });
    }
}

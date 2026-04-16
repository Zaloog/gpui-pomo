use std::time::Duration;

use gpui::{App, ClickEvent, Context, Entity, FocusHandle, Window, actions, div, prelude::*, px};

use crate::state::{PomoAppState, TOTAL_SESSIONS};

actions!(close, [CloseWindow]);

pub struct RootView {
    focus_handle: FocusHandle,
}

impl RootView {
    pub fn new(cx: &mut App, window: &mut Window) -> Entity<RootView> {
        cx.new(|cx| {
            let focus_handle = cx.focus_handle();
            focus_handle.focus(window);
            Self { focus_handle }
        })
    }

    pub fn toggle_timer(&mut self, _: &ClickEvent, _: &mut Window, cx: &mut Context<Self>) {
        let state = cx.global_mut::<PomoAppState>();

        if state.is_all_done() {
            *state = PomoAppState::default();
            cx.notify();
            return;
        }

        state.running = !state.running;

        if state.running {
            self.start_timer(cx);
        }
    }

    fn start_timer(&self, cx: &mut Context<Self>) {
        cx.spawn(async |this, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(10))
                    .await;

                let should_continue = this.update(cx, |_, cx| {
                    let running = {
                        let state = cx.global_mut::<PomoAppState>();
                        if state.running {
                            state.tick();
                        }
                        state.running
                    };
                    if running {
                        cx.notify();
                    }
                    running
                });

                match should_continue {
                    Ok(true) => {}
                    _ => break,
                }
            }
        })
        .detach();
    }

    fn accent_color(is_break: bool) -> gpui::Hsla {
        if is_break {
            gpui::rgb(0x4A90D9).into()
        } else {
            gpui::green()
        }
    }

    fn phase_label(all_done: bool, is_break: bool) -> &'static str {
        if all_done {
            "All Done!"
        } else if is_break {
            "Break"
        } else {
            "Focus"
        }
    }

    fn button_label(all_done: bool, running: bool, seconds_left: u64, phase_seconds: u64) -> &'static str {
        if all_done {
            "Restart"
        } else if running {
            "Pause"
        } else if seconds_left == phase_seconds {
            "Start"
        } else {
            "Continue"
        }
    }
}

impl Render for RootView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = cx.global::<PomoAppState>();

        let seconds_left = state.seconds_left;
        let is_break = state.is_break;
        let sessions_completed = state.sessions_completed;
        let running = state.running;
        let all_done = state.is_all_done();
        let phase_seconds = state.phase_seconds();

        let accent = Self::accent_color(is_break);
        let phase_label = Self::phase_label(all_done, is_break);
        let button_label = Self::button_label(all_done, running, seconds_left, phase_seconds);

        div()
            .on_action(|_: &CloseWindow, window, _| window.remove_window())
            .track_focus(&self.focus_handle)
            .flex()
            .flex_col()
            .id("root")
            .size_full()
            .bg(gpui::white())
            .justify_center()
            .items_center()
            .gap_4()
            .child(
                div()
                    .text_xl()
                    .font_weight(gpui::FontWeight(600.))
                    .text_color(accent)
                    .child(phase_label),
            )
            .child(
                div()
                    .font_weight(gpui::FontWeight(900.))
                    .text_color(accent)
                    .text_size(px(64.))
                    .child(format!("{:02}:{:02}", seconds_left / 60, seconds_left % 60)),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap_2()
                    .items_center()
                    .children((0..TOTAL_SESSIONS as usize).map(|i| {
                        let completed = i < sessions_completed as usize;
                        let is_current = !all_done && !is_break && i == sessions_completed as usize;
                        div()
                            .w(px(16.))
                            .h(px(16.))
                            .rounded_full()
                            .bg(if completed {
                                gpui::green()
                            } else if is_current {
                                gpui::rgb(0xa8d5a2).into()
                            } else {
                                gpui::rgb(0xe0e0e0).into()
                            })
                    })),
            )
            .child(
                div()
                    .id("btn_toggle")
                    .mt_2()
                    .flex_none()
                    .px_4()
                    .py_1()
                    .text_base()
                    .font_weight(gpui::FontWeight(500.))
                    .bg(gpui::rgb(0xf7f7f7))
                    .active(|this| this.opacity(0.85))
                    .border_1()
                    .border_color(gpui::rgb(0xe0e0e0))
                    .rounded_sm()
                    .cursor_pointer()
                    .child(button_label)
                    .on_click(cx.listener(Self::toggle_timer)),
            )
    }
}

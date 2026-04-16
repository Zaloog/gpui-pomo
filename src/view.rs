use std::f32::consts::PI;
use std::time::Duration;

use gpui::{
    App, ClickEvent, Context, Entity, FocusHandle, PathBuilder, Window, actions, canvas, div,
    point, prelude::*, px,
};

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
                    .timer(Duration::from_millis(1))
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

    fn button_label(
        all_done: bool,
        running: bool,
        seconds_left: u64,
        phase_seconds: u64,
    ) -> &'static str {
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

/// Paints a donut (annular) arc sector from `start_angle` to `end_angle` (radians,
/// measured clockwise from the 12 o'clock position in screen coords).
/// `sweep` for the outer arc is always clockwise; the inner arc reverses.
fn paint_donut_arc(
    cx_f: f32,
    cy_f: f32,
    outer_r: f32,
    inner_r: f32,
    start: f32,
    end: f32,
    color: gpui::Hsla,
    window: &mut Window,
) {
    let span = end - start;
    if span.abs() < 0.001 {
        return;
    }

    let os = (cx_f + outer_r * start.cos(), cy_f + outer_r * start.sin());
    let oe = (cx_f + outer_r * end.cos(), cy_f + outer_r * end.sin());
    let ie = (cx_f + inner_r * end.cos(), cy_f + inner_r * end.sin());
    let is_ = (cx_f + inner_r * start.cos(), cy_f + inner_r * start.sin());

    let large_arc = span > PI;

    let mut b = PathBuilder::fill();
    b.move_to(point(px(os.0), px(os.1)));
    b.arc_to(
        point(px(outer_r), px(outer_r)),
        px(0.),
        large_arc,
        true, // clockwise on screen
        point(px(oe.0), px(oe.1)),
    );
    b.line_to(point(px(ie.0), px(ie.1)));
    b.arc_to(
        point(px(inner_r), px(inner_r)),
        px(0.),
        large_arc,
        false, // counter-clockwise on screen (reverse inner edge)
        point(px(is_.0), px(is_.1)),
    );
    b.close();

    if let Ok(path) = b.build() {
        window.paint_path(path, color);
    }
}

/// Paints a complete ring track by drawing two half-donuts to avoid the
/// degenerate case where start == end in `arc_to`.
fn paint_ring_track(cx_f: f32, cy_f: f32, outer_r: f32, inner_r: f32, window: &mut Window) {
    let gray: gpui::Hsla = gpui::rgb(0xe5e5e5).into();
    // Right half: top → bottom (clockwise through right)
    paint_donut_arc(
        cx_f,
        cy_f,
        outer_r,
        inner_r,
        -PI / 2.,
        PI / 2.,
        gray,
        window,
    );
    // Left half: bottom → top (clockwise through left)
    paint_donut_arc(
        cx_f,
        cy_f,
        outer_r,
        inner_r,
        PI / 2.,
        3. * PI / 2.,
        gray,
        window,
    );
}

/// Paints a progress arc starting at 12 o'clock, extending clockwise by `progress` (0..=1).
fn paint_ring_progress(
    cx_f: f32,
    cy_f: f32,
    outer_r: f32,
    inner_r: f32,
    progress: f32,
    color: gpui::Hsla,
    window: &mut Window,
) {
    if progress < 0.001 {
        return;
    }
    if progress > 0.999 {
        // Full ring: same two-halves trick
        paint_donut_arc(
            cx_f,
            cy_f,
            outer_r,
            inner_r,
            -PI / 2.,
            PI / 2.,
            color,
            window,
        );
        paint_donut_arc(
            cx_f,
            cy_f,
            outer_r,
            inner_r,
            PI / 2.,
            3. * PI / 2.,
            color,
            window,
        );
        return;
    }
    let end = -PI / 2. + progress * 2. * PI;
    paint_donut_arc(cx_f, cy_f, outer_r, inner_r, -PI / 2., end, color, window);
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
        let progress = 1.0 - seconds_left as f32 / phase_seconds as f32;

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
            .gap_6()
            // Ring container with progress arc and inner content
            .child(
                div()
                    .relative()
                    .w(px(300.))
                    .h(px(300.))
                    // Canvas draws the ring behind the content
                    .child(
                        canvas(
                            |_, _, _| {},
                            move |bounds, _, window, _| {
                                let w = f32::from(bounds.size.width);
                                let h = f32::from(bounds.size.height);
                                let cx_f = f32::from(bounds.origin.x) + w / 2.0;
                                let cy_f = f32::from(bounds.origin.y) + h / 2.0;
                                let outer_r = w / 2.0 - 4.0;
                                let inner_r = outer_r - 20.0;
                                paint_ring_track(cx_f, cy_f, outer_r, inner_r, window);
                                paint_ring_progress(
                                    cx_f, cy_f, outer_r, inner_r, progress, accent, window,
                                );
                            },
                        )
                        .absolute()
                        .top_0()
                        .left_0()
                        .size_full(),
                    )
                    // Content centered inside the ring
                    .child(
                        div()
                            .absolute()
                            .top_0()
                            .left_0()
                            .size_full()
                            .flex()
                            .flex_col()
                            .justify_center()
                            .items_center()
                            .gap_4()
                            // Phase label
                            .child(
                                div()
                                    .text_xl()
                                    .font_weight(gpui::FontWeight(600.))
                                    .text_color(accent)
                                    .child(phase_label),
                            )
                            // Timer display
                            .child(
                                div()
                                    .font_weight(gpui::FontWeight(900.))
                                    .text_color(accent)
                                    .text_size(px(64.))
                                    .child(format!(
                                        "{:02}:{:02}",
                                        seconds_left / 60,
                                        seconds_left % 60
                                    )),
                            )
                            // Session progress dots
                            .child(div().flex().flex_row().gap_2().items_center().children(
                                (0..TOTAL_SESSIONS as usize).map(|i| {
                                    let completed = i < sessions_completed as usize;
                                    let is_current =
                                        !all_done && !is_break && i == sessions_completed as usize;
                                    div().w(px(16.)).h(px(16.)).rounded_full().bg(if completed {
                                        gpui::green()
                                    } else if is_current {
                                        gpui::rgb(0xa8d5a2).into()
                                    } else {
                                        gpui::rgb(0xe0e0e0).into()
                                    })
                                }),
                            )),
                    ),
            )
            // Control button below the ring
            .child(
                div()
                    .id("btn_toggle")
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

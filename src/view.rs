use std::f32::consts::PI;
use std::time::Duration;

use gpui::{
    AnyElement, App, ClickEvent, Context, Entity, FocusHandle, PathBuilder, SharedString, Window,
    actions, canvas, div, point, prelude::*, px,
};

use crate::state::PomoAppState;

actions!(close, [CloseWindow]);

// ── Navigation state ──────────────────────────────────────────────────────────

#[derive(Clone, PartialEq)]
enum AppView {
    Timer,
    Settings,
    Edit(EditTarget),
}

#[derive(Clone, PartialEq)]
enum EditTarget {
    FocusMinutes,
    BreakMinutes,
    TotalSessions,
}

impl EditTarget {
    fn label(&self) -> &'static str {
        match self {
            EditTarget::FocusMinutes => "Focus Time",
            EditTarget::BreakMinutes => "Break Time",
            EditTarget::TotalSessions => "Total Sessions",
        }
    }

    fn unit(&self) -> &'static str {
        match self {
            EditTarget::FocusMinutes | EditTarget::BreakMinutes => "minutes",
            EditTarget::TotalSessions => "sessions",
        }
    }

    fn min(&self) -> u32 {
        1
    }

    fn max(&self) -> u32 {
        match self {
            EditTarget::FocusMinutes => 99,
            EditTarget::BreakMinutes => 30,
            EditTarget::TotalSessions => 10,
        }
    }

    fn value_display(&self, value: u32) -> String {
        match self {
            EditTarget::FocusMinutes | EditTarget::BreakMinutes => format!("{} min", value),
            EditTarget::TotalSessions => format!("{}", value),
        }
    }
}

// ── Pending settings (not yet applied, waiting for reset) ─────────────────────

#[derive(Clone)]
struct PendingSettings {
    focus_minutes: u32,
    break_minutes: u32,
    total_sessions: u8,
}


// ── View ──────────────────────────────────────────────────────────────────────

pub struct RootView {
    focus_handle: FocusHandle,
    current_view: AppView,
    edit_value: u32,
    pending: Option<PendingSettings>,
}

impl RootView {
    pub fn new(cx: &mut App, window: &mut Window) -> Entity<RootView> {
        cx.new(|cx| {
            let focus_handle = cx.focus_handle();
            focus_handle.focus(window);
            Self {
                focus_handle,
                current_view: AppView::Timer,
                edit_value: 0,
                pending: None,
            }
        })
    }

    // ── Timer actions ─────────────────────────────────────────────────────────

    fn toggle_timer(&mut self, _: &ClickEvent, _: &mut Window, cx: &mut Context<Self>) {
        let state = cx.global_mut::<PomoAppState>();

        if state.is_all_done() {
            *state = PomoAppState {
                focus_minutes: state.focus_minutes,
                break_minutes: state.break_minutes,
                total_sessions: state.total_sessions,
                seconds_left: state.focus_seconds(),
                ..Default::default()
            };
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

    fn reset_timer(&mut self, _: &ClickEvent, _: &mut Window, cx: &mut Context<Self>) {
        let (focus_minutes, break_minutes, total_sessions) = match &self.pending {
            Some(p) => (p.focus_minutes, p.break_minutes, p.total_sessions),
            None => {
                let s = cx.global::<PomoAppState>();
                (s.focus_minutes, s.break_minutes, s.total_sessions)
            }
        };
        self.pending = None;
        *cx.global_mut::<PomoAppState>() = PomoAppState {
            focus_minutes,
            break_minutes,
            total_sessions,
            seconds_left: focus_minutes as u64 * 60,
            ..Default::default()
        };
        cx.notify();
    }

    // ── Timer render helpers ──────────────────────────────────────────────────

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

    // ── Screen renderers ──────────────────────────────────────────────────────

    fn render_timer(&mut self, _: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let state = cx.global::<PomoAppState>();

        let seconds_left = state.seconds_left;
        let is_break = state.is_break;
        let sessions_completed = state.sessions_completed;
        let total_sessions = state.total_sessions;
        let running = state.running;
        let all_done = state.is_all_done();
        let phase_seconds = state.phase_seconds();

        let accent = Self::accent_color(is_break);
        let phase_label = Self::phase_label(all_done, is_break);
        let button_label = Self::button_label(all_done, running, seconds_left, phase_seconds);
        let progress = 1.0 - seconds_left as f32 / phase_seconds as f32;
        let has_pending = self.pending.is_some();

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
            // Ring container
            .child(
                div()
                    .relative()
                    .w(px(300.))
                    .h(px(300.))
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
                                    .child(format!(
                                        "{:02}:{:02}",
                                        seconds_left / 60,
                                        seconds_left % 60
                                    )),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_row()
                                    .gap_2()
                                    .items_center()
                                    .children((0..total_sessions as usize).map(|i| {
                                        let completed = i < sessions_completed as usize;
                                        let is_current = !all_done
                                            && !is_break
                                            && i == sessions_completed as usize;
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
                            ),
                    ),
            )
            // Button row: [ ↺ ]  [ Start/Pause ]  [ ⚙ ]
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap_2()
                    .items_center()
                    // Reset button
                    .child(
                        div()
                            .id("btn_reset")
                            .w(px(30.))
                            .h(px(30.))
                            .flex()
                            .justify_center()
                            .items_center()
                            .rounded_full()
                            .bg(gpui::rgb(0xf7f7f7))
                            .border_1()
                            .border_color(gpui::rgb(0xe0e0e0))
                            .cursor_pointer()
                            .active(|s| s.opacity(0.75))
                            .child("↺")
                            .on_click(cx.listener(Self::reset_timer)),
                    )
                    // Toggle button
                    .child(
                        div()
                            .id("btn_toggle")
                            .flex_none()
                            .px_4()
                            .py_1()
                            .text_base()
                            .font_weight(gpui::FontWeight(500.))
                            .bg(gpui::rgb(0xf7f7f7))
                            .active(|s| s.opacity(0.85))
                            .border_1()
                            .border_color(gpui::rgb(0xe0e0e0))
                            .rounded_sm()
                            .cursor_pointer()
                            .child(button_label)
                            .on_click(cx.listener(Self::toggle_timer)),
                    )
                    // Settings button with pending badge
                    .child(
                        div()
                            .id("btn_settings")
                            .relative()
                            .w(px(30.))
                            .h(px(30.))
                            .flex()
                            .justify_center()
                            .items_center()
                            .rounded_full()
                            .bg(gpui::rgb(0xf7f7f7))
                            .border_1()
                            .border_color(gpui::rgb(0xe0e0e0))
                            .cursor_pointer()
                            .active(|s| s.opacity(0.75))
                            .child("⚙")
                            .when(has_pending, |s| {
                                s.child(
                                    div()
                                        .absolute()
                                        .top(px(-3.))
                                        .right(px(-3.))
                                        .w(px(9.))
                                        .h(px(9.))
                                        .rounded_full()
                                        .bg(gpui::rgb(0xe53e3e)),
                                )
                            })
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.current_view = AppView::Settings;
                                cx.notify();
                            })),
                    ),
            )
            .into_any()
    }

    fn render_settings(&mut self, _: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        // Show pending values if any, otherwise current applied values
        let (focus_min, break_min, total_sess) = match &self.pending {
            Some(p) => (p.focus_minutes, p.break_minutes, p.total_sessions as u32),
            None => {
                let s = cx.global::<PomoAppState>();
                (s.focus_minutes, s.break_minutes, s.total_sessions as u32)
            }
        };
        let has_pending = self.pending.is_some();

        let rows: [(EditTarget, &'static str, u32); 3] = [
            (EditTarget::FocusMinutes, "Focus Time", focus_min),
            (EditTarget::BreakMinutes, "Break Time", break_min),
            (EditTarget::TotalSessions, "Total Sessions", total_sess),
        ];

        div()
            .on_action(|_: &CloseWindow, window, _| window.remove_window())
            .track_focus(&self.focus_handle)
            .id("settings")
            .size_full()
            .flex()
            .flex_col()
            .bg(gpui::white())
            // Header
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .px(px(20.))
                    .pt(px(20.))
                    .pb(px(12.))
                    .gap_3()
                    .child(
                        div()
                            .id("settings_back")
                            .cursor_pointer()
                            .text_xl()
                            .text_color(gpui::rgb(0x555555))
                            .active(|s| s.opacity(0.6))
                            .child("←")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.current_view = AppView::Timer;
                                cx.notify();
                            })),
                    )
                    .child(
                        div()
                            .text_xl()
                            .font_weight(gpui::FontWeight(700.))
                            .child("Settings"),
                    )
                    // Pending indicator in header
                    .when(has_pending, |s| {
                        s.child(
                            div()
                                .flex()
                                .flex_row()
                                .items_center()
                                .gap_1()
                                .child(
                                    div()
                                        .w(px(7.))
                                        .h(px(7.))
                                        .rounded_full()
                                        .bg(gpui::rgb(0xe53e3e)),
                                )
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(gpui::rgb(0xe53e3e))
                                        .child("pending reset"),
                                ),
                        )
                    }),
            )
            // Setting rows
            .children(rows.into_iter().map(|(target, label, value)| {
                let value_str = target.value_display(value);
                let id = SharedString::from(label);
                div()
                    .id(id)
                    .flex()
                    .flex_row()
                    .justify_between()
                    .items_center()
                    .px(px(24.))
                    .py(px(16.))
                    .border_b_1()
                    .border_color(gpui::rgb(0xf0f0f0))
                    .cursor_pointer()
                    .active(|s| s.bg(gpui::rgb(0xf5f5f5)))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.edit_value = value;
                        this.current_view = AppView::Edit(target.clone());
                        cx.notify();
                    }))
                    .child(div().text_base().child(label))
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .gap_2()
                            .items_center()
                            .child(
                                div()
                                    .text_base()
                                    .text_color(gpui::rgb(0x888888))
                                    .child(value_str),
                            )
                            .child(
                                div()
                                    .text_base()
                                    .text_color(gpui::rgb(0xcccccc))
                                    .child("›"),
                            ),
                    )
            }))
            .into_any()
    }

    fn render_edit(&mut self, _: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let target = match &self.current_view {
            AppView::Edit(t) => t.clone(),
            _ => return div().into_any(),
        };

        let value = self.edit_value;
        let at_min = value <= target.min();
        let at_max = value >= target.max();
        let unit = target.unit();
        let label = target.label();
        let min = target.min();
        let max = target.max();
        let target_for_back = target.clone();

        div()
            .on_action(|_: &CloseWindow, window, _| window.remove_window())
            .track_focus(&self.focus_handle)
            .id("edit")
            .size_full()
            .flex()
            .flex_col()
            .bg(gpui::white())
            // Header: back arrow + setting name
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .px(px(20.))
                    .pt(px(20.))
                    .pb(px(12.))
                    .gap_3()
                    .child(
                        div()
                            .id("edit_back")
                            .cursor_pointer()
                            .text_xl()
                            .text_color(gpui::rgb(0x555555))
                            .active(|s| s.opacity(0.6))
                            .child("←")
                            .on_click(cx.listener(move |this, _, _, cx| {
                                let new_value = this.edit_value;
                                // Read current state values for comparison
                                let (sf, sb, ss) = {
                                    let s = cx.global::<PomoAppState>();
                                    (s.focus_minutes, s.break_minutes, s.total_sessions)
                                };
                                // Update or initialize pending
                                {
                                    let p = this.pending.get_or_insert_with(|| PendingSettings {
                                        focus_minutes: sf,
                                        break_minutes: sb,
                                        total_sessions: ss,
                                    });
                                    match &target_for_back {
                                        EditTarget::FocusMinutes => p.focus_minutes = new_value,
                                        EditTarget::BreakMinutes => p.break_minutes = new_value,
                                        EditTarget::TotalSessions => {
                                            p.total_sessions = new_value as u8
                                        }
                                    }
                                }
                                // Clear pending if it now matches the applied state
                                if let Some(ref p) = this.pending {
                                    if p.focus_minutes == sf
                                        && p.break_minutes == sb
                                        && p.total_sessions == ss
                                    {
                                        this.pending = None;
                                    }
                                }
                                this.current_view = AppView::Settings;
                                cx.notify();
                            })),
                    )
                    .child(
                        div()
                            .text_xl()
                            .font_weight(gpui::FontWeight(700.))
                            .child(label),
                    ),
            )
            // Stepper centered
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .justify_center()
                    .items_center()
                    .gap_4()
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap(px(32.))
                            // Minus button
                            .child(
                                div()
                                    .id("btn_minus")
                                    .w(px(52.))
                                    .h(px(52.))
                                    .flex()
                                    .justify_center()
                                    .items_center()
                                    .rounded_full()
                                    .border_1()
                                    .border_color(if at_min {
                                        gpui::rgb(0xe5e5e5)
                                    } else {
                                        gpui::rgb(0xbbbbbb)
                                    })
                                    .text_size(px(24.))
                                    .font_weight(gpui::FontWeight(700.))
                                    .text_color(if at_min {
                                        gpui::rgb(0xcccccc)
                                    } else {
                                        gpui::rgb(0x333333)
                                    })
                                    .cursor_pointer()
                                    .when(!at_min, |s| s.active(|s| s.bg(gpui::rgb(0xf0f0f0))))
                                    .child("−")
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        if this.edit_value > min {
                                            this.edit_value -= 1;
                                            cx.notify();
                                        }
                                    })),
                            )
                            // Current value
                            .child(
                                div()
                                    .w(px(80.))
                                    .flex()
                                    .justify_center()
                                    .text_size(px(52.))
                                    .font_weight(gpui::FontWeight(700.))
                                    .child(format!("{}", value)),
                            )
                            // Plus button
                            .child(
                                div()
                                    .id("btn_plus")
                                    .w(px(52.))
                                    .h(px(52.))
                                    .flex()
                                    .justify_center()
                                    .items_center()
                                    .rounded_full()
                                    .border_1()
                                    .border_color(if at_max {
                                        gpui::rgb(0xe5e5e5)
                                    } else {
                                        gpui::rgb(0xbbbbbb)
                                    })
                                    .text_size(px(24.))
                                    .font_weight(gpui::FontWeight(700.))
                                    .text_color(if at_max {
                                        gpui::rgb(0xcccccc)
                                    } else {
                                        gpui::rgb(0x333333)
                                    })
                                    .cursor_pointer()
                                    .when(!at_max, |s| s.active(|s| s.bg(gpui::rgb(0xf0f0f0))))
                                    .child("+")
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        if this.edit_value < max {
                                            this.edit_value += 1;
                                            cx.notify();
                                        }
                                    })),
                            ),
                    )
                    .child(
                        div()
                            .text_color(gpui::rgb(0x999999))
                            .child(unit),
                    ),
            )
            .into_any()
    }
}

// ── Ring drawing ──────────────────────────────────────────────────────────────

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
        true,
        point(px(oe.0), px(oe.1)),
    );
    b.line_to(point(px(ie.0), px(ie.1)));
    b.arc_to(
        point(px(inner_r), px(inner_r)),
        px(0.),
        large_arc,
        false,
        point(px(is_.0), px(is_.1)),
    );
    b.close();

    if let Ok(path) = b.build() {
        window.paint_path(path, color);
    }
}

fn paint_ring_track(cx_f: f32, cy_f: f32, outer_r: f32, inner_r: f32, window: &mut Window) {
    let gray: gpui::Hsla = gpui::rgb(0xe5e5e5).into();
    paint_donut_arc(cx_f, cy_f, outer_r, inner_r, -PI / 2., PI / 2., gray, window);
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
        paint_donut_arc(cx_f, cy_f, outer_r, inner_r, -PI / 2., PI / 2., color, window);
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

// ── Render dispatch ───────────────────────────────────────────────────────────

impl Render for RootView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = self.current_view.clone();
        match view {
            AppView::Timer => self.render_timer(window, cx),
            AppView::Settings => self.render_settings(window, cx),
            AppView::Edit(_) => self.render_edit(window, cx),
        }
    }
}

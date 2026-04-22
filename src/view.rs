use std::f32::consts::PI;
use std::time::Duration;

use gpui::{
    AnyElement, App, ClickEvent, Context, Entity, FocusHandle, KeyDownEvent, PathBuilder,
    SharedString, Window, actions, canvas, div, point, prelude::*, px,
};

use crate::config::Config;
use crate::state::PomoAppState;

actions!(close, [CloseWindow]);

// ── Color palette ─────────────────────────────────────────────────────────────

const BG: u32 = 0x1c1612;
const SURFACE: u32 = 0x262018;
const SURFACE_ACTIVE: u32 = 0x342c24;
const BORDER: u32 = 0x3d3328;
const ACCENT_FOCUS: u32 = 0xf97316;
const ACCENT_BREAK: u32 = 0xf59e0b;
const TEXT_PRIMARY: u32 = 0xf5ede0;
const TEXT_SECONDARY: u32 = 0x9c8977;
const TEXT_MUTED: u32 = 0x5a4f44;
const SESSION_DONE: u32 = 0xf97316;
const SESSION_CURRENT: u32 = 0xa64a1a;
const SESSION_IDLE: u32 = 0x3d3328;
const RED: u32 = 0xef4444;

fn col(hex: u32) -> gpui::Hsla {
    gpui::rgb(hex).into()
}

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

// ── Pending settings ──────────────────────────────────────────────────────────

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
    is_editing_value: bool,
    input_text: String,
    selected_settings_row: usize,
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
                is_editing_value: false,
                input_text: String::new(),
                selected_settings_row: 0,
            }
        })
    }

    // ── Timer actions ─────────────────────────────────────────────────────────

    fn do_toggle(&mut self, cx: &mut Context<Self>) {
        let state = cx.global_mut::<PomoAppState>();

        if state.is_all_done() {
            *state = PomoAppState {
                focus_minutes: state.focus_minutes,
                break_minutes: state.break_minutes,
                total_sessions: state.total_sessions,
                millis_left: state.focus_millis(),
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

    fn toggle_timer(&mut self, _: &ClickEvent, _: &mut Window, cx: &mut Context<Self>) {
        self.do_toggle(cx);
    }

    fn start_timer(&self, cx: &mut Context<Self>) {
        cx.spawn(async |this, cx| {
            loop {
                let delta_ms = 16;
                cx.background_executor()
                    .timer(Duration::from_millis(delta_ms))
                    .await;

                let should_continue = this.update(cx, |_, cx| {
                    let (running, phase_switched) = {
                        let state = cx.global_mut::<PomoAppState>();
                        let switched = if state.running {
                            state.tick(delta_ms * 100)
                        } else {
                            false
                        };
                        (state.running, switched)
                    };
                    cx.notify();
                    (running, phase_switched)
                });

                match should_continue {
                    Ok((running, switched)) => {
                        if switched {
                            let _ = cx.update(|app| app.activate(true));
                        }
                        if !running {
                            break;
                        }
                    }
                    _ => break,
                }
            }
        })
        .detach();
    }

    fn commit_input(&mut self) {
        if self.is_editing_value && !self.input_text.is_empty() {
            if let Ok(v) = self.input_text.parse::<u32>() {
                let (min, max) = match &self.current_view {
                    AppView::Edit(t) => (t.min(), t.max()),
                    _ => (1, 99),
                };
                self.edit_value = v.clamp(min, max);
            }
        }
        self.is_editing_value = false;
        self.input_text = self.edit_value.to_string();
    }

    fn do_reset(&mut self, cx: &mut Context<Self>) {
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
            millis_left: focus_minutes as u64 * 60_000,
            ..Default::default()
        };
        Config {
            focus_minutes,
            break_minutes,
            total_sessions,
        }
        .save();
        cx.notify();
    }

    fn reset_timer(&mut self, _: &ClickEvent, _: &mut Window, cx: &mut Context<Self>) {
        self.do_reset(cx);
    }

    fn commit_edit_and_go_back(&mut self, cx: &mut Context<Self>) {
        self.commit_input();
        let new_value = self.edit_value;
        let target = match self.current_view.clone() {
            AppView::Edit(t) => t,
            _ => return,
        };

        let pristine = {
            let state = cx.global::<PomoAppState>();
            !state.running
                && state.sessions_completed == 0
                && !state.is_break
                && state.millis_left == state.focus_millis()
        };

        if pristine {
            let (focus_minutes, break_minutes, total_sessions) = {
                let state = cx.global_mut::<PomoAppState>();
                match &target {
                    EditTarget::FocusMinutes => {
                        state.focus_minutes = new_value;
                        state.millis_left = state.focus_millis();
                    }
                    EditTarget::BreakMinutes => state.break_minutes = new_value,
                    EditTarget::TotalSessions => state.total_sessions = new_value as u8,
                }
                (
                    state.focus_minutes,
                    state.break_minutes,
                    state.total_sessions,
                )
            };
            self.pending = None;
            Config {
                focus_minutes,
                break_minutes,
                total_sessions,
            }
            .save();
        } else {
            let (sf, sb, ss) = {
                let state = cx.global::<PomoAppState>();
                (
                    state.focus_minutes,
                    state.break_minutes,
                    state.total_sessions,
                )
            };
            {
                let p = self.pending.get_or_insert_with(|| PendingSettings {
                    focus_minutes: sf,
                    break_minutes: sb,
                    total_sessions: ss,
                });
                match &target {
                    EditTarget::FocusMinutes => p.focus_minutes = new_value,
                    EditTarget::BreakMinutes => p.break_minutes = new_value,
                    EditTarget::TotalSessions => p.total_sessions = new_value as u8,
                }
            }
            if let Some(ref p) = self.pending {
                if p.focus_minutes == sf && p.break_minutes == sb && p.total_sessions == ss {
                    self.pending = None;
                }
            }
        }

        self.current_view = AppView::Settings;
        cx.notify();
    }

    // ── Timer render helpers ──────────────────────────────────────────────────

    fn accent_color(is_break: bool) -> gpui::Hsla {
        col(if is_break { ACCENT_BREAK } else { ACCENT_FOCUS })
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
        millis_left: u64,
        phase_millis: u64,
    ) -> &'static str {
        if all_done {
            "Restart"
        } else if running {
            "Pause"
        } else if millis_left >= phase_millis {
            "Start"
        } else {
            "Continue"
        }
    }

    // ── Screen renderers ──────────────────────────────────────────────────────

    fn render_timer(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let state = cx.global::<PomoAppState>();

        let millis_left = state.millis_left;
        let is_break = state.is_break;
        let sessions_completed = state.sessions_completed;
        let total_sessions = state.total_sessions;
        let running = state.running;
        let all_done = state.is_all_done();
        let phase_millis = state.phase_millis();
        let progress = state.progress();
        let secs = state.seconds_display();

        let accent = Self::accent_color(is_break);
        let phase_label = Self::phase_label(all_done, is_break);
        let button_label = Self::button_label(all_done, running, millis_left, phase_millis);
        let has_pending = self.pending.is_some();

        // Dynamic window title
        let title = if all_done {
            "Pomo · All Done!".to_string()
        } else {
            let phase = if is_break { "Break" } else { "Focus" };
            format!("Pomo · {:02}:{:02} {}", secs / 60, secs % 60, phase)
        };
        window.set_window_title(&title);

        div()
            .on_action(|_: &CloseWindow, window, _| window.remove_window())
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                match event.keystroke.key.as_str() {
                    "space" => this.do_toggle(cx),
                    "r" => this.do_reset(cx),
                    "s" => {
                        this.current_view = AppView::Settings;
                        cx.notify();
                    }
                    _ => {}
                }
            }))
            .flex()
            .flex_col()
            .id("root")
            .size_full()
            .bg(col(BG))
            .items_center()
            // Center ring + buttons, pin hints at bottom
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .justify_center()
                    .items_center()
                    .gap_6()
                    // Ring container
                    .child(
                        div()
                            .relative()
                            .w(px(280.))
                            .h(px(280.))
                            .child(
                                canvas(
                                    |_, _, _| {},
                                    move |bounds, _, window, _| {
                                        let width = f32::from(bounds.size.width);
                                        let height = f32::from(bounds.size.height);
                                        // Calculate Center Position
                                        let center_x = f32::from(bounds.origin.x) + width / 2.0;
                                        let center_y = f32::from(bounds.origin.y) + height / 2.0;
                                        let outer_radius = width / 2.0 - 4.0;
                                        let inner_radius = outer_radius - 18.0;
                                        paint_ring_track(
                                            center_x,
                                            center_y,
                                            outer_radius,
                                            inner_radius,
                                            window,
                                        );
                                        paint_ring_progress(
                                            center_x,
                                            center_y,
                                            outer_radius,
                                            inner_radius,
                                            progress,
                                            accent,
                                            window,
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
                                            .text_size(px(60.))
                                            .child(format!("{:02}:{:02}", secs / 60, secs % 60)),
                                    )
                                    .child(
                                        div().flex().flex_row().gap_2().items_center().children(
                                            (0..total_sessions as usize).map(|i| {
                                                let completed = i < sessions_completed as usize;
                                                let is_current = !all_done
                                                    && !is_break
                                                    && i == sessions_completed as usize;
                                                div().w(px(9.)).h(px(24.)).rounded_full().bg(
                                                    if completed {
                                                        col(SESSION_DONE)
                                                    } else if is_current {
                                                        col(SESSION_CURRENT)
                                                    } else {
                                                        col(SESSION_IDLE)
                                                    },
                                                )
                                            }),
                                        ),
                                    ),
                            ),
                    )
                    // Button row
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .gap_2()
                            .items_center()
                            .w(px(280.))
                            .child(
                                div()
                                    .id("btn_reset")
                                    .w(px(40.))
                                    .h(px(40.))
                                    .flex()
                                    .justify_center()
                                    .items_center()
                                    .rounded_full()
                                    .bg(col(SURFACE))
                                    .border_1()
                                    .border_color(col(BORDER))
                                    .text_color(col(TEXT_SECONDARY))
                                    .cursor_pointer()
                                    .active(|s| s.bg(col(SURFACE_ACTIVE)))
                                    .child("↺")
                                    .on_click(cx.listener(Self::reset_timer)),
                            )
                            .child(
                                div()
                                    .id("btn_toggle")
                                    .flex_1()
                                    .h(px(40.))
                                    .flex()
                                    .justify_center()
                                    .items_center()
                                    .text_base()
                                    .font_weight(gpui::FontWeight(600.))
                                    .bg(col(ACCENT_FOCUS))
                                    .text_color(col(BG))
                                    .active(|s| s.opacity(0.85))
                                    .border_1()
                                    .border_color(col(ACCENT_FOCUS))
                                    .rounded_full()
                                    .cursor_pointer()
                                    .child(button_label)
                                    .on_click(cx.listener(Self::toggle_timer)),
                            )
                            .child(
                                div()
                                    .id("btn_settings")
                                    .relative()
                                    .h(px(40.))
                                    .w(px(40.))
                                    .flex()
                                    .justify_center()
                                    .items_center()
                                    .rounded_full()
                                    .bg(col(SURFACE))
                                    .border_1()
                                    .border_color(col(BORDER))
                                    .text_color(col(TEXT_SECONDARY))
                                    .cursor_pointer()
                                    .active(|s| s.bg(col(SURFACE_ACTIVE)))
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
                                                .bg(col(RED)),
                                        )
                                    })
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.current_view = AppView::Settings;
                                        cx.notify();
                                    })),
                            ),
                    ),
            )
            // Shortcut hints pinned to bottom
            .child(shortcuts_row("space  toggle   r  reset   s  settings"))
            .into_any()
    }

    fn render_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        window.set_window_title("Pomo · Settings");

        let (focus_min, break_min, total_sess) = match &self.pending {
            Some(p) => (p.focus_minutes, p.break_minutes, p.total_sessions as u32),
            None => {
                let s = cx.global::<PomoAppState>();
                (s.focus_minutes, s.break_minutes, s.total_sessions as u32)
            }
        };
        let has_pending = self.pending.is_some();
        let selected_row = self.selected_settings_row;

        let rows: [(EditTarget, &'static str, u32); 3] = [
            (EditTarget::FocusMinutes, "Focus Time", focus_min),
            (EditTarget::BreakMinutes, "Break Time", break_min),
            (EditTarget::TotalSessions, "Total Sessions", total_sess),
        ];

        div()
            .on_action(|_: &CloseWindow, window, _| window.remove_window())
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                match event.keystroke.key.as_str() {
                    "j" => {
                        this.selected_settings_row = (this.selected_settings_row + 1).min(2);
                        cx.notify();
                    }
                    "k" => {
                        this.selected_settings_row = this.selected_settings_row.saturating_sub(1);
                        cx.notify();
                    }
                    "space" | "enter" => {
                        let row = this.selected_settings_row;
                        let (focus_min, break_min, total_sess) = match &this.pending {
                            Some(p) => (p.focus_minutes, p.break_minutes, p.total_sessions as u32),
                            None => {
                                let s = cx.global::<PomoAppState>();
                                (s.focus_minutes, s.break_minutes, s.total_sessions as u32)
                            }
                        };
                        let (target, value) = match row {
                            0 => (EditTarget::FocusMinutes, focus_min),
                            1 => (EditTarget::BreakMinutes, break_min),
                            _ => (EditTarget::TotalSessions, total_sess),
                        };
                        this.edit_value = value;
                        this.current_view = AppView::Edit(target);
                        cx.notify();
                    }
                    "escape" | "s" => {
                        this.current_view = AppView::Timer;
                        cx.notify();
                    }
                    _ => {}
                }
            }))
            .id("settings")
            .size_full()
            .flex()
            .flex_col()
            .bg(col(BG))
            .text_color(col(TEXT_PRIMARY))
            // Header
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .px(px(20.))
                    .pt(px(24.))
                    .pb(px(12.))
                    .gap_3()
                    .child(
                        div()
                            .id("settings_back")
                            .cursor_pointer()
                            .text_xl()
                            .text_color(col(TEXT_SECONDARY))
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
                    .when(has_pending, |s| {
                        s.child(
                            div()
                                .flex()
                                .flex_row()
                                .items_center()
                                .gap_1()
                                .child(div().w(px(7.)).h(px(7.)).rounded_full().bg(col(RED)))
                                .child(div().text_sm().text_color(col(RED)).child("pending reset")),
                        )
                    }),
            )
            // Setting rows
            .children(
                rows.into_iter()
                    .enumerate()
                    .map(|(i, (target, label, value))| {
                        let value_str = target.value_display(value);
                        let id = SharedString::from(label);
                        let is_selected = i == selected_row;
                        div()
                            .id(id)
                            .flex()
                            .flex_row()
                            .justify_between()
                            .items_center()
                            .px(px(24.))
                            .py(px(16.))
                            .border_b_1()
                            .border_color(col(BORDER))
                            .cursor_pointer()
                            .when(is_selected, |s| s.bg(col(SURFACE_ACTIVE)))
                            .active(|s| s.bg(col(SURFACE)))
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
                                            .text_color(col(TEXT_SECONDARY))
                                            .child(value_str),
                                    )
                                    .child(
                                        div().text_base().text_color(col(TEXT_MUTED)).child("›"),
                                    ),
                            )
                    }),
            )
            // Spacer + hints
            .child(div().flex_1())
            .child(shortcuts_row("j/k  navigate   space  open   esc  back"))
            .into_any()
    }

    fn render_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
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
        let is_editing = self.is_editing_value;
        let input_text = self.input_text.clone();

        window.set_window_title(&format!("Pomo · {}", label));

        div()
            .on_action(|_: &CloseWindow, window, _| window.remove_window())
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(move |this, event: &KeyDownEvent, _, cx| {
                if this.is_editing_value {
                    match event.keystroke.key.as_str() {
                        k @ ("0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9") => {
                            if this.input_text.len() < 2 {
                                this.input_text.push_str(k);
                                cx.notify();
                            }
                        }
                        "backspace" => {
                            if this.input_text.pop().is_some() {
                                cx.notify();
                            }
                        }
                        "enter" => {
                            this.commit_input();
                            cx.notify();
                        }
                        "escape" => {
                            this.is_editing_value = false;
                            this.input_text = this.edit_value.to_string();
                            cx.notify();
                        }
                        _ => {}
                    }
                } else {
                    match event.keystroke.key.as_str() {
                        "j" => {
                            if this.edit_value > min {
                                this.edit_value -= 1;
                                cx.notify();
                            }
                        }
                        "k" => {
                            if this.edit_value < max {
                                this.edit_value += 1;
                                cx.notify();
                            }
                        }
                        "space" | "enter" => {
                            this.commit_edit_and_go_back(cx);
                        }
                        "escape" => {
                            this.current_view = AppView::Settings;
                            cx.notify();
                        }
                        _ => {}
                    }
                }
            }))
            .id("edit")
            .size_full()
            .flex()
            .flex_col()
            .bg(col(BG))
            .text_color(col(TEXT_PRIMARY))
            // Header
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .px(px(20.))
                    .pt(px(24.))
                    .pb(px(12.))
                    .gap_3()
                    .child(
                        div()
                            .id("edit_back")
                            .cursor_pointer()
                            .text_xl()
                            .text_color(col(TEXT_SECONDARY))
                            .active(|s| s.opacity(0.6))
                            .child("←")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.commit_edit_and_go_back(cx);
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
                                    .border_color(col(if at_min { SURFACE } else { BORDER }))
                                    .text_size(px(24.))
                                    .font_weight(gpui::FontWeight(700.))
                                    .text_color(col(if at_min { TEXT_MUTED } else { TEXT_PRIMARY }))
                                    .cursor_pointer()
                                    .when(!at_min, |s| s.active(|s| s.bg(col(SURFACE_ACTIVE))))
                                    .child("−")
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.commit_input();
                                        if this.edit_value > min {
                                            this.edit_value -= 1;
                                        }
                                        cx.notify();
                                    })),
                            )
                            .child(
                                div()
                                    .id("value_display")
                                    .w(px(90.))
                                    .flex()
                                    .justify_center()
                                    .items_center()
                                    .py(px(2.))
                                    .text_size(px(52.))
                                    .font_weight(gpui::FontWeight(700.))
                                    .text_color(col(ACCENT_FOCUS))
                                    .when(is_editing, |s| {
                                        s.border_b_2().border_color(col(ACCENT_FOCUS))
                                    })
                                    .when(!is_editing, |s| {
                                        s.cursor_pointer().on_click(cx.listener(
                                            |this, _, _, cx| {
                                                this.input_text = this.edit_value.to_string();
                                                this.is_editing_value = true;
                                                cx.notify();
                                            },
                                        ))
                                    })
                                    .child(if is_editing {
                                        format!("{}|", input_text)
                                    } else {
                                        format!("{}", value)
                                    }),
                            )
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
                                    .border_color(col(if at_max { SURFACE } else { BORDER }))
                                    .text_size(px(24.))
                                    .font_weight(gpui::FontWeight(700.))
                                    .text_color(col(if at_max { TEXT_MUTED } else { TEXT_PRIMARY }))
                                    .cursor_pointer()
                                    .when(!at_max, |s| s.active(|s| s.bg(col(SURFACE_ACTIVE))))
                                    .child("+")
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.commit_input();
                                        if this.edit_value < max {
                                            this.edit_value += 1;
                                        }
                                        cx.notify();
                                    })),
                            ),
                    )
                    .child(div().text_color(col(TEXT_SECONDARY)).child(unit)),
            )
            // Hints
            .child(shortcuts_row(if is_editing {
                "0-9  type   backspace  delete   enter  confirm   esc  cancel"
            } else {
                "j/k  adjust   space  confirm   esc  cancel"
            }))
            .into_any()
    }
}

// ── Shortcut hint row ─────────────────────────────────────────────────────────

fn shortcuts_row(text: &'static str) -> gpui::Div {
    div()
        .w_full()
        .flex()
        .justify_center()
        .pb(px(14.))
        .text_size(px(10.))
        .text_color(col(TEXT_MUTED))
        .child(text)
}

// ── Ring drawing ──────────────────────────────────────────────────────────────

fn paint_donut_arc(
    center_x: f32,
    center_y: f32,
    outer_radius: f32,
    inner_radius: f32,
    start: f32,
    end: f32,
    color: gpui::Hsla,
    window: &mut Window,
) {
    let span = end - start;
    if span.abs() < 0.001 {
        return;
    }

    let outer_start = (
        center_x + outer_radius * start.cos(),
        center_y + outer_radius * start.sin(),
    );
    let outer_end = (
        center_x + outer_radius * end.cos(),
        center_y + outer_radius * end.sin(),
    );
    let inner_end = (
        center_x + inner_radius * end.cos(),
        center_y + inner_radius * end.sin(),
    );
    let inner_start = (
        center_x + inner_radius * start.cos(),
        center_y + inner_radius * start.sin(),
    );

    let large_arc = span > PI;
    let mut builder = PathBuilder::fill();
    // Start at outer_start
    builder.move_to(point(px(outer_start.0), px(outer_start.1)));
    // Move arc clockwise to outer_end
    builder.arc_to(
        point(px(outer_radius), px(outer_radius)),
        px(0.),
        large_arc,
        true,
        point(px(outer_end.0), px(outer_end.1)),
    );
    // Straight line to inner_end
    builder.line_to(point(px(inner_end.0), px(inner_end.1)));
    // Move arc counter_clockwise to inner_start
    builder.arc_to(
        point(px(inner_radius), px(inner_radius)),
        px(0.),
        large_arc,
        false,
        point(px(inner_start.0), px(inner_start.1)),
    );
    // Close Segment
    builder.close();

    if let Ok(path) = builder.build() {
        window.paint_path(path, color);
    }
}

fn paint_ring_track(
    center_x: f32,
    center_y: f32,
    outer_radius: f32,
    inner_radius: f32,
    window: &mut Window,
) {
    let track = col(BORDER);
    // Paint right half
    paint_donut_arc(
        center_x,
        center_y,
        outer_radius,
        inner_radius,
        -PI / 2.,
        PI / 2.,
        track,
        window,
    );
    // Paint left half
    paint_donut_arc(
        center_x,
        center_y,
        outer_radius,
        inner_radius,
        PI / 2.,
        3. * PI / 2.,
        track,
        window,
    );
}

fn paint_ring_progress(
    center_x: f32,
    center_y: f32,
    outer_radius: f32,
    inner_radius: f32,
    progress: f32,
    color: gpui::Hsla,
    window: &mut Window,
) {
    if progress < 0.001 {
        return;
    }
    if progress > 0.999 {
        paint_donut_arc(
            center_x,
            center_y,
            outer_radius,
            inner_radius,
            -PI / 2.,
            PI / 2.,
            color,
            window,
        );
        paint_donut_arc(
            center_x,
            center_y,
            outer_radius,
            inner_radius,
            PI / 2.,
            3. * PI / 2.,
            color,
            window,
        );
        return;
    }
    let end = -PI / 2. + progress * 2. * PI;
    paint_donut_arc(
        center_x,
        center_y,
        outer_radius,
        inner_radius,
        -PI / 2.,
        end,
        color,
        window,
    );
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

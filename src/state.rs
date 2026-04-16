use gpui::Global;

const DEFAULT_FOCUS_MINUTES: u32 = 25;
const DEFAULT_BREAK_MINUTES: u32 = 5;
const DEFAULT_TOTAL_SESSIONS: u8 = 5;

pub struct PomoAppState {
    pub running: bool,
    pub is_break: bool,
    pub sessions_completed: u8,
    pub seconds_left: u64,
    pub focus_minutes: u32,
    pub break_minutes: u32,
    pub total_sessions: u8,
}

impl Global for PomoAppState {}

impl Default for PomoAppState {
    fn default() -> Self {
        Self {
            running: false,
            is_break: false,
            sessions_completed: 0,
            seconds_left: DEFAULT_FOCUS_MINUTES as u64 * 60,
            focus_minutes: DEFAULT_FOCUS_MINUTES,
            break_minutes: DEFAULT_BREAK_MINUTES,
            total_sessions: DEFAULT_TOTAL_SESSIONS,
        }
    }
}

impl PomoAppState {
    pub fn focus_seconds(&self) -> u64 {
        self.focus_minutes as u64 * 60
    }

    pub fn break_seconds(&self) -> u64 {
        self.break_minutes as u64 * 60
    }

    pub fn phase_seconds(&self) -> u64 {
        if self.is_break {
            self.break_seconds()
        } else {
            self.focus_seconds()
        }
    }

    pub fn is_all_done(&self) -> bool {
        self.sessions_completed >= self.total_sessions && self.seconds_left == 0
    }

    pub fn tick(&mut self) {
        if self.seconds_left > 0 {
            self.seconds_left -= 1;
        } else {
            self.advance_phase();
        }
    }

    fn advance_phase(&mut self) {
        if self.is_break {
            self.is_break = false;
            self.seconds_left = self.focus_seconds();
        } else {
            self.sessions_completed += 1;
            if self.sessions_completed < self.total_sessions {
                self.is_break = true;
                self.seconds_left = self.break_seconds();
            } else {
                self.running = false;
            }
        }
    }
}

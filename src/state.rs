use gpui::Global;

pub const FOCUS_SECONDS: u64 = 25 * 60;
pub const BREAK_SECONDS: u64 = 5 * 60;
pub const TOTAL_SESSIONS: u8 = 5;

pub struct PomoAppState {
    pub running: bool,
    pub is_break: bool,
    pub sessions_completed: u8,
    pub seconds_left: u64,
}

impl Global for PomoAppState {}

impl Default for PomoAppState {
    fn default() -> Self {
        Self {
            running: false,
            is_break: false,
            sessions_completed: 0,
            seconds_left: FOCUS_SECONDS,
        }
    }
}

impl PomoAppState {
    pub fn is_all_done(&self) -> bool {
        self.sessions_completed >= TOTAL_SESSIONS && self.seconds_left == 0
    }

    pub fn phase_seconds(&self) -> u64 {
        if self.is_break {
            BREAK_SECONDS
        } else {
            FOCUS_SECONDS
        }
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
            // Break over — auto-continue into next focus session
            self.is_break = false;
            self.seconds_left = FOCUS_SECONDS;
        } else {
            self.sessions_completed += 1;
            if self.sessions_completed < TOTAL_SESSIONS {
                // Focus done — auto-continue into break
                self.is_break = true;
                self.seconds_left = BREAK_SECONDS;
            } else {
                // All sessions complete — stop
                self.running = false;
            }
        }
    }
}

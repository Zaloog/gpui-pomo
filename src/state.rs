use gpui::Global;

pub struct PomoAppState {
    pub running: bool,
    pub is_break: bool,
    pub sessions_completed: u8,
    pub millis_left: u64,
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
            millis_left: 25 * 60_000,
            focus_minutes: 25,
            break_minutes: 5,
            total_sessions: 5,
        }
    }
}

impl PomoAppState {
    pub fn focus_millis(&self) -> u64 {
        self.focus_minutes as u64 * 60_000
    }

    pub fn break_millis(&self) -> u64 {
        self.break_minutes as u64 * 60_000
    }

    pub fn phase_millis(&self) -> u64 {
        if self.is_break {
            self.break_millis()
        } else {
            self.focus_millis()
        }
    }

    pub fn seconds_display(&self) -> u64 {
        self.millis_left / 1000
    }

    pub fn progress(&self) -> f32 {
        let total = self.phase_millis();
        if total == 0 {
            return 1.0;
        }
        1.0 - self.millis_left as f32 / total as f32
    }

    pub fn is_all_done(&self) -> bool {
        self.sessions_completed >= self.total_sessions && self.millis_left == 0
    }

    pub fn tick(&mut self, delta_ms: u64) -> bool {
        if self.millis_left > delta_ms {
            self.millis_left -= delta_ms;
            false
        } else {
            self.millis_left = 0;
            self.advance_phase()
        }
    }

    fn advance_phase(&mut self) -> bool {
        if self.is_break {
            self.is_break = false;
            self.millis_left = self.focus_millis();
            true
        } else {
            self.sessions_completed += 1;
            if self.sessions_completed < self.total_sessions {
                self.is_break = true;
                self.millis_left = self.break_millis();
                true
            } else {
                self.running = false;
                true
            }
        }
    }
}

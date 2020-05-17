use std::sync::Mutex;

#[derive(Debug, Default)]
pub struct PauseState {
    paused: Mutex<bool>,
}

impl PauseState {
    pub fn pause(&self) {
        self.set_paused(true);
    }

    pub fn resume(&self) {
        self.set_paused(false);
    }

    pub fn paused(&self) -> bool {
        *self.paused.lock().unwrap()
    }

    fn set_paused(&self, paused: bool) {
        *self.paused.lock().unwrap() = paused;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_pause_and_unpause() {
        let pause = PauseState::default();
        assert!(!pause.paused());
        pause.pause();
        assert!(pause.paused());
        pause.resume();
        assert!(!pause.paused());
    }
}

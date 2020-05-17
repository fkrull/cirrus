use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug, Default)]
pub struct PauseState {
    paused: AtomicBool,
}

impl PauseState {
    pub fn pause(&self) {
        self.set_paused(true);
    }

    pub fn resume(&self) {
        self.set_paused(false);
    }

    pub fn paused(&self) -> bool {
        self.paused.load(Ordering::Relaxed)
    }

    fn set_paused(&self, paused: bool) {
        self.paused.store(paused, Ordering::Relaxed);
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

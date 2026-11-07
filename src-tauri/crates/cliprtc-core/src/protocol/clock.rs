#[derive(Debug, Default, Clone)]
pub struct LamportClock {
    value: u64,
}

impl LamportClock {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn tick_local(&mut self) -> u64 {
        self.value += 1;
        self.value
    }

    pub fn observe(&mut self, remote: u64) -> u64 {
        self.value = self.value.max(remote) + 1;
        self.value
    }

    pub fn value(&self) -> u64 {
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_ticks_are_monotonic() {
        let mut clock = LamportClock::new();
        assert_eq!(clock.tick_local(), 1);
        assert_eq!(clock.tick_local(), 2);
    }

    #[test]
    fn observing_remote_jumps_ahead() {
        let mut clock = LamportClock::new();
        clock.tick_local();
        assert_eq!(clock.observe(10), 11);
        assert_eq!(clock.tick_local(), 12);
    }
}

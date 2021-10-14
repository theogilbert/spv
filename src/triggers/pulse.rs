use std::thread;
use std::time::{Duration, Instant};

pub struct Pulse {
    last_tick: Instant,
    refresh_period: Duration,
}

/// Drives the cadency of the application, using the blocking pulse method which sleeps for the
/// given refresh period
impl Pulse {
    pub fn new(refresh_period: Duration) -> Self {
        Pulse { last_tick: Instant::now(), refresh_period }
    }

    /// Blocking method that only returns on the next pulse
    pub fn pulse(&mut self) {
        let elapsed = Instant::now().duration_since(self.last_tick);
        thread::sleep(self.refresh_period - elapsed);

        self.last_tick = Instant::now();
    }
}

#[cfg(test)]
mod test_pulse {
    use std::time::{Duration, SystemTime};

    use crate::triggers::pulse::Pulse;

    #[test]
    fn test_should_respect_refresh_period() {
        let tolerance_in_ms = 2;
        let mut pulse = Pulse::new(Duration::from_millis(10));

        let start = SystemTime::now();

        for _ in 0..10 {
            pulse.pulse();
        }

        let elapsed = SystemTime::now().duration_since(start)
            .expect("Error calculating pulse test elapsed time");

        // 10 pulses at a refresh period of 10ms should take ~100ms to complete
        assert!(elapsed.as_millis() > 100 - tolerance_in_ms);
        assert!(elapsed.as_millis() < 100 + tolerance_in_ms);
    }
}
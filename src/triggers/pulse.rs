use std::time::{Duration, Instant};
use std::thread;

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
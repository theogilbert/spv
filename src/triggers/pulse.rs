use std::time::{Duration, Instant};
use std::thread;

pub struct Pulse {
    last_tick: Instant,
    iteration_lapse: Duration,
}

/// Drives the cadency of the application, using the blocking pulse method which releases on given
/// interval
impl Pulse {
    pub fn new(periodic_time: Duration) -> Self {
        Pulse { last_tick: Instant::now(), iteration_lapse: periodic_time }
    }

    /// Blocking method that only returns on the next pulse
    pub fn pulse(&mut self) {
        let elapsed = Instant::now().duration_since(self.last_tick);
        thread::sleep(self.iteration_lapse - elapsed);

        self.last_tick = Instant::now();
    }
}
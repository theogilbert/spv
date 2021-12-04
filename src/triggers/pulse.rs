use std::ops::{Add, Div};
use std::thread;
use std::time::{Duration, Instant};

/// Offers a blocking pulse method which only releases after the given refresh period as elapsed.<br/>
/// This method can be used to drive the cadency of the application, by sending out an event every time the `pulse()`
/// method releases.
pub struct Pulse {
    last_tick: Instant,
    refresh_period: Duration,
    poll_sleep: Duration,
}

impl Pulse {
    pub fn new(refresh_period: Duration) -> Self {
        Pulse {
            last_tick: Instant::now(),
            refresh_period,
            poll_sleep: Self::tolerance(refresh_period),
        }
    }

    /// Calculates the maximum error tolerance of the pulse() method, compared to the expected elapsed time `refresh_period`
    /// Currently, this value is defined at 10% of the refresh period.
    pub fn tolerance(refresh_period: Duration) -> Duration {
        refresh_period.div(10)
    }

    /// Blocking method that only returns after the refresh period (± 10%) has elapsed
    ///
    /// Calling `pulse()` repeatedly is guaranteed not to provoke a drift over time.<br/>
    ///
    /// After calling `pulse()` `N` times with a refresh period `R`, the elapsed time is guaranteed to be (`N` * `R`) ± `T`,
    /// where `T` is the inaccuracy tolerance of `pulse()`. `T` is currently configured as 10% of the refresh period `R`.
    ///
    /// If `pulse()` were to drift, after `N` calls, the elapsed time would be `N` * (`R` + `D`),
    /// where `D` is the local drift duration of the `pulse()` method.
    /// Although the drift would be negligeable compared to the inaccuracy tolerance `T` for low `N` values,
    /// as `N` increases, the drift would become more and more noticeable.
    pub fn pulse(&mut self) {
        let next_pulse_instant = self.next_pulse_instant();

        while Instant::now() < next_pulse_instant {
            thread::sleep(self.poll_sleep);
        }

        self.last_tick = next_pulse_instant;
    }

    fn next_pulse_instant(&self) -> Instant {
        self.last_tick.add(self.refresh_period)
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

        let elapsed = SystemTime::now()
            .duration_since(start)
            .expect("Error calculating pulse test elapsed time");

        // 10 pulses at a refresh period of 10ms should take ~100ms to complete
        assert!(elapsed.as_millis() > 100 - tolerance_in_ms);
        assert!(elapsed.as_millis() < 100 + tolerance_in_ms);
    }
}

//! Time tracking utilities
//!
//! In this application, we do not want to manipulate Instant directly for the following reasons:
//! - We want events happening during the same iteration to have the exact same timestamp
//! - Instant makes test-writing difficult without mocks.
//!   By localizing all `Instant` references to this location, we facilitate mock facilities

use std::cell::RefCell;
use std::ops::Sub;
use std::time::Duration;
#[cfg(not(test))]
use std::time::Instant;

#[cfg(test)]
use sn_fake_clock::FakeClock as Instant;

thread_local! {
    /// Contains the internal timestamp value of the last iteration
    static LAST_ITER_STAMP: RefCell<Instant> = RefCell::new(Instant::now());
}

pub(crate) fn update_iteration_timestamp() {
    LAST_ITER_STAMP.with(|stamp_rc| stamp_rc.replace(Instant::now()));
}

/// Contains various utilities used to manipulate the current time
/// Note that updating the current time will not affect `Timestamp::now()` is `update_iteration_timestamp()` is not called.
#[cfg(test)]
pub mod time_mocking {
    use crate::core::time::update_iteration_timestamp;
    use sn_fake_clock::FakeClock;

    pub fn set_last_iteration_timestamp(time: u64) {
        FakeClock::set_time(time);
        update_iteration_timestamp();
    }

    pub fn advance_current_time(millis: u64) {
        FakeClock::advance_time(millis)
    }
}

fn last_iteration_stamp() -> Instant {
    LAST_ITER_STAMP
        .with(|stamp_rc| stamp_rc.try_borrow().map(|s| *s))
        // This should never happen: last_iteration_stamp is never borrowed mutably,
        // maybe except when calling replace_with() above, which should not be an issue in a local thread context.
        .expect("Error fetching timestamp: unexpectedly mutably borrowed")
}

/// A `Timestamp` represents a temporal pointer to an event
#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Copy, Clone)]
pub struct Timestamp {
    stamp: Instant,
}

impl Timestamp {
    /// Returns a `Timestamp` referencing the current time.
    /// Multiple timestamps generated during a single iteration will always be equal.
    pub fn now() -> Self {
        Self {
            stamp: last_iteration_stamp(),
        }
    }

    /// Calculates elapsed time between `self` and `earlier`
    /// If `earlier` is later than `self`, this method will panic
    pub fn duration_since(&self, earlier: &Timestamp) -> Duration {
        self.stamp.duration_since(earlier.stamp)
    }
}

impl Sub<Duration> for Timestamp {
    type Output = Timestamp;

    fn sub(self, rhs: Duration) -> Self::Output {
        Timestamp {
            stamp: self.stamp - rhs,
        }
    }
}

#[cfg(test)]
mod test_timestamp {
    use crate::core::time::time_mocking::{advance_current_time, set_last_iteration_timestamp};
    use crate::core::time::{update_iteration_timestamp, Timestamp};
    use std::time::Duration;

    #[test]
    fn test_should_always_produce_same_stamp_on_same_iteration() {
        let timestamp_1 = Timestamp::now();
        advance_current_time(1000);
        let timestamp_2 = Timestamp::now();

        assert_eq!(timestamp_1, timestamp_2);
    }

    #[test]
    fn test_should_produce_different_stamps_on_different_iterations() {
        let timestamp_1 = Timestamp::now();
        advance_current_time(1000);
        update_iteration_timestamp();
        let timestamp_2 = Timestamp::now();

        assert!(timestamp_1 < timestamp_2);
    }

    #[test]
    fn test_should_correctly_calculate_duration_between_timestamps() {
        let timestamp_1 = Timestamp::now();
        advance_current_time(1234);
        update_iteration_timestamp();
        let timestamp_2 = Timestamp::now();

        assert_eq!(timestamp_2.duration_since(&timestamp_1), Duration::from_millis(1234));
    }

    #[test]
    fn test_should_correctly_substract_duration_from_timestamp() {
        set_last_iteration_timestamp(100000);
        let timestamp_1 = Timestamp::now();
        let timestamp_2 = timestamp_1 - Duration::from_millis(123);

        assert_eq!(timestamp_1.duration_since(&timestamp_2), Duration::from_millis(123));
    }
}

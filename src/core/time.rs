//! Time tracking utilities
//!
//! In this application, we do not want to manipulate Instant directly for the following reasons:
//! - We want events happening during the same iteration to have the exact same timestamp
//! - Instant makes test-writing difficult without mocks.
//!   By localizing all `Instant` references to this location, we facilitate mock facilities

use std::cell::RefCell;
use std::ops::{Add, Sub};
use std::time::Duration;
#[cfg(not(test))]
use std::time::Instant;

#[cfg(test)]
use sn_fake_clock::FakeClock as Instant;

/// Represents the data that needs to be static and local to a thread, to synchronize all timestamps of a same iteration
struct GlobalTimestamp {
    current_timestamp: RefCell<Timestamp>,
    initial_timestamp: Timestamp,
}

impl GlobalTimestamp {
    fn new() -> Self {
        let now = Timestamp::from_current_instant();
        Self {
            current_timestamp: RefCell::new(now),
            initial_timestamp: now,
        }
    }

    fn refresh(&self) {
        let now = Timestamp::from_current_instant();
        self.current_timestamp.replace(now);
    }

    fn current(&self) -> Timestamp {
        *self
            .current_timestamp
            .try_borrow()
            .expect("Error fetching timestamp: unexpectedly mutably borrowed")
    }

    fn initial(&self) -> Timestamp {
        self.initial_timestamp
    }
}

thread_local! {
    static GLOBAL_TIMESTAMP: GlobalTimestamp = GlobalTimestamp::new();
}

fn last_iteration_stamp() -> Timestamp {
    GLOBAL_TIMESTAMP.with(|stamp_rc| stamp_rc.current())
}

fn first_iteration_timestamp() -> Timestamp {
    GLOBAL_TIMESTAMP.with(|stamp_rc| stamp_rc.initial())
}

/// Updates the value returned by `Timestamp::now()`.
///
/// All timestamp creations between two calls of this function return the same value.
pub(crate) fn refresh_current_timestamp() {
    GLOBAL_TIMESTAMP.with(|stamp_rc| stamp_rc.refresh());
}

/// Contains various utilities used to manipulate the current time
#[cfg(test)]
pub mod test_utils {
    use std::time::Duration;

    use sn_fake_clock::FakeClock;

    use crate::core::time::refresh_current_timestamp;

    /// Advance the time so that `Timestamp::now()` returns an updated value
    pub fn advance_time_and_refresh_timestamp(duration: Duration) {
        refresh_current_timestamp(); // We do a first refresh to set Timestamp::app_init()
        FakeClock::advance_time(duration.as_millis() as u64);
        refresh_current_timestamp();
    }

    /// FakeClock returns a default Instant with a timestamp 0 (represented as u64) if the time is not set.
    /// This means that subtracting from Instant::now() in a test will produce a subtraction overflow, as we will try
    /// to produce an unsigned value under 0.
    /// To avoid this error, we configure the current time of FakeClock to a high value, so that we can safely subtract
    /// from it.
    pub fn setup_fake_clock_to_prevent_substract_overflow() {
        const CURRENT_MS: u64 = 365 * 24 * 3600 * 1000;

        // We do a first refresh to set Timestamp::app_init() to 5 min in the past
        FakeClock::set_time(CURRENT_MS - 300 * 1000);
        refresh_current_timestamp();
        // And now to actually make Timestamp::now() reflect the updated time
        FakeClock::set_time(CURRENT_MS);
        refresh_current_timestamp();
    }
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
        last_iteration_stamp()
    }

    /// Returns the `Timestamp` at which the application started
    pub fn app_init() -> Self {
        first_iteration_timestamp()
    }

    /// Builds a timestamp from an `Instant` value
    pub fn from_instant(instant: Instant) -> Self {
        Self { stamp: instant }
    }

    /// Builds a timestamp from the current `Instant::now()` value
    pub fn from_current_instant() -> Self {
        Self::from_instant(Instant::now())
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

impl Add<Duration> for Timestamp {
    type Output = Timestamp;

    fn add(self, rhs: Duration) -> Self::Output {
        Timestamp {
            stamp: self.stamp + rhs,
        }
    }
}

#[cfg(test)]
mod test_timestamp {
    use std::time::Duration;

    use sn_fake_clock::FakeClock;

    use crate::core::time::test_utils::advance_time_and_refresh_timestamp;
    use crate::core::time::Timestamp;

    #[test]
    fn test_should_always_produce_same_stamp_on_same_iteration() {
        let timestamp_1 = Timestamp::now();
        FakeClock::advance_time(1000);
        let timestamp_2 = Timestamp::now();

        assert_eq!(timestamp_1, timestamp_2);
    }

    #[test]
    fn test_should_produce_different_stamps_on_different_iterations() {
        let timestamp_1 = Timestamp::now();
        advance_time_and_refresh_timestamp(Duration::from_secs(1));
        let timestamp_2 = Timestamp::now();

        assert!(timestamp_1 < timestamp_2);
    }

    #[test]
    fn test_should_correctly_calculate_duration_between_timestamps() {
        let timestamp_1 = Timestamp::now();
        advance_time_and_refresh_timestamp(Duration::from_millis(1234));
        let timestamp_2 = Timestamp::now();

        assert_eq!(timestamp_2.duration_since(&timestamp_1), Duration::from_millis(1234));
    }

    #[test]
    fn test_should_correctly_substract_duration_from_timestamp() {
        let timestamp_1 = Timestamp::now();
        let timestamp_2 = timestamp_1 + Duration::from_millis(123);

        assert_eq!(timestamp_2.duration_since(&timestamp_1), Duration::from_millis(123));
    }
}

/// Represents a temporal region
///
/// A `Span` is defined by a `begin` and an `end` timestamp
/// The `begin` and `end` timestamps are inclusive.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct Span {
    begin: Timestamp,
    end: Timestamp,
}

impl Span {
    pub fn new(begin: Timestamp, end: Timestamp) -> Self {
        Span { begin, end }
    }

    /// Creates a `Span` starting and ending at the given `Timestamp`
    ///
    /// # Arguments
    /// * `begin`: The left-bound iteration of the span
    pub fn from_begin(begin: Timestamp) -> Self {
        let end = begin;
        Span { begin, end }
    }

    /// Creates a `Span` that ends at `Timestamp::now()` and covers the given duration
    /// To update the end of the span, see [`set_end_and_shift`](#method.set_end_and_shift)
    ///
    /// # Arguments
    /// * `size`: The size of the `Span`. It must at least 1.
    pub fn from_duration(duration: Duration) -> Self {
        let end = Timestamp::now();
        let begin = end - duration;

        Span { begin, end }
    }

    /// Updates the begining value of the span without updating its end
    /// After this operation, the `end` value of the span will remain the same.
    ///
    /// This method panics if `begin` is greater than `end`.
    ///
    /// # Arguments
    /// * `begin`: The first timestamp covered by the span

    pub fn set_begin_and_resize(&mut self, begin: Timestamp) {
        if begin > self.end {
            panic!("Invalid begin for span {:?}: {:?}", self, begin);
        }
        self.begin = begin;
    }

    /// Updates the end of the span without updating the begining of the span
    /// After this operation, the `begin` value of the span will remain the same.
    ///
    /// This method panics if `end` is less than `begin`.
    ///
    /// # Arguments
    /// * `end`: The last timestamp covered by the span
    pub fn set_end_and_resize(&mut self, end: Timestamp) {
        if end < self.begin {
            panic!("Invalid end for span {:?}: {:?}", self, end);
        }
        self.end = end;
    }

    /// Updates the end of the span and updates the `begin` attribute so that the span covers the same duration
    ///
    /// # Arguments
    /// * `end`: The new maximum timestamp covered by the span
    pub fn set_end_and_shift(&mut self, end: Timestamp) {
        let duration = self.end.duration_since(&self.begin);
        self.end = end;
        self.begin = end - duration;
    }

    /// Returns the first timestamp covered by the span.
    /// This value can never be greater than `self.end()`
    pub fn begin(&self) -> Timestamp {
        self.begin
    }

    /// Returns the last timestamp covered by the span
    /// This value can never be less than `self.begin()`
    pub fn end(&self) -> Timestamp {
        self.end
    }

    /// Returns the amount of time covered by the span.<br/>
    pub fn duration(&self) -> Duration {
        self.end.duration_since(&self.begin)
    }

    /// Returns true if `self` intersects with `other`
    ///
    /// # Arguments
    /// * `other`: A `Span` reference for which to test an intersection with `self`
    pub fn intersects(&self, other: &Span) -> bool {
        !(self.end < other.begin || self.begin > other.end)
    }

    /// Returns true if `self` contains the timestamp
    pub fn contains(&self, timestamp: Timestamp) -> bool {
        self.begin <= timestamp && timestamp <= self.end
    }
}

#[cfg(test)]
mod test_span {
    use std::time::Duration;

    use rstest::*;

    use crate::core::time::test_utils::setup_fake_clock_to_prevent_substract_overflow;
    use crate::core::time::{Span, Timestamp};

    #[test]
    fn test_should_correctly_define_span_when_creating_from_begin() {
        let span = Span::from_begin(Timestamp::now());

        assert_eq!(span.begin(), Timestamp::now());
        assert_eq!(span.end(), Timestamp::now());
        assert_eq!(span.duration(), Duration::from_secs(0));
    }

    #[test]
    fn test_should_correctly_define_span_when_creating_from_duration() {
        setup_fake_clock_to_prevent_substract_overflow();
        let span = Span::from_duration(Duration::from_secs(10));

        assert_eq!(span.end(), Timestamp::now());
        assert_eq!(span.begin(), span.end() - Duration::from_secs(10));
        assert_eq!(span.duration(), Duration::from_secs(10));
    }

    #[test]
    fn test_should_update_span_when_setting_end_and_updating_begin() {
        setup_fake_clock_to_prevent_substract_overflow();
        let mut span = Span::from_duration(Duration::from_secs(60));
        let original_span = span;

        span.set_end_and_shift(span.end() + Duration::from_secs(120));

        assert_eq!(span.begin(), original_span.begin() + Duration::from_secs(120));
        assert_eq!(span.end(), original_span.end() + Duration::from_secs(120));
        assert_eq!(span.duration(), Duration::from_secs(60));
    }

    #[test]
    fn test_should_update_span_when_setting_begin_and_updating_size() {
        setup_fake_clock_to_prevent_substract_overflow();

        let mut span = Span::from_duration(Duration::from_secs(10));
        let original_span = span;

        span.set_begin_and_resize(span.begin() + Duration::from_secs(3));

        assert_eq!(span.begin(), original_span.begin() + Duration::from_secs(3));
        assert_eq!(span.end(), original_span.end());
        assert_eq!(span.duration(), Duration::from_secs(7));
    }

    #[test]
    fn test_should_update_span_when_setting_end_and_updating_size() {
        let mut span = Span::from_begin(Timestamp::now());
        let original_span = span;

        span.set_end_and_resize(span.end() + Duration::from_secs(10));

        assert_eq!(span.begin(), original_span.begin());
        assert_eq!(span.end(), original_span.end() + Duration::from_secs(10));
    }

    #[rstest]
    #[case(50, 250)]
    #[case(50, 100)]
    #[case(120, 170)]
    #[case(199, 250)]
    fn test_should_return_true_if_spans_intersect(#[case] begin_other: u64, #[case] end_other: u64) {
        let now = Timestamp::now();

        let span = Span::new(now + Duration::from_secs(100), now + Duration::from_secs(199));
        let other_span = Span::new(
            now + Duration::from_secs(begin_other),
            now + Duration::from_secs(end_other),
        );

        assert!(span.intersects(&other_span));
        assert!(other_span.intersects(&span));
    }

    #[rstest]
    #[case(50, 75)]
    #[case(250, 275)]
    fn test_should_return_false_if_spans_do_not_intersect(#[case] begin_other: u64, #[case] end_other: u64) {
        let now = Timestamp::now();

        let span = Span::new(now + Duration::from_secs(100), now + Duration::from_secs(199));
        let other_span = Span::new(
            now + Duration::from_secs(begin_other),
            now + Duration::from_secs(end_other),
        );

        assert!(!span.intersects(&other_span));
        assert!(!other_span.intersects(&span));
    }

    #[rstest]
    #[case(- 10, 10)]
    #[case(0, 10)]
    #[case(- 10, 0)]
    fn test_should_return_true_if_timestamp_contained_in_span(#[case] relative_begin: i64, #[case] relative_end: u64) {
        setup_fake_clock_to_prevent_substract_overflow();
        let timestamp = Timestamp::now();

        let span = Span::new(
            timestamp - Duration::from_secs(relative_begin.unsigned_abs()),
            timestamp + Duration::from_secs(relative_end),
        );

        assert!(span.contains(timestamp));
    }

    #[rstest]
    fn test_should_return_false_if_timestamp_not_contained_in_span() {
        let timestamp = Timestamp::now();

        let span = Span::new(timestamp + Duration::from_secs(1), timestamp + Duration::from_secs(20));

        assert!(!span.contains(timestamp));
    }
}

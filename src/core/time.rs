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
        let now = Timestamp::from_instant(Instant::now());
        Self {
            current_timestamp: RefCell::new(now),
            initial_timestamp: now,
        }
    }

    fn refresh(&self) {
        let now = Timestamp::from_instant(Instant::now());
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
    use crate::core::time::refresh_current_timestamp;
    use sn_fake_clock::FakeClock;
    use std::time::Duration;

    /// Advance the time so that `Timestamp::now()` returns an updated value
    pub fn advance_time_and_refresh_timestamp(duration: Duration) {
        FakeClock::advance_time(duration.as_millis() as u64);
        refresh_current_timestamp();
    }

    /// FakeClock returns a default Instant of 0 if the time is not set.
    /// This means that substracting from Instant::now() in a test will produce a subtraction overflow, as we will try
    /// to calculate an unsigned value under 0.
    /// To avoid this error, we configure the current time of FakeClock to a high value, so that we can safely subtract
    /// from it.
    pub fn setup_fake_clock_to_prevent_substract_overflow() {
        FakeClock::set_time(365 * 24 * 3600 * 1000);
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

    /// Builds a timestamp from an `Instant` value
    /// For internal usage only
    fn from_instant(instant: Instant) -> Self {
        Self { stamp: instant }
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
    use crate::core::time::test_utils::advance_time_and_refresh_timestamp;
    use crate::core::time::Timestamp;
    use sn_fake_clock::FakeClock;
    use std::time::Duration;

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
    tolerance: Duration,
}

impl Span {
    #[cfg(test)]
    pub fn new(begin: Timestamp, end: Timestamp) -> Self {
        let tolerance = Duration::default();

        Span { begin, end, tolerance }
    }

    /// Creates a `Span` starting and ending at the given `Timestamp`
    ///
    /// # Arguments
    /// * `begin`: The left-bound iteration of the span
    pub fn from_begin(begin: Timestamp) -> Self {
        let end = begin;
        let tolerance = Duration::default();

        Span { begin, end, tolerance }
    }

    /// Creates a `Span` that ends at `Timestamp::now()` and covers the given duration
    /// To update the end of the span, see [`set_end_and_shift`](#method.set_end_and_shift)
    ///
    /// # Arguments
    /// * `size`: The size of the `Span`. It must at least 1.
    pub fn from_duration(duration: Duration) -> Self {
        let end = Timestamp::now();
        let begin = end - duration;
        let tolerance = Duration::default();

        Span { begin, end, tolerance }
    }

    /// Tracking time precisely to the nanosecond is difficult.<br/>
    /// Setting a tolerance, will loosen the constraints of the span, used by `contains()` and `intersects()`.
    /// This is done by executing these checks as if the span started `tolerance` earlier.<br/>
    ///
    /// By default, a span has a tolerance of 0 seconds.
    pub fn set_tolerance(&mut self, tolerance: Duration) {
        self.tolerance = tolerance;
    }

    /// Updates the end of the span without updating the begining of the span
    /// After this operation, the `begin` iteration of the span will remain the same.
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
        !(self.end < other.tolerant_begin() || self.tolerant_begin() > other.end)
    }

    /// Returns true if `self` contains the timestamp
    pub fn contains(&self, timestamp: Timestamp) -> bool {
        self.tolerant_begin() <= timestamp && timestamp <= self.end
    }

    fn tolerant_begin(&self) -> Timestamp {
        self.begin - self.tolerance
    }

    /// Updates the span by offseting the `begin` and `end` attributes of the span toward the future
    ///
    /// The span cannot be scrolled after the current timestamp.
    ///
    /// # Arguments
    /// * `delta`: Indicates by how much time to shift the span to the right.<br/>
    pub fn scroll_right(&mut self, delta: Duration) {
        self.set_bounded_end_and_shift(self.end + delta);
    }

    /// Updates the span by offseting the `begin` and `end` attributes of the span toward the past
    ///
    /// The span cannot be scrolled before the first iteration of the program
    ///
    /// # Arguments
    /// * `delta`: Indicates by how much time to shift the span to the left.<br/>
    pub fn scroll_left(&mut self, delta: Duration) {
        self.set_bounded_end_and_shift(self.end - delta);
    }

    /// Behaves the same way as `set_end_and_shift()`, except the span is bounded between the first timestamp of the
    /// application and the current one.
    fn set_bounded_end_and_shift(&mut self, unbounded_end: Timestamp) {
        let min_end = first_iteration_timestamp() + self.duration();
        let max_end = Timestamp::now();
        let bounded_end = unbounded_end.max(min_end).min(max_end);
        self.set_end_and_shift(bounded_end);
    }

    /// Indicates if the span is fully scrolled to the right (toward the current timestamp) or if it can be further
    /// scrolled to the right.
    ///
    /// # Arguments
    /// * `current_iteration` The current iteration of the program
    pub fn is_fully_scrolled_right(&self) -> bool {
        self.end == Timestamp::now()
    }
}

#[cfg(test)]
mod test_span {
    use rstest::*;
    use std::time::Duration;

    use crate::core::time::test_utils::{
        advance_time_and_refresh_timestamp, setup_fake_clock_to_prevent_substract_overflow,
    };
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

    #[test]
    fn test_should_be_tolerant_with_span_in_past_within_tolerance_constraints() {
        let now = Timestamp::now();

        let mut span = Span::new(now + Duration::from_secs(100), now + Duration::from_secs(199));
        let other_span = Span::new(now + Duration::from_secs(10), now + Duration::from_secs(98));

        span.set_tolerance(Duration::from_secs(2));

        assert!(span.intersects(&other_span));
        assert!(other_span.intersects(&span));
    }

    #[test]
    fn test_should_not_be_tolerant_with_span_in_past_out_of_tolerance_bounds() {
        let now = Timestamp::now();

        let mut span = Span::new(now + Duration::from_secs(100), now + Duration::from_secs(199));
        let other_span = Span::new(now + Duration::from_secs(10), now + Duration::from_secs(98));

        span.set_tolerance(Duration::from_secs(1));

        assert!(!span.intersects(&other_span));
        assert!(!other_span.intersects(&span));
    }

    #[test]
    fn test_should_not_be_tolerant_with_span_in_future() {
        let now = Timestamp::now();

        let mut span = Span::new(now + Duration::from_secs(100), now + Duration::from_secs(199));
        let other_span = Span::new(now + Duration::from_secs(200), now + Duration::from_secs(299));

        span.set_tolerance(Duration::from_secs(10));

        assert!(!span.intersects(&other_span));
        assert!(!other_span.intersects(&span));
    }

    #[rstest]
    #[case(-10, 10)]
    #[case(0, 10)]
    #[case(-10, 0)]
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

    #[test]
    fn test_should_be_tolerant_with_timestamp_in_past_within_tolerance_constraints() {
        let timestamp = Timestamp::now();

        let mut span = Span::new(timestamp + Duration::from_secs(2), timestamp + Duration::from_secs(20));
        span.set_tolerance(Duration::from_secs(2));

        assert!(span.contains(timestamp));
    }

    #[test]
    fn test_should_not_be_tolerant_with_timestamp_in_past_out_of_tolerance_bounds() {
        let timestamp = Timestamp::now();

        let mut span = Span::new(timestamp + Duration::from_secs(2), timestamp + Duration::from_secs(20));
        span.set_tolerance(Duration::from_secs(1));

        assert!(!span.contains(timestamp));
    }

    #[test]
    fn test_should_not_be_tolerant_with_timestamp_in_future() {
        let origin = Timestamp::now();

        let mut span = Span::new(origin + Duration::from_secs(90), origin + Duration::from_secs(100));
        span.set_tolerance(Duration::from_secs(10));

        assert!(!span.contains(origin + Duration::from_secs(101)));
    }

    #[test]
    fn test_should_scroll_to_the_right() {
        let first_timestamp = Timestamp::now();
        advance_time_and_refresh_timestamp(Duration::from_secs(60));

        let mut span = Span::new(
            first_timestamp + Duration::from_secs(10),
            first_timestamp + Duration::from_secs(20),
        );
        span.scroll_right(Duration::from_secs(60));

        assert_eq!(span.begin(), first_timestamp + Duration::from_secs(50));
        assert_eq!(span.end(), first_timestamp + Duration::from_secs(60));
    }

    #[test]
    fn test_should_not_scroll_after_current_iteration() {
        let first_timestamp = Timestamp::now();
        advance_time_and_refresh_timestamp(Duration::from_secs(60));

        let mut span = Span::new(
            first_timestamp + Duration::from_secs(10),
            first_timestamp + Duration::from_secs(20),
        );
        span.scroll_right(Duration::from_secs(10));

        assert_eq!(span.begin(), first_timestamp + Duration::from_secs(20));
        assert_eq!(span.end(), first_timestamp + Duration::from_secs(30));
    }

    #[test]
    fn test_should_scroll_to_the_left() {
        let first_timestamp = Timestamp::now();
        advance_time_and_refresh_timestamp(Duration::from_secs(60));

        let mut span = Span::new(
            first_timestamp + Duration::from_secs(20),
            first_timestamp + Duration::from_secs(30),
        );
        span.scroll_left(Duration::from_secs(10));

        assert_eq!(span.begin(), first_timestamp + Duration::from_secs(10));
        assert_eq!(span.end(), first_timestamp + Duration::from_secs(20));
    }

    #[test]
    fn test_should_not_scroll_before_first_timestamp() {
        let first_timestamp = Timestamp::now();
        advance_time_and_refresh_timestamp(Duration::from_secs(60));

        let mut span = Span::new(
            first_timestamp + Duration::from_secs(20),
            first_timestamp + Duration::from_secs(30),
        );
        span.scroll_left(Duration::from_secs(30));

        assert_eq!(span.begin(), first_timestamp + Duration::from_secs(0));
        assert_eq!(span.end(), first_timestamp + Duration::from_secs(10));
    }

    #[test]
    fn test_should_be_fully_scrolled_to_the_right_by_default() {
        setup_fake_clock_to_prevent_substract_overflow();
        let span = Span::from_duration(Duration::from_secs(60));

        assert!(span.is_fully_scrolled_right());
    }

    #[test]
    fn test_should_be_fully_scrolled_to_the_right_when_shifted_to_current_iteration() {
        setup_fake_clock_to_prevent_substract_overflow();
        let mut span = Span::from_duration(Duration::from_secs(60));
        advance_time_and_refresh_timestamp(Duration::from_secs(60));
        span.set_end_and_shift(span.end() + Duration::from_secs(60));

        assert!(span.is_fully_scrolled_right());
    }

    #[test]
    fn test_should_not_be_fully_scrolled_to_the_right_when_not_ends_at_current_iteration() {
        setup_fake_clock_to_prevent_substract_overflow();
        let span = Span::from_duration(Duration::from_secs(60));
        advance_time_and_refresh_timestamp(Duration::from_secs(60));

        assert!(!span.is_fully_scrolled_right());
    }
}
